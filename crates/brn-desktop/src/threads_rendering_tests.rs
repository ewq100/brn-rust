//! Opt-in Linux rendering witness using the real GPUI renderer and text system.
//! This proves Linux pixels and checked Save, not macOS rendering or OS integration.
#![cfg(target_os = "linux")]

use super::*;
use gpui_kit::{HeadlessAppContext, test::TestWindowExt};

#[test]
#[ignore = "requires a usable Vulkan/OpenGL renderer, including software Vulkan"]
fn linux_rendered_note_edit_save_and_reopen() {
    const ORIGINAL: &str = "Original harbor note.";
    const EDITED: &str = "Edited harbor note: õun.";
    const WIDTH: u32 = 1260;
    const HEIGHT: u32 = 860;

    // Reserve a new directory when artifacts are requested. An existing path is
    // refused, so a rerun cannot replace user files or reuse stale screenshots.
    let artifacts = std::env::var_os("BRN_UI_SCREENSHOT_DIR").map(|directory| {
        let directory = PathBuf::from(directory);
        assert!(
            directory.is_absolute(),
            "screenshot directory must be absolute"
        );
        std::fs::create_dir(&directory)
            .expect("BRN_UI_SCREENSHOT_DIR must name a new directory in an existing parent");
        directory
    });

    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("linux-rendered-note");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let original = workspace
        .new_note("Linux rendering fixture", ORIGINAL)
        .unwrap();
    let note_id = original.id.clone();

    let mut cx = HeadlessAppContext::with_platform(
        gpui_kit::platform::current_platform(true).text_system(),
        Arc::new(gpui_kit::assets::Assets),
        gpui_kit::platform::current_headless_renderer,
    );
    cx.update(gpui_kit::init);
    let (handle, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(WIDTH as f32), px(HEIGHT as f32)),
                ))),
                focus: false,
                show: false,
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut view = ThreadsUi::new(workspace, data.clone(), window, cx);
                    view.poll = None;
                    view.open_note(&note_id, None, window, cx);
                    view.layout.open(TileKind::Note);
                    view.layout.toggle_maximize(TileKind::Note);
                    view
                })
            },
        )
        .expect("real Linux headless window must open")
    });
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("note-write", cx);
        assert!(
            !view.read(cx).note_reading,
            "Write must expose the actual note editor before keyboard input"
        );
    })
    .unwrap();
    cx.run_until_parked();
    let (body_bounds, scale_factor, device_size) = cx
        .update_window(handle, |_, window, cx| {
            window.blur(cx);
            window.render_frame(cx);
            assert!(window.find("note-body").visible());
            let logical_size = window.bounds().size;
            assert_eq!(logical_size, size(px(WIDTH as f32), px(HEIGHT as f32)));
            let scale_factor = window.scale_factor();
            assert!(scale_factor.is_finite() && scale_factor > 0.);
            (
                window.find("note-body").bounds(),
                scale_factor,
                logical_size.to_device_pixels(scale_factor),
            )
        })
        .unwrap();
    let before = cx
        .capture_screenshot(handle)
        .expect("Linux must capture real pixels; renderer absence is a failure");
    let device_width = u32::from(device_size.width);
    let device_height = u32::from(device_size.height);
    assert_eq!(before.dimensions(), (device_width, device_height));
    let first_pixel = before.get_pixel(0, 0);
    assert!(
        before.pixels().any(|pixel| pixel != first_pixel),
        "the workspace screenshot must contain more than a blank clear color"
    );
    if let Some(directory) = &artifacts {
        before.save(directory.join("01-saved-note.png")).unwrap();
    }

    // Select only as fixture preparation; the replacement itself travels through
    // the focused editor's real Unicode keyboard/input dispatch.
    cx.update_window(handle, |_, window, cx| {
        let editor = view.read(cx).note_editor.clone();
        editor.update(cx, |editor, cx| {
            editor.focus(window, cx);
            editor.set_selected_range(0..ORIGINAL.len(), cx);
        });
        window.input(EDITED, cx);
        window.blur(cx);
        window.render_frame(cx);
    })
    .unwrap();
    cx.run_until_parked();
    let edited_bounds = cx
        .update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(window.scale_factor(), scale_factor);
            let current = view.read(cx);
            assert_eq!(current.note_editor.read(cx).value().as_ref(), EDITED);
            assert!(current.note.as_ref().unwrap().session.is_some());
            assert_eq!(
                current.workspace.store.note(&note_id).unwrap().unwrap(),
                original,
                "typing must leave the saved record unchanged until Save"
            );
            assert!(current.worker.is_none(), "fixture must remain offline");
            assert!(window.find("save-note").visible());
            window.find("note-body").bounds()
        })
        .unwrap();
    let edited = cx.capture_screenshot(handle).unwrap();
    assert_eq!(edited.dimensions(), before.dimensions());

    // Count changes inside the common document body, away from dirty badges and
    // Save controls. A cursor alone cannot satisfy this text-rasterization check.
    let left =
        (f32::from(body_bounds.origin.x.max(edited_bounds.origin.x)) * scale_factor).ceil() as u32;
    let top =
        (f32::from(body_bounds.origin.y.max(edited_bounds.origin.y)) * scale_factor).ceil() as u32;
    let right =
        (f32::from(body_bounds.right().min(edited_bounds.right())) * scale_factor).floor() as u32;
    let bottom =
        (f32::from(body_bounds.bottom().min(edited_bounds.bottom())) * scale_factor).floor() as u32;
    assert!(left < right && top < bottom);
    assert!(right <= device_width && bottom <= device_height);
    let changed_body_pixels = (top..bottom)
        .flat_map(|y| (left..right).map(move |x| (x, y)))
        .filter(|&(x, y)| before.get_pixel(x, y) != edited.get_pixel(x, y))
        .count();
    assert!(
        changed_body_pixels > 100,
        "typing must change rendered note text; changed {changed_body_pixels} body pixels"
    );
    if let Some(directory) = &artifacts {
        edited.save(directory.join("02-unsaved-edit.png")).unwrap();
    }

    cx.update_window(handle, |_, window, cx| window.click("save-note", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.blur(cx);
        window.render_frame(cx);
        assert!(window.try_find("save-note").is_none());
        let current = view.read(cx);
        assert!(current.note.as_ref().unwrap().session.is_none());
        let saved = current.workspace.store.note(&note_id).unwrap().unwrap();
        assert!(saved.version > original.version);
        let RecordData::Note(note) = saved.data else {
            panic!("saved record must remain a note")
        };
        assert_eq!(note.markdown, EDITED);
    })
    .unwrap();
    let saved = cx.capture_screenshot(handle).unwrap();
    if let Some(directory) = &artifacts {
        saved.save(directory.join("03-saved-edit.png")).unwrap();
        eprintln!("Linux UI screenshots: {}", directory.display());
    }
    eprintln!("Linux rendered edit changed {changed_body_pixels} document-body pixels");

    drop(view);
    drop(cx);
    let reopened = Workspace::open(&data).unwrap();
    let saved = reopened.store.note(&note_id).unwrap().unwrap();
    let RecordData::Note(note) = saved.data else {
        panic!("reopened record must remain a note")
    };
    assert_eq!(
        note.markdown, EDITED,
        "UI Save must survive workspace restart"
    );
}
