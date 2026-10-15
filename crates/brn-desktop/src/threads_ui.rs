//! Final-use native Threads workspace. All durable changes use the shared service.
use brn_threads_app::*;
use brn_threads_intake::{ConversionResult, Converter, DocumentKind, ImportIntent, PdfTools};
use gpui_kit::base::input::{
    RangeDecoration, RangeDecorationCollection, RangeDecorationStyle, TextDecoration,
    TextDecorationCollection,
};
use gpui_kit::base::{Disableable, Selectable};
use gpui_kit::component::WindowExt;
use gpui_kit::{
    App, AppContext, Bounds, Context, Entity, FocusHandle, HighlightStyle, Image, ImageFormat,
    KeyBinding, PathPromptOptions, Subscription, Task, TestSupportExt, Window, WindowBounds,
    WindowOptions,
    base::{
        MarkdownExtensions, MarkdownNode, MarkdownParseContext, MarkdownPlugin, TextView,
        TextViewState, markdown_ast::Node,
    },
    component::{
        Root, TitleBar,
        button::{Button, ButtonRounded},
        input::{Editor, EditorState, Input, InputEvent, InputState},
    },
    div, point,
    prelude::*,
    px, rgb, size,
};
use std::result::Result;
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    path::PathBuf,
    sync::{
        Arc,
        atomic::AtomicBool,
        mpsc::{self, Receiver},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
#[path = "threads_chat.rs"]
mod chat;
#[path = "threads_connections.rs"]
mod connections_view;
#[path = "threads_context_actions.rs"]
mod context_actions;
#[path = "threads_diff.rs"]
mod diff;
#[path = "threads_document.rs"]
mod document;
#[path = "threads_format.rs"]
mod formatting;
#[path = "threads_layout.rs"]
mod layout;
#[path = "threads_lists.rs"]
mod lists;
#[path = "threads_projects.rs"]
mod projects_view;
#[path = "threads_search.rs"]
mod search_view;
#[path = "threads_shell.rs"]
mod shell;
#[path = "threads_state.rs"]
mod state;
#[path = "threads_theme.rs"]
mod theme;
#[path = "threads_tiles.rs"]
mod tiles;
#[path = "threads_workbench.rs"]
mod workbench;
#[path = "threads_writing.rs"]
mod writing;
use layout::{Layout, Preset, TileKind};
use lists::{Activity, NoteRow, ThreadFilter, ThreadRow};
use state::{
    ComposerDestination, ComposerDrafts, WorkerKind, can_continue_thread, comment_location,
    continuation_notice, delegation_status, latest_run, linked, outcome_message, save_message,
    sections,
};
use theme::{ThemeId, pal};

#[cfg(all(test, feature = "native-test-support"))]
#[path = "threads_context_actions_tests.rs"]
mod context_actions_tests;
#[cfg(all(test, feature = "native-test-support"))]
#[path = "threads_projects_tests.rs"]
mod projects_tests;

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
#[path = "threads_writing_tests.rs"]
mod writing_tests;

#[cfg(all(test, feature = "native-test-support"))]
#[path = "threads_workbench_tests.rs"]
mod workbench_tests;

#[cfg(all(test, feature = "native-test-support"))]
#[path = "threads_document_tests.rs"]
mod document_tests;

#[cfg(all(test, feature = "native-test-support"))]
#[path = "threads_tiled_tests.rs"]
mod tiled_tests;

#[cfg(all(test, feature = "native-test-support"))]
#[path = "threads_omakase_tests.rs"]
mod omakase_tests;

#[cfg(all(test, feature = "native-test-support"))]
#[path = "threads_intake_tests.rs"]
mod intake_tests;

gpui_kit::actions!(
    brn_threads,
    [
        Save,
        Ask,
        Home,
        NewNote,
        NewThread,
        FocusAsk,
        Cancel,
        Quit,
        OpenLauncher,
        CloseLauncher,
        LauncherNext,
        LauncherPrevious,
        CommentSelection,
        AskAboutSelection,
        Rewrite,
        ToggleThreads,
        FocusPreviousTile,
        FocusNextTile,
        MaximizeTile,
        OpenSettings,
        LayoutWrite,
        LayoutChat,
        LayoutSolo,
        ShowKeys
    ]
);

/// What the chat tile's composer sends to.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ChatTarget {
    /// A standalone new thread, created on send.
    New,
    /// A new thread linked to this note, created on send.
    NewForNote(String),
    Thread(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsSection {
    General,
    Models,
    Delegation,
    Data,
}
impl SettingsSection {
    const ALL: [Self; 4] = [Self::General, Self::Models, Self::Delegation, Self::Data];
    fn key(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Models => "models",
            Self::Delegation => "delegation",
            Self::Data => "data",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Models => "Models",
            Self::Delegation => "Delegation",
            Self::Data => "Data",
        }
    }
}

#[derive(Clone)]
struct CommentDraft {
    anchor: Option<(String, u64, Range<usize>, String)>,
    body: String,
    operation: Option<String>,
}

struct OpenNote {
    record: Record,
    session: Option<EditSession>,
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
    Activity(String),
    Text(String),
    Finished(Result<String, String>),
    RunFinished {
        result: Result<String, String>,
        terminal: Option<(String, u64)>,
    },
    CopilotLogin {
        verification_uri: String,
        user_code: String,
    },
    CopilotConnected(String),
    CopilotModels(Vec<String>),
    Converted {
        result: Box<Result<ConversionResult, String>>,
        title: String,
        chat: Option<String>,
    },
}
struct Worker {
    receiver: Receiver<WorkerEvent>,
    cancel: CancellationToken,
    conversion_cancel: Arc<AtomicBool>,
    kind: WorkerKind,
    /// The native stream and cancellation belong to this invocation's thread,
    /// independently of whichever thread the owner is currently viewing.
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
/// Only absolute lowercase http(s) URLs may open externally; every other
/// scheme stays inert. Unusual casing such as `HTTPS://` fails closed rather
/// than opening an unexpected URL.
fn external_link(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}
/// Images resolve only from the caller-visible stored asset map; any other
/// reference renders an empty placeholder instead of authorizing a read.
fn stored_image(assets: &BTreeMap<String, Arc<Image>>, uri: &str) -> Arc<Image> {
    assets
        .get(uri)
        .cloned()
        .unwrap_or_else(|| Arc::new(Image::from_bytes(ImageFormat::Png, Vec::new())))
}
/// The one Markdown policy shared by every rendered note and message: HTML is
/// shown literally, images resolve only through `stored_image` for the
/// caller's own asset map, and `external_link` gates which links may open.
/// Callers keep their own fonts, scrolling and caller-scoped asset map.
fn safe_markdown(view: TextView, assets: Arc<BTreeMap<String, Arc<Image>>>) -> TextView {
    view.markdown_extensions(
        MarkdownExtensions::default()
            .plugin(LiteralHtml { block: true })
            .plugin(LiteralHtml { block: false })
            .parser_revision(1),
    )
    .image_source(move |uri| stored_image(&assets, uri.as_ref()).into())
    .on_link_click(|url, _, _, cx| {
        if external_link(url) {
            cx.open_url(url);
        }
        cx.stop_propagation();
    })
}
fn markdown(view: &Entity<TextViewState>, assets: Arc<BTreeMap<String, Arc<Image>>>) -> TextView {
    safe_markdown(
        TextView::new(view)
            .font_family(UI_FONT)
            .text_size(PROSE_TEXT)
            .w_full()
            .h_full()
            .scrollable(true),
        assets,
    )
}

// Presentation is shared by every tile. Toolkit controls keep their focus,
// selected and disabled behavior; prose retains the normal system font.
const MONO: &str = "Menlo";
const UI_FONT: &str = ".SystemUIFont";
const SMALL_TEXT: gpui_kit::Pixels = px(12.);
const UI_TEXT: gpui_kit::Pixels = px(14.);
const PROSE_TEXT: gpui_kit::Pixels = px(16.);
const TITLE_TEXT: gpui_kit::Pixels = px(18.);
fn default_copilot_credentials_dir() -> std::io::Result<PathBuf> {
    use std::os::unix::fs::DirBuilderExt as _;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .ok_or_else(|| std::io::Error::other("HOME is not set"))?;
    let parent = home.join(".config").join("brn");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&parent)?;
    Ok(parent.join("copilot"))
}
const TOP_BAR_HEIGHT: gpui_kit::Pixels = px(38.);
fn top_bar_window_options() -> WindowOptions {
    let mut options = TitleBar::window_options();
    if let Some(titlebar) = options.titlebar.as_mut() {
        // Centre the 16px macOS traffic lights inside the 38px top bar.
        titlebar.traffic_light_position = Some(point(px(12.), (TOP_BAR_HEIGHT - px(16.)) / 2.));
    }
    options
}
fn button(id: impl Into<gpui_kit::ElementId>) -> Button {
    Button::new(id)
        .rounded(ButtonRounded::Small)
        .font_family(UI_FONT)
        .text_size(UI_TEXT)
}
fn prose_editor(state: &Entity<EditorState>) -> Editor {
    Editor::new(state)
        .font_family(UI_FONT)
        .text_size(PROSE_TEXT)
}
fn panel() -> gpui_kit::Div {
    let p = pal();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .min_w_0()
        .border_1()
        .border_color(rgb(p.border))
        .bg(rgb(p.tile))
}
fn heading(text: impl Into<gpui_kit::SharedString>) -> gpui_kit::Div {
    div()
        .font_family(UI_FONT)
        .text_size(TITLE_TEXT)
        .text_color(rgb(pal().ink))
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .child(text.into())
}
fn section(text: impl Into<gpui_kit::SharedString>) -> gpui_kit::Div {
    div()
        .font_family(UI_FONT)
        .text_size(SMALL_TEXT)
        .text_color(rgb(pal().muted))
        .pt_2()
        .child(text.into())
}
fn metadata(text: impl Into<gpui_kit::SharedString>) -> gpui_kit::Div {
    div()
        .text_size(SMALL_TEXT)
        .text_color(rgb(pal().muted))
        .child(text.into())
}
fn empty(
    id: impl Into<gpui_kit::ElementId>,
    text: impl Into<gpui_kit::SharedString>,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let text = text.into();
    metadata(text.clone())
        .id(id)
        .aria_label(text)
        .test_support()
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
        RecordData::ChatControl(_) => "Chat controls".into(),
        RecordData::Project(p) => p.title.clone(),
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
    root_focus: FocusHandle,
    layout: Layout,
    tile_scroll: gpui_kit::ScrollHandle,
    tile_reveal: Option<(Vec<TileKind>, Option<TileKind>)>,
    chat: ChatTarget,
    theme: ThemeId,
    theme_pending: bool,
    settings_section: SettingsSection,
    records: Vec<Record>,
    history: Vec<Receipt>,
    candidates: Vec<Prepared>,
    buffers: Vec<EditSession>,
    activity: Activity,
    thread_rows: Vec<ThreadRow>,
    note_rows: Vec<NoteRow>,
    composer: Entity<EditorState>,
    composer_drafts: ComposerDrafts,
    draft_generations: BTreeMap<String, u64>,
    scrollers: std::cell::RefCell<BTreeMap<String, chat::ChatScroller>>,
    attaching_chat: Option<String>,
    selected_project: Option<String>,
    note_project: Option<String>,
    projects_trash: bool,
    editing_queue: Option<String>,
    note_editor: Entity<EditorState>,
    note_read_view: Entity<TextViewState>,
    note_reading: bool,
    read_marks_key: String,
    read_source: String,
    review_reading: bool,
    comment_marks: Option<RangeDecorationCollection>,
    diff_editor: Entity<EditorState>,
    diff_marks: Option<TextDecorationCollection>,
    current_view: Entity<TextViewState>,
    candidate_view: Entity<TextViewState>,
    review_editor: Entity<EditorState>,
    review_editing: bool,
    review_inputs: bool,
    search: Entity<InputState>,
    hits: Vec<brn_threads_app::SearchHit>,
    results: search_view::SearchView,
    topics: Option<Topics>,
    graph: connections_view::ConnectionsView,
    knowledge: connections_view::KnowledgeView,
    notes_letter: char,
    threads_query: Entity<InputState>,
    thread_filter: ThreadFilter,
    launcher_query: Entity<InputState>,
    launcher_open: bool,
    launcher_index: usize,
    model: Entity<InputState>,
    auth_path: Entity<InputState>,
    credentials_path: Entity<InputState>,
    helper_model: Entity<InputState>,
    settings: WorkspaceSettings,
    note: Option<OpenNote>,
    review: Option<Review>,
    margin: bool,
    show_coverage: bool,
    selection: Option<(Range<usize>, String)>,
    general_feedback: Entity<EditorState>,
    general_drafts: BTreeMap<String, String>,
    comment_input: Entity<EditorState>,
    comment_anchor: Option<(String, u64, Range<usize>, String)>,
    comment_operation: Option<String>,
    comment_drafts: BTreeMap<String, CommentDraft>,
    writing_threads: BTreeMap<String, String>,
    excluded_comments: BTreeSet<String>,
    expanded_requests: BTreeSet<String>,
    applied_rewrite: BTreeMap<String, (String, String)>,
    assets: Arc<BTreeMap<String, Arc<Image>>>,
    worker: Option<Worker>,
    pending_send: Option<PendingSend>,
    stream: String,
    progress: String,
    run_activity_labels: Vec<String>,
    run_activity_chat: Option<String>,
    context_edit_note: Option<(String, String, u64)>,
    run_activity_status: String,
    run_activity_scroll: gpui_kit::ScrollHandle,
    notice: String,
    last_work_notice: Option<String>,
    export_status: String,
    copilot_models: Vec<String>,
    backup_status: String,
    subscriptions: Vec<Subscription>,
    poll: Option<Task<()>>,
}
impl ThreadsUi {
    fn new(
        mut workspace: Workspace,
        data_dir: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        if workspace.chat_control().is_ok_and(|r| matches!(r.data,RecordData::ChatControl(c) if c.pending.is_some() || !c.queue.is_empty())) { let _ = workspace.recover_queue(&format!("startup:{}", uuid::Uuid::new_v4())); }
        let saved_drafts = workspace.store.chat_drafts().unwrap_or_default();
        let mut drafts = ComposerDrafts::default();
        let mut draft_generations = BTreeMap::new();
        for d in saved_drafts {
            if let Ok(destination) = serde_json::from_str(&d.destination) {
                drafts.preserve(destination, d.text);
            }
            draft_generations.insert(d.destination, d.generation);
        }
        let settings = workspace
            .settings()
            .expect("workspace settings were checked before opening the window");
        let theme = ThemeId::from_key(settings.theme.as_deref());
        theme::apply(theme, cx);
        let composer = cx.new(|cx| {
            EditorState::new(window, cx)
                .submit_on_enter(true)
                .line_number(false)
                .placeholder("Ask or delegate…")
        });
        let note_editor = cx.new(|cx| EditorState::new(window, cx).default_value(""));
        let note_read_view = cx.new(|cx| {
            TextViewState::markdown("", cx)
                .selectable(true)
                .scrollable(true)
        });
        let diff_editor = cx.new(|cx| EditorState::new(window, cx).default_value(""));
        diff_editor.update(cx, |editor, cx| editor.set_readonly(true, cx));
        let review_editor = cx.new(|cx| EditorState::new(window, cx).default_value(""));
        let comment_input = cx.new(|cx| {
            EditorState::new(window, cx)
                .line_number(false)
                .auto_close(false)
                .placeholder("What should change here?")
        });
        let general_feedback = cx.new(|cx| {
            EditorState::new(window, cx)
                .line_number(false)
                .auto_close(false)
                .placeholder("Tone, structure, or anything for the whole note…")
        });
        let current_view = cx.new(|cx| {
            TextViewState::markdown("", cx)
                .selectable(true)
                .scrollable(true)
        });
        let candidate_view = cx.new(|cx| {
            TextViewState::markdown("", cx)
                .selectable(true)
                .scrollable(true)
        });
        let search = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Filter titles and search note text")
        });
        let threads_query =
            cx.new(|cx| InputState::new(window, cx).placeholder("Filter chats as you type"));
        let launcher_query =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search notes, chats, commands…"));
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
        let helper_model = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(
                    settings
                        .helper
                        .as_ref()
                        .map_or("gpt-6-luna".into(), |helper| helper.model.clone()),
                )
                .placeholder("Explicit helper model")
        });
        let subscriptions = vec![
            cx.subscribe_in(
                &general_feedback,
                window,
                |this, editor, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        if let Some(note) = this.open_note_id().map(str::to_owned) {
                            this.general_drafts
                                .insert(note, editor.read(cx).value().to_string());
                        }
                        cx.notify();
                    }
                },
            ),
            cx.observe_in(&note_read_view, window, |this, _, _, cx| {
                if this.note_reading && this.review.is_none() {
                    this.track_selection(cx);
                    cx.notify();
                }
            }),
            cx.subscribe_in(
                &composer,
                window,
                |this, editor, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.save_composer(this.destination(), editor.read(cx).value().to_string());
                    }
                },
            ),
            cx.subscribe_in(
                &note_editor,
                window,
                |this, editor, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) && this.note_editor_visible() {
                        this.note_changed(editor.read(cx).value().to_string());
                        this.sync_read_text(cx);
                        this.track_selection(cx);
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(
                &comment_input,
                window,
                |this, _, event: &InputEvent, _, _| {
                    if matches!(event, InputEvent::Change) {
                        this.comment_operation = None;
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
            cx.subscribe_in(
                &composer,
                window,
                |this, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::PressEnter { shift: false, .. }) {
                        this.ask(window, cx);
                    }
                },
            ),
            cx.subscribe_in(
                &search,
                window,
                |this, input, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        let query = input.read(cx).value().to_string();
                        this.notice.clear();
                        this.search_changed(query, window, cx);
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(&threads_query, window, |_, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.subscribe_in(
                &launcher_query,
                window,
                |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::Change => {
                        this.launcher_index = 0;
                        cx.notify();
                    }
                    InputEvent::PressEnter { .. } => this.run_launcher(None, window, cx),
                    _ => {}
                },
            ),
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
            root_focus: cx.focus_handle(),
            layout: Layout::default(),
            tile_scroll: gpui_kit::ScrollHandle::new(),
            tile_reveal: None,
            chat: ChatTarget::New,
            theme,
            theme_pending: false,
            settings_section: SettingsSection::General,
            records: vec![],
            history: vec![],
            candidates: vec![],
            buffers: vec![],
            activity: Activity::default(),
            thread_rows: vec![],
            note_rows: vec![],
            composer,
            composer_drafts: drafts,
            draft_generations,
            scrollers: std::cell::RefCell::new(BTreeMap::new()),
            attaching_chat: None,
            selected_project: None,
            note_project: None,
            projects_trash: false,
            editing_queue: None,
            note_editor,
            note_read_view,
            note_reading: false,
            read_marks_key: String::new(),
            read_source: String::new(),
            review_reading: false,
            comment_marks: None,
            diff_editor,
            diff_marks: None,
            current_view,
            candidate_view,
            review_editor,
            review_editing: false,
            review_inputs: false,
            search,
            hits: vec![],
            results: search_view::SearchView::default(),
            topics: None,
            graph: connections_view::ConnectionsView::default(),
            knowledge: connections_view::KnowledgeView::default(),
            notes_letter: 'A',
            threads_query,
            thread_filter: ThreadFilter::All,
            launcher_query,
            launcher_open: false,
            launcher_index: 0,
            model,
            auth_path,
            credentials_path,
            helper_model,
            settings,
            note: None,
            review: None,
            margin: true,
            show_coverage: false,
            selection: None,
            general_feedback,
            general_drafts: BTreeMap::new(),
            comment_input,
            comment_anchor: None,
            comment_operation: None,
            comment_drafts: BTreeMap::new(),
            writing_threads: BTreeMap::new(),
            excluded_comments: BTreeSet::new(),
            expanded_requests: BTreeSet::new(),
            applied_rewrite: BTreeMap::new(),
            assets: Arc::new(BTreeMap::new()),
            worker: None,
            pending_send: None,
            stream: String::new(),
            progress: String::new(),
            run_activity_labels: vec![],
            run_activity_chat: None,
            context_edit_note: None,
            run_activity_status: String::new(),
            run_activity_scroll: gpui_kit::ScrollHandle::new(),
            notice: String::new(),
            last_work_notice: None,
            export_status: "No export created in this session.".into(),
            copilot_models: Vec::new(),
            backup_status: "No backup created in this session.".into(),
            subscriptions,
            poll: Some(poll),
        };
        this.refresh();
        this.prepare_search(window, cx);
        if let Some(row) = this.note_rows.first() {
            this.notes_letter = lists::letter(&row.title);
        }
        let home = this
            .composer_drafts
            .text(&ComposerDestination::Home)
            .to_owned();
        this.composer
            .update(cx, |e, cx| e.set_value(home, window, cx));
        this.root_focus.focus(window, cx);
        this
    }
    fn refresh(&mut self) {
        let result = (|| -> AppResult<()> {
            self.records = self.workspace.records()?;
            self.history = self.workspace.history()?;
            self.candidates = self.workspace.candidates()?;
            self.buffers = self.workspace.store.recovery_buffers()?;
            self.settings = self.workspace.settings()?;
            self.topics = self.workspace.topics()?;
            // Only an actual target-note Apply receipt proves addressed feedback or Undo.
            self.applied_rewrite.clear();
            let requests=self.records.iter().filter(|record| !record.archived && matches!(&record.data,RecordData::Message(message) if message.role=="writing-request")).map(|record|record.id.as_str()).collect::<BTreeSet<_>>();
            let undone = self
                .history
                .iter()
                .filter_map(|receipt| receipt.undo_of.as_ref().map(OperationId::as_str))
                .collect::<BTreeSet<_>>();
            for receipt in self.history.iter().rev().filter(|receipt| {
                !requests.is_empty()
                    && receipt.undo_of.is_none()
                    && !undone.contains(receipt.operation.as_str())
                    && receipt.writes.iter().any(|write| {
                        write.before.is_some() && matches!(write.after.data, RecordData::Note(_))
                    })
            }) {
                let Ok(candidate) = self.workspace.store.prepared(&receipt.operation) else {
                    continue;
                };
                let Some(snapshot) = candidate
                    .request
                    .inputs
                    .iter()
                    .filter(|input| requests.contains(input.record.as_str()))
                    .find_map(|input| self.workspace.writing_request(&input.record).ok())
                else {
                    continue;
                };
                if receipt.writes.iter().any(|write| {
                    write.before.as_ref() == Some(&snapshot.note)
                        && write.after.id == snapshot.note.id
                        && matches!(write.after.data, RecordData::Note(_))
                }) {
                    self.applied_rewrite
                        .entry(snapshot.note.id)
                        .or_insert((snapshot.request_message, receipt.operation.as_str().into()));
                }
            }
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
        self.activity = Activity::new(&self.history);
        self.thread_rows = lists::thread_rows(&self.records, &self.activity);
        self.note_rows = lists::note_rows(&self.records, &self.activity);
    }
    /// A recoverable edit exists that this view has not opened; typing would
    /// start a competing edit, so the text stays read-only until recovered.
    fn recovery_blocked(&self) -> bool {
        self.note.as_ref().is_some_and(|open| {
            open.session.is_none() && self.buffers.iter().any(|s| s.note == open.record.id)
        })
    }
    fn note_tile_visible(&self) -> bool {
        !self.layout.settings_open()
            && self
                .layout
                .visible()
                .iter()
                .any(|tile| tile.kind == TileKind::Note)
    }
    fn note_editor_visible(&self) -> bool {
        self.review.is_none()
            && !self.note_reading
            && self.note.is_some()
            && self.note_tile_visible()
            && !self.recovery_blocked()
    }
    fn preserve_visible_note_input(&mut self, cx: &Context<Self>) {
        if self.note_editor_visible() {
            self.note_changed(self.note_editor.read(cx).value().to_string());
        }
    }
    fn focus_root(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.root_focus.focus(window, cx);
    }
    fn focus_visible_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tile_reveal = None;
        self.focus_root(window, cx);
        if self.review_editing
            && self.note_tile_visible()
            && self
                .review
                .as_ref()
                .is_some_and(|review| review.note.is_some())
        {
            self.review_editor
                .update(cx, |editor, cx| editor.focus(window, cx));
        } else if self.note_reading && self.review.is_none() && self.note_tile_visible() {
            self.note_read_view
                .read(cx)
                .focus_handle()
                .clone()
                .focus(window, cx);
        } else if self.note_editor_visible() {
            self.note_editor
                .update(cx, |editor, cx| editor.focus(window, cx));
        }
    }
    fn focus_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tile_reveal = None;
        self.layout.open(TileKind::Chat);
        self.composer
            .update(cx, |editor, cx| editor.focus(window, cx));
        cx.notify();
    }
    fn show_last_work_notice(&mut self, cx: &mut Context<Self>) {
        // Never replace a newer error/status or show a past completion as live work.
        if self.worker.is_none()
            && self.notice.is_empty()
            && let Some(notice) = &self.last_work_notice
        {
            self.notice = notice.clone();
            cx.notify();
        }
    }
    fn show_tile(&mut self, kind: TileKind, window: &mut Window, cx: &mut Context<Self>) {
        self.preserve_visible_note_input(cx);
        self.layout.open(kind);
        self.focus_tile_contents(window, cx);
        self.refresh();
        if kind == TileKind::Connections {
            self.refresh_connections(cx);
        }
        cx.notify();
    }
    fn apply_layout(&mut self, preset: Preset, window: &mut Window, cx: &mut Context<Self>) {
        self.relayout(window, cx, |layout| layout.apply(preset));
    }
    /// Change the arrangement, then keep keyboard focus in a visible tile.
    fn relayout(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        change: impl FnOnce(&mut Layout),
    ) {
        self.preserve_visible_note_input(cx);
        change(&mut self.layout);
        self.focus_tile_contents(window, cx);
    }
    /// The first-use delegation choice renders in exactly one visible tile.
    fn delegation_host(&self) -> Option<TileKind> {
        let visible: Vec<TileKind> = self.layout.visible().iter().map(|tile| tile.kind).collect();
        if self.layout.settings_open() && self.settings_section == SettingsSection::Delegation {
            Some(TileKind::Settings)
        } else if self.settings.delegation_chosen {
            None
        } else if visible.contains(&TileKind::Overview) {
            Some(TileKind::Overview)
        } else if visible.contains(&TileKind::Chat) && !matches!(self.chat, ChatTarget::Thread(_)) {
            Some(TileKind::Chat)
        } else {
            None
        }
    }
    fn open_note_id(&self) -> Option<&str> {
        self.note.as_ref().map(|open| open.record.id.as_str())
    }
    fn thread_note(&self, thread: &str) -> Option<String> {
        self.thread_rows
            .iter()
            .find(|row| row.id == thread)
            .and_then(|row| row.note.clone())
    }
    fn chat_thread(&self) -> Option<&str> {
        match &self.chat {
            ChatTarget::Thread(thread) => Some(thread),
            _ => None,
        }
    }
    fn destination(&self) -> ComposerDestination {
        match &self.chat {
            ChatTarget::New => ComposerDestination::Home,
            ChatTarget::NewForNote(note) => ComposerDestination::Note(note.clone(), None),
            ChatTarget::Thread(thread) => ComposerDestination::Thread(thread.clone()),
        }
    }
    /// Each destination keeps its own unsent draft; contextual actions have separate inputs.
    fn set_chat(&mut self, target: ChatTarget, window: &mut Window, cx: &mut Context<Self>) {
        if self.chat == target {
            return;
        }
        let old = self.destination();
        self.save_composer(old, self.composer.read(cx).value().to_string());
        self.chat = target;
        // Choosing a thread linked to the open note makes it that note's thread.
        if let ChatTarget::Thread(thread) = &self.chat
            && let Some(note) = self.open_note_id().map(str::to_owned)
            && linked(&self.records, thread).contains(&note)
        {
            self.writing_threads.insert(note, thread.clone());
        }
        let text = self.composer_drafts.text(&self.destination()).to_owned();
        self.composer
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        cx.notify();
    }
    /// The first linked thread inherits this note's unsent draft, never another's.
    fn adopt_writing_thread(&mut self, note: &str) {
        let Some(thread) = self.writing_threads.get(note).cloned() else {
            return;
        };
        let pending = ComposerDestination::Note(note.into(), None);
        if matches!(&self.chat, ChatTarget::NewForNote(open) if open == note) {
            // The visible composer text now belongs to the created thread.
            self.chat = ChatTarget::Thread(thread);
            self.composer_drafts.posted(&pending);
            return;
        }
        // A draft saved while another thread was shown moves with it.
        let text = self.composer_drafts.text(&pending).to_owned();
        let target = ComposerDestination::Thread(thread);
        if !text.is_empty() && self.composer_drafts.text(&target).is_empty() {
            self.composer_drafts.posted(&pending);
            self.composer_drafts.preserve(target, text);
        }
    }
    fn preserve_comment_draft(&mut self, cx: &Context<Self>) {
        if let Some(open) = &self.note {
            self.general_drafts.insert(
                open.record.id.clone(),
                self.general_feedback.read(cx).value().to_string(),
            );
            self.comment_drafts.insert(
                open.record.id.clone(),
                CommentDraft {
                    anchor: self.comment_anchor.clone(),
                    body: self.comment_input.read(cx).value().to_string(),
                    operation: self.comment_operation.clone(),
                },
            );
        }
    }
    fn restore_comment_draft(&mut self, note: &str, window: &mut Window, cx: &mut Context<Self>) {
        let draft = self.comment_drafts.get(note).cloned();
        self.comment_anchor = draft.as_ref().and_then(|draft| draft.anchor.clone());
        self.comment_operation = draft.as_ref().and_then(|draft| draft.operation.clone());
        self.comment_input.update(cx, |editor, cx| {
            editor.set_value(
                draft.map(|draft| draft.body).unwrap_or_default(),
                window,
                cx,
            )
        });
    }
    fn open_note(
        &mut self,
        id: &str,
        recovery: Option<EditSession>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.note_project = None;
        if self
            .review
            .as_ref()
            .is_some_and(|review| review.note.as_deref() != Some(id))
        {
            self.keep_review(window, cx);
            if self.review.is_some() {
                return;
            }
        }
        self.preserve_visible_note_input(cx);
        self.preserve_comment_draft(cx);
        self.focus_root(window, cx);
        let recovery = recovery.or_else(|| {
            self.workspace
                .store
                .recovery_buffers()
                .ok()?
                .into_iter()
                .find(|session| session.note == id)
        });
        if !self.writing_threads.contains_key(id)
            && let Ok(threads) = self.workspace.writing_threads(id)
            && let Some(thread) = self
                .history
                .iter()
                .rev()
                .flat_map(|receipt| receipt.writes.iter().rev())
                .filter(|write| write.before.is_none())
                .find_map(|write| {
                    self.workspace
                        .writing_request(&write.after.id)
                        .ok()
                        .filter(|snapshot| snapshot.note.id == id)
                        .map(|snapshot| snapshot.thread)
                })
                .or_else(|| threads.first().map(|record| record.id.clone()))
        {
            self.writing_threads.insert(id.into(), thread);
        }
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
                    session: recovery,
                });
                self.selection = None;
                self.note_editor
                    .update(cx, |editor, cx| editor.set_value(text.clone(), window, cx));
                self.sync_read_text(cx);
                self.current_view
                    .update(cx, |view, cx| view.set_text(&note.markdown, cx));
                self.restore_comment_draft(id, window, cx);
                let feedback = self.general_drafts.get(id).cloned().unwrap_or_default();
                self.general_feedback
                    .update(cx, |editor, cx| editor.set_value(feedback, window, cx));
                self.layout.open(TileKind::Note);
                self.refresh();
                // Remember this note's chat without replacing the owner's selected conversation.
                if let Some(chat) = &note.chat
                    && self
                        .records
                        .iter()
                        .any(|r| r.id == *chat && matches!(r.data, RecordData::Thread(_)))
                {
                    self.writing_threads.insert(id.into(), chat.clone());
                }
                let target = self
                    .writing_threads
                    .get(id)
                    .cloned()
                    .map_or_else(|| ChatTarget::NewForNote(id.into()), ChatTarget::Thread);
                if matches!(self.chat, ChatTarget::New)
                    && self.composer.read(cx).value().trim().is_empty()
                {
                    self.set_chat(target, window, cx);
                }
                self.sync_note_marks(cx);
                self.focus_visible_editor(window, cx);
                self.refresh_connections(cx);
                cx.notify();
            }
            Ok(None) => self.notice = "That note is no longer current.".into(),
            Err(e) => self.notice = e.to_string(),
        }
    }
    /// Show a thread in the chat tile; a thread linked to a note also brings
    /// that note into the note tile when one is open.
    fn open_thread(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(note) = self.thread_note(id)
            && self.layout.contains(TileKind::Note)
            && self.open_note_id() != Some(note.as_str())
        {
            self.writing_threads.insert(note.clone(), id.into());
            self.open_note(&note, None, window, cx);
        }
        self.set_chat(ChatTarget::Thread(id.into()), window, cx);
        self.layout.open(TileKind::Chat);
        self.refresh();
        self.focus_tile_contents(window, cx);
        cx.notify();
    }
    fn new_thread(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_chat(ChatTarget::New, window, cx);
        self.focus_composer(window, cx);
    }
    /// Persistent comment highlights on the exact saved text. With a recovered
    /// edit the text differs, so nothing is guessed.
    fn sync_note_marks(&mut self, cx: &mut Context<Self>) {
        let p = pal();
        let mut ranges = Vec::new();
        if let Some(open) = &self.note
            && open.session.is_none()
            && let RecordData::Note(note) = &open.record.data
        {
            for entry in self
                .workspace
                .writing_comments(&open.record.id)
                .unwrap_or_default()
                .into_iter()
                .filter(|entry| !entry.addressed)
            {
                if let RecordData::Comment(comment) = &entry.record.data
                    && let Some(range) =
                        comment_location(comment, open.record.version, &note.markdown)
                {
                    ranges.push(
                        RangeDecoration::new(range)
                            .with_style(RangeDecorationStyle::Fill)
                            .with_color(rgb(p.comment).into()),
                    );
                }
            }
            if let Some((anchor, version, range, _)) = &self.comment_anchor
                && anchor == &open.record.id
                && *version == open.record.version
            {
                ranges.push(
                    RangeDecoration::new(range.clone())
                        .with_style(RangeDecorationStyle::Fill)
                        .with_color(rgb(p.comment_strong).into()),
                );
            }
        }
        match &self.comment_marks {
            Some(marks) => marks.set(ranges, cx),
            None => {
                self.comment_marks = Some(self.note_editor.update(cx, |editor, cx| {
                    editor.create_range_decorations_collection(ranges, cx)
                }))
            }
        }
    }
    /// The read-only tracked-change view of the exact candidate under review.
    fn sync_diff(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let p = pal();
        let (text, decorations) = match &self.review {
            Some(review) if review.note.is_some() => {
                let current = review
                    .note
                    .as_ref()
                    .and_then(|id| self.workspace.store.note(id).ok().flatten())
                    .and_then(|record| match record.data {
                        RecordData::Note(note) => Some(note.markdown),
                        _ => None,
                    })
                    .unwrap_or_default();
                let tracked = diff::tracked(&current, &review.original);
                let decorations = tracked
                    .marks
                    .iter()
                    .map(|(range, mark)| {
                        let style = match mark {
                            diff::Mark::Deleted => HighlightStyle {
                                background_color: Some(rgb(p.delete).into()),
                                strikethrough: Some(gpui_kit::StrikethroughStyle {
                                    thickness: px(1.),
                                    color: Some(rgb(p.ink).into()),
                                }),
                                ..Default::default()
                            },
                            diff::Mark::Inserted => HighlightStyle {
                                background_color: Some(rgb(p.add).into()),
                                underline: Some(gpui_kit::UnderlineStyle {
                                    thickness: px(1.),
                                    color: Some(rgb(p.ink).into()),
                                    wavy: false,
                                }),
                                ..Default::default()
                            },
                        };
                        TextDecoration::new(range.clone(), style)
                    })
                    .collect::<Vec<_>>();
                (tracked.text, decorations)
            }
            _ => (String::new(), vec![]),
        };
        self.diff_editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        match &self.diff_marks {
            Some(marks) => marks.set(decorations, cx),
            None => {
                self.diff_marks = Some(self.diff_editor.update(cx, |editor, cx| {
                    editor.create_decorations_collection(decorations, cx)
                }))
            }
        }
    }
    /// Track the current text selection for the selection strip.
    fn track_selection(&mut self, cx: &mut Context<Self>) {
        let editor = self.note_editor.read(cx);
        let range = if self.note_reading {
            let view = self.note_read_view.read(cx);
            if view.rendered_text().source() != editor.value().as_ref() {
                None
            } else {
                view.selected_source_range()
            }
        } else {
            Some(editor.selected_range())
        };
        let selection = range
            .filter(|range| range.start < range.end)
            .and_then(|range| {
                editor
                    .value()
                    .get(range.clone())
                    .map(|quote| (range, quote.to_owned()))
            });
        if selection != self.selection {
            self.selection = selection;
            cx.notify();
        }
    }
    /// Comments anchor to the saved version, so an unsaved edit disables them.
    fn comment_on_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = &self.note else { return };
        if open.session.is_some() {
            self.notice =
                "Save your edit first: comments anchor to the saved version of the note.".into();
            cx.notify();
            return;
        }
        let RecordData::Note(note) = &open.record.data else {
            return;
        };
        let range = if self.note_reading {
            self.selection
                .as_ref()
                .map(|(range, _)| range.clone())
                .unwrap_or(0..0)
        } else {
            self.note_editor.read(cx).selected_range()
        };
        if self.note_editor.read(cx).value().as_ref() == note.markdown
            && range.start < range.end
            && let Some(quote) = note.markdown.get(range.clone())
        {
            let anchor = (
                open.record.id.clone(),
                open.record.version,
                range,
                quote.into(),
            );
            if self.comment_anchor.as_ref() != Some(&anchor) {
                self.comment_operation = None;
                self.comment_anchor = Some(anchor);
            }
            self.margin = true;
            self.sync_note_marks(cx);
            self.comment_input.update(cx, |e, cx| e.focus(window, cx));
        } else {
            self.notice =
                "Select a passage to comment on. The selection must map to the saved note.".into();
        }
        cx.notify();
    }
    /// Quote the selection into the thread composer without sending anything.
    fn ask_about_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, quote)) = self.selection.clone() else {
            return;
        };
        let quoted = quote
            .lines()
            .map(|line| format!("> {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let existing = self.composer.read(cx).value().to_string();
        let text = if existing.trim().is_empty() {
            format!("{quoted}\n\n")
        } else {
            format!("{existing}\n\n{quoted}\n\n")
        };
        self.composer
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        self.focus_composer(window, cx);
    }
    fn reveal(&mut self, range: Range<usize>, window: &mut Window, cx: &mut Context<Self>) {
        if self.note.is_some() {
            self.layout.open(TileKind::Note);
            if self.note_reading {
                let rendered = self.note_read_view.read(cx).rendered_text();
                if let Some(target) = rendered.range_for_source(range.clone()) {
                    self.note_read_view.update(cx, |view, cx| {
                        let _ = view.reveal_range(target, cx);
                    });
                    self.note_read_view
                        .read(cx)
                        .focus_handle()
                        .clone()
                        .focus(window, cx);
                    cx.notify();
                    return;
                }
                self.set_note_reading(false, window, cx);
            }
            self.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(range, cx);
                editor.focus(window, cx);
            });
        }
        cx.notify();
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
                self.notice = "A newer edit exists. It was preserved; recover it from the notes tile or overview.".into()
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
                    .child(button("create-named-note").label("Create note").on_click(
                        move |_, window, cx| {
                            let name = input.read(cx).value().to_string();
                            if name.trim().is_empty() {
                                return;
                            }
                            let mut created = false;
                            let _ = target.update(cx, |this, cx| {
                                match this.workspace.new_note(name.trim(), "") {
                                    Ok(record) => {
                                        this.open_note(&record.id, None, window, cx);
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
                        },
                    )),
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
                // The prepared selection replaces the reviewed candidate.
                self.review = None;
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
                // The prepared final text replaces the reviewed candidate.
                self.review = None;
                self.open_review(candidate, window, cx);
                self.notice =
                    "Your final text is prepared. Review this version, then apply it.".into();
            }
            Err(e) => self.notice = e.to_string(),
        };
        self.refresh();
        cx.notify();
    }
    fn select_writing_thread(
        &mut self,
        note: String,
        thread: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.note_project = self
            .workspace
            .store
            .project_context_record(&thread)
            .ok()
            .flatten()
            .filter(|r| !r.archived)
            .map(|r| r.id);
        self.writing_threads.insert(note, thread.clone());
        self.set_chat(ChatTarget::Thread(thread), window, cx);
        self.show_tile(TileKind::Chat, window, cx);
    }
    fn new_writing_thread(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = &self.note else {
            return;
        };
        let note = open.record.id.clone();
        let version = open.record.version;
        let result = self
            .workspace
            .new_document_chat(
                &format!("Writing: {}", title(&open.record)),
                &note,
                self.note_project.as_deref(),
            )
            .and_then(|thread| {
                self.workspace
                    .link_writing_thread(
                        &format!("writing-link:{}", uuid::Uuid::new_v4()),
                        &note,
                        version,
                        &thread.id,
                    )
                    .map(|_| thread.id)
            });
        match result {
            Ok(thread) => self.select_writing_thread(note, thread, window, cx),
            Err(error) => {
                self.notice =
                    format!("Could not link a new chat: {error}. Your drafts are preserved.")
            }
        };
        self.refresh();
        cx.notify();
    }
    fn ensure_writing_thread(&mut self, note: &str, version: u64) -> AppResult<String> {
        if let Some(project) = &self.note_project
            && self.writing_threads.get(note).is_some_and(|chat| {
                self.workspace
                    .store
                    .project_context_record(chat)
                    .ok()
                    .flatten()
                    .is_none_or(|p| p.id != *project)
            })
        {
            self.writing_threads.remove(note);
        }

        if let Some(thread) = self.writing_threads.get(note).cloned() {
            if !self
                .workspace
                .store
                .record(&thread)?
                .is_some_and(|record| matches!(record.data, RecordData::Thread(_)))
            {
                return Err(AppError::Invalid("The selected chat is unavailable. Choose New chat for this note; your drafts are preserved.".into()));
            }
            // Continuing a resolved thread reopens it with a recorded reason.
            self.reopen_if_resolved(&thread, "Owner continued a resolved chat with new feedback")?;
            return Ok(thread);
        }
        let thread_title = format!(
            "Writing: {}",
            self.note
                .as_ref()
                .map(|open| title(&open.record))
                .unwrap_or_else(|| note.into())
        );
        let thread =
            self.workspace
                .new_document_chat(&thread_title, note, self.note_project.as_deref())?;
        self.workspace.link_writing_thread(
            &format!("writing-link:{}", uuid::Uuid::new_v4()),
            note,
            version,
            &thread.id,
        )?;
        self.writing_threads.insert(note.into(), thread.id.clone());
        Ok(thread.id)
    }
    fn post_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((note, version, range, quote)) = self.comment_anchor.clone() else {
            return;
        };
        if self
            .note
            .as_ref()
            .is_none_or(|open| open.record.id != note || open.session.is_some())
        {
            self.notice = "Save your edit, then select the saved passage again. Your comment draft is preserved.".into();
            cx.notify();
            return;
        }
        let body = self.comment_input.read(cx).value().to_string();
        if body.trim().is_empty() {
            return;
        }
        let result = self
            .ensure_writing_thread(&note, version)
            .and_then(|thread| {
                let key = self
                    .comment_operation
                    .get_or_insert_with(|| format!("writing-comment:{}", uuid::Uuid::new_v4()))
                    .clone();
                self.workspace.add_writing_comment(
                    &key,
                    &WritingCommentInput {
                        note: note.clone(),
                        version,
                        thread,
                        range: (range.start, range.end),
                        quote,
                        body,
                    },
                )
            });
        match result {
            Ok(_) => {
                self.comment_anchor = None;
                self.comment_operation = None;
                self.comment_input
                    .update(cx, |e, cx| e.set_value("", window, cx));
                self.focus_visible_editor(window, cx);
                self.notice =
                    "Comment added to pending feedback. Rewrite with AI when ready.".into();
            }
            Err(error) => {
                self.notice =
                    format!("{error}. Your comment draft and saved passage are preserved.")
            }
        }
        self.adopt_writing_thread(&note);
        self.preserve_comment_draft(cx);
        self.refresh();
        self.sync_note_marks(cx);
        cx.notify();
    }
    fn rewrite_note(&mut self, cx: &mut Context<Self>) {
        if let Some(worker) = &self.worker {
            self.notice = worker.kind.busy_notice().into();
            cx.notify();
            return;
        }
        let Some(open) = &self.note else {
            return;
        };
        if open.session.is_some() {
            self.notice = "Save your edit before requesting a rewrite of the saved note.".into();
            cx.notify();
            return;
        }
        let note = open.record.id.clone();
        let version = open.record.version;
        let comments = match self.workspace.writing_comments(&note) {
            Ok(comments) => comments,
            Err(error) => {
                self.notice = error.to_string();
                cx.notify();
                return;
            }
        };
        let ids = comments
            .into_iter()
            .filter(|comment| {
                !comment.addressed && !self.excluded_comments.contains(&comment.record.id)
            })
            .map(|comment| comment.record.id)
            .collect::<Vec<_>>();
        let feedback = self.general_feedback.read(cx).value().to_string();
        self.general_drafts.insert(note.clone(), feedback.clone());
        let result = self
            .ensure_writing_thread(&note, version)
            .and_then(|thread| {
                self.workspace.snapshot_writing_request_with_feedback(
                    &format!("writing-request:{}", uuid::Uuid::new_v4()),
                    &note,
                    version,
                    &thread,
                    &ids,
                    &feedback,
                )
            });
        self.adopt_writing_thread(&note);
        match result {
            Ok(snapshot) => self.retry_rewrite(snapshot.request_message, cx),
            Err(error) => {
                self.notice =
                    format!("Rewrite could not be saved: {error}. Your feedback remains pending.");
                cx.notify();
            }
        }
        self.refresh();
        cx.notify();
    }
    fn confirm_addressed(
        &mut self,
        request: &str,
        operation: &str,
        ids: &[String],
        cx: &mut Context<Self>,
    ) {
        match self.workspace.mark_writing_comments_addressed(
            &format!("writing-addressed:{}", uuid::Uuid::new_v4()),
            request,
            operation,
            ids,
        ) {
            Ok(_) => {
                self.notice =
                    "Selected captured comments marked addressed. Other feedback remains pending."
                        .into()
            }
            Err(error) => self.notice = error.to_string(),
        }
        self.refresh();
        cx.notify();
    }
    fn retry_rewrite(&mut self, message: String, cx: &mut Context<Self>) {
        match self.workspace.writing_request(&message) {
            Ok(snapshot) => {
                let changed = snapshot.current_inputs().iter().any(|input| {
                    self.workspace
                        .store
                        .record(&input.record)
                        .ok()
                        .flatten()
                        .is_none_or(|record| record.archived || record.version != input.version)
                });
                if changed {
                    self.notice = "Saved rewrite context changed. Feedback is preserved; Reload saved note, then request a new Rewrite with AI.".into();
                    cx.notify();
                    return;
                }
                self.start_run("Rewrite the saved document using the captured feedback and selected chat; prepare a whole-note proposal for review.".into(), snapshot.thread, vec![], Some(brn_ai::threads::OwnerRequest::RewriteNote { request_message: message }), cx);
                if self.worker.is_none() {
                    self.notice = format!(
                        "Rewrite request saved; AI has not started. {} Retry rewrite uses these same saved inputs.",
                        self.notice
                    );
                }
            }
            Err(error) => self.notice = error.to_string(),
        }
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
        let writing = matches!(
            &owner_request,
            Some(brn_ai::threads::OwnerRequest::RewriteNote { .. })
        );
        let continuation = if writing {
            "Retry from the writing tile"
        } else {
            "Continue from the chat"
        };
        let selection = match self.settings.provider.to_ascii_lowercase().as_str() {
            "codex" | "chatgpt" => brn_ai::Selection {
                provider: brn_ai::Provider::Chatgpt,
                model: self.settings.model.clone(),
            },
            "copilot" => {
                if !self.copilot_models.is_empty()
                    && !self.copilot_models.contains(&self.settings.model)
                {
                    self.notice = format!(
                        "{} is not offered by GitHub Copilot, so BRN has not started. Choose a model from the model menu: {}.",
                        self.settings.model,
                        self.copilot_models.join(", ")
                    );
                    cx.notify();
                    return;
                }
                brn_ai::Selection {
                    provider: brn_ai::Provider::Copilot,
                    model: self.settings.model.clone(),
                }
            }
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
        let queued = self
            .queue_state()
            .pending
            .filter(|q| q.thread == thread && q.text == prompt && q.run.is_none());
        let context_edit_note = match &owner_request {
            Some(brn_ai::threads::OwnerRequest::EditNote { id, version }) => {
                Some((id.clone(), *version))
            }
            _ => None,
        };
        // Without a handle the search tools are not offered; say why.
        let search = match self.workspace.search_handle() {
            Ok(handle) => Some(Arc::new(handle) as Arc<dyn brn_ai::threads::RunNoteSearch>),
            Err(error) => {
                self.notice = format!("Note search is unavailable for this run: {error}");
                None
            }
        };
        let request = brn_ai::threads::ThreadRunRequest {
            request_message: queued.as_ref().and_then(|q| q.message.clone()),
            queue_claim: queued.and_then(|q| q.dispatch),
            owner_request,
            data_dir: self.data_dir.clone(),
            thread: thread.clone(),
            prompt,
            selection,
            effort,
            max_tool_rounds: brn_ai::threads::default_tool_rounds(!sources.is_empty()),
            auth_file: self.settings.auth_file.as_ref().map(PathBuf::from),
            credentials_dir: self.settings.credentials_dir.as_ref().map(PathBuf::from),
            sources,
            search,
            trace: brn_ai::threads::TraceLevel::Off,
        };
        let prepared = match brn_ai::threads::prepare_run(request) {
            Ok(prepared) => prepared,
            Err(error) => {
                self.notice = format!(
                    "Could not start. Your saved message remains available; {continuation}. {error}"
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
            let outcome = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string()).and_then(|runtime| {
                runtime.block_on(brn_ai::threads::run_prepared(prepared, worker_cancel, Arc::new(move |event| {
                    let item = match event {
                        brn_ai::AiEvent::Text(text) => WorkerEvent::Text(text),
                        brn_ai::AiEvent::AnswerFinalized(text) => WorkerEvent::AnswerFinalized(text),
                        brn_ai::AiEvent::ToolStarted { name } => WorkerEvent::Progress(brn_ai::threads::tool_activity(&name).unwrap_or("Working…").into()),
                        brn_ai::AiEvent::Activity(text) => WorkerEvent::Activity(text),
                        brn_ai::AiEvent::ReasoningSummary(text) => WorkerEvent::Activity(format!("reasoning summary> {text}")),
                        brn_ai::AiEvent::BudgetProgress { model_turns: _, tool_rounds, max_tool_rounds } => WorkerEvent::Progress(format!("Working · {tool_rounds} of {max_tool_rounds} steps used")),
                    };
                    if matches!(item, WorkerEvent::Text(_) | WorkerEvent::AnswerFinalized(_) | WorkerEvent::Activity(_)) { let _ = progress_sender.send(item); }
                    else { let _ = progress_sender.try_send(item); }
                }))).map_err(|e| e.to_string())
            });
            let (result, terminal) = match outcome {
                Ok(r) => {
                    // Host notices (such as an unavailable helper) follow the outcome.
                    let notices = r.notices.iter().map(|n| format!(" {n}")).collect::<String>();
                    (match r.answer.terminal {
                        brn_ai::AiTerminal::Completed => Ok(format!("{}{notices}", if writing { "Finished. Check the writing tile for the result." } else { "Finished. The answer and saved changes are in this chat." })),
                        brn_ai::AiTerminal::Interrupted => Err(format!("Interrupted. The queue is paused; Resume when ready.{notices}")),
                        brn_ai::AiTerminal::Failed(e) => Err(format!("{e}{notices}")),
                    }, r.terminal_run)
                }
                Err(e) => (Err(e), None),
            };
            let _ = sender.send(WorkerEvent::RunFinished { result, terminal });
        });
        if let Err(error) = spawned {
            self.notice = format!(
                "Could not start. Your saved instruction remains available; {continuation}. {error}"
            );
            self.refresh();
            cx.notify();
            return;
        }
        self.context_edit_note =
            context_edit_note.map(|(note, version)| (run_id.clone(), note, version));
        self.run_activity_chat = Some(thread.clone());
        self.run_activity_status.clear();
        self.worker = Some(Worker {
            receiver,
            cancel,
            conversion_cancel: Arc::new(AtomicBool::new(false)),
            kind: WorkerKind::Run,
            thread: Some(thread),
            run_id: Some(run_id),
        });
        self.stream.clear();
        self.run_activity_labels.clear();
        self.progress = "Starting…".into();
        self.notice.clear();
        cx.notify();
    }
    fn can_stop_and_send(&self) -> bool {
        self.pending_send.is_none()
            && self.worker.as_ref().is_some_and(|worker| {
                worker.kind == WorkerKind::Run
                    && worker.run_id.is_some()
                    && matches!(&self.chat, ChatTarget::Thread(thread) if worker.thread.as_ref() == Some(thread))
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
        let thread = worker.thread.clone().expect("run chat");
        let run_id = worker.run_id.as_ref().expect("prepared run");
        let cancellation = worker.cancel.clone();
        match self.workspace.stop_and_post_with_partial(
            &format!("stop-and-send:{}", uuid::Uuid::new_v4()),
            run_id,
            &thread,
            &prompt,
            (!self.stream.is_empty()).then_some(self.stream.as_str()),
        ) {
            Ok(_) => {
                // The durable commit and run fence precede input clearing and cancellation.
                self.pending_send = Some(PendingSend {
                    thread: thread.clone(),
                    prompt,
                });
                self.save_composer(self.destination(), String::new());
                self.composer
                    .update(cx, |editor, cx| editor.set_value("", window, cx));
                cancellation.cancel();
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
        if self.editing_queue.is_some()
            || self.worker.is_some()
            || self.queue_state().paused
                && (self.queue_state().pending.is_some() || !self.queue_state().queue.is_empty())
        {
            self.enqueue_composer(window, cx);
            return;
        }
        self.ask_with_authority(None, window, cx);
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
        let destination = self.destination();
        if prompt.trim().is_empty() {
            return;
        }
        let thread = match self.chat.clone() {
            ChatTarget::Thread(id) => self
                .reopen_if_resolved(&id, "Owner replied to a resolved chat")
                .map(|_| id),
            ChatTarget::NewForNote(note) => {
                let version = self
                    .records
                    .iter()
                    .find(|record| record.id == note)
                    .map_or(0, |record| record.version);
                self.ensure_writing_thread(&note, version)
            }
            ChatTarget::New => self.workspace.new_thread(&short(&prompt)).map(|r| r.id),
        };
        match thread.and_then(|thread| {
            self.workspace
                .post_message(&thread, &prompt)
                .map(|_| thread)
        }) {
            Ok(thread) => {
                self.save_composer(destination, String::new());
                self.composer
                    .update(cx, |e, cx| e.set_value("", window, cx));
                self.chat = ChatTarget::Thread(thread.clone());
                self.start_run(prompt, thread, vec![], owner_request, cx);
            }
            Err(e) => self.notice = format!("{e}. Your text is preserved."),
        }
        self.refresh();
        cx.notify();
    }
    fn reopen_if_resolved(&mut self, thread: &str, _reason: &str) -> AppResult<()> {
        if self.workspace.store.record(thread)?.is_some_and(|r| {
            r.archived || matches!(r.data,RecordData::Thread(t) if t.state==ThreadState::Resolved)
        }) {
            return Err(AppError::Invalid(
                "Unarchive this chat before continuing".into(),
            ));
        }
        Ok(())
    }
    fn resolve_thread(&mut self, thread: &str, cx: &mut Context<Self>) {
        let key = format!("archive:{}", uuid::Uuid::new_v4());
        let result = if let Some(run) = self
            .worker
            .as_ref()
            .filter(|w| w.thread.as_deref() == Some(thread))
            .and_then(|w| w.run_id.as_deref())
        {
            self.workspace
                .archive_chat_progress(&key, thread, run, &self.stream)
        } else {
            self.workspace.archive_chat(&key, thread)
        };
        match result {
            Ok(_) => {
                self.notice = "Chat archived. Work stopped and the queue is paused. Actions retain their progress.".into();
                if let Some(worker) = &self.worker
                    && worker.thread.as_deref() == Some(thread)
                {
                    worker.cancel.cancel();
                }
                self.pending_send = None;
            }
            Err(e) => self.notice = e.to_string(),
        }
        self.refresh();
        cx.notify();
    }
    fn reopen_thread(&mut self, thread: &str, cx: &mut Context<Self>) {
        match self
            .workspace
            .unarchive_chat(&format!("unarchive:{}", uuid::Uuid::new_v4()), thread)
        {
            Ok(_) => self.notice = "Chat unarchived. The queue stays paused until Resume.".into(),
            Err(e) => self.notice = e.to_string(),
        }
        self.refresh();
        cx.notify();
    }
    fn choose_theme(&mut self, theme: ThemeId, window: &mut Window, cx: &mut Context<Self>) {
        self.theme = theme;
        theme::apply(theme, cx);
        self.theme_pending = self.settings.theme.as_deref() != Some(theme.key());
        self.persist_theme();
        self.sync_note_marks(cx);
        if self.review.is_some() {
            self.sync_diff(window, cx);
        }
        window.refresh();
        cx.notify();
    }
    /// Settings writes would stop running invocations, so the core defers this
    /// while any run is working; it is retried when local work finishes.
    fn persist_theme(&mut self) {
        if !self.theme_pending {
            return;
        }
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.kind == WorkerKind::Run)
        {
            self.notice = format!(
                "{} applied. It is saved for next time when the current work finishes.",
                self.theme.label()
            );
            return;
        }
        match self.workspace.choose_theme(self.theme.key()) {
            Ok(ApplyOutcome::Applied(_)) => {
                self.theme_pending = false;
                self.refresh();
            }
            Ok(ApplyOutcome::Deferred { .. }) => {
                self.notice = format!(
                    "{} applied. It is saved for next time once no work is running.",
                    self.theme.label()
                )
            }
            Ok(outcome) => self.notice = outcome_message(&outcome),
            Err(error) => self.notice = error.to_string(),
        }
    }
    /// Pills change the explicit Settings through the existing configure path.
    fn set_model(
        &mut self,
        provider: &str,
        model: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut settings = self.settings.clone();
        settings.provider = provider.into();
        settings.model = model.into();
        self.configure_explicit(settings, window, cx);
    }
    fn set_effort(&mut self, effort: &str, window: &mut Window, cx: &mut Context<Self>) {
        let mut settings = self.settings.clone();
        settings.effort = effort.into();
        self.configure_explicit(settings, window, cx);
    }
    fn configure_explicit(
        &mut self,
        settings: WorkspaceSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(worker) = &self.worker {
            self.notice = worker.kind.busy_notice().into();
            cx.notify();
            return;
        }
        match self.workspace.configure(settings) {
            Ok(outcome) => self.notice = outcome_message(&outcome),
            Err(e) => self.notice = e.to_string(),
        }
        self.refresh();
        let model = self.settings.model.clone();
        self.model
            .update(cx, |input, cx| input.set_value(model, window, cx));
        cx.notify();
    }
    fn open_review(&mut self, candidate: Prepared, window: &mut Window, cx: &mut Context<Self>) {
        // Showing the candidate already under review keeps any edited final text.
        if self.review.as_ref().is_some_and(|review| {
            review.candidate.operation.as_str() == candidate.operation.as_str()
        }) {
            self.show_tile(TileKind::Note, window, cx);
            return;
        }
        if self.review.as_ref().is_some_and(|review| {
            review.candidate.operation.as_str() != candidate.operation.as_str()
        }) {
            self.keep_review(window, cx);
            if self.review.is_some() {
                return;
            }
        }
        self.preserve_visible_note_input(cx);
        let snapshot = candidate
            .request
            .inputs
            .iter()
            .find_map(|input| self.workspace.writing_request(&input.record).ok());
        let write = candidate
            .request
            .writes
            .iter()
            .find(|w| matches!(w.data, RecordData::Note(_)));
        if let Some(snapshot) = &snapshot {
            self.writing_threads
                .insert(snapshot.note.id.clone(), snapshot.thread.clone());
        }
        let target = snapshot
            .as_ref()
            .map(|snapshot| snapshot.note.id.clone())
            .or_else(|| write.map(|write| write.id.clone()));
        if let Some(note) = &target
            && self.open_note_id() != Some(note.as_str())
        {
            self.review = None;
            self.open_note(note, None, window, cx);
        }
        if let Some(snapshot) = snapshot
            && self.open_note_id() == Some(snapshot.note.id.as_str())
        {
            self.set_chat(ChatTarget::Thread(snapshot.thread), window, cx);
        }
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
        self.review_editing = false;
        self.review_inputs = false;
        self.review_reading = false;
        self.review_editor
            .update(cx, |e, cx| e.set_value(text.clone(), window, cx));
        self.current_view
            .update(cx, |v, cx| v.set_text(&current, cx));
        self.candidate_view
            .update(cx, |v, cx| v.set_text(&text, cx));
        self.layout.open(TileKind::Note);
        self.sync_diff(window, cx);
        self.focus_tile_contents(window, cx);
        cx.notify();
    }
    fn start_review_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .review
            .as_ref()
            .is_some_and(|review| review.note.is_some())
        {
            self.review_editing = true;
            self.focus_visible_editor(window, cx);
            cx.notify();
        }
    }
    fn cancel_review_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(review) = &mut self.review {
            let original = review.original.clone();
            review.edited = false;
            self.review_editor
                .update(cx, |e, cx| e.set_value(original, window, cx));
            if let Some(review) = &mut self.review {
                review.edited = false;
            }
        }
        self.review_editing = false;
        self.focus_root(window, cx);
        cx.notify();
    }
    fn apply_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = &self.review else { return };
        if review.edited || review.selected.len() != review.groups.groups.len() {
            self.notice = "Prepare your edited text or selected groups before applying it.".into();
            cx.notify();
            return;
        }
        let candidate = review.candidate.clone();
        self.apply_candidate(candidate, window, cx);
    }
    /// Card actions must not silently discard final text being edited in review.
    fn refuse_if_editing(&mut self, candidate: &Prepared, cx: &mut Context<Self>) -> bool {
        let editing = self.review.as_ref().is_some_and(|review| {
            review.edited && review.candidate.operation.as_str() == candidate.operation.as_str()
        });
        if editing {
            self.notice = "You are editing this proposal's final text. Use this text or cancel the edit in the note tile first.".into();
            cx.notify();
        }
        editing
    }
    fn card_apply(&mut self, candidate: Prepared, window: &mut Window, cx: &mut Context<Self>) {
        if !self.refuse_if_editing(&candidate, cx) {
            self.apply_candidate(candidate, window, cx);
        }
    }
    fn card_dismiss(&mut self, candidate: Prepared, window: &mut Window, cx: &mut Context<Self>) {
        if !self.refuse_if_editing(&candidate, cx) {
            self.dismiss_candidate(candidate, window, cx);
        }
    }
    /// Apply exactly this prepared candidate; nothing is re-derived.
    fn apply_candidate(
        &mut self,
        candidate: Prepared,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.workspace.review_apply(candidate.operation.as_str()) {
            Ok(outcome) => {
                self.notice = outcome_message(&outcome);
                if let ApplyOutcome::Applied(receipt) = &outcome {
                    if let Some(input) = candidate
                        .request
                        .inputs
                        .iter()
                        .find(|input| self.workspace.writing_request(&input.record).is_ok())
                        && let Ok(snapshot) = self.workspace.writing_request(&input.record)
                    {
                        self.applied_rewrite.insert(
                            snapshot.note.id,
                            (input.record.clone(), receipt.operation.as_str().into()),
                        );
                    }
                    if self.review.as_ref().is_some_and(|review| {
                        review.candidate.operation.as_str() == candidate.operation.as_str()
                    }) {
                        self.review = None;
                        self.review_editing = false;
                    }
                    self.reload_open_note(window, cx);
                }
            }
            Err(e) => self.notice = e.to_string(),
        };
        self.refresh();
        cx.notify();
    }
    /// After a checked change, show the newly saved note unless the owner has
    /// an unsaved edit, which stays untouched.
    fn reload_open_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(open) = &mut self.note
            && open.session.is_none()
            && let Ok(Some(record)) = self.workspace.store.note(&open.record.id)
            && let RecordData::Note(note) = &record.data
        {
            self.note_editor.update(cx, |editor, cx| {
                editor.set_value(note.markdown.clone(), window, cx)
            });
            self.current_view
                .update(cx, |view, cx| view.set_text(&note.markdown, cx));
            open.record = record;
            self.selection = None;
        }
        self.sync_read_text(cx);
        self.refresh();
        self.sync_note_marks(cx);
        self.focus_visible_editor(window, cx);
    }
    fn dismiss_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = &self.review else { return };
        let candidate = review.candidate.clone();
        self.dismiss_candidate(candidate, window, cx);
    }
    fn dismiss_candidate(
        &mut self,
        candidate: Prepared,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.workspace.review_dismiss(candidate.operation.as_str()) {
            Ok(()) => {
                if self.review.as_ref().is_some_and(|review| {
                    review.candidate.operation.as_str() == candidate.operation.as_str()
                }) {
                    self.review = None;
                    self.review_editing = false;
                }
                self.focus_visible_editor(window, cx);
                self.notice = "Dismissed the proposal. The note is unchanged.".into();
            }
            Err(error) => self.notice = error.to_string(),
        }
        self.refresh();
        cx.notify();
    }
    fn undo_receipt(&mut self, operation: &str, window: &mut Window, cx: &mut Context<Self>) {
        match self.workspace.undo(operation) {
            Ok(outcome) => self.notice = outcome_message(&outcome),
            Err(error) => self.notice = error.to_string(),
        };
        self.refresh();
        self.reload_open_note(window, cx);
        cx.notify();
    }
    fn toggle_protection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = &self.note else { return };
        if open.session.is_some() {
            return;
        }
        let record = open.record.clone();
        let RecordData::Note(note) = &record.data else {
            return;
        };
        let mut changed = note.clone();
        changed.protected = !changed.protected;
        self.change(
            "Owner changed note protection",
            record.clone(),
            RecordData::Note(changed),
            cx,
        );
        self.open_note(&record.id, None, window, cx);
    }
    fn open_launcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.launcher_open = true;
        self.launcher_index = 0;
        self.launcher_query
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.launcher_query
            .update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }
    fn close_launcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.launcher_open {
            self.launcher_open = false;
            self.focus_tile_contents(window, cx);
            cx.notify();
        }
    }
    fn launcher_items(&self, cx: &App) -> Vec<lists::LaunchItem> {
        lists::launcher(
            self.launcher_query.read(cx).value().as_ref(),
            &self.thread_rows,
            &self.note_rows,
        )
    }
    fn move_launcher(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.launcher_items(cx).len();
        if count > 0 {
            self.launcher_index =
                (self.launcher_index as isize + delta).rem_euclid(count as isize) as usize;
            cx.notify();
        }
    }
    fn run_launcher(&mut self, index: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.launcher_items(cx);
        let Some(item) = items.get(index.unwrap_or(self.launcher_index)).cloned() else {
            return;
        };
        self.close_launcher(window, cx);
        match item {
            lists::LaunchItem::Thread { id, .. } => self.open_thread(&id, window, cx),
            lists::LaunchItem::Note { id, .. } => self.open_note(&id, None, window, cx),
            lists::LaunchItem::Command(command) => self.run_command(command, window, cx),
        }
    }
    fn run_command(
        &mut self,
        command: lists::Command,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match command {
            lists::Command::NewThread => self.new_thread(window, cx),
            lists::Command::NewNote => self.new_note(window, cx),
            lists::Command::Rewrite => self.rewrite_note(cx),
            lists::Command::Save => self.save(window, cx),
            lists::Command::Layout(preset) => self.apply_layout(preset, window, cx),
            lists::Command::Theme(theme) => self.choose_theme(theme, window, cx),
            lists::Command::OpenTile(kind) => {
                self.show_tile(kind, window, cx);
            }
            lists::Command::Keys => self.show_keys(window, cx),
        }
    }
    fn continue_run(&mut self, thread: String, cx: &mut Context<Self>) {
        if self
            .queue_state()
            .pending
            .is_some_and(|q| q.thread == thread)
        {
            self.resume_chat_queue(cx);
            return;
        }
        if let Ok(messages) = self.workspace.messages(&thread) {
            for record in messages.iter().rev() {
                if let RecordData::Message(message) = &record.data {
                    if message.role == "owner" {
                        break;
                    }
                    if message.role == "writing-request" {
                        self.retry_rewrite(record.id.clone(), cx);
                        return;
                    }
                }
            }
        }
        self.start_run("Continue the work from the saved conversation, current linked notes and recorded outcomes. Recheck current versions and authority.".into(),thread,vec![],None,cx);
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        if self.pending_send.take().is_some() {
            self.notice = "Automatic continuation stopped. Your saved instruction remains in the chat; Continue when ready.".into();
        }
        let stopped = if let Some(run) = self.worker.as_ref().and_then(|w| w.run_id.clone()) {
            self.workspace.stop_chat_run(
                &format!("stop:{}", uuid::Uuid::new_v4()),
                &run,
                (!self.stream.is_empty()).then_some(self.stream.as_str()),
            )
        } else {
            self.workspace
                .pause_queue(&format!("pause:{}", uuid::Uuid::new_v4()))
        };
        if let Err(e) = stopped {
            self.notice = e.to_string();
            cx.notify();
            return;
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
                self.pause_failed_worker();
                if self
                    .worker
                    .as_ref()
                    .is_some_and(|w| w.kind == WorkerKind::Run)
                {
                    self.run_activity_status = "Disconnected".into();
                    self.context_edit_note = None;
                }
                self.worker = None;
                self.pending_send = None;
                self.stream.clear();
                self.progress.clear();
                self.notice="Work stopped unexpectedly. Saved progress remains available; retry from the writing tile or continue from the chat.".into();
                self.refresh();
                cx.notify();
            }
            return;
        }
        let mut finished = false;
        let mut allow_dispatch = false;
        for event in events {
            // Events belong to the old worker, even when a producer queues after Finished.
            if finished {
                continue;
            }
            let event = match event {
                WorkerEvent::RunFinished { result, terminal } => {
                    let advance = self.acknowledge_run(terminal, result.is_ok());
                    allow_dispatch = advance;
                    WorkerEvent::Finished(result)
                }
                event => event,
            };
            match event {
                WorkerEvent::RunFinished { .. } => unreachable!(),
                WorkerEvent::AnswerFinalized(text) => {
                    if !self
                        .worker
                        .as_ref()
                        .is_some_and(|w| w.cancel.is_cancelled())
                    {
                        self.stream = text;
                    }
                }
                WorkerEvent::Text(text) => {
                    if !self
                        .worker
                        .as_ref()
                        .is_some_and(|w| w.cancel.is_cancelled())
                    {
                        self.stream.push_str(&text);
                    }
                }
                WorkerEvent::Progress(text) => {
                    if self
                        .worker
                        .as_ref()
                        .is_some_and(|w| w.kind == WorkerKind::Run && !w.cancel.is_cancelled())
                    {
                        self.record_activity(&text);
                    }
                    if !self
                        .worker
                        .as_ref()
                        .is_some_and(|w| w.cancel.is_cancelled())
                    {
                        self.progress = text;
                    }
                }
                WorkerEvent::Activity(text) => {
                    if self
                        .worker
                        .as_ref()
                        .is_some_and(|w| w.kind == WorkerKind::Run && !w.cancel.is_cancelled())
                    {
                        self.record_activity(&text);
                    }
                }
                WorkerEvent::CopilotLogin {
                    verification_uri,
                    user_code,
                } => {
                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(user_code.clone()));
                    cx.open_url(&verification_uri);
                    self.progress = format!(
                        "GitHub sign-in: enter code {user_code} at {verification_uri} (code copied)"
                    );
                }
                WorkerEvent::CopilotModels(models) => self.copilot_models = models,
                WorkerEvent::CopilotConnected(dir) => {
                    let mut settings = self.settings.clone();
                    settings.provider = "copilot".into();
                    settings.credentials_dir = Some(dir);
                    if let Err(error) = self.workspace.configure(settings.clone()) {
                        self.notice = error.to_string();
                    } else {
                        self.settings = settings;
                    }
                }
                WorkerEvent::Finished(result) => {
                    finished = true;
                    let kind = self.worker.as_ref().map(|w| w.kind);
                    let context_edit = self.context_edit_note.take().filter(|(run, _, _)| {
                        self.worker.as_ref().and_then(|w| w.run_id.as_deref()) == Some(run.as_str())
                    });
                    if kind == Some(WorkerKind::Run) {
                        self.run_activity_status = self
                            .worker
                            .as_ref()
                            .and_then(|w| w.run_id.as_deref())
                            .and_then(|id| self.workspace.store.record(id).ok().flatten())
                            .and_then(|r| match r.data {
                                RecordData::Run(run) => Some(if run.state == RunState::Working {
                                    "Work ended · status unconfirmed".into()
                                } else {
                                    format!("{:?}", run.state)
                                }),
                                _ => None,
                            })
                            .unwrap_or_else(|| "Work ended".into());
                    }
                    if kind != Some(WorkerKind::Run) {
                        allow_dispatch = result.is_ok();
                        if result.is_err() {
                            self.pause_failed_worker();
                        }
                    }

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
                    self.last_work_notice = Some(self.notice.clone());
                    match kind {
                        Some(WorkerKind::Export) => self.export_status = self.notice.clone(),
                        Some(WorkerKind::Backup) => self.backup_status = self.notice.clone(),
                        _ => {}
                    }
                    self.refresh();
                    if let Some((_, note, version)) = context_edit {
                        self.refresh_context_edit(&note, version, window, cx);
                    }
                }
                WorkerEvent::Converted {
                    result,
                    title,
                    chat,
                } => {
                    let cancelled = self.worker.as_ref().is_some_and(|worker| {
                        worker.cancel.is_cancelled()
                            || worker
                                .conversion_cancel
                                .load(std::sync::atomic::Ordering::Acquire)
                    });
                    finished = true;
                    self.worker = None;
                    self.progress.clear();
                    let result = if cancelled {
                        Err("Document conversion was stopped before saving.".into())
                    } else {
                        *result
                    };
                    match result {
                        Ok(converted) => {
                            if converted.intent == ImportIntent::FullNote {
                                match if let Some(chat) = &chat {
                                    self.workspace.attach_full(
                                        &format!("attach:{}", uuid::Uuid::new_v4()),
                                        chat,
                                        &title,
                                        &converted,
                                    )
                                } else {
                                    self.workspace.import_full(
                                        &format!("full-import:{}", uuid::Uuid::new_v4()),
                                        &title,
                                        &converted,
                                    )
                                } {
                                    Ok(outcome) => {
                                        allow_dispatch =
                                            matches!(outcome, ApplyOutcome::Applied(_));
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
                                        if let Some(id) = id
                                            && chat.is_none()
                                        {
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
                    if !allow_dispatch && self.worker.is_none() {
                        let _ = self
                            .workspace
                            .pause_queue(&format!("conversion-paused:{}", uuid::Uuid::new_v4()));
                    }
                    self.refresh();
                }
            }
        }
        if finished {
            self.persist_theme();
            if let Some(pending) = self.pending_send.take() {
                // Launch only after draining the complete old batch, without navigation or grants.
                self.start_run(pending.prompt, pending.thread, vec![], None, cx);
            } else if allow_dispatch {
                self.dispatch_next(cx);
            }
        } else if disconnected && self.worker.is_some() {
            self.pause_failed_worker();
            if self
                .worker
                .as_ref()
                .is_some_and(|w| w.kind == WorkerKind::Run)
            {
                self.run_activity_status = "Disconnected".into();
                self.context_edit_note = None;
            }
            self.worker = None;
            self.pending_send = None;
            self.stream.clear();
            self.progress.clear();
            let _ = self
                .workspace
                .recover_queue(&format!("disconnect:{}", uuid::Uuid::new_v4()));
            self.notice = "Work stopped unexpectedly. Your saved instruction remains available; Continue from the chat.".into();
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
        // Re-supply lands in the chat that already received this source, in
        // the open chat's project (general otherwise), unless work there is live.
        let project = self
            .chat_thread()
            .and_then(|chat| brn_threads_app::project_for_chat(&self.records, chat))
            .map(|project| project.id.clone());
        match self
            .workspace
            .source_intake_chat(
                project.as_deref(),
                &converted.source_reference,
                &format!("Information from {title}"),
            )
            .map(|(chat, _)| chat)
        {
            Ok(thread) => {
                let instruction = brn_threads_app::SOURCE_INTAKE_INSTRUCTION;
                match self
                    .workspace
                    .post_intake_instruction(&thread.id, &converted.source_reference)
                {
                    Ok(_) => {
                        self.refresh();
                        self.open_thread(&thread.id, window, cx);
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
        let attachment_chat = self.attaching_chat.take();
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
                        this.convert_file(data, path, intent, attachment_chat, cx);
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
        attachment_chat: Option<String>,
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
            Some("html" | "htm") => DocumentKind::Html,
            Some("xlsx") => DocumentKind::Xlsx,
            Some("pptx") => DocumentKind::Pptx,
            _ => {
                self.notice =
                    "Choose DOCX, PDF, HTML, XLSX, PPTX, Markdown, text or an EML email file."
                        .into();
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
        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        let auth_file = self.settings.auth_file.as_ref().map(PathBuf::from);
        let credentials_dir = self.settings.credentials_dir.as_ref().map(PathBuf::from);
        let (sender, receiver) = mpsc::sync_channel(8);
        let progress_sender = sender.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<ConversionResult, String> {
                let limits = brn_threads_intake::import_limits(kind);
                let bytes = brn_threads_intake::read_input_file(
                    &path,
                    limits.max_input_bytes as usize,
                    &conversion_cancel,
                )
                .map_err(|e| e.to_string())?;
                let helper = std::env::current_exe()
                    .map_err(|e| e.to_string())?
                    .with_file_name("brn-intake-helper");
                let converter = Converter {
                    helper_path: helper,
                    pdf_tools: PdfTools::discover(),
                    temp_root: std::env::temp_dir().join("brn-threads-intake"),
                    limits,
                };
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?;
                runtime
                    .block_on(brn_ai::conversion::convert(
                        brn_ai::conversion::ConversionJob {
                            converter,
                            kind,
                            intent,
                            bytes,
                            source_reference: reference,
                            inventory: None,
                            auth_file,
                            credentials_dir,
                        },
                        worker_cancel,
                        conversion_cancel,
                        Arc::new(move |progress| {
                            let _ = progress_sender
                                .try_send(WorkerEvent::Progress(progress.label().into()));
                        }),
                    ))
                    .map_err(|e| e.to_string())
            })();
            let _ = sender.send(WorkerEvent::Converted {
                result: Box::new(result),
                title,
                chat: attachment_chat,
            });
        });
        self.worker = Some(Worker {
            receiver,
            cancel,
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
    fn connect_copilot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copilot_worker(true, Some(window), cx);
    }
    pub(super) fn load_copilot_models(&mut self, cx: &mut Context<Self>) {
        self.copilot_worker(false, None, cx);
    }
    /// Copilot model IDs differ from Codex IDs, so learn them before the owner sends.
    fn auto_load_copilot_models(&mut self, cx: &mut Context<Self>) {
        if shell::provider_key(&self.settings.provider) == "copilot"
            && self.settings.credentials_dir.is_some()
            && self.worker.is_none()
        {
            self.load_copilot_models(cx);
        }
    }
    fn copilot_worker(
        &mut self,
        connect: bool,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        if let Some(worker) = &self.worker {
            self.notice = worker.kind.busy_notice().into();
            cx.notify();
            return;
        }
        let mut dir = self.credentials_path.read(cx).value().trim().to_string();
        if dir.is_empty() {
            dir = self.settings.credentials_dir.clone().unwrap_or_default();
        }
        if dir.is_empty() {
            if !connect {
                self.notice = "Connect GitHub Copilot in Settings first.".into();
                cx.notify();
                return;
            }
            match default_copilot_credentials_dir() {
                Ok(default) => dir = default.to_string_lossy().into_owned(),
                Err(error) => {
                    self.notice =
                        format!("Could not prepare the Copilot credentials folder: {error}");
                    cx.notify();
                    return;
                }
            }
            if let Some(window) = window {
                self.credentials_path
                    .update(cx, |input, cx| input.set_value(dir.clone(), window, cx));
            }
        }
        let current_model = self.settings.model.clone();
        let (sender, receiver) = mpsc::sync_channel(16);
        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        let login_sender = sender.clone();
        let connected_sender = sender.clone();
        let path = PathBuf::from(&dir);
        let spawned = std::thread::Builder::new()
            .name("brn-copilot-connect".into())
            .spawn(move || {
                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())
                    .and_then(|runtime| {
                        runtime.block_on(async {
                            let auth = brn_ai::Auth::open(&path).map_err(|e| e.to_string())?;
                            let mut message = String::new();
                            if connect {
                                let login = Arc::new(move |prompt: brn_ai::LoginPrompt| {
                                    let _ = login_sender.try_send(WorkerEvent::CopilotLogin {
                                        verification_uri: prompt.verification_uri,
                                        user_code: prompt.user_code,
                                    });
                                });
                                let status = auth
                                    .connect(brn_ai::Provider::Copilot, login, worker_cancel.clone())
                                    .await
                                    .map_err(|e| e.to_string())?;
                                let _ = connected_sender
                                    .send(WorkerEvent::CopilotConnected(path.to_string_lossy().into()));
                                message = match status.name {
                                    Some(name) => format!("Connected to GitHub Copilot as {name}. "),
                                    None => "Connected to GitHub Copilot. ".into(),
                                };
                            }
                            match auth.models(brn_ai::Provider::Copilot, worker_cancel).await {
                                Ok(models) => {
                                    let ids: Vec<String> =
                                        models.into_iter().map(|m| m.id).collect();
                                    if ids.contains(&current_model) {
                                        message.push_str(&format!(
                                            "{} Copilot models available.",
                                            ids.len()
                                        ));
                                    } else {
                                        message.push_str(&format!(
                                            "{current_model} is not offered by GitHub Copilot. Choose a model from the model menu: {}.",
                                            ids.join(", ")
                                        ));
                                    }
                                    let _ = connected_sender.send(WorkerEvent::CopilotModels(ids));
                                }
                                Err(error) if connect => message.push_str(&format!(
                                    "Could not list Copilot models: {error}"
                                )),
                                Err(error) => return Err(error.to_string()),
                            }
                            Ok(message)
                        })
                    });
                let _ = sender.send(WorkerEvent::Finished(result));
            });
        if let Err(error) = spawned {
            self.notice = format!("Could not start GitHub Copilot sign-in. {error}");
            cx.notify();
            return;
        }
        self.worker = Some(Worker {
            receiver,
            cancel,
            conversion_cancel: Arc::new(AtomicBool::new(false)),
            kind: WorkerKind::Connect,
            thread: None,
            run_id: None,
        });
        self.progress = if connect {
            "Starting GitHub sign-in…"
        } else {
            "Loading GitHub Copilot models…"
        }
        .into();
        cx.notify();
    }
    fn configure(&mut self, cx: &mut Context<Self>) {
        self.settings.model = self.model.read(cx).value().to_string();
        let auth = self.auth_path.read(cx).value().to_string();
        let credentials = self.credentials_path.read(cx).value().to_string();
        self.settings.auth_file = (!auth.trim().is_empty()).then_some(auth);
        self.settings.credentials_dir =
            (!credentials.trim().is_empty()).then_some(credentials.clone());
        if let Some(helper) = &mut self.settings.helper {
            helper.model = self.helper_model.read(cx).value().trim().to_string();
            // The helper uses the credential location shown for its provider.
            let codex = !helper.provider.eq_ignore_ascii_case("copilot");
            helper.auth_file = self.settings.auth_file.clone().filter(|_| codex);
            helper.credentials_dir = (!credentials.trim().is_empty()
                && (!codex || helper.auth_file.is_none()))
            .then_some(credentials);
        }
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
    fn keep_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.review.as_ref().is_some_and(|r| r.edited) {
            self.prepare_review_edit(window, cx);
        }
        if self.review.as_ref().is_some_and(|r| !r.edited) {
            self.review = None;
            self.review_editing = false;
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
            .update(cx, |e, cx| e.set_value(text.clone(), window, cx));
        self.candidate_view
            .update(cx, |v, cx| v.set_text(&text, cx));
        self.current_view
            .update(cx, |v, cx| v.set_text(&current, cx));
        self.review_editing = false;
        self.sync_diff(window, cx);
        cx.notify();
    }
    fn save_copy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = &self.note else { return };
        let text = self.note_editor.read(cx).value().to_string();
        let name = format!("Recovered {}", title(&open.record));
        match self.workspace.new_note(&name, &text) {
            Ok(record) => {
                self.open_note(&record.id, None, window, cx);
                self.notice="Saved your text as a separate working note. The original edit remains recoverable from the overview.".into();
            }
            Err(e) => self.notice = e.to_string(),
        };
        self.refresh();
        cx.notify();
    }
}

fn bindings() -> Vec<KeyBinding> {
    let mut bindings = vec![
        KeyBinding::new("cmd-k", OpenLauncher, None),
        KeyBinding::new("cmd-s", Save, None),
        KeyBinding::new("cmd-enter", Ask, None),
        KeyBinding::new("cmd-l", FocusAsk, None),
        KeyBinding::new("cmd-n", NewNote, None),
        KeyBinding::new("cmd-t", NewThread, None),
        KeyBinding::new("cmd-0", Home, None),
        KeyBinding::new("cmd-/", CommentSelection, None),
        KeyBinding::new("cmd-shift-r", Rewrite, None),
        KeyBinding::new("cmd-\\", ToggleThreads, None),
        KeyBinding::new("cmd-shift-enter", MaximizeTile, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("alt-1", LayoutWrite, None),
        KeyBinding::new("alt-2", LayoutChat, None),
        KeyBinding::new("alt-3", LayoutSolo, None),
        KeyBinding::new("cmd-q", Quit, None),
    ];
    // Text inputs bind these keys to indent/outdent and code actions. Binding
    // the same keys in the input context later gives BRN's meaning precedence.
    for context in [None, Some("Input")] {
        bindings.push(KeyBinding::new("cmd-[", FocusPreviousTile, context));
        bindings.push(KeyBinding::new("cmd-]", FocusNextTile, context));
        bindings.push(KeyBinding::new("cmd-.", Cancel, context));
    }
    bindings
}

pub fn launch(data_dir: PathBuf) -> Result<(), String> {
    let mut workspace = Workspace::open(&data_dir).map_err(|e| e.to_string())?;
    workspace.settings().map_err(|e| e.to_string())?;
    // An unusable model folder is reported; keyword search still works.
    let model_notice = match workspace.embedding_model_from_env() {
        Some(Err(error)) => Some(format!("Semantic search is off: {error}")),
        _ => None,
    };
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.bind_keys(bindings());
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
                    ..top_bar_window_options()
                },
                move |window, cx| {
                    let view = cx.new(|cx| {
                        let mut view = ThreadsUi::new(workspace, data_dir, window, cx);
                        if let Some(notice) = model_notice {
                            view.notice = notice;
                        }
                        view
                    });
                    view.update(cx, |view, cx| view.auto_load_copilot_models(cx));
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
            .expect("failed to open BRN Chats window");
            cx.activate(true);
        });
    Ok(())
}

#[cfg(all(test, feature = "native-test-support"))]
mod policy_tests {
    use super::*;

    #[test]
    fn external_links_allow_only_absolute_lowercase_http_s() {
        assert!(external_link("https://example.com/a"));
        assert!(external_link("http://example.com"));
        for inert in [
            "javascript:alert(1)",
            "data:text/html,<b>",
            "file:///etc/passwd",
            "brn-asset://record",
            "HTTPS://example.com",
            "https:example.com",
            "//example.com",
            "example.com",
            " https://example.com",
            "",
        ] {
            assert!(!external_link(inert), "{inert:?} must stay inert");
        }
    }

    #[test]
    fn images_resolve_only_from_the_caller_s_own_stored_assets() {
        let key = "brn-asset://stored";
        let image = Arc::new(Image::from_bytes(ImageFormat::Png, vec![1, 2, 3]));
        let assets = BTreeMap::from([(key.to_owned(), image.clone())]);
        assert!(Arc::ptr_eq(&stored_image(&assets, key), &image));
        for uri in [
            "brn-asset://missing",
            "https://example.com/figure.png",
            "file:///tmp/figure.png",
            "",
        ] {
            let fallback = stored_image(&assets, uri);
            assert_eq!(fallback.format, ImageFormat::Png);
            assert!(fallback.bytes.is_empty(), "{uri:?} must not resolve");
        }
    }
}
