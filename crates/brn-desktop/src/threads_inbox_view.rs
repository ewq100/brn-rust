//! Inbox: For you (reviews, decisions, failures and waiting work) and
//! Imports (file intake with an explicit intent, the manual folder inbox and
//! recent imports). Presentation only: every action calls the existing
//! checked services, and nothing here schedules, watches or approves.
use super::components::Tone;
use super::*;
use gpui_kit::SharedString;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable as _, Size};

pub(super) use super::tiles::{PAGE_WIDTH, column};

/// The Inbox's session-only state.
#[derive(Default)]
pub(super) struct InboxPage {
    /// The latest file import started from Imports, with its outcome.
    pub(super) attempt: Option<ImportAttempt>,
    /// The task whose detail panel is open in Tasks.
    pub(super) task: Option<super::tasks_view::TaskDetail>,
}

/// One file import started from Imports. Kept so a failure keeps its
/// filename, intent and destination, and offers Retry with the same choice.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ImportAttempt {
    pub(super) path: PathBuf,
    pub(super) title: String,
    pub(super) intent: ImportIntent,
    /// Where the result goes, as shown before the import started.
    pub(super) destination: String,
    /// The Project a useful-information import was scoped to when it
    /// started (None: General). Retry reuses it instead of the open chat.
    pub(super) project: Option<String>,
    pub(super) state: AttemptState,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) enum AttemptState {
    Running,
    /// The service's reported outcome for this import.
    Finished(Tone, String),
    Failed(String),
    Stopped(String),
}

/// Why a For you item needs the owner. Only the first three are counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Need {
    Review,
    Decision,
    Failed,
    Waiting,
}
impl Need {
    pub(super) const ALL: [Self; 4] = [Self::Review, Self::Decision, Self::Failed, Self::Waiting];
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Review => "Needs your review",
            Self::Decision => "Needs your decision",
            Self::Failed => "Failed",
            Self::Waiting => "Waiting on someone else",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Review => "review",
            Self::Decision => "decision",
            Self::Failed => "failed",
            Self::Waiting => "waiting",
        }
    }
    fn tone(self) -> Option<Tone> {
        match self {
            Self::Review => Some(Tone::Info),
            Self::Decision => Some(Tone::Warning),
            Self::Failed => Some(Tone::Error),
            Self::Waiting => None,
        }
    }
    fn icon(self) -> IconName {
        match self {
            Self::Review => IconName::GitPullRequest,
            Self::Decision => IconName::CircleAlert,
            Self::Failed => IconName::CircleX,
            Self::Waiting => IconName::Hourglass,
        }
    }
    /// Waiting work is not the owner's next action, so it is not counted.
    pub(super) fn counted(self) -> bool {
        self != Self::Waiting
    }
}

/// Where a For you item opens: its exact originating context.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Target {
    Chat(String),
    /// A prepared proposal, opened in its document review.
    Review(String),
    /// A kept edit, by its position in the workspace's edit sessions.
    Recover(usize),
    Task(String),
    Imports,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ForYouItem {
    /// Stable within one render; used for element IDs.
    pub(super) key: String,
    pub(super) need: Need,
    /// The short state word on the row ("Failed", "Stopped", …).
    pub(super) state: &'static str,
    pub(super) title: String,
    /// Where it came from, in owner terms.
    pub(super) context: String,
    /// The originating chat, when the item has one besides its target.
    pub(super) chat: Option<String>,
    pub(super) target: Target,
    pub(super) protected: bool,
}

/// Every For you item, grouped Review, Decision, Failed, Waiting and kept in
/// source order within each group. Pure, so classification is testable.
pub(super) fn for_you_items(
    records: &[Record],
    rows: &[ThreadRow],
    candidates: &[Prepared],
    buffers: &[EditSession],
    inbox: Option<&InboxState>,
) -> Vec<ForYouItem> {
    let mut items = Vec::new();
    let mut shown = BTreeSet::new();
    for row in rows.iter().filter(|row| row.needs_you()) {
        let Some(RecordData::Thread(thread)) = records
            .iter()
            .find(|record| record.id == row.id)
            .map(|record| &record.data)
        else {
            continue;
        };
        let chat = one_line(&row.title);
        for (index, attention) in thread.attention.iter().enumerate() {
            let key = format!("chat-{}-{index}", row.id);
            let candidate = (attention.kind == AttentionKind::Review)
                .then(|| {
                    candidates.iter().find(|candidate| {
                        attention.record.as_deref() == Some(candidate.operation.as_str())
                    })
                })
                .flatten();
            let item = if let Some(candidate) = candidate {
                shown.insert(candidate.operation.as_str().to_owned());
                let summary = review_summary(candidate, records);
                ForYouItem {
                    key: format!("review-{}", candidate.operation.as_str()),
                    need: Need::Review,
                    state: "Review",
                    title: review_label(&summary, &candidate.request.reason),
                    context: format!("Proposed in chat · {chat}"),
                    chat: Some(row.id.clone()),
                    target: Target::Review(candidate.operation.as_str().to_owned()),
                    protected: summary.protected,
                }
            } else {
                let (need, state) = match attention.kind {
                    AttentionKind::Review => (Need::Review, "Review"),
                    AttentionKind::Blocker
                        if brn_ai::threads::is_terminal_notice(&attention.reason) =>
                    {
                        if attention.reason.starts_with("Run failed") {
                            (Need::Failed, "Failed")
                        } else {
                            (Need::Failed, "Stopped")
                        }
                    }
                    AttentionKind::Question => (Need::Decision, "Question"),
                    AttentionKind::Conflict => (Need::Decision, "Conflict"),
                    AttentionKind::Blocker => (Need::Decision, "Blocked"),
                };
                // The chat's title leads; the reason is the second line.
                ForYouItem {
                    key,
                    need,
                    state,
                    title: chat.clone(),
                    context: format!(
                        "{}{}",
                        one_line(&attention.reason),
                        if row.resolved {
                            " · archived chat"
                        } else {
                            ""
                        }
                    ),
                    chat: None,
                    target: Target::Chat(row.id.clone()),
                    protected: false,
                }
            };
            items.push(item);
        }
    }
    for candidate in candidates {
        if shown.contains(candidate.operation.as_str()) {
            continue;
        }
        let summary = review_summary(candidate, records);
        items.push(ForYouItem {
            key: format!("review-{}", candidate.operation.as_str()),
            need: Need::Review,
            state: "Review",
            title: review_label(&summary, &candidate.request.reason),
            context: "Proposed change · saved notes unchanged until you apply it".into(),
            chat: None,
            target: Target::Review(candidate.operation.as_str().to_owned()),
            protected: summary.protected,
        });
    }
    for (index, buffer) in buffers.iter().enumerate() {
        let note = records
            .iter()
            .find(|record| record.id == buffer.note)
            .map(title)
            .unwrap_or_else(|| "Unavailable note".into());
        items.push(ForYouItem {
            key: format!("recover-{index}"),
            need: Need::Decision,
            state: "Unsaved",
            title: one_line(&note),
            context: "Recover edit · kept apart from the saved note until you save or discard it"
                .into(),
            chat: None,
            target: Target::Recover(index),
            protected: false,
        });
    }
    if let Some(inbox) = inbox {
        for (index, entry) in inbox.entries.iter().enumerate() {
            if entry.status == InboxStatus::Failed {
                items.push(ForYouItem {
                    key: format!("import-{index}"),
                    need: Need::Failed,
                    state: "Failed",
                    title: format!("{} wasn't imported", entry.name),
                    context: format!("Folder inbox → General knowledge · {}", entry.detail),
                    chat: None,
                    target: Target::Imports,
                    protected: false,
                });
            }
        }
    }
    for record in records.iter().filter(|record| !record.archived) {
        if let RecordData::Action(action) = &record.data
            && action.state == ActionState::Waiting
        {
            items.push(ForYouItem {
                key: format!("task-{}", record.id),
                need: Need::Waiting,
                state: "Waiting",
                title: one_line(&action.description),
                context: match &action.waiting_on {
                    Some(who) => format!("Task · waiting on {}", one_line(who)),
                    None => "Task · waiting".into(),
                },
                chat: None,
                target: Target::Task(record.id.clone()),
                protected: false,
            });
        }
    }
    // Stable: keeps source order inside each group.
    items.sort_by_key(|item| item.need);
    items
}

/// One line of owner text: whitespace collapsed.
pub(super) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A page column inside the workspace's one scroller: at most `width` wide,
/// centred, with the page inset (compact below 720 pt of primary width).
pub(super) fn page_frame(
    id: impl Into<gpui_kit::ElementId>,
    width: f32,
    compact: bool,
    column: gpui_kit::Div,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let inset = if compact {
        theme::metrics::COMPACT_PAGE_INSET
    } else {
        theme::metrics::PAGE_INSET
    };
    scroll(id)
        .gap_0()
        .items_center()
        .px(px(inset))
        .pt(components::tokens::SPACE_4)
        .pb(components::tokens::SPACE_8)
        .child(column.w_full().max_w(px(width)))
}

/// A view's lead line: what it holds, in one quiet sentence.
pub(super) fn lead(id: &'static str, text: &'static str) -> gpui_kit::AnyElement {
    div()
        .id(id)
        .pb(components::tokens::SPACE_2)
        .text_size(px(theme::metrics::TEXT_BODY))
        .text_color(rgb(pal().muted))
        .aria_label(text)
        .child(text)
        .test_support()
        .into_any_element()
}

/// A page group: its section label, then content, divided from the group
/// above by whitespace only.
pub(super) fn page_group(id: impl Into<gpui_kit::ElementId>) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .gap(px(2.))
        .min_w_0()
        .pb(components::tokens::SPACE_4)
}

/// A group heading at the section size, with an optional count.
pub(super) fn group_heading(label: &str, count: Option<usize>) -> gpui_kit::Div {
    let text = match count {
        Some(count) => format!("{label} · {count}"),
        None => label.to_owned(),
    };
    div()
        .flex()
        .items_center()
        .gap_2()
        .pt(components::tokens::SPACE_4)
        .pb(components::tokens::SPACE_2)
        .text_size(px(theme::metrics::TEXT_BODY))
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .text_color(rgb(pal().ink))
        .child(div().flex_1().min_w_0().truncate().child(text))
}

/// A two-line item: state icon, title, where it came from, the state chip
/// and (when it opens something) a chevron.
fn item_content(
    id: &SharedString,
    icon: IconName,
    tone: Option<Tone>,
    title: SharedString,
    context: SharedString,
    state: SharedString,
    opens: bool,
) -> gpui_kit::Div {
    let p = pal();
    let icon_color = tone.map_or(p.muted, Tone::color);
    div()
        .flex()
        .flex_1()
        .min_w_0()
        .items_center()
        .gap_3()
        .child(
            Icon::new(icon)
                .with_size(Size::Small)
                .flex_shrink_0()
                .text_color(rgb(icon_color)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(2.))
                .child(
                    div()
                        .id(SharedString::from(format!("{id}-title")))
                        .min_w_0()
                        .truncate()
                        .text_size(px(theme::metrics::TEXT_BODY))
                        .font_weight(gpui_kit::FontWeight::MEDIUM)
                        .text_color(rgb(p.ink))
                        .child(title)
                        .test_support(),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("{id}-context")))
                        .min_w_0()
                        .truncate()
                        .text_size(px(theme::metrics::TEXT_LABEL))
                        .text_color(rgb(p.muted))
                        .child(context)
                        .test_support(),
                ),
        )
        .when(!state.is_empty(), |line| {
            line.child(components::status_chip(
                SharedString::from(format!("{id}-state")),
                tone,
                None,
                state,
            ))
        })
        .when(opens, |line| {
            line.child(
                Icon::new(IconName::ChevronRight)
                    .with_size(Size::Small)
                    .flex_shrink_0()
                    .text_color(rgb(p.muted)),
            )
        })
}

/// A keyboard-reachable item row. A Kit button, so Tab, ↵ and Space reach
/// it; the accessible name carries every visible part.
pub(super) fn item_row(
    id: impl Into<SharedString>,
    icon: IconName,
    tone: Option<Tone>,
    title: impl Into<SharedString>,
    context: impl Into<SharedString>,
    state: impl Into<SharedString>,
) -> gpui_kit::component::button::Button {
    let id: SharedString = id.into();
    let (title, context, state) = (title.into(), context.into(), state.into());
    let announced = components::row_text(&title, &[&context, &state]);
    button(id.clone())
        .ghost()
        .w_full()
        .h(px(56.))
        .px_2()
        .rounded(px(theme::metrics::RADIUS_CONTROL))
        .accessibility_label(announced.clone())
        .tooltip(announced)
        .child(item_content(&id, icon, tone, title, context, state, true))
}

/// The same item when it opens nothing: static text, not a focus stop.
pub(super) fn item_line(
    id: impl Into<SharedString>,
    icon: IconName,
    tone: Option<Tone>,
    title: impl Into<SharedString>,
    context: impl Into<SharedString>,
    state: impl Into<SharedString>,
) -> gpui_kit::AnyElement {
    let id: SharedString = id.into();
    let (title, context, state) = (title.into(), context.into(), state.into());
    let announced = components::row_text(&title, &[&context, &state]);
    div()
        .id(id.clone())
        .flex()
        .items_center()
        .h(px(56.))
        .px_2()
        .min_w_0()
        .aria_label(announced.clone())
        .tooltip(move |window, cx| Tooltip::new(announced.clone()).build(window, cx))
        .child(item_content(&id, icon, tone, title, context, state, false))
        .test_support()
        .into_any_element()
}

impl ThreadsUi {
    pub(super) fn inbox_compact(&self) -> bool {
        self.arrangement.primary < 720.
    }
    /// For you, as rendered now.
    pub(super) fn for_you(&self) -> Vec<ForYouItem> {
        let inbox = self.workspace.inbox().ok().flatten();
        for_you_items(
            &self.records,
            &self.thread_rows,
            &self.candidates,
            &self.buffers,
            inbox.as_ref(),
        )
    }
    /// The Inbox count: actionable For you items only.
    pub(super) fn for_you_count(&self) -> usize {
        self.for_you()
            .iter()
            .filter(|item| item.need.counted())
            .count()
    }
    fn open_for_you(&mut self, target: &Target, window: &mut Window, cx: &mut Context<Self>) {
        match target {
            Target::Chat(id) => self.open_thread(id, window, cx),
            Target::Review(operation) => {
                match self
                    .candidates
                    .iter()
                    .find(|candidate| candidate.operation.as_str() == operation)
                    .cloned()
                {
                    Some(candidate) => self.open_review(candidate, window, cx),
                    None => {
                        self.warn("This proposal is no longer waiting for review. Recent changes shows what happened to it.");
                        cx.notify();
                    }
                }
            }
            Target::Recover(index) => match self.buffers.get(*index).cloned() {
                Some(buffer) => {
                    let note = buffer.note.clone();
                    self.open_note(&note, Some(buffer), window, cx)
                }
                None => {
                    self.warn("This kept edit was already saved or discarded.");
                    cx.notify();
                }
            },
            Target::Task(id) => {
                self.show_tile(TileKind::Actions, window, cx);
                self.open_task(id, window, cx);
            }
            Target::Imports => self.show_tile(TileKind::Imports, window, cx),
        }
    }
    /// Inbox → For you. Reviews, decisions and failures that need the owner,
    /// then work waiting on someone else. Each row opens where it started.
    pub(super) fn overview_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let items = self.for_you();
        let mut view = column().child(lead(
            "for-you-lead",
            "Reviews, decisions and failures that need you. Each opens where it started.",
        ));
        // The first-use delegation choice is a decision only the owner makes.
        let delegation = self.delegation_host() == Some(TileKind::Overview);
        if delegation {
            view = view.child(
                div()
                    .id("for-you-delegation")
                    .pt(components::tokens::SPACE_2)
                    .pb(components::tokens::SPACE_4)
                    .child(self.working_knowledge(cx))
                    .test_support(),
            );
        }
        for need in Need::ALL {
            let group: Vec<&ForYouItem> = items.iter().filter(|item| item.need == need).collect();
            if group.is_empty() {
                continue;
            }
            let mut list = page_group(format!("for-you-{}", need.key()))
                .role(gpui_kit::Role::Group)
                .aria_label(format!("{} · {}", need.label(), group.len()))
                .child(group_heading(need.label(), Some(group.len())));
            for item in group {
                let target = item.target.clone();
                let state = if item.protected {
                    format!("{} · Protected", item.state)
                } else {
                    item.state.to_owned()
                };
                let mut line = div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .min_w_0()
                    .border_b_1()
                    .border_color(rgb(pal().hairline))
                    .child(
                        div().flex_1().min_w_0().child(
                            item_row(
                                format!("for-you-{}", item.key),
                                need.icon(),
                                need.tone(),
                                item.title.clone(),
                                item.context.clone(),
                                state,
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| this.open_for_you(&target, window, cx),
                            )),
                        ),
                    );
                if let Some(chat) = item.chat.clone() {
                    line = line.child(
                        button(format!("for-you-{}-chat", item.key))
                            .label("Open chat")
                            .ghost()
                            .compact()
                            .h(components::tokens::COMPACT_CONTROL)
                            .flex_shrink_0()
                            .accessibility_label(format!("Open chat: {}", item.title))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_thread(&chat, window, cx)
                            })),
                    );
                }
                list = list.child(line);
            }
            view = view.child(list.test_support());
        }
        if items.is_empty() && !delegation {
            let this = cx.entity().downgrade();
            view = view.child(
                div()
                    .pt(components::tokens::SPACE_8)
                    .child(components::empty_state(
                        "for-you-empty",
                        IconName::CheckCheck,
                        "You're all caught up",
                        "Nothing needs your review or a decision. Add something new when you're ready.",
                        Some(
                            button("for-you-capture")
                                .label("Add useful information…")
                                .outline()
                                .disabled(self.worker.is_some())
                                .on_click(move |_, window, cx| {
                                    let _ = this.update(cx, |view, cx| {
                                        view.choose_import_from_inbox(
                                            ImportIntent::UsefulInformation,
                                            window,
                                            cx,
                                        )
                                    });
                                }),
                        ),
                    )),
            );
        }
        // Routine delegated maintenance stays in Recent changes, not here.
        view = view.child(
            div()
                .id("for-you-history")
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .pt(components::tokens::SPACE_6)
                .mt(components::tokens::SPACE_4)
                .border_t_1()
                .border_color(rgb(pal().hairline))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(220.))
                        .text_size(px(theme::metrics::TEXT_LABEL))
                        .text_color(rgb(pal().muted))
                        .child("Routine changes BRN makes for you are listed in Recent changes, each with Undo where it applies."),
                )
                .child(
                    button("inbox-open-history")
                        .label("Recent changes")
                        .icon(IconName::RotateCcwClock)
                        .outline()
                        .compact()
                        .h(components::tokens::COMPACT_CONTROL)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.show_tile(TileKind::History, window, cx)
                        })),
                )
                .test_support(),
        );
        // Knowledge checks moved here with Home's other attention content.
        view = view.child(
            div()
                .pt(components::tokens::SPACE_4)
                .child(self.knowledge_panel(cx)),
        );
        page_frame("home-content", PAGE_WIDTH, self.inbox_compact(), view)
            .test_support()
            .into_any_element()
    }

    /// Inbox → Imports: add a file with an explicit intent, the manual
    /// folder inbox, and what was imported recently.
    pub(super) fn imports_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let view = column()
            .child(lead(
                "imports-lead",
                "Bring documents into BRN. Choose what each import should become.",
            ))
            .child(self.imports_add(cx))
            .child(self.inbox_settings(cx))
            .child(self.recent_imports(cx));
        div()
            .id("imports-drop")
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .on_drop(
                cx.listener(|this, paths: &gpui_kit::ExternalPaths, window, cx| {
                    this.dropped_import(paths.paths(), window, cx)
                }),
            )
            .capture_key_down(
                cx.listener(|this, event: &gpui_kit::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "v"
                        && event.keystroke.modifiers.platform
                        && let Some(item) = cx.read_from_clipboard()
                    {
                        let paths: Vec<PathBuf> = item
                            .entries()
                            .iter()
                            .filter_map(|entry| match entry {
                                gpui_kit::ClipboardEntry::ExternalPaths(paths) => {
                                    Some(paths.paths().to_vec())
                                }
                                _ => None,
                            })
                            .flatten()
                            .collect();
                        if !paths.is_empty() {
                            cx.stop_propagation();
                            this.dropped_import(&paths, window, cx);
                        }
                    }
                }),
            )
            .child(
                page_frame("imports-content", PAGE_WIDTH, self.inbox_compact(), view)
                    .test_support(),
            )
            .into_any_element()
    }

    /// Where an import made now would go, as the service will scope it.
    fn import_destination(&self, intent: ImportIntent) -> String {
        self.import_destination_for(intent, self.import_project(intent).as_deref())
    }

    /// The Project a useful-information import started now is scoped to.
    pub(super) fn import_project(&self, intent: ImportIntent) -> Option<String> {
        (intent == ImportIntent::UsefulInformation)
            .then(|| {
                self.chat_thread()
                    .and_then(|chat| project_for_chat(&self.records, chat))
                    .map(|project| project.id.clone())
            })
            .flatten()
    }

    fn import_destination_for(&self, intent: ImportIntent, project: Option<&str>) -> String {
        match (intent, project) {
            (ImportIntent::FullNote, _) => "General knowledge".into(),
            (_, Some(id)) => match self.records.iter().find(|record| record.id == id) {
                Some(project) => format!("Project {}", title(project)),
                None => "Its original Project (no longer available)".into(),
            },
            (_, None) => "A new general chat".into(),
        }
    }

    fn imports_add(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let busy = self.worker.is_some();
        let importing = self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.kind() == WorkerKind::Import);
        let documents = &self.settings.document_conversion;
        let choice = |id: &'static str,
                      intent: ImportIntent,
                      label: &'static str,
                      explanation: &'static str,
                      primary: bool,
                      cx: &mut Context<Self>| {
            let control = button(id)
                .label(label)
                .disabled(busy)
                .accessibility_label(format!("{label} {explanation}"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.choose_import_from_inbox(intent, window, cx)
                }));
            let control = if primary {
                control.primary()
            } else {
                control.outline()
            };
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_x_3()
                .gap_y_1()
                .child(div().flex_shrink_0().child(control))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(200.))
                        .text_size(px(theme::metrics::TEXT_LABEL))
                        .text_color(rgb(p.muted))
                        .child(explanation),
                )
        };
        let mut card = div()
            .id("imports-add")
            .flex()
            .flex_col()
            .gap_3()
            .p(components::tokens::SPACE_4)
            .rounded(px(theme::metrics::RADIUS_CARD))
            .border_1()
            .border_color(rgb(p.hairline))
            .bg(rgb(p.alt))
            .child(choice(
                "imports-useful",
                ImportIntent::UsefulInformation,
                "Add useful information…",
                "BRN reads the file in a chat and proposes what is worth keeping. A summary is not a complete import.",
                true,
                cx,
            ))
            .child(choice(
                "imports-full",
                ImportIntent::FullNote,
                "Import full note…",
                "Keeps a protected, readable note in General knowledge with its figures and a coverage report.",
                false,
                cx,
            ))
            .child(
                div()
                    .text_size(px(theme::metrics::TEXT_LABEL))
                    .text_color(rgb(p.muted))
                    .child("You can also drop a file here, or paste one with ⌘V, and choose which of the two it becomes."),
            )
            .child(
                div()
                    .id("imports-converter")
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .text_size(px(theme::metrics::TEXT_LABEL))
                    .text_color(rgb(p.muted))
                    .child(format!(
                        "Converter: {} · AI conversion {}",
                        match documents.converter {
                            DocumentConverter::Xberg => "Xberg",
                            DocumentConverter::XbergBrn => "Xberg + BRN layer",
                        },
                        if documents.ai { "on" } else { "off" }
                    ))
                    .child(
                        button("imports-documents-settings")
                            .label("Change…")
                            .ghost()
                            .compact()
                            .h(components::tokens::COMPACT_CONTROL)
                            .accessibility_label("Change document conversion in Settings")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_settings_section(SettingsSection::Documents, window, cx)
                            })),
                    )
                    .test_support(),
            );
        if importing {
            card = card.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().min_w_0().child(components::loading(
                        "imports-progress",
                        self.progress.clone(),
                    )))
                    .child(
                        button("imports-stop")
                            .label("Stop import")
                            .outline()
                            .compact()
                            .h(components::tokens::COMPACT_CONTROL)
                            .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                    ),
            );
        }
        if let Some(attempt) = &self.inbox_page.attempt {
            card = card.child(self.attempt_line(attempt, busy, cx));
        }
        let capture = div()
            .id("imports-capture")
            .pt(components::tokens::SPACE_3)
            .text_size(px(theme::metrics::TEXT_LABEL))
            .text_color(rgb(p.muted))
            .child("From a conversation, choose Capture useful outcomes in the chat. Its proposals appear in For you; nothing is applied without Apply.")
            .test_support();
        page_group("imports-add-group")
            .child(group_heading("Add a file", None))
            .child(card.test_support())
            .child(capture)
            .into_any_element()
    }

    fn attempt_line(
        &self,
        attempt: &ImportAttempt,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let intent = match attempt.intent {
            ImportIntent::FullNote => "Full note",
            _ => "Useful information",
        };
        let (tone, outcome) = match &attempt.state {
            AttemptState::Running => (None, "Importing…".to_owned()),
            AttemptState::Finished(tone, text) => (Some(*tone), text.clone()),
            AttemptState::Failed(error) => (Some(Tone::Error), format!("Failed: {error}")),
            AttemptState::Stopped(error) => (Some(Tone::Warning), format!("Stopped: {error}")),
        };
        let retry = matches!(
            attempt.state,
            AttemptState::Failed(_) | AttemptState::Stopped(_)
        );
        let summary = format!("{} · {intent} → {}", attempt.title, attempt.destination);
        let mut line = div()
            .id("imports-attempt")
            .flex()
            .items_start()
            .gap_2()
            .pt(components::tokens::SPACE_2)
            .border_t_1()
            .border_color(rgb(pal().hairline))
            .aria_label(format!("{summary} · {outcome}"))
            .children(tone.map(|tone| components::tone_mark("imports-attempt-tone", tone)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        div()
                            .id("imports-attempt-file")
                            .min_w_0()
                            .truncate()
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .text_color(rgb(pal().ink))
                            .child(summary.clone())
                            .tooltip(move |window, cx| {
                                Tooltip::new(summary.clone()).build(window, cx)
                            })
                            .test_support(),
                    )
                    .child(
                        div()
                            .id("imports-attempt-outcome")
                            .text_size(px(theme::metrics::TEXT_LABEL))
                            .text_color(rgb(pal().muted))
                            .child(outcome)
                            .test_support(),
                    ),
            );
        if retry {
            line = line.child(
                button("imports-retry")
                    .label("Retry")
                    .icon(IconName::RefreshCw)
                    .outline()
                    .compact()
                    .h(components::tokens::COMPACT_CONTROL)
                    .disabled(busy)
                    .accessibility_label(format!("Retry importing {}", attempt.title))
                    .on_click(cx.listener(|this, _, _, cx| this.retry_import(cx))),
            );
        }
        line.test_support().into_any_element()
    }

    /// The newest notes that carry an import record: what they became, the
    /// coverage the converter reported, figures and their source.
    fn recent_imports(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let mut imported: Vec<(&Record, &ImportCoverage, &Note)> = self
            .records
            .iter()
            .filter(|record| !record.archived)
            .filter_map(|record| match &record.data {
                RecordData::Note(note) => note.import.as_ref().map(|import| (record, import, note)),
                _ => None,
            })
            .collect();
        imported.sort_by_key(|(record, _, _)| std::cmp::Reverse(self.activity.order(&record.id)));
        let mut group = page_group("imports-recent").child(group_heading(
            "Recent imports",
            (!imported.is_empty()).then_some(imported.len()),
        ));
        if imported.is_empty() {
            group = group.child(
                div()
                    .id("imports-recent-empty")
                    .px_2()
                    .text_size(px(theme::metrics::TEXT_LABEL))
                    .text_color(rgb(pal().muted))
                    .child("Nothing imported yet.")
                    .test_support(),
            );
        }
        for (record, import, note) in imported.into_iter().take(8) {
            let id = record.id.clone();
            let figures = self
                .records
                .iter()
                .filter(|asset| {
                    !asset.archived && matches!(&asset.data, RecordData::Asset(a) if a.note == id)
                })
                .count();
            let kind = if import.full_note {
                if note.protected {
                    "Full note · protected"
                } else {
                    "Full note"
                }
            } else {
                "Useful information · not a complete import"
            };
            let coverage = match &import.report {
                Some(report) => documents::report_summary(report),
                None if import.gaps.is_empty() => "Coverage not recorded".to_owned(),
                None => format!("{} coverage gaps", import.gaps.len()),
            };
            let mut facts = vec![kind.to_owned(), coverage];
            if figures > 0 {
                facts.push(format!(
                    "{figures} figure{}",
                    if figures == 1 { "" } else { "s" }
                ));
            }
            let source = std::path::Path::new(&import.source)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&import.source)
                .to_owned();
            facts.push(format!("From {source}"));
            let partial = import.report.as_ref().is_some_and(|report| {
                report.status != ImportStatus::Complete || !report.issues.is_empty()
            }) || !import.gaps.is_empty();
            let row = div().border_b_1().border_color(rgb(pal().hairline)).child(
                item_row(
                    format!("imports-note-{id}"),
                    IconName::FileText,
                    partial.then_some(Tone::Warning),
                    one_line(&note.title),
                    facts.join(" · "),
                    "",
                )
                .tooltip(format!("{} · {}", note.title, import.source))
                .on_click(
                    cx.listener(move |this, _, window, cx| this.open_note(&id, None, window, cx)),
                ),
            );
            group = group.child(row);
        }
        group.test_support().into_any_element()
    }

    /// Choose a file for an Imports intent; the import then runs through the
    /// same conversion and intake services as every other entry point.
    pub(super) fn choose_import_from_inbox(
        &mut self,
        intent: ImportIntent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(worker) = &self.worker {
            self.warn(worker.kind().busy_notice());
            cx.notify();
            return;
        }
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(
                match intent {
                    ImportIntent::FullNote => "Choose a document to keep as a full note",
                    _ => "Choose a document for useful information",
                }
                .into(),
            ),
        });
        cx.spawn_in(window, async move |this, cx| {
            let paths = receiver.await;
            let _ = this.update_in(cx, |this, _, cx| match paths {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        this.start_import_attempt(path, intent, None, cx);
                    }
                }
                Ok(Ok(None)) => {}
                _ => {
                    this.fail("Could not choose a file.");
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// A dropped or pasted file: ask what it should become first.
    pub(super) fn dropped_import(
        &mut self,
        paths: &[PathBuf],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = paths.first().cloned() else {
            return;
        };
        if paths.len() > 1 {
            self.warn("Add one file at a time. The first file is ready to import.");
        }
        if let Some(worker) = &self.worker {
            self.warn(worker.kind().busy_notice());
            cx.notify();
            return;
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let target = cx.entity().downgrade();
        let useful_destination = self.import_destination(ImportIntent::UsefulInformation);
        self.open_dialog(window, cx, move |dialog, window, _cx| {
            let choose = |id: &'static str, label: &'static str, intent: ImportIntent, primary: bool| {
                let target = target.clone();
                let path = path.clone();
                let control = button(id).label(label).on_click(move |_, window, cx| {
                    window.close_dialog(cx);
                    let _ = target.update(cx, |this, cx| {
                        this.start_import_attempt(path.clone(), intent, None, cx)
                    });
                });
                if primary { control.primary() } else { control.outline() }
            };
            dialog
                .title("What should this file become?")
                .on_ok(|_, _, _| false)
                .child(
                    components::dialog_body("imports-intent-body", window)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(div().font_weight(gpui_kit::FontWeight::MEDIUM).child(name.clone()))
                                .child(format!("Useful information: BRN reads it in a chat ({useful_destination}) and proposes what to keep. A summary is not a complete import."))
                                .child("Full note: a protected, readable note in General knowledge with figures and a coverage report. The original file stays untouched."),
                        )
                        .test_support(),
                )
                .footer(
                    components::dialog_footer()
                        .child(
                            button("imports-intent-cancel")
                                .label("Cancel")
                                .outline()
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(choose("imports-intent-full", "Import full note", ImportIntent::FullNote, false))
                        .child(choose(
                            "imports-intent-useful",
                            "Add useful information",
                            ImportIntent::UsefulInformation,
                            true,
                        )),
                )
        });
    }

    pub(super) fn start_import_attempt(
        &mut self,
        path: PathBuf,
        intent: ImportIntent,
        pinned_project: Option<Option<String>>,
        cx: &mut Context<Self>,
    ) {
        if let Some(worker) = &self.worker {
            self.warn(format!(
                "{} Then choose this file again.",
                worker.kind().busy_notice()
            ));
            cx.notify();
            return;
        }
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        // Retry keeps the original scope; a new attempt takes the open chat's.
        let project = match pinned_project {
            Some(project) => project,
            None => self.import_project(intent),
        };
        let destination = self.import_destination_for(intent, project.as_deref());
        if let Some(id) = &project
            && !self
                .records
                .iter()
                .any(|record| record.id == *id && !record.archived)
        {
            self.inbox_page.attempt = Some(ImportAttempt {
                path,
                title,
                intent,
                destination,
                project,
                state: AttemptState::Failed(
                    "Its Project is no longer available, so nothing was imported. Choose the file again from where it should go.".into(),
                ),
            });
            cx.notify();
            return;
        }
        self.convert_file_scoped(None, path.clone(), intent, None, Some(project.clone()), cx);
        // convert_file refuses unsupported files without starting work.
        let started = self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.kind() == WorkerKind::Import);
        self.inbox_page.attempt = Some(ImportAttempt {
            path,
            title,
            intent,
            destination,
            project,
            state: if started {
                AttemptState::Running
            } else {
                AttemptState::Failed(self.notice.text().to_owned())
            },
        });
        cx.notify();
    }

    pub(super) fn retry_import(&mut self, cx: &mut Context<Self>) {
        let Some(attempt) = self.inbox_page.attempt.clone() else {
            return;
        };
        self.start_import_attempt(attempt.path, attempt.intent, Some(attempt.project), cx);
    }

    /// Called when a conversion finishes: records this import's outcome on
    /// the Imports attempt, using the outcome the service just reported.
    pub(super) fn finish_import_attempt(&mut self, error: Option<String>, cancelled: bool) {
        let Some(attempt) = self.inbox_page.attempt.as_mut() else {
            return;
        };
        if attempt.state != AttemptState::Running {
            return;
        }
        attempt.state = match error {
            Some(error) if cancelled => AttemptState::Stopped(error),
            Some(error) => AttemptState::Failed(error),
            None if attempt.intent == ImportIntent::FullNote => {
                let text = self.notice.text();
                if text.is_empty() {
                    AttemptState::Finished(Tone::Success, "Converted.".into())
                } else {
                    AttemptState::Finished(self.notice.tone(), text.to_owned())
                }
            }
            None => AttemptState::Finished(
                Tone::Info,
                "Opened a chat that reads this file and proposes useful information.".into(),
            ),
        };
    }

    /// Open Settings at one section (used by Inbox links).
    pub(super) fn open_settings_section(
        &mut self,
        section: SettingsSection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.settings_section = section;
        self.show_tile(TileKind::Settings, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, data: RecordData) -> Record {
        Record {
            id: id.into(),
            version: 1,
            archived: false,
            data,
        }
    }
    fn chat(id: &str, attention: Vec<Attention>) -> (Record, ThreadRow) {
        (
            record(
                id,
                RecordData::Thread(Thread {
                    private_context: false,
                    context_epoch: 0,
                    title: format!("Chat {id}"),
                    state: ThreadState::Open,
                    attention: attention.clone(),
                }),
            ),
            ThreadRow {
                id: id.into(),
                title: format!("Chat {id}"),
                resolved: false,
                reasons: attention.iter().map(|a| a.reason.clone()).collect(),
                note: None,
                has_action: false,
                order: 0,
                day: None,
            },
        )
    }
    fn attention(kind: AttentionKind, reason: &str) -> Attention {
        Attention {
            kind,
            reason: reason.into(),
            record: None,
        }
    }
    fn task(id: &str, state: ActionState, waiting_on: Option<&str>) -> Record {
        record(
            id,
            RecordData::Action(Action {
                kept_outside_project: false,
                chat: None,
                description: format!("Task {id}"),
                state,
                internal: false,
                evidence: None,
                due: None,
                owner: None,
                waiting_on: waiting_on.map(Into::into),
                follow_up: None,
            }),
        )
    }

    #[test]
    fn for_you_distinguishes_review_decision_failed_and_waiting() {
        let (failed, failed_row) = chat(
            "c1",
            vec![attention(
                AttentionKind::Blocker,
                "Run failed: provider unavailable",
            )],
        );
        let (stopped, stopped_row) = chat(
            "c2",
            vec![attention(
                AttentionKind::Blocker,
                brn_ai::threads::INTERRUPTED_ATTENTION,
            )],
        );
        let (asked, asked_row) = chat(
            "c3",
            vec![
                attention(AttentionKind::Question, "Which date should I use?"),
                attention(AttentionKind::Review, "Review prepared changes"),
            ],
        );
        let records = vec![
            failed,
            stopped,
            asked,
            task("t1", ActionState::Waiting, Some("Legal")),
            task("t2", ActionState::Open, None),
        ];
        let rows = vec![failed_row, stopped_row, asked_row];
        let inbox = InboxState {
            revision: 1,
            generation: "g".into(),
            folder: "/tmp/in".into(),
            folder_identity: "i".into(),
            destination: InboxDestination::General,
            entries: vec![
                InboxEntry {
                    name: "ok.md".into(),
                    source_sha256: None,
                    operation_key: None,
                    status: InboxStatus::Imported,
                    detail: "Protected full note in General knowledge".into(),
                    note: Some("n".into()),
                },
                InboxEntry {
                    name: "bad.xyz".into(),
                    source_sha256: None,
                    operation_key: None,
                    status: InboxStatus::Failed,
                    detail: "Unsupported file".into(),
                    note: None,
                },
            ],
        };
        let items = for_you_items(&records, &rows, &[], &[], Some(&inbox));
        let summary: Vec<(Need, &str, &str)> = items
            .iter()
            .map(|item| match item.target {
                // A chat item leads with the chat; its reason is the context.
                Target::Chat(_) => (item.need, item.state, item.context.as_str()),
                _ => (item.need, item.state, item.title.as_str()),
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                (Need::Review, "Review", "Review prepared changes"),
                (Need::Decision, "Question", "Which date should I use?"),
                (Need::Failed, "Failed", "Run failed: provider unavailable"),
                (
                    Need::Failed,
                    "Stopped",
                    brn_ai::threads::INTERRUPTED_ATTENTION
                ),
                (Need::Failed, "Failed", "bad.xyz wasn't imported"),
                (Need::Waiting, "Waiting", "Task t1"),
            ]
        );
        // Each opens its exact origin; the failed import keeps its filename
        // and target scope.
        assert_eq!(items[0].target, Target::Chat("c3".into()));
        assert_eq!(items[2].target, Target::Chat("c1".into()));
        assert_eq!(items[4].target, Target::Imports);
        assert!(items[4].context.contains("General knowledge"));
        assert_eq!(items[5].target, Target::Task("t1".into()));
        assert_eq!(items[5].context, "Task · waiting on Legal");
        // Waiting on someone else is shown but not counted.
        assert_eq!(items.iter().filter(|item| item.need.counted()).count(), 5);
    }

    #[test]
    fn a_caught_up_workspace_has_no_items() {
        let (quiet, row) = chat("c1", vec![]);
        let records = vec![quiet, task("t1", ActionState::Open, None)];
        assert!(for_you_items(&records, &[row], &[], &[], None).is_empty());
    }
}
