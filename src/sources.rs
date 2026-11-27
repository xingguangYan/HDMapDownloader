//! 数据源目录 v3.0 —— 与 web.html 清单一致，免Key源已实测连通（2026-11）
//!
//! - kind=Xyz: url_template 含 {z}/{x}/{y}，需Key源含 {key}
//! - kind=Wms: 按 GIBS WMS GetMap 拼装（BBOX 为该瓦片的 Web Mercator 边界）
//! - kind=Portal: 数据门户，只能打开主页注册（瓦片服务需对方账号）
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceKind {
    Xyz,
    Wms,
    Portal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub kind: SourceKind,
    pub max_zoom: u8,
    pub requires_key: bool,
    pub time_support: bool,
    pub license: &'static str,
    pub note: &'static str,
    pub homepage: &'static str,
    /// kind=Xyz 时使用，含 {z}/{x}/{y}，可选 {key}
    pub url_template: &'static str,
    /// kind=Wms 时使用
    pub wms_endpoint: &'static str,
    /// kind=Wms 时使用的图层名
    pub wms_layers: &'static str,
}

impl SourceEntry {
    pub fn tile_url(&self, z: u8, x: i32, y: i32, key: &str, date: &str) -> String {
        match self.kind {
            SourceKind::Xyz => self
                .url_template
                .replace("{z}", &z.to_string())
                .replace("{x}", &x.to_string())
                .replace("{y}", &y.to_string())
                .replace("{key}", key),
            SourceKind::Wms => {
                let (xmin, ymin, xmax, ymax) = crate::geo::tile_mercator_bounds(z, x, y);
                let mut u = format!(
                    "{}?SERVICE=WMS&REQUEST=GetMap&VERSION=1.3.0&LAYERS={}&CRS=EPSG:3857&BBOX={},{},{},{}&WIDTH=256&HEIGHT=256&FORMAT=image/png",
                    self.wms_endpoint, self.wms_layers, xmin, ymin, xmax, ymax
                );
                if self.time_support && !date.is_empty() {
                    u.push_str("&TIME=");
                    u.push_str(date);
                }
                u
            }
            SourceKind::Portal => self.homepage.to_string(),
        }
    }
}

const GIBS_WMS: &str = "https://gibs.earthdata.nasa.gov/wms/epsg3857/best/wms.cgi";

pub fn catalog() -> Vec<SourceEntry> {
    vec![
        // ---------- 基础底图 ----------
        SourceEntry {
            id: "esri", name: "Esri 世界影像（高清卫星）", group: "基础底图",
            kind: SourceKind::Xyz, max_zoom: 19, requires_key: false, time_support: false,
            license: "Esri 服务条款", note: "全球高清影像，免Key，实测可用。商用请遵守 Esri 条款。",
            homepage: "https://www.esri.com/",
            url_template: "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}",
            wms_endpoint: "", wms_layers: "",
        },
        SourceEntry {
            id: "osm", name: "OpenStreetMap 矢量", group: "基础底图",
            kind: SourceKind::Xyz, max_zoom: 19, requires_key: false, time_support: false,
            license: "ODbL 1.0", note: "官方 tile.openstreetmap.org 国内被限流，改用 OSM.de 镜像，实测可用。",
            homepage: "https://www.openstreetmap.org/",
            url_template: "https://tile.openstreetmap.de/{z}/{x}/{y}.png",
            wms_endpoint: "", wms_layers: "",
        },
        SourceEntry {
            id: "carto_light", name: "CartoDB 浅色矢量", group: "基础底图",
            kind: SourceKind::Xyz, max_zoom: 18, requires_key: false, time_support: false,
            license: "CC-BY 4.0", note: "免Key（a 子域名不稳，用 b/c/d），实测可用。",
            homepage: "https://carto.com/",
            url_template: "https://b.basemaps.cartocdn.com/light_all/{z}/{x}/{y}.png",
            wms_endpoint: "", wms_layers: "",
        },
        SourceEntry {
            id: "carto_dark", name: "CartoDB 深色矢量", group: "基础底图",
            kind: SourceKind::Xyz, max_zoom: 18, requires_key: false, time_support: false,
            license: "CC-BY 4.0", note: "免Key（a 子域名不稳，用 b/c/d），实测可用。",
            homepage: "https://carto.com/",
            url_template: "https://b.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}.png",
            wms_endpoint: "", wms_layers: "",
        },
        // ---------- 卫星影像 ----------
        SourceEntry {
            id: "eox_s2", name: "Sentinel-2 无云镶嵌（2020 全球）", group: "卫星影像",
            kind: SourceKind::Xyz, max_zoom: 18, requires_key: false, time_support: false,
            license: "CC-BY-NC 4.0", note: "EOX 提供，免Key，实测可用。注意 NC：非商业用途。",
            homepage: "https://s2maps.eu/",
            url_template: "https://tiles.maps.eox.at/wmts/1.0.0/s2cloudless-2020_3857/default/GoogleMapsCompatible/{z}/{y}/{x}.jpg",
            wms_endpoint: "", wms_layers: "",
        },
        SourceEntry {
            id: "nasa_bm", name: "NASA Blue Marble 全球底图", group: "卫星影像",
            kind: SourceKind::Wms, max_zoom: 8, requires_key: false, time_support: false,
            license: "NASA 公有领域", note: "GIBS WMS 端点（WMTS 低级别会黑屏，已切到 WMS），实测可用。",
            homepage: "https://www.earthdata.nasa.gov/",
            url_template: "", wms_endpoint: GIBS_WMS,
            wms_layers: "BlueMarble_ShadedRelief_Bathymetry",
        },
        // ---------- 时间序列（GIBS WMS，免Key，按日期加载） ----------
        SourceEntry {
            id: "modis", name: "MODIS Terra 真彩色（每日）", group: "时间序列",
            kind: SourceKind::Wms, max_zoom: 9, requires_key: false, time_support: true,
            license: "NASA 公有领域", note: "GIBS WMS，支持 TIME 参数，实测可用。",
            homepage: "https://www.earthdata.nasa.gov/",
            url_template: "", wms_endpoint: GIBS_WMS,
            wms_layers: "MODIS_Terra_CorrectedReflectance_TrueColor",
        },
        SourceEntry {
            id: "aqua", name: "MODIS Aqua 真彩色（每日）", group: "时间序列",
            kind: SourceKind::Wms, max_zoom: 9, requires_key: false, time_support: true,
            license: "NASA 公有领域", note: "GIBS WMS，支持 TIME 参数。",
            homepage: "https://www.earthdata.nasa.gov/",
            url_template: "", wms_endpoint: GIBS_WMS,
            wms_layers: "MODIS_Aqua_CorrectedReflectance_TrueColor",
        },
        SourceEntry {
            id: "viirs", name: "VIIRS SNPP 真彩色（每日）", group: "时间序列",
            kind: SourceKind::Wms, max_zoom: 9, requires_key: false, time_support: true,
            license: "NASA 公有领域", note: "GIBS WMS，支持 TIME 参数。",
            homepage: "https://www.earthdata.nasa.gov/",
            url_template: "", wms_endpoint: GIBS_WMS,
            wms_layers: "VIIRS_SNPP_CorrectedReflectance_TrueColor",
        },
        SourceEntry {
            id: "hls_l", name: "Landsat 8/9 影像（HLS L30）", group: "时间序列",
            kind: SourceKind::Wms, max_zoom: 13, requires_key: false, time_support: true,
            license: "NASA 公有领域", note: "GIBS WMS HLS_L30，支持 TIME 参数，实测可用。",
            homepage: "https://www.earthdata.nasa.gov/",
            url_template: "", wms_endpoint: GIBS_WMS,
            wms_layers: "HLS_L30_Nadir_BRDF_Adjusted_Reflectance",
        },
        SourceEntry {
            id: "hls_s", name: "Sentinel-2 影像（HLS S30）", group: "时间序列",
            kind: SourceKind::Wms, max_zoom: 13, requires_key: false, time_support: true,
            license: "NASA 公有领域", note: "GIBS WMS HLS_S30，支持 TIME 参数，实测可用。",
            homepage: "https://www.earthdata.nasa.gov/",
            url_template: "", wms_endpoint: GIBS_WMS,
            wms_layers: "HLS_S30_Nadir_BRDF_Adjusted_Reflectance",
        },
        // ---------- 需Key源 ----------
        SourceEntry {
            id: "tianditu", name: "天地图影像（img_w）", group: "需Key源",
            kind: SourceKind::Xyz, max_zoom: 18, requires_key: true, time_support: false,
            license: "天地图服务条款", note: "免费，但必须申请 tk（lbs.tianditu.gov.cn），填入左侧密钥后启用。",
            homepage: "https://lbs.tianditu.gov.cn/",
            url_template: "https://t0.tianditu.gov.cn/img_w/wmts?SERVICE=WMTS&REQUEST=GetTile&VERSION=1.0.0&LAYER=img&STYLE=default&TILEMATRIXSET=w&FORMAT=tiles&TILEMATRIX={z}&TILEROW={y}&TILECOL={x}&tk={key}",
            wms_endpoint: "", wms_layers: "",
        },
        SourceEntry {
            id: "planet", name: "Planet NICFI 月度镶嵌", group: "需Key源",
            kind: SourceKind::Xyz, max_zoom: 18, requires_key: true, time_support: false,
            license: "Planet 服务条款", note: "在 planet.com 注册 → API Keys → 粘贴（NICFI 有限免费额度）。",
            homepage: "https://www.planet.com/nicfi/",
            url_template: "https://tiles.planet.com/basemaps/v1/planet-tiles/global_monthly_2024_06_mosaic/gmap/{z}/{x}/{y}.png?api_key={key}",
            wms_endpoint: "", wms_layers: "",
        },
        // ---------- 数据门户（需注册） ----------
        SourceEntry {
            id: "geovis", name: "中科星图 · 星图地球", group: "数据门户",
            kind: SourceKind::Portal, max_zoom: 0, requires_key: true, time_support: false,
            license: "平台条款", note: "10m 全球 / 0.8m 中国，需注册账号。",
            homepage: "https://daily.geovisearth.com/",
            url_template: "", wms_endpoint: "", wms_layers: "",
        },
        SourceEntry {
            id: "jl1", name: "长光卫星 · 吉林一号", group: "数据门户",
            kind: SourceKind::Portal, max_zoom: 0, requires_key: true, time_support: false,
            license: "活动条款", note: "0.5m 影像，活动期限量免费，需注册。",
            homepage: "https://www.jl1.cn/",
            url_template: "", wms_endpoint: "", wms_layers: "",
        },
        SourceEntry {
            id: "cresda", name: "中国资源卫星应用中心", group: "数据门户",
            kind: SourceKind::Portal, max_zoom: 0, requires_key: true, time_support: false,
            license: "科研公益", note: "高分系列，科研与公益免费，需注册审批。",
            homepage: "https://data.cresda.cn/",
            url_template: "", wms_endpoint: "", wms_layers: "",
        },
        SourceEntry {
            id: "copernicus", name: "Copernicus 数据空间", group: "数据门户",
            kind: SourceKind::Portal, max_zoom: 0, requires_key: true, time_support: false,
            license: "Copernicus 开放", note: "Sentinel 全系，注册免费。",
            homepage: "https://dataspace.copernicus.eu/",
            url_template: "", wms_endpoint: "", wms_layers: "",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_has_free_sources() {
        let c = catalog();
        assert!(c.len() >= 15);
        assert!(c.iter().any(|s| s.id == "esri" && !s.requires_key));
        assert!(c.iter().any(|s| s.id == "hls_l" && s.time_support));
    }
    #[test]
    fn wms_url_has_bbox_and_time() {
        let c = catalog();
        let hls = c.iter().find(|s| s.id == "hls_l").unwrap();
        let u = hls.tile_url(10, 800, 400, "", "2024-06-01");
        assert!(u.contains("BBOX=") && u.contains("TIME=2024-06-01"));
        let u2 = hls.tile_url(10, 800, 400, "", "");
        assert!(!u2.contains("TIME="));
    }
}
