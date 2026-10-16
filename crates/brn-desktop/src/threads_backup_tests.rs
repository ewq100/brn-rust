//! Production backup actions with synthetic picker outcomes; no OS picker or provider.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};

struct Fixture {
    _root: tempfile::TempDir,
    data: PathBuf,
    folder: PathBuf,
    window: gpui_kit::AnyWindowHandle,
    view: Entity<ThreadsUi>,
}
fn fixture(cx: &mut TestAppContext) -> Fixture {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("workspace");
    let folder = root.path().join("backups");
    std::fs::create_dir(&folder).unwrap();
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1260.), px(1500.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                    ui.poll = None;
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
        data,
        folder,
        window,
        view,
    }
}
fn finish_worker(f: &Fixture, cx: &mut TestAppContext) {
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
    panic!("backup worker did not finish");
}
fn saved_folder(f: &Fixture) -> Option<String> {
    Workspace::open(&f.data)
        .unwrap()
        .settings()
        .unwrap()
        .backup_directory
}

#[gpui_kit::test]
fn first_picker_saves_then_backs_up_and_saved_click_reloads_folder_and_uses_fresh_name(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        assert_eq!(
            window.find("backup-directory").label(),
            Some("Backup folder not chosen")
        );
        assert_eq!(
            window.find("backup-now").label(),
            Some("Create full backup")
        );
        assert!(window.find("backup-change-folder").visible());
        f.view.update(cx, |ui, cx| {
            ui.backup_folder_chosen(Ok(Some(vec![f.folder.clone()])), true, cx);
            assert_eq!(ui.worker.as_ref().unwrap().kind, WorkerKind::Backup);
            assert_eq!(
                ui.workspace.settings().unwrap().backup_directory.as_deref(),
                f.folder.to_str()
            );
        });
    })
    .unwrap();
    finish_worker(&f, cx);
    let first = std::fs::read_dir(&f.folder)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(first.is_file());
    cx.update_window(f.window, |_, window, cx| {
        assert_eq!(window.find("backup-directory").label(), f.folder.to_str());
        f.view
            .update(cx, |ui, _| ui.settings.backup_directory = None);
        window.click("backup-now", cx);
    })
    .unwrap();
    finish_worker(&f, cx);
    let backups = std::fs::read_dir(&f.folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(backups.len(), 2);
    assert_ne!(backups[0], backups[1]);
    cx.read(|cx| {
        let ui = f.view.read(cx);
        assert!(ui.backup_status.starts_with("Full backup created at "));
        assert!(ui.export_status.contains("No export"));
        // Settings → Data shows the path once, in its status line; the
        // banner above says the backup was made.
        assert!(ui.layout.settings_open());
        assert_eq!(ui.settings_section, SettingsSection::Data);
        assert_eq!(ui.notice, "Full backup created.");
        assert_eq!(
            ui.last_work_notice.as_ref().map(|(_, text)| text.as_str()),
            Some("Full backup created.")
        );
    });
    assert_eq!(saved_folder(&f).as_deref(), f.folder.to_str());
}

#[gpui_kit::test]
fn change_folder_only_saves_and_picker_cancel_error_preserve_preference(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |ui, cx| {
            ui.backup_folder_chosen(Ok(None), true, cx);
            assert!(ui.workspace.settings().unwrap().backup_directory.is_none());
            assert_eq!(ui.backup_status, "No backup created in this session.");
            ui.backup_folder_chosen(Ok(Some(vec![f.folder.clone()])), false, cx);
            assert!(ui.worker.is_none());
            assert_eq!(ui.backup_status, "Backup folder saved.");
            ui.backup_folder_chosen(Ok(None), false, cx);
            assert_eq!(ui.backup_status, "Backup folder saved.");
            ui.backup_folder_chosen(Err("Synthetic picker failure".into()), true, cx);
            assert!(ui.backup_status.contains("Synthetic picker failure"));
            assert!(ui.worker.is_none());
        })
    })
    .unwrap();
    assert_eq!(saved_folder(&f).as_deref(), f.folder.to_str());
    assert_eq!(std::fs::read_dir(&f.folder).unwrap().count(), 0);
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("backup-change-folder").visible())
    })
    .unwrap();
}

#[gpui_kit::test]
fn picker_return_and_backup_click_refuse_busy_without_changing_folder(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let (_sender, receiver) = mpsc::sync_channel(4);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.worker = Some(Worker {
                receiver,
                cancel: CancellationToken::new(),
                conversion_cancel: Arc::new(AtomicBool::new(false)),
                kind: WorkerKind::Import,
                thread: None,
                run_id: None,
            });
            ui.backup_folder_chosen(Ok(Some(vec![f.folder.clone()])), true, cx);
            assert!(ui.workspace.settings().unwrap().backup_directory.is_none());
            assert_eq!(ui.worker.as_ref().unwrap().kind, WorkerKind::Import);
            assert_eq!(ui.notice, WorkerKind::Import.busy_notice());
            ui.choose_output(true, window, cx);
            ui.choose_backup_folder(false, window, cx);
            assert_eq!(ui.worker.as_ref().unwrap().kind, WorkerKind::Import);
            assert_eq!(ui.backup_status, "No backup created in this session.");
        })
    })
    .unwrap();
    assert!(saved_folder(&f).is_none());
    assert_eq!(std::fs::read_dir(&f.folder).unwrap().count(), 0);
}

#[gpui_kit::test]
fn unavailable_saved_folder_reports_failure_without_fallback_and_change_remains_available(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |ui, cx| {
            ui.backup_folder_chosen(Ok(Some(vec![f.folder.clone()])), false, cx);
        })
    })
    .unwrap();
    std::fs::remove_dir(&f.folder).unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| window.click("backup-now", cx))
        .unwrap();
    finish_worker(&f, cx);
    cx.read(|cx| {
        let ui = f.view.read(cx);
        assert!(!ui.backup_status.contains("created at"));
        assert!(ui.backup_status.contains("No such file"));
        assert!(ui.worker.is_none());
    });
    assert!(!f.folder.exists());
    assert_eq!(saved_folder(&f).as_deref(), f.folder.to_str());
    assert!(std::fs::read_dir(&f.data).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("BRN-backup-")
    }));
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("backup-change-folder").visible());
        assert_eq!(window.find("backup-directory").label(), f.folder.to_str());
    })
    .unwrap();
}

#[gpui_kit::test]
fn invalid_selection_keeps_preference_and_change_saves_new_folder_without_backup(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let other = f._root.path().join("other-backups");
    std::fs::create_dir(&other).unwrap();
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |ui, cx| {
            ui.backup_folder_chosen(Ok(Some(vec![f.data.clone()])), true, cx);
            assert!(ui.backup_status.contains("outside the workspace"));
            assert!(ui.worker.is_none());
            assert!(ui.workspace.settings().unwrap().backup_directory.is_none());
            ui.backup_folder_chosen(Ok(Some(vec![f.folder.clone()])), false, cx);
            ui.backup_folder_chosen(Ok(Some(vec![f.data.clone()])), false, cx);
            assert!(ui.backup_status.contains("outside the workspace"));
            assert_eq!(
                ui.workspace.settings().unwrap().backup_directory.as_deref(),
                f.folder.to_str()
            );
            ui.backup_folder_chosen(Ok(Some(vec![other.clone()])), false, cx);
            assert!(ui.worker.is_none());
            assert_eq!(ui.backup_status, "Backup folder saved.");
        })
    })
    .unwrap();
    assert_eq!(saved_folder(&f).as_deref(), other.to_str());
    assert_eq!(std::fs::read_dir(&f.folder).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&other).unwrap().count(), 0);
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert_eq!(window.find("backup-directory").label(), other.to_str())
    })
    .unwrap();
}
