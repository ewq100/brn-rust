//! The conversation composer (spec §6.1): one rounded field holding the
//! context chip, the + (reference and intake) menu, model and effort, and
//! one primary Send / Queue / Stop control. Every control calls the same
//! checked service path as before; nothing here decides scope or authority.
use super::components::AccessibleDropdownMenu;
use super::shell::{item, one_line, pill};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use super::*;
use gpui_kit::component::input::{self, Textarea};
use gpui_kit::component::{Icon, Sizable, Size};
use gpui_kit::{EntityInputHandler, Focusable};

/// What BRN can use for the current conversation, as its context chip
/// shows it. Built from the same records the run's scope check reads; it
/// never names a narrower scope than retrieval uses (spec §5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ContextScope {
    /// The chip's short label.
    pub label: String,
    /// The chip menu's informational lines, in order.
    pub lines: Vec<String>,
    /// The project whose documents are in scope, for "Project settings…".
    pub project: Option<String>,
}

/// The primary control the composer shows for the current state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PrimaryControl {
    Send,
    Queue,
    SaveQueued,
    Stop,
}

impl ThreadsUi {
    /// The composer's text changed: keep its recovery draft and re-render
    /// only when the field turns empty or non-empty (the primary control
    /// changes between Stop and Queue).
    pub(super) fn composer_changed(&mut self, text: String, cx: &mut Context<Self>) {
        let empty = text.trim().is_empty();
        self.save_composer(self.destination(), text);
        if empty != self.conversation.composer_empty {
            self.conversation.composer_empty = empty;
            cx.notify();
        }
    }
    /// ↵ sends when idle and queues while work runs (CHAT-01). Shift-↵ is
    /// the editor's own newline, and ⌘↵ reaches the window's Ask binding, so
    /// neither submits here. An active IME composition owns ↵: the platform
    /// routes it to the input method first, and this guard keeps a composed
    /// ↵ from ever submitting a half-typed message.
    pub(super) fn composer_enter(
        &mut self,
        shift: bool,
        secondary: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if shift || secondary {
            return;
        }
        let composing = self
            .composer
            .update(cx, |state, cx| state.marked_text_range(window, cx))
            .is_some();
        if composing {
            return;
        }
        self.ask(window, cx);
    }
    /// A local run for the open chat is working in this window.
    pub(super) fn running_here(&self) -> bool {
        self.worker
            .as_ref()
            .and_then(Worker::run_identity)
            .is_some_and(|run| self.chat_thread() == Some(run.thread.as_str()))
    }
    /// The one primary control: Save queued while editing a waiting
    /// message; Stop while this chat's reply runs and the field is empty;
    /// Queue while any work runs (or the queue is paused with items), since
    /// ↵ queues then; otherwise Send.
    pub(super) fn primary_control(&self, empty: bool) -> PrimaryControl {
        let queue = self.queue_state();
        if self.queued_edit_for_current_chat().is_some() {
            PrimaryControl::SaveQueued
        } else if self.running_here() && empty {
            PrimaryControl::Stop
        } else if self.worker.is_some()
            || queue.paused && (queue.pending.is_some() || !queue.queue.is_empty())
        {
            PrimaryControl::Queue
        } else {
            PrimaryControl::Send
        }
    }
    /// The real retrieval scope for the open conversation (spec §5): a
    /// general chat uses the saved library; a project chat uses its project
    /// documents and, when the project uses BRN knowledge, the saved library
    /// too. Chat attachments stay private to the chat. A chat about a
    /// document names it first, without claiming that only it is used.
    pub(super) fn context_scope(&self) -> ContextScope {
        let chat = self.chat_thread();
        // The document the conversation is about: the open document this
        // chat assists, never a private attachment of the chat itself.
        let about = match &self.chat {
            ChatTarget::NewForNote(note) => Some(note.clone()),
            ChatTarget::Thread(id) => self.thread_note(id),
            ChatTarget::New => None,
        };
        let note = about
            .filter(|id| {
                self.layout.primary() == TileKind::Note && self.open_note_id() == Some(id.as_str())
            })
            .and_then(|id| {
                self.records
                    .iter()
                    .find(|record| record.id == id && !record.archived)
                    .map(|record| one_line(&title(record)))
            });
        let project = chat.and_then(|chat| project_for_chat(&self.records, chat));
        let mut lines = Vec::new();
        if let Some(note) = &note {
            lines.push(format!("About {}", short(note)));
        }
        let library = "Your saved library: current notes, including protected ones";
        let label = match project.map(|record| (record, &record.data)) {
            Some((record, RecordData::Project(data))) => {
                let name = one_line(&data.title);
                let documents = project_document_ids(&self.records, record).len();
                lines.push(format!(
                    "Project {}: {documents} document{}",
                    short(&name),
                    if documents == 1 { "" } else { "s" }
                ));
                if data.use_brn_knowledge {
                    lines.push(library.to_owned());
                } else {
                    lines.push(
                        "Saved library: off for this project; only its documents and notes linked here"
                            .to_owned(),
                    );
                }
                if data.use_brn_knowledge {
                    format!("Project {} + library", short(&name))
                } else {
                    format!("Project {}", short(&name))
                }
            }
            _ => {
                lines.push(library.to_owned());
                "Saved library".to_owned()
            }
        };
        if let Some(chat) = chat {
            let attached = self
                .records
                .iter()
                .filter(|record| {
                    !record.archived
                        && matches!(&record.data, RecordData::Note(note) if note.chat.as_deref() == Some(chat))
                })
                .count();
            if attached > 0 {
                lines.push(format!(
                    "{attached} file{} attached to this chat, private until Keep in BRN",
                    if attached == 1 { "" } else { "s" }
                ));
            }
        }
        lines.push("Not used: proposals, scratch, older versions, archived notes".to_owned());
        let maintenance = if !self.settings.delegation_chosen {
            "Maintenance: not chosen yet"
        } else if !self.settings.maintenance {
            "Maintenance: paused"
        } else if self.settings.review_first {
            "Maintenance: review changes first"
        } else {
            "Maintenance: BRN may maintain ordinary notes"
        };
        lines.push(maintenance.to_owned());
        ContextScope {
            label: note.map_or(label, |note| short(&note)),
            lines,
            project: project.map(|record| record.id.clone()),
        }
    }
    /// The context chip: the scope's label; its menu lists the real scope,
    /// references and the knowledge setting, with routes to change them.
    fn context_chip(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let this = cx.entity().downgrade();
        let scope = self.context_scope();
        let announced = format!("Context: {}", scope.label);
        // Beside a document the chip keeps its icon and announced name; its
        // menu still lists the whole scope.
        let narrow = self.chat_narrow;
        button("composer-context")
            .ghost()
            .compact()
            .h(components::tokens::COMPACT_CONTROL)
            .min_w_0()
            .max_w(px(220.))
            .when(narrow, |chip| chip.flex_shrink_0())
            .when(!narrow, |chip| chip.flex_shrink(1.).dropdown_caret(true))
            .icon(IconName::BookOpen)
            .accessibility_label(announced.clone())
            .tooltip(format!("{announced} · what BRN can use here"))
            .when(!narrow, |chip| {
                chip.child(div().min_w_0().truncate().child(scope.label.clone()))
            })
            .accessible_dropdown_menu(move |mut menu, _, _| {
                menu = menu.label("BRN can use");
                for line in &scope.lines {
                    menu = menu.label(line.clone());
                }
                menu = menu.separator();
                if let Some(project) = scope.project.clone() {
                    menu = menu.item(item(
                        &this,
                        "Project settings…",
                        None,
                        false,
                        false,
                        move |view, window, cx| {
                            view.projects_trash = false;
                            view.selected_project = Some(project.clone());
                            view.show_tile(TileKind::Projects, window, cx)
                        },
                    ));
                }
                menu.item(item(
                    &this,
                    "Delegation settings…",
                    None,
                    false,
                    false,
                    |view, window, cx| {
                        view.settings_section = SettingsSection::Delegation;
                        view.show_tile(TileKind::Settings, window, cx);
                    },
                ))
            })
            .into_any_element()
    }
    /// The + menu (§6.1.1): attach a document, add useful information, a
    /// full note, a new document and an approved local tool's exact-input
    /// review. Skills are automatic and `/` commands; web access is the
    /// agent's, so neither has a menu entry here.
    fn plus_menu(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let this = cx.entity().downgrade();
        let busy = self.worker.is_some();
        let tools: Vec<String> = match (&self.chat, &self.mcp_view.status) {
            (ChatTarget::Thread(_), Some(status)) if status.connection_enabled => {
                status.tools.iter().map(|tool| tool.name.clone()).collect()
            }
            _ => Vec::new(),
        };
        components::icon_button("composer-add", IconName::Plus, "Add to this chat")
            .tooltip("Attach a document, add information or start a document")
            .accessible_dropdown_menu(move |mut menu, _, _| {
                menu = menu
                    .item(item(
                        &this,
                        "Attach document…",
                        None,
                        busy,
                        false,
                        |view, window, cx| view.choose_chat_attachment(window, cx),
                    ))
                    .item(item(
                        &this,
                        "Add useful information…",
                        None,
                        busy,
                        false,
                        |view, window, cx| {
                            view.choose_import(ImportIntent::UsefulInformation, None, window, cx)
                        },
                    ))
                    .item(item(
                        &this,
                        "Import full note…",
                        None,
                        busy,
                        false,
                        |view, window, cx| {
                            view.choose_import(ImportIntent::FullNote, None, window, cx)
                        },
                    ))
                    .item(item(
                        &this,
                        "New document",
                        None,
                        false,
                        false,
                        |view, window, cx| view.new_note(window, cx),
                    ));
                if !tools.is_empty() {
                    menu = menu.separator().label("Local tools");
                    for tool in &tools {
                        let name = tool.clone();
                        menu = menu.item(item(
                            &this,
                            format!("Review input for {tool}…"),
                            None,
                            busy,
                            false,
                            move |view, window, cx| view.begin_mcp_input(&name, window, cx),
                        ));
                    }
                }
                menu
            })
            .into_any_element()
    }
    /// Send, Queue, Save queued or Stop, with Stop beside Queue while this
    /// chat's reply runs and the field holds text. One filled control per
    /// panel: an archived chat's Unarchive or a recovery card's Continue
    /// leaves Send outline.
    fn primary_controls(&self, empty: bool, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let control = self.primary_control(empty);
        let archived = self
            .chat_thread()
            .is_some_and(|id| self.thread_resolved(id));
        let mut out = Vec::new();
        let stop = |id: &'static str| {
            button(id)
                .compact()
                .h(components::tokens::COMPACT_CONTROL)
                .icon(IconName::Square)
                .tooltip("Stop the current reply and pause the queue · ⌘.")
                .on_click(cx.listener(|this, _, _, cx| this.cancel(cx)))
        };
        if self.can_stop_and_send() && !empty {
            out.push(
                pill("chat-steer", "Steer")
                    .tooltip("Stop the current reply and start fresh with this message")
                    .on_click(cx.listener(|this, _, window, cx| this.stop_and_send(window, cx)))
                    .into_any_element(),
            );
        }
        if control == PrimaryControl::Stop {
            out.push(
                stop("run-stop")
                    .label("Stop")
                    .primary()
                    .accessibility_label("Stop")
                    .into_any_element(),
            );
            return out;
        }
        if self.running_here() {
            out.push(
                stop("run-stop")
                    .outline()
                    .accessibility_label("Stop")
                    .into_any_element(),
            );
        }
        let send = button("ask-delegate");
        let send = if archived || self.recovery_card_leads() {
            send.outline()
        } else {
            send.primary()
        };
        let (label, tooltip) = match control {
            PrimaryControl::SaveQueued => (
                "Save queued",
                "Save the queued message ↵ · Shift-↵ adds a line",
            ),
            PrimaryControl::Queue => ("Queue", "Add this message to the queue ↵"),
            _ => ("Send", "Send ↵ · Shift-↵ adds a line"),
        };
        out.push(
            send.label(label)
                .compact()
                .h(components::tokens::COMPACT_CONTROL)
                .tooltip(tooltip)
                .on_click(cx.listener(|this, _, window, cx| this.ask(window, cx)))
                .into_any_element(),
        );
        out
    }
    /// The archived chat's one banner, which holds its only Unarchive.
    fn archived_banner_row(&self, chat: &str, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let chat = chat.to_owned();
        div()
            .id("composer-destination")
            .track_focus(&self.archived_banner)
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .rounded(px(theme::metrics::RADIUS_CONTROL))
            .border_1()
            .border_color(rgb(p.hairline))
            .bg(rgb(p.tile))
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .text_color(rgb(p.ink))
            .aria_label("Archived. Unarchive to reply.")
            .child(
                Icon::new(IconName::Archive)
                    .with_size(Size::Small)
                    .flex_shrink_0()
                    .text_color(rgb(p.muted)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child("Archived. Unarchive to reply."),
            )
            .child(
                button("reopen-thread")
                    .label("Unarchive")
                    .compact()
                    .primary()
                    .h(components::tokens::COMPACT_CONTROL)
                    .tooltip("Unarchive this chat. The queue stays paused until you resume it.")
                    .on_click(cx.listener(move |this, _, _, cx| this.reopen_thread(&chat, cx))),
            )
            .test_support()
            .into_any_element()
    }
    /// The composer: archived banner, then one rounded field (editing note,
    /// the multiline editor growing 2–8 rows, and the control row). Drop and
    /// ⌘V of a file attach it to this chat, as before.
    pub(super) fn composer_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let archived = self
            .chat_thread()
            .filter(|id| self.thread_resolved(id))
            .map(str::to_owned);
        let empty = self.composer.read(cx).value().trim().is_empty();
        let editing = self.queued_edit_for_current_chat().is_some();
        let field = div()
            .relative()
            .id("composer-box")
            .flex()
            .flex_col()
            .gap_1()
            .w_full()
            .px_3()
            .pt_2()
            .pb_2()
            .rounded(px(theme::metrics::RADIUS_COMPOSER))
            .border_1()
            .border_color(rgb(p.border))
            .bg(rgb(p.alt))
            .when(editing, |field| {
                field.child(
                    div()
                        .id("composer-editing")
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(p.muted))
                        .aria_label("Editing a queued message")
                        .child(div().flex_1().min_w_0().child("Editing a queued message"))
                        .child(
                            pill("queue-edit-cancel", "Cancel edit")
                                .tooltip("Keep the queued message as it was")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.cancel_queued_edit(window, cx)
                                })),
                        )
                        .test_support(),
                )
            })
            .children(self.slash_menu(cx))
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .on_mouse_down(
                        gpui_kit::MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.close_slash(cx)),
                    )
                    .child(
                        Textarea::new(&self.composer)
                            .appearance(false)
                            .text_size(px(theme::metrics::TEXT_BODY))
                            .aria_label("Ask or delegate"),
                    ),
            )
            .child(
                div()
                    .id("composer-row")
                    .flex()
                    .items_center()
                    .min_w_0()
                    .gap_1()
                    .child(self.plus_menu(cx))
                    .child(self.context_chip(cx))
                    .child(div().flex_1())
                    .child(self.model_control(cx))
                    .children(self.primary_controls(empty, cx))
                    .test_support(),
            )
            .test_support();
        div()
            .id("composer-tile")
            .on_drop(
                cx.listener(|this, paths: &gpui_kit::ExternalPaths, window, cx| {
                    this.attach_chat_path(paths.paths(), window, cx)
                }),
            )
            // The editor binds ↑, ↓, ↵, Tab and Escape to its own actions; the
            // open / list claims them first.
            .capture_action(cx.listener(|this, _: &input::MoveUp, _, cx| {
                if this.slash.menu.is_some() {
                    cx.stop_propagation();
                    this.move_slash(-1, cx);
                }
            }))
            .capture_action(cx.listener(|this, _: &input::MoveDown, _, cx| {
                if this.slash.menu.is_some() {
                    cx.stop_propagation();
                    this.move_slash(1, cx);
                }
            }))
            // ↵ chooses; Shift-↵ stays a newline and an active IME
            // composition keeps ↵.
            .capture_action(cx.listener(|this, action: &input::Enter, window, cx| {
                if this.slash.menu.is_some()
                    && !action.shift
                    && !action.secondary
                    && this
                        .composer
                        .update(cx, |state, cx| state.marked_text_range(window, cx))
                        .is_none()
                {
                    cx.stop_propagation();
                    this.choose_slash(None, window, cx);
                }
            }))
            .capture_action(cx.listener(|this, _: &input::MoveLeft, _, cx| this.close_slash(cx)))
            .capture_action(cx.listener(|this, _: &input::MoveRight, _, cx| this.close_slash(cx)))
            .capture_action(cx.listener(|this, _: &input::IndentInline, window, cx| {
                if this.slash.menu.is_some() {
                    cx.stop_propagation();
                    this.choose_slash(None, window, cx);
                }
            }))
            .capture_action(cx.listener(|this, _: &input::Escape, _, cx| {
                if this.slash.menu.is_some() {
                    cx.stop_propagation();
                    this.dismiss_slash(cx);
                }
            }))
            .capture_key_down(
                cx.listener(|this, event: &gpui_kit::KeyDownEvent, window, cx| {
                    let modifiers = &event.keystroke.modifiers;
                    // Plain ↵ submits here, before the editor sees it, so
                    // the field never gains the newline ↵ would insert.
                    if event.keystroke.key == "enter"
                        && !modifiers.shift
                        && !modifiers.platform
                        && !modifiers.control
                        && !modifiers.alt
                        && this.composer.focus_handle(cx).is_focused(window)
                    {
                        cx.stop_propagation();
                        this.composer_enter(false, false, window, cx);
                        return;
                    }
                    if event.keystroke.key == "v"
                        && event.keystroke.modifiers.platform
                        && let Some(item) = cx.read_from_clipboard()
                    {
                        let paths = item
                            .entries()
                            .iter()
                            .filter_map(|entry| match entry {
                                gpui_kit::ClipboardEntry::ExternalPaths(paths) => {
                                    Some(paths.paths())
                                }
                                _ => None,
                            })
                            .flatten()
                            .cloned()
                            .collect::<Vec<_>>();
                        if !paths.is_empty() {
                            cx.stop_propagation();
                            this.attach_chat_path(&paths, window, cx);
                        }
                    }
                }),
            )
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_2()
            .w_full()
            .min_w_0()
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .when_some(archived, |composer, chat| {
                composer.child(self.archived_banner_row(&chat, cx))
            })
            .child(field)
            .test_support()
            .into_any_element()
    }
    /// Leave a queued edit: the waiting message keeps its saved text and the
    /// composer gets back the draft it held before Edit.
    pub(super) fn cancel_queued_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(thread) = self.chat_thread().map(str::to_owned) {
            self.editing_queue.retain(|edit| edit.thread != thread);
        }
        let text = self.conversation.take_stash(&self.destination());
        self.composer
            .update(cx, |editor, cx| editor.set_value(text.clone(), window, cx));
        self.save_composer(self.destination(), text);
        cx.notify();
    }
}
