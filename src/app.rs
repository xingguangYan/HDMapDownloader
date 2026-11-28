//! 应用状态、瓦片纹理管理、下载调度、AI/矢量/影像检索（v4.0）
use crate::ai::{self, AiEvent, AiMsg};
use crate::adm::{self, AdmFeature};
use crate::download::{self, DlEvent, TileJob};
use crate::geo::LngLat;
use crate::i18n::Lang;
use crate::mapview::MapView;
use crate::sources::{self, CustomSource, SourceEntry, SourceKind};
use crate::stac::{self, Scene, SearchResult, SearchParams};
use crossbeam_channel::Receiver;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub enum BandEvent {
    Progress(usize, usize),
    Done(Result<Vec<PathBuf>, String>),
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

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub lang_en: bool,
    #[serde(default)]
    pub ai_base: String,
    #[serde(default)]
    pub ai_key: String,
    #[serde(default)]
    pub ai_model: String,
}

pub struct App {
    pub sources: Vec<SourceEntry>,
    pub custom: Vec<CustomSource>,
    pub active_source: String,
    pub api_key: String,
    /// 时间序列源的影像日期，如 2024-06-01
    pub date: String,

    pub lang: Lang,
    /// 0 关闭 1 框选 2 多边形 3 点选
    pub tool: u8,
    pub select_mode: bool, // 兼容：tool==1
    pub sel_bounds: Option<(LngLat, LngLat)>,
    drag_start: Option<egui::Pos2>,
    pub poly_pts: Vec<LngLat>,

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

    // ---- 行政矢量 ----
    pub adm_kw: String,
    pub adm_loading: bool,
    pub adm_polys: Vec<AdmFeature>,
    pub adm_cur: Option<usize>,
    adm_rx: Option<Receiver<Result<Vec<AdmFeature>, String>>>,

    // ---- STAC 影像 ----
    pub sc_collection: String,
    pub sc_date_from: String,
    pub sc_date_to: String,
    pub sc_cloud: i32,
    pub time_modal: bool,
    pub sc_loading: bool,
    pub scenes: Vec<Scene>,
    pub matched: u64,
    pub sc_page: u32,
    pub scenes_follow: bool, // 检索后定位到结果范围
    sc_rx: Option<Receiver<Result<SearchResult, String>>>,
    pub scene_sel_bands: Vec<Vec<String>>,
    pub band_dl_status: String,
    band_rx: Option<Receiver<BandEvent>>,

    // ---- AI ----
    pub ai_base: String,
    pub ai_key: String,
    pub ai_model: String,
    pub ai_input: String,
    pub ai_log: Vec<AiMsg>,
    ai_rx: Option<Receiver<AiEvent>>,
    pub ai_busy: bool,

    // ---- 自定义源 ----
    pub cs_open: bool,
    pub cs_name: String,
    pub cs_url: String,
    pub cs_max: u8,
    pub cs_key: String,
}

impl App {
    pub fn new(cache_root: PathBuf) -> Self {
        let sources = sources::catalog();
        let active = sources
            .iter()
            .find(|s| !s.requires_key && s.kind != SourceKind::Portal)
            .map(|s| s.id.to_string())
            .unwrap_or_default();
        let cfg: Config = std::fs::read(cache_root.join("config.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let lang = if cfg.lang_en { Lang::En } else { Lang::Zh };
        let custom: Vec<CustomSource> = std::fs::read(cache_root.join("custom.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            sources,
            active_source: active,
            api_key: String::new(),
            date: "2024-06-01".to_string(),
            lang,
            tool: 0,
            select_mode: false,
            sel_bounds: None,
            drag_start: None,
            poly_pts: Vec::new(),
            min_zoom_dl: 10,
            max_zoom_dl: 12,
            view: MapView { center: LngLat::new(116.4, 39.9), zoom: 6 },
            cache_root: cache_root.clone(),
            textures: BTreeMap::new(),
            fetching: HashSet::new(),
            progress: Progress::default(),
            events_rx: None,
            cancel: None,
            status: "就绪。选择工具（框选/多边形/点选）划定区域，或用行政矢量 / AI 帮你选。".to_string(),
            last_zip: None,
            show_alipay: false,
            alipay_texture: None,
            frames: 0,
            adm_kw: String::new(),
            adm_loading: false,
            adm_polys: Vec::new(),
            adm_cur: None,
            adm_rx: None,
            sc_collection: "sentinel-2-l2a".to_string(),
            sc_date_from: "2024-06-01".to_string(),
            sc_date_to: "2024-06-30".to_string(),
            sc_cloud: 20,
            time_modal: false,
            sc_loading: false,
            scenes: Vec::new(),
            matched: 0,
            sc_page: 1,
            scenes_follow: false,
            sc_rx: None,
            scene_sel_bands: Vec::new(),
            band_dl_status: String::new(),
            band_rx: None,
            ai_base: if cfg.ai_base.is_empty() { "https://api.openai.com/v1".into() } else { cfg.ai_base },
            ai_key: cfg.ai_key,
            ai_model: if cfg.ai_model.is_empty() { "gpt-4o-mini".into() } else { cfg.ai_model },
            ai_input: String::new(),
            ai_log: vec![AiMsg { role: "sys".into(), text: "填入兼容 OpenAI 的 Base URL / Key / 模型，Key 仅保存在本机。".into() }],
            ai_rx: None,
            ai_busy: false,
            cs_open: false,
            cs_name: String::new(),
            cs_url: String::new(),
            cs_max: 18,
            cs_key: String::new(),
            custom,
        }
    }

    pub fn tr(&self, zh: &str, en: &str) -> String {
        crate::i18n::t(&self.lang, zh, en).to_string()
    }

    pub fn save_config(&self) {
        let cfg = Config {
            lang_en: matches!(self.lang, Lang::En),
            ai_base: self.ai_base.clone(),
            ai_key: self.ai_key.clone(),
            ai_model: self.ai_model.clone(),
        };
        let _ = std::fs::write(self.cache_root.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap_or_default());
        let _ = std::fs::write(self.cache_root.join("custom.json"), serde_json::to_vec_pretty(&self.custom).unwrap_or_default());
    }

    // ---------- 活动源（静态目录或自定义） ----------
    pub fn active(&self) -> Option<&SourceEntry> {
        self.sources.iter().find(|s| s.id == self.active_source)
    }
    pub fn active_custom(&self) -> Option<&CustomSource> {
        self.custom.iter().find(|s| s.id == self.active_source)
    }
    pub fn active_name(&self) -> String {
        if let Some(c) = self.active_custom() {
            return c.name.clone();
        }
        self.active().map(|s| s.name.to_string()).unwrap_or_else(|| "(未选择)".into())
    }
    pub fn active_max_zoom(&self) -> u8 {
        if let Some(c) = self.active_custom() {
            return c.max_zoom;
        }
        self.active().map(|s| s.max_zoom).unwrap_or(19)
    }
    pub fn active_needs_key(&self) -> bool {
        if let Some(c) = self.active_custom() {
            return c.requires_key;
        }
        self.active().map(|s| s.requires_key).unwrap_or(false)
    }
    pub fn active_kind(&self) -> SourceKind {
        if self.active_custom().is_some() {
            return SourceKind::Xyz;
        }
        self.active().map(|s| s.kind).unwrap_or(SourceKind::Xyz)
    }
    pub fn tile_url_for(&self, z: u8, x: i32, y: i32) -> String {
        let key = self.api_key.trim().to_string();
        let date = self.date.trim().to_string();
        if let Some(c) = self.active_custom() {
            return c.tile_url(z, x, y, &key);
        }
        if let Some(s) = self.active() {
            return s.tile_url(z, x, y, &key, &date);
        }
        String::new()
    }

    pub fn estimate(&self) -> usize {
        let Some((min, max)) = self.sel_bounds else { return 0 };
        let max_z = self.effective_max_zoom();
        let mut total = 0usize;
        for z in self.min_zoom_dl..=max_z {
            let (x0, y0, x1, y1) = download::tiles_in_rect(min.lng, min.lat, max.lng, max.lat, z);
            let cols = (x1 - x0 + 1).max(0) as usize;
            let rows = (y1 - y0 + 1).max(0) as usize;
            total = total.saturating_add(cols.saturating_mul(rows));
        }
        total
    }
    fn effective_max_zoom(&self) -> u8 {
        self.max_zoom_dl.min(self.active_max_zoom().max(1))
    }

    pub fn build_jobs(&self) -> Vec<TileJob> {
        let Some((min, max)) = self.sel_bounds else { return vec![] };
        let max_z = self.effective_max_zoom();
        let mut jobs = Vec::new();
        for z in self.min_zoom_dl..=max_z {
            let (x0, y0, x1, y1) = download::tiles_in_rect(min.lng, min.lat, max.lng, max.lat, z);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    jobs.push(TileJob { z, x, y, url: self.tile_url_for(z, x, y) });
                }
            }
        }
        jobs
    }

    pub fn start_download(&mut self, ctx: &egui::Context) {
        if self.progress.running {
            return;
        }
        if self.active().is_none() && self.active_custom().is_none() {
            self.status = self.tr("未选择数据源", "No source selected");
            return;
        }
        if self.active_kind() == SourceKind::Portal {
            self.status = self.tr("「数据门户」请在左侧打开主页注册，无法直接下载瓦片", "Portal source: open its homepage to register");
            return;
        }
        if self.active_needs_key() && self.api_key.trim().is_empty() {
            self.status = self.tr("该数据源需要密钥，请在左侧填入 tk / API Key", "This source needs a key; fill it in the left panel");
            return;
        }
        if self.sel_bounds.is_none() {
            self.status = self.tr("请先用框选/多边形/点选或行政矢量划定区域", "Select a region first (box/polygon/point/admin vector)");
            return;
        }
        let jobs = self.build_jobs();
        let total = jobs.len();
        if jobs.is_empty() {
            self.status = self.tr("没有可下载的瓦片（检查选区与缩放范围）", "No tiles to download (check region/zoom)");
            return;
        }
        let (tx, rx) = crossbeam_channel::unbounded();
        self.events_rx = Some(rx);
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = Some(cancel.clone());
        let root = self.cache_root.clone();
        let layer = self.active_source.clone();
        let ctx2 = ctx.clone();
        std::thread::spawn(move || {
            download::run_batch(jobs, root, layer, download::DEFAULT_CONCURRENCY, download::DEFAULT_TIMEOUT_SECS, download::DEFAULT_RETRIES, true, cancel, tx);
            ctx2.request_repaint();
        });
        self.progress = Progress { running: true, total, ..Progress::default() };
        self.status = format!("{}：{} {}", self.tr("开始下载", "Downloading"), total, self.tr("块瓦片", "tiles"));
    }

    pub fn cancel_download(&mut self) {
        if let Some(c) = self.cancel.take() {
            c.store(true, Ordering::Relaxed);
            self.status = self.tr("已请求取消", "Cancel requested");
        }
    }

    // ---------- 事件泵 ----------
    pub fn tick_events(&mut self) {
        let Some(rx) = self.events_rx.take() else { return };
        let mut keep = true;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                DlEvent::Started { total, skipped } => {
                    self.progress.total = total;
                    self.progress.skipped = skipped;
                    self.push_log(format!("{} {} {}", self.tr("开始", "start"), total, self.tr("块", "tiles")));
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
                    self.status = format!("{}：{} {} {} {} {}", self.tr("下载完成", "Done"),
                        self.tr("成功", "ok"), ok, self.tr("失败", "failed"), failed, self.tr("跳过", "skipped"));
                    keep = false;
                    self.cancel = None;
                    self.push_log("—— 结束 ——".into());
                }
            }
        }
        if keep {
            self.events_rx = Some(rx);
        }
        self.tick_adm();
        self.tick_stac();
        self.tick_band();
        self.tick_ai();
    }

    fn push_log(&mut self, s: String) {
        if self.progress.log.len() < 300 {
            self.progress.log.push(s);
        }
    }

    pub fn export_zip(&self) -> Result<PathBuf, String> {
        let dir = PathBuf::from("exports");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let fname = format!("HDMap_{}_z{}-{}.zip", self.active_source, self.min_zoom_dl, self.effective_max_zoom());
        let out = dir.join(fname);
        let _n = crate::export::zip_tiles(&self.cache_root, &self.active_source, &out)?;
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
        if self.active().is_none() && self.active_custom().is_none() {
            return;
        }
        if self.active_kind() == SourceKind::Portal {
            return;
        }
        if self.active_needs_key() && self.api_key.trim().is_empty() {
            return;
        }
        let (x0, y0, x1, y1) = crate::mapview::visible_tiles(&self.view, rect);
        let z = self.view.zoom;
        let mut spawned = 0;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if self.fetching.contains(&(z, x, y)) { continue; }
                let path = download::tile_path(&self.cache_root, &self.active_source, z, x, y);
                if path.exists() { continue; }
                self.fetching.insert((z, x, y));
                let url = self.tile_url_for(z, x, y);
                let layer = self.active_source.clone();
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

    // ---------- 选区工具 ----------
    pub fn set_tool(&mut self, t: u8) {
        if self.tool == t {
            self.tool = 0;
        } else {
            self.tool = t;
            if t != 2 {
                self.poly_pts.clear();
            }
        }
        self.select_mode = self.tool == 1;
    }

    pub fn sel_bbox(&self) -> Option<(f64, f64, f64, f64)> {
        let (min, max) = self.sel_bounds?;
        Some((min.lng.min(max.lng), min.lat.min(max.lat), min.lng.max(max.lng), min.lat.max(max.lat)))
    }

    pub fn clear_sel(&mut self) {
        self.sel_bounds = None;
        self.poly_pts.clear();
        self.adm_cur = None;
        self.status = self.tr("选区已清除", "Selection cleared");
    }

    pub fn set_sel_bbox(&mut self, b: (f64, f64, f64, f64)) {
        self.sel_bounds = Some((LngLat::new(b.0, b.1), LngLat::new(b.2, b.3)));
    }

    // ---------- 行政矢量 ----------
    pub fn adm_find(&mut self) {
        let kw = self.adm_kw.trim().to_string();
        if kw.is_empty() || self.adm_loading {
            return;
        }
        self.adm_loading = true;
        self.status = self.tr("检索行政区矢量中 ...", "Searching admin vectors ...");
        let (tx, rx) = crossbeam_channel::unbounded();
        self.adm_rx = Some(rx);
        std::thread::spawn(move || {
            let r = adm::find_region(&kw);
            let _ = tx.send(r);
        });
    }

    fn tick_adm(&mut self) {
        let Some(rx) = self.adm_rx.take() else { return };
        if let Ok(r) = rx.try_recv() {
            self.adm_loading = false;
            match r {
                Ok(feats) => {
                    if let Some(f) = feats.first() {
                        let (w, s, e, n) = f.bbox;
                        self.view.center = LngLat::new((w + e) / 2.0, (s + n) / 2.0);
                        self.view.zoom = zoom_for_bbox((w, s, e, n));
                    }
                    self.adm_polys = feats;
                    self.adm_cur = None;
                    self.status = format!("{} ({})", self.tr("矢量已载入，点击地图要素可设为选区", "Vectors loaded; click a feature to select"), self.adm_kw);
                }
                Err(e) => {
                    self.status = format!("{}: {}", self.tr("行政区检索失败", "Admin search failed"), e);
                }
            }
        } else {
            self.adm_rx = Some(rx);
        }
    }

    /// 地图点击：选中命中的行政区（工具关闭时）
    pub fn adm_hit_select(&mut self, ll: &LngLat) -> bool {
        let pt = (ll.lng, ll.lat);
        let mut best: Option<(usize, f64)> = None;
        for (i, f) in self.adm_polys.iter().enumerate() {
            if !adm::hit(pt, &f.outer_rings) {
                continue;
            }
            let (w, s, e, n) = f.bbox;
            let area = (e - w) * (n - s);
            if best.map(|(_, a)| area < a).unwrap_or(true) {
                best = Some((i, area));
            }
        }
        if let Some((i, _)) = best {
            let f = self.adm_polys[i].clone();
            self.set_sel_bbox(f.bbox);
            self.adm_cur = Some(i);
            self.status = format!("{}：{}", self.tr("已选行政区", "Admin region selected"), f.name);
            true
        } else {
            false
        }
    }

    pub fn adm_save_geojson(&self) -> Result<PathBuf, String> {
        let Some(i) = self.adm_cur else { return Err(self.tr("请先点击选中一个行政区", "Click an admin feature first").into()) };
        let f = &self.adm_polys[i];
        let dir = PathBuf::from("exports");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let rings: Vec<Vec<Vec<f64>>> = f.outer_rings.iter().map(|r| r.iter().map(|(a, b)| vec![*a, *b]).collect()).collect();
        let gj = serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {"adcode": f.code, "name": f.name, "level": f.level},
                "geometry": {"type": "MultiPolygon", "coordinates": [rings.iter().map(|r| vec![r.clone()]).collect::<Vec<_>>()]}
            }]
        });
        let path = dir.join(format!("adm_{}_{}.geojson", f.code, f.name));
        std::fs::write(&path, serde_json::to_string_pretty(&gj).unwrap_or_default()).map_err(|e| e.to_string())?;
        Ok(path)
    }

    // ---------- STAC 检索 ----------
    pub fn stac_search(&mut self, page: u32) {
        let Some(b) = self.sel_bbox() else {
            self.status = self.tr("请先划定区域再检索影像", "Select a region before searching imagery");
            return;
        };
        if self.sc_loading {
            return;
        }
        self.time_modal = false;
        self.sc_loading = true;
        self.sc_page = page;
        self.status = self.tr("检索影像场景中 ...", "Searching scenes ...");
        let p = SearchParams {
            collection: self.sc_collection.clone(),
            bbox: b,
            date_from: self.sc_date_from.clone(),
            date_to: self.sc_date_to.clone(),
            cloud_max: Some(self.sc_cloud),
            page,
        };
        let (tx, rx) = crossbeam_channel::unbounded();
        self.sc_rx = Some(rx);
        std::thread::spawn(move || {
            let r = stac::search(&p);
            let _ = tx.send(r);
        });
    }

    fn tick_stac(&mut self) {
        let Some(rx) = self.sc_rx.take() else { return };
        if let Ok(r) = rx.try_recv() {
            self.sc_loading = false;
            match r {
                Ok(res) => {
                    self.status = format!("{}：{}/{} (p{})", self.tr("检索完成", "Search done"), self.scenes.len() + res.scenes.len(), res.matched, self.sc_page);
                    self.scenes_follow = self.sc_page == 1 && !res.scenes.is_empty();
                    if self.scenes_follow {
                        let b = res.scenes[0].bbox;
                        self.view.center = LngLat::new((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0);
                        if (b.2 - b.0).abs() > 0.001 {
                            self.view.zoom = zoom_for_bbox(b).max(4).min(14);
                        }
                    }
                    if self.sc_page == 1 {
                        self.scenes = res.scenes;
                    } else {
                        self.scenes.extend(res.scenes);
                    }
                    self.matched = res.matched;
                    self.scene_sel_bands = vec![Vec::new(); self.scenes.len()];
                }
                Err(e) => {
                    self.status = format!("{}: {}", self.tr("检索失败", "Search failed"), e);
                }
            }
        } else {
            self.sc_rx = Some(rx);
        }
    }

    pub fn scene_download(&mut self, idx: usize) {
        if self.band_rx.is_some() {
            return;
        }
        let Some(sc) = self.scenes.get(idx) else { return };
        let names = self.scene_sel_bands.get(idx).cloned().unwrap_or_default();
        if names.is_empty() {
            self.band_dl_status = self.tr("请先勾选要下载的波段", "Check at least one band first");
            return;
        }
        let dir = PathBuf::from("exports");
        let scene = sc.clone();
        let (tx, rx) = crossbeam_channel::unbounded();
        self.band_rx = Some(rx);
        self.band_dl_status = self.tr("波段下载中 ...", "Downloading bands ...");
        std::thread::spawn(move || {
            let total = names.len();
            let names2 = names.clone();
            let _ = tx.send(BandEvent::Progress(0, total));
            let r = stac::download_bands(&scene, &names2, &dir);
            let _ = tx.send(BandEvent::Done(r));
        });
    }

    fn tick_band(&mut self) {
        let Some(rx) = self.band_rx.take() else { return };
        if let Ok(ev) = rx.try_recv() {
            match ev {
                BandEvent::Progress(d, t) => {
                    self.band_dl_status = format!("{} {}/{}", self.tr("波段下载中", "Downloading bands"), d, t);
                    self.band_rx = Some(rx);
                }
                BandEvent::Done(r) => match r {
                    Ok(paths) => {
                        self.band_dl_status = format!("{}：{} {}", self.tr("波段已保存到 exports 目录", "Bands saved to exports"), paths.len(), self.tr("个文件", "files"));
                    }
                    Err(e) => {
                        self.band_dl_status = format!("{}: {}", self.tr("波段下载失败", "Band download failed"), e);
                    }
                },
            }
        } else {
            self.band_rx = Some(rx);
        }
    }

    // ---------- AI ----------
    pub fn ai_save_cfg(&mut self) {
        self.save_config();
        self.ai_log.push(AiMsg { role: "sys".into(), text: self.tr("AI 设置已保存（本机）", "AI settings saved locally").clone() });
    }

    pub fn ai_send(&mut self) {
        let q = self.ai_input.trim().to_string();
        if q.is_empty() || self.ai_busy {
            return;
        }
        if self.ai_key.trim().is_empty() {
            self.ai_log.push(AiMsg { role: "sys".into(), text: self.tr("请先填写并保存 API Key", "Save your API key first") });
            return;
        }
        self.ai_log.push(AiMsg { role: "user".into(), text: q.clone() });
        self.ai_input.clear();
        self.ai_log.push(AiMsg { role: "sys".into(), text: self.tr("已发给 AI，等待返回 ...", "Sent to AI, waiting ...") });
        self.ai_busy = true;
        let (tx, rx) = crossbeam_channel::unbounded();
        self.ai_rx = Some(rx);
        let snap = ai::PlanSnap {
            bbox: self.sel_bbox(),
            date_from: self.sc_date_from.clone(),
            date_to: self.sc_date_to.clone(),
            cloud: self.sc_cloud,
            collection: self.sc_collection.clone(),
            export_dir: PathBuf::from("exports"),
        };
        let base = self.ai_base.clone();
        let key = self.ai_key.clone();
        let model = self.ai_model.clone();
        std::thread::spawn(move || {
            ai::run_plan(base, key, model, q, snap, tx);
        });
    }

    fn tick_ai(&mut self) {
        let Some(rx) = self.ai_rx.take() else { return };
        let mut more = true;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                AiEvent::Log(m) => self.ai_log.push(m),
                AiEvent::SetTime(a, b) => {
                    self.sc_date_from = a;
                    self.sc_date_to = b;
                }
                AiEvent::SetCloud(c) => self.sc_cloud = c.clamp(0, 100),
                AiEvent::SetCollection(c) => self.sc_collection = c,
                AiEvent::SetBounds(b, name) => {
                    self.set_sel_bbox(b);
                    self.status = format!("{}：{}", self.tr("AI 已选区", "AI selected region"), name);
                }
                AiEvent::Scenes(r) => {
                    self.scenes = r.scenes;
                    self.matched = r.matched;
                    self.scene_sel_bands = vec![Vec::new(); self.scenes.len()];
                    self.status = format!("{}：{}", self.tr("AI 检索得到场景", "AI found scenes"), self.scenes.len());
                }
                AiEvent::BandDone(paths) => {
                    self.status = format!("{}：{} {}", self.tr("AI 波段已保存", "AI saved bands"), paths.len(), self.tr("个文件", "files"));
                }
                AiEvent::Err(e) => self.ai_log.push(AiMsg { role: "sys".into(), text: e }),
                AiEvent::Finished => {
                    self.ai_busy = false;
                    more = false;
                }
            }
        }
        if more {
            self.ai_rx = Some(rx);
        }
    }

    // ---------- 自定义源 ----------
    pub fn cs_add(&mut self) {
        let name = self.cs_name.trim().to_string();
        let url = self.cs_url.trim().to_string();
        if name.is_empty() || !url.contains("{z}") || !url.contains("{x}") || !url.contains("{y}") {
            self.status = self.tr("需要名称和含 {z}/{x}/{y} 的 URL 模板", "Need a name and URL with {z}/{x}/{y}");
            return;
        }
        let id = format!("custom_{}", self.custom.len() + 1);
        let s = CustomSource {
            id: id.clone(),
            name,
            group: "自定义".into(),
            url_template: url,
            max_zoom: self.cs_max.min(22),
            requires_key: !self.cs_key.trim().is_empty(),
            note: "用户接入（登录后）· 用户负责遵守该源条款".into(),
        };
        self.custom.push(s);
        if !self.cs_key.trim().is_empty() {
            self.api_key = self.cs_key.trim().to_string();
        }
        self.save_config();
        self.active_source = id;
        self.cs_open = false;
        self.cs_name.clear();
        self.cs_url.clear();
        self.cs_key.clear();
        self.on_source_changed();
        self.status = self.tr("自定义源已接入并启用", "Custom source connected");
    }
}

fn zoom_for_bbox((w, s, e, n): (f64, f64, f64, f64)) -> u8 {
    let dx = (e - w).abs().max(0.0001);
    let dy = (n - s).abs().max(0.0001);
    let d = dx.max(dy);
    if d > 60.0 { 2 } else if d > 20.0 { 4 } else if d > 8.0 { 5 } else if d > 3.0 { 7 } else if d > 1.0 { 9 } else if d > 0.3 { 11 } else if d > 0.08 { 13 } else if d > 0.02 { 15 } else { 17 }
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
        assert!(app.ai_base.contains("openai"));
    }
    #[test]
    fn custom_tile_url() {
        let c = CustomSource {
            id: "custom_1".into(), name: "t".into(), group: "g".into(),
            url_template: "https://x.com/{z}/{x}/{y}.png?key={key}".into(),
            max_zoom: 18, requires_key: false, note: "".into(),
        };
        let u = c.tile_url(3, 2, 1, "abc");
        assert!(u.contains("/3/2/1.png"));
        assert!(u.ends_with("key=abc"));
    }
}