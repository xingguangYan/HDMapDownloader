//! Map view: viewport, rendering, and tile caching
use crate::download::tile_path;
use crate::geo::LngLat;
use crate::sources::SourceEntry;
use eframe::egui;
use std::path::PathBuf;
use std::collections::HashMap;

const TILE_SIZE: usize = 256;

#[derive(Clone, Copy, Debug, Default)]
pub struct MapView {
    pub center: LngLat,
    pub zoom: u8,
    pub zoom_f: f32,
}

pub fn zoom_in(view: &mut MapView) {
    if view.zoom < 19 { VIEW.zoom += 1; view.zoom_f = view.zoom as f32; }
}

pub fn zoom_out(view: &mut MapView) {
    if view.zoom > 0 { view.zoom -= 1; view.zoom_f = view.zoom as f32; }
}

pub fn pan_by_pixels(view: &mut MapView, dx: f32, dy: f32) {
    let res = crate::geo::resolution(view.zoom, view.center.lat) / 1000.0;
    view.center.lng -= (dx * 0.001 / res) as f64 * 180.0;
    view.center.lat -= (dy * 0.001 / res) as f64 * 180.0 * 0.95;
}

pub fn zoom_at(view: &mut MapView, rect: egui::Rect, pos: egui::Pos2, scroll: f32) {
    let mut z = view.zoom as f32;
    z = z.clamp(0.0, 19.0) + scroll * 0.1;
    view.zoom = z as u8;
    view.zoom_f = z;
}

pub struct VisibleTile {
    pub z: u8,
    pub x: i32,
    pub y: i32,
    pub screen: egui::Rect,
}

pub fn visible_tiles(view: &MapView, rect: egui::Rect, z: u8) -> Vec<VisibleTile> {
    let n = 1 << z;
    let mut tiles = Vec::new();
    let min_x = (view.center.lng + 180.0) / 360.0 * n as f64;
    let max_x = (min_x + rect.width() / 256.0 * n as f32).ceil() as i32;
    let min_y = 0;
    let max_y = (min_x + rect.height() / 256.0 * n as f32).ceil() as i32;
    for y in min_y..=max_y.min(n as i32 - 1) {
        for x in min_x.floor() as i32..=max_x.min(n as i32 - 1) {
            let screen = egui::Rect::from_min_max(
                egui::pos2((x - min_x.floor() as i32) * 256.0, (y - min_y) * 256.0),
                egui::pos2((x - min_x.floor() as i32 + 1) * 256.0, (y - min_y + 1) * 256.0),
            );
            tiles.push(VisibleTile { z, x, y, screen });
        }
    }
    tiles
}

pub struct TileCache {
    textures: HashMap<String, egui::TextureHandle>,
}

impl Default for TileCache {
    fn default() -> Self { Self { textures: HashMap::new() } }
}

impl TileCache {
    pub fn get_or_load(&mut self, ctx: &egui::Context, root: &PathBuf, src_id: &str, z: u8, x: i32, y: i32) -> Option<egui::TextureHandle> {
        let key = format!("{}_{}_{}_{}", src_id, z, x, y);
        if let Some(tex) = self.textures.get(&key) { return Some(tex.clone()); }
        let path = tile_path(root, src_id, z, x, y);
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(img) = image::load_from_memory(&bytes) {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba);
                let tex = ctx.load_texture(key, ci, egui::TextureOptions::LINEAR);
                self.textures.insert(key, tex.clone());
                return Some(tex);
            }
        }
        None
    }
    pub fn clear(&mut self) { self.textures.clear(); }
}

pub fn missing_tile_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode { egui::Color32::DARK_GRAY } else { egui::Color32::WHITE }
}

impl TileCache {
    pub fn ensure_demo_tile(root: &PathBuf, src_id: &str, z: u8, x: i32, y: i32) -> bool {
        let path = tile_path(root, src_id, z, x, y);
        if path.exists() { return true; }
        let img = image::ImageBuffer::from_pixel(256, 256, image::Rgba([100, 100, 100, 255]));
        if let Some(dir) = path.parent() {
            if let Ok(bytes) = img.to_png() {
                std::fs::create_dir_all(dir).ok();
                let _ = std::fs::write(&path, &bytes);
                return true;
            }
        }
        false
    }
}

// Helper functions for coordinate conversion
pub fn lnglat_to_screen(view: &MapView, rect: egui::Rect, point: LngLat) -> Option<egui::Pos2> {
    let n = 1 << view.zoom;
    let x = ((point.lng + 180.0) / 360.0 * n as f64 - (view.center.lng + 180.0) / 360.0 * n as f64) * 256.0 + rect.left();
    // Estimate y using approximate Mercator transformation
    let lat_scale = 111.32 * point.lat.to_radians().cos();
    let dy = (point.lat - view.center.lat) * lat_scale / 1000.0 * (256.0 / (view.zoom as f64 * 0.1)) + rect.top();
    Some(egui::pos2(x, dy))
}

pub fn screen_to_lnglat(view: &MapView, rect: egui::Rect, point: egui::Pos2) -> Option<LngLat> {
    let n = 1 << view.zoom;
    let dx = point.x - rect.left();
    let dy = point.y - rect.top();
    let lng = ((dx / 256.0 + (view.center.lng + 180.0) / 360.0 * n as f64) * 360.0 / n as f64) - 180.0;
    let lat = view.center.lat + (dy / (111.32 * view.center.lat.to_radians().cos() * 1000.0 * (view.zoom as f64 * 0.1)) * 1.0);
    Some(LngLat::new(lng, lat))
}
