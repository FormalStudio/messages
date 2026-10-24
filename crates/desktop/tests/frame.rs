//! The window's frame (sidebar, its search, the conversation header, the composer) under the
//! Fluent and Adwaita presets, drawn by GPUI headless on the demo data and written as PNGs to
//! `$CARGO_TARGET_TMPDIR/frame/`. Runs on macOS (Metal) and Linux (wgpu), where GPUI renders
//! headless.

use std::path::PathBuf;
use std::sync::Arc;

use nd_backend_gpui::GpuiHost;
use nd_gpui::gpui::HeadlessAppContext;
use nd_theme::{OsTheme, Preset, Scheme, ThemeRegistry};

fn settle(cx: &mut HeadlessAppContext, host: &GpuiHost) {
    for _ in 0..6 {
        cx.run_until_parked();
        cx.update(|cx| {
            host.pump(cx);
            host.draw(cx);
        });
    }
    cx.run_until_parked();
}

fn main() {
    if std::env::args().any(|a| a == "--list") {
        println!("frame: test");
        return;
    }
    if nd_gpui::gpui_platform::current_headless_renderer().ok().flatten().is_none() {
        println!("skipped: no headless renderer here");
        return;
    }
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("frame");
    std::fs::create_dir_all(&dir).unwrap();
    let presets: Vec<Preset> = match std::env::var("FRAME_PRESETS") {
        Ok(list) => list.split(',').filter_map(Preset::parse).collect(),
        Err(_) => vec![Preset::Fluent, Preset::Adwaita],
    };
    let sizes = [("wide", messages::DEFAULT_SIZE), ("narrow", (560., 700.))];
    for preset in presets {
        for scheme in [Scheme::Light, Scheme::Dark] {
            for (label, size) in sizes {
                let text = nd_gpui::gpui_platform::current_platform(true).text_system();
                let mut cx = HeadlessAppContext::with_platform(
                    text,
                    Arc::new(()),
                    nd_gpui::gpui_platform::current_headless_renderer,
                );
                let mut window = None;
                let host = cx.update(|cx| {
                    GpuiHost::builder().quit_when_empty(false).start(cx, |app| {
                        window = Some(messages::open_demo_sized(app, size).id());
                    })
                });
                let mut registry = ThemeRegistry::new(preset);
                registry.set_os(OsTheme { scheme: Some(scheme), ..registry.os().clone() });
                host.backend().set_theme(registry);
                settle(&mut cx, &host);
                let window = window.expect("the window opened");
                let capture = host.job(|app| app.capture(window)).expect("capture");
                let name = format!("{}-{}-{label}.png", format!("{preset:?}").to_lowercase(), scheme_name(scheme));
                let out = dir.join(name);
                std::fs::write(&out, &capture.png).unwrap();
                println!("screenshot {}", out.display());
            }
        }
    }
}

fn scheme_name(scheme: Scheme) -> &'static str {
    if scheme == Scheme::Dark { "dark" } else { "light" }
}
