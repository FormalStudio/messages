//! The window's frame (sidebar, its search and pins, the conversation header, the details, the
//! composer, the empty and new message states) under the Fluent and Adwaita presets, drawn by
//! GPUI headless on the demo data and written as PNGs to `$CARGO_TARGET_TMPDIR/frame/`. Runs on
//! macOS (Metal) and Linux (wgpu), where GPUI renders headless. nd reads `ND_PRESET` once per
//! process, so each look runs in a child process of its own.

use std::path::PathBuf;
use std::sync::Arc;

use nd_backend_gpui::{GpuiBackend, GpuiChrome, GpuiHost};
use nd_gpui::gpui::HeadlessAppContext;
use nd_theme::{OsTheme, Preset, Scheme, ThemeRegistry};
use nd_ui::semantics::{MouseButton, PointerPhase, SyntheticInput};
use nd_ui::{Point, WindowId};

const ALEX: &str = "iMessage;-;+14155550134";

#[derive(Clone, Copy)]
enum Scene {
    /// Alex's conversation open.
    Chat,
    /// Nothing selected.
    Empty,
    /// The new message screen.
    NewMessage,
    /// A draft in the field and the "+" menu open.
    PlusMenu,
    /// Alex's conversation with the details opened from the conversation's title.
    Details,
    /// Five pinned conversations, Alex's open among them and an unread one.
    Pins,
}

const SCENES: [(&str, Scene, (f32, f32)); 10] = [
    ("wide", Scene::Chat, messages::DEFAULT_SIZE),
    ("narrow", Scene::Chat, (760., 620.)),
    ("paged-list", Scene::Empty, (400., 700.)),
    ("paged-chat", Scene::Chat, (400., 700.)),
    ("empty", Scene::Empty, messages::DEFAULT_SIZE),
    ("new", Scene::NewMessage, messages::DEFAULT_SIZE),
    ("plus", Scene::PlusMenu, messages::DEFAULT_SIZE),
    ("details", Scene::Details, messages::DEFAULT_SIZE),
    ("pins", Scene::Pins, messages::DEFAULT_SIZE),
    ("narrow-pins", Scene::Pins, (760., 620.)),
];

/// Pinned on top of the demo's own pin. The store saves pins to the config file, which the
/// children keep under the target directory.
const PINS: [&str; 4] =
    [ALEX, "iMessage;+;chat881204957120", "iMessage;-;+14155550199", "iMessage;-;priya.natarajan@icloud.com"];

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

fn centre(host: &GpuiHost, window: WindowId, id: &str) -> Option<Point> {
    host.read(|app| {
        let tree = app.semantic_window(window)?;
        let node = tree.root.iter().find(|n| n.test_id.as_deref() == Some(id))?;
        let b = node.bounds?;
        Some(Point { x: b.origin.x + b.size.width / 2., y: b.origin.y + b.size.height / 2. })
    })
}

fn click(cx: &mut HeadlessAppContext, host: &GpuiHost, window: WindowId, at: Point) {
    cx.update(|gcx| {
        host.update(gcx, |app| {
            for phase in [PointerPhase::Down, PointerPhase::Up] {
                let input = SyntheticInput::Pointer { phase, at, button: MouseButton::Left, click_count: 1 };
                app.synthesize(window, &input).expect("posted");
            }
        })
    });
    settle(cx, host);
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
    let Some(preset) = Preset::forced() else {
        let presets = std::env::var("FRAME_PRESETS").unwrap_or_else(|_| "fluent,adwaita".to_owned());
        for name in presets.split(',').filter(|n| !n.is_empty()) {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .env("ND_PRESET", name)
                .env("XDG_CONFIG_HOME", dir.join("config"))
                .status()
                .unwrap();
            assert!(status.success(), "the {name} captures failed");
        }
        return;
    };
    for scheme in [Scheme::Light, Scheme::Dark] {
        for (label, scene, size) in SCENES {
            let text = nd_gpui::gpui_platform::current_platform(true).text_system();
            let mut cx = HeadlessAppContext::with_platform(
                text,
                Arc::new(()),
                nd_gpui::gpui_platform::current_headless_renderer,
            );
            let chat = (!matches!(scene, Scene::Empty)).then_some(ALEX);
            let mut window = None;
            let host = cx.update(|cx| {
                GpuiHost::builder().quit_when_empty(false).start(cx, |app| {
                    // The runtime installs the drawn menus and alerts; a bare host has none.
                    let backend = app.backend_as::<GpuiBackend>().expect("GPUI backend").clone();
                    app.set_global(nd::components::Chrome(GpuiChrome::new(backend)));
                    window = Some(messages::open_demo_with(app, size, chat).id());
                })
            });
            let mut registry = ThemeRegistry::new(preset);
            // Animations off, so every capture shows where an entrance or a fade lands.
            let mut os = OsTheme { scheme: Some(scheme), ..registry.os().clone() };
            os.metrics.reduce_motion = Some(true);
            registry.set_os(os);
            // The app reads its own registry (`cx.theme()`), the backend draws from its copy.
            cx.update(|gcx| host.update(gcx, |app| nd::theme::update_registry(app, |r| *r = registry.clone())));
            host.backend().set_theme(registry);
            cx.update(|gcx| host.update(gcx, |app| app.set_reduce_motion(true)));
            settle(&mut cx, &host);
            let window = window.expect("the window opened");
            match scene {
                Scene::Chat | Scene::Empty => {}
                Scene::Pins => {
                    cx.update(|gcx| {
                        host.update(gcx, |app| {
                            let store = messages::bridge::store(app).expect("the demo store");
                            for guid in PINS {
                                store.toggle_pin(guid);
                            }
                            // The pump that carries store events to the views does not run
                            // headless; hand the list change over directly.
                            messages::bridge::Bridge::deliver(app, Ok(messages_core::store::StoreEvent::ChatList));
                        })
                    });
                    settle(&mut cx, &host);
                    assert!(centre(&host, window, &format!("pinned-{ALEX}")).is_some(), "Alex is pinned");
                }
                Scene::NewMessage => {
                    cx.update(|gcx| {
                        host.update(gcx, |app| messages::root::with_root(app, |root, cx| root.start_new_chat(cx)))
                    });
                    settle(&mut cx, &host);
                }
                Scene::PlusMenu => {
                    let at = centre(&host, window, "composer.attach").expect("the plus button is laid out");
                    click(&mut cx, &host, window, at);
                    assert!(host.backend().menu_open(window), "the plus opened its menu");
                }
                Scene::Details => {
                    let at = centre(&host, window, "thread-identity").expect("the title is laid out");
                    click(&mut cx, &host, window, at);
                    assert!(centre(&host, window, "details-panel").is_some(), "the title opened the details");
                }
            }
            // Reduced motion still fades a page in: wait it out so the capture shows where it lands.
            std::thread::sleep(std::time::Duration::from_millis(400));
            settle(&mut cx, &host);
            let capture = host.job(|app| app.capture(window)).expect("capture");
            let name = format!("{}-{}-{label}.png", format!("{preset:?}").to_lowercase(), scheme_name(scheme));
            let out = dir.join(name);
            std::fs::write(&out, &capture.png).unwrap();
            println!("screenshot {}", out.display());
        }
    }
}

fn scheme_name(scheme: Scheme) -> &'static str {
    if scheme == Scheme::Dark { "dark" } else { "light" }
}
