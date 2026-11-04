//! Knowledge navigation for the ordinary-wiki B flow: the Connections tile
//! (a bounded, visibility-checked neighbourhood of the open note, as a
//! readable list), the root Topics section, and the owner-triggered
//! knowledge check. Every request runs off the UI thread.
use super::shell::pill;
use super::tiles::{row, scroll};
use super::*;
use brn_threads_app::search::{CancelOnDrop, Connections, Neighbour, NodeKind};

/// Relation filters shown as pills; other names are grouped as "other".
const OTHER: &str = "other";

#[derive(Default)]
pub(super) struct ConnectionsView {
    generation: u64,
    pub(super) depth: u8,
    pub(super) sources: bool,
    /// `None` shows every relation.
    pub(super) filter: Option<String>,
    pub(super) pending: bool,
    pub(super) result: Option<Connections>,
    pub(super) error: Option<String>,
    task: Option<Task<()>>,
    cancel: Option<CancelOnDrop>,
}

#[derive(Default)]
pub(super) struct KnowledgeView {
    pub(super) running: bool,
    pub(super) result: Option<KnowledgeCheck>,
    pub(super) error: Option<String>,
    snapshot: Option<KnowledgeTriage>,
    selected: Option<(KnowledgeSelection, String)>,
    filter: Option<FindingKind>,
    page: usize,
    generation: u64,
    busy: bool,
    task: Option<Task<()>>,
}
#[derive(Clone)]
enum KnowledgeAction {
    Inspect(usize),
    Open(String),
    Review,
    Draft,
}
pub(super) enum KnowledgeAnswer {
    Selected,
    Open(String),
    Review(Prepared),
    Draft(String, ChatDraft),
}
const KNOWLEDGE_PAGE: usize = 8;
fn finding_label(kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::BrokenLink => "Broken links",
        FindingKind::StaleInput => "Outdated inputs",
        FindingKind::StaleCandidate => "Outdated proposals",
        FindingKind::CoverageGap => "Coverage gaps",
        FindingKind::Orphan => "Unconnected notes",
        FindingKind::DuplicateTitle => "Similar titles",
        FindingKind::DuplicateContent => "Duplicate content",
        FindingKind::Contradiction => "Recorded conflicts",
        FindingKind::Supersession => "Superseded notes",
        FindingKind::RelationVariant => "Relation names",
        FindingKind::UnlistedTopic => "Unlisted topics",
    }
}

fn relation_label(relation: &brn_threads_app::search::Relation) -> String {
    let known = match relation.relation.as_str() {
        relations::SOURCE => Some("source"),
        relations::SUPERSEDES => Some("supersedes"),
        relations::CONTRADICTS => Some("contradicts"),
        relations::RELATED => Some("related"),
        _ => None,
    };
    let arrow = if relation.outgoing { "→" } else { "←" };
    format!("{arrow} {}", known.unwrap_or(relation.relation.as_str()))
}

fn matches_filter(node: &Neighbour, filter: Option<&str>) -> bool {
    match filter {
        None => true,
        Some(OTHER) => node
            .relations
            .iter()
            .any(|r| !relations::is_recommended(&r.relation)),
        Some(name) => node.relations.iter().any(|r| r.relation == name),
    }
}

impl ThreadsUi {
    /// Requests the open note's neighbourhood when the Connections tile is
    /// visible. A newer request replaces the older one.
    pub(super) fn refresh_connections(&mut self, cx: &mut Context<Self>) {
        let visible = self
            .layout
            .visible()
            .iter()
            .any(|tile| tile.kind == TileKind::Connections);
        let graph = &mut self.graph;
        graph.generation += 1;
        graph.task = None;
        graph.cancel = None;
        graph.pending = false;
        graph.result = None;
        graph.error = None;
        if graph.depth == 0 {
            graph.depth = 1;
        }
        let Some(note) = self.open_note_id().map(str::to_owned).filter(|_| visible) else {
            self.graph.pending = false;
            return;
        };
        let handle = match self.workspace.search_handle() {
            Ok(handle) => handle,
            Err(error) => {
                self.graph.result = None;
                self.graph.error = Some(error.to_string());
                return;
            }
        };
        let graph = &mut self.graph;
        let scope = self.note_project.clone().or_else(|| {
            self.records
                .iter()
                .find(|r| r.id == note)
                .and_then(|r| match &r.data {
                    RecordData::Note(n) => n.chat.clone(),
                    _ => None,
                })
        });
        let ticket =
            handle.request_connections(&note, scope.as_deref(), graph.depth, graph.sources);
        graph.cancel = Some(ticket.cancel_on_drop());
        graph.pending = true;
        let generation = graph.generation;
        graph.task = Some(cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move { ticket.wait() })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.graph.generation != generation {
                    return;
                }
                this.graph.pending = false;
                this.graph.cancel = None;
                match answer {
                    Ok(found) => {
                        this.graph.error = None;
                        this.graph.result = Some(found);
                    }
                    Err(error) => {
                        this.graph.result = None;
                        this.graph.error = Some(error.to_string());
                    }
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn connections_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let graph = &self.graph;
        let mut list = scroll("connections-content").gap_2().p_3();
        let Some(open) = self.open_note_id() else {
            return list
                .child(components::empty_state(
                    "connections-empty",
                    IconName::Network,
                    "No note open",
                    "Open a note to see the notes and sources it links with.",
                    None,
                ))
                .test_support()
                .into_any_element();
        };
        let depth = graph.depth.max(1);
        let mut controls = div().flex().flex_wrap().gap_1().items_center();
        for hops in [1u8, 2] {
            controls = controls.child(
                pill(
                    format!("connections-depth-{hops}"),
                    if hops == 1 { "Direct" } else { "Two hops" },
                )
                .selected(depth == hops)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.graph.depth = hops;
                    this.refresh_connections(cx);
                    cx.notify();
                })),
            );
        }
        controls = controls.child(
            pill("connections-sources", "Sources")
                .selected(graph.sources)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.graph.sources = !this.graph.sources;
                    this.refresh_connections(cx);
                    cx.notify();
                })),
        );
        let mut filters = div().flex().flex_wrap().gap_1();
        for name in [None]
            .into_iter()
            .chain(relations::RECOMMENDED.iter().map(|r| Some(*r)))
            .chain([Some(OTHER)])
        {
            let id = format!("connections-filter-{}", name.unwrap_or("all"));
            // Relation names appear as written.
            let label = name.unwrap_or("All").to_owned();
            filters = filters.child(
                pill(id, label)
                    .selected(graph.filter.as_deref() == name)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.graph.filter = name.map(str::to_owned);
                        cx.notify();
                    })),
            );
        }
        list = list
            .child(section(if depth == 1 {
                "Local connections · direct links"
            } else {
                "Local connections · two hops"
            }))
            .child(controls)
            .child(filters);
        if let Some(error) = &graph.error {
            list = list.child(components::banner(
                "connections-error",
                Tone::Error,
                format!(
                    "Connections failed: {error}. Your notes are unchanged; choose Direct or Two hops to try again."
                ),
                vec![],
                None,
            ));
        }
        let current = graph
            .result
            .as_ref()
            .filter(|found| found.centre.id == open);
        let Some(found) = current else {
            return list
                .child(if graph.pending {
                    components::loading("connections-pending", "Finding connections…")
                        .into_any_element()
                } else {
                    metadata("Connections appear here for the open note.").into_any_element()
                })
                .test_support()
                .into_any_element();
        };
        let shown: usize = found.hops.iter().map(|h| h.shown.len()).sum();
        let available: usize = found.hops.iter().map(|h| h.available).sum();
        list = list.child(
            metadata(format!(
                "{shown} of {available} connected items shown · click a note to open and recenter"
            ))
            .id("connections-status")
            .test_support(),
        );
        for (index, hop) in found.hops.iter().enumerate() {
            list = list.child(section(format!(
                "{} · {} of {}",
                if index == 0 { "Linked" } else { "Second hop" },
                hop.shown.len(),
                hop.available
            )));
            let nodes: Vec<&Neighbour> = hop
                .shown
                .iter()
                .filter(|node| matches_filter(node, graph.filter.as_deref()))
                .collect();
            if nodes.is_empty() {
                list = list.child(metadata("Nothing with this relation."));
            }
            for node in nodes {
                let id = node.id.clone();
                let mut facts: Vec<String> = node.relations.iter().map(relation_label).collect();
                if node.kind == NodeKind::Source {
                    facts.push("source".into());
                }
                if let Some(via) = &node.via {
                    let through = found
                        .hops
                        .first()
                        .and_then(|h| h.shown.iter().find(|n| &n.id == via))
                        .map_or(via.as_str(), |n| n.title.as_str());
                    facts.push(format!("via {}", short(through)));
                }
                let links = if node.inbound_links > 0 {
                    format!("← {}", node.inbound_links)
                } else {
                    String::new()
                };
                let mut entry =
                    components::ListRow::new(format!("connection-{}", node.id), node.title.clone())
                        .meta(links);
                if !facts.is_empty() {
                    entry = entry.reason(facts.join(" · "));
                }
                let mut entry = entry.build();
                if node.kind == NodeKind::Note {
                    entry = entry.on_click(cx.listener(move |this, _, window, cx| {
                        this.open_note(&id, None, window, cx);
                        this.refresh_connections(cx);
                    }));
                }
                list = list.child(entry);
            }
        }
        list = list.child(metadata(
            "Only saved links are shown; nothing is inferred. Relation names appear as written.",
        ));
        list.test_support().into_any_element()
    }

    /// The Topics section at the top of the notes tile.
    pub(super) fn topics_section(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let mut block = div().id("topics").flex().flex_col().gap_1();
        let Some(topics) = &self.topics else {
            // One row; the explanation is in the tooltip.
            return block
                .flex_row()
                .items_center()
                .gap_2()
                .pt_2()
                .px_2()
                .font_family(UI_FONT)
                .text_size(SMALL_TEXT)
                .font_weight(gpui_kit::FontWeight::MEDIUM)
                .text_color(rgb(p.muted))
                .aria_label("Topics · Create Topics note")
                .child(div().flex_1().min_w_0().truncate().child("Topics"))
                .child(
                    pill("create-topics", "Create Topics note")
                        .tooltip(
                            "A Topics note links your topic pages, so you and BRN start from the same map.",
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.create_topics(window, cx)),
                        ),
                )
                .test_support()
                .into_any_element();
        };
        let id = topics.note.clone();
        block = block
            .child(section(format!("Topics · {}", topics.entries.len())))
            .child(
                row("topics-note", topics.title.clone(), "Topics note", false).on_click(
                    cx.listener(move |this, _, window, cx| this.open_note(&id, None, window, cx)),
                ),
            );
        if topics.entries.is_empty() {
            block = block
                .child(metadata("Link topic pages from the Topics note to list them here.").px_2());
        }
        for entry in &topics.entries {
            let id = entry.note.clone();
            let mut topic =
                components::ListRow::new(format!("topic-{}", entry.note), entry.title.clone())
                    .meta(format!("{} linked", entry.linked_notes))
                    .selected(self.open_note_id() == Some(entry.note.as_str()));
            if !entry.description.is_empty() {
                topic = topic.reason(entry.description.clone());
            }
            block = block.child(topic.build().on_click(
                cx.listener(move |this, _, window, cx| this.open_note(&id, None, window, cx)),
            ));
        }
        block.test_support().into_any_element()
    }

    fn create_topics(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.workspace.create_topics_note() {
            Ok((note, outcome)) => {
                if !matches!(outcome, ApplyOutcome::Applied(_)) {
                    self.warn(
                        "The Topics note was created; choose it again after the current run finishes.",
                    );
                }
                self.refresh();
                self.open_note(&note.id, None, window, cx);
            }
            Err(error) => self.fail(error.to_string()),
        }
        cx.notify();
    }

    fn run_knowledge_check(&mut self, cx: &mut Context<Self>) {
        let data = self.data_dir.clone();
        self.knowledge.generation += 1;
        self.knowledge.snapshot = None;
        self.knowledge.selected = None;
        self.knowledge.filter = None;
        self.knowledge.page = 0;
        self.knowledge.busy = false;
        self.knowledge.running = true;
        self.knowledge.result = None;
        self.knowledge.error = None;
        // A separate read view keeps the check off the UI thread.
        self.knowledge.task = Some(cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move { Workspace::open(&data)?.knowledge_triage() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.knowledge.running = false;
                match answer {
                    Ok(snapshot) => {
                        this.knowledge.result = Some(snapshot.check.clone());
                        this.knowledge.snapshot = Some(snapshot);
                    }
                    Err(error) => this.knowledge.error = Some(error.to_string()),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn knowledge_action(
        &mut self,
        action: KnowledgeAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.knowledge.running || self.knowledge.busy {
            return;
        }
        let Some(snapshot) = self.knowledge.snapshot.clone() else {
            return;
        };
        let selected = self.knowledge.selected.clone();
        let data = self.data_dir.clone();
        self.knowledge.generation += 1;
        let generation = self.knowledge.generation;
        self.knowledge.busy = true;
        self.knowledge.error = None;
        self.knowledge.task = Some(cx.spawn_in(window, async move |this, cx| {
            let answer: AppResult<(KnowledgeSelection, KnowledgeAnswer)> = cx.background_executor().spawn(async move {
                let mut workspace = Workspace::open(&data)?;
                if let KnowledgeAction::Inspect(index) = action {
                    return workspace.select_knowledge_finding(&snapshot, index).map(|selection| (selection, KnowledgeAnswer::Selected));
                }
                let (selection, key) = selected.ok_or_else(|| AppError::Invalid("Select a finding first".into()))?;
                workspace.validate_knowledge_selection(&selection)?;
                let answer = match action {
                    KnowledgeAction::Open(id) => selection.records.iter().find(|r| r.id == id && !r.archived && matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none())).map(|r| KnowledgeAnswer::Open(r.id.clone())).ok_or_else(|| AppError::Invalid("This affected note is unavailable".into())),
                    KnowledgeAction::Review => selection.candidate.clone().map(KnowledgeAnswer::Review).ok_or_else(|| AppError::Invalid("No pending fix exists for this finding".into())),
                    KnowledgeAction::Draft => workspace.draft_knowledge_follow_up(&key, &selection).map(|(chat, draft)| KnowledgeAnswer::Draft(chat, draft)),
                    KnowledgeAction::Inspect(_) => unreachable!(),
                }?;
                Ok((selection, answer))
            }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.knowledge.generation != generation { return; }
                this.knowledge.busy = false;
                let result = answer.and_then(|(selection, answer)| this.finish_knowledge_action(&selection, answer, window, cx));
                if let Err(error) = result {
                    this.knowledge.selected = None;
                    this.knowledge.snapshot = None;
                    this.knowledge.result = None;
                    this.knowledge.error = Some(error.to_string());
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Validate again after executor delivery before exposing records or a proposal.
    pub(super) fn finish_knowledge_action(
        &mut self,
        selection: &KnowledgeSelection,
        answer: KnowledgeAnswer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AppResult<()> {
        self.workspace.validate_knowledge_selection(selection)?;
        match answer {
            KnowledgeAnswer::Selected => {
                self.knowledge.selected =
                    Some((selection.clone(), uuid::Uuid::new_v4().to_string()))
            }
            KnowledgeAnswer::Open(id) => self.open_note(&id, None, window, cx),
            KnowledgeAnswer::Review(candidate) => self.open_review(candidate, window, cx),
            KnowledgeAnswer::Draft(chat, delivered) => {
                let thread = self
                    .workspace
                    .store
                    .record(&chat)?
                    .ok_or_else(|| AppError::Invalid("Draft chat disappeared".into()))?;
                if thread.archived
                    || !matches!(thread.data, RecordData::Thread(ref t) if t.state == ThreadState::Open)
                    || self.workspace.store.chat_has_private_context(&chat)?
                    || self
                        .workspace
                        .store
                        .project_context_record(&chat)?
                        .is_some()
                {
                    return Err(AppError::Invalid(
                        "Draft destination changed; reopen it explicitly".into(),
                    ));
                }
                // The user can edit while background work runs. Never replace a
                // visible target composer, even if the user just emptied it.
                if self.chat_thread() != Some(chat.as_str()) {
                    let destination = ComposerDestination::Thread(chat.clone());
                    let latest = self
                        .workspace
                        .store
                        .chat_drafts()?
                        .into_iter()
                        .find(|d| d.destination == delivered.destination)
                        .ok_or_else(|| AppError::Invalid("Draft recovery disappeared".into()))?;
                    let local = self.draft_generations.get(&latest.destination).copied();
                    if local.is_none_or(|generation| generation < latest.generation) {
                        self.draft_generations
                            .insert(latest.destination, latest.generation);
                        self.composer_drafts.preserve(destination, latest.text);
                    }
                }
                self.refresh();
                self.open_thread(&chat, window, cx);
                self.warn("Unsent follow-up draft. Review and edit it before sending; no AI request has started.");
            }
        }
        Ok(())
    }

    /// The overview panel for the owner-triggered knowledge check.
    pub(super) fn knowledge_panel(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let knowledge = &self.knowledge;
        let mut panel = div()
            .id("knowledge-check")
            .flex()
            .flex_col()
            .gap_2()
            .min_w_0()
            .child(section("Knowledge check"))
            .child(metadata(
                "Look for broken links, outdated inputs, gaps, orphans, duplicates and unresolved conflicts. A clean check is not proof of correctness.",
            ))
            .child(
                div().child(
                    button("knowledge-run")
                        .label(if knowledge.running {
                            "Checking…"
                        } else {
                            "Check knowledge"
                        })
                        .outline()
                        .loading(knowledge.running)
                        .disabled(knowledge.running || knowledge.busy)
                        .when(knowledge.running, |button| {
                            let (announced, tooltip) =
                                components::unavailable("Check knowledge", Some("a check is running"));
                            button.accessibility_label(announced).tooltip(tooltip)
                        })
                        .on_click(cx.listener(|this, _, _, cx| this.run_knowledge_check(cx))),
                ),
            );
        if let Some(error) = &knowledge.error {
            panel = panel.child(components::banner(
                "knowledge-error",
                Tone::Error,
                format!("Check failed: {error}. Your notes are unchanged; choose Check knowledge to try again."),
                vec![],
                None,
            ));
        }
        if let Some(check) = &knowledge.result {
            let total: usize = check.counts.values().sum();
            panel = panel.child(
                metadata(format!(
                    "{total} findings in {} notes and {} links",
                    check.notes, check.links
                ))
                .id("knowledge-summary")
                .test_support(),
            );
            let mut filters = div().flex().flex_wrap().gap_2().child(
                button("knowledge-filter-all")
                    .label("All findings")
                    .outline()
                    .selected(knowledge.filter.is_none())
                    .disabled(knowledge.busy)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.knowledge.filter = None;
                        this.knowledge.page = 0;
                        this.knowledge.selected = None;
                        cx.notify();
                    })),
            );
            for (&kind, count) in &check.counts {
                filters = filters.child(
                    button(format!("knowledge-filter-{kind:?}"))
                        .label(format!("{} · {count}", finding_label(kind)))
                        .outline()
                        .selected(knowledge.filter == Some(kind))
                        .disabled(knowledge.busy)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.knowledge.filter = Some(kind);
                            this.knowledge.page = 0;
                            this.knowledge.selected = None;
                            cx.notify();
                        })),
                );
            }
            panel = panel.child(filters);
            let listed: Vec<_> = check
                .findings
                .iter()
                .enumerate()
                .filter(|(_, f)| knowledge.filter.is_none_or(|kind| kind == f.kind))
                .collect();
            let count = knowledge
                .filter
                .map_or(total, |kind| check.counts.get(&kind).copied().unwrap_or(0));
            let start = knowledge.page * KNOWLEDGE_PAGE;
            let page_summary = format!(
                "Showing {}–{} of {} listed findings ({count} total).",
                if listed.is_empty() { 0 } else { start + 1 },
                (start + KNOWLEDGE_PAGE).min(listed.len()),
                listed.len()
            );
            panel = panel.child(
                metadata(page_summary.clone())
                    .id("knowledge-page-summary")
                    .role(gpui_kit::Role::Label)
                    .aria_label(page_summary.clone())
                    .aria_value(page_summary)
                    .test_support(),
            );
            if count > listed.len() {
                panel = panel.child(metadata("Only the first 25 findings per kind are listed. Check again after reviewing them."));
            }
            for &(index, finding) in listed.iter().skip(start).take(KNOWLEDGE_PAGE) {
                panel = panel.child(
                    div()
                        .id(format!("knowledge-finding-{index}"))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(finding.detail.clone())
                        .child(
                            button(format!("knowledge-inspect-{index}"))
                                .label("Inspect finding")
                                .outline()
                                .disabled(knowledge.busy)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.knowledge_action(
                                        KnowledgeAction::Inspect(index),
                                        window,
                                        cx,
                                    )
                                })),
                        ),
                );
            }
            if listed.len() > KNOWLEDGE_PAGE {
                panel = panel.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            button("knowledge-previous")
                                .label("Previous")
                                .outline()
                                .disabled(knowledge.page == 0 || knowledge.busy)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.knowledge.page = this.knowledge.page.saturating_sub(1);
                                    this.knowledge.selected = None;
                                    cx.notify();
                                })),
                        )
                        .child(
                            button("knowledge-next")
                                .label("Next")
                                .outline()
                                .disabled(start + KNOWLEDGE_PAGE >= listed.len() || knowledge.busy)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.knowledge.page += 1;
                                    this.knowledge.selected = None;
                                    cx.notify();
                                })),
                        ),
                );
            }
            if let Some((selection, _)) = &knowledge.selected {
                let mut detail = div()
                    .id("knowledge-selected")
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(section(finding_label(selection.finding.kind)))
                    .child(selection.finding.detail.clone());
                for record in &selection.records {
                    if let RecordData::Note(note) = &record.data {
                        let id = record.id.clone();
                        detail = detail.child(
                            button(format!("knowledge-open-{id}"))
                                .label(format!("Open {} · v{}", note.title, record.version))
                                .outline()
                                .disabled(
                                    knowledge.busy
                                        || record.archived
                                        || note.superseded_by.is_some(),
                                )
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.knowledge_action(
                                        KnowledgeAction::Open(id.clone()),
                                        window,
                                        cx,
                                    )
                                })),
                        );
                    } else if let RecordData::Source(source) = &record.data {
                        detail = detail.child(metadata(format!(
                            "Source {} · v{} · {}",
                            source.label, record.version, source.outcome
                        )));
                    } else if let RecordData::Link(link) = &record.data {
                        detail = detail.child(metadata(format!(
                            "Link {} · v{} · {} → {} ({})",
                            record.id, record.version, link.from, link.to, link.relation
                        )));
                    }
                }
                for id in &selection.finding.records {
                    if !selection.records.iter().any(|r| &r.id == id)
                        && selection
                            .candidate
                            .as_ref()
                            .is_none_or(|p| p.operation.as_str() != id)
                    {
                        detail = detail.child(metadata(format!("Unavailable reference: {id}")));
                    }
                }
                if selection.candidate.is_some() {
                    detail = detail.child(
                        button("knowledge-review-fix")
                            .label("Review existing pending fix")
                            .outline()
                            .disabled(knowledge.busy)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.knowledge_action(KnowledgeAction::Review, window, cx)
                            })),
                    );
                }
                detail = detail.child(metadata("Drafting a follow-up starts no AI request and changes no knowledge. Sending later uses your current delegation settings."))
                    .child(button("knowledge-draft").label("Draft follow-up in chat").outline().disabled(knowledge.busy)
                        .on_click(cx.listener(|this, _, window, cx| this.knowledge_action(KnowledgeAction::Draft, window, cx))));
                panel = panel.child(detail.test_support());
            }
        }
        panel.test_support().into_any_element()
    }
}
