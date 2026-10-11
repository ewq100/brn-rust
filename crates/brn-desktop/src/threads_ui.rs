//! Final-use native Threads workspace. All durable changes use the shared service.
use brn_threads_app::*;
use brn_threads_intake::{
    ConversionRequest, ConversionResult, Converter, DocumentKind, ImportIntent, PdfTools,
};
use gpui_kit::base::{Disableable, RangeHighlight, Selectable};
use gpui_kit::component::WindowExt;
use gpui_kit::{
    App, AppContext, Bounds, Context, Entity, Image, ImageFormat, KeyBinding, PathPromptOptions,
    Subscription, Task, TestSupportExt, Window, WindowBounds, WindowOptions,
    base::{
        MarkdownExtensions, MarkdownNode, MarkdownParseContext, MarkdownPlugin, TextView,
        TextViewState, markdown_ast::Node,
    },
    component::{
        Root, TitleBar,
        button::Button,
        input::{Editor, EditorState, Input, InputEvent, InputState},
    },
    div, point,
    prelude::*,
    px, rgb, size,
};
use std::result::Result;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::AtomicBool,
        mpsc::{self, Receiver},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
#[path = "threads_state.rs"]
mod state;
use state::{
    ComposerDestination, ComposerDrafts, WorkerKind, can_continue_thread, comment_location,
    continuation_notice, delegation_status, latest_run, linked, outcome_message, passage_request,
    save_message, sections,
};

#[cfg(all(test, feature = "native-test-support"))]
#[path = "threads_focus_tests.rs"]
mod focus_tests;

#[cfg(all(test, feature = "native-test-support"))]
#[path = "threads_backup_tests.rs"]
mod backup_tests;

#[cfg(all(test, feature = "native-test-support"))]
#[path = "threads_steering_tests.rs"]
mod steering_tests;

#[cfg(all(test, feature = "native-test-support"))]
mod search_tests {
    use super::*;
    use gpui_kit::{TestAppContext, test::TestWindowExt};

    #[gpui_kit::test]
    fn clearing_native_search_restores_workspace_without_retrieval_error(cx: &mut TestAppContext) {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = root.path().join("search-clear-regression");
        let mut workspace = Workspace::open(&data).unwrap();
        focus_tests::configure_offline_fixture(&mut workspace);
        let note = workspace
            .new_note("Search witness", "harborneedle õun and saved text")
            .unwrap();
        cx.update(gpui_kit::init);
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| {
                    let mut view = ThreadsUi::new(workspace, data, window, cx);
                    view.poll = None;
                    view
                })
            })
            .unwrap()
        });
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            view.read(cx)
                .search
                .clone()
                .update(cx, |input, cx| input.focus(window, cx));
            window.input("harborneedle", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.read(|cx| {
            assert!(view.read(cx).hits.iter().any(|hit| hit.note == note.id));
            assert!(view.read(cx).notice.is_empty());
        });
        cx.update_window(window, |_, window, cx| {
            window.press("cmd-a", cx);
            window.press("backspace", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            let current = view.read(cx);
            assert!(current.search.read(cx).value().is_empty());
            assert!(current.hits.is_empty());
            assert!(
                current.notice.is_empty(),
                "clearing a valid search is normal navigation"
            );
            assert!(window.try_find(format!("note-{}", note.id)).is_some());
            window.click(format!("note-{}", note.id), cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            assert!(view.read(cx).page == Page::Note(note.id.clone()));
            view.read(cx)
                .search
                .clone()
                .update(cx, |input, cx| input.focus(window, cx));
            window.input("   ", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            assert!(view.read(cx).hits.is_empty());
            assert!(view.read(cx).notice.is_empty());
            assert!(
                window.try_find(format!("note-{}", note.id)).is_some(),
                "whitespace-only search also shows normal workspace"
            );
        })
        .unwrap();
    }
}

gpui_kit::actions!(
    brn_threads,
    [Save, Ask, Home, NewNote, FocusAsk, Cancel, Quit]
);

#[derive(Clone, Debug, PartialEq)]
enum Page {
    Home,
    Thread(String),
    Note(String),
    History,
    Settings,
    Actions,
}
impl Page {
    fn composer_destination(&self) -> ComposerDestination {
        match self {
            Self::Thread(id) => ComposerDestination::Thread(id.clone()),
            _ => ComposerDestination::Home,
        }
    }
}
struct OpenNote {
    record: Record,
    session: Option<EditSession>,
    editing: bool,
}
struct Review {
    candidate: Prepared,
    note: Option<String>,
    original: String,
    edited: bool,
    groups: ReviewGroups,
    selected: BTreeSet<String>,
}
enum WorkerEvent {
    AnswerFinalized(String),
    Progress(String),
    Text(String),
    Finished(Result<String, String>),
    Converted {
        result: Box<Result<ConversionResult, String>>,
        title: String,
    },
}
struct Worker {
    receiver: Receiver<WorkerEvent>,
    cancel: CancellationToken,
    conversion_cancel: Arc<AtomicBool>,
    kind: WorkerKind,
    /// The native stream and cancellation belong to this invocation's thread,
    /// independently of whichever page the owner is currently viewing.
    thread: Option<String>,
    /// Created synchronously before the provider worker can start.
    run_id: Option<String>,
}

struct PendingSend {
    thread: String,
    prompt: String,
}

// Inline HTML is shown literally. Managed figures use only stored image bytes;
// source URLs never authorize network or filesystem reads during rendering.
#[derive(Clone)]
struct LiteralHtml {
    block: bool,
}
impl MarkdownPlugin for LiteralHtml {
    fn name(&self) -> &str {
        "brn-literal-html"
    }
    fn is_block(&self) -> bool {
        self.block
    }
    fn parse(&self, node: &Node, _: &MarkdownParseContext<'_>) -> Option<MarkdownNode> {
        if let Node::Html(html) = node {
            Some(MarkdownNode::new(self.name(), ()).text(html.value.clone()))
        } else {
            None
        }
    }
    fn render(&self, node: &MarkdownNode, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child(node.as_text().to_owned())
    }
}
fn markdown(view: &Entity<TextViewState>, assets: Arc<BTreeMap<String, Arc<Image>>>) -> TextView {
    TextView::new(view)
        .w_full()
        .h_full()
        .scrollable(true)
        .markdown_extensions(
            MarkdownExtensions::default()
                .plugin(LiteralHtml { block: true })
                .plugin(LiteralHtml { block: false })
                .parser_revision(1),
        )
        .image_source(move |uri| {
            assets
                .get(uri.as_ref())
                .cloned()
                .unwrap_or_else(|| Arc::new(Image::from_bytes(ImageFormat::Png, Vec::new())))
                .into()
        })
        .on_link_click(|url, _, _, cx| {
            if url.starts_with("https://") || url.starts_with("http://") {
                cx.open_url(url);
            }
            cx.stop_propagation();
        })
}
fn panel() -> gpui_kit::Div {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .rounded_md()
        .bg(rgb(0xf3f5f7))
}
fn heading(text: impl Into<gpui_kit::SharedString>) -> gpui_kit::Div {
    div()
        .text_lg()
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .child(text.into())
}
fn title(record: &Record) -> String {
    match &record.data {
        RecordData::Note(n) => n.title.clone(),
        RecordData::Thread(t) => t.title.clone(),
        RecordData::Action(a) => a.description.clone(),
        RecordData::Source(s) => s.label.clone(),
        RecordData::Comment(c) => c.body.clone(),
        RecordData::Message(m) => m.text.clone(),
        RecordData::Run(r) => r.progress.clone(),
        RecordData::Link(l) => l.relation.clone(),
        RecordData::Asset(_) => "Figure".into(),
        RecordData::Settings(_) => "Workspace settings".into(),
    }
}
fn short(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let prefix: String = text.chars().take(70).collect();
    if text.chars().count() > 70 {
        format!("{prefix}…")
    } else {
        prefix
    }
}

struct ThreadsUi {
    workspace: Workspace,
    data_dir: PathBuf,
    page: Page,
    records: Vec<Record>,
    history: Vec<Receipt>,
    candidates: Vec<Prepared>,
    buffers: Vec<EditSession>,
    composer: Entity<EditorState>,
    composer_drafts: ComposerDrafts,
    note_editor: Entity<EditorState>,
    note_view: Entity<TextViewState>,
    current_view: Entity<TextViewState>,
    review_editor: Entity<EditorState>,
    search: Entity<InputState>,
    hits: Vec<brn_threads_app::SearchHit>,
    model: Entity<InputState>,
    auth_path: Entity<InputState>,
    credentials_path: Entity<InputState>,
    settings: WorkspaceSettings,
    note: Option<OpenNote>,
    review: Option<Review>,
    comment_input: Entity<EditorState>,
    comment_anchor: Option<(String, u64, std::ops::Range<usize>, String)>,
    assets: Arc<BTreeMap<String, Arc<Image>>>,
    worker: Option<Worker>,
    pending_send: Option<PendingSend>,
    stream: String,
    progress: String,
    notice: String,
    export_status: String,
    backup_status: String,
    importing: Option<ImportIntent>,
    subscriptions: Vec<Subscription>,
    poll: Option<Task<()>>,
}
impl ThreadsUi {
    fn new(
        workspace: Workspace,
        data_dir: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let settings = workspace
            .settings()
            .expect("workspace settings were checked before opening the window");
        let composer = cx
            .new(|cx| EditorState::new(window, cx).placeholder("Ask a question or delegate work…"));
        let note_editor = cx.new(|cx| EditorState::new(window, cx).default_value(""));
        let review_editor = cx.new(|cx| EditorState::new(window, cx).default_value(""));
        let comment_input =
            cx.new(|cx| EditorState::new(window, cx).placeholder("Discuss this passage…"));
        let note_view = cx.new(|cx| {
            TextViewState::markdown("", cx)
                .selectable(true)
                .scrollable(true)
        });
        let current_view = cx.new(|cx| {
            TextViewState::markdown("", cx)
                .selectable(true)
                .scrollable(true)
        });
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search current notes"));
        let model = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(settings.model.clone())
                .placeholder("Explicit model")
        });
        let auth_path = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(settings.auth_file.clone().unwrap_or_default())
                .placeholder("Authentication file (optional)")
        });
        let credentials_path = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(settings.credentials_dir.clone().unwrap_or_default())
                .placeholder("Credentials folder (optional)")
        });
        let subscriptions = vec![
            cx.subscribe_in(
                &note_editor,
                window,
                |this, editor, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) && this.note_editor_visible() {
                        this.note_changed(editor.read(cx).value().to_string());
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(
                &review_editor,
                window,
                |this, editor, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change)
                        && let Some(review) = &mut this.review
                    {
                        review.edited = editor.read(cx).value().as_ref() != review.original;
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(&search, window, |this, input, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let query = input.read(cx).value().to_string();
                    this.hits.clear();
                    this.notice.clear();
                    if !query.trim().is_empty() {
                        match this.workspace.search(&query) {
                            Ok(hits) => this.hits = hits,
                            Err(e) => this.notice = e.to_string(),
                        };
                    }
                    cx.notify();
                }
            }),
        ];
        let poll = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(80))
                    .await;
                if this
                    .update_in(cx, |this, window, cx| this.poll_worker(window, cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        let mut this = Self {
            workspace,
            data_dir,
            page: Page::Home,
            records: vec![],
            history: vec![],
            candidates: vec![],
            buffers: vec![],
            composer,
            composer_drafts: ComposerDrafts::default(),
            note_editor,
            note_view,
            current_view,
            review_editor,
            search,
            hits: vec![],
            model,
            auth_path,
            credentials_path,
            settings,
            note: None,
            review: None,
            comment_input,
            comment_anchor: None,
            assets: Arc::new(BTreeMap::new()),
            worker: None,
            pending_send: None,
            stream: String::new(),
            progress: String::new(),
            notice: String::new(),
            export_status: "No export created in this session.".into(),
            backup_status: "No backup created in this session.".into(),
            importing: None,
            subscriptions,
            poll: Some(poll),
        };
        this.refresh();
        this
    }
    fn refresh(&mut self) {
        let result = (|| -> AppResult<()> {
            self.records = self.workspace.records()?;
            self.history = self.workspace.history()?;
            self.candidates = self.workspace.candidates()?;
            self.buffers = self.workspace.store.recovery_buffers()?;
            self.settings = self.workspace.settings()?;
            Ok(())
        })();
        if let Err(error) = result {
            self.notice = error.to_string();
        }
        self.assets = Arc::new(
            self.records
                .iter()
                .filter_map(|record| {
                    let RecordData::Asset(asset) = &record.data else {
                        return None;
                    };
                    let format = match asset.media_type.as_str() {
                        "image/png" => ImageFormat::Png,
                        "image/jpeg" => ImageFormat::Jpeg,
                        _ => return None,
                    };
                    Some((
                        format!("brn-asset://{}", record.id),
                        Arc::new(Image::from_bytes(format, asset.bytes.clone())),
                    ))
                })
                .collect(),
        );
    }
    fn note_editor_visible(&self) -> bool {
        self.review.is_none()
            && self
                .note
                .as_ref()
                .is_some_and(|open| open.editing && self.page == Page::Note(open.record.id.clone()))
    }
    fn preserve_visible_note_input(&mut self, cx: &Context<Self>) {
        if self.note_editor_visible() {
            self.note_changed(self.note_editor.read(cx).value().to_string());
        }
    }
    fn focus_visible_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.blur(cx);
        if self
            .review
            .as_ref()
            .is_some_and(|review| review.note.is_some())
        {
            self.review_editor
                .update(cx, |editor, cx| editor.focus(window, cx));
        } else if self.note_editor_visible() {
            self.note_editor
                .update(cx, |editor, cx| editor.focus(window, cx));
        }
    }
    fn toggle_note_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.preserve_visible_note_input(cx);
        if let Some(open) = &mut self.note {
            open.editing = !open.editing;
        }
        let text = self.note_editor.read(cx).value().to_string();
        self.note_view
            .update(cx, |view, cx| view.set_text(&text, cx));
        self.focus_visible_editor(window, cx);
        cx.notify();
    }
    fn navigate(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        self.preserve_visible_note_input(cx);
        window.blur(cx);
        self.change_page(page, window, cx);
        self.comment_anchor = None;
        self.refresh();
        cx.notify();
    }
    fn change_page(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        let old = self.page.composer_destination();
        let next = page.composer_destination();
        self.composer_drafts
            .preserve(old.clone(), self.composer.read(cx).value().to_string());
        if old != next {
            let text = self.composer_drafts.text(&next).to_owned();
            self.composer
                .update(cx, |editor, cx| editor.set_value(text, window, cx));
        }
        self.page = page;
    }
    fn composer_destination_label(&self) -> String {
        match &self.page {
            Page::Thread(id) => {
                let name = self
                    .records
                    .iter()
                    .find(|record| record.id == *id)
                    .map(title)
                    .unwrap_or_else(|| id.clone());
                format!("Send to thread: {}", name)
            }
            Page::Note(id) => {
                let name = self
                    .records
                    .iter()
                    .find(|record| record.id == *id)
                    .map(title)
                    .unwrap_or_else(|| id.clone());
                format!("Start a new thread about: {}", name)
            }
            _ => "Start a new thread · Home draft".into(),
        }
    }
    fn open_note(
        &mut self,
        id: &str,
        recovery: Option<EditSession>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.preserve_visible_note_input(cx);
        window.blur(cx);
        match self.workspace.store.note(id) {
            Ok(Some(record)) => {
                let RecordData::Note(note) = &record.data else {
                    return;
                };
                let text = recovery
                    .as_ref()
                    .map(|s| s.markdown.clone())
                    .unwrap_or_else(|| note.markdown.clone());
                self.note = Some(OpenNote {
                    record: record.clone(),
                    editing: recovery.is_some(),
                    session: recovery,
                });
                self.note_editor
                    .update(cx, |editor, cx| editor.set_value(text.clone(), window, cx));
                self.note_view
                    .update(cx, |view, cx| view.set_text(&text, cx));
                self.current_view
                    .update(cx, |view, cx| view.set_text(&note.markdown, cx));
                self.change_page(Page::Note(id.into()), window, cx);
                self.comment_anchor = None;
                self.focus_visible_editor(window, cx);
                self.refresh();
                cx.notify();
            }
            Ok(None) => self.notice = "That note is no longer current.".into(),
            Err(e) => self.notice = e.to_string(),
        }
    }
    fn note_changed(&mut self, text: String) {
        let Some(open) = &mut self.note else { return };
        let RecordData::Note(note) = &open.record.data else {
            return;
        };
        if open.session.is_none() && text == note.markdown {
            return;
        }
        let result = (|| -> brn_threads_app::AppResult<()> {
            if open.session.is_none() {
                open.session = Some(
                    self.workspace
                        .store
                        .begin_edit(&open.record.id, open.record.version)?,
                );
            }
            let session = open.session.as_ref().unwrap();
            if session.markdown == text {
                return Ok(());
            }
            match self.workspace.store.update_buffer(&session.id,session.generation+1,&text)? {
                BufferOutcome::Updated(updated)=>{open.session=Some(updated);self.notice="Your edit is recoverable. Save to update the note.".into();},
                BufferOutcome::Ignored(_)=>self.notice="This recovery buffer changed elsewhere. Your visible text is retained; compare before saving.".into(),
            }
            Ok(())
        })();
        if let Err(e) = result {
            self.notice = format!(
                "Could not preserve this edit: {e}. Keep this window open or copy your text."
            );
        }
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.note_editor.read(cx).value().to_string();
        self.note_changed(text);
        let Some(session) = self.note.as_ref().and_then(|n| n.session.clone()) else {
            self.notice = "No unsaved edit.".into();
            cx.notify();
            return;
        };
        match self.workspace.store.save(&session.id, session.generation) {
            Ok(outcome) => {
                self.notice = save_message(&outcome).into();
                if let SaveOutcome::Saved(receipt) = &outcome
                    && !receipt.needs_refresh.is_empty()
                {
                    self.notice.push_str(&format!(" {} directly derived records may need refresh. Open What changed for the affected records and a follow-up.", receipt.needs_refresh.len()));
                }
                if matches!(outcome, SaveOutcome::Saved(_)) {
                    self.open_note(&session.note, None, window, cx);
                } else if let Ok(Some(record)) = self.workspace.store.note(&session.note)
                    && let RecordData::Note(current) = record.data
                {
                    // A refused Save compares against the current saved record,
                    // while the editor and recovery buffer retain the owner's text.
                    self.current_view
                        .update(cx, |view, cx| view.set_text(&current.markdown, cx));
                }
            }
            Err(e) => self.notice = e.to_string(),
        }
        self.refresh();
        cx.notify();
    }
    fn discard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.note.as_ref().and_then(|n| n.session.clone()) else {
            return;
        };
        match self
            .workspace
            .store
            .discard(&session.id, session.generation)
        {
            Ok(BufferOutcome::Updated(closed)) if closed.closed => {
                self.open_note(&session.note, None, window, cx);
                self.notice = "Discarded the unsaved edit.".into();
            }
            Ok(_) => {
                self.notice = "A newer edit exists. It was preserved; open it from Home.".into()
            }
            Err(e) => self.notice = e.to_string(),
        }
        self.refresh();
        cx.notify();
    }
    fn new_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Note title"));
        let target = cx.entity().downgrade();
        input.update(cx, |input, cx| input.focus(window, cx));
        window.open_dialog(cx, move |dialog, _, _| {
            let input = input.clone();
            let target = target.clone();
            dialog.title("New note").child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Input::new(&input))
                    .child(
                        Button::new("create-named-note")
                            .label("Create note")
                            .on_click(move |_, window, cx| {
                                let name = input.read(cx).value().to_string();
                                if name.trim().is_empty() {
                                    return;
                                }
                                let mut created = false;
                                let _ = target.update(cx, |this, cx| {
                                    match this.workspace.new_note(name.trim(), "") {
                                        Ok(record) => {
                                            this.open_note(&record.id, None, window, cx);
                                            if let Some(note) = &mut this.note {
                                                note.editing = true;
                                            }
                                            created = true;
                                        }
                                        Err(e) => this.notice = e.to_string(),
                                    }
                                    cx.notify();
                                });
                                if created {
                                    window.close_dialog(cx);
                                    let _ = target.update(cx, |this, cx| {
                                        this.focus_visible_editor(window, cx);
                                    });
                                }
                            }),
                    ),
            )
        });
    }
    fn change(&mut self, reason: &str, record: Record, data: RecordData, cx: &mut Context<Self>) {
        let request = ChangeRequest {
            reason: reason.into(),
            writes: vec![Put {
                id: record.id,
                expected_version: Some(record.version),
                archived: record.archived,
                data,
            }],
            inputs: vec![],
        };
        match self
            .workspace
            .owner_change(&format!("ui:{}", uuid::Uuid::new_v4()), &request)
        {
            Ok(outcome) => self.notice = outcome_message(&outcome),
            Err(e) => self.notice = e.to_string(),
        };
        self.refresh();
        cx.notify();
    }
    fn open_review(&mut self, candidate: Prepared, window: &mut Window, cx: &mut Context<Self>) {
        self.preserve_visible_note_input(cx);
        window.blur(cx);
        let write = candidate
            .request
            .writes
            .iter()
            .find(|w| matches!(w.data, RecordData::Note(_)));
        let (id, text, current) = write
            .map(|write| {
                let RecordData::Note(note) = &write.data else {
                    unreachable!()
                };
                let current = self
                    .workspace
                    .store
                    .note(&write.id)
                    .ok()
                    .flatten()
                    .and_then(|r| {
                        if let RecordData::Note(n) = r.data {
                            Some(n.markdown)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                (Some(write.id.clone()), note.markdown.clone(), current)
            })
            .unwrap_or_default();
        let groups = self
            .workspace
            .review_groups(candidate.operation.as_str())
            .unwrap_or(ReviewGroups {
                groups: vec![],
                ungrouped_writes: vec![],
            });
        let selected = groups.groups.iter().map(|group| group.id.clone()).collect();
        self.review = Some(Review {
            candidate,
            note: id,
            original: text.clone(),
            edited: false,
            groups,
            selected,
        });
        self.review_editor
            .update(cx, |e, cx| e.set_value(text, window, cx));
        self.current_view
            .update(cx, |v, cx| v.set_text(&current, cx));
        self.focus_visible_editor(window, cx);
        cx.notify();
    }
    fn prepare_review_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = &self.review else { return };
        if review.edited {
            self.notice = "Prepare your edited text before choosing proposal groups.".into();
            cx.notify();
            return;
        }
        let selected = review.selected.iter().cloned().collect::<Vec<_>>();
        match self
            .workspace
            .review_select(review.candidate.operation.as_str(), &selected)
        {
            Ok(candidate) => {
                self.open_review(candidate, window, cx);
                self.notice = "The selected whole records are prepared. Review this new proposal, then apply it.".into();
            }
            Err(error) => self.notice = error.to_string(),
        }
        self.refresh();
        cx.notify();
    }
    fn prepare_review_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = &self.review else { return };
        let Some(note) = &review.note else { return };
        let text = self.review_editor.read(cx).value().to_string();
        match self
            .workspace
            .review_replace(review.candidate.operation.as_str(), note, &text)
        {
            Ok(candidate) => {
                self.open_review(candidate, window, cx);
                self.notice =
                    "Your final text is prepared. Review this version, then apply it.".into();
            }
            Err(e) => self.notice = e.to_string(),
        };
        self.refresh();
        cx.notify();
    }
    fn apply_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = &self.review else { return };
        if review.edited || review.selected.len() != review.groups.groups.len() {
            self.notice = "Prepare your edited text or selected groups before applying it.".into();
            cx.notify();
            return;
        }
        match self
            .workspace
            .review_apply(review.candidate.operation.as_str())
        {
            Ok(outcome) => {
                self.notice = outcome_message(&outcome);
                if matches!(outcome, ApplyOutcome::Applied(_)) {
                    self.review = None;
                    self.focus_visible_editor(window, cx);
                }
            }
            Err(e) => self.notice = e.to_string(),
        };
        self.refresh();
        cx.notify();
    }
    fn dismiss_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = &self.review else { return };
        match self
            .workspace
            .review_dismiss(review.candidate.operation.as_str())
        {
            Ok(()) => {
                self.review = None;
                self.focus_visible_editor(window, cx);
                self.notice = "Dismissed the proposal.".into();
            }
            Err(error) => self.notice = error.to_string(),
        }
        self.refresh();
        cx.notify();
    }
    fn capture_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = &self.note else { return };
        if open.session.is_some() {
            self.notice = "Save your note before attaching a passage comment.".into();
            cx.notify();
            return;
        }
        let RecordData::Note(note) = &open.record.data else {
            return;
        };
        let range = if open.editing {
            Some(self.note_editor.read(cx).selected_range())
        } else {
            let view = self.note_view.read(cx);
            if view.rendered_text().source() != note.markdown {
                self.notice =
                    "The note is still loading. Select the passage again when it is ready.".into();
                cx.notify();
                return;
            }
            view.selected_source_range()
        };
        if let Some(range) = range
            && range.start < range.end
            && let Some(quote) = note.markdown.get(range.clone())
        {
            self.comment_anchor = Some((
                open.record.id.clone(),
                open.record.version,
                range,
                quote.into(),
            ));
            self.comment_input.update(cx, |e, cx| e.focus(window, cx));
        } else {
            self.notice =
                "Select a passage to discuss. The selection must map to the saved note.".into();
        }
        cx.notify();
    }
    fn post_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((note, version, range, quote)) = self.comment_anchor.clone() else {
            return;
        };
        let body = self.comment_input.read(cx).value().to_string();
        if body.trim().is_empty() {
            return;
        }
        let key = format!("comment:{}", uuid::Uuid::new_v4());
        let operation = match self.workspace.store.allocate_operation(&key) {
            Ok(op) => op,
            Err(e) => {
                self.notice = e.to_string();
                cx.notify();
                return;
            }
        };
        let (thread, request) = passage_request(&operation, &note, version, range, &quote, &body);
        match self.workspace.owner_change(&key, &request) {
            Ok(outcome) => {
                self.notice = outcome_message(&outcome);
                if matches!(outcome, ApplyOutcome::Applied(_)) {
                    self.comment_anchor = None;
                    self.comment_input
                        .update(cx, |e, cx| e.set_value("", window, cx));
                    self.navigate(Page::Thread(thread), window, cx);
                }
            }
            Err(e) => self.notice = e.to_string(),
        };
        self.refresh();
        cx.notify();
    }
    fn start_run(
        &mut self,
        prompt: String,
        thread: String,
        sources: Vec<brn_ai::threads::TransientSource>,
        owner_request: Option<brn_ai::threads::OwnerRequest>,
        cx: &mut Context<Self>,
    ) {
        if let Some(worker) = &self.worker {
            self.notice = worker.kind.busy_notice().into();
            cx.notify();
            return;
        }
        self.refresh();
        let selection = match self.settings.provider.to_ascii_lowercase().as_str() {
            "codex" | "chatgpt" => brn_ai::Selection {
                provider: brn_ai::Provider::Chatgpt,
                model: self.settings.model.clone(),
            },
            "copilot" => brn_ai::Selection {
                provider: brn_ai::Provider::Copilot,
                model: self.settings.model.clone(),
            },
            _ => {
                self.notice = "Choose a provider and model in Settings before asking BRN.".into();
                cx.notify();
                return;
            }
        };
        let effort = match self.settings.effort.as_str() {
            "low" => brn_ai::ReasoningEffort::Low,
            "medium" => brn_ai::ReasoningEffort::Medium,
            "high" => brn_ai::ReasoningEffort::High,
            _ => {
                self.notice = "Choose a supported reasoning effort in Settings.".into();
                cx.notify();
                return;
            }
        };
        let request = brn_ai::threads::ThreadRunRequest {
            owner_request,
            data_dir: self.data_dir.clone(),
            thread: thread.clone(),
            prompt,
            selection,
            effort,
            max_tool_rounds: 8,
            auth_file: self.settings.auth_file.as_ref().map(PathBuf::from),
            credentials_dir: self.settings.credentials_dir.as_ref().map(PathBuf::from),
            sources,
        };
        let prepared = match brn_ai::threads::prepare_run(request) {
            Ok(prepared) => prepared,
            Err(error) => {
                self.notice = format!(
                    "Could not start. Your saved message remains available; Continue from the thread. {error}"
                );
                self.refresh();
                cx.notify();
                return;
            }
        };
        let run_id = prepared.run_id().to_owned();
        let (sender, receiver) = mpsc::sync_channel(128);
        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        let progress_sender = sender.clone();
        let spawned = std::thread::Builder::new().name("brn-thread-run".into()).spawn(move || {
            let result=tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e|e.to_string()).and_then(|runtime|runtime.block_on(brn_ai::threads::run_prepared(prepared,worker_cancel,Arc::new(move |event| {let item=match event{brn_ai::AiEvent::Text(text)=>WorkerEvent::Text(text),brn_ai::AiEvent::AnswerFinalized(text)=>WorkerEvent::AnswerFinalized(text),brn_ai::AiEvent::ToolStarted{name}=>WorkerEvent::Progress(brn_ai::threads::tool_activity(&name).unwrap_or("Working…").into()),brn_ai::AiEvent::BudgetProgress{model_turns:_,tool_rounds,max_tool_rounds}=>WorkerEvent::Progress(format!("Working · {tool_rounds} of {max_tool_rounds} steps used"))};let _=progress_sender.try_send(item);}))).map(|result|match result.answer.terminal{brn_ai::AiTerminal::Completed=>"Finished. The result and any saved changes are in this thread.".into(),brn_ai::AiTerminal::Interrupted=>"Interrupted. Continue starts again with this thread and current notes.".into(),brn_ai::AiTerminal::Failed(e)=>e.to_string()}).map_err(|e|e.to_string()));
            let _ = sender.send(WorkerEvent::Finished(result));
        });
        if let Err(error) = spawned {
            self.notice = format!(
                "Could not start. Your saved instruction remains available; Continue from the thread. {error}"
            );
            self.refresh();
            cx.notify();
            return;
        }
        self.worker = Some(Worker {
            receiver,
            cancel,
            conversion_cancel: Arc::new(AtomicBool::new(false)),
            kind: WorkerKind::Run,
            thread: Some(thread),
            run_id: Some(run_id),
        });
        self.stream.clear();
        self.progress = "Starting…".into();
        cx.notify();
    }
    fn can_stop_and_send(&self) -> bool {
        self.pending_send.is_none()
            && self.worker.as_ref().is_some_and(|worker| {
                worker.kind == WorkerKind::Run
                    && worker.run_id.is_some()
                    && matches!(&self.page, Page::Thread(thread) if worker.thread.as_ref() == Some(thread))
            })
    }
    fn stop_and_send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_stop_and_send() {
            return;
        }
        let prompt = self.composer.read(cx).value().to_string();
        if prompt.trim().is_empty() {
            return;
        }
        let worker = self.worker.as_ref().expect("available worker");
        let thread = worker.thread.clone().expect("run thread");
        let run_id = worker.run_id.as_ref().expect("prepared run");
        match self.workspace.stop_and_post(
            &format!("stop-and-send:{}", uuid::Uuid::new_v4()),
            run_id,
            &thread,
            &prompt,
        ) {
            Ok(_) => {
                // The durable commit and run fence precede input clearing and cancellation.
                self.pending_send = Some(PendingSend {
                    thread: thread.clone(),
                    prompt,
                });
                self.composer_drafts
                    .posted(&ComposerDestination::Thread(thread));
                self.composer
                    .update(cx, |editor, cx| editor.set_value("", window, cx));
                worker.cancel.cancel();
                self.progress =
                    "Stopping… New instruction saved; waiting for this run to finish.".into();
                self.notice = "Instruction saved. Waiting for the current run to stop.".into();
                self.refresh();
            }
            Err(error) => self.notice = format!("Could not send: {error}. Your text is preserved."),
        }
        cx.notify();
    }
    fn ask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.can_stop_and_send() {
            self.stop_and_send(window, cx);
        } else {
            self.ask_with_authority(None, window, cx);
        }
    }
    fn ask_with_authority(
        &mut self,
        owner_request: Option<brn_ai::threads::OwnerRequest>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(worker) = &self.worker {
            self.notice = worker.kind.busy_notice().into();
            cx.notify();
            return;
        }
        let prompt = self.composer.read(cx).value().to_string();
        let destination = self.page.composer_destination();
        if prompt.trim().is_empty() {
            return;
        }
        let thread = match &self.page {
            Page::Thread(id) => Ok(id.clone()),
            _ => self.workspace.new_thread(&short(&prompt)).map(|r| r.id),
        };
        match thread.and_then(|thread| {
            if let Page::Note(note) = &self.page {
                let key = format!("ask-link:{}", uuid::Uuid::new_v4());
                let operation = self.workspace.store.allocate_operation(&key)?;
                let request = ChangeRequest {
                    reason: "Owner asked about a note".into(),
                    writes: vec![Put {
                        id: operation.creation_id(0),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Link(Link {
                            from: thread.clone(),
                            to: note.clone(),
                            relation: "discusses".into(),
                        }),
                    }],
                    inputs: vec![],
                };
                let outcome = self.workspace.owner_change(&key, &request)?;
                if !matches!(outcome, ApplyOutcome::Applied(_)) {
                    return Err(AppError::Invalid(outcome_message(&outcome)));
                }
            }
            self.workspace
                .post_message(&thread, &prompt)
                .map(|_| thread)
        }) {
            Ok(thread) => {
                self.composer_drafts.posted(&destination);
                self.composer
                    .update(cx, |e, cx| e.set_value("", window, cx));
                self.navigate(Page::Thread(thread.clone()), window, cx);
                self.start_run(prompt, thread, vec![], owner_request, cx);
            }
            Err(e) => self.notice = e.to_string(),
        }
        self.refresh();
        cx.notify();
    }
    fn continue_run(&mut self, thread: String, cx: &mut Context<Self>) {
        self.start_run("Continue the work from the saved conversation, current linked notes and recorded outcomes. Recheck current versions and authority.".into(),thread,vec![],None,cx);
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        if self.pending_send.take().is_some() {
            self.notice = "Automatic continuation stopped. Your saved instruction remains in the thread; Continue when ready.".into();
        }
        if let Some(worker) = &self.worker
            && worker.kind.can_cancel()
        {
            worker.cancel.cancel();
            worker
                .conversion_cancel
                .store(true, std::sync::atomic::Ordering::SeqCst);
            self.progress = "Stopping… Your saved progress will remain available.".into();
        }
        cx.notify();
    }
    fn stream_for(&self, thread: &str) -> Option<&str> {
        self.worker
            .as_ref()
            .filter(|worker| worker.thread.as_deref() == Some(thread))
            .filter(|_| !self.stream.is_empty())
            .map(|_| self.stream.as_str())
    }
    fn resolve_discussion(&mut self, thread: &str, cx: &mut Context<Self>) {
        match self.workspace.resolve_thread(thread) {
            Ok(outcome) => {
                self.notice = outcome_message(&outcome);
                if matches!(outcome, ApplyOutcome::Applied(_))
                    && self
                        .worker
                        .as_ref()
                        .is_some_and(|worker| worker.thread.as_deref() == Some(thread))
                {
                    self.cancel(cx);
                }
            }
            Err(e) => self.notice = e.to_string(),
        };
        self.refresh();
        cx.notify();
    }
    fn poll_worker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut events = Vec::new();
        let mut disconnected = false;
        if let Some(worker) = &self.worker {
            loop {
                match worker.receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        if events.is_empty() {
            if disconnected {
                self.worker = None;
                self.pending_send = None;
                self.stream.clear();
                self.progress.clear();
                self.notice="Work stopped unexpectedly. Saved progress remains available; continue from the thread.".into();
                self.refresh();
                cx.notify();
            }
            return;
        }
        let mut finished = false;
        for event in events {
            // Events belong to the old worker, even when a producer queues after Finished.
            if finished {
                continue;
            }
            match event {
                WorkerEvent::AnswerFinalized(text) => {
                    self.stream = text;
                }
                WorkerEvent::Text(text) => {
                    self.stream.push_str(&text);
                }
                WorkerEvent::Progress(text) => self.progress = text,
                WorkerEvent::Finished(result) => {
                    finished = true;
                    let kind = self.worker.as_ref().map(|w| w.kind);
                    let thread = self
                        .worker
                        .as_ref()
                        .and_then(|worker| worker.thread.clone());
                    self.worker = None;
                    self.stream.clear();
                    self.progress.clear();
                    self.notice = result.unwrap_or_else(|e| format!("Work stopped: {e}"));
                    if let Some(thread) = thread {
                        let name = self
                            .records
                            .iter()
                            .find(|record| record.id == thread)
                            .map(title)
                            .unwrap_or(thread);
                        self.notice = format!("Work in {} · {}", short(&name), self.notice);
                    }
                    match kind {
                        Some(WorkerKind::Export) => self.export_status = self.notice.clone(),
                        Some(WorkerKind::Backup) => self.backup_status = self.notice.clone(),
                        _ => {}
                    }
                    self.refresh();
                }
                WorkerEvent::Converted { result, title } => {
                    finished = true;
                    self.worker = None;
                    self.progress.clear();
                    match *result {
                        Ok(converted) => {
                            if converted.intent == ImportIntent::FullNote {
                                match self.workspace.import_full(
                                    &format!("full-import:{}", uuid::Uuid::new_v4()),
                                    &title,
                                    &converted,
                                ) {
                                    Ok(outcome) => {
                                        self.notice = outcome_message(&outcome);
                                        let id = if let ApplyOutcome::Applied(receipt) = outcome {
                                            receipt
                                                .writes
                                                .into_iter()
                                                .find(|w| {
                                                    matches!(w.after.data, RecordData::Note(_))
                                                })
                                                .map(|w| w.after.id)
                                        } else {
                                            None
                                        };
                                        if let Some(id) = id {
                                            self.open_note(&id, None, window, cx);
                                        }
                                    }
                                    Err(e) => self.notice = e.to_string(),
                                }
                            } else {
                                self.useful_intake(converted, title, window, cx);
                            }
                        }
                        Err(e) => self.notice = format!("Import stopped: {e}"),
                    }
                    self.refresh();
                }
            }
        }
        if finished {
            if let Some(pending) = self.pending_send.take() {
                // Launch only after draining the complete old batch, without navigation or grants.
                self.start_run(pending.prompt, pending.thread, vec![], None, cx);
            }
        } else if disconnected && self.worker.is_some() {
            self.worker = None;
            self.pending_send = None;
            self.stream.clear();
            self.progress.clear();
            self.notice = "Work stopped unexpectedly. Your saved instruction remains available; Continue from the thread.".into();
            self.refresh();
        }
        cx.notify();
    }
    fn useful_intake(
        &mut self,
        converted: ConversionResult,
        title: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let report = brn_threads_app::import_report(&converted);
        let fingerprint = brn_intake::hex(&converted.source_sha256);
        match self
            .workspace
            .new_thread(&format!("Information from {title}"))
        {
            Ok(thread) => {
                let instruction = "Capture useful supported information from this source, maintaining relevant ordinary notes and source references. Preserve qualifications; do not copy the whole source as a note. Retain meaningful selected figures using the read_source host links and preserve detected omissions; unchecked coverage does not mean proven loss.";
                match self.workspace.post_message(&thread.id, instruction) {
                    Ok(_) => {
                        self.navigate(Page::Thread(thread.id.clone()), window, cx);
                        self.start_run(
                            instruction.into(),
                            thread.id,
                            vec![brn_ai::threads::TransientSource {
                                reference: converted.source_reference,
                                text: converted.markdown,
                                fingerprint: Some(fingerprint),
                                report: Some(report),
                                assets: converted
                                    .assets
                                    .into_iter()
                                    .map(|asset| brn_ai::threads::TransientAsset {
                                        name: asset.name,
                                        media_type: asset.media_type,
                                        sha256: asset.sha256,
                                        width: asset.width,
                                        height: asset.height,
                                        bytes: asset.bytes,
                                    })
                                    .collect(),
                            }],
                            None,
                            cx,
                        );
                    }
                    Err(e) => self.notice = e.to_string(),
                }
            }
            Err(e) => self.notice = e.to_string(),
        }
    }
    fn choose_import(&mut self, intent: ImportIntent, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(worker) = &self.worker {
            self.notice = worker.kind.busy_notice().into();
            cx.notify();
            return;
        }
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose a document, note or email".into()),
        });
        let data = self.data_dir.clone();
        cx.spawn_in(window, async move |this, cx| {
            let paths = receiver.await;
            let _ = this.update_in(cx, |this, _, cx| match paths {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        this.convert_file(data, path, intent, cx);
                    }
                }
                Ok(Ok(None)) => {}
                _ => {
                    this.notice = "Could not choose a file.".into();
                    cx.notify();
                }
            });
        })
        .detach();
    }
    fn convert_file(
        &mut self,
        _data: PathBuf,
        path: PathBuf,
        intent: ImportIntent,
        cx: &mut Context<Self>,
    ) {
        if let Some(worker) = &self.worker {
            self.notice = format!("{} Then choose this file again.", worker.kind.busy_notice());
            cx.notify();
            return;
        }
        let kind = match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("docx") => DocumentKind::Docx,
            Some("pdf") => DocumentKind::Pdf,
            Some("md" | "markdown") => DocumentKind::Markdown,
            Some("txt") => DocumentKind::Text,
            Some("eml") => DocumentKind::Eml,
            _ => {
                self.notice = "Choose DOCX, PDF, Markdown, text or an EML email file.".into();
                cx.notify();
                return;
            }
        };
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Imported note")
            .to_owned();
        let reference = path.to_string_lossy().to_string();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let conversion_cancel = cancel_flag.clone();
        let (sender, receiver) = mpsc::sync_channel(8);
        std::thread::spawn(move || {
            let result = (|| -> Result<ConversionResult, String> {
                let limits = brn_intake::IntakeLimits::default();
                let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
                if !metadata.is_file() || metadata.len() > limits.max_input_bytes as u64 {
                    return Err("The file exceeds the supported import size.".into());
                }
                let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                let helper = std::env::current_exe()
                    .map_err(|e| e.to_string())?
                    .with_file_name("brn-intake-helper");
                let converter = Converter {
                    helper_path: helper,
                    pdf_tools: PdfTools::discover(),
                    temp_root: std::env::temp_dir().join("brn-threads-intake"),
                    limits,
                };
                converter
                    .convert(
                        ConversionRequest {
                            kind,
                            intent,
                            bytes: &bytes,
                            source_reference: &reference,
                            inventory: None,
                        },
                        &conversion_cancel,
                    )
                    .map_err(|e| e.to_string())
            })();
            let _ = sender.send(WorkerEvent::Converted {
                result: Box::new(result),
                title,
            });
        });
        self.worker = Some(Worker {
            receiver,
            cancel: CancellationToken::new(),
            conversion_cancel: cancel_flag,
            kind: WorkerKind::Import,
            thread: None,
            run_id: None,
        });
        self.progress = if intent == ImportIntent::FullNote {
            "Reading the full document…"
        } else {
            "Reading useful information…"
        }
        .into();
        cx.notify();
    }
    fn choose_output(&mut self, backup: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(worker) = &self.worker {
            self.notice = worker.kind.busy_notice().into();
            cx.notify();
            return;
        }
        if backup {
            match self.workspace.settings() {
                Ok(settings) => {
                    let chosen = settings.backup_directory.is_some();
                    self.settings = settings;
                    if chosen {
                        self.output(None, cx);
                    } else {
                        self.choose_backup_folder(true, window, cx);
                    }
                }
                Err(error) => {
                    self.backup_status = error.to_string();
                    cx.notify();
                }
            }
            return;
        }
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose a folder for a new Markdown export".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver.await;
            let _ = this.update_in(cx, |this, _, cx| {
                if let Ok(Ok(Some(paths))) = result
                    && let Some(parent) = paths.first()
                {
                    this.output(Some(parent.clone()), cx);
                }
            });
        })
        .detach();
    }
    fn choose_backup_folder(
        &mut self,
        create_backup: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(worker) = &self.worker {
            self.notice = worker.kind.busy_notice().into();
            cx.notify();
            return;
        }
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose a folder for full backups".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = receiver
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result.map_err(|error| error.to_string()));
            let _ = this.update_in(cx, |this, _, cx| {
                this.backup_folder_chosen(result, create_backup, cx);
            });
        })
        .detach();
    }
    fn backup_folder_chosen(
        &mut self,
        result: Result<Option<Vec<PathBuf>>, String>,
        create_backup: bool,
        cx: &mut Context<Self>,
    ) {
        // A different operation may have started while the system picker was open.
        if let Some(worker) = &self.worker {
            self.notice = worker.kind.busy_notice().into();
            cx.notify();
            return;
        }
        let directory = match result {
            Ok(Some(paths)) => match paths.into_iter().next() {
                Some(directory) => directory,
                None => return,
            },
            Ok(None) => return,
            Err(error) => {
                self.backup_status = format!("Could not choose a backup folder: {error}");
                cx.notify();
                return;
            }
        };
        match self.workspace.choose_backup_directory(&directory) {
            Ok(ApplyOutcome::Applied(_)) => {
                self.refresh();
                self.backup_status = "Backup folder saved.".into();
                if create_backup {
                    self.output(None, cx);
                }
            }
            Ok(outcome) => self.backup_status = outcome_message(&outcome),
            Err(error) => self.backup_status = error.to_string(),
        }
        cx.notify();
    }
    // None dispatches a backup using the current persisted preference on the worker.
    fn output(&mut self, export_parent: Option<PathBuf>, cx: &mut Context<Self>) {
        if let Some(worker) = &self.worker {
            self.notice = worker.kind.busy_notice().into();
            cx.notify();
            return;
        }
        let backup = export_parent.is_none();
        let destination =
            export_parent.map(|parent| parent.join(format!("BRN-export-{}", uuid::Uuid::new_v4())));
        let destination_label = destination.as_ref().map(|path| path.display().to_string());
        let data = self.data_dir.clone();
        let (sender, receiver) = mpsc::sync_channel(4);
        std::thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                let workspace = Workspace::open(data).map_err(|e| e.to_string())?;
                let destination = if let Some(destination) = destination {
                    workspace
                        .export_now(&destination)
                        .map_err(|e| e.to_string())?;
                    destination
                } else {
                    workspace
                        .backup_to_saved_directory()
                        .map_err(|e| e.to_string())?
                };
                Ok(format!(
                    "{} created at {}",
                    if backup {
                        "Full backup"
                    } else {
                        "Markdown snapshot"
                    },
                    destination.display()
                ))
            })();
            let _ = sender.send(WorkerEvent::Finished(result));
        });
        if backup {
            self.backup_status = "Creating a full backup in the saved folder…".into();
        } else {
            self.export_status = format!(
                "Creating a current Markdown snapshot at {}…",
                destination_label.expect("export has a destination")
            );
        }
        self.worker = Some(Worker {
            receiver,
            cancel: CancellationToken::new(),
            conversion_cancel: Arc::new(AtomicBool::new(false)),
            kind: if backup {
                WorkerKind::Backup
            } else {
                WorkerKind::Export
            },
            thread: None,
            run_id: None,
        });
        self.progress = "Saving portable workspace data…".into();
        cx.notify();
    }
    fn configure(&mut self, cx: &mut Context<Self>) {
        self.settings.model = self.model.read(cx).value().to_string();
        let auth = self.auth_path.read(cx).value().to_string();
        let credentials = self.credentials_path.read(cx).value().to_string();
        self.settings.auth_file = (!auth.trim().is_empty()).then_some(auth);
        self.settings.credentials_dir = (!credentials.trim().is_empty()).then_some(credentials);
        match self.workspace.configure(self.settings.clone()) {
            Ok(outcome) => self.notice = outcome_message(&outcome),
            Err(e) => self.notice = e.to_string(),
        };
        self.refresh();
        cx.notify();
    }
    fn choose_delegation(&mut self, choice: DelegationChoice, cx: &mut Context<Self>) {
        match self.workspace.choose_delegation(choice) {
            Ok(outcome) => self.notice = outcome_message(&outcome),
            Err(error) => self.notice = error.to_string(),
        }
        self.refresh();
        cx.notify();
    }
    fn working_knowledge(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let status = delegation_status(&self.settings);
        let mut knowledge = panel()
            .id(if self.settings.delegation_chosen { "delegation-settings" } else { "delegation-pending" })
            .child(heading("Working knowledge"))
            .child(div().id("delegation-status").aria_label(status).child(status).test_support())
            .child("Maintain ordinary notes lets BRN update ordinary notes and useful source references. Review changes first keeps proposed changes for your approval. Protected notes still need an applicable instruction or your review.")
            .child(Button::new("delegation-maintain")
                .label("Maintain ordinary notes")
                .selected(self.settings.delegation_chosen && !self.settings.review_first)
                .on_click(cx.listener(|this, _, _, cx| this.choose_delegation(DelegationChoice::MaintainOrdinaryNotes, cx))))
            .child(Button::new("delegation-review-first")
                .label("Review changes first")
                .selected(self.settings.delegation_chosen && self.settings.review_first)
                .on_click(cx.listener(|this, _, _, cx| this.choose_delegation(DelegationChoice::ReviewFirst, cx))));
        if self.settings.delegation_chosen {
            knowledge = knowledge.child(
                Button::new("maintenance-setting")
                    .label(if self.settings.maintenance {
                        "Pause maintenance"
                    } else {
                        "Resume maintenance"
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        match this.workspace.pause(this.settings.maintenance) {
                            Ok(outcome) => this.notice = outcome_message(&outcome),
                            Err(error) => this.notice = error.to_string(),
                        }
                        this.refresh();
                        cx.notify();
                    })),
            );
        }
        knowledge.test_support().into_any_element()
    }
    fn keep_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.review.as_ref().is_some_and(|r| r.edited) {
            self.prepare_review_edit(window, cx);
        }
        if self.review.as_ref().is_some_and(|r| !r.edited) {
            self.review = None;
            self.focus_visible_editor(window, cx);
        }
        cx.notify();
    }
    fn select_review_note(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.review.as_ref().is_some_and(|r| r.edited) {
            self.prepare_review_edit(window, cx);
            if self.review.as_ref().is_some_and(|r| r.edited) {
                return;
            }
        }
        let Some(review) = &mut self.review else {
            return;
        };
        let Some(write) = review.candidate.request.writes.iter().find(|w| w.id == id) else {
            return;
        };
        let RecordData::Note(note) = &write.data else {
            return;
        };
        let text = note.markdown.clone();
        review.note = Some(id.clone());
        review.original = text.clone();
        review.edited = false;
        let current = self
            .workspace
            .store
            .note(&id)
            .ok()
            .flatten()
            .and_then(|r| {
                if let RecordData::Note(n) = r.data {
                    Some(n.markdown)
                } else {
                    None
                }
            })
            .unwrap_or_default();
        self.review_editor
            .update(cx, |e, cx| e.set_value(text, window, cx));
        self.current_view
            .update(cx, |v, cx| v.set_text(&current, cx));
        cx.notify();
    }
    fn save_copy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = &self.note else { return };
        let text = self.note_editor.read(cx).value().to_string();
        let name = format!("Recovered {}", title(&open.record));
        match self.workspace.new_note(&name, &text) {
            Ok(record) => {
                self.open_note(&record.id, None, window, cx);
                self.notice="Saved your text as a separate working note. The original edit remains recoverable from Home.".into();
            }
            Err(e) => self.notice = e.to_string(),
        };
        self.refresh();
        cx.notify();
    }
    fn sidebar(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut rail = div()
            .w(px(245.))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(rgb(0xe8edf1))
            .child(heading("BRN Threads"))
            .child(
                Button::new("home")
                    .label("Home")
                    .selected(self.page == Page::Home)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.navigate(Page::Home, window, cx)),
                    ),
            )
            .child(
                Button::new("actions")
                    .label("Actions")
                    .selected(self.page == Page::Actions)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.navigate(Page::Actions, window, cx)),
                    ),
            )
            .child(
                Button::new("history")
                    .label("What changed")
                    .selected(self.page == Page::History)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.navigate(Page::History, window, cx)),
                    ),
            )
            .child(
                Button::new("settings")
                    .label("Settings")
                    .selected(self.page == Page::Settings)
                    .on_click(
                        cx.listener(|this, _, window, cx| {
                            this.navigate(Page::Settings, window, cx)
                        }),
                    ),
            )
            .child(Input::new(&self.search).w_full());
        let mut list = div()
            .id("workspace-list")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2();
        if !self.search.read(cx).value().trim().is_empty() {
            list = list.child(heading("Current results"));
            for (i, hit) in self.hits.iter().enumerate() {
                let id = hit.note.clone();
                let start = hit.start;
                let end = hit.end;
                list = list
                    .child(
                        Button::new(format!("hit-{i}"))
                            .label(short(&hit.title))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_note(&id, None, window, cx);
                                this.reveal(start..end, window, cx);
                            })),
                    )
                    .child(div().text_sm().child(short(&hit.quote)));
            }
            if self.hits.is_empty() {
                list = list.child("No current notes match.");
            }
        } else {
            list = list.child(heading("Threads"));
            for record in self
                .records
                .iter()
                .filter(|r| !r.archived && matches!(r.data, RecordData::Thread(_)))
                .rev()
            {
                let id = record.id.clone();
                let RecordData::Thread(thread) = &record.data else {
                    continue;
                };
                let label = format!(
                    "{}{}",
                    if thread.state == ThreadState::Resolved {
                        "✓ "
                    } else {
                        ""
                    },
                    short(&thread.title)
                );
                list = list.child(
                    Button::new(format!("thread-{id}"))
                        .label(label)
                        .selected(self.page == Page::Thread(id.clone()))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.navigate(Page::Thread(id.clone()), window, cx)
                        })),
                );
            }
            list = list.child(heading("Notes")).child(
                Button::new("new-note")
                    .label("New note")
                    .on_click(cx.listener(|this, _, window, cx| this.new_note(window, cx))),
            );
            for record in self.records.iter().filter(|r| {
                !r.archived && matches!(&r.data,RecordData::Note(n) if n.superseded_by.is_none())
            }) {
                let id = record.id.clone();
                list = list.child(
                    Button::new(format!("note-{id}"))
                        .label(short(&title(record)))
                        .selected(self.page == Page::Note(id.clone()))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_note(&id, None, window, cx)
                        })),
                );
            }
        }
        rail = rail
            .child(list)
            .child(
                Button::new("useful-import")
                    .label("Add useful information…")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.choose_import(ImportIntent::UsefulInformation, window, cx)
                    })),
            )
            .child(
                Button::new("full-import")
                    .label("Import full note…")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.choose_import(ImportIntent::FullNote, window, cx)
                    })),
            );
        rail.into_any_element()
    }
    fn reveal(
        &mut self,
        range: std::ops::Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.note.as_ref().is_some_and(|n| n.editing) {
            self.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(range, cx);
                editor.focus(window, cx);
            });
        } else {
            self.note_view.update(cx, |view, cx| {
                let snapshot = view.rendered_text();
                if let Some(rendered) = snapshot.range_for_source(range) {
                    let _ = view.set_range_highlights(
                        [RangeHighlight::new(rendered.clone(), rgb(0xffe8a1))],
                        cx,
                    );
                    let _ = view.reveal_range(rendered, cx);
                }
            });
        }
        cx.notify();
    }
    fn home_view(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut home = div()
            .id("home-content")
            .h_full()
            .w_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_4()
            .p_5()
            .child(heading("Home"))
            .child(
                "Give BRN information and a goal. Your notes, work and decisions stay connected.",
            );
        if !self.settings.delegation_chosen {
            home = home.child(self.working_knowledge(cx));
        }
        let mut queue = panel().child(heading("Needs you"));
        let needs:Vec<_>=self.records.iter().filter(|r|!r.archived&&matches!(&r.data,RecordData::Thread(t) if t.state==ThreadState::Open&&!t.attention.is_empty())).collect();
        if needs.is_empty() {
            queue = queue.child("Nothing needs your decision right now.");
        }
        for record in needs {
            let id = record.id.clone();
            let RecordData::Thread(thread) = &record.data else {
                continue;
            };
            let mut item = div().flex().flex_col().gap_1().child(
                Button::new(format!("attention-{id}"))
                    .label(short(&thread.title))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.navigate(Page::Thread(id.clone()), window, cx)
                    })),
            );
            for attention in &thread.attention {
                item = item.child(attention.reason.clone());
            }
            queue = queue.child(item);
        }
        home = home.child(queue);
        if !self.buffers.is_empty() {
            let mut recovery = panel().child(heading("Recover your edits")).child(
                "These edits remain separate from current notes until you Save or discard them.",
            );
            for (i, buffer) in self.buffers.iter().enumerate() {
                let buffer = buffer.clone();
                let id = buffer.note.clone();
                let label = self
                    .records
                    .iter()
                    .find(|r| r.id == id)
                    .map(title)
                    .unwrap_or_else(|| "Unavailable note".into());
                recovery = recovery.child(
                    Button::new(format!("recover-{i}"))
                        .label(format!("Recover {}", short(&label)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_note(&id, Some(buffer.clone()), window, cx)
                        })),
                );
            }
            home = home.child(recovery);
        }
        if !self.candidates.is_empty() {
            let mut review = panel().child(heading("Review changes"));
            for candidate in &self.candidates {
                let candidate = candidate.clone();
                review = review.child(
                    Button::new(format!("candidate-{}", candidate.operation.as_str()))
                        .label(short(&candidate.request.reason))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_review(candidate.clone(), window, cx)
                        })),
                );
            }
            home = home.child(review);
        }
        let mut recent = panel().child(heading("What changed"));
        for receipt in self.history.iter().rev().take(6) {
            recent = recent.child(format!("{} · {}", receipt.committed_at, receipt.reason));
        }
        if self.history.is_empty() {
            recent = recent.child("Your meaningful changes will appear here.");
        }
        home.child(recent).into_any_element()
    }
    fn safe_message(&self, id: String, text: String) -> TextView {
        let assets = self.assets.clone();
        TextView::markdown(id, text)
            .w_full()
            .markdown_extensions(
                MarkdownExtensions::default()
                    .plugin(LiteralHtml { block: true })
                    .plugin(LiteralHtml { block: false })
                    .parser_revision(1),
            )
            .image_source(move |uri| {
                assets
                    .get(uri.as_ref())
                    .cloned()
                    .unwrap_or_else(|| Arc::new(Image::from_bytes(ImageFormat::Png, Vec::new())))
                    .into()
            })
            .on_link_click(|url, _, _, cx| {
                if url.starts_with("https://") || url.starts_with("http://") {
                    cx.open_url(url);
                }
                cx.stop_propagation();
            })
    }
    fn action_row(&self, record: &Record, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let RecordData::Action(action) = &record.data else {
            return div().into_any_element();
        };
        let mut row = panel()
            .child(heading(short(&action.description)))
            .child(format!(
                "{:?}{}",
                action.state,
                action
                    .due
                    .as_ref()
                    .map(|due| format!(" · due {due}"))
                    .unwrap_or_default()
            ));
        if let Some(evidence) = &action.evidence {
            row = row.child(evidence.clone());
        }
        if action.state != ActionState::Done && action.state != ActionState::Cancelled {
            let record_open = record.clone();
            let mut opened = action.clone();
            opened.state = ActionState::Open;
            let record_wait = record.clone();
            let mut waiting = action.clone();
            waiting.state = ActionState::Waiting;
            let record_done = record.clone();
            let mut done = action.clone();
            done.state = ActionState::Done;
            done.evidence = Some("Owner confirmed completion in BRN.".into());
            let record_cancel = record.clone();
            let mut cancelled = action.clone();
            cancelled.state = ActionState::Cancelled;
            row = row.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new(format!("open-action-{}", record.id))
                            .label(if action.state == ActionState::Suggested {
                                "Accept action"
                            } else {
                                "Open"
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.change(
                                    "Owner opened an Action",
                                    record_open.clone(),
                                    RecordData::Action(opened.clone()),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new(format!("wait-action-{}", record.id))
                            .label("Waiting")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.change(
                                    "Owner marked an Action Waiting",
                                    record_wait.clone(),
                                    RecordData::Action(waiting.clone()),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new(format!("done-action-{}", record.id))
                            .label("Confirm done")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.change(
                                    "Owner confirmed an Action is complete",
                                    record_done.clone(),
                                    RecordData::Action(done.clone()),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new(format!("cancel-action-{}", record.id))
                            .label("Cancel action")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.change(
                                    "Owner cancelled an Action",
                                    record_cancel.clone(),
                                    RecordData::Action(cancelled.clone()),
                                    cx,
                                )
                            })),
                    ),
            );
        }
        row.into_any_element()
    }
    fn thread_view(&self, id: &str, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(record) = self.records.iter().find(|r| r.id == id) else {
            return div().child("Thread unavailable.").into_any_element();
        };
        let RecordData::Thread(thread) = &record.data else {
            return div().into_any_element();
        };
        let mut conversation = div()
            .id("thread-conversation")
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_4()
            .p_5()
            .child(heading(thread.title.clone()));
        let thread_id = id.to_owned();
        if thread.state == ThreadState::Open {
            conversation = conversation.child(
                Button::new("resolve-discussion")
                    .label("Resolve discussion")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.resolve_discussion(&thread_id, cx);
                    })),
            );
        } else {
            conversation =
                conversation.child("Discussion resolved. Linked Actions keep their own progress.");
        }
        for attention in &thread.attention {
            let mut item = panel().child(attention.reason.clone());
            if let Some(candidate) = self
                .candidates
                .iter()
                .find(|c| attention.record.as_deref() == Some(c.operation.as_str()))
            {
                let candidate = candidate.clone();
                item = item.child(
                    Button::new(format!("thread-review-{}", candidate.operation.as_str()))
                        .label("Review this change")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_review(candidate.clone(), window, cx)
                        })),
                );
            }
            conversation = conversation.child(item);
        }
        let linked_ids = linked(&self.records, id);
        for record in self.records.iter().filter(|r| linked_ids.contains(&r.id)) {
            if let RecordData::Comment(comment) = &record.data {
                let note = self.records.iter().find(|r| r.id == comment.note);
                let location = note.and_then(|r| {
                    if let RecordData::Note(n) = &r.data {
                        comment_location(comment, r.version, &n.markdown)
                    } else {
                        None
                    }
                });
                let mut context = panel()
                    .child(heading("Passage context"))
                    .child(comment.quote.clone())
                    .child(if location.is_some() {
                        "Located in the current note"
                    } else {
                        "Location unresolved · preserved from an earlier note version"
                    });
                if let Some(note) = note {
                    let id = note.id.clone();
                    context = context.child(
                        Button::new(format!("comment-note-{}", record.id))
                            .label(format!("Open {}", short(&title(note))))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_note(&id, None, window, cx)
                            })),
                    );
                }
                conversation = conversation.child(context);
            }
        }
        if let Ok(messages) = self.workspace.messages(id) {
            for message in messages {
                let RecordData::Message(body) = &message.data else {
                    continue;
                };
                conversation = conversation.child(
                    panel()
                        .child(
                            div()
                                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                .child(if body.role == "owner" { "You" } else { "BRN" }),
                        )
                        .child(
                            self.safe_message(format!("message-{}", message.id), body.text.clone()),
                        ),
                );
            }
        }
        if let Some(stream) = self.stream_for(id) {
            conversation = conversation.child(
                panel()
                    .child("BRN · provisional answer")
                    .child(self.safe_message("stream-answer".into(), stream.into())),
            );
        }
        if can_continue_thread(
            &self.records,
            &self.history,
            id,
            &thread.state,
            self.worker.is_some(),
        ) {
            let latest = latest_run(&self.records, &self.history, id);
            let run = latest.and_then(|record| match &record.data {
                RecordData::Run(run) => Some(run),
                _ => None,
            });
            let thread_id = id.to_owned();
            let status = run.map_or_else(
                || "Saved instruction is ready to continue".into(),
                |run| {
                    if run.state == RunState::Working {
                        "Saved run status: Working · current activity is unconfirmed".into()
                    } else {
                        format!("{:?}", run.state)
                    }
                },
            );
            conversation = conversation.child(
                panel()
                    .child(status)
                    .child(run.map_or_else(String::new, |run| run.progress.clone()))
                    .child(run.map_or(
                        "Continue starts fresh with the saved conversation and current notes.",
                        |run| continuation_notice(&run.state),
                    ))
                    .child(
                        Button::new(format!(
                            "continue-{}",
                            latest.map_or(id, |record| record.id.as_str())
                        ))
                        .label("Continue with current context")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.continue_run(thread_id.clone(), cx)
                        })),
                    ),
            );
        }
        let mut context = div()
            .id("thread-linked-work")
            .w(px(320.))
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .bg(rgb(0xf3f5f7))
            .child(heading("Linked work"));
        for record in self
            .records
            .iter()
            .filter(|r| !r.archived && linked_ids.contains(&r.id))
        {
            match &record.data {
                RecordData::Note(note) => {
                    let id = record.id.clone();
                    context = context.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child("Current note excerpt")
                            .child(
                                self.safe_message(
                                    format!("linked-note-preview-{id}"),
                                    note.markdown.clone(),
                                )
                                .max_lines(12),
                            ),
                    );
                    context = context.child(
                        Button::new(format!("linked-note-{id}"))
                            .label(short(&note.title))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_note(&id, None, window, cx)
                            })),
                    );
                }
                RecordData::Action(_) => context = context.child(self.action_row(record, cx)),
                RecordData::Source(source) => {
                    let locator = source.locator.clone();
                    let open_locator = locator.clone();
                    context = context.child(
                        panel()
                            .child(source.label.clone())
                            .child(short(&source.locator))
                            .child(source.outcome.clone())
                            .children(
                                source
                                    .report
                                    .as_ref()
                                    .map(|report| {
                                        report
                                            .issues
                                            .iter()
                                            .map(ImportIssue::display)
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_else(|| source.gaps.clone()),
                            )
                            .child(
                                Button::new(format!("open-source-{}", record.id))
                                    .label("Open source")
                                    .on_click(move |_, _, cx| {
                                        if std::path::Path::new(&open_locator).is_absolute() {
                                            cx.open_with_system(std::path::Path::new(
                                                &open_locator,
                                            ));
                                        } else if open_locator.starts_with("https://")
                                            || open_locator.starts_with("http://")
                                            || open_locator.starts_with("outlook:")
                                        {
                                            cx.open_url(&open_locator);
                                        }
                                    }),
                            )
                            .child(
                                Button::new(format!("copy-source-{}", record.id))
                                    .label("Copy source reference")
                                    .on_click(move |_, _, cx| {
                                        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                            locator.clone(),
                                        ))
                                    }),
                            ),
                    );
                }
                _ => {}
            }
        }
        if linked_ids.is_empty() {
            context =
                context.child("Notes, sources and Actions used in this discussion appear here.");
        }
        div()
            .flex()
            .h_full()
            .w_full()
            .child(conversation)
            .child(context)
            .into_any_element()
    }
    fn note_view(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(open) = &self.note else {
            return div().child("Choose a note.").into_any_element();
        };
        let RecordData::Note(note) = &open.record.data else {
            return div().into_any_element();
        };
        let mut controls = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .items_center()
            .child(heading(note.title.clone()))
            .child(if note.protected {
                "Protected"
            } else {
                "Working note"
            })
            .child(if note.confirmed { "Confirmed" } else { "" })
            .child(
                Button::new("toggle-edit")
                    .label(if open.editing { "Read" } else { "Edit" })
                    .disabled(
                        open.session.is_none()
                            && self.buffers.iter().any(|s| s.note == open.record.id),
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_note_edit(window, cx))),
            )
            .child(
                Button::new("save-note")
                    .label("Save")
                    .disabled(open.session.is_none())
                    .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
            )
            .child(
                Button::new("discard-note")
                    .label("Discard edit")
                    .disabled(open.session.is_none())
                    .on_click(cx.listener(|this, _, window, cx| this.discard(window, cx))),
            )
            .child(
                Button::new("comment-passage")
                    .label("Comment on selection")
                    .disabled(open.session.is_some())
                    .on_click(cx.listener(|this, _, window, cx| this.capture_comment(window, cx))),
            );
        if open.session.is_none() {
            let record = open.record.clone();
            let mut protected = note.clone();
            protected.protected = !protected.protected;
            controls = controls.child(
                Button::new("protect-note")
                    .label(if note.protected {
                        "Make working note"
                    } else {
                        "Protect note"
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.change(
                            "Owner changed note protection",
                            record.clone(),
                            RecordData::Note(protected.clone()),
                            cx,
                        );
                        this.open_note(&record.id, None, window, cx);
                    })),
            );
        }
        let mut content = div()
            .h_full()
            .w_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(controls);
        if let Some(current) = self.records.iter().find(|r| r.id == open.record.id)
            && current.version != open.record.version
        {
            let id = open.record.id.clone();
            content = content.child(
                panel()
                    .child(
                        "This note has a newer saved version. Your visible text remains preserved.",
                    )
                    .child(
                        Button::new("show-current-note")
                            .label("Open current saved note")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_note(&id, None, window, cx)
                            })),
                    ),
            );
        }
        if open.session.is_none()
            && let Some(buffer) = self.buffers.iter().find(|s| s.note == open.record.id)
        {
            let buffer = buffer.clone();
            let id = buffer.note.clone();
            content = content.child(
                panel()
                    .child("A recoverable edit exists for this note.")
                    .child(
                        Button::new("recover-note-edit")
                            .label("Recover saved edit")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_note(&id, Some(buffer.clone()), window, cx)
                            })),
                    ),
            );
        }
        if let Some(coverage) = &note.import {
            if let Some(record) = self.records.iter().find(|r| r.id == coverage.source)
                && let RecordData::Source(source) = &record.data
            {
                let locator = source.locator.clone();
                content = content.child(
                    Button::new("open-import-original")
                        .label("Open external source")
                        .on_click(move |_, _, cx| {
                            if std::path::Path::new(&locator).is_absolute() {
                                cx.open_with_system(std::path::Path::new(&locator));
                            } else if locator.starts_with("https://")
                                || locator.starts_with("http://")
                            {
                                cx.open_url(&locator);
                            }
                        }),
                );
            }
            let intent = if coverage.full_note {
                "Full imported reference"
            } else {
                "Useful information"
            };
            let (summary, details) = if let Some(report) = &coverage.report {
                (
                    report.summary(),
                    report
                        .issues
                        .iter()
                        .map(ImportIssue::display)
                        .collect::<Vec<_>>(),
                )
            } else {
                (
                    if coverage.gaps.is_empty() {
                        "Coverage not recorded"
                    } else {
                        "Partial / coverage requires inspection"
                    },
                    coverage.gaps.clone(),
                )
            };
            content = content.child(
                div()
                    .id("note-import-summary")
                    .flex_shrink_0()
                    .text_sm()
                    .child(format!("{intent} · {summary}"))
                    .test_support(),
            );
            if !details.is_empty() {
                // Provenance may contain long paths and many qualifications.
                // Keep every issue scrollable without consuming the note body.
                content = content.child(
                    div()
                        .id("note-coverage-details")
                        .w_full()
                        .max_h(px(120.))
                        .flex_shrink_0()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .border_1()
                        .border_color(rgb(0xdce2e8))
                        .rounded_md()
                        .p_2()
                        .child(
                            div()
                                .text_sm()
                                .child(format!("Coverage details ({})", details.len())),
                        )
                        .children(details.into_iter().enumerate().map(|(index, detail)| {
                            div()
                                .id(format!("note-coverage-detail-{index}"))
                                .flex_shrink_0()
                                .text_sm()
                                .child(detail)
                                .test_support()
                        }))
                        .test_support(),
                );
            }
        }
        if let Some(session) = &open.session {
            content=content.child("Unsaved edit · recoverable from Home. Changes to this note wait until this edit is finished.");
            content = content.child(
                Button::new("save-buffer-copy")
                    .label("Save text as a new note")
                    .on_click(cx.listener(|this, _, window, cx| this.save_copy(window, cx))),
            );
            if self
                .records
                .iter()
                .any(|r| r.id == session.note && r.version != session.base_version)
            {
                content=content.child(panel().child("The current note changed since this edit began. Your text is preserved.").child(div().h(px(180.)).child(markdown(&self.current_view,self.assets.clone()))));
            }
        }
        let text = self.note_editor.read(cx).value().to_string();
        let mut nav = div().flex().flex_wrap().gap_2();
        for (i, (title, range)) in sections(&text).into_iter().enumerate() {
            nav = nav.child(
                Button::new(format!("section-{i}"))
                    .label(short(
                        title
                            .strip_prefix("**")
                            .and_then(|value| value.strip_suffix("**"))
                            .unwrap_or(&title),
                    ))
                    .compact()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.reveal(range.clone(), window, cx)
                    })),
            );
        }
        content = content.child(nav);
        let editor = if open.editing {
            Editor::new(&self.note_editor)
                .h_full()
                .aria_label("Note Markdown editor")
                .into_any_element()
        } else {
            markdown(&self.note_view, self.assets.clone()).into_any_element()
        };
        content = content.child(
            div()
                .id("note-body")
                .flex_1()
                .min_h_0()
                .w_full()
                .child(editor)
                .test_support(),
        );
        if let Some((_, _, _, quote)) = &self.comment_anchor {
            content = content.child(
                panel()
                    .child("Comment on this passage")
                    .child(short(quote))
                    .child(
                        div()
                            .h(px(90.))
                            .child(Editor::new(&self.comment_input).h_full()),
                    )
                    .child(
                        Button::new("post-comment")
                            .label("Start discussion")
                            .on_click(
                                cx.listener(|this, _, window, cx| this.post_comment(window, cx)),
                            ),
                    ),
            );
        }
        let mut comments = div()
            .id("note-comments")
            .max_h(px(170.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2();
        for record in self.records.iter().filter(|r| !r.archived) {
            let RecordData::Comment(comment) = &record.data else {
                continue;
            };
            if comment.note != open.record.id {
                continue;
            }
            let location = comment_location(comment, open.record.version, &note.markdown);
            let mut row = div()
                .flex()
                .flex_col()
                .gap_1()
                .child(short(&comment.body))
                .child(format!(
                    "Passage comment · {}",
                    if location.is_some() {
                        "Current passage"
                    } else {
                        "Location unresolved; original quote preserved"
                    }
                ))
                .child(short(&comment.quote));
            if let Some(range) = location {
                row = row.child(
                    Button::new(format!("reveal-comment-{}", record.id))
                        .label("Show passage")
                        .disabled(open.session.is_some())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.reveal(range.clone(), window, cx)
                        })),
                );
            }
            for thread_id in linked(&self.records, &record.id) {
                if self
                    .records
                    .iter()
                    .any(|r| r.id == thread_id && matches!(r.data, RecordData::Thread(_)))
                {
                    row = row.child(
                        Button::new(format!("open-comment-thread-{thread_id}"))
                            .label("Open discussion")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.navigate(Page::Thread(thread_id.clone()), window, cx)
                            })),
                    );
                }
            }
            comments = comments.child(row);
        }
        content = content.child(comments);
        let linked_ids = linked(&self.records, &open.record.id);
        let mut discussion = div().flex().flex_wrap().gap_2();
        for record in self
            .records
            .iter()
            .filter(|r| linked_ids.contains(&r.id) && matches!(r.data, RecordData::Thread(_)))
        {
            let id = record.id.clone();
            discussion = discussion.child(
                Button::new(format!("note-thread-{id}"))
                    .label(format!("Discuss: {}", short(&title(record))))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.navigate(Page::Thread(id.clone()), window, cx)
                    })),
            );
        }
        content.child(discussion).into_any_element()
    }
    fn review_view(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(review) = &self.review else {
            return div().into_any_element();
        };
        let mut summary = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(heading("Review change"))
            .child(review.candidate.request.reason.clone());
        if review.groups.groups.len() > 1 {
            summary = summary.child("Choose complete notes and Actions. Prepare your selection to review a new immutable proposal.");
            for group in &review.groups.groups {
                let id = group.id.clone();
                let included = review.selected.contains(&id);
                summary = summary.child(
                    Button::new(format!("review-group-{id}"))
                        .label(format!(
                            "{} {}: {}",
                            if included { "Include" } else { "Omit" },
                            group.kind,
                            short(&group.title)
                        ))
                        .selected(included)
                        .disabled(review.edited || !review.groups.ungrouped_writes.is_empty())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(review) = &mut this.review
                                && !review.selected.remove(&id)
                            {
                                review.selected.insert(id.clone());
                            }
                            cx.notify();
                        })),
                );
                if !group.required_groups.is_empty() {
                    let required = group
                        .required_groups
                        .iter()
                        .filter_map(|id| review.groups.groups.iter().find(|g| &g.id == id))
                        .map(|g| g.title.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    summary = summary.child(format!("Requires: {required}"));
                }
            }
            if !review.groups.ungrouped_writes.is_empty() {
                summary = summary.child("This proposal has shared changes that cannot be separated safely. Keep the complete proposal or request a rewrite.");
            }
        }
        for write in &review.candidate.request.writes {
            if let RecordData::Note(note) = &write.data {
                let id = write.id.clone();
                summary = summary.child(
                    Button::new(format!("review-member-{id}"))
                        .label(format!("Compare {}", short(&note.title)))
                        .selected(review.note.as_ref() == Some(&id))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.select_review_note(id.clone(), window, cx)
                        })),
                );
            }
            let label = match &write.data {
                RecordData::Note(n) => format!(
                    "Note: {}{}",
                    n.title,
                    if n.protected { " · protected" } else { "" }
                ),
                RecordData::Action(a) => format!("Action: {} · {:?}", a.description, a.state),
                RecordData::Thread(t) => format!("Discussion: {}", t.title),
                RecordData::Message(m) => format!("Message: {}", short(&m.text)),
                RecordData::Link(_) => "Connect related work".into(),
                RecordData::Comment(c) => format!("Comment: {}", short(&c.body)),
                RecordData::Source(s) => format!("Source: {}", s.label),
                RecordData::Asset(_) => "Retain a meaningful figure".into(),
                RecordData::Run(_) => "Record work progress".into(),
                RecordData::Settings(_) => "Change workspace settings".into(),
            };
            summary = summary.child(label);
        }
        let mut view = div()
            .id("review-panel")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_5()
            .bg(rgb(0xf7f5ee))
            .child(summary);
        if review.note.is_some() {
            view = view.child(
                div()
                    .flex()
                    .gap_4()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(heading("Current note"))
                            .child(
                                div()
                                    .flex_1()
                                    .min_h_0()
                                    .child(markdown(&self.current_view, self.assets.clone())),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(heading("Your final proposed text"))
                            .child(
                                div().flex_1().min_h_0().child(
                                    Editor::new(&self.review_editor)
                                        .h_full()
                                        .aria_label("Final proposed note text"),
                                ),
                            ),
                    ),
            );
        }
        view.child(div().flex().flex_wrap().gap_2()
            .child(Button::new("prepare-final").label("Prepare edited proposal").disabled(!review.edited).on_click(cx.listener(|this,_,window,cx|this.prepare_review_edit(window,cx))))
            .child(Button::new("prepare-selected").label("Prepare selected groups").disabled(review.edited || review.selected.is_empty() || review.selected.len() == review.groups.groups.len()).on_click(cx.listener(|this,_,window,cx|this.prepare_review_selection(window,cx))))
            .child(Button::new("apply-final").label("Apply this proposal").disabled(review.edited || review.selected.len() != review.groups.groups.len()).on_click(cx.listener(|this,_,window,cx|this.apply_review(window,cx))))
            .child(Button::new("dismiss-proposal").label("Dismiss proposal").on_click(cx.listener(|this,_,window,cx|this.dismiss_review(window,cx))))
            .child(Button::new("close-review").label("Keep for later").on_click(cx.listener(|this,_,window,cx|this.keep_review(window,cx)))))
            .child("Applying saves this exact proposal. Changed or actively edited notes remain unchanged until the issue is resolved.").into_any_element()
    }
    fn history_view(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut view = div()
            .id("history-content")
            .h_full()
            .w_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .p_5()
            .child(heading("What changed"));
        for receipt in self.history.iter().rev() {
            let id = receipt.operation.as_str().to_owned();
            let mut entry = panel()
                .child(heading(receipt.reason.clone()))
                .child(receipt.committed_at.clone());
            for write in &receipt.writes {
                entry = entry.child(format!(
                    "{} · version {}",
                    title(&write.after),
                    write.after.version
                ));
            }
            if !receipt.needs_refresh.is_empty() {
                entry = entry.child("Directly derived work may need refreshing after this change. Review its current inputs before updating it.");
                if let Ok(records) = self.workspace.refresh_records(&id) {
                    for record in records {
                        let page = if matches!(record.data, RecordData::Note(_)) {
                            Page::Note(record.id.clone())
                        } else {
                            Page::Actions
                        };
                        entry = entry.child(
                            Button::new(format!("refresh-record-{id}-{}", record.id))
                                .label(format!("May need refresh: {}", short(&title(&record))))
                                .on_click(cx.listener(move |this, _, window, cx| match &page {
                                    Page::Note(id) => this.open_note(id, None, window, cx),
                                    _ => this.navigate(page.clone(), window, cx),
                                })),
                        );
                    }
                }
                let receipt_id = id.clone();
                entry = entry.child(
                    Button::new(format!("refresh-followup-{id}"))
                        .label("Open refresh follow-up")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            match this.workspace.refresh_followup(&receipt_id) {
                                Ok(thread) => {
                                    this.refresh();
                                    this.navigate(Page::Thread(thread.id), window, cx);
                                }
                                Err(error) => {
                                    this.notice = error.to_string();
                                    cx.notify();
                                }
                            }
                        })),
                );
            }
            entry = entry.child(
                Button::new(format!("undo-{id}"))
                    .label("Undo this change")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        match this.workspace.undo(&id) {
                            Ok(outcome) => this.notice = outcome_message(&outcome),
                            Err(e) => this.notice = e.to_string(),
                        };
                        this.refresh();
                        cx.notify();
                    })),
            );
            view = view.child(entry);
        }
        view.into_any_element()
    }
    fn settings_view(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut provider = div().flex().gap_2();
        for (value, label) in [
            ("chatgpt", "Codex / ChatGPT"),
            ("copilot", "GitHub Copilot"),
        ] {
            provider = provider.child(
                Button::new(format!("provider-{value}"))
                    .label(label)
                    .selected(
                        self.settings.provider.eq_ignore_ascii_case(value)
                            || value == "Codex"
                                && self.settings.provider.eq_ignore_ascii_case("chatgpt"),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings.provider = value.into();
                        cx.notify();
                    })),
            );
        }
        let mut efforts = div().flex().gap_2();
        for effort in ["low", "medium", "high"] {
            efforts = efforts.child(
                Button::new(format!("effort-{effort}"))
                    .label(effort)
                    .selected(self.settings.effort == effort)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings.effort = effort.into();
                        cx.notify();
                    })),
            );
        }
        let mut view = div()
            .id("settings-content")
            .h_full()
            .w_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_4()
            .p_5()
            .child(heading("Workspace settings"))
            .child(
                panel()
                    .child(heading("Provider and model"))
                    .child(provider)
                    .child(Input::new(&self.model))
                    .child("Reasoning effort")
                    .child(efforts)
                    .child(Input::new(&self.auth_path))
                    .child(Input::new(&self.credentials_path))
                    .child("Credentials stay outside notes, exports and History.")
                    .child(
                        Button::new("save-settings")
                            .label("Save settings")
                            .disabled(self.worker.is_some())
                            .on_click(cx.listener(|this, _, _, cx| this.configure(cx))),
                    ),
            );
        view = view.child(self.working_knowledge(cx));
        view.child(panel().child(heading("Portable copies")).child("Markdown export is a current snapshot. A full backup includes notes, discussions, history, figures and recovery state.")
            .child(Button::new("export-now").label("Export Markdown now…").on_click(cx.listener(|this,_,window,cx|this.choose_output(false,window,cx))))
            .child(self.export_status.clone())
            .child(div().id("backup-directory").aria_label(self.settings.backup_directory.clone().unwrap_or_else(|| "Backup folder not chosen".into())).child(self.settings.backup_directory.clone().unwrap_or_else(|| "Backup folder not chosen".into())).test_support())
            .child("Choose a folder covered by your separate backup system.")
            .child(Button::new("backup-change-folder").label("Change folder…").on_click(cx.listener(|this,_,window,cx|this.choose_backup_folder(false,window,cx))))
            .child(Button::new("backup-now").label("Create full backup").on_click(cx.listener(|this,_,window,cx|this.choose_output(true,window,cx))))
            .child(self.backup_status.clone())).into_any_element()
    }
}
impl Render for ThreadsUi {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _retained = (&self.subscriptions, &self.poll, &self.importing);
        let view = if self.review.is_some() {
            self.review_view(cx)
        } else {
            match self.page.clone() {
                Page::Home => self.home_view(cx),
                Page::Thread(id) => self.thread_view(&id, cx),
                Page::Note(_) => self.note_view(cx),
                Page::History => self.history_view(cx),
                Page::Settings => self.settings_view(cx),
                Page::Actions => {
                    let mut view = div()
                        .id("actions-content")
                        .h_full()
                        .w_full()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .p_5()
                        .child(heading("Actions"));
                    for record in self
                        .records
                        .iter()
                        .filter(|r| !r.archived && matches!(r.data, RecordData::Action(_)))
                    {
                        view = view.child(self.action_row(record, cx));
                    }
                    view.into_any_element()
                }
            }
        };
        let mut footer = div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .border_t_1()
            .border_color(rgb(0xd5dce1));
        if !self.notice.is_empty() {
            footer = footer.child(div().text_sm().child(self.notice.clone()));
        }
        if let Some(worker) = &self.worker {
            let status = worker.kind.status(&self.progress);
            let progress = worker.thread.as_ref().map_or_else(
                || status.clone(),
                |thread| {
                    let name = self
                        .records
                        .iter()
                        .find(|record| &record.id == thread)
                        .map(title)
                        .unwrap_or_else(|| thread.clone());
                    format!("Work in {} · {}", short(&name), status)
                },
            );
            let mut controls = div().flex().gap_3().items_center().child(
                div()
                    .flex_1()
                    .min_w_0()
                    .id("worker-status")
                    .aria_label(progress.clone())
                    .child(progress)
                    .test_support(),
            );
            if worker.kind.can_cancel() {
                controls = controls.child(
                    Button::new("stop-work")
                        .label("Stop")
                        .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                );
            }
            footer = footer.child(controls);
        }
        let destination = self.composer_destination_label();
        footer = footer.child(
            div()
                .id("composer-destination")
                .text_sm()
                .aria_label(destination.clone())
                .child(destination)
                .test_support(),
        );
        footer = footer.child(
            div()
                .flex()
                .gap_3()
                .items_center()
                .child(
                    div().h(px(90.)).flex_1().min_w_0().child(
                        Editor::new(&self.composer)
                            .h_full()
                            .aria_label("Ask or delegate"),
                    ),
                )
                .child(
                    Button::new("ask-delegate")
                        .label(if self.can_stop_and_send() {
                            "Stop and send"
                        } else {
                            "Ask or delegate"
                        })
                        .disabled(self.worker.is_some() && !self.can_stop_and_send())
                        .on_click(cx.listener(|this, _, window, cx| this.ask(window, cx))),
                ),
        );
        if let Page::Note(id) = &self.page
            && let Some(record) = self
                .records
                .iter()
                .find(|r| r.id == *id && matches!(r.data, RecordData::Note(_)))
        {
            let authority = brn_ai::threads::OwnerRequest::EditNote {
                id: record.id.clone(),
                version: record.version,
            };
            footer = footer.child(
                Button::new("ask-edit-selected-note")
                    .label("Edit selected note · this request")
                    .disabled(self.worker.is_some())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.ask_with_authority(Some(authority.clone()), window, cx)
                    })),
            );
        }
        footer = footer.child(
            Button::new("ask-record-decision")
                .label("Record decision · this request")
                .disabled(self.worker.is_some())
                .on_click(cx.listener(|this, _, window, cx| {
                    this.ask_with_authority(
                        Some(brn_ai::threads::OwnerRequest::RecordDecision),
                        window,
                        cx,
                    )
                })),
        );
        let _ = window;
        div()
            .h_full()
            .w_full()
            .flex()
            .bg(rgb(0xffffff))
            .text_color(rgb(0x25313b))
            .on_action(cx.listener(|this, _: &Save, window, cx| {
                if this.review.is_some() {
                    this.prepare_review_edit(window, cx);
                } else if matches!(this.page, Page::Note(_)) {
                    this.save(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Ask, window, cx| this.ask(window, cx)))
            .on_action(
                cx.listener(|this, _: &Home, window, cx| this.navigate(Page::Home, window, cx)),
            )
            .on_action(cx.listener(|this, _: &NewNote, window, cx| this.new_note(window, cx)))
            .on_action(cx.listener(|this, _: &FocusAsk, window, cx| {
                this.composer.update(cx, |e, cx| e.focus(window, cx))
            }))
            .on_action(cx.listener(|this, _: &Cancel, _, cx| this.cancel(cx)))
            .child(self.sidebar(cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(div().flex_1().min_h_0().child(view))
                    .child(footer),
            )
    }
}

pub fn launch(data_dir: PathBuf) -> Result<(), String> {
    let workspace = Workspace::open(&data_dir).map_err(|e| e.to_string())?;
    workspace.settings().map_err(|e| e.to_string())?;
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.bind_keys([
                KeyBinding::new("cmd-s", Save, None),
                KeyBinding::new("cmd-enter", Ask, None),
                KeyBinding::new("cmd-l", FocusAsk, None),
                KeyBinding::new("cmd-n", NewNote, None),
                KeyBinding::new("cmd-0", Home, None),
                KeyBinding::new("cmd-.", Cancel, None),
                KeyBinding::new("cmd-q", Quit, None),
            ]);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(70.), px(70.)),
                        size(px(1260.), px(860.)),
                    ))),
                    window_min_size: Some(size(px(850.), px(600.))),
                    ..TitleBar::window_options()
                },
                move |window, cx| {
                    let view = cx.new(|cx| ThreadsUi::new(workspace, data_dir, window, cx));
                    let weak = view.downgrade();
                    window.on_window_should_close(cx, move |_, cx| {
                        let _ = weak.update(cx, |this, _| {
                            this.pending_send = None;
                            if let Some(worker) = &this.worker {
                                worker.cancel.cancel();
                                worker
                                    .conversion_cancel
                                    .store(true, std::sync::atomic::Ordering::SeqCst);
                            }
                        });
                        true
                    });
                    let quit_view = view.downgrade();
                    cx.on_action::<Quit>(move |_, cx| {
                        let _ = quit_view.update(cx, |this, _| {
                            this.pending_send = None;
                            if let Some(worker) = &this.worker {
                                worker.cancel.cancel();
                                worker
                                    .conversion_cancel
                                    .store(true, std::sync::atomic::Ordering::SeqCst);
                            }
                        });
                        cx.quit();
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("failed to open BRN Threads window");
            cx.activate(true);
        });
    Ok(())
}
