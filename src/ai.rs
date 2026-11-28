//! AI 助手：调用 OpenAI 兼容 /chat/completions，把自然语言解析为可执行 JSON 动作
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AiMsg {
    pub role: String, // "user" | "bot" | "sys"
    pub text: String,
}

pub const AI_SYS: &str = "You are the built-in assistant of HD Map Downloader. Convert the user request into JSON actions (an array). Available actions: \
{\"action\":\"select_region\",\"name\":\"admin region name or 6-digit adcode\"} \
{\"action\":\"set_time\",\"from\":\"YYYY-MM-DD\",\"to\":\"YYYY-MM-DD\"} \
{\"action\":\"set_cloud\",\"max\":20} \
{\"action\":\"search\",\"collection\":\"sentinel-2-l2a|landsat-c2-l2|sentinel-1-grd|cop-dem-glo-90\"} \
{\"action\":\"download_bands\",\"item\":0,\"bands\":[\"B04\",\"B03\",\"B02\"]} \
Rules: output JSON only, no explanation. Default collection sentinel-2-l2a, cloud<=20, latest year. item is the 0-based scene index in the search result.";

pub fn build_body(model: &str, user_msg: &str) -> String {
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": AI_SYS},
            {"role": "user", "content": user_msg}
        ],
        "temperature": 0.2
    });
    body.to_string()
}

/// 调用聊天接口，返回助手文本
pub fn chat(base: &str, key: &str, model: &str, user_msg: &str) -> Result<String, String> {
    let base = base.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err("请先填写 Base URL".into());
    }
    if key.trim().is_empty() {
        return Err("请先填写 API Key".into());
    }
    let url = format!("{}/chat/completions", base);
    let body = build_body(model, user_msg);
    let resp = ureq::post(&url)
        .timeout(std::time::Duration::from_secs(90))
        .set("Authorization", &format!("Bearer {}", key.trim()))
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("AI 请求失败: {e}"))?;
    let v: serde_json::Value = resp.into_json().map_err(|e| format!("解析失败: {e}"))?;
    if let Some(err) = v.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
        return Err(err.to_string());
    }
    let txt = v.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    if txt.is_empty() {
        return Err("AI 返回为空".into());
    }
    Ok(txt)
}

/// 从返回文本中解析出 JSON 动作数组
pub fn parse_commands(txt: &str) -> Option<Vec<serde_json::Value>> {
    let trimmed = txt.trim();
    let v: Option<serde_json::Value> = serde_json::from_str(trimmed).ok().or_else(|| {
        if let (Some(s), Some(e)) = (trimmed.find('['), trimmed.rfind(']')) {
            if e > s {
                if let Ok(v) = serde_json::from_str(&trimmed[s..=e]) {
                    return Some(v);
                }
            }
        }
        let (Some(s), Some(e)) = (trimmed.find('{'), trimmed.rfind('}')) else { return None };
        if e > s { serde_json::from_str(&trimmed[s..=e]).ok() } else { None }
    });
    let v = v?;
    if let Some(arr) = v.as_array() {
        return Some(arr.clone());
    }
    if let Some(actions) = v.get("actions").and_then(|a| a.as_array()) {
        return Some(actions.clone());
    }
    Some(vec![v])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_cmds() {
        let v = parse_commands("{\"action\":\"set_cloud\",\"max\":10}").unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0]["action"], "set_cloud");
        let v2 = parse_commands("没错\n[{\"action\":\"search\"},{\"action\":\"set_time\",\"from\":\"2024-06-01\",\"to\":\"2024-06-30\"}]").unwrap();
        assert_eq!(v2.len(), 2);
    }
}
/// AI 计划执行快照
pub struct PlanSnap {
    pub bbox: Option<(f64, f64, f64, f64)>,
    pub date_from: String,
    pub date_to: String,
    pub cloud: i32,
    pub collection: String,
    pub export_dir: std::path::PathBuf,
}

/// AI 事件：后台线程 -> 主线程
pub enum AiEvent {
    Log(AiMsg),
    SetTime(String, String),
    SetCloud(i32),
    SetCollection(String),
    SetBounds((f64, f64, f64, f64), String),
    Scenes(crate::stac::SearchResult),
    BandDone(Vec<std::path::PathBuf>),
    Err(String),
    Finished,
}

fn log(tx: &crossbeam_channel::Sender<AiEvent>, role: &str, text: String) {
    let _ = tx.send(AiEvent::Log(AiMsg { role: role.to_string(), text }));
}

/// 一次完整 AI 计划：chat -> 解析 JSON -> 逐步执行（全部在后台线程）
pub fn run_plan(
    base: String,
    key: String,
    model: String,
    user_msg: String,
    snap: PlanSnap,
    tx: crossbeam_channel::Sender<AiEvent>,
) {
    std::thread::spawn(move || {
        let finish = |tx: &crossbeam_channel::Sender<AiEvent>| {
            let _ = tx.send(AiEvent::Finished);
        };
        match chat(&base, &key, &model, &user_msg) {
            Ok(txt) => {
                log(&tx, "bot", txt.clone());
                match parse_commands(&txt) {
                    Some(cmds) => run_cmds(&tx, snap, cmds),
                    None => log(&tx, "sys", "未解析出可执行 JSON".to_string()),
                }
            }
            Err(e) => log(&tx, "sys", e),
        }
        finish(&tx);
    });
}

fn run_cmds(tx: &crossbeam_channel::Sender<AiEvent>, snap: PlanSnap, cmds: Vec<serde_json::Value>) {
    use crate::adm;
    use crate::stac;
    let mut bbox = snap.bbox;
    let mut from = snap.date_from;
    let mut to = snap.date_to;
    let mut cloud = snap.cloud;
    let mut collection = snap.collection;
    let mut scenes: Vec<stac::Scene> = Vec::new();
    for c in cmds {
        let action = c.get("action").and_then(|a| a.as_str()).unwrap_or("").to_string();
        match action.as_str() {
            "select_region" => {
                if let Some(name) = c.get("name").and_then(|n| n.as_str()) {
                    match adm::find_region(name) {
                        Ok(feats) => {
                            if let Some(f) = feats.first() {
                                bbox = Some(f.bbox);
                                let _ = tx.send(AiEvent::SetBounds(f.bbox, f.name.clone()));
                                log(tx, "sys", format!("已选区：{}", f.name));
                            } else {
                                log(tx, "sys", "行政区无要素".to_string());
                            }
                        }
                        Err(e) => log(tx, "sys", format!("行政区未找到 {}: {}", name, e)),
                    }
                }
            }
            "set_time" => {
                if let Some(a) = c.get("from").and_then(|v| v.as_str()) {
                    from = a.to_string();
                }
                if let Some(b) = c.get("to").and_then(|v| v.as_str()) {
                    to = b.to_string();
                }
                let _ = tx.send(AiEvent::SetTime(from.clone(), to.clone()));
            }
            "set_cloud" => {
                if let Some(m) = c.get("max").and_then(|v| v.as_i64()) {
                    cloud = m as i32;
                    let _ = tx.send(AiEvent::SetCloud(cloud));
                }
            }
            "search" => {
                if let Some(col) = c.get("collection").and_then(|v| v.as_str()) {
                    collection = col.to_string();
                    let _ = tx.send(AiEvent::SetCollection(collection.clone()));
                }
                match bbox {
                    Some(b) => {
                        let p = stac::SearchParams { collection: collection.clone(), bbox: b, date_from: from.clone(), date_to: to.clone(), cloud_max: Some(cloud), page: 1 };
                        match stac::search(&p) {
                            Ok(r) => {
                                log(tx, "sys", format!("检索到 {} 景", r.scenes.len()));
                                scenes = r.scenes.clone();
                                let _ = tx.send(AiEvent::Scenes(r));
                            }
                            Err(e) => log(tx, "sys", format!("检索失败: {e}")),
                        }
                    }
                    None => log(tx, "sys", "未指定区域，无法检索".to_string()),
                }
            }
            "download_bands" => {
                let item = c.get("item").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let bands: Vec<String> = c.get("bands")
                    .and_then(|b| b.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                if scenes.is_empty() {
                    if let Some(b) = bbox {
                        let p = stac::SearchParams { collection: collection.clone(), bbox: b, date_from: from.clone(), date_to: to.clone(), cloud_max: Some(cloud), page: 1 };
                        if let Ok(r) = stac::search(&p) {
                            scenes = r.scenes.clone();
                            let _ = tx.send(AiEvent::Scenes(r));
                        }
                    }
                }
                match scenes.get(item) {
                    Some(sc) => {
                        log(tx, "sys", format!("开始下载 {} 的 {} 个波段", sc.id, bands.len()));
                        match stac::download_bands(sc, &bands, &snap.export_dir) {
                            Ok(paths) => {
                                let _ = tx.send(AiEvent::BandDone(paths.clone()));
                                log(tx, "sys", format!("波段已保存 {} 个文件到 {}", paths.len(), snap.export_dir.display()));
                            }
                            Err(e) => log(tx, "sys", format!("波段下载失败: {e}")),
                        }
                    }
                    None => log(tx, "sys", format!("没有第 {} 景", item)),
                }
            }
            _ => log(tx, "sys", format!("未知动作: {action}")),
        }
    }
}