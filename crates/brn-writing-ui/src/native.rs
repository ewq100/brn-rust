//! Concrete writing panel state and immediate host requests; no runtime or store ownership.
use crate::{components, layout, theme::pal};
use brn_threads_app::*;
use gpui_kit::base::TextViewState;
use gpui_kit::base::input::{RangeDecorationCollection, TextDecorationCollection};
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::input::{Editor, EditorState, TextareaState};
use gpui_kit::{App, AppContext, Context, Entity, Image, Window, div, prelude::*, px, rgb};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::Arc,
};

#[path = "references.rs"]
mod references;
#[path = "setup.rs"]
pub mod setup;
use gpui_kit::component::input::InputState;
#[path = "review.rs"]
mod review;
pub use review::ReviewLifecycle;
#[path = "note.rs"]
mod note;
pub use note::{NoteDiscard, NoteLifecycle, OpenNote};

gpui_kit::actions!(
    brn_threads,
    [Save, CommentSelection, AskAboutSelection, Rewrite]
);
#[derive(Clone)]
pub struct CommentDraft {
    pub anchor: Option<(String, u64, Range<usize>, String)>,
    pub body: String,
    pub operation: Option<String>,
}

pub struct Review {
    pub candidate: Prepared,
    pub note: Option<String>,
    pub original: String,
    pub groups: ReviewGroups,
    pub selected: BTreeSet<String>,
}
pub const MONO: &str = "Menlo";
pub const UI_FONT: &str = ".SystemUIFont";
pub const SMALL_TEXT: gpui_kit::Pixels = px(12.);
pub const UI_TEXT: gpui_kit::Pixels = px(14.);
pub const PROSE_TEXT: gpui_kit::Pixels = px(16.);
pub const TITLE_TEXT: gpui_kit::Pixels = px(18.);
/// Controls use the 6 px control radius (`theme.radius`).
pub fn button(id: impl Into<gpui_kit::ElementId>) -> Button {
    Button::new(id)
        .rounded(ButtonRounded::Medium)
        .font_family(UI_FONT)
        .text_size(UI_TEXT)
}
pub fn prose_editor(state: &Entity<EditorState>) -> Editor {
    Editor::new(state)
        .font_family(UI_FONT)
        .text_size(PROSE_TEXT)
}
/// A title inside a page or panel.
pub fn subheading(text: impl Into<gpui_kit::SharedString>) -> gpui_kit::Div {
    div()
        .font_family(UI_FONT)
        .text_size(UI_TEXT)
        .text_color(rgb(pal().ink))
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .child(text.into())
}
/// A sentence-case group label.
pub fn section(text: impl Into<gpui_kit::SharedString>) -> gpui_kit::Div {
    components::section_header(text, None, None)
}
pub fn metadata(text: impl Into<gpui_kit::SharedString>) -> gpui_kit::Div {
    div()
        .text_size(SMALL_TEXT)
        .text_color(rgb(pal().muted))
        .child(text.into())
}
pub fn title(record: &Record) -> String {
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
        RecordData::WritingSetup(_) => "Writing setup".into(),
        RecordData::ChatControl(_) => "Chat controls".into(),
        RecordData::Project(p) => p.title.clone(),
    }
}
pub fn short(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let prefix: String = text.chars().take(70).collect();
    if text.chars().count() > 70 {
        format!("{prefix}…")
    } else {
        prefix
    }
}

pub struct WritingPanel {
    pub setup_open: bool,
    pub setup: Option<setup::SetupDraft>,
    pub setup_brief: Entity<TextareaState>,
    pub reference_query: Entity<InputState>,
    pub note_editor: Entity<EditorState>,
    pub note_read_view: Entity<TextViewState>,
    pub read_marks_key: String,
    pub read_source: String,
    pub comment_marks: Option<RangeDecorationCollection>,
    pub diff_editor: Entity<EditorState>,
    pub diff_marks: Option<TextDecorationCollection>,
    pub current_view: Entity<TextViewState>,
    pub candidate_view: Entity<TextViewState>,
    pub review_editor: Entity<EditorState>,
    pub note: NoteLifecycle,
    pub review: ReviewLifecycle,
    /// The comments margin beside the text (Wide, Narrow): `None` follows the
    /// note (shown with comments or a draft); the toggle sets it.
    pub margin: Option<bool>,
    /// The in-tile comments section (Stacked), collapsed until opened.
    pub comments_expanded: bool,
    /// Keeps the comments list's scroll position while it is collapsed.
    pub comments_scroll: gpui_kit::ScrollHandle,
    /// Where comments sit for the current window and layout.
    pub margin_mode: layout::MarginMode,
    /// The Note toolbar's arrangement for the Note panel's current width.
    pub note_fit: layout::NoteFit,
    /// The review bar's arrangement for the Note panel's current width.
    pub review_fit: layout::ReviewFit,
    pub show_coverage: bool,
    pub selection: Option<(Range<usize>, String)>,
    pub general_feedback: Entity<TextareaState>,
    pub general_drafts: BTreeMap<String, String>,
    pub comment_input: Entity<EditorState>,
    pub comment_anchor: Option<(String, u64, Range<usize>, String)>,
    pub comment_operation: Option<String>,
    pub comment_drafts: BTreeMap<String, CommentDraft>,
    pub excluded_comments: BTreeSet<String>,
    pub expanded_requests: BTreeSet<String>,
    pub applied_rewrite: BTreeMap<String, (String, String)>,
}

/// A saved-request failure before run selection or any durable run write.
#[derive(Debug)]
pub enum RewriteStartError {
    Context(AppError),
    StaleInputs,
}

impl WritingPanel {
    /// Build the shared start request from this Note's exact saved inputs.
    /// The host owns preparation/worker dispatch; current Chat and display
    /// caches cannot authorize or retarget a saved retry.
    pub fn rewrite_run_start(
        &self,
        workspace: &Workspace,
        message: &str,
        displayed_scope: Option<&str>,
        offered_copilot_models: Vec<String>,
    ) -> std::result::Result<ThreadRunStart, RewriteStartError> {
        let note = self.open_note_id().ok_or_else(|| {
            RewriteStartError::Context(AppError::Invalid("Working Note is unavailable".into()))
        })?;
        let snapshot = workspace
            .writing_request_in_context(message, note, displayed_scope)
            .map_err(RewriteStartError::Context)?;
        let changed = snapshot.current_inputs().iter().any(|input| {
            workspace
                .store
                .record(&input.record)
                .ok()
                .flatten()
                .is_none_or(|record| record.archived || record.version != input.version)
        });
        if changed {
            return Err(RewriteStartError::StaleInputs);
        }
        Ok(ThreadRunStart {
            thread: snapshot.thread,
            prompt: "Rewrite the saved document using the captured feedback and selected chat; prepare a whole-note proposal for review.".into(),
            request_message: None,
            queue_claim: None,
            owner_request: Some(OwnerRequest::RewriteNote {
                request_message: message.into(),
            }),
            sources: vec![],
            max_tool_rounds: None,
            trace: TraceLevel::Off,
            offered_copilot_models: Some(offered_copilot_models),
        })
    }
    pub(crate) fn note_asset_chat(&self) -> Option<&str> {
        self.note
            .as_ref()
            .and_then(|open| match &open.record().data {
                RecordData::Note(note) => note.chat.as_deref(),
                _ => None,
            })
    }
    pub fn open_note_id(&self) -> Option<&str> {
        self.note.as_ref().map(|open| open.record().id.as_str())
    }
    pub fn recovery_blocked(&self, buffers: &[EditSession]) -> bool {
        self.note.as_ref().is_some_and(|open| {
            open.session().is_none() && buffers.iter().any(|s| s.note == open.record().id)
        })
    }
    /// A real editor Change must reach the checked owner even if a newer
    /// display batch has discovered another guard since the keystroke.
    pub fn note_input_active(&self, note_visible: bool) -> bool {
        self.review.is_none() && !self.note.is_reading() && self.note.is_some() && note_visible
    }
    pub fn note_editor_visible(&self, buffers: &[EditSession], note_visible: bool) -> bool {
        self.note_input_active(note_visible) && !self.recovery_blocked(buffers)
    }
    pub fn toggle_comments(&mut self, open: bool) {
        if self.margin_mode == layout::MarginMode::Stacked {
            self.comments_expanded = open;
        } else {
            self.margin = Some(open);
        }
    }
    pub fn new<T: 'static>(window: &mut Window, cx: &mut Context<T>) -> Self {
        let note_editor = cx.new(|cx| components::prose_editor_state(window, cx).default_value(""));
        let note_read_view = cx.new(|cx| {
            TextViewState::markdown("", cx)
                .selectable(true)
                .scrollable(true)
        });
        let diff_editor = cx.new(|cx| components::prose_editor_state(window, cx).default_value(""));
        diff_editor.update(cx, |editor, cx| editor.set_readonly(true, cx));
        let review_editor =
            cx.new(|cx| components::prose_editor_state(window, cx).default_value(""));
        let comment_input = cx.new(|cx| {
            EditorState::new(window, cx)
                .line_number(false)
                .auto_close(false)
                .placeholder("What should change here?")
        });
        // One line when empty, growing to four as the owner writes.
        let general_feedback = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(1, 4)
                .placeholder("Overall feedback: tone, structure, anything for the whole note…")
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
        let setup_brief = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(2, 5)
                .placeholder("Purpose, audience, tone, requirements (optional)")
        });
        let reference_query = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Reference Note title or saved text")
        });
        Self {
            setup_open: false,
            setup: None,
            setup_brief,
            reference_query,
            note_editor,
            note_read_view,
            read_marks_key: String::new(),
            read_source: String::new(),
            comment_marks: None,
            diff_editor,
            diff_marks: None,
            current_view,
            candidate_view,
            review_editor,
            note: NoteLifecycle::default(),
            review: ReviewLifecycle::default(),
            margin: None,
            comments_expanded: false,
            comments_scroll: gpui_kit::ScrollHandle::new(),
            margin_mode: layout::MarginMode::Wide,
            note_fit: layout::NoteFit::default(),
            review_fit: layout::ReviewFit::default(),
            show_coverage: false,
            selection: None,
            general_feedback,
            general_drafts: BTreeMap::new(),
            comment_input,
            comment_anchor: None,
            comment_operation: None,
            comment_drafts: BTreeMap::new(),
            excluded_comments: BTreeSet::new(),
            expanded_requests: BTreeSet::new(),
            applied_rewrite: BTreeMap::new(),
        }
    }
}

/// A control's immediate request to the owning window. Canonical actions stay
/// in that window's existing checked handlers; this is not a durable command.
#[derive(Clone)]
pub enum WritingRequest {
    OpenSetup,
    RecheckSetup,
    ReloadSetup,
    CloseSetup,
    SaveSetup,
    SearchReferences(Option<String>),
    AttachReference {
        note: String,
        purpose: WritingPurpose,
    },
    ReferencePurpose {
        note: String,
        purpose: WritingPurpose,
    },
    ReferenceCurrent(String),
    RemoveReference(String),
    OpenReference(String),
    ImportReference(WritingPurpose),
    RewriteConfigured(bool),
    Noop,
    Save,
    Reload,
    SaveCopy,
    CommentSelection,
    CommentOrSave,
    Rewrite,
    EditNote,
    Connections,
    Compare,
    ToggleProtection,
    Export,
    ToggleCoverage,
    NewNote,
    NewWritingThread,
    ConfirmDiscard,
    CompareOpenEdit,
    ConfirmDiscardOpenEdit,
    AskSelection,
    TrackSelection,
    ToggleComments,
    CollapseComments,
    CancelComment,
    PostComment,
    ToggleReviewInputs,
    StartReviewEdit,
    KeepReview,
    DismissReview,
    CancelReviewEdit,
    ApplyReview,
    PrepareReviewSelection,
    PrepareReviewEdit,
    Cancel,
    OpenNote {
        id: String,
        recovery: Option<EditSession>,
    },
    ConfirmAddressed {
        request: String,
        operation: String,
        comments: Vec<String>,
    },
    ToggleExcluded(String),
    ToggleRequest(String),
    HideApplied(String),
    ToggleGroup(String),
    Undo(String),
    Retry(String),
    SelectReviewNote(String),
    Reveal(Range<usize>),
    OpenReview(Prepared),
    CardDismiss(Prepared),
    ReviewReading(bool),
    NoteReading(bool),
    Format(crate::formatting::Format),
    StopSaved(String),
}

/// A caller-scoped image map, using only the host's already retained bytes.
pub type WritingAssets = Arc<BTreeMap<String, Arc<Image>>>;

pub type WritingDispatch<T> = fn(&mut T, WritingRequest, &mut Window, &mut Context<T>);

/// Display identity only. The worker, cancellation, queues and runtime are host-owned.
pub struct WritingWorker<'a> {
    pub run_id: Option<&'a str>,
}

/// Short-lived view of one concrete panel and the host's checked display batch.
/// No Workspace or cache is copied or retained by the panel.
pub struct WritingView<'a, T: 'static> {
    pub host: &'a T,
    pub displayed_scope: Option<&'a str>,
    pub reference_importing: bool,
    pub panel: &'a WritingPanel,
    pub workspace: &'a Workspace,
    pub records: &'a Vec<Record>,
    pub history: &'a Vec<Receipt>,
    pub candidates: &'a Vec<Prepared>,
    pub buffers: &'a Vec<EditSession>,
    pub writing_threads: &'a BTreeMap<String, String>,
    pub writing_reviews: &'a BTreeMap<String, WritingCandidateReview>,
    pub writing_requests: &'a BTreeMap<String, WritingSnapshot>,
    pub worker: Option<WritingWorker<'a>>,
    pub progress: &'a str,
    pub centered: bool,
    // These are the existing concrete host presentation functions. Construction
    // stores them without scanning records/receipts or resolving any assets.
    pub edit_note_blocked: fn(&T) -> Option<&'static str>,
    pub chat_card_stop: fn(&T) -> Option<String>,
    pub assets_in_chat: fn(&T, Option<&str>) -> WritingAssets,
    pub latest_run:
        for<'records> fn(&'records [Record], &[Receipt], &str) -> Option<&'records Record>,
    pub dispatch: WritingDispatch<T>,
}
impl<T: 'static> std::ops::Deref for WritingView<'_, T> {
    type Target = WritingPanel;
    fn deref(&self) -> &WritingPanel {
        self.panel
    }
}
impl<T: 'static> WritingView<'_, T> {
    pub(crate) fn on<E: ?Sized + 'static>(
        &self,
        cx: &Context<T>,
        request: WritingRequest,
    ) -> impl Fn(&E, &mut Window, &mut App) + 'static + use<T, E> {
        let dispatch = self.dispatch;
        let host = cx.entity().downgrade();
        // Same weak update as the pinned Context::listener, without retaining
        // its opaque Context lifetime in this reusable callback's type.
        move |_: &E, window, cx| {
            let _ = host.update(cx, |host, cx| dispatch(host, request.clone(), window, cx));
        }
    }
    pub(crate) fn collapse_listener(
        &self,
        cx: &Context<T>,
    ) -> impl Fn(&gpui_kit::KeyDownEvent, &mut Window, &mut App) + 'static + use<T> {
        let dispatch = self.dispatch;
        let host = cx.entity().downgrade();
        move |event: &gpui_kit::KeyDownEvent, window, cx| {
            let _ = host.update(cx, |host, cx| {
                if event.keystroke.key == "escape" {
                    dispatch(host, WritingRequest::CollapseComments, window, cx);
                    cx.stop_propagation();
                }
            });
        }
    }
    pub fn open_note_id(&self) -> Option<&str> {
        self.panel.open_note_id()
    }
    pub fn recovery_blocked(&self) -> bool {
        self.panel.recovery_blocked(self.buffers)
    }
    pub(crate) fn edit_note_blocked(&self) -> Option<&'static str> {
        (self.edit_note_blocked)(self.host)
    }
    pub(crate) fn chat_card_stop(&self) -> Option<String> {
        (self.chat_card_stop)(self.host)
    }
    pub fn note_assets(&self) -> Arc<BTreeMap<String, Arc<Image>>> {
        (self.assets_in_chat)(self.host, self.panel.note_asset_chat())
    }
    pub(crate) fn safe_message_for_thread(
        &self,
        id: String,
        text: String,
        thread: Option<&str>,
    ) -> gpui_kit::base::TextView {
        // Keep the same checked-request scope as the former display map. No
        // map is built for unrelated requests, and no unscoped fallback exists.
        let assets = match thread.filter(|thread| {
            self.writing_requests
                .values()
                .any(|snapshot| snapshot.thread == *thread)
        }) {
            Some(thread) => (self.assets_in_chat)(self.host, Some(thread)),
            None => Arc::new(BTreeMap::new()),
        };
        crate::markdown::safe_markdown(
            gpui_kit::base::TextView::markdown(id, text).w_full(),
            assets,
        )
    }
    pub(crate) fn stop_saved_run_button(&self, run: &str, cx: &Context<T>) -> Button {
        button(format!("stop-saved-run-{run}"))
            .label("Stop…")
            .compact()
            .outline()
            .h(components::tokens::COMPACT_CONTROL)
            .tooltip("Stop this reply; asks first")
            .on_click(self.on(cx, WritingRequest::StopSaved(run.into())))
    }
    pub(crate) fn latest_run(&self, thread: &str) -> Option<&Record> {
        (self.latest_run)(self.records, self.history, thread)
    }
}

/// A menu item carries the same concrete request and displayed shortcut.
pub(crate) fn item<T: 'static>(
    target: &gpui_kit::WeakEntity<T>,
    label: impl Into<gpui_kit::SharedString>,
    action: Option<Box<dyn gpui_kit::Action>>,
    disabled: bool,
    checked: bool,
    request: WritingRequest,
    dispatch: WritingDispatch<T>,
) -> gpui_kit::component::menu::PopupMenuItem {
    let target = target.clone();
    let mut item = gpui_kit::component::menu::PopupMenuItem::new(label)
        .disabled(disabled)
        .checked(checked)
        .on_click(move |_, window, cx| {
            let _ = target.update(cx, |host, cx| dispatch(host, request.clone(), window, cx));
        });
    if let Some(action) = action {
        item = item.action(action);
    }
    item
}

pub fn pill(
    id: impl Into<gpui_kit::ElementId>,
    label: impl Into<gpui_kit::SharedString>,
) -> Button {
    button(id)
        .label(label)
        .compact()
        .ghost()
        .h(components::tokens::COMPACT_CONTROL)
}
pub fn scroll(id: impl Into<gpui_kit::ElementId>) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .size_full()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
}

/// A run that ended Failed or Interrupted, as the chat's recorded notice for
/// it (the Blocker attention whose record is this run, the text Needs you
/// shows) in that state's tone. `None` for other states, or when the chat
/// holds no such notice; the caller then keeps its own error text. The
/// run's progress is the agent's own summary, never the failure.
pub fn terminal_notice(run: &Run, run_id: &str, attention: &[Attention]) -> Option<(Tone, String)> {
    let tone = match run.state {
        RunState::Failed => Tone::Error,
        RunState::Interrupted => Tone::Warning,
        _ => return None,
    };
    attention
        .iter()
        .rev()
        .find(|item| item.kind == AttentionKind::Blocker && item.record.as_deref() == Some(run_id))
        .map(|item| item.reason.trim())
        .filter(|reason| !reason.is_empty())
        .map(|reason| (tone, reason.to_owned()))
}

pub use crate::components::{IconName, Tone};
pub(crate) use crate::layout::{comment_location, sections, unlocated_reason};

pub use crate::workbench::{RunOutcome, WritingRow};

/// Project only the stored converter and AI identity; historical reports remain unlabelled.
pub fn import_report_details(report: &ImportReport) -> Vec<String> {
    let mut details = Vec::new();
    if let Some(conversion) = &report.conversion {
        details.push(format!(
            "{} · AI conversion: {}",
            match conversion.settings.converter {
                DocumentConverter::Xberg => "Xberg",
                DocumentConverter::XbergBrn => "Xberg + BRN layer",
            },
            if conversion.settings.ai {
                "On (GPT-6 Luna, high)"
            } else {
                "Off"
            },
        ));
    }
    details.extend(report.issues.iter().map(ImportIssue::display));
    details
}
