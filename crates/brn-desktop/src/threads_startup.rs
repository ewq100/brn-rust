//! A startup explanation stays visible before the native folder picker opens.
use super::*;

struct UpgradeStartup {
    data_dir: PathBuf,
    choosing: bool,
    error: Option<String>,
}
pub(super) fn open(data_dir: PathBuf, cx: &mut App) -> Result<(), String> {
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(150.), px(150.)),
                size(px(500.), px(280.)),
            ))),
            ..Default::default()
        },
        |window, cx| {
            window.set_window_title("BRN workspace upgrade");
            let view = cx.new(|_| UpgradeStartup {
                data_dir,
                choosing: false,
                error: None,
            });
            cx.new(|cx| Root::new(view, window, cx))
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
impl UpgradeStartup {
    fn choose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.choosing = true;
        self.error = None;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            // On macOS this is the picker button's label, not explanatory text.
            prompt: Some("Choose folder".into()),
        });
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let selection = match receiver.await {
                Ok(result) => result.map_err(|e| e.to_string()),
                Err(error) => Err(error.to_string()),
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.choosing = false;
                let result = selection.and_then(|paths| {
                    crate::startup::with_backup_folder(
                        &this.data_dir,
                        paths.and_then(|paths| paths.into_iter().next()),
                    )
                });
                match result {
                    Ok(Some(workspace)) => {
                        match open_workspace_window(workspace, this.data_dir.clone(), cx) {
                            Ok(()) => window.remove_window(),
                            Err(error) => this.error = Some(error),
                        }
                    }
                    Ok(None) => window.remove_window(),
                    Err(error) => this.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }
}
impl Render for UpgradeStartup {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .child(section("Upgrade this workspace"))
            .child(
                div()
                    .id("upgrade-explanation")
                    .child(crate::startup::BACKUP_PROMPT)
                    .aria_label(crate::startup::BACKUP_PROMPT)
                    .test_support(),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button("upgrade-choose")
                            .label("Choose backup folder…")
                            .primary()
                            .disabled(self.choosing)
                            .on_click(cx.listener(|this, _, window, cx| this.choose(window, cx))),
                    )
                    .child(
                        button("upgrade-cancel")
                            .label("Cancel")
                            .outline()
                            .disabled(self.choosing)
                            .on_click(cx.listener(|_, _, window, _| window.remove_window())),
                    ),
            )
            .children(
                self.error
                    .clone()
                    .map(|error| div().id("upgrade-error").child(error).test_support()),
            )
    }
}

#[cfg(all(test, feature = "native-test-support"))]
mod tests {
    use super::*;
    use gpui_kit::{TestAppContext, test::TestWindowExt};
    #[gpui_kit::test]
    fn startup_explains_then_picker_selection_upgrades_or_cancel_leaves_bytes(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        for choose in [false, true] {
            let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            let data = root.path().join("workspace");
            drop(Workspace::open_without_default_credentials(&data).unwrap());
            let db = data.join(DATABASE_FILENAME);
            rusqlite::Connection::open(&db)
                .unwrap()
                .pragma_update(None, "user_version", 12)
                .unwrap();
            let bytes = std::fs::read(&db).unwrap();
            let backups = root.path().join("backups");
            std::fs::create_dir(&backups).unwrap();
            cx.update(|cx| open(data.clone(), cx).unwrap());
            cx.run_until_parked();
            let window = cx.read(|cx| *cx.windows().last().unwrap());
            cx.update_window(window, |_, window, cx| {
                assert_eq!(
                    window.find("upgrade-explanation").label(),
                    Some(crate::startup::BACKUP_PROMPT)
                );
                assert!(window.find("upgrade-cancel").visible());
                window.click("upgrade-choose", cx);
            })
            .unwrap();
            assert!(cx.did_prompt_for_paths());
            cx.simulate_path_prompt_response({
                let backups = backups.clone();
                move |_| choose.then_some(vec![backups])
            });
            cx.run_until_parked();
            let version: i64 = rusqlite::Connection::open(&db)
                .unwrap()
                .pragma_query_value(None, "user_version", |r| r.get(0))
                .unwrap();
            assert_eq!(version, if choose { 13 } else { 12 });
            assert_eq!(
                std::fs::read_dir(&backups).unwrap().count(),
                usize::from(choose)
            );
            if !choose {
                assert_eq!(std::fs::read(&db).unwrap(), bytes);
            }
        }
    }
}
