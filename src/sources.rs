//! Enhanced data sources: 20+ free tile sources
//! Including Google, Gaode, TianDiTu, Bing, Esri, OSM, NASA, USGS, and more
//!
//! Usage: load data sources from data-sources.yaml
//! Note: Some sources require API keys - check comments for details

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceEntry {
    pub id: String,
    pub name: String,
    /// tile format: xyz / wmts / wms / api
    pub kind: String,
    pub url_template: String,
    pub resolution: String,
    pub license: String,
    pub license_note: String,
    #[serde(default)]
    pub requires_key: bool,
    #[serde(default)]
    pub enabled_by_default: bool,
    /// 0 = no limit
    pub max_zoom: u8,
    pub min_zoom: u8,
    pub layer: String,
    #[serde(default = "default_availability")]
    pub availability: String,
    pub homepage: String,
    pub verified_clean_license: bool,
    // New in v2.0: source priority for favoring one over another
    pub priority: u32,
}

fn default_availability() -> String {
    "public".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceDoc {
    pub version: u32,
    pub note: String,
    pub sources: Vec<SourceEntry>,
}

impl SourceDoc {
    pub fn default_doc() -> Self {
        // Load from embedded data-sources.yaml
        const DATA: &str = include_str!("../data-sources.yaml");
        Self::parse_yaml(DATA).unwrap_or_else(|| SourceDoc {
            version: 1,
            note: "内置数据源清单解析失败".into(),
            sources: Vec::new(),
        })
    }

    // Simplified YAML parser (subset)
    fn parse_yaml(text: &str) -> Option<SourceDoc> {
        let mut doc = SourceDoc {
            version: 1,
            note: String::new(),
            sources: Vec::new(),
        };
        let mut cur: Option<std::collections::BTreeMap<String, String>> = None;
        let mut in_sources = false;

        for raw in text.lines() {
            let line = raw.trim_end();
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let indent = line.len() - trimmed.len();
            if indent == 0 && !trimmed.starts_with('-') {
                if let Some((k, v)) = trimmed.split_once(':') {
                    match k.trim() {
                        "version" => doc.version = v.trim().parse().unwrap_or(1),
                        "note" => doc.note = v.trim().to_string(),
                        "sources" => in_sources = true,
                        _ => {}
                    }
                }
                continue;
            }
            if !in_sources {
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("- ") {
                if let Some(m) = cur.take() {
                    doc.sources.push(map_to_entry(&m));
                }
                cur = Some(std::collections::BTreeMap::new());
                if let Some((k, v)) = rest.split_once(':') {
                    cur.as_mut().unwrap().insert(k.trim().to_string(), v.trim().to_string());
                }
            } else if let Some((k, v)) = trimmed.split_once(':') {
                if let Some(m) = cur.as_mut() {
                    m.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }

        if let Some(m) = cur.take() {
            doc.sources.push(map_to_entry(&m));
        }

        if doc.sources.is_empty() {
            return None;
        }

        Some(doc)
    }

    pub fn get_by_id(&self, id: &str) -> Option<&SourceEntry> {
        self.sources.iter().find(|s| s.id == id)
    }
}

impl SourceEntry {
    pub fn default_on(&self) -> bool {
        self.verified_clean_license && self.enabled_by_default && !self.requires_key
    }

    pub fn url(&self, z: u8, x: i32, y: i32) -> String {
        let y_flip = (1i64 << z) as i64 - 1 - y as i64;
        self.url_template
            .replace("{z}", &z.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y_flip}", &y_flip.to_string())
            .replace("{y}", &y.to_string())
    }

    pub fn license_badge(&self) -> &'static str {
        if self.verified_clean_license {
            "许可清晰"
        } else if self.requires_key {
            "需申请 Key"
        } else {
            "条件性/未核实"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_builtin_list() {
        let doc = SourceDoc::default_doc();
        assert!(doc.sources.len() >= 20, "数据源应至少 20 条");
    }

    #[test]
    fn url_substitution() {
        let e = SourceEntry {
            id: "x".into(),
            name: "x".into(),
            kind: "xyz".into(),
            url_template: "https://a/{z}/{x}/{y}.png".into(),
            resolution: "".into(),
            license: "".into(),
            license_note: "".into(),
            requires_key: false,
            enabled_by_default: true,
            max_zoom: 18,
            min_zoom: 0,
            layer: "imagery".into(),
            availability: "public".into(),
            homepage: "".into(),
            verified_clean_license: true,
            priority: 1,
        };
        assert_eq!(e.url(3, 4, 5), "https://a/3/4/5.png");
    }
}
