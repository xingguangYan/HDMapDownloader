//! 地理坐标 / 瓦片换算（Web Mercator, EPSG:3857）
use serde::{Deserialize, Serialize};

const EARTH_CIRCUMFERENCE: f64 = 40075016.68557849;
const HALF_EARTH: f64 = 20037508.342789245;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LngLat {
    pub lng: f64,
    pub lat: f64,
}

impl LngLat {
    pub fn new(lng: f64, lat: f64) -> Self {
        Self { lng, lat }
    }
    pub fn clamp(self) -> Self {
        Self {
            lng: self.lng.clamp(-180.0, 180.0),
            lat: self.lat.clamp(-85.05112878, 85.05112878),
        }
    }
}

pub fn tile_count(z: u8) -> f64 {
    (1u32 << z.min(22)) as f64
}

/// 经纬度 -> 整数瓦片 (x, y)，Google/OSM 约定，左上原点
pub fn tile_of(ll: LngLat, z: u8) -> (i32, i32) {
    let n = tile_count(z);
    let x = ((ll.lng + 180.0) / 360.0 * n).floor() as i32;
    let y = ((1.0 - ll.lat.to_radians().tan().asinh() / std::f64::consts::PI) / 2.0 * n).floor() as i32;
    (x, y)
}

/// 瓦片左上角经纬度
pub fn tile_northwest(x: i32, y: i32, z: u8) -> LngLat {
    let n = tile_count(z);
    let lng = x as f64 / n * 360.0 - 180.0;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * y as f64 / n)).sinh().atan().to_degrees();
    LngLat::new(lng, lat)
}

/// 瓦片的 Web Mercator 边界（米），用于 WMS GetMap 的 BBOX
pub fn tile_mercator_bounds(z: u8, x: i32, y: i32) -> (f64, f64, f64, f64) {
    let n = tile_count(z);
    let xmin = x as f64 / n * EARTH_CIRCUMFERENCE - HALF_EARTH;
    let xmax = (x + 1) as f64 / n * EARTH_CIRCUMFERENCE - HALF_EARTH;
    let ymin = HALF_EARTH - (y + 1) as f64 / n * EARTH_CIRCUMFERENCE;
    let ymax = HALF_EARTH - y as f64 / n * EARTH_CIRCUMFERENCE;
    (xmin, ymin, xmax, ymax)
}

/// 某纬度某缩放级别下的地面分辨率（米/像素）
pub fn resolution(z: u8, lat: f64) -> f64 {
    EARTH_CIRCUMFERENCE * lat.to_radians().cos() / 256.0 / tile_count(z)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn beijing_tile_roundtrip() {
        let beijing = LngLat::new(116.4, 39.9);
        let (x, y) = tile_of(beijing, 10);
        assert_eq!((x, y), (843, 388));
        let nw = tile_northwest(x, y, 10);
        assert!(nw.lng <= 116.4 && nw.lat >= 39.9);
        assert_eq!(tile_of(LngLat::new(0.0, 0.0), 1), (1, 1));
    }
}
