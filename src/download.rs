//! 瓦片下载引擎：并发抓取、失败重试、本地缓存
use crossbeam_channel::{Receiver, Sender};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

pub const DEFAULT_CONCURRENCY: usize = 4;
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_RETRIES: u32 = 3;
pub const USER_AGENT: &str = "HDMapDownloader/3.0 (Rust)";

#[derive(Clone, Debug)]
pub struct TileJob {
    pub z: u8,
    pub x: i32,
    pub y: i32,
    pub url: String,
}

#[derive(Clone, Debug)]
pub enum DlEvent {
    Started { total: usize, skipped: usize },
    Done { z: u8, x: i32, y: i32, bytes: u64 },
    Failed { z: u8, x: i32, y: i32, reason: String },
    Finished { ok: usize, failed: usize, skipped: usize },
}

pub type EventRx = Receiver<DlEvent>;
pub type EventTx = Sender<DlEvent>;

pub fn tile_path(root: &Path, layer: &str, z: u8, x: i32, y: i32) -> PathBuf {
    root.join("tiles").join(layer).join(z.to_string()).join(x.to_string()).join(format!("{y}.png"))
}

pub fn load_cached(root: &Path, layer: &str, z: u8, x: i32, y: i32) -> Option<Vec<u8>> {
    std::fs::read(tile_path(root, layer, z, x, y)).ok()
}

pub fn save_cached(root: &Path, layer: &str, z: u8, x: i32, y: i32, bytes: &[u8]) -> std::io::Result<()> {
    let p = tile_path(root, layer, z, x, y);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(p, bytes)
}

/// 只接受真正的图片字节（PNG/JPEG/GIF/WEBP），挡掉 403/404 的 HTML 错误页
pub fn looks_like_image(b: &[u8]) -> bool {
    b.starts_with(&[0x89, b'P', b'N', b'G'])
        || b.starts_with(&[0xFF, 0xD8])
        || b.starts_with(b"GIF8")
        || (b.len() > 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP")
}

pub fn fetch_tile(url: &str, timeout_secs: u64, retries: u32) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .user_agent(USER_AGENT)
        .build();
    let mut last = String::new();
    for attempt in 0..=retries {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(300 * attempt as u64));
        }
        let resp = agent.get(url).set("Referer", "https://www.openstreetmap.org/").call();
        match resp {
            Ok(resp) => {
                let status = resp.status();
                if status >= 400 {
                    last = format!("HTTP {}", status);
                    continue;
                }
                let mut buf = Vec::new();
                match resp.into_reader().read_to_end(&mut buf) {
                    Ok(_) => {
                        if looks_like_image(&buf) {
                            return Ok(buf);
                        }
                        last = if buf.is_empty() {
                            "空响应".to_string()
                        } else {
                            format!("响应不是图片（{} 字节）", buf.len())
                        };
                    }
                    Err(e) => last = format!("读取失败: {e}"),
                }
            }
            Err(e) => last = format!("{e}"),
        }
    }
    Err(last)
}

/// 框选经纬度范围 -> 该缩放级别下的瓦片行列范围
pub fn tiles_in_rect(min_lng: f64, min_lat: f64, max_lng: f64, max_lat: f64, z: u8) -> (i32, i32, i32, i32) {
    let ll_min = crate::geo::LngLat::new(min_lng, min_lat).clamp();
    let ll_max = crate::geo::LngLat::new(max_lng, max_lat).clamp();
    let (x0, y0) = crate::geo::tile_of(ll_min, z);
    let (x1, y1) = crate::geo::tile_of(ll_max, z);
    let n = (1i32 << z.min(22)) - 1;
    (
        x0.min(x1).max(0),
        y0.min(y1).max(0),
        x0.max(x1).min(n),
        y0.max(y1).min(n),
    )
}

pub fn run_batch(
    jobs: Vec<TileJob>,
    root: PathBuf,
    layer: String,
    concurrency: usize,
    timeout_secs: u64,
    retries: u32,
    resume: bool,
    cancel: Arc<AtomicBool>,
    events: Sender<DlEvent>,
) {
    let total = jobs.len();
    let mut pending = Vec::new();
    let mut skipped = 0;
    for j in jobs {
        if resume && tile_path(&root, &layer, j.z, j.x, j.y).exists() {
            skipped += 1;
            continue;
        }
        pending.push(j);
    }
    let _ = events.send(DlEvent::Started { total, skipped });

    let queue = Arc::new(std::sync::Mutex::new(pending.into_iter().collect::<std::collections::VecDeque<_>>()));
    let ok = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));

    let threads = concurrency.clamp(1, 32);
    std::thread::scope(|s| {
        for _ in 0..threads {
            let queue = Arc::clone(&queue);
            let root = root.clone();
            let layer = layer.clone();
            let ok = Arc::clone(&ok);
            let failed = Arc::clone(&failed);
            let ev = events.clone();
            let cancel = Arc::clone(&cancel);
            s.spawn(move || loop {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let job = {
                    let mut q = queue.lock().unwrap();
                    q.pop_front()
                };
                let Some(job) = job else { break };
                match fetch_tile(&job.url, timeout_secs, retries) {
                    Ok(bytes) => {
                        if let Err(e) = save_cached(&root, &layer, job.z, job.x, job.y, &bytes) {
                            failed.fetch_add(1, Ordering::Relaxed);
                            let _ = ev.send(DlEvent::Failed {
                                z: job.z, x: job.x, y: job.y,
                                reason: format!("写入失败: {e}"),
                            });
                        } else {
                            ok.fetch_add(1, Ordering::Relaxed);
                            let _ = ev.send(DlEvent::Done {
                                z: job.z, x: job.x, y: job.y,
                                bytes: bytes.len() as u64,
                            });
                        }
                    }
                    Err(reason) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                        let _ = ev.send(DlEvent::Failed { z: job.z, x: job.x, y: job.y, reason });
                    }
                }
            });
        }
    });

    let _ = events.send(DlEvent::Finished {
        ok: ok.load(Ordering::Relaxed),
        failed: failed.load(Ordering::Relaxed),
        skipped,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_sniff() {
        assert!(looks_like_image(&[0x89, b'P', b'N', b'G', 1, 2]));
        assert!(!looks_like_image(b"<html>denied</html>"));
    }
    #[test]
    fn rect_tiles_cover_beijing() {
        let (x0, y0, x1, y1) = tiles_in_rect(116.2, 39.8, 116.5, 40.0, 12);
        assert!(x1 >= x0 && y1 >= y0);
    }
}
