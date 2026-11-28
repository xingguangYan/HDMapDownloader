//! 公开影像 STAC 检索（Earth Search）与波段下载
use serde::{Deserialize, Serialize};

const STAC: &str = "https://earth-search.aws.element84.com/v1/search";

pub const COLLECTIONS: [&str; 4] = ["sentinel-2-l2a", "landsat-c2-l2", "sentinel-1-grd", "cop-dem-glo-90"];

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SceneBand {
    pub name: String,
    pub href: String,
    pub direct: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub datetime: String,
    pub cloud: Option<f64>,
    pub platform: String,
    pub collection: String,
    pub bands: Vec<SceneBand>,
    pub bbox: (f64, f64, f64, f64),
    pub footprint: Vec<Vec<(f64, f64)>>,
}

#[derive(Clone, Debug, Default)]
pub struct SearchResult {
    pub scenes: Vec<Scene>,
    pub matched: u64,
}

#[derive(Clone, Debug)]
pub struct SearchParams {
    pub collection: String,
    pub bbox: (f64, f64, f64, f64),
    pub date_from: String,
    pub date_to: String,
    pub cloud_max: Option<i32>,
    pub page: u32,
}

fn pct(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn band_visible(k: &str, href: &str) -> bool {
    if matches!(k, "thumbnail" | "preview" | "metadata") {
        return false;
    }
    if href.ends_with(".tif") || href.ends_with(".tiff") {
        return true;
    }
    let pats = ["B0", "B1", "B2", "B3", "B4", "B5", "B6", "B7", "B8", "B9", "B8A", "TCI", "AOT", "WVP",
        "coastal", "blue", "green", "nir", "swir", "lwir", "qa_", "vv", "vh", "data"];
    pats.iter().any(|p| k.starts_with(p) || k == *p)
}

fn ring_from(arr: &serde_json::Value) -> Option<Vec<(f64, f64)>> {
    let ring = arr.as_array()?;
    let r: Vec<(f64, f64)> = ring.iter().filter_map(|p| {
        let a = p.as_array()?;
        if a.len() >= 2 {
            Some((a[0].as_f64()?, a[1].as_f64()?))
        } else {
            None
        }
    }).collect();
    if r.len() >= 3 { Some(r) } else { None }
}

fn parse_scene(f: &serde_json::Value, collection: &str) -> Option<Scene> {
    let id = f.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let props = f.get("properties")?;
    let datetime = props.get("datetime").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let cloud = props.get("eo:cloud_cover").and_then(|v| v.as_f64());
    let platform = props.get("platform").and_then(|v| v.as_str())
        .or_else(|| props.get("constellation").and_then(|v| v.as_str()))
        .unwrap_or("").to_string();
    let mut bands = Vec::new();
    if let Some(map) = f.get("assets").and_then(|a| a.as_object()) {
        for (k, v) in map {
            let href = v.get("href").and_then(|h| h.as_str()).unwrap_or("").to_string();
            if href.is_empty() || !band_visible(k, &href) {
                continue;
            }
            bands.push(SceneBand { name: k.clone(), href: href.clone(), direct: href.starts_with("https") });
        }
    }
    let mut bbox = (0.0, 0.0, 0.0, 0.0);
    if let Some(b) = f.get("bbox").and_then(|v| v.as_array()) {
        if b.len() == 4 {
            bbox = (
                b[0].as_f64().unwrap_or(0.0),
                b[1].as_f64().unwrap_or(0.0),
                b[2].as_f64().unwrap_or(0.0),
                b[3].as_f64().unwrap_or(0.0),
            );
        }
    }
    let mut footprint = Vec::new();
    if let Some(geom) = f.get("geometry") {
        let gtype = geom.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let coords = geom.get("coordinates");
        if gtype == "Polygon" {
            if let Some(first) = coords.and_then(|c| c.as_array()).and_then(|r| r.first()) {
                if let Some(r) = ring_from(first) {
                    footprint.push(r);
                }
            }
        } else if gtype == "MultiPolygon" {
            if let Some(polys) = coords.and_then(|c| c.as_array()) {
                for poly in polys {
                    if let Some(first) = poly.as_array().and_then(|r| r.first()) {
                        if let Some(r) = ring_from(first) {
                            footprint.push(r);
                        }
                    }
                }
            }
        }
    }
    Some(Scene { id, datetime, cloud, platform, collection: collection.to_string(), bands, bbox, footprint })
}

pub fn search(p: &SearchParams) -> Result<SearchResult, String> {
    let (w, s, e, n) = p.bbox;
    let mut url = format!(
        "{STAC}/search?collections={}&limit=100&page={}&bbox={},{},{},{}&datetime={}T00:00:00Z/{}T23:59:59Z",
        pct(&p.collection), p.page, w, s, e, n, pct(&p.date_from), pct(&p.date_to)
    );
    if let Some(c) = p.cloud_max {
        if p.collection == "sentinel-2-l2a" || p.collection == "landsat-c2-l2" {
            let q = format!("{{\"eo:cloud_cover\":{{\"lte\":{}}}}}", c);
            url.push_str("&query=");
            url.push_str(&pct(&q));
        }
    }
    let resp = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(60))
        .call()
        .map_err(|e| format!("检索失败: {e}"))?;
    let v: serde_json::Value = resp.into_json().map_err(|e| format!("解析失败: {e}"))?;
    let matched = v.get("context").and_then(|c| c.get("matched")).and_then(|m| m.as_u64()).unwrap_or(0);
    let mut scenes = Vec::new();
    if let Some(arr) = v.get("features").and_then(|f| f.as_array()) {
        for f in arr {
            if let Some(sc) = parse_scene(f, &p.collection) {
                scenes.push(sc);
            }
        }
    }
    Ok(SearchResult { scenes, matched })
}

/// 下载场景的指定波段（仅 https 直连，如 Sentinel-2 COG），保存到 out_dir
pub fn download_bands(scene: &Scene, names: &[String], out_dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>, String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let mut saved = Vec::new();
    let mut errs = Vec::new();
    for n in names {
        let Some(b) = scene.bands.iter().find(|b| &b.name == n) else { continue };
        if !b.direct {
            errs.push(format!("{}: S3 路径非直连", b.name));
            continue;
        }
        let path = out_dir.join(format!("{}_{}.tif", scene.id, b.name));
        match ureq::get(&b.href).timeout(std::time::Duration::from_secs(300)).call() {
            Ok(resp) => {
                let mut reader = resp.into_reader();
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut reader, &mut buf).map_err(|e| e.to_string())?;
                std::fs::write(&path, &buf).map_err(|e| e.to_string())?;
                saved.push(path);
            }
            Err(e) => errs.push(format!("{}: {e}", b.name)),
        }
    }
    if saved.is_empty() && !errs.is_empty() {
        return Err(errs.join("; "));
    }
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_bbox() {
        let v: serde_json::Value = serde_json::json!({
            "id": "S2_test",
            "properties": {"datetime": "2024-06-15T03:00:00Z", "eo:cloud_cover": 5.0, "platform": "sentinel-2b"},
            "bbox": [120.0, 30.0, 121.0, 31.0],
            "geometry": {"type": "Polygon", "coordinates": [[[120.0,30.0],[121.0,30.0],[121.0,31.0],[120.0,30.0]]]},
            "assets": {"B02": {"href": "https://sentinel-cogs.s3.us-west-2.amazonaws.com/x/B02.tif"}, "thumbnail": {"href": "https://x/thumb.png"}}
        });
        let sc = parse_scene(&v, "sentinel-2-l2a").unwrap();
        assert_eq!(sc.id, "S2_test");
        assert_eq!(sc.cloud, Some(5.0));
        assert_eq!(sc.bbox.0, 120.0);
        assert_eq!(sc.bands.len(), 1);
        assert!(sc.bands[0].direct);
        assert_eq!(sc.footprint.len(), 1);
    }
}