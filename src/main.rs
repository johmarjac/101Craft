//! 101Craft – a hex and binary editor with templates.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod chrome;
mod dialogs;
mod doc;
mod hexview;
mod i18n;
mod panels;
mod sample;
mod scenes;
mod template;
mod tools;
mod tparse;

use std::path::PathBuf;

fn icon() -> egui::IconData {
    let n = 64usize;
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let i = (y * n + x) * 4;
            let edge = x < 4 || y < 4 || x >= n - 4 || y >= n - 4;
            let cell = (x / 8 + y / 8) % 2 == 0;
            let (r, g, b) = if edge {
                (20, 70, 150)
            } else if cell {
                (45, 120, 215)
            } else {
                (90, 160, 240)
            };
            rgba[i..i + 4].copy_from_slice(&[r, g, b, 255]);
        }
    }
    egui::IconData {
        rgba,
        width: n as u32,
        height: n as u32,
    }
}

fn main() -> eframe::Result {
    let mut screenshot: Option<PathBuf> = None;
    let mut scene: Option<String> = None;
    let mut files = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--screenshot" {
            screenshot = args.next().map(PathBuf::from);
        } else if a == "--scene" {
            scene = args.next();
            if let Some(s) = &scene {
                if !scenes::SCENES.contains(&s.as_str()) {
                    let lang = i18n::load();
                    eprintln!(
                        "{}",
                        tr!(
                            lang,
                            "main.unknownscene",
                            s = s,
                            list = scenes::SCENES.join(", ")
                        )
                    );
                }
            }
        } else if let Some(v) = a.strip_prefix("--screenshot=") {
            screenshot = Some(PathBuf::from(v));
        } else {
            files.push(PathBuf::from(a));
        }
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("101Craft")
        .with_inner_size([1360.0, 820.0])
        .with_min_inner_size([760.0, 480.0])
        .with_icon(icon())
        .with_drag_and_drop(true);
    if screenshot.is_some() {
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }

    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    if screenshot.is_some() {
        options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            builder.with_activate_ignoring_other_apps(false);
        }));
    }

    eframe::run_native(
        "101Craft",
        options,
        Box::new(move |_cc| Ok(Box::new(app::App::new(files, screenshot, scene)))),
    )
}
