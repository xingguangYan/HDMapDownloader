//! 界面布局：顶栏 + 左侧控制面板 + 中央地图 + 右侧（场景/AI）（v4.0）
use crate::adm::AdmFeature;
use crate::app::App;
use crate::donate;
use crate::geo::LngLat;
use crate::mapview;
use crate::sources::SourceKind;
use eframe::egui;

pub const EMAIL: &str = "xingguangyan0703@gmail.com";

pub fn ui(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading(if matches!(app.lang, crate::i18n::Lang::En) {
                "HD Map Downloader v4.0"
            } else {
                "HD 高清地图下载器 v4.0"
            });
            ui.separator();
            ui.label(format!("{}: {}", app.tr("当前源", "Source"), app.active_name()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(if matches!(app.lang, crate::i18n::Lang::En) { "中文" } else { "EN" }).clicked() {
                    app.lang = if matches!(app.lang, crate::i18n::Lang::En) { crate::i18n::Lang::Zh } else { crate::i18n::Lang::En };
                    app.status = app.tr("语言已切换", "Language switched");
                    app.save_config();
                }
                ui.label(egui::RichText::new(format!("{EMAIL}")).small().weak());
            });
        });
    });

    egui::SidePanel::left("side")
        .default_width(360.0)
        .width_range(280.0..=460.0)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                source_section(app, ui);
                time_section(app, ui);
                tool_section(app, ui);
                adm_section(app, ui);
                stac_section(app, ui);
                download_section(app, ui, ctx);
                portals_section(app, ui);
                donate::card(app, ui);
                ui.add_space(6.0);
                ui.label(egui::RichText::new(format!("{}: {EMAIL}", app.tr("底部联系", "Contact"))).small().weak());
            });
        });

    // 右栏：场景列表 + AI 助手
    egui::SidePanel::right("right")
        .default_width(360.0)
        .width_range(260.0..=520.0)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                scenes_section(app, ui);
                ai_section(app, ui);
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        map_panel(app, ui, ctx);
    });

    if app.time_modal {
        time_modal(app, ctx);
    }
    if app.cs_open {
        custom_source_window(app, ctx);
    }
    if app.show_alipay {
        qr_window(app, ctx);
    }
}

fn source_section(app: &mut App, ui: &mut egui::Ui) {
    ui.heading(app.tr("数据源", "Map Sources"));
    let items: Vec<(String, String, String, bool)> = app
        .sources
        .iter()
        .map(|s| (s.id.to_string(), s.name.to_string(), s.group.to_string(), s.requires_key))
        .collect();
    let custom_items: Vec<(String, String)> = app
        .custom
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();
    let selected_name = app.active_name();
    let prev = app.active_source.clone();
    egui::ComboBox::from_label(app.tr("选择地图源", "Select source"))
        .selected_text(selected_name)
        .show_ui(ui, |ui| {
            for group in ["基础底图", "卫星影像", "时间序列", "需Key源", "数据门户"] {
                let entries: Vec<&(String, String, String, bool)> = items.iter().filter(|i| i.2 == group).collect();
                if entries.is_empty() {
                    continue;
                }
                ui.label(egui::RichText::new(group).strong());
                for (id, name, _, needs_key) in entries {
                    let label = if *needs_key { format!("{name}（需Key）") } else { name.clone() };
                    ui.selectable_value(&mut app.active_source, id.clone(), label);
                }
            }
            if !custom_items.is_empty() {
                ui.label(egui::RichText::new("自定义").strong());
                for (id, name) in &custom_items {
                    ui.selectable_value(&mut app.active_source, id.clone(), name.clone());
                }
            }
        });
    if app.active_source != prev {
        app.on_source_changed();
    }

    if ui.button(app.tr("＋ 接入自定义源（登录后）", "＋ Add custom source (post-login)")).clicked() {
        app.cs_open = true;
    }

    let info = app.active().map(|s| {
        let license = format!("{}: {} | {}", app.tr("许可", "License"), s.license, s.note);
        let homepage = if !s.homepage.is_empty() && s.kind != SourceKind::Portal {
            Some(s.homepage.to_string())
        } else {
            None
        };
        (license, homepage, s.requires_key)
    });
    if let Some((license, homepage, needs_key)) = info {
        ui.add_space(2.0);
        ui.label(egui::RichText::new(license).small().weak());
        if let Some(h) = &homepage {
            ui.hyperlink_to(&app.tr("数据源主页", "Homepage"), h);
        }
        if needs_key {
            ui.add_space(2.0);
            ui.label(egui::RichText::new(app.tr("密钥（tk / API Key）:", "Key (tk / API Key):")).small());
            ui.add(egui::TextEdit::singleline(&mut app.api_key).hint_text("tk / API Key"));
            if let Some(h) = &homepage {
                ui.hyperlink_to(&app.tr("申请地址", "Apply here"), h);
            }
        }
    } else if let Some(c) = app.active_custom().cloned() {
        ui.label(egui::RichText::new(format!("{}: {}", app.tr("许可", "License"), c.note)).small().weak());
        if c.requires_key {
            ui.label(egui::RichText::new(app.tr("密钥（tk / API Key）:", "Key:")).small());
            ui.add(egui::TextEdit::singleline(&mut app.api_key).hint_text("token / key"));
        }
    }
}

fn time_section(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.heading(app.tr("影像时间（时间序列源可用）", "Image Date (time-series sources)"));
    let has_time = app.active().map(|s| s.time_support).unwrap_or(false);
    ui.add_enabled(has_time, egui::TextEdit::singleline(&mut app.date).hint_text("2024-06-01"));
    if has_time {
        ui.label(egui::RichText::new(app.tr("格式 YYYY-MM-DD，MODIS / Landsat / Sentinel-2(HLS) 按日期加载", "Format YYYY-MM-DD; MODIS/Landsat/Sentinel-2(HLS) load by date")).small().weak());
    } else {
        ui.label(egui::RichText::new(app.tr("当前源不支持时间参数", "Current source has no time support")).small().weak());
    }
}

fn tool_section(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.heading(app.tr("选择工具", "Selection Tools"));
    ui.horizontal(|ui| {
        if ui.selectable_label(app.tool == 1, app.tr("▢ 框选", "▢ Rectangle")).clicked() {
            app.set_tool(1);
        }
        if ui.selectable_label(app.tool == 2, app.tr("⬠ 多边形", "⬠ Polygon")).clicked() {
            app.set_tool(2);
        }
        if ui.selectable_label(app.tool == 3, app.tr("◉ 点选", "◉ Point")).clicked() {
            app.set_tool(3);
        }
    });
    if ui.button(app.tr("✕ 清除选区", "Clear selection")).clicked() {
        app.clear_sel();
    }
    if let Some((min, max)) = app.sel_bounds {
        let (w, s, e, n) = (
            min.lng.min(max.lng), min.lat.min(max.lat),
            min.lng.max(max.lng), min.lat.max(max.lat),
        );
        ui.label(format!("{} {:.4}  {:.4}", app.tr("西/南", "W/S"), w, s));
        ui.label(format!("{} {:.4}  {:.4}", app.tr("东/北", "E/N"), e, n));
    } else {
        ui.label(egui::RichText::new(app.tr("尚未选区", "No selection yet")).small().weak());
    }
    if app.tool == 2 {
        ui.label(egui::RichText::new(app.tr("多边形：单击加点（≥3），双击结束", "Polygon: click to add vertices (>=3), double-click to finish")).small().weak());
    }
}

fn adm_section(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.heading(app.tr("行政矢量（国家/省/市/县）", "Admin Vectors (country/province/city/county)"));
    ui.horizontal(|ui| {
        let hint_adm = app.tr("浙江 / 杭州 / 西湖 / 6位adcode", "Zhejiang / Hangzhou / adcode");
        ui.add(egui::TextEdit::singleline(&mut app.adm_kw).hint_text(hint_adm));
    });
    ui.horizontal(|ui| {
        if ui.button(app.tr("定位并载入矢量", "Locate & load")).clicked() {
            app.adm_find();
        }
        if ui.button(app.tr("下载当前矢量", "Download GeoJSON")).clicked() {
            match app.adm_save_geojson() {
                Ok(p) => app.status = format!("{}: {}", app.tr("矢量已保存", "GeoJSON saved"), p.display()),
                Err(e) => app.status = e,
            }
        }
    });
    if app.adm_loading {
        ui.label(egui::RichText::new(app.tr("检索中 ...", "Searching ...")).small());
    }
    if !app.adm_polys.is_empty() {
        let n = app.adm_polys.len();
        let cur = app.adm_cur.map(|i| app.adm_polys[i].name.clone()).unwrap_or_default();
        ui.label(egui::RichText::new(format!("{}：{}", app.tr("已载入要素", "Features loaded"), n)).small().weak());
        if !cur.is_empty() {
            ui.label(egui::RichText::new(format!("{}：{}", app.tr("当前", "Current"), cur)).small().weak());
        }
        ui.label(egui::RichText::new(app.tr("点击地图上的矢量可选区下载", "Click a vector on the map to select it")).small().weak());
    }
}

fn stac_section(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.heading(app.tr("公开影像检索 (STAC)", "Open Imagery Search (STAC)"));
    let prev = app.sc_collection.clone();
    egui::ComboBox::from_label(app.tr("数据集", "Dataset"))
        .selected_text(app.sc_collection.clone())
        .show_ui(ui, |ui| {
            for c in crate::stac::COLLECTIONS {
                ui.selectable_value(&mut app.sc_collection, c.to_string(), c.to_string());
            }
        });
    let _ = prev;
    ui.horizontal(|ui| {
        ui.label(app.tr("从", "From"));
        ui.add(egui::TextEdit::singleline(&mut app.sc_date_from).hint_text("2024-06-01").desired_width(90.0));
        ui.label(app.tr("到", "To"));
        ui.add(egui::TextEdit::singleline(&mut app.sc_date_to).hint_text("2024-06-30").desired_width(90.0));
    });
    ui.horizontal(|ui| {
        ui.label(format!("{}:", app.tr("云量", "Cloud")));
        ui.add(egui::Slider::new(&mut app.sc_cloud, 0..=100).text("%"));
    });
    if app.sc_loading {
        ui.label(egui::RichText::new(app.tr("检索中 ...", "Searching ...")).small());
    } else if ui.button(app.tr("检索选区场景", "Search scenes in selection")).clicked() {
        if app.sel_bounds.is_none() {
            app.status = app.tr("请先划定区域再检索影像", "Select a region before searching imagery");
        } else {
            // 检索前先提示确认时间范围
            app.time_modal = true;
        }
    }
    ui.label(egui::RichText::new(app.tr("检索前会提示选择时间范围；Sentinel-2 波段可直接下载（Landsat 为 S3 路径）", "Time range is confirmed before search; Sentinel-2 bands download directly, Landsat gives S3 paths")).small().weak());
}

fn download_section(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.add_space(6.0);
    ui.heading(app.tr("瓦片下载", "Tile Download"));
    ui.horizontal(|ui| {
        ui.label(format!("{}:", app.tr("缩放级别", "Zoom")));
        let tmin = app.tr("最小", "min");
        let tmax = app.tr("最大", "max");
        ui.add(egui::Slider::new(&mut app.min_zoom_dl, 0..=22).text(tmin));
        ui.add(egui::Slider::new(&mut app.max_zoom_dl, 0..=22).text(tmax));
    });
    if app.max_zoom_dl < app.min_zoom_dl {
        app.max_zoom_dl = app.min_zoom_dl;
    }
    let mz = app.active_max_zoom();
    if mz != 0 && app.max_zoom_dl > mz {
        ui.label(egui::RichText::new(format!("{}: {}", app.tr("提示：该源最大缩放为", "Note: source max zoom"), mz)).small().weak());
    }
    ui.label(format!("{}: {}", app.tr("预计瓦片数", "Estimated tiles"), app.estimate()));

    ui.horizontal(|ui| {
        if ui.add_enabled(!app.progress.running, egui::Button::new(app.tr("开始下载", "Start download"))).clicked() {
            app.start_download(ctx);
        }
        if app.progress.running && ui.button(app.tr("取消", "Cancel")).clicked() {
            app.cancel_download();
        }
    });

    let p = &app.progress;
    if p.running || p.ok + p.failed + p.skipped > 0 {
        let done = p.ok + p.failed + p.skipped;
        let frac = if p.total > 0 { done as f32 / p.total as f32 } else { 0.0 };
        ui.add(egui::ProgressBar::new(frac).show_percentage());
        ui.label(format!("{} {}  {} {}  {} {}", app.tr("成功", "ok"), p.ok, app.tr("失败", "failed"), p.failed, app.tr("跳过", "skipped"), p.skipped));
    }

    ui.label(egui::RichText::new(&app.status).small());

    egui::ScrollArea::vertical()
        .max_height(120.0)
        .show(ui, |ui| {
            for line in &app.progress.log {
                ui.label(egui::RichText::new(line).small().weak());
            }
        });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button(app.tr("导出已下载瓦片为 ZIP", "Export tiles as ZIP")).clicked() {
            match app.export_zip() {
                Ok(path) => {
                    app.status = format!("{}: {}", app.tr("已导出 ZIP", "ZIP exported"), path.display());
                    app.last_zip = Some(path.display().to_string());
                }
                Err(e) => app.status = format!("{}: {e}", app.tr("导出失败", "Export failed")),
            }
        }
        if let Some(p) = app.last_zip.clone() {
            if ui.button(app.tr("打开所在文件夹", "Open folder")).clicked() {
                crate::export::open_in_explorer(std::path::Path::new(&p));
            }
        }
    });
}

fn portals_section(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.heading(app.tr("免费数据门户（需注册）", "Free data portals (registration required)"));
    let portals: Vec<(String, String, String)> = app
        .sources
        .iter()
        .filter(|s| s.kind == SourceKind::Portal)
        .map(|s| (s.name.to_string(), s.note.to_string(), s.homepage.to_string()))
        .collect();
    for (name, note, url) in portals {
        ui.horizontal(|ui| {
            ui.label(&name);
            ui.hyperlink_to(&app.tr("打开主页", "open"), &url);
        });
        ui.label(egui::RichText::new(&note).small().weak());
    }
}

fn scenes_section(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.heading(format!("{} ({}/{})", app.tr("影像场景", "Scenes"), app.scenes.len(), app.matched));
    if app.scenes.is_empty() {
        ui.label(egui::RichText::new(app.tr("先在左侧「公开影像检索」检索场景", "Search scenes from the left panel first")).small().weak());
        return;
    }
    let len = app.scenes.len();
    for i in 0..len {
        let sc = app.scenes[i].clone();
        egui::CollapsingHeader::new(format!("{} | {}{}", sc.id, sc.datetime, sc.cloud.map(|c| format!(" | {:>5.1}%云", c)).unwrap_or_default()))
            .default_open(i == 0)
            .show(ui, |ui| {
                ui.label(format!("{}: {} | {}: {}", app.tr("平台", "Platform"), if sc.platform.is_empty() { sc.collection.clone() } else { sc.platform.clone() }, app.tr("波段", "Bands"), sc.bands.len()));
                ui.horizontal_wrapped(|ui| {
                    for b in &sc.bands {
                        let mut on = app.scene_sel_bands.get(i).map(|v| v.contains(&b.name)).unwrap_or(false);
                        if ui.checkbox(&mut on, &b.name).changed() {
                            let list = app.scene_sel_bands.get_mut(i).map(|v| v.clone()).unwrap_or_default();
                            let mut list = list;
                            if on {
                                if !list.contains(&b.name) { list.push(b.name.clone()); }
                            } else {
                                list.retain(|x| x != &b.name);
                            }
                            if let Some(v) = app.scene_sel_bands.get_mut(i) {
                                *v = list;
                            }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button(app.tr("下载所选波段", "Download selected bands")).clicked() {
                        app.scene_download(i);
                    }
                    if !sc.bands.iter().any(|b| b.direct) {
                        ui.label(egui::RichText::new(app.tr("该数据集为 S3 requester-pays，仅复制路径", "S3 requester-pays; paths only")).small().weak());
                    }
                });
            });
        if i > 0 && i % 20 == 0 {
            // 长列表分块提示
            ui.label(egui::RichText::new(format!("{}/{}", i + 1, len)).small().weak());
        }
    }
    if app.scenes.len() < app.matched as usize {
        if ui.button(app.tr("加载更多场景", "Load more scenes")).clicked() {
            app.stac_search(app.sc_page + 1);
        }
    }
    if !app.band_dl_status.is_empty() {
        ui.label(egui::RichText::new(&app.band_dl_status).small().weak());
    }
}

fn ai_section(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.heading(app.tr("AI 助手（接入大模型）", "AI Assistant (LLM)"));
    ui.horizontal(|ui| {
        ui.label(app.tr("Base", "Base"));
        ui.add(egui::TextEdit::singleline(&mut app.ai_base).hint_text("https://api.openai.com/v1"));
    });
    ui.horizontal(|ui| {
        ui.label(app.tr("Key", "Key"));
        ui.add(egui::TextEdit::singleline(&mut app.ai_key).hint_text("sk-... (仅本机保存)").password(true));
    });
    ui.horizontal(|ui| {
        ui.label(app.tr("模型", "Model"));
        ui.add(egui::TextEdit::singleline(&mut app.ai_model).hint_text("gpt-4o-mini").desired_width(100.0));
        if ui.button(app.tr("保存", "Save")).clicked() {
            app.ai_save_cfg();
        }
    });
    ui.label(egui::RichText::new(app.tr("OpenAI 兼容接口均可（OpenAI / DeepSeek / 通义 / 智谱…），Key 仅本机保存。", "Any OpenAI-compatible endpoint. Key stored locally only.")).small().weak());

    let log_len = app.ai_log.len();
    for i in 0..log_len {
        let m = &app.ai_log[i];
        let color = match m.role.as_str() {
            "user" => egui::Color32::from_rgb(120, 170, 255),
            "bot" => egui::Color32::from_rgb(220, 228, 238),
            _ => egui::Color32::from_rgb(125, 139, 154),
        };
        ui.label(egui::RichText::new(&m.text).color(color).small());
    }
    ui.horizontal(|ui| {
        let hint_ai = app.tr("例如：帮我下载杭州市 2024-06 云量≤10% 的 Sentinel-2 B02/B03", "e.g. Download Sentinel-2 B02/B03 of Hangzhou, 2024-06, cloud<=10%");
        ui.add(egui::TextEdit::singleline(&mut app.ai_input).hint_text(hint_ai));
        if ui.add_enabled(!app.ai_busy, egui::Button::new(app.tr("发送", "Send"))).clicked() {
            app.ai_send();
        }
    });
    if app.ai_busy {
        ui.label(egui::RichText::new(app.tr("AI 执行中 ...", "AI working ...")).small().weak());
    }
}

fn time_modal(app: &mut App, ctx: &egui::Context) {
    let mut open = app.time_modal;
    egui::Window::new(app.tr("检索前：选择时间范围与云量", "Before search: time range & cloud"))
        .open(&mut open)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(app.tr("开始", "From"));
                ui.add(egui::TextEdit::singleline(&mut app.sc_date_from).hint_text("2024-06-01").desired_width(110.0));
                ui.label(app.tr("结束", "To"));
                ui.add(egui::TextEdit::singleline(&mut app.sc_date_to).hint_text("2024-06-30").desired_width(110.0));
            });
            ui.horizontal(|ui| {
                ui.label(app.tr("云量上限", "Cloud max"));
                ui.add(egui::Slider::new(&mut app.sc_cloud, 0..=100).text("%"));
            });
            ui.horizontal(|ui| {
                if ui.button(app.tr("开始检索", "Start search")).clicked() {
                    app.stac_search(1);
                }
                if ui.button(app.tr("取消", "Cancel")).clicked() {
                    app.time_modal = false;
                }
            });
        });
    app.time_modal = open;
}

fn custom_source_window(app: &mut App, ctx: &egui::Context) {
    let mut open = app.cs_open;
    egui::Window::new(app.tr("接入自定义地图源（登录后接入）", "Add custom map source (post-login)"))
        .open(&mut open)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new(app.tr("在中科星图/长光等平台登录后，把其瓦片 URL 模板粘贴进来（含 {z}/{x}/{y}），Key 用 {key} 占位。", "After logging in to GeoVIS/JL1 etc., paste their tile URL template with {z}/{x}/{y}; use {key} for the token.")).small().weak());
            ui.horizontal(|ui| {
                ui.label(app.tr("名称", "Name"));
                ui.add(egui::TextEdit::singleline(&mut app.cs_name).desired_width(150.0));
            });
            ui.horizontal(|ui| {
                ui.label(app.tr("URL", "URL"));
                ui.add(egui::TextEdit::singleline(&mut app.cs_url).hint_text("https://.../{z}/{x}/{y}.png?key={key}"));
            });
            ui.horizontal(|ui| {
                ui.label(app.tr("MaxZoom", "MaxZoom"));
                ui.add(egui::Slider::new(&mut app.cs_max, 1..=22));
            });
            ui.horizontal(|ui| {
                ui.label(app.tr("Key", "Key"));
                ui.add(egui::TextEdit::singleline(&mut app.cs_key).hint_text("登录后复制的 Token"));
            });
            ui.horizontal(|ui| {
                if ui.button(app.tr("接入", "Connect")).clicked() {
                    app.cs_add();
                }
                if ui.button(app.tr("取消", "Cancel")).clicked() {
                    app.cs_open = false;
                }
            });
        });
    app.cs_open = open;
}

fn qr_window(app: &mut App, ctx: &egui::Context) {
    let mut open = app.show_alipay;
    egui::Window::new(app.tr("收款码 · 请我喝杯咖啡", "QR · Buy me a coffee"))
        .open(&mut open)
        .resizable(false)
        .show(ctx, |ui| {
            donate::qr(app, ui);
            ui.add_space(2.0);
            ui.label(egui::RichText::new(format!("{}: {EMAIL}", app.tr("联系", "Contact"))).small());
        });
    app.show_alipay = open;
}

fn draw_feature_rings(painter: &egui::Painter, view: &crate::mapview::MapView, rect: &egui::Rect, feats: &[AdmFeature], color: egui::Color32, thickness: f32) {
    for f in feats {
        for ring in &f.outer_rings {
            let pts: Vec<egui::Pos2> = ring.iter().map(|(lng, lat)| {
                mapview::lnglat_to_screen(view, rect, LngLat::new(*lng, *lat))
            }).collect();
            if pts.len() >= 2 {
                painter.add(egui::Shape::line(pts, egui::Stroke::new(thickness, color)));
            }
        }
    }
}

fn draw_scene_footprints(painter: &egui::Painter, view: &crate::mapview::MapView, rect: &egui::Rect, app: &App) {
    let color = egui::Color32::from_rgb(240, 179, 94);
    for sc in &app.scenes {
        for ring in &sc.footprint {
            let pts: Vec<egui::Pos2> = ring.iter().map(|(lng, lat)| {
                mapview::lnglat_to_screen(view, rect, LngLat::new(*lng, *lat))
            }).collect();
            if pts.len() >= 2 {
                painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0_f32, color)));
            }
        }
    }
}

fn map_panel(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    let rect = ui.available_rect_before_wrap();
    let response = ui.interact(rect, ui.id().with("map"), egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(20, 24, 30));

    // 滚轮缩放
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll > 0.0 && app.view.zoom < 22 {
        app.view.zoom += 1;
        app.clear_fetching();
    } else if scroll < 0.0 && app.view.zoom > 0 {
        app.view.zoom -= 1;
        app.clear_fetching();
    }

    // 拖动：框选 or 平移
    if response.drag_started() {
        app.set_drag_start(response.interact_pointer_pos());
    }
    if response.dragged() && app.tool != 1 {
        let d = response.drag_delta();
        if d != egui::Vec2::ZERO {
            let center = rect.center();
            let target = center - d;
            let ll = mapview::screen_to_lnglat(&app.view, &rect, target);
            app.view.center = ll.clamp();
        }
    }

    // 后台抓取可见瓦片
    app.fetch_visible(ctx, &rect);

    // 绘制瓦片
    let view2 = app.view;
    let (x0, y0, x1, y1) = mapview::visible_tiles(&view2, &rect);
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let sr = mapview::tile_screen_rect(&view2, &rect, tx, ty);
            if let Some(tex) = app.get_tile_texture(ctx, view2.zoom, tx, ty) {
                painter.image(
                    tex.id(),
                    sr,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                painter.rect_filled(sr, 0.0, egui::Color32::from_rgb(28, 34, 42));
            }
        }
    }

    // 行政矢量
    if !app.adm_polys.is_empty() {
        let feats = app.adm_polys.clone();
        draw_feature_rings(&painter, &view2, &rect, &feats, egui::Color32::from_rgb(88, 166, 255), 1.5);
    }
    // 场景足迹
    draw_scene_footprints(&painter, &view2, &rect, app);

    // 已确认选区
    if let Some((a, b)) = app.sel_bounds {
        let p1 = mapview::lnglat_to_screen(&view2, &rect, a);
        let p2 = mapview::lnglat_to_screen(&view2, &rect, b);
        let r = egui::Rect::from_two_pos(p1, p2);
        painter.rect_filled(r, 0.0, egui::Color32::from_rgba_unmultiplied(80, 200, 255, 24));
        painter.rect_stroke(r, 0.0, egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(80, 200, 255)), egui::StrokeKind::Outside);
    }

    // 框选拖动中的矩形
    if app.tool == 1 && response.dragged() {
        if let (Some(start), Some(cur)) = (app.drag_start(), response.interact_pointer_pos()) {
            let r = egui::Rect::from_two_pos(start, cur);
            painter.rect_filled(r, 0.0, egui::Color32::from_rgba_unmultiplied(255, 180, 60, 28));
            painter.rect_stroke(r, 0.0, egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(255, 180, 60)), egui::StrokeKind::Outside);
            let a = mapview::screen_to_lnglat(&view2, &rect, r.min);
            let b = mapview::screen_to_lnglat(&view2, &rect, r.max);
            let (w, s, e, n) = (a.lng.min(b.lng), a.lat.min(b.lat), a.lng.max(b.lng), a.lat.max(b.lat));
            painter.text(
                egui::pos2(r.left() + 6.0, r.top() + 4.0),
                egui::Align2::LEFT_TOP,
                format!("{:.3},{:.3} - {:.3},{:.3}", w, s, e, n),
                egui::FontId::proportional(12.0),
                egui::Color32::from_rgb(255, 220, 150),
            );
        }
    }
    if app.tool == 1 && response.drag_stopped() {
        if let (Some(start), Some(cur)) = (app.drag_start(), response.interact_pointer_pos()) {
            let a = mapview::screen_to_lnglat(&view2, &rect, start);
            let b = mapview::screen_to_lnglat(&view2, &rect, cur);
            if (a.lng - b.lng).abs() > 1e-6 && (a.lat - b.lat).abs() > 1e-6 {
                app.set_sel_bbox((a.lng.min(b.lng), a.lat.min(b.lat), a.lng.max(b.lng), a.lat.max(b.lat)));
                app.status = app.tr("已框选区域，设置缩放范围后点「开始下载」", "Region selected; set zoom range then Start download");
            }
        }
        app.set_drag_start(None);
    }

    // 多边形预览 / 加点；点选；点击矢量选区
    if response.clicked() {
        if let Some(p) = response.interact_pointer_pos() {
            let ll = mapview::screen_to_lnglat(&view2, &rect, p);
            match app.tool {
                2 => {
                    if app.poly_pts.len() < 60 {
                        app.poly_pts.push(ll);
                    }
                }
                3 => {
                    app.set_sel_bbox((ll.lng - 0.005, ll.lat - 0.005, ll.lng + 0.005, ll.lat + 0.005));
                    app.status = app.tr("点选区域完成（±0.005 度）", "Point region set (+-0.005 deg)");
                }
                _ => {
                    app.adm_hit_select(&ll);
                }
            }
        }
    }
    if app.tool == 2 {
        let mut pts: Vec<egui::Pos2> = app.poly_pts.iter().map(|ll| mapview::lnglat_to_screen(&view2, &rect, *ll)).collect();
        if let Some(hp) = response.hover_pos() {
            pts.push(hp);
        }
        if pts.len() >= 2 {
            painter.add(egui::Shape::line(pts, egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(240, 179, 94))));
            for p in &app.poly_pts {
                let sp = mapview::lnglat_to_screen(&view2, &rect, *p);
                painter.circle_filled(sp, 3.0, egui::Color32::from_rgb(240, 179, 94));
            }
        }
    }
    if response.double_clicked() && app.tool == 2 && app.poly_pts.len() >= 3 {
        let mut w = f64::MAX; let mut s = f64::MAX; let mut e = f64::MIN; let mut n = f64::MIN;
        for ll in &app.poly_pts {
            w = w.min(ll.lng); e = e.max(ll.lng);
            s = s.min(ll.lat); n = e.max(ll.lat);
        }
        app.set_sel_bbox((w, s, e, n));
        app.poly_pts.clear();
        app.status = app.tr("多边形选区完成", "Polygon region selected");
    }

    // 鼠标坐标与提示
    if let Some(p) = response.hover_pos() {
        let ll = mapview::screen_to_lnglat(&view2, &rect, p);
        let txt = format!("{:.5}°N  {:.5}°E   z{}", ll.lat, ll.lng, view2.zoom);
        painter.text(
            egui::pos2(rect.left() + 8.0, rect.bottom() - 8.0),
            egui::Align2::LEFT_BOTTOM,
            txt,
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(200, 210, 220),
        );
    }
    let hint = match app.tool {
        1 => app.tr("框选模式：拖出矩形（滚轮缩放）", "Rectangle: drag a box (wheel to zoom)"),
        2 => app.tr("多边形模式：单击加点，双击结束", "Polygon: click vertices, double-click to finish"),
        3 => app.tr("点选模式：单击选点", "Point mode: click to pick"),
        _ => app.tr("拖动平移 · 滚轮缩放 · 左侧选工具/矢量/AI", "Drag to pan - wheel to zoom - pick a tool / vector / AI on the left"),
    };
    painter.text(
        egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
        egui::Align2::LEFT_TOP,
        hint,
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(150, 165, 180),
    );

    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 52, 62)), egui::StrokeKind::Inside);
}