//! 界面布局：顶栏 + 左侧控制面板 + 中央地图
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
            ui.heading("HD 高清地图下载器 v3.0");
            ui.separator();
            ui.label(format!("当前源: {}", app.active().map(|s| s.name).unwrap_or("(未选择)")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("联系方式: {EMAIL}")).small().weak());
            });
        });
    });

    egui::SidePanel::left("side")
        .default_width(340.0)
        .width_range(280.0..=440.0)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                source_section(app, ui);
                time_section(app, ui);
                select_section(app, ui);
                download_section(app, ui, ctx);
                portals_section(app, ui);
                donate::card(app, ui);
                ui.add_space(6.0);
                ui.label(egui::RichText::new(format!("底部联系: {EMAIL}")).small().weak());
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        map_panel(app, ui, ctx);
    });

    if app.show_alipay {
        qr_window(app, ctx);
    }
}

fn source_section(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("数据源");
    let items: Vec<(String, String, String, bool)> = app
        .sources
        .iter()
        .map(|s| (s.id.to_string(), s.name.to_string(), s.group.to_string(), s.requires_key))
        .collect();
    let selected_name = app
        .active()
        .map(|s| s.name.to_string())
        .unwrap_or_else(|| "(未选择)".to_string());
    let prev = app.active_source.clone();
    egui::ComboBox::from_label("选择地图源")
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
        });
    if app.active_source != prev {
        app.on_source_changed();
    }

    // 先把展示信息复制出来，避免和 &mut app.api_key 的可变借用冲突
    let info = app.active().map(|s| {
        let license = format!("许可: {} | {}", s.license, s.note);
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
            ui.hyperlink_to("数据源主页", h);
        }
        if needs_key {
            ui.add_space(2.0);
            ui.label(egui::RichText::new("密钥（tk / API Key）:").small());
            ui.add(egui::TextEdit::singleline(&mut app.api_key).hint_text("粘贴你的 tk / API Key"));
            if let Some(h) = &homepage {
                ui.hyperlink_to("申请地址", h);
            }
        }
    }
}

fn time_section(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.heading("影像时间（时间序列源可用）");
    let has_time = app.active().map(|s| s.time_support).unwrap_or(false);
    ui.add_enabled(has_time, egui::TextEdit::singleline(&mut app.date).hint_text("2024-06-01"));
    if has_time {
        ui.label(egui::RichText::new("格式 YYYY-MM-DD，MODIS / Landsat / Sentinel-2(HLS) 按日期加载").small().weak());
    } else {
        ui.label(egui::RichText::new("当前源不支持时间参数").small().weak());
    }
}

fn select_section(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.heading("框选区域");
    let label = if app.select_mode { "关闭框选模式" } else { "开启框选模式（在地图上拖矩形）" };
    if ui.button(label).clicked() {
        app.select_mode = !app.select_mode;
    }
    if let Some((min, max)) = app.sel_bounds {
        let (w, s, e, n) = (
            min.lng.min(max.lng), min.lat.min(max.lat),
            min.lng.max(max.lng), min.lat.max(max.lat),
        );
        ui.label(format!("西 {w:.4}  南 {s:.4}"));
        ui.label(format!("东 {e:.4}  北 {n:.4}"));
        if ui.button("清除选区").clicked() {
            app.sel_bounds = None;
        }
    } else {
        ui.label(egui::RichText::new("尚未框选").small().weak());
    }
}

fn download_section(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.add_space(6.0);
    ui.heading("下载");
    ui.horizontal(|ui| {
        ui.label("缩放级别:");
        ui.add(egui::Slider::new(&mut app.min_zoom_dl, 0..=22).text("最小"));
        ui.add(egui::Slider::new(&mut app.max_zoom_dl, 0..=22).text("最大"));
    });
    if app.max_zoom_dl < app.min_zoom_dl {
        app.max_zoom_dl = app.min_zoom_dl;
    }
    if let Some(s) = app.active() {
        if s.max_zoom != 0 && app.max_zoom_dl > s.max_zoom {
            ui.label(egui::RichText::new(format!("提示：该源最大缩放为 {}", s.max_zoom)).small().weak());
        }
    }
    ui.label(format!("预计瓦片数: {}", app.estimate()));

    ui.horizontal(|ui| {
        if ui.add_enabled(!app.progress.running, egui::Button::new("开始下载")).clicked() {
            app.start_download(ctx);
        }
        if app.progress.running && ui.button("取消").clicked() {
            app.cancel_download();
        }
    });

    let p = &app.progress;
    if p.running || p.ok + p.failed + p.skipped > 0 {
        let done = p.ok + p.failed + p.skipped;
        let frac = if p.total > 0 { done as f32 / p.total as f32 } else { 0.0 };
        ui.add(egui::ProgressBar::new(frac).show_percentage());
        ui.label(format!("成功 {}  失败 {}  跳过 {}", p.ok, p.failed, p.skipped));
    }

    ui.label(egui::RichText::new(&app.status).small());

    egui::ScrollArea::vertical()
        .max_height(130.0)
        .show(ui, |ui| {
            for line in &app.progress.log {
                ui.label(egui::RichText::new(line).small().weak());
            }
        });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("导出已下载瓦片为 ZIP").clicked() {
            match app.export_zip() {
                Ok(path) => {
                    app.status = format!("已导出 ZIP: {}", path.display());
                    app.last_zip = Some(path.display().to_string());
                }
                Err(e) => app.status = format!("导出失败: {e}"),
            }
        }
        if let Some(p) = app.last_zip.clone() {
            if ui.button("打开所在文件夹").clicked() {
                crate::export::open_in_explorer(std::path::Path::new(&p));
            }
        }
    });
}

fn portals_section(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.heading("免费数据门户（需注册）");
    let portals: Vec<(String, String, String)> = app
        .sources
        .iter()
        .filter(|s| s.kind == SourceKind::Portal)
        .map(|s| (s.name.to_string(), s.note.to_string(), s.homepage.to_string()))
        .collect();
    for (name, note, url) in portals {
        ui.horizontal(|ui| {
            ui.label(&name);
            ui.hyperlink_to("打开主页", &url);
        });
        ui.label(egui::RichText::new(&note).small().weak());
    }
}

fn qr_window(app: &mut App, ctx: &egui::Context) {
    let mut open = app.show_alipay;
    egui::Window::new("收款码 · 请我喝杯咖啡")
        .open(&mut open)
        .resizable(false)
        .show(ctx, |ui| {
            donate::qr(app, ui);
            ui.add_space(2.0);
            ui.label(egui::RichText::new(format!("联系: {EMAIL}")).small());
        });
    app.show_alipay = open;
}

fn map_panel(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    let rect = ui.available_rect_before_wrap();
    let response = ui.interact(rect, ui.id().with("map"), egui::Sense::drag());
    let painter = ui.painter_at(rect);

    // 深色底
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
    let view = app.view;
    if response.drag_started() {
        app.set_drag_start(response.interact_pointer_pos());
    }
    if response.dragged() && !app.select_mode {
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
    let (x0, y0, x1, y1) = mapview::visible_tiles(&view, &rect);
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let sr = mapview::tile_screen_rect(&view, &rect, tx, ty);
            if let Some(tex) = app.get_tile_texture(ctx, view.zoom, tx, ty) {
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

    // 已确认选区
    if let Some((a, b)) = app.sel_bounds {
        let p1 = mapview::lnglat_to_screen(&view, &rect, a);
        let p2 = mapview::lnglat_to_screen(&view, &rect, b);
        let r = egui::Rect::from_two_pos(p1, p2);
        painter.rect_filled(r, 0.0, egui::Color32::from_rgba_unmultiplied(80, 200, 255, 24));
        painter.rect_stroke(r, 0.0, egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(80, 200, 255)), egui::StrokeKind::Outside);
    }

    // 框选过程中的矩形
    if app.select_mode && response.dragged() {
        if let (Some(start), Some(cur)) = (app.drag_start(), response.interact_pointer_pos()) {
            let r = egui::Rect::from_two_pos(start, cur);
            painter.rect_filled(r, 0.0, egui::Color32::from_rgba_unmultiplied(255, 180, 60, 28));
            painter.rect_stroke(r, 0.0, egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(255, 180, 60)), egui::StrokeKind::Outside);
            let a = mapview::screen_to_lnglat(&view, &rect, r.min);
            let b = mapview::screen_to_lnglat(&view, &rect, r.max);
            let (w, s, e, n) = (a.lng.min(b.lng), a.lat.min(b.lat), a.lng.max(b.lng), a.lat.max(b.lat));
            painter.text(
                egui::pos2(r.left() + 6.0, r.top() + 4.0),
                egui::Align2::LEFT_TOP,
                format!("西 {w:.3} 南 {s:.3}  东 {e:.3} 北 {n:.3}"),
                egui::FontId::proportional(12.0),
                egui::Color32::from_rgb(255, 220, 150),
            );
        }
    }
    // 松手后确认选区
    if app.select_mode && response.drag_stopped() {
        if let (Some(start), Some(cur)) = (app.drag_start(), response.interact_pointer_pos()) {
            let a = mapview::screen_to_lnglat(&view, &rect, start);
            let b = mapview::screen_to_lnglat(&view, &rect, cur);
            if (a.lng - b.lng).abs() > 1e-9 && (a.lat - b.lat).abs() > 1e-9 {
                app.sel_bounds = Some((LngLat::new(a.lng, a.lat), LngLat::new(b.lng, b.lat)));
                app.status = "已框选区域，设置缩放范围后点「开始下载」".to_string();
            }
        }
        app.set_drag_start(None);
    }

    // 鼠标坐标与提示
    if let Some(p) = response.hover_pos() {
        let ll = mapview::screen_to_lnglat(&view, &rect, p);
        let txt = format!("{:.5}°N  {:.5}°E   缩放 z{}", ll.lat, ll.lng, view.zoom);
        painter.text(
            egui::pos2(rect.left() + 8.0, rect.bottom() - 8.0),
            egui::Align2::LEFT_BOTTOM,
            txt,
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(200, 210, 220),
        );
    }
    let hint = if app.select_mode {
        "框选模式：拖出矩形选择下载区域（滚轮缩放）"
    } else {
        "拖动平移地图 · 滚轮缩放 · 开启左侧「框选模式」后拖出下载区域"
    };
    painter.text(
        egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
        egui::Align2::LEFT_TOP,
        hint,
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(150, 165, 180),
    );

    // 给 map 区域一个边框，视觉上与侧栏分隔
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 52, 62)), egui::StrokeKind::Inside);
}
