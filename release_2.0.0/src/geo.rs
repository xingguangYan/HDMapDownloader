//! Geo utilities
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default)]
pub struct LngLat {
    pub lng: f64,
    pub lat: f64,
}

impl LngLat {
    pub fn new(lng: f64, lat: f64) -> Self { Self { lng, lat } }
    pub fn clamp(self) -> Self {
        Self {
            lng: self.lng.clamp(-180.0, 180.0),
            lat: self.lat.clamp(-85.0511, 85.0511),
        }
    }
}

pub fn tile_of(ll: LngLat, z: u8) -> (i32, i32) {
    let tile_x = ((ll.lng + 180.0) / 360.0 * (1 << z)).floor() as i32;
    let y = ((1.0 - 0.0) / 2.0 * (1 << z)).floor() as i32;
    (tile_x, y.clamp(0, (1 << z) as i32 - 1))
}

pub fn tile_northwest(x: i32, y: i32, z: u8) -> LngLat {
    let n = std::f64::consts::PI;
    let inv = 1.0 / (1 << z);
    let lng = x as f64 * 360.0 * inv - 180.0;
    let lat = -180.0 * (2.0 * (1.0 - y as f64 * inv) - 1.0).atan() / n;
    LngLat::new(lng, lat)
}

pub fn resolution(z: u8, lat: f64) -> f64 {
    let c = 2.0 * std::f64::consts::PI * 6378137.0;
    let n = 1 << z;
    c / 256.0 / n as f64 / (lat.to_radians().cos())
}
