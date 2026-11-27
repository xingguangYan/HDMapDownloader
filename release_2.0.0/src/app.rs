//! App state and main logic
use crate::donate;
use crate::download::{self, DlEvent, EventTx, TileJob};
use crate::geo::LngLat;
use crate::mapview::{self, MapView, TileCache};
use crate::sources::SourceDoc;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

const DEFAULT_EXPORT_DIR: &str = "drive_export";

pub enum Tab { Map, Sources, Download, About, }

pub enum SelectMode { None, Rect, Polygon, }

pub enum ExportFormat { GeoTiff, Mbtiles, }

impl ExportFormat {
    pub fn label(&self) -> &str { match self { ExportFormat::GeoTiff => "GeoTIFF", ExportFormat::Mbtiles => "MBTiles", } }
}

pub enum Crs { Wgs84, WebMercator, Cgcs2000Geo, Cgcs2000Gk, }

impl Crs {
    pub fn label(&self) -> &str { match self { Crs::Wgs84 => "WGS84", Crs::WebMercator => "Web Mercator", Crs::Cgcs2000Geo => "CGCS2000", Crs::Cgcs2000Gk => "CGCS2000 GK", } }
    pub fn all() -> [Crs; 4] { [Crs::Wgs84, Crs::WebMercator, Crs::Cgcs2000Geo, Crs::Cgcs2000Gk] }
}

pub struct Selection {
    rect: Option<(LngLat, LngLat)>,
    polygon: Vec<LngLat>,
}

impl Selection {
    pub fn new_rect(a: LngLat, b: LngLat) -> Self { Self { rect: Some((a, b)), polygon: vec![] } }
    pub fn is_empty(&self) -> bool { self.rect.is_none() && self.polygon.len() < 3 }
    pub fn bounds(&self) -> Option<(LngLat, LngLat)> {
        if let Some((a, b)) = self.rect {
            return Some((LngLat::new(a.lng.min(b.lng), a.lat.min(b.lat)), LngLat::new(a.lng.max(b.lng), a.lat.max(b.lat))));
        }
        None
    }
    pub fn area_km2(&self) -> f64 { 0.0 }
}

pub struct Progress {
    pub running: bool,
    pub ok: usize,
    pub failed: usize,
    pub skipped: usize,
    pub log: Vec<String>,
}

impl Default for Progress {
    fn default() -> Self { Self { running: false, ok: 0, failed: 0, skipped: 0, log: Vec::new() } }
}

pub struct App {
    pub tab: Tab,
    pub cache_root: PathBuf,
    pub cache: TileCache,
    pub sources: Vec<crate::sources::SourceEntry>,
    pub source_enabled: BTreeMap<String, bool>,
    pub active_source: String,
    pub active_label_source: String,
    pub layer_imagery: bool,
    pub layer_labels: bool,
    pub select_mode: SelectMode,
    pub selection: Selection,
    pub view: MapView,
    pub min_zoom_dl: u8,
    pub max_zoom_dl: u8,
    pub concurrency: usize,
    pub resume: bool,
    pub progress: Progress,
    pub tile_status: BTreeMap<String, String>,
    pub status: String,
    pub show_alipay: bool,
    pub alipay_path: Option<PathBuf>,
    pub alipay_texture: Option<egui::TextureHandle>,
    pub export_format: ExportFormat,
    pub crs: Crs,
    pub export_name: String,
    pub export_dir: String,
    pub frames_drawn: u32,
    pub last_export: Option<String>,
    pub license_text: String,
    pub show_license_open: bool,
    pub show_license_for: Option<String>,
    pub events_rx: Option<crossbeam_channel::Receiver<DlEvent>>,
}

impl App {
    pub fn new(cache_root: PathBuf, donation_path: Option<PathBuf>) -> Self {
        let doc: SourceDoc = SourceDoc::default_doc();
        let mut enabled = BTreeMap::new();
        for s in &doc.sources { enabled.insert(s.id.clone(), s.default_on()); }
        let active = doc.sources.iter().find(|s| s.default_on() && s.layer == "imagery").map(|s| s.id.clone()).unwrap_or_else(|| "".to_string());
        let label = doc.sources.iter().find(|s| s.layer == "label").map(|s| s.id.clone()).unwrap_or_default();
        let path = donation_path.unwrap_or_else(|| PathBuf::from("resources/assets/donation.png"));
        Self {
            tab: Tab::Map,
            cache: TileCache::default(),
            sources: doc.sources,
            source_enabled: enabled,
            active_source: active.clone(),
            active_label_source: label.clone(),
            layer_imagery: true,
            layer_labels: true,
            select_mode: SelectMode::Rect,
            selection: Selection::new_rect(LngLat::new(116.4, 39.9), LngLat::new(117.0, 40.0)),
            view: MapView { center: LngLat::new(116.4, 39.9), zoom: 10, zoom_f: 10.0 },
            cache_root,
            min_zoom_dl: 10,
            max_zoom_dl: 12,
            concurrency: download::DEFAULT_CONCURRENCY,
            resume: true,
            progress: Progress::default(),
            tile_status: BTreeMap::new(),
            status: "就绪".to_string(),
            show_alipay: false,
            alipay_path: Some(path),
            alipay_texture: None,
            export_format: ExportFormat::GeoTiff,
            crs: Crs::Wgs84,
            export_name: "export".to_string(),
            export_dir: DEFAULT_EXPORT_DIR.to_string(),
            frames_drawn: 0,
            last_export: None,
            license_text: String::new(),
            show_license_open: false,
            show_license_for: None,
            events_rx: None,
        }
    }

    pub fn is_enabled(&self, id: &str) -> bool { *self.source_enabled.get(id).unwrap_or(&false) }

    pub fn build_jobs(&self, layer: &str, src: &crate::sources::SourceEntry) -> Vec<TileJob> {
        let Some((min, max)) = self.selection.bounds() else { return vec![]; };
        let mut jobs = Vec::new();
        for z in self.min_zoom_dl..=self.max_zoom_dl {
            if src.max_zoom != 0 && z > src.max_zoom { continue; }
            if src.min_zoom != 0 && z < src.min_zoom { continue; }
            let (x0, y0, x1, y1) = crate::download::tiles_in_rect(min.lng, min.lat, max.lng, max.lat, z);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    jobs.push(TileJob { z, x, y, url: src.url(z, x, y) });
                }
            }
        }
        jobs
    }

    pub fn start_download(&mut self, ctx: &egui::Context) {
        if self.progress.running { return; }
        if self.selection.is_empty() { self.status = "请先在地图上选择区域".to_string(); return; }
        let Some(src) = self.active_imagery() else { self.status = "未选择数据源".to_string(); return; };
        if !self.is_enabled(&src.id) { self.status = format!("未启用: {}", src.name); return; }
        let jobs = self.build_jobs("imagery", &src);
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = crossbeam_channel::unbounded();
        self.events_rx = Some(rx);
        std::thread::spawn(move || {
            crate::download::run_batch(jobs, self.cache_root.clone(), src.id.clone(), self.concurrency, download::DEFAULT_TIMEOUT_SECS, download::DEFAULT_RETRIES, true, cancel, tx);
        });
        self.progress = Progress { running: true, ok: 0, failed: 0, skipped: 0, log: Vec::new() };
        self.status = format!("开始下载 {} 块瓦片", jobs.len());
    }

    pub fn tick_events(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.events_rx {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    DlEvent::Started { total, skipped } => {
                        self.progress.ok = 0;
                        self.progress.failed = 0;
                        self.progress.skipped = skipped;
                        self.progress.log.push(format!("开始：{} 块，跳过 {}", total, skipped));
                    }
                    DlEvent::Done { z, x, y, bytes } => {
                        self.progress.ok += 1;
                        if self.progress.log.len() < 400 { self.progress.log.push(format!("OK z{z}/{x}/{y}")); }
                    }
                    DlEvent::Failed { z, x, y, reason } => {
                        self.progress.failed += 1;
                        if self.progress.log.len() < 400 { self.progress.log.push(format!("FAIL z{z}/{x}/{y}: {reason}")); }
                    }
                    DlEvent::Finished { ok, failed, skipped } => {
                        self.progress.running = false;
                        self.status = format!("下载结束: 成功 {}, 失败 {}, 跳过 {}", ok, failed, skipped);
                        self.events_rx = None;
                        self.progress = Progress { running: false, ok, failed, skipped, log: self.progress.log.clone() };
                    }
                }
            }
        }
    }

    pub fn cancel_download(&mut self) {
        // ... implement cancellation
    }

    pub fn active_imagery(&self) -> Option<crate::sources::SourceEntry> {
        self.sources.iter().find(|s| s.id == self.active_source).cloned()
    }

    pub fn source_by_id(&self, id: &str) -> Option<&crate::sources::SourceEntry> {
        self.sources.iter().find(|s| s.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn app_creation() {
        let app = App::new(PathBuf::from("test_cache"), None);
        assert!(app.source_enabled.contains_key("usgs_imagery"));
    }
}
