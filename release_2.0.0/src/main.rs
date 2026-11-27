//! HD Map Downloader - Enhanced Version
//! Features: Multiple free sources, improved UI, QR code integration

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod donate;
mod download;
mod export;
mod geo;
mod mapview;
mod mosaic;
mod sources;
mod ui;

use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

const SCREENSHOT_FRAMES: u32 = 120;

struct Options {
    screenshot: Option<PathBuf>,
    cache_root: PathBuf,
    donation_path: Option<PathBuf>,
}

fn parse_args() -> Options {
    let mut screenshot = None;
    let mut cache_root = PathBuf::from("hdmap_cache");
    let mut donation_path = std::path::PathBuf::from("resources/assets/donation.png");
    if !donation_path.exists() {
        donation_path = std::path::PathBuf::from("donation.png");
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" => {
                if i + 1 < args.len() {
                    screenshot = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--cache-dir" => {
                if i + 1 < args.len() {
                    cache_root = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--donation" => {
                if i + 1 < args.len() {
                    donation_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!(
                    "用法:\n  hdmapdl\n  hdmapdl --screenshot <file.png>   截图退出\n  hdmapdl --cache-dir <dir>\n  hdmapdl --donation <path>"
                );
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    Options {
        screenshot,
        cache_root,
        donation_path: if donation_path.exists() { Some(donation_path) } else { None },
    }
}

struct MapDl {
    app: app::App,
    frames: Arc<AtomicU32>,
    shot_path: Option<PathBuf>,
    done: bool,
}

impl eframe::App for MapDl {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.frames_drawn += 1;
        if self.app.frames_drawn == 1 {
            if let Some(src) = self.app.active_imagery() {
                crate::mapview::TileCache::ensure_demo_tile(
                    &self.app.cache_root,
                    &src.id,
                    self.app.view.zoom,
                    0,
                    0,
                );
            }
        }
        self.app.tick_events(ui.ctx());
        crate::ui::ui(&mut self.app, ui);

        if let Some(path) = self.shot_path.clone() {
            let n = self.frames.fetch_add(1, Ordering::Relaxed);
            if n < SCREENSHOT_FRAMES {
                ui.ctx().request_repaint();
            } else if !self.done {
                self.done = true;
                ui.ctx().request_repaint();
            }
            if n == SCREENSHOT_FRAMES {
                let p2 = path.clone();
                ui.ctx().request_screenshot(move |img| {
                    let rgba = &img.pixels;
                    let (w, h) = (img.size[0] as u32, img.size[1] as u32);
                    let mut buf = image::RgbaImage::new(w, h);
                    for (idx, px) in rgba.iter().enumerate() {
                        let ix = (idx % w as usize) as u32;
                        let iy = (idx / w as usize) as u32;
                        let [r, g, b, a] = px.to_array();
                        buf.put_pixel(ix, iy, image::Rgba([r, g, b, a]));
                    }
                    if let Some(dir) = p2.parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    match buf.save(&p2) {
                        Ok(()) => println!("screenshot saved: {}", p2.display()),
                        Err(e) => eprintln!("screenshot failed: {e}"),
                    }
                    std::process::exit(0);
                });
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {}
}

fn install_fonts(ctx: &egui::Context) -> Option<String> {
    let mut fonts = egui::FontDefinitions::default();
    let candidates: Vec<String> = vec![
        "C:/Windows/Fonts/NotoSansSC-VF.ttf".into(),
        "C:/Windows/Fonts/simhei.ttf".into(),
        "C:/Windows/Fonts/msyh.ttf".into(),
        "C:/Windows/Fonts/simfang.ttf".into(),
        "C:/Windows/Fonts/segoeui.ttf".into(),
        "D:/探索/HDMapDownloader/resources/assets/fonts/NotoSansSC-Regular.ttf".into(),
    ];
    let mut picked: Option<String> = None;
    for path in candidates {
        let p = PathBuf::from(&path);
        if !p.exists() {
            continue;
        }
        let data = match std::fs::read(&p) {
            Ok(d) if !d.is_empty() => d,
            _ => continue,
        };
        let font_data = egui::FontData::from_owned(data);
        fonts.font_data.insert("cjk".to_string(), font_data.into());
        for family in ["Proportional", "Monospace"] {
            fonts.families.entry(egui::FontFamily::Name(family.into()))
                .or_default()
                .push("cjk".to_string());
        }
        picked = Some(path.clone());
        break;
    }
    ctx.set_fonts(fonts);
    if let Some(p) = &picked {
        eprintln!("[font] 使用中文字体: {p}");
    } else {
        eprintln!("[font] 警告：未找到可用的中文字体，中文会显示为方框");
    }
    picked
}

fn main() -> eframe::Result<()> {
    let opts = parse_args();
    let _ = std::fs::create_dir_all(&opts.cache_root);

    let viewport = if opts.screenshot.is_some() {
        let mut vb = egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([900.0, 600.0])
            .with_active(false)
            .with_visible(true);
        #[cfg(target_os = "windows")]
        {
            vb = vb.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
        vb
    } else {
        egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([900.0, 600.0])
    };

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    let frames = Arc::new(AtomicU32::new(0));
    let state = MapDl {
        app: app::App::new(opts.cache_root.clone(), opts.donation_path.clone()),
        frames: Arc::clone(&frames),
        shot_path: opts.screenshot.clone(),
        done: false,
    };

    eframe::run_native(
        "HD Map Downloader v2.0",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            Ok(Box::new(state))
        }),
    )
}
