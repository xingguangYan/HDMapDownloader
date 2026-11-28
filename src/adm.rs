//! 行政区划矢量（DataV GeoAtlas）：国家/省/市/县边界
//! https://geo.datav.aliyun.com/areas_v3/bound/{adcode}_full.json
use serde::{Deserialize, Serialize};

const DATAV: &str = "https://geo.datav.aliyun.com/areas_v3/bound";

/// 一个行政区划要素（仅保留外环用于绘制与点击命中）
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AdmFeature {
    pub code: String,
    pub name: String,
    pub level: String,
    /// 外环数组（lng,lat）
    pub outer_rings: Vec<Vec<(f64, f64)>>,
    /// (w, s, e, n)
    pub bbox: (f64, f64, f64, f64),
}

pub fn short_name(n: &str) -> String {
    let mut s = n.to_string();
    for suf in ["省", "市", "自治区", "特别行政区", "壮族", "回族", "维吾尔", "地区", "盟", "自治州", "自治县", "区", "县"] {
        s = s.replace(suf, "");
    }
    s
}

pub fn prov_table() -> Vec<(&'static str, &'static str)> {
    vec![
        ("110000", "北京市"), ("120000", "天津市"), ("130000", "河北省"), ("140000", "山西省"),
        ("150000", "内蒙古自治区"), ("210000", "辽宁省"), ("220000", "吉林省"), ("230000", "黑龙江省"),
        ("310000", "上海市"), ("320000", "江苏省"), ("330000", "浙江省"), ("340000", "安徽省"),
        ("350000", "福建省"), ("360000", "江西省"), ("370000", "山东省"), ("410000", "河南省"),
        ("420000", "湖北省"), ("430000", "湖南省"), ("440000", "广东省"), ("450000", "广西壮族自治区"),
        ("460000", "海南省"), ("500000", "重庆市"), ("510000", "四川省"), ("520000", "贵州省"),
        ("530000", "云南省"), ("540000", "西藏自治区"), ("610000", "陕西省"), ("620000", "甘肃省"),
        ("630000", "青海省"), ("640000", "宁夏回族自治区"), ("650000", "新疆维吾尔自治区"),
        ("710000", "台湾省"), ("810000", "香港特别行政区"), ("820000", "澳门特别行政区"),
    ]
}

fn fetch_json(url: &str) -> Result<serde_json::Value, String> {
    let resp = ureq::get(url)
        .timeout(std::time::Duration::from_secs(40))
        .call()
        .map_err(|e| format!("请求失败: {e}"))?;
    resp.into_json::<serde_json::Value>().map_err(|e| format!("解析失败: {e}"))
}

fn parse_feature(f: &serde_json::Value) -> Option<AdmFeature> {
    let props = f.get("properties")?;
    let code = props.get("adcode").and_then(|v| v.as_u64()).map(|c| c.to_string())?;
    let name = props.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let level = props.get("level").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let geom = f.get("geometry").or_else(|| f.get("geom"))?;
    let gtype = geom.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let coords = geom.get("coordinates")?;
    let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut all_coords: Vec<(f64, f64)> = Vec::new();
    match gtype {
        "Polygon" => {
            if let Some(arr) = coords.as_array() {
                if let Some(first) = arr.first().and_then(|r| r.as_array()) {
                    let ring = first.iter().filter_map(|p| {
                        let a = p.as_array()?;
                        if a.len() >= 2 {
                            Some((a[0].as_f64()?, a[1].as_f64()?))
                        } else {
                            None
                        }
                    }).collect::<Vec<_>>();
                    if !ring.is_empty() {
                        rings.push(ring);
                    }
                }
            }
        }
        "MultiPolygon" => {
            if let Some(polys) = coords.as_array() {
                for poly in polys {
                    if let Some(arr) = poly.as_array() {
                        if let Some(first) = arr.first().and_then(|r| r.as_array()) {
                            let ring = first.iter().filter_map(|p| {
                                let a = p.as_array()?;
                                if a.len() >= 2 {
                                    Some((a[0].as_f64()?, a[1].as_f64()?))
                                } else {
                                    None
                                }
                            }).collect::<Vec<_>>();
                            if !ring.is_empty() {
                                rings.push(ring);
                            }
                        }
                    }
                }
            }
        }
        _ => return None,
    }
    if rings.is_empty() {
        return None;
    }
    for ring in &rings {
        for &(lng, lat) in ring {
            all_coords.push((lng, lat));
        }
    }
    let mut w = f64::MAX; let mut s = f64::MAX; let mut e = f64::MIN; let mut n = f64::MIN;
    for &(lng, lat) in &all_coords {
        w = w.min(lng); e = e.max(lng);
        s = s.min(lat); n = n.max(lat);
    }
    Some(AdmFeature { code, name, level, outer_rings: rings, bbox: (w, s, e, n) })
}

/// 抓取某 adcode 的 {code}_full.json，解析全部子要素（含自身）
pub fn load_code(code: &str) -> Result<Vec<AdmFeature>, String> {
    let url = format!("{DATAV}/{code}_full.json");
    let v = fetch_json(&url)?;
    let feats = v.get("features").and_then(|f| f.as_array()).ok_or("无 features")?;
    let out: Vec<AdmFeature> = feats.iter().filter_map(parse_feature).collect();
    if out.is_empty() {
        return Err("解析不到任何要素".into());
    }
    Ok(out)
}

/// 按关键字定位：支持 adcode / 中国 / 省 / 市 / 县
pub fn find_region(kw: &str) -> Result<Vec<AdmFeature>, String> {
    let kw = kw.trim();
    if kw.is_empty() {
        return Err("请输入行政区名".into());
    }
    if kw.chars().all(|c| c.is_ascii_digit()) && kw.len() == 6 {
        return load_code(kw);
    }
    if kw == "中国" || kw == "全国" || kw.eq_ignore_ascii_case("china") {
        return load_code("100000");
    }
    let provs = prov_table();
    // 省
    for (code, name) in &provs {
        if name.starts_with(kw) || kw.starts_with(name) || short_name(name) == kw {
            return load_code(code);
        }
    }
    // 市/县：先按省名前缀试，再全局扫
    let sn = short_name(kw);
    let mut order: Vec<&'static str> = Vec::new();
    for (code, name) in &provs {
        if sn.starts_with(&short_name(name)) {
            order.push(code);
        }
    }
    for (code, _) in &provs {
        if !order.contains(code) {
            order.push(code);
        }
    }
    for code in order {
        let feats = load_code(code)?;
        for f in &feats {
            if f.name == kw || short_name(&f.name) == sn {
                return Ok(vec![f.clone()]);
            }
        }
    }
    Err(format!("未找到该行政区: {kw}"))
}

/// 点击命中：点在任一外环内（ray casting）
pub fn hit(pt: (f64, f64), rings: &[Vec<(f64, f64)>]) -> bool {
    for ring in rings {
        if point_in_ring(pt, ring) {
            return true;
        }
    }
    false
}

fn point_in_ring(pt: (f64, f64), ring: &[(f64, f64)]) -> bool {
    let (x, y) = pt;
    let mut inside = false;
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = ring[i];
        let (xj, yj) = ring[j];
        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hit_test() {
        let ring = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0), (0.0, 0.0)];
        assert!(hit((5.0, 5.0), &[ring.clone()]));
        assert!(!hit((15.0, 5.0), &[ring]));
    }
    #[test]
    fn short_name_test() {
        assert_eq!(short_name("杭州市"), "杭州");
        assert_eq!(short_name("浙江省"), "浙江");
    }
}