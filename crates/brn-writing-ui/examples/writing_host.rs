//! Disposable, offline F18 development host. No owner data or provider transport.
#![recursion_limit = "256"]
#[path = "writing_host/fixture.rs"]
mod fixture;
#[path = "writing_host/host.rs"]
mod host;
#[cfg(all(test, feature = "native-test-support"))]
#[path = "writing_host/journey.rs"]
mod journey;

use gpui_kit::{AppContext, WindowOptions, px, size};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture::Fixture::new()?;
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.bind_keys(host::bindings());
            brn_writing_ui::theme::apply(Default::default(), cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.open_window(
                WindowOptions {
                    window_min_size: Some(size(px(1000.), px(800.))),
                    ..Default::default()
                },
                move |window, cx| {
                    let view = cx.new(|cx| host::WritingHost::new(fixture, window, cx));
                    cx.new(|cx| gpui_kit::component::Root::new(view, window, cx))
                },
            )
            .expect("open disposable writing host");
        });
    Ok(())
}
