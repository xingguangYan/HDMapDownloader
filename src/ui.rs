//! UI implementation
use crate::app::{App, Crs, ExportFormat, Selection, SelectMode, Tab};
use crate::donate;
use crate::geo::resolution;
use crate::mapview;
use crate::sources::SourceEntry;
use egui::Col;
use egui::Color32;
use egui::RichText;
use egui::Stroke;
use egui::StrokeKind;
use egui::Vec2;

pub fn ui(app: &mut App, ui: &mut egui::Ui) {
    egui::Panel::top("top").show(ui, |ui| { top_bar(app, ui); });
    egui::Panel::bottom("bottom").show(ui, |ui| { bottom_bar(app, ui); });
    egui::Panel::left("left").resizable(true).default_size(280.0).show(ui, |ui| { left_panel(app, ui); });
    egui::Panel::right("right").resizable(true).default_size(300.0).show(ui, |ui| { right_panel(app, ui); });
    egui::CentralPanel::default().show(ui, |ui| {
        match app.tab {
            Tab::Map => central_map(app, ui),
            Tab::Sources => sources_panel(app, ui),
            Tab::Download => download_panel(app, ui),
            Tab::About => about_panel(app, ui),
        }
    });
    process_license_modal(app, ui.ctx());
    process_alipay_modal(app, ui.ctx());
}

fn top_bar(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading("HD Map Downloader v2.0");
        ui.separator();
        ui.selectable_value(&mut app.tab, Tab::Map, "地图");
        ui.selectable_value(&mut app.tab, Tab::Sources, "数据源");
        ui.selectable_value(&mut app.tab, Tab::Download, "下载与导出");
        ui.selectable_value(&mut app.tab, Tab::About, "关于");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("＋").on_hover_text("放大").clicked() {
                mapview::zoom_in(&mut app.view);
            }
            if ui.button("－").on_hover_text("缩小").clicked() {
                mapview::zoom_out(&mut app.view);
            }
            ui.label(format!("级别 {}", app.view.zoom));
            ui.separator();
            ui.label(format!(" {:.5}, {:.5}", app.view.center.lng, app.view.center.lat));
        });
    });
}

fn bottom_bar(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(&app.status);
        ui.separator();
        if app.progress.running {
            let total = app.progress.ok + app.progress.failed;
            let frac = (total.max(1) as f32) / (1000.0); // approximate progress
            ui.add(egui::ProgressBar::new(frac).desired_width(220.0).text(format!("OK {}/{}", total, total)));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("数据源许可各异，请阅读").small().weak());
        });
    });
}

fn left_panel(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.label(RichText::new("影像图层").strong());
    let imagery: Vec<_> = app.sources.iter().filter(|s| s.layer == "imagery" && (s.kind == "xyz" || s.kind == "wmts")).cloned().collect();
    for s in imagery {
        let enabled = app.is_enabled(&s.id);
        ui.horizontal(|ui| {
            let mut on = enabled;
            if ui.checkbox(&mut on, "").changed() {
                app.source_enabled.insert(s.id.clone(), on);
            }
            let label = format!("{} [{}]", s.name, s.license_badge());
            if ui.selectable_label(app.active_source == s.id, label).clicked() {
                app.active_source = s.id.clone();
                app.status = format!("已切换到 {}", s.name);
            }
        });
        ui.add_space(2.0);
    }
    ui.separator();
    ui.label(RichText::new("注记图层").strong());
    let labels: Vec<_> = app.sources.iter().filter(|s| s.layer == "label").cloned().collect();
    for s in labels {
        let mut on = app.is_enabled(&s.id);
        ui.horizontal(|ui| {
            if ui.checkbox(&mut on, "").changed() {
                app.source_enabled.insert(s.id.clone(), on);
                if on {
                    app.layer_labels = true;
                    app.active_label_source = s.id.clone();
                }
            }
            ui.label(&s.name);
        });
    }
    ui.separator();
    ui.label(RichText::new("选择方式").strong());
    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.select_mode, SelectMode::Rect, "矩形");
        ui.selectable_value(&mut app.select_mode, SelectMode::Polygon, "多边形");
    });
    if ui.button("清除选择").clicked() {
        app.selection = Selection::new_rect(LngLat::new(0.0, 0.0), LngLat::new(0.0, 0.0));
        app.selection = Selection::new_rect(LngLat::new(0.0, 0.0), LngLat::new(0.0, 0.0));
    }
}

fn right_panel(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.label(RichText::new("下载设置").strong());
    ui.label("级别范围");
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut app.min_zoom_dl).range(0..=19).prefix("最小 "));
        ui.add(egui::DragValue::new(&mut app.max_zoom_dl).range(0..=19).prefix("最大 "));
    });
    if app.max_zoom_dl < app.min_zoom_dl { app.max_zoom_dl = app.min_zoom_dl; }
    ui.checkbox(&mut app.resume, "断点续传");
    ui.horizontal(|ui| {
        ui.label("并发");
        ui.add(egui::DragValue::new(&mut app.concurrency).range(1..=32));
    });
    ui.add_space(6.0);
    if ui.button("开始下载").clicked() {
        app.start_download(ui.ctx());
    }
    if ui.button("取消").clicked() && app.progress.running {
        app.cancel_download();
    }
    ui.separator();
    ui.label(RichText::new("导出").strong());
    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.export_format, ExportFormat::GeoTiff, "GeoTIFF");
        ui.selectable_value(&mut app.export_format, ExportFormat::Mbtiles, "MBTiles");
    });
    ui.horizontal(|ui| {
        ui.label("文件名");
        ui.text_edit_singleline(&mut app.export_name);
    });
    if ui.button("导出已下载区域").clicked() {
        do_export(app);
    }
    if let Some(p) = &app.last_export {
        ui.label(RichText::new(format!("上次导出：{}", p)).small());
    }
}

fn central_map(app: &mut App, ui: &mut egui::Ui) {
    let size = ui.available_size();
    let (resp, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
    let rect = resp.rect;
    painter.rect_filled(rect, 0.0, mapview::missing_tile_color(ui.visuals()));
    if app.layer_imagery {
        if let Some(src) = app.active_imagery() {
            draw_layer(app, ui, &painter, rect, &src);
        }
    }
    if app.layer_labels {
        if let Some(src) = app.sources.iter().find(|s| s.id == app.active_label_source) {
            draw_layer(app, ui, &painter, rect, src);
        }
    }
    draw_selection(app, &painter, rect);
    if resp.dragged() {
        let d = resp.drag_delta();
        mapview::pan_by_pixels(&mut app.view, d.x, d.y);
        app.cache.clear();
    }
    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.1 {
            if let Some(p) = ui.input(|i| i.pointer.hover_pos()) {
                mapview::zoom_at(&mut app.view, rect, p, scroll);
                app.cache.clear();
            }
        }
    }
    if resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let ll = mapview::lnglat_to_screen(&app.view, rect, p).unwrap_or(app.view.center);
            match app.select_mode {
                SelectMode::Polygon => {
                    if let Selection::Polygon(pts) = &mut app.selection {
                        pts.push(ll);
                        app.status = format!("多边形：{} 个顶点", pts.len());
                    } else {
                        app.selection = Selection::new_rect(ll, ll);
                    }
                }
                SelectMode::Rect => {
                    if let Selection::Rect { a, b } = &mut app.selection {
                        *b = ll;
                    } else {
                        app.selection = Selection::new_rect(ll, ll);
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(p) = ui.input(|i| i.pointer.hover_pos()) {
        if rect.contains(p) {
            let ll = mapview::screen_to_lnglat(&app.view, rect, p).unwrap_or(app.view.center);
            painter.text(rect.left_bottom() + Vec2::new(8.0, -8.0), egui::Align2::LEFT_BOTTOM, format!("{:.5}, {:.5}", ll.lng, ll.lat), egui::FontId::monospace(12.0), Color32::from_white_alpha(220));
        }
    }
    draw_scale_bar(&painter, rect, &app.view);
}

fn draw_layer(app: &App, ui: &mut egui::Ui, painter: &egui::Painter, rect: egui::Rect, src: &SourceEntry) {
    let z = app.view.zoom;
    if src.max_zoom != 0 && z > src.max_zoom {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, format!("该源最高 {}", src.max_zoom), egui::FontId::proportional(14.0), Color32::LIGHT_GRAY);
        return;
    }
    let tiles = mapview::visible_tiles(&app.view, rect, z);
    for t in tiles.iter() {
        let tex = app.cache.get_or_load(ui.ctx(), &app.cache_root, &src.id, t.z, t.x, t.y);
        if let Some(tex_handle) = tex {
            let r = t.screen;
            painter.image(tex_handle.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
        } else if t.z <= 4 {
            let ok = crate::mapview::TileCache::ensure_demo_tile(&app.cache_root, &src.id, t.z, t.x, t.y);
            if let Some(tex) = app.cache.get_or_load(ui.ctx(), &app.cache_root, &src.id, t.z, t.x, t.y) {
                painter.image(tex.id(), t.screen, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                continue;
            }
        }
        let r = t.screen;
        painter.rect_filled(r, 0.0, mapview::missing_tile_color(ui.visuals()));
        painter.rect_stroke(r, 0.0, Stroke::new(0.5, Color32::from_rgb(150, 150, 150)), StrokeKind::Inside);
    }
}

fn draw_selection(app: &App, painter: &egui::Painter, rect: egui::Rect) {
    match &app.selection {
        Selection::Rect { a, b } => {
            let tl = mapview::lnglat_to_screen(&app.view, rect, *a).unwrap_or(egui::pos2(0.0, 0.0));
            let br = mapview::lnglat_to_screen(&app.view, rect, *b).unwrap_or(egui::pos2(0.0, 0.0));
            let r = egui::Rect::from_min_max(egui::pos2(tl.x.min(br.x), tl.y.min(br.y)), egui::pos2(tl.x.max(br.x), tl.y.max(br.y)));
            painter.rect_stroke(r, 4.0, Stroke::new(2.0, Color32::YELLOW), StrokeKind::Inside);
            painter.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(255, 255, 0, 30));
        }
        Selection::Polygon(pts) if pts.len() >= 2 => {
            let screen: Vec<egui::Pos2> = pts.iter().map(|ll| mapview::lnglat_to_screen(&app.view, rect, *ll).unwrap_or(egui::pos2(0.0, 0.0))).collect();
            for i in 1..screen.len() {
                painter.line_segment([screen[i - 1], screen[i]], Stroke::new(2.0, Color32::YELLOW));
            }
            if pts.len() >= 3 {
                painter.line_segment([screen[screen.len() - 1], screen[0]], Stroke::new(2.0, Color32::YELLOW));
            }
        }
        _ => {}
    }
}

fn draw_scale_bar(painter: &egui::Painter, rect: egui::Rect, view: &crate::mapview::MapView) {
    let y = rect.bottom() - 20.0;
    let x0 = rect.left() + 8.0;
    let km_per_px = resolution(view.zoom, view.center.lat) / 1000.0;
    let bar_px = 100.0_f32;
    let bar_km = bar_px * km_per_px as f32;
    let bar_km_round = (bar_km / 5.0).ceil() * 5.0;
    let bar_px_round = (bar_km_round / bar_km * bar_px) as f32;
    let x1 = x0 + bar_px_round;
    painter.line_segment([egui::pos2(x0, y), egui::pos2(x1, y)], Stroke::new(2.0, Color32::WHITE));
    painter.text(egui::pos2(x0, y - 4.0), egui::Align2::LEFT_BOTTOM, format!("{:.0} km", bar_km_round), egui::FontId::monospace(10.0), Color32::WHITE);
}

fn sources_panel(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("数据源列表");
    ui.add_space(4.0);
    egui::ScrollArea::vertical().show(ui, |ui| {
        for s in &app.sources {
            let enabled = app.is_enabled(&s.id);
            ui.horizontal(|ui| {
                let mut on = enabled;
                if ui.checkbox(&mut on, "").changed() {
                    app.source_enabled.insert(s.id.clone(), on);
                }
                ui.vertical(|ui| {
                    ui.label(RichText::new(&s.name).strong());
                    ui.label(RichText::new(format!("{} | {} | z{}-{} | {}", s.layer, s.kind, s.min_zoom, s.max_zoom, s.license_badge())).small());
                    ui.label(RichText::new(&s.license_note).small().weak());
                });
            });
            ui.add_space(4.0);
        }
    });
}

fn download_panel(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("下载队列与进度");
    if app.progress.running {
        ui.label(format!("进行中：{} / {} (成功 {}, 失败 {})", app.progress.ok + app.progress.failed, 1000, app.progress.ok, app.progress.failed));
        for log in app.progress.log.iter().rev().take(10) {
            ui.label(RichText::new(log).small());
        }
    } else {
        ui.label("当前无下载任务");
    }
}

fn about_panel(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("关于");
    ui.add_space(8.0);
    ui.label("HD Map Downloader v2.0");
    ui.label("Rust + eframe/egui");
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);
    ui.label("• 使用公开免费的瓦片源");
    ui.label("• 遵守各数据源的许可");
    ui.label("• 打赏完全自愿、不联网、不记录");
    ui.add_space(8.0);
    donate::card(app, ui);
    ui.add_space(8.0);
    if ui.button("打赏作者").clicked() {
        app.show_alipay = true;
        let ctx = ui.ctx().clone();
        donate::load_qr(app, &ctx);
    }
    ui.add_space(4.0);
    ui.label("源码与问题反馈：见项目仓库");
}

pub fn do_export(app: &mut App) {
    let format_label = app.export_format.label().to_string();
    app.status = format!("正在导出 {}...", format_label);
    let Some((min, max)) = app.selection.bounds() else {
        app.status = "请先选择区域".into();
        return;
    };
    let Some(src) = app.active_imagery() else {
        app.status = "未选择数据源".into();
        return;
    };
    let z = app.max_zoom_dl.min(19);
    let (x0, y0, x1, y1) = crate::download::tiles_in_rect(min.lng, min.lat, max.lng, max.lat, z);
    let count = (x1 - x0 + 1) as usize * (y1 - y0 + 1) as usize;
    if count == 0 {
        app.status = " Selec 区域内没有瓦片".into();
        return;
    }
    if count > 4096 {
        app.status = format!("导出范围过大（{} 块瓦片）", count);
        return;
    }
    let tw = ((x1 - x0 + 1) * 256) as usize;
    let th = ((y1 - y0 + 1) * 256) as usize;
    let mut canvas = image::RgbImage::new(tw as u32, th as u32);
    let mut found = 0;
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let path = crate::download::tile_path(&app.cache_root, &src.id, z, tx, ty);
            if let Ok(bytes) = std::fs::read(&path) {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let rgb = img.to_rgb8();
                    let ox = ((tx - x0) * 256) as u32;
                    let oy = ((ty - y0) * 256) as u32;
                    image::imageops::replace(&mut canvas, &rgb, ox as i64, oy as i64);
                    found += 1;
                }
            }
        }
    }
    if found == 0 {
        app.status = "没有找到已下载的瓦片".into();
        return;
    }
    let raw = canvas.into_raw();
    let out_dir = if app.export_dir.trim().is_empty() {
        app.cache_root.clone()
    } else {
        std::path::PathBuf::from(&app.export_dir)
    };
    let fname = if app.export_name.trim().is_empty() { "export" } else { app.export_name.trim() };
    let result = match app.export_format {
        ExportFormat::GeoTiff => {
            let out = out_dir.join(format!("{}.tif", fname));
            let nw = crate::geo::tile_northwest(x0, y0, z);
            let se = crate::geo::tile_northwest(x1 + 1, y1 + 1, z);
            let spec = crate::export::GeoTiffSpec {
                width: tw as u32,
                height: th as u32,
                origin_x: nw.lng,
                origin_y: nw.lat,
                scale_x: (se.lng - nw.lng) / tw as f64,
                scale_y: (nw.lat - se.lat) / th as f64,
                epsg: Some(match app.crs {
                    app::Crs::Wgs84 => 4326,
                    app::Crs::WebMercator => 3857,
                    app::Crs::Cgcs2000Geo => 4490,
                    app::Crs::Cgcs2000Gk => 4490,
                }),
                description: format!("导出自 HD Map Downloader - {}", src.name),
            };
            crate::export::write_geotiff(&out, &spec, &raw).map(|_| out.display().to_string())
        }
        ExportFormat::Mbtiles => {
            let out = out_dir.join(format!("{}.mbtiles", fname));
            let mut tiles = vec![];
            for ty in y0..=y1 {
                for tx in x0..=x1 {
                    let path = crate::download::tile_path(&app.cache_root, &src.id, z, tx, ty);
                    if let Ok(b) = std::fs::read(&path) {
                        tiles.push(crate::export::MbtilesTile { z, x: tx, y: ty, png: b });
                    }
                }
            }
            let mut meta = std::collections::BTreeMap::new();
            meta.insert("name".into(), src.name.clone());
            meta.insert("description".into(), "导出自 HD Map Downloader".into());
            meta.insert("bounds".into(), format!("{},{},{},{}", min.lng, min.lat, max.lng, max.lat));
            meta.insert("attribution".into(), format!("数据源：{}", src.name));
            crate::export::write_mbtiles(&out, &tiles, &meta).map(|_| out.display().to_string())
        }
    };
    match result {
        Ok(p) => {
            app.last_export = Some(p.clone());
            app.status = format!("已导出 {} ({:?} 块瓦片)", p, found);
        }
        Err(e) => {
            app.status = format!("导出失败: {}", e);
        }
    }
}

fn process_license_modal(app: &mut App, ctx: &egui::Context) {
    if let Some(id) = app.show_license_for.take() {
        if let Some(src) = app.sources.iter().find(|s| s.id == id) {
            app.license_text = format!("\n{}\n\n许可：{}\n说明：{}\n官网：{}", src.name, src.license, src.license_note, src.url(0,0,0));
        }
    }
    if !app.license_text.is_empty() {
        let mut text = std::mem::take(&mut app.license_text);
        egui::Window::new("许可信息").show(ctx, |ui| {
            ui.add(egui::TextEdit::multiline(&mut text).desired_width(500.0).desired_rows(10).interactive(false));
            if ui.button("关闭").clicked() {
                app.show_license_open = false;
            }
        });
        if !app.show_license_open {
            app.license_text.clear();
        } else {
            app.license_text = text;
        }
    }
}

fn process_alipay_modal(app: &mut App, ctx: &egui::Context) {
    if app.show_alipay {
        let mut closed = false;
        let tex = app.alipay_texture.as_ref().map(|t| (t.id(), t.size_vec2()));
        egui::Window::new("打赏作者（纯自愿）").open(&mut app.show_alipay).show(ctx, |ui| {
            ui.label("感谢支持！本工具完全免费开源。");
            ui.add_space(4.0);
            if let Some((id, size)) = tex {
                ui.image((id, size));
            } else {
                ui.label(RichText::new("[未找到收款码图片]").weak());
            }
            if ui.button("关闭").clicked() { closed = true; }
        });
        if closed {
            app.show_alipay = false;
        }
    }
}
