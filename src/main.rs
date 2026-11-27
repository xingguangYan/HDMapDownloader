//! HD 高清地图下载器 v3.0 —— 桌面端入口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod donate;
mod download;
mod export;
mod geo;
mod mapview;
mod sources;
mod ui;

use eframe::egui;

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // simhei.ttf 在中文 Windows 上基本都存在；msyh.ttc 为雅黑（集合字体）
    let candidates = [
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/msyh.ttf",
        "C:/Windows/Fonts/NotoSansSC-Regular.ttf",
        "C:/Windows/Fonts/NotoSansSC-VF.ttf",
    ];
    for path in candidates {
        let p = std::path::Path::new(path);
        if !p.exists() {
            continue;
        }
        if let Ok(data) = std::fs::read(p) {
            if data.is_empty() {
                continue;
            }
            fonts.font_data.insert("cjk".into(), egui::FontData::from_owned(data).into());
            for family in ["Proportional", "Monospace"] {
                fonts
                    .families
                    .entry(egui::FontFamily::Name(family.into()))
                    .or_default()
                    .push("cjk".into());
            }
            break;
        }
    }
    ctx.set_fonts(fonts);
}

struct MapDl {
    app: app::App,
}

impl eframe::App for MapDl {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.tick_events();
        self.app.frames += 1;
        ui::ui(&mut self.app, ctx);
    }
}

fn main() -> eframe::Result<()> {
    let cache = std::path::PathBuf::from("hdmap_cache");
    let _ = std::fs::create_dir_all(&cache);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([960.0, 640.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "HD 高清地图下载器 v3.0",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            Ok(Box::new(MapDl { app: app::App::new(cache) }))
        }),
    )
}
