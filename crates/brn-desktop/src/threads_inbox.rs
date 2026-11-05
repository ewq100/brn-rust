//! Manual document inbox controls. No watcher, project inheritance or source-file writes.
use super::components::Tone;
use super::*;

impl ThreadsUi {
    /// Inbox → Imports: the manual folder inbox. Setup and Scan are separate
    /// owner choices; there is no watcher and no scan at launch.
    pub(super) fn inbox_settings(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        use super::inbox_page::{group_heading, item_line, item_row, page_group};
        let p = pal();
        let state = self.workspace.inbox();
        let scanning = self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.kind() == WorkerKind::Inbox);
        let busy = self.worker.is_some();
        let configured = matches!(&state, Ok(Some(_)));
        let quiet = |text: String| {
            div()
                .text_size(px(theme::metrics::TEXT_LABEL))
                .text_color(rgb(p.muted))
                .child(text)
        };
        let mut panel = page_group("settings-inbox")
            .child(group_heading("Folder inbox", None))
            .child(
                div()
                    .id("inbox-destination")
                    .text_size(px(theme::metrics::TEXT_BODY))
                    .text_color(rgb(p.ink))
                    .child(INBOX_DESTINATION_NOTICE)
                    .test_support(),
            )
            .child(quiet("Scan is manual: nothing is read until you choose Scan inbox, and BRN never watches the folder. Only this folder's files are considered; subfolders and links are not followed. Up to 256 entries, 16 MiB per file and 64 MiB per scan.".into()))
            .child(quiet(format!(
                "Next scan: {} · AI conversion {}",
                match self.settings.document_conversion.converter {
                    DocumentConverter::Xberg => "Xberg",
                    DocumentConverter::XbergBrn => "Xberg + BRN layer",
                },
                if self.settings.document_conversion.ai {
                    "on"
                } else {
                    "off"
                }
            )));
        let mut actions = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .py(components::tokens::SPACE_2)
            .child(
                button("inbox-scan")
                    .label("Scan inbox")
                    .icon(IconName::Inbox)
                    .primary()
                    .disabled(busy || !configured)
                    .when(!configured, |button| {
                        let (announced, tooltip) =
                            components::unavailable("Scan inbox", Some("choose a folder first"));
                        button.accessibility_label(announced).tooltip(tooltip)
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.start_inbox_scan(cx))),
            )
            .child(
                button("inbox-choose-folder")
                    .label(if configured {
                        "Change folder…"
                    } else {
                        "Choose General inbox folder…"
                    })
                    .outline()
                    .disabled(busy)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.choose_inbox_folder(window, cx)),
                    ),
            );
        if scanning {
            actions = actions.child(
                button("inbox-stop")
                    .label("Stop scan")
                    .outline()
                    .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
            );
        }
        panel = panel.child(actions);
        if scanning {
            panel = panel.child(components::loading("inbox-progress", self.progress.clone()));
        }
        match state {
            Ok(Some(state)) => {
                panel = panel.child(
                    div()
                        .id("inbox-folder")
                        .font_family(MONO)
                        .text_size(px(theme::metrics::TEXT_LABEL))
                        .text_color(rgb(p.ink))
                        .child(state.folder.clone())
                        .test_support(),
                );
                let count = |status: InboxStatus| {
                    state
                        .entries
                        .iter()
                        .filter(|entry| entry.status == status)
                        .count()
                };
                let (imported, failed, waiting) = (
                    count(InboxStatus::Imported),
                    count(InboxStatus::Failed),
                    count(InboxStatus::Pending),
                );
                if state.entries.is_empty() {
                    panel = panel.child(
                        quiet("No scan yet for this folder. Choose Scan inbox when the files are ready.".into())
                            .id("inbox-entries-empty")
                            .test_support(),
                    );
                } else {
                    let summary = format!(
                        "Latest scan: {imported} imported · {failed} failed · {waiting} not processed"
                    );
                    panel = panel.child(
                        div()
                            .id("inbox-summary")
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_2()
                            .pt(components::tokens::SPACE_2)
                            .aria_label(summary.clone())
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(200.))
                                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                                    .text_color(rgb(p.ink))
                                    .child(summary),
                            )
                            .when(failed + waiting > 0, |line| {
                                line.child(
                                    button("inbox-retry")
                                        .label("Retry by scanning again")
                                        .icon(IconName::RefreshCw)
                                        .outline()
                                        .compact()
                                        .h(components::tokens::COMPACT_CONTROL)
                                        .disabled(busy)
                                        .tooltip("Scanning again retries failed and unprocessed files. Files already imported are recognised and not duplicated.")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.start_inbox_scan(cx)
                                        })),
                                )
                            })
                            .test_support(),
                    );
                }
                for (index, entry) in state.entries.into_iter().enumerate() {
                    let (tone, icon, word) = match entry.status {
                        InboxStatus::Imported => {
                            (Some(Tone::Success), IconName::CircleCheck, "Imported")
                        }
                        InboxStatus::Failed => (Some(Tone::Error), IconName::CircleX, "Failed"),
                        InboxStatus::Pending => (None, IconName::Clock, "Not processed"),
                    };
                    let row = match entry.note.clone() {
                        Some(note) => item_row(
                            format!("inbox-entry-{index}"),
                            icon,
                            tone,
                            entry.name.clone(),
                            entry.detail.clone(),
                            word,
                        )
                        .tooltip(format!("{} · {} · Open the note", entry.name, entry.detail))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_note(&note, None, window, cx)
                        }))
                        .into_any_element(),
                        None => item_line(
                            format!("inbox-entry-{index}"),
                            icon,
                            tone,
                            entry.name.clone(),
                            entry.detail.clone(),
                            word,
                        ),
                    };
                    panel =
                        panel.child(div().border_b_1().border_color(rgb(p.hairline)).child(row));
                }
            }
            Ok(None) => {
                panel = panel.child(quiet(
                    "No inbox folder chosen. Choosing one authorizes only the explicit scans you start, into General knowledge.".into(),
                ));
            }
            Err(error) => {
                panel = panel.child(components::banner(
                    "inbox-error",
                    Tone::Error,
                    format!("The folder inbox couldn't be read: {error}. Nothing was imported."),
                    vec![],
                    None,
                ));
            }
        }
        panel.test_support().into_any_element()
    }
    pub(super) fn choose_inbox_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(worker) = &self.worker {
            self.warn(worker.kind().busy_notice());
            cx.notify();
            return;
        }
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose inbox for General knowledge (available across projects)".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r.map_err(|e| e.to_string()));
            let _ = this.update_in(cx, |this, window, cx| {
                this.inbox_folder_chosen(result, window, cx)
            });
        })
        .detach();
    }
    pub(super) fn inbox_folder_chosen(
        &mut self,
        result: Result<Option<Vec<PathBuf>>, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(worker) = &self.worker {
            self.warn(worker.kind().busy_notice());
            cx.notify();
            return;
        }
        match result {
            Ok(Some(paths)) => {
                if let Some(path) = paths.first() {
                    self.confirm_inbox_destination(path.clone(), window, cx);
                }
            }
            Ok(None) => {}
            Err(error) => self.fail(format!("Could not choose inbox folder: {error}")),
        }
        cx.notify();
    }
    fn confirm_inbox_destination(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = cx.entity().downgrade();
        self.open_dialog(window,cx,move|dialog,window,_cx|{
            let target=target.clone();let chosen=path.clone();
            dialog.title("Use this folder for General knowledge?")
                .on_ok(|_,_,_|false)
                .child(components::dialog_body("inbox-consent-body",window)
                    .child(div().flex().flex_col().gap_2().child(INBOX_DESTINATION_NOTICE).child(path.display().to_string()).child("Nothing is imported until you separately choose Scan inbox.")))
                .footer(components::dialog_footer()
                    .child(button("inbox-consent-cancel").label("Cancel").outline().on_click(|_,window,cx|window.close_dialog(cx)))
                    .child(button("inbox-consent-confirm").label("Use General knowledge inbox").primary().on_click(move|_,window,cx|{
                        let mut close=false;
                        let _=target.update(cx,|this,cx|{
                            if let Some(worker)=&this.worker {this.warn(worker.kind().busy_notice());cx.notify();return;}
                            match this.workspace.choose_inbox(&chosen,InboxDestination::General) {
                                Ok(_)=>{this.inform("Inbox folder selected for General knowledge. Choose Scan inbox to import.");close=true;},
                                Err(error)=>this.fail(error.to_string()),
                            }
                            cx.notify();
                        });
                        if close {window.close_dialog(cx);}
                    })))
        });
    }
    pub(super) fn start_inbox_scan(&mut self, cx: &mut Context<Self>) {
        if let Some(worker) = &self.worker {
            self.warn(worker.kind().busy_notice());
            cx.notify();
            return;
        }
        if !matches!(self.workspace.inbox(), Ok(Some(_))) {
            self.warn("Choose an inbox folder for General knowledge first.");
            cx.notify();
            return;
        }
        let data = self.data_dir.clone();
        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        let flag = Arc::new(AtomicBool::new(false));
        let worker_flag = flag.clone();
        let (sender, receiver) = mpsc::sync_channel(8);
        std::thread::spawn(move || {
            let result = (|| -> AppResult<String> {
                let mut workspace = Workspace::open_existing_no_upgrade(data)?;
                let canonical = workspace.settings()?;
                let auth_file = canonical.auth_file.map(PathBuf::from);
                let credentials_dir = canonical.credentials_dir.map(PathBuf::from);
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                let summary = workspace.scan_inbox(&worker_flag, |document, settings| {
                    // Per-item progress: the file being converted now.
                    let name = std::path::Path::new(&document.source_reference)
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let _ = sender.try_send(WorkerEvent::Progress(format!(
                        "Importing {name} into General knowledge…"
                    )));
                    let helpers = DocumentHelpers::discover()
                        .map_err(|e| AppError::Unavailable(e.to_string()))?;
                    let converter = Converter {
                        temp_root: std::env::temp_dir().join("brn-threads-intake"),
                        limits: brn_threads_intake::import_limits(document.kind),
                    };
                    runtime
                        .block_on(brn_ai::document_refine::convert_document_job(
                            brn_ai::document_refine::DocumentConversionJob {
                                converter,
                                helpers,
                                settings: settings.clone(),
                                kind: document.kind,
                                intent: ImportIntent::FullNote,
                                bytes: document.bytes,
                                source_reference: document.source_reference,
                                auth_file: auth_file.clone(),
                                credentials_dir: credentials_dir.clone(),
                            },
                            worker_cancel.clone(),
                            worker_flag.clone(),
                            Arc::new(|_| {}),
                        ))
                        .map_err(|e| AppError::Unavailable(e.to_string()))
                })?;
                Ok(summary.to_string())
            })()
            .map_err(|e| e.to_string());
            let _ = sender.send(WorkerEvent::Finished(result));
        });
        self.worker = Some(Worker::inbox(receiver, cancel, flag));
        self.progress = "Scanning inbox into General knowledge…".into();
        cx.notify();
    }
}
