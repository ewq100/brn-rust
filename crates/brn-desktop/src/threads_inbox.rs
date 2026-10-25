//! Manual document inbox controls. No watcher, project inheritance or source-file writes.
use super::*;

impl ThreadsUi {
    pub(super) fn inbox_settings(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let state = self.workspace.inbox();
        let mut panel=div().id("settings-inbox").flex().flex_col().gap_2()
            .child(section("Document inbox"))
            .child(div().id("inbox-destination").child(INBOX_DESTINATION_NOTICE).test_support())
            .child(metadata("Scan is manual. Only this folder's files are considered; subfolders and links are not followed. Statuses describe the latest explicit scan. Scan again to retry pending or failed items."));
        panel = panel.child(metadata(format!(
            "Next scan: {} · AI refinement {}",
            match self.settings.document_conversion.converter {
                DocumentConverter::Xberg => "Stock Xberg",
                DocumentConverter::XbergBrn => "BRN Xberg",
            },
            if self.settings.document_conversion.ai {
                "on"
            } else {
                "off"
            }
        )));
        panel=panel.child(metadata("Up to 256 entries; 16 MiB per file; 64 MiB of source files and converted content per scan."));
        let configured = matches!(&state, Ok(Some(_)));
        panel = panel.child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(
                    button("inbox-choose-folder")
                        .label("Choose General inbox folder…")
                        .outline()
                        .disabled(self.worker.is_some())
                        .on_click(
                            cx.listener(|this, _, window, cx| this.choose_inbox_folder(window, cx)),
                        ),
                )
                .child(
                    button("inbox-scan")
                        .label("Scan inbox")
                        .primary()
                        .disabled(self.worker.is_some() || !configured)
                        .on_click(cx.listener(|this, _, _, cx| this.start_inbox_scan(cx))),
                ),
        );
        match state {
            Ok(Some(state))=>{
                panel=panel.child(div().id("inbox-folder").child(state.folder).test_support());
                for (index,entry) in state.entries.into_iter().enumerate() {
                    let label=format!("{} — {:?}: {}",entry.name,entry.status,entry.detail);
                    panel=panel.child(div().id(format!("inbox-entry-{index}")).child(label).test_support());
                }
            },
            Ok(None)=>panel=panel.child(metadata("No inbox folder selected. Choosing a folder authorizes only explicit scans into General knowledge.")),
            Err(error)=>panel=panel.child(metadata(error.to_string())),
        }
        panel.into_any_element()
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
                    let _ = sender.try_send(WorkerEvent::Progress(
                        "Importing inbox into General knowledge…".into(),
                    ));
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
