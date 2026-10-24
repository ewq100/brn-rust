//! Production UI handlers plus actual staged restore; synthetic picker and data.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};

struct Fixture {
    _root: tempfile::TempDir,
    backup: PathBuf,
    data: PathBuf,
    parent: PathBuf,
    window: gpui_kit::AnyWindowHandle,
    view: Entity<ThreadsUi>,
}
fn fixture(cx: &mut TestAppContext) -> Fixture {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut source = Workspace::open(root.path().join("source")).unwrap();
    let saved_thread = source.new_thread("Saved waiting work").unwrap();
    source
        .queue_message("saved-queue", &saved_thread.id, "Wait for owner Resume")
        .unwrap();
    let backup = root.path().join("full.sqlite3");
    source.backup(&backup).unwrap();
    let data = root.path().join("current");
    let parent = root.path().join("copies");
    std::fs::create_dir(&parent).unwrap();
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1260.), px(2000.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                    ui.poll = None;
                    let thread = ui.workspace.new_thread("Waiting sentinel").unwrap();
                    ui.workspace
                        .queue_message(
                            "restore-waiting",
                            &thread.id,
                            "Keep waiting without a provider call",
                        )
                        .unwrap();
                    ui.refresh();
                    assert!(!ui.queue_state().paused);
                    ui.settings_section = SettingsSection::Data;
                    ui.layout.open(TileKind::Settings);
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    Fixture {
        _root: root,
        backup,
        data,
        parent,
        window,
        view,
    }
}
fn update(
    f: &Fixture,
    cx: &mut TestAppContext,
    action: impl FnOnce(&mut ThreadsUi, &mut Context<ThreadsUi>),
) {
    cx.update_window(f.window, |_, _, cx| f.view.update(cx, action))
        .unwrap();
}
fn finish(f: &Fixture, cx: &mut TestAppContext) {
    for _ in 0..500 {
        let done = cx
            .update_window(f.window, |_, window, cx| {
                f.view.update(cx, |ui, cx| {
                    ui.poll_worker(window, cx);
                    ui.worker.is_none()
                })
            })
            .unwrap();
        if done {
            cx.run_until_parked();
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("restore did not finish");
}
fn select(f: &Fixture, cx: &mut TestAppContext) {
    update(f, cx, |ui, cx| {
        ui.restore_source_chosen(ui.restore.generation, Ok(Some(vec![f.backup.clone()])), cx);
        ui.restore_parent_chosen(ui.restore.generation, Ok(Some(vec![f.parent.clone()])), cx);
    });
}
fn current_records(f: &Fixture) -> String {
    serde_json::to_string(&Workspace::open(&f.data).unwrap().records().unwrap()).unwrap()
}

#[gpui_kit::test]
fn restore_preview_confirm_reopen_and_source_current_workspace_unchanged(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let original = std::fs::read(&f.backup).unwrap();
    let current = current_records(&f);
    cx.update_window(f.window, |_, window, cx| window.click("restore-select", cx))
        .unwrap();
    cx.simulate_path_prompt_response({
        let backup = f.backup.clone();
        move |_| Some(vec![backup])
    });
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        window.click("restore-destination", cx)
    })
    .unwrap();
    cx.simulate_path_prompt_response({
        let parent = f.parent.clone();
        move |_| Some(vec![parent])
    });
    cx.run_until_parked();
    finish(&f, cx);
    let mut destination = PathBuf::new();
    update(&f, cx, |ui, _| {
        let RestorePhase::Review(prepared) = &ui.restore.phase else {
            panic!("no review: {}", ui.restore.status)
        };
        destination = prepared.destination().to_owned();
        assert!(!destination.exists());
    });
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("restore-confirm").visible());
        assert!(window.find("restore-discard").visible());
        window.click("restore-confirm", cx);
    })
    .unwrap();
    finish(&f, cx);
    update(&f, cx, |ui, _| {
        assert!(matches!(&ui.restore.phase, RestorePhase::Complete(p) if p == &destination));
        assert!(!ui.queue_state().paused);
        assert_eq!(ui.queue_state().queue.len(), 1);
    });
    let restored = Workspace::open(&destination).unwrap();
    assert_eq!(
        restored.store.workspace_purpose().unwrap().as_str(),
        "owner"
    );
    drop(restored);
    assert!(Workspace::open(&destination).is_ok());
    assert_eq!(std::fs::read(&f.backup).unwrap(), original);
    assert_eq!(current_records(&f), current);
    let windows_before = cx.read(|cx| cx.windows().len());
    cx.update_window(f.window, |_, window, cx| window.click("restore-open", cx))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(cx.read(|cx| cx.windows().len()), windows_before + 1);
    let recovered = Workspace::open(&destination).unwrap();
    let RecordData::ChatControl(queue) = recovered.chat_control().unwrap().data else {
        panic!()
    };
    assert!(queue.paused);
    assert_eq!(queue.queue.len(), 1);
    assert_eq!(current_records(&f), current);
    assert_eq!(std::fs::read(&f.backup).unwrap(), original);
    assert!(std::fs::read_dir(&f.parent).unwrap().all(|p| {
        !p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".brn-")
    }));
}

#[gpui_kit::test]
fn cancelled_validation_and_discarded_review_publish_nothing_and_preserve_queue(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let current = current_records(&f);
    select(&f, cx);
    update(&f, cx, |ui, cx| ui.cancel(cx));
    finish(&f, cx);
    assert_eq!(std::fs::read_dir(&f.parent).unwrap().count(), 0);
    assert_eq!(current_records(&f), current);
    select(&f, cx);
    finish(&f, cx);
    update(&f, cx, |ui, cx| ui.discard_restore(cx));
    assert_eq!(std::fs::read_dir(&f.parent).unwrap().count(), 0);
    assert_eq!(current_records(&f), current);
}

#[gpui_kit::test]
fn malformed_backup_failure_can_retry_without_false_success(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let original = std::fs::read(&f.backup).unwrap();
    std::fs::write(&f.backup, b"not a SQLite backup").unwrap();
    select(&f, cx);
    finish(&f, cx);
    update(&f, cx, |ui, _| {
        assert!(matches!(ui.restore.phase, RestorePhase::Idle));
        assert!(ui.restore.status.contains("could not be restored"));
    });
    assert_eq!(std::fs::read_dir(&f.parent).unwrap().count(), 0);
    std::fs::write(&f.backup, original).unwrap();
    select(&f, cx);
    finish(&f, cx);
    update(&f, cx, |ui, _| {
        assert!(matches!(ui.restore.phase, RestorePhase::Review(_)))
    });
}

#[gpui_kit::test]
fn restore_destination_race_preserves_human_file_and_failure_allows_new_selection(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    select(&f, cx);
    finish(&f, cx);
    let mut occupied = PathBuf::new();
    update(&f, cx, |ui, cx| {
        let RestorePhase::Review(prepared) = &ui.restore.phase else {
            panic!()
        };
        occupied = prepared.destination().to_owned();
        std::fs::create_dir(&occupied).unwrap();
        std::fs::write(occupied.join("human"), b"keep").unwrap();
        ui.confirm_restore(cx);
    });
    finish(&f, cx);
    assert_eq!(std::fs::read(occupied.join("human")).unwrap(), b"keep");
    update(&f, cx, |ui, _| {
        assert!(ui.restore.status.contains("did not finish"));
        assert!(matches!(ui.restore.phase, RestorePhase::Idle));
    });
    select(&f, cx);
    finish(&f, cx);
    update(&f, cx, |ui, _| {
        let RestorePhase::Review(prepared) = &ui.restore.phase else {
            panic!()
        };
        assert_ne!(prepared.destination(), occupied);
    });
}

#[gpui_kit::test]
fn stale_cancelled_and_failed_pickers_cannot_start_or_replace_restore(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| window.click("restore-select", cx))
        .unwrap();
    assert!(cx.did_prompt_for_paths());
    cx.simulate_path_prompt_response(|_| None);
    cx.run_until_parked();
    update(&f, cx, |ui, cx| {
        assert!(ui.restore.source.is_none());
        let stale = ui.restore.generation;
        ui.discard_restore(cx);
        ui.restore_source_chosen(stale, Ok(Some(vec![f.backup.clone()])), cx);
        assert!(ui.restore.source.is_none());
        ui.restore_source_chosen(
            ui.restore.generation,
            Err("synthetic picker error".into()),
            cx,
        );
        assert!(ui.restore.status.contains("synthetic picker error"));
        ui.restore_source_chosen(ui.restore.generation, Ok(Some(vec![f.backup.clone()])), cx);
        ui.restore_parent_chosen(ui.restore.generation, Ok(Some(vec![f.data.clone()])), cx);
        assert!(ui.restore.status.contains("outside the current workspace"));
        assert!(ui.worker.is_none());
    });
    assert_eq!(std::fs::read_dir(&f.parent).unwrap().count(), 0);
}

#[test]
fn restore_picker_validation_rejects_nonfiles_relative_paths_and_symlinks() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let file = root.path().join("backup");
    std::fs::write(&file, b"synthetic").unwrap();
    assert!(selected_source(&file).is_ok());
    assert!(selected_source(root.path()).is_err());
    assert!(selected_source(std::path::Path::new("relative")).is_err());
    #[cfg(unix)]
    {
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        assert!(selected_source(&link).is_err());
    }
}

#[gpui_kit::test]
fn disconnected_restore_worker_preserves_waiting_queue_and_allows_retry(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let current = current_records(&f);
    update(&f, cx, |ui, _| {
        let (sender, receiver) = mpsc::sync_channel(1);
        drop(sender);
        ui.restore.phase = RestorePhase::Validating;
        ui.worker = Some(Worker::restore(receiver, CancellationToken::new(), false));
    });
    finish(&f, cx);
    update(&f, cx, |ui, _| {
        assert!(matches!(ui.restore.phase, RestorePhase::Idle));
        assert!(ui.restore.status.contains("unexpectedly"));
        assert!(!ui.queue_state().paused);
        assert_eq!(ui.queue_state().queue.len(), 1);
    });
    assert_eq!(current_records(&f), current);
    select(&f, cx);
    finish(&f, cx);
    update(&f, cx, |ui, _| {
        assert!(matches!(ui.restore.phase, RestorePhase::Review(_)))
    });
}
