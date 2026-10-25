//! Recovery copies use the core's existing staged restore. The open workspace
//! is never replaced, and its queue is not paused/dispatched by recovery work.
use super::*;
use brn_threads_app::{PreparedRestore, Store};

#[derive(Default)]
pub(super) struct RestoreFlow {
    generation: u64,
    source: Option<PathBuf>,
    phase: RestorePhase,
    status: String,
}
#[derive(Default)]
enum RestorePhase {
    #[default]
    Idle,
    Validating,
    Review(PreparedRestore),
    Publishing(PathBuf),
    Complete(PathBuf),
}
impl ThreadsUi {
    pub(super) fn restore_section(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let busy = self.worker.is_some();
        let review = matches!(self.restore.phase, RestorePhase::Review(_));
        let destination = match &self.restore.phase {
            RestorePhase::Review(prepared) => Some(prepared.destination()),
            RestorePhase::Publishing(path) | RestorePhase::Complete(path) => Some(path.as_path()),
            _ => None,
        };
        let mut panel = div().id("settings-restore").flex().flex_col().gap_2()
            .child("Restore a full backup")
            .child("Create a separate recovery copy. This workspace and the backup file stay unchanged.")
            .child(button("restore-select").label("Choose backup…").outline().disabled(busy)
                .on_click(cx.listener(|this, _, window, cx| this.choose_restore_source(window, cx))))
            .child(div().id("restore-source").child(self.restore.source.as_ref()
                .map(|p| p.display().to_string()).unwrap_or_else(|| "No backup selected".into())).test_support());
        if self.restore.source.is_some()
            && !review
            && !matches!(self.restore.phase, RestorePhase::Publishing(_))
        {
            panel = panel.child(
                button("restore-destination")
                    .label("Choose recovery folder…")
                    .outline()
                    .disabled(busy)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.choose_restore_parent(window, cx)),
                    ),
            );
        }
        if let Some(destination) = destination {
            panel = panel.child(
                div()
                    .id("restore-destination-path")
                    .child(destination.display().to_string())
                    .test_support(),
            );
        }
        if review {
            panel = panel.child("Storage integrity and current records checked in a private temporary copy. Restore creates the new folder shown above. Saved discussions, figures, history and unsaved recovery buffers are included. Installed skill files and original documents are not in the backup. The recovered copy requires a new inbox folder choice.")
                .child(button("restore-confirm").label("Restore this copy").primary().disabled(busy)
                    .on_click(cx.listener(|this, _, _, cx| this.confirm_restore(cx))))
                .child(button("restore-discard").label("Cancel restore").outline()
                    .on_click(cx.listener(|this, _, _, cx| this.discard_restore(cx))));
        }
        if matches!(self.restore.phase, RestorePhase::Validating) {
            panel = panel.child(
                button("restore-cancel-validation")
                    .label("Cancel validation")
                    .outline()
                    .on_click(cx.listener(|this, _, _, cx| this.cancel_restore(cx))),
            );
        }
        if matches!(self.restore.phase, RestorePhase::Complete(_)) {
            panel = panel.child(
                button("restore-open")
                    .label("Open recovered workspace")
                    .outline()
                    .disabled(busy)
                    .on_click(cx.listener(|this, _, _, cx| this.open_restored_workspace(cx))),
            );
        }
        panel
            .child(
                div()
                    .id("restore-status")
                    .child(self.restore.status.clone())
                    .test_support(),
            )
            .into_any_element()
    }

    fn choose_restore_source(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.worker.is_some() {
            return;
        }
        self.discard_restore(cx);
        let generation = self.restore.generation;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose a BRN full backup file".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r.map_err(|e| e.to_string()));
            let _ = this.update_in(cx, |this, _, cx| {
                this.restore_source_chosen(generation, result, cx)
            });
        })
        .detach();
    }
    fn restore_source_chosen(
        &mut self,
        generation: u64,
        result: Result<Option<Vec<PathBuf>>, String>,
        cx: &mut Context<Self>,
    ) {
        if generation != self.restore.generation || self.worker.is_some() {
            return;
        }
        match one_path(result) {
            Ok(Some(path)) => {
                match selected_source(&path) {
                    Ok(()) => {
                        self.restore.source = Some(path);
                        self.restore.status = "Choose a folder for a new recovery copy. Nothing has been restored yet.".into();
                    }
                    Err(error) => self.restore.status = error,
                }
            }
            Ok(None) => {}
            Err(error) => self.restore.status = format!("Could not choose backup: {error}"),
        }
        cx.notify();
    }
    fn choose_restore_parent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.worker.is_some() || self.restore.source.is_none() {
            return;
        }
        self.restore.generation += 1;
        let generation = self.restore.generation;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose a folder for a new BRN recovery copy".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r.map_err(|e| e.to_string()));
            let _ = this.update_in(cx, |this, _, cx| {
                this.restore_parent_chosen(generation, result, cx)
            });
        })
        .detach();
    }
    fn restore_parent_chosen(
        &mut self,
        generation: u64,
        result: Result<Option<Vec<PathBuf>>, String>,
        cx: &mut Context<Self>,
    ) {
        if generation != self.restore.generation || self.worker.is_some() {
            return;
        }
        let parent = match one_path(result) {
            Ok(Some(path)) => path,
            Ok(None) => return,
            Err(error) => {
                self.restore.status = format!("Could not choose recovery folder: {error}");
                cx.notify();
                return;
            }
        };
        let Some(source) = self.restore.source.clone() else {
            return;
        };
        let result = recovery_destination(&parent, &self.data_dir).and_then(|destination| {
            selected_source(&source)?;
            Ok(destination)
        });
        let destination = match result {
            Ok(path) => path,
            Err(error) => {
                self.restore.status = error;
                cx.notify();
                return;
            }
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        let cancel = CancellationToken::new();
        let cancelled = cancel.clone();
        std::thread::spawn(move || {
            let result = Store::prepare_restore(&source, destination).map_err(|e| e.to_string());
            // SQLite's bounded page-copy loop must finish before cleanup. Cancel
            // never publishes; dropping either this result or a closed receiver
            // removes the private stage.
            let result = if cancelled.is_cancelled() {
                drop(result);
                Err("Restore validation cancelled; no recovery copy published.".into())
            } else {
                result
            };
            let _ = sender.send(WorkerEvent::RestorePrepared(result));
        });
        self.restore.phase = RestorePhase::Validating;
        self.restore.status = "Validating backup in a private temporary copy…".into();
        self.progress = self.restore.status.clone();
        self.worker = Some(Worker::restore(receiver, cancel, false));
        cx.notify();
    }
    pub(super) fn restore_prepared(&mut self, result: Result<PreparedRestore, String>) {
        let cancelled = self
            .worker
            .as_ref()
            .is_none_or(|worker| worker.cancel.is_cancelled());
        self.worker = None;
        self.progress.clear();
        if cancelled {
            drop(result);
            self.restore.phase = RestorePhase::Idle;
            self.restore.status =
                "Restore validation cancelled; no recovery copy published.".into();
            return;
        }
        match result {
            Ok(prepared) => {
                self.restore.phase = RestorePhase::Review(prepared);
                self.restore.status =
                    "Ready for your confirmation. Nothing has been published.".into();
            }
            Err(error) => {
                self.restore.phase = RestorePhase::Idle;
                self.restore.status = format!("Backup could not be restored: {error}");
            }
        }
    }
    fn confirm_restore(&mut self, cx: &mut Context<Self>) {
        if self.worker.is_some() || !matches!(self.restore.phase, RestorePhase::Review(_)) {
            return;
        }
        let RestorePhase::Review(prepared) = std::mem::take(&mut self.restore.phase) else {
            return;
        };
        let destination = prepared.destination().to_owned();
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result = prepared.publish().map(|store| { drop(store); "Recovery copy created. Open it when ready; your current workspace is unchanged.".into() })
                .map_err(|error| format!("Restore did not finish: {error}. The destination may already contain a complete copy; preserve it and choose a new destination for retry."));
            let _ = sender.send(WorkerEvent::Finished(result));
        });
        self.restore.phase = RestorePhase::Publishing(destination);
        self.restore.status = "Publishing the validated recovery copy… Please wait.".into();
        self.progress = self.restore.status.clone();
        self.worker = Some(Worker::restore(receiver, CancellationToken::new(), true));
        cx.notify();
    }
    pub(super) fn restore_finished(&mut self, result: Result<String, String>) {
        self.worker = None;
        self.progress.clear();
        let phase = std::mem::take(&mut self.restore.phase);
        match (phase, result) {
            (RestorePhase::Publishing(path), Ok(status)) => { self.restore.phase = RestorePhase::Complete(path); self.restore.status = status; }
            (_, Err(error)) => self.restore.status = error,
            _ => self.restore.status = "Restore ended unexpectedly. Preserve the selected destination and inspect it before retrying.".into(),
        }
    }
    pub(super) fn cancel_restore(&mut self, cx: &mut Context<Self>) {
        if matches!(self.restore.phase, RestorePhase::Validating) {
            if let Some(worker) = &self.worker {
                worker.cancel.cancel();
            }
            self.restore.status = "Cancelling validation… Waiting for the private copy to close; nothing will be published.".into();
            self.progress = self.restore.status.clone();
        }
        cx.notify();
    }
    fn discard_restore(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.restore.phase,
            RestorePhase::Validating | RestorePhase::Publishing(_)
        ) {
            return;
        }
        self.restore.generation += 1;
        self.restore.source = None;
        self.restore.phase = RestorePhase::Idle;
        self.restore.status = "No recovery copy published by this selection.".into();
        cx.notify();
    }
    fn open_restored_workspace(&mut self, cx: &mut Context<Self>) {
        let RestorePhase::Complete(path) = &self.restore.phase else {
            return;
        };
        let path = path.clone();
        let workspace = match Workspace::open(&path) {
            Ok(workspace) => workspace,
            Err(error) => {
                self.restore.status =
                    format!("Recovery copy preserved, but could not open it: {error}");
                cx.notify();
                return;
            }
        };
        let result = cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(px(90.), px(90.)), size(px(1260.), px(860.))))),
            window_min_size: Some(size(px(850.), px(600.))), ..top_bar_window_options() }, move |window, cx| {
                window.set_window_title("BRN — Recovered workspace");
                let view = cx.new(|cx| { let mut view = ThreadsUi::new(workspace, path, window, cx); view.inform("Recovered workspace. Saved queues stay paused until Resume. Review unsaved recovery before continuing."); view });
                install_exit_handlers(window, cx, view.downgrade());
                cx.new(|cx| Root::new(view, window, cx))
            });
        if let Err(error) = result {
            self.restore.status =
                format!("Recovery copy preserved, but its window could not open: {error}");
        }
        cx.notify();
    }
}
fn one_path(result: Result<Option<Vec<PathBuf>>, String>) -> Result<Option<PathBuf>, String> {
    match result? {
        None => Ok(None),
        Some(mut paths) if paths.len() == 1 => Ok(paths.pop()),
        Some(_) => Err("Choose exactly one path.".into()),
    }
}
fn selected_source(source: &std::path::Path) -> Result<(), String> {
    if !source.is_absolute()
        || source
            .ancestors()
            .any(|p| std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()))
        || !std::fs::metadata(source).is_ok_and(|m| m.is_file())
    {
        return Err(
            "Choose an existing backup file using its real absolute path, without symbolic links."
                .into(),
        );
    }
    Ok(())
}
fn recovery_destination(
    parent: &std::path::Path,
    current: &std::path::Path,
) -> Result<PathBuf, String> {
    if !parent.is_absolute()
        || !parent.is_dir()
        || parent
            .ancestors()
            .any(|p| std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()))
    {
        return Err("Choose an existing recovery folder using its real absolute path, without symbolic links.".into());
    }
    let parent = parent.canonicalize().map_err(|e| e.to_string())?;
    let current = current.canonicalize().map_err(|e| e.to_string())?;
    if parent.starts_with(current) {
        return Err("Choose a recovery folder outside the current workspace.".into());
    }
    Ok(parent.join(format!("BRN-restored-{}", uuid::Uuid::new_v4())))
}

#[cfg(all(test, feature = "native-test-support"))]
#[path = "threads_restore_tests.rs"]
mod tests;
