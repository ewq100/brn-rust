//! The conversation composer (spec §6.1): one rounded field holding the
//! context chip, the + (reference and intake) menu, model and effort, and
//! one primary Send / Queue / Stop control. Every control calls the same
//! checked service path as before; nothing here decides scope or authority.
use super::components::AccessibleDropdownMenu;
use super::shell::{item, one_line, pill};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use super::skills_view::SkillAction;
use super::*;
use gpui_kit::component::input::Textarea;
use gpui_kit::component::{Icon, Sizable, Size};
use gpui_kit::{EntityInputHandler, Focusable, WeakEntity};

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
    /// The + menu: attach a document, add useful information, a full note,
    /// Search the web…, an approved local tool's exact-input review and the
    /// skill choice (which was a permanent pill row).
    fn plus_menu(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let this = cx.entity().downgrade();
        let busy = self.worker.is_some();
        let tools: Vec<String> = match (&self.chat, &self.mcp_view.status) {
            (ChatTarget::Thread(_), Some(status)) if status.connection_enabled => {
                status.tools.iter().map(|tool| tool.name.clone()).collect()
            }
            _ => Vec::new(),
        };
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let skills = self.skill_choices();
        components::icon_button("composer-add", IconName::Plus, "Add to this chat")
            .tooltip("Attach a document, add information, search the web or choose a skill")
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
                    .separator()
                    .item(item(
                        &this,
                        "Search the web…",
                        None,
                        false,
                        false,
                        |view, window, cx| view.open_web_search(window, cx),
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
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                {
                    menu = skills.menu(menu.separator(), &this);
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
    /// Model and effort from the configured capabilities (no invented list,
    /// no silent fallback). Below 420 pt the model control also names the
    /// effort and its menu holds the effort choices.
    fn model_controls(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let this = cx.entity().downgrade();
        let model_this = this.clone();
        let model = self.settings.model.clone();
        let effort = self.settings.effort.clone();
        if self.chat_narrow {
            return vec![
                button("model-pill")
                    .compact()
                    .ghost()
                    .h(components::tokens::COMPACT_CONTROL)
                    .flex_shrink(1.)
                    .min_w_0()
                    .max_w(px(140.))
                    .dropdown_caret(true)
                    .accessibility_label(format!("Model {model}, effort {effort}"))
                    .tooltip(
                        "Provider, model and reasoning effort · changes Settings · no fallback",
                    )
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            .child(div().min_w_0().truncate().child(model))
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .whitespace_nowrap()
                                    .child(format!("\u{a0}· {effort}")),
                            ),
                    )
                    .accessible_dropdown_menu(move |menu, _, cx| match model_this.upgrade() {
                        Some(view) => {
                            let view = view.read(cx);
                            view.effort_section(
                                view.model_menu(menu, &model_this).separator(),
                                &model_this,
                            )
                        }
                        None => menu,
                    })
                    .into_any_element(),
            ];
        }
        let effort_this = this;
        vec![
            pill(
                "model-pill",
                super::shell::model_display(&self.settings.provider, &model),
            )
            .min_w_0()
            .flex_shrink(1.)
            .dropdown_caret(true)
            .tooltip("Provider and model · changes Settings · no fallback")
            .accessible_dropdown_menu(move |menu, _, cx| match model_this.upgrade() {
                Some(view) => view.read(cx).model_menu(menu, &model_this),
                None => menu,
            })
            .into_any_element(),
            pill("effort-pill", effort)
                .dropdown_caret(true)
                .tooltip("Reasoning effort")
                .accessible_dropdown_menu(move |menu, _, cx| match effort_this.upgrade() {
                    Some(view) => view.read(cx).effort_menu(menu, &effort_this),
                    None => menu,
                })
                .into_any_element(),
        ]
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
        let skill = self.selected_skill_label();
        let field = div()
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
            .child(
                div().w_full().min_w_0().child(
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
                    .children(skill.map(|label| self.skill_chip(label, cx)))
                    .child(div().flex_1())
                    .children(self.model_controls(cx))
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
            .capture_key_down(
                cx.listener(|this, event: &gpui_kit::KeyDownEvent, window, cx| {
                    // Plain ↵ submits here, before the editor sees it, so
                    // the field never gains the newline ↵ would insert.
                    let modifiers = &event.keystroke.modifiers;
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
    /// The selected skill's chip, with a Clear. Choosing happens in the +
    /// menu; the chip keeps the choice visible beside the context.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn skill_chip(&self, label: String, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let project = self.skill_project();
        pill("composer-skill-selected", format!("Skill · {label}"))
            .icon(IconName::Close)
            .accessibility_label(format!("Skill {label}, clear"))
            .tooltip("Clear the selected skill")
            .min_w_0()
            .flex_shrink(1.)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.skill_action(SkillAction::Clear, "", "", project.clone(), cx)
            }))
            .into_any_element()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn skill_chip(&self, label: String, _: &mut Context<Self>) -> gpui_kit::AnyElement {
        components::status_chip("composer-skill-selected", None, None, label).into_any_element()
    }
    /// The selected skill for this scope, when its view is current.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn selected_skill_label(&self) -> Option<String> {
        if self.skill_view.project != self.skill_project() {
            return None;
        }
        self.skill_view
            .rows
            .iter()
            .find(|skill| skill.selected && skill.enabled)
            .map(|skill| {
                if skill.available {
                    skill.id.clone()
                } else {
                    format!("{} · unavailable", skill.id)
                }
            })
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn selected_skill_label(&self) -> Option<String> {
        None
    }
    /// The skill section of the + menu, captured at render.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn skill_choices(&self) -> SkillChoices {
        let project = self.skill_project();
        let current = self.skill_view.project == project;
        SkillChoices {
            project,
            rows: if current {
                self.skill_view
                    .rows
                    .iter()
                    .filter(|skill| skill.enabled)
                    .map(|skill| SkillChoice {
                        id: skill.id.clone(),
                        digest: skill.digest.clone(),
                        selected: skill.selected,
                        available: skill.available,
                    })
                    .collect()
            } else {
                Vec::new()
            },
            current,
        }
    }
    /// Leave a queued edit: the waiting message keeps its saved text and the
    /// composer gets back the draft it held before Edit.
    pub(super) fn cancel_queued_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_queue = None;
        let text = self.conversation.take_stash(&self.destination());
        self.composer
            .update(cx, |editor, cx| editor.set_value(text.clone(), window, cx));
        self.save_composer(self.destination(), text);
        cx.notify();
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
struct SkillChoice {
    id: String,
    digest: String,
    selected: bool,
    available: bool,
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
struct SkillChoices {
    project: Option<String>,
    rows: Vec<SkillChoice>,
    current: bool,
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
impl SkillChoices {
    fn menu(&self, menu: PopupMenu, this: &WeakEntity<ThreadsUi>) -> PopupMenu {
        let mut menu = menu.label("Skill");
        if self.current {
            for skill in &self.rows {
                let (id, digest, scope) =
                    (skill.id.clone(), skill.digest.clone(), self.project.clone());
                let label = if skill.available {
                    skill.id.clone()
                } else {
                    format!("{} · unavailable", skill.id)
                };
                menu = menu.item(item(
                    this,
                    label,
                    None,
                    !skill.available,
                    skill.selected,
                    move |view, _, cx| {
                        view.skill_action(SkillAction::Select, &id, &digest, scope.clone(), cx)
                    },
                ));
            }
            let scope = self.project.clone();
            menu = menu.item(item(
                this,
                "No skill",
                None,
                false,
                !self.rows.iter().any(|skill| skill.selected),
                move |view, _, cx| view.skill_action(SkillAction::Clear, "", "", scope.clone(), cx),
            ));
        }
        menu.item(item(
            this,
            "Refresh skills",
            None,
            false,
            false,
            |view, _, cx| {
                view.force_refresh_skills();
                cx.notify();
            },
        ))
    }
}
