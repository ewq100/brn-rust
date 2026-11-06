//! Explicit, ephemeral subscription research. No BRN content is attached.
use super::*;
use brn_ai::subscription_search::{SearchError, SearchResult};

#[derive(Default)]
pub(super) struct WebSearch {
    generation: u64,
    cancel: Option<CancellationToken>,
    receiver: Option<mpsc::Receiver<(u64, Result<SearchResult, SearchError>)>>,
    selection: Option<SearchSelection>,
    result: Option<Result<SearchResult, String>>,
}
impl Drop for WebSearch {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.cancel();
        }
    }
}
#[derive(PartialEq, Eq)]
struct SearchSelection {
    provider: String,
    model: String,
    effort: String,
    auth_file: Option<String>,
    credentials_dir: Option<String>,
}
fn selection(settings: &WorkspaceSettings) -> SearchSelection {
    SearchSelection {
        provider: settings.provider.clone(),
        model: settings.model.clone(),
        effort: settings.effort.clone(),
        auth_file: settings.auth_file.clone(),
        credentials_dir: settings.credentials_dir.clone(),
    }
}
impl ThreadsUi {
    pub(super) fn reconcile_web_settings(&mut self, settings: &WorkspaceSettings) {
        if self
            .web_search
            .selection
            .as_ref()
            .is_some_and(|old| *old != selection(settings))
        {
            self.stop_web_search();
            self.web_search.result = Some(Err(
                "Search stopped because the selected settings changed.".into(),
            ));
        }
    }
    fn stop_web_search(&mut self) {
        self.web_search.generation = self.web_search.generation.wrapping_add(1);
        if let Some(cancel) = self.web_search.cancel.take() {
            cancel.cancel();
        }
        self.web_search.receiver = None;
        self.web_search.selection = None;
        self.web_search.result = None;
    }
    fn start_web_search(&mut self, query: String, cx: &mut Context<Self>) {
        if self.web_search.cancel.is_some() {
            return;
        }
        self.stop_web_search();
        let settings = match self.workspace.settings() {
            Ok(settings) => settings,
            Err(error) => {
                self.web_search.result = Some(Err(error.to_string()));
                cx.notify();
                return;
            }
        };
        let request = match brn_threads_app::subscription_search::prepare(&settings, query) {
            Ok(request) => request,
            Err(error) => {
                self.web_search.result = Some(Err(error));
                cx.notify();
                return;
            }
        };
        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        let generation = self.web_search.generation;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| SearchError::Transport)
                .and_then(|runtime| {
                    runtime.block_on(brn_ai::subscription_search::search(request, worker_cancel))
                });
            let _ = sender.send((generation, result));
        });
        self.web_search.cancel = Some(cancel);
        self.web_search.receiver = Some(receiver);
        self.web_search.selection = Some(selection(&settings));
        cx.notify();
    }
    pub(super) fn poll_web_search(&mut self, cx: &mut Context<Self>) {
        if let Some(expected) = &self.web_search.selection
            && self
                .workspace
                .settings()
                .map(|s| selection(&s))
                .ok()
                .as_ref()
                != Some(expected)
        {
            self.stop_web_search();
            self.web_search.result = Some(Err(
                "Search stopped because the selected settings changed.".into(),
            ));
            cx.notify();
            return;
        }
        let received = self
            .web_search
            .receiver
            .as_ref()
            .and_then(|r| match r.try_recv() {
                Ok(message) => Some(message),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some((self.web_search.generation, Err(SearchError::Transport)))
                }
            });
        if let Some((generation, result)) = received
            && generation == self.web_search.generation
        {
            self.web_search.receiver = None;
            self.web_search.cancel = None;
            self.web_search.result = Some(result.map_err(|e| e.to_string()));
            cx.notify();
        }
    }
    pub(super) fn open_web_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_web_search();
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter a web search query"));
        let target = cx.entity().downgrade();
        let field = input.clone();
        self.open_dialog(window, cx, move |dialog, window, cx| {
            let Some(view) = target.upgrade() else { return dialog; };
            let state = &view.read(cx).web_search;
            let running = state.cancel.is_some();
            let result = state.result.clone();
            let close_label = if result.is_some() { "Close" } else { "Cancel" };
            let mut body = div().flex().flex_col().gap_2()
                .child("Only the query you enter is sent. Existing account instructions may apply. Search results are untrusted evidence.")
                .child(Input::new(&field).id("web-search-query"));
            if running { body = body.child("Searching…"); }
            if let Some(result) = result {
                match result {
                    Err(error) => body = body.child(components::field_error("web-search-error", error)),
                    Ok(result) => {
                        body = body.child(format!("Provider: {:?} · Model: {}", result.provenance.provider, result.provenance.model))
                            .child(result.answer);
                        for (index, source) in result.sources.into_iter().enumerate() {
                            let url = source.url;
                            let label = format!("{} — {}", source.title.unwrap_or_else(|| "Source".into()), url);
                            body = body.child(button(format!("web-search-source-{index}")).label(label)
                                .on_click(move |_, _, cx| cx.open_url(&url)));
                        }
                    }
                }
            }
            let close_target = target.clone();
            let cancel_target = target.clone();
            let submit_target = target.clone();
            let submit_field = field.clone();
            let submit = std::rc::Rc::new(move |_: &mut Window, cx: &mut App| {
                let query = submit_field.read(cx).value().to_string();
                let _ = submit_target.update(cx, |view, cx| view.start_web_search(query, cx));
            });
            let click = submit.clone();
            let stop_target = target.clone();
            let keyboard_close = target.clone();
            dialog.title("Search the web")
                .on_ok(|_, _, _| false)
                .on_cancel(move |_, _, cx| { let _ = cancel_target.update(cx, |view, cx| { view.stop_web_search(); cx.notify(); }); true })
                .on_close(move |_, _, cx| { let _ = close_target.update(cx, |view, cx| { view.stop_web_search(); cx.notify(); }); })
                .child(components::dialog_body("web-search-body", window).child(body))
                .footer(components::dialog_footer()
                    .child(div().id("web-search-close-keyboard")
                    .on_action(move |_: &gpui_kit::component::dialog::Confirm, window, cx| {
                        let _ = keyboard_close.update(cx, |view, cx| { view.stop_web_search(); cx.notify(); });
                        window.close_dialog(cx);
                    })
                    .child(button("web-search-cancel").label(close_label).outline().on_click(move |_, window, cx| {
                        let _ = stop_target.update(cx, |view, cx| { view.stop_web_search(); cx.notify(); });
                        window.close_dialog(cx);
                    })))
                    .child(div().id("web-search-submit-keyboard")
                        .on_action(move |_: &gpui_kit::component::dialog::Confirm, window, cx| { if !running { submit(window, cx); } })
                        .child(button("web-search-submit").label("Search").primary().disabled(running)
                            .on_click(move |_, window, cx| click(window, cx))))
                    .when(running, |footer| {
                        let target = target.clone();
                        let keyboard = target.clone();
                        footer.child(div().id("web-search-stop-keyboard")
                        .on_action(move |_: &gpui_kit::component::dialog::Confirm, _, cx| {
                            let _ = keyboard.update(cx, |view, cx| { view.stop_web_search(); view.web_search.result = Some(Err("Search stopped.".into())); cx.notify(); });
                        })
                        .child(button("web-search-stop").label("Stop search").outline().on_click(move |_, _, cx| {
                            let _ = target.update(cx, |view, cx| { view.stop_web_search(); view.web_search.result = Some(Err("Search stopped.".into())); cx.notify(); });
                        })))
                    }))
        });
        input.update(cx, |input, cx| input.focus(window, cx));
    }
}

#[cfg(all(test, feature = "native-test-support"))]
mod tests {
    use super::*;
    use gpui_kit::{TestAppContext, test::TestWindowExt};

    /// Search the web… is in the composer's + menu.
    fn open_from_plus_menu(cx: &mut TestAppContext, window: gpui_kit::AnyWindowHandle) {
        cx.update_window(window, |_, window, cx| window.click("composer-add", cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            let mut menu = window.within("popup-menu");
            let index = (0..60usize)
                .find(|index| {
                    menu.try_find(*index)
                        .is_some_and(|row| row.label() == Some("Search the web…"))
                })
                .expect("the + menu offers Search the web…");
            menu.click(index, cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    #[gpui_kit::test]
    fn opening_and_dismissing_search_never_writes_or_dispatches(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.bind_keys(bindings());
            cx.set_reduce_motion(true);
        });
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        let mut workspace = Workspace::open_without_default_credentials(&data).unwrap();
        focus_tests::configure_offline_fixture(&mut workspace);
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data, window, cx);
                    ui.poll = None;
                    ui
                })
            })
            .unwrap()
        });
        cx.run_until_parked();
        let before = cx.read(|cx| view.read(cx).workspace.records().unwrap());
        for dismissal in ["escape", "click"] {
            open_from_plus_menu(cx, window);
            cx.update_window(window, |_, window, cx| {
                assert!(window.find("web-search-submit").visible());
                window.click("web-search-submit", cx);
                if dismissal == "escape" {
                    window.press("escape", cx);
                } else {
                    window.click("web-search-cancel", cx);
                }
            })
            .unwrap();
            cx.run_until_parked();
            cx.read(|cx| {
                let ui = view.read(cx);
                assert_eq!(ui.workspace.records().unwrap(), before);
                assert!(ui.worker.is_none());
                assert!(ui.web_search.cancel.is_none());
            });
        }
    }

    #[gpui_kit::test]
    fn settings_change_cancels_and_discards_late_search(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.bind_keys(bindings());
            cx.set_reduce_motion(true);
        });
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        let mut workspace = Workspace::open_without_default_credentials(&data).unwrap();
        focus_tests::configure_offline_fixture(&mut workspace);
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data, window, cx);
                    ui.poll = None;
                    ui
                })
            })
            .unwrap()
        });
        let token = CancellationToken::new();
        let check = token.clone();
        cx.update_window(window, |_, _, cx| {
            view.update(cx, |ui, cx| {
                let settings = ui.workspace.settings().unwrap();
                let mut old = selection(&settings);
                old.credentials_dir = Some("/synthetic/changed-account-route".into());
                ui.web_search.selection = Some(old);
                ui.web_search.cancel = Some(token);
                let (tx, rx) = mpsc::channel();
                tx.send((ui.web_search.generation, Err(SearchError::Transport)))
                    .unwrap();
                ui.web_search.receiver = Some(rx);
                ui.poll_web_search(cx);
                assert!(check.is_cancelled());
                assert!(ui.web_search.receiver.is_none());
                assert!(
                    ui.web_search
                        .result
                        .as_ref()
                        .unwrap()
                        .as_ref()
                        .unwrap_err()
                        .contains("settings changed")
                );
                assert!(ui.worker.is_none());
            })
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn cited_result_is_visible_and_dismissal_leaves_records_unchanged(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.bind_keys(bindings());
            cx.set_reduce_motion(true);
        });
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        let mut workspace = Workspace::open_without_default_credentials(&data).unwrap();
        focus_tests::configure_offline_fixture(&mut workspace);
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data, window, cx);
                    ui.poll = None;
                    ui
                })
            })
            .unwrap()
        });
        cx.run_until_parked();
        let before = cx.read(|cx| view.read(cx).workspace.records().unwrap());
        open_from_plus_menu(cx, window);
        cx.update_window(window, |_, _, cx| {
            view.update(cx, |ui, cx| {
                ui.web_search.result = Some(Ok(SearchResult {
                    answer: "Synthetic cited answer".into(),
                    sources: vec![brn_ai::subscription_search::Source {
                        url: "https://example.org/evidence".into(),
                        title: Some("Synthetic evidence".into()),
                    }],
                    provenance: brn_ai::subscription_search::Provenance {
                        provider: brn_ai::Provider::Chatgpt,
                        model: "synthetic-model".into(),
                    },
                }));
                cx.notify();
            });
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            assert!(window.find("web-search-source-0").visible());
            window.click("web-search-cancel", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.read(|cx| {
            let ui = view.read(cx);
            assert_eq!(ui.workspace.records().unwrap(), before);
            assert!(ui.worker.is_none());
            assert!(ui.web_search.result.is_none());
        });
    }
}
