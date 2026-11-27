//! 地图视口：中心/缩放、经纬度与屏幕像素互转、可见瓦片范围
use crate::geo::{self, LngLat};
use eframe::egui;

const TILE: f64 = 256.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MapView {
    pub center: LngLat,
    pub zoom: u8,
}

/// 经纬度 -> 全球像素坐标（该 zoom 下 256*n 见方的全球像素网格，左上为原点）
fn global_pixel(ll: LngLat, z: u8) -> (f64, f64) {
    let n = TILE * geo::tile_count(z);
    let x = (ll.lng + 180.0) / 360.0 * n;
    let y = (1.0 - ll.lat.to_radians().tan().asinh() / std::f64::consts::PI) / 2.0 * n;
    (x, y)
}

fn unproject_y(py: f64, z: u8) -> f64 {
    let n = TILE * geo::tile_count(z);
    (std::f64::consts::PI * (1.0 - 2.0 * py / n)).sinh().atan().to_degrees()
}

/// 视口左上角的全局像素坐标
pub fn view_origin(view: &MapView, rect: &egui::Rect) -> (f64, f64) {
    let (cx, cy) = global_pixel(view.center, view.zoom);
    (cx - rect.width() as f64 / 2.0, cy - rect.height() as f64 / 2.0)
}

pub fn lnglat_to_screen(view: &MapView, rect: &egui::Rect, ll: LngLat) -> egui::Pos2 {
    let (ox, oy) = view_origin(view, rect);
    let (px, py) = global_pixel(ll, view.zoom);
    egui::pos2((px - ox) as f32, (py - oy) as f32)
}

pub fn screen_to_lnglat(view: &MapView, rect: &egui::Rect, p: egui::Pos2) -> LngLat {
    let (ox, oy) = view_origin(view, rect);
    let n = TILE * geo::tile_count(view.zoom);
    let lng = (p.x as f64 + ox) / n * 360.0 - 180.0;
    let lat = unproject_y(p.y as f64 + oy, view.zoom);
    LngLat::new(lng, lat)
}

/// 当前视口可见的瓦片行列范围（含边界）
pub fn visible_tiles(view: &MapView, rect: &egui::Rect) -> (i32, i32, i32, i32) {
    let n = geo::tile_count(view.zoom) as i32;
    let (ox, oy) = view_origin(view, rect);
    let x0 = (ox / TILE).floor() as i32;
    let y0 = (oy / TILE).floor() as i32;
    let x1 = ((ox + rect.width() as f64) / TILE).floor() as i32;
    let y1 = ((oy + rect.height() as f64) / TILE).floor() as i32;
    (x0.max(0), y0.max(0), x1.min(n - 1), y1.min(n - 1))
}

/// 某块瓦片在屏幕上的矩形
pub fn tile_screen_rect(view: &MapView, rect: &egui::Rect, x: i32, y: i32) -> egui::Rect {
    let (ox, oy) = view_origin(view, rect);
    let left = (x as f64 * TILE - ox) as f32;
    let top = (y as f64 * TILE - oy) as f32;
    egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(TILE as f32, TILE as f32))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn screen_roundtrip() {
        let view = MapView { center: LngLat::new(116.4, 39.9), zoom: 10 };
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let p = egui::pos2(400.0, 300.0); // 中心
        let ll = screen_to_lnglat(&view, &rect, p);
        assert!((ll.lng - 116.4).abs() < 1e-6);
        assert!((ll.lat - 39.9).abs() < 1e-6);
        let back = lnglat_to_screen(&view, &rect, ll);
        assert!((back.x - 400.0).abs() < 1e-3 && (back.y - 300.0).abs() < 1e-3);
    }
}
