//! 打赏模块 - 二维码已内嵌进二进制，永不丢失
use crate::app::App;
use egui::ColorImage;

// 二维码直接编译进程序，路径找不到也能显示
pub const QR_BYTES: &[u8] = include_bytes!("../assets/qr.png");

// 用户本地的收款码路径（可选，优先使用）
pub const USER_QR_PATH: &str = "C:/Users/Administrator/Pictures/Saved Pictures/收款码1.png";

pub fn card(app: &mut App, ui: &mut egui::Ui) {
    egui::Frame::group(ui.style())
        .fill(if ui.visuals().dark_mode { egui::Color32::from_rgb(26,33,41) } else { egui::Color32::from_rgb(248,249,251) })
        .show(ui, |ui| {
            ui.label(egui::RichText::new("自愿打赏").strong());
            ui.add_space(2.0);
            ui.label(egui::RichText::new("本工具完全免费，功能不受任何限制。打赏纯属自愿，与下载速度、数据量、功能开关都没有关系。").small());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("显示收款码").clicked() {
                    app.show_alipay = true;
                    let ctx = ui.ctx().clone();
                    load_qr(app, &ctx);
                }
            });
            if app.alipay_texture.is_none() {
                ui.label(egui::RichText::new("点击上方按钮查看收款码").small().weak());
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
    }
}

pub fn load_qr(app: &mut App, ctx: &egui::Context) {
    if app.alipay_texture.is_some() { return; }

    // 优先尝试用户本地图片
    let p = std::path::PathBuf::from(USER_QR_PATH);
    let bytes: Vec<u8> = if p.exists() {
        std::fs::read(&p).unwrap_or_else(|_| QR_BYTES.to_vec())
    } else {
        // 回退到内嵌图片
        QR_BYTES.to_vec()
    };

    if let Ok(img) = image::load_from_memory(&bytes) {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let ci = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw());
        app.alipay_texture = Some(ctx.load_texture("alipay_qr", ci, egui::TextureOptions::LINEAR));
    }
}
