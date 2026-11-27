//! 打赏模块 - 安装你的收款码
use crate::app::App;
use egui::ColorImage;

// 你的收款码路径
pub const DEFAULT_QR_PATH: &str = "C:/Users/Administrator/Pictures/Saved Pictures/款码 1.png";

pub fn card(app: &mut App, ui: &mut egui::Ui) {
    egui::Frame::group(ui.style())
        .fill(if ui.visuals().dark_mode { egui::Color32::DARK_GRAY } else { egui::Color32::WHITE })
        .show(ui, |ui| {
            ui.label(egui::RichText::new("自愿打赏").bold());
            ui.add_space(2.0);
            ui.label(egui::RichText::new("本工具完全免费，不存在任何付费限制。打赏纯属自愿，与软件功能无关。").small());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("显示收款码").clicked() {
                    app.show_alipay = true;
                    let ctx = ui.ctx().clone();
                    load_qr(app, &ctx);
                }
                ui.label(egui::RichText::new(format!("路径：{}", DEFAULT_QR_PATH)).small().weak());
            });
            if app.alipay_texture.is_none() {
                ui.label(egui::RichText::new("未找到收款码图片，请检查路径").small().weak());
            }
        });
}

pub fn qr(app: &mut App, ui: &mut egui::Ui) {
    if app.alipay_texture.is_none() {
        let ctx = ui.ctx().clone();
        load_qr(app, &ctx);
    }
    ui.label(egui::RichText::new("感谢支持，请自行确认收款方").small());
    ui.add_space(6.0);
    if let Some(tex) = &app.alipay_texture {
        let s = tex.size_vec2();
        let scale = (320.0 / s.x).min(320.0 / s.y).max(0.1);
        ui.image((tex.id(), s * scale));
        if let Some(p) = &app.alipay_path {
            ui.label(egui::RichText::new(format!("来源：{}", p.display())).small().weak());
        }
    }
}

pub fn load_qr(app: &mut App, ctx: &egui::Context) {
    if app.alipay_texture.is_some() { return; }
    let path = app.alipay_path.clone().unwrap_or_else(|| DEFAULT_QR_PATH.into());
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(img) = image::load_from_memory(&bytes) {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let ci = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw());
            app.alipay_texture = Some(ctx.load_texture("alipay_qr", ci, egui::TextureOptions::LINEAR));
            app.alipay_path = Some(path);
        }
    }
}
