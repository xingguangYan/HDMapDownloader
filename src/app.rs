//! 应用状态、瓦片纹理管理、下载调度
use crate::download::{self, DlEvent, TileJob};
use crate::geo::LngLat;
use crate::mapview::MapView;
use crate::sources::{self, SourceEntry, SourceKind};
use crossbeam_channel::Receiver;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct App {
    pub sources: Vec<SourceEntry>,
    pub active_source: String,
    pub api_key: String,
    /// 时间序列源的影像日期，如 2024-06-01
    pub date: String,

    pub select_mode: bool,
    pub sel_bounds: Option<(LngLat, LngLat)>,
    drag_start: Option<egui::Pos2>,

    pub min_zoom_dl: u8,
    pub max_zoom_dl: u8,

    pub view: MapView,
    pub cache_root: PathBuf,

    pub textures: BTreeMap<String, egui::TextureHandle>,
    fetching: HashSet<(u8, i32, i32)>,

    pub progress: Progress,
    pub events_rx: Option<Receiver<DlEvent>>,
    cancel: Option<Arc<AtomicBool>>,
    pub status: String,

    pub last_zip: Option<String>,

    pub show_alipay: bool,
    pub alipay_texture: Option<egui::TextureHandle>,
    pub frames: u32,
}

pub struct Progress {
    pub running: bool,
    pub total: usize,
    pub ok: usize,
    pub failed: usize,
    pub skipped: usize,
    pub log: Vec<String>,
}

impl Default for Progress {
    fn default() -> Self {
        Self { running: false, total: 0, ok: 0, failed: 0, skipped: 0, log: Vec::new() }
    }
}

impl App {
    pub fn new(cache_root: PathBuf) -> Self {
        let sources = sources::catalog();
        let active = sources
            .iter()
            .find(|s| !s.requires_key && s.kind != SourceKind::Portal)
            .map(|s| s.id.to_string())
            .unwrap_or_default();
        Self {
            sources,
            active_source: active,
            api_key: String::new(),
            date: "2024-06-01".to_string(),
            select_mode: false,
            sel_bounds: None,
            drag_start: None,
            min_zoom_dl: 10,
            max_zoom_dl: 12,
            view: MapView { center: LngLat::new(116.4, 39.9), zoom: 6 },
            cache_root,
            textures: BTreeMap::new(),
            fetching: HashSet::new(),
            progress: Progress::default(),
            events_rx: None,
            cancel: None,
            status: "就绪。先开左侧「框选模式」，在地图上拖出矩形，再开始下载。".to_string(),
            last_zip: None,
            show_alipay: false,
            alipay_texture: None,
            frames: 0,
        }
    }

    pub fn active(&self) -> Option<&SourceEntry> {
        self.sources.iter().find(|s| s.id == self.active_source)
    }

    pub fn estimate(&self) -> usize {
        let Some((min, max)) = self.sel_bounds else { return 0 };
        let Some(src) = self.active() else { return 0 };
        let mut total = 0usize;
        for z in self.min_zoom_dl..=self.max_zoom_dl {
            if src.max_zoom != 0 && z > src.max_zoom { continue; }
            let (x0, y0, x1, y1) = download::tiles_in_rect(min.lng, min.lat, max.lng, max.lat, z);
            let cols = (x1 - x0 + 1).max(0) as usize;
            let rows = (y1 - y0 + 1).max(0) as usize;
            total = total.saturating_add(cols.saturating_mul(rows));
        }
        total
    }

    pub fn build_jobs(&self) -> Vec<TileJob> {
        let Some(src) = self.active() else { return vec![] };
        let Some((min, max)) = self.sel_bounds else { return vec![] };
        let key = self.api_key.trim().to_string();
        let date = self.date.trim().to_string();
        let mut jobs = Vec::new();
        for z in self.min_zoom_dl..=self.max_zoom_dl {
            if src.max_zoom != 0 && z > src.max_zoom { continue; }
            let (x0, y0, x1, y1) = download::tiles_in_rect(min.lng, min.lat, max.lng, max.lat, z);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    jobs.push(TileJob { z, x, y, url: src.tile_url(z, x, y, &key, &date) });
                }
            }
        }
        jobs
    }

    pub fn start_download(&mut self, ctx: &egui::Context) {
        if self.progress.running {
            return;
        }
        let Some(src) = self.active().cloned() else {
            self.status = "未选择数据源".into();
            return;
        };
        if src.kind == SourceKind::Portal {
            self.status = format!("「{}」是数据门户，请在左侧「数据门户」打开主页注册，无法直接下载瓦片", src.name);
            return;
        }
        if src.requires_key && self.api_key.trim().is_empty() {
            self.status = format!("「{}」需要密钥，请在左侧「密钥」填入 tk / API Key", src.name);
            return;
        }
        if self.sel_bounds.is_none() {
            self.status = "请先开启「框选模式」，在地图上拖出矩形".into();
            return;
        }
        let jobs = self.build_jobs();
        let total = jobs.len();
        if jobs.is_empty() {
            self.status = "没有可下载的瓦片（检查框选与缩放范围）".into();
            return;
        }
        let (tx, rx) = crossbeam_channel::unbounded();
        self.events_rx = Some(rx);
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = Some(cancel.clone());
        let root = self.cache_root.clone();
        let layer = src.id.to_string();
        let ctx2 = ctx.clone();
        std::thread::spawn(move || {
            download::run_batch(jobs, root, layer, download::DEFAULT_CONCURRENCY, download::DEFAULT_TIMEOUT_SECS, download::DEFAULT_RETRIES, true, cancel, tx);
            ctx2.request_repaint();
        });
        self.progress = Progress { running: true, total, ..Progress::default() };
        self.status = format!("开始下载：{} 块瓦片（{}）", total, src.name);
    }

    pub fn cancel_download(&mut self) {
        if let Some(c) = self.cancel.take() {
            c.store(true, Ordering::Relaxed);
            self.status = "已请求取消".into();
        }
    }

    pub fn tick_events(&mut self) {
        let Some(rx) = self.events_rx.take() else { return };
        let mut keep = true;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                DlEvent::Started { total, skipped } => {
                    self.progress.total = total;
                    self.progress.skipped = skipped;
                    self.push_log(format!("开始：{} 块，跳过 {} 块", total, skipped));
                }
                DlEvent::Done { z, x, y, .. } => {
                    self.progress.ok += 1;
                    self.push_log(format!("OK z{}/{}/{}", z, x, y));
                }
                DlEvent::Failed { z, x, y, reason } => {
                    self.progress.failed += 1;
                    self.push_log(format!("FAIL z{}/{}/{}: {}", z, x, y, reason));
                }
                DlEvent::Finished { ok, failed, skipped } => {
                    self.progress.running = false;
                    self.progress.ok = ok;
                    self.progress.failed = failed;
                    self.progress.skipped = skipped;
                    self.status = format!("下载完成：成功 {}，失败 {}，跳过 {}", ok, failed, skipped);
                    keep = false;
                    self.cancel = None;
                    self.push_log("—— 结束 ——".into());
                }
            }
        }
        if keep {
            self.events_rx = Some(rx);
        }
    }

    fn push_log(&mut self, s: String) {
        if self.progress.log.len() < 300 {
            self.progress.log.push(s);
        }
    }

    pub fn export_zip(&self) -> Result<PathBuf, String> {
        let Some(src) = self.active() else { return Err("未选择数据源".into()) };
        let dir = PathBuf::from("exports");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let fname = format!("HDMap_{}_z{}-{}.zip", src.id, self.min_zoom_dl, self.max_zoom_dl);
        let out = dir.join(fname);
        let _n = crate::export::zip_tiles(&self.cache_root, &src.id, &out)?;
        Ok(out)
    }

    /// 从磁盘缓存取某块瓦片的纹理（没有就返回 None，由后台抓取补上）
    pub fn get_tile_texture(&mut self, ctx: &egui::Context, z: u8, x: i32, y: i32) -> Option<egui::TextureHandle> {
        let key = format!("{}_{}_{}_{}", self.active_source, z, x, y);
        if let Some(t) = self.textures.get(&key) {
            return Some(t.clone());
        }
        let path = download::tile_path(&self.cache_root, &self.active_source, z, x, y);
        let Ok(bytes) = std::fs::read(&path) else { return None };
        let Ok(img) = image::load_from_memory(&bytes) else { return None };
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw());
        let tex = ctx.load_texture(key.clone(), ci, egui::TextureOptions::LINEAR);
        self.textures.insert(key, tex.clone());
        Some(tex)
    }

    /// 为可见但尚未下载的瓦片启动后台抓取（每帧最多 8 个）
    pub fn fetch_visible(&mut self, ctx: &egui::Context, rect: &egui::Rect) {
        let Some(src) = self.active().cloned() else { return };
        if src.kind == SourceKind::Portal { return; }
        if src.requires_key && self.api_key.trim().is_empty() { return; }
        let (x0, y0, x1, y1) = crate::mapview::visible_tiles(&self.view, rect);
        let z = self.view.zoom;
        let key = self.api_key.trim().to_string();
        let date = self.date.trim().to_string();
        let mut spawned = 0;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if self.fetching.contains(&(z, x, y)) { continue; }
                let path = download::tile_path(&self.cache_root, &src.id, z, x, y);
                if path.exists() { continue; }
                self.fetching.insert((z, x, y));
                let url = src.tile_url(z, x, y, &key, &date);
                let layer = src.id.to_string();
                let root = self.cache_root.clone();
                let ctx2 = ctx.clone();
                std::thread::spawn(move || {
                    if let Ok(bytes) = download::fetch_tile(&url, 30, 2) {
                        let _ = download::save_cached(&root, &layer, z, x, y, &bytes);
                    }
                    ctx2.request_repaint();
                });
                spawned += 1;
                if spawned >= 8 {
                    return;
                }
            }
        }
    }

    pub fn clear_fetching(&mut self) {
        self.fetching.clear();
    }

    pub fn on_source_changed(&mut self) {
        self.clear_fetching();
        self.textures.clear();
    }

    pub fn drag_start(&self) -> Option<egui::Pos2> {
        self.drag_start
    }
    pub fn set_drag_start(&mut self, p: Option<egui::Pos2>) {
        self.drag_start = p;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn app_creation() {
        let app = App::new(PathBuf::from("test_cache"));
        assert!(!app.sources.is_empty());
        assert!(!app.active_source.is_empty());
        assert!(app.estimate() == 0); // 还没框选
    }
}
