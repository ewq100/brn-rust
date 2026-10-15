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
    task: Option<Task<()>>,
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
        let p = pal();
        let graph = &self.graph;
        let mut list = scroll("connections-content").gap_2().p_3();
        let Some(open) = self.open_note_id() else {
            return list
                .child(empty(
                    "connections-empty",
                    "Open a note to see the notes and sources it links with.",
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
            let label = name.unwrap_or("all").to_owned();
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
            list = list.child(empty(
                "connections-error",
                format!("Connections failed: {error}"),
            ));
        }
        let current = graph
            .result
            .as_ref()
            .filter(|found| found.centre.id == open);
        let Some(found) = current else {
            return list
                .child(metadata(if graph.pending {
                    "Finding connections…"
                } else {
                    "Connections appear here for the open note."
                }))
                .test_support()
                .into_any_element();
        };
        let shown: usize = found.hops.iter().map(|h| h.shown.len()).sum();
        let available: usize = found.hops.iter().map(|h| h.available).sum();
        list = list.child(
            metadata(format!(
                "{shown} of {available} connected records shown · click a note to open and recenter"
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
                    facts.push("source record".into());
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
                let mut entry = row(
                    format!("connection-{}", node.id),
                    short(&node.title),
                    links,
                    false,
                );
                if node.kind == NodeKind::Note {
                    entry = entry.on_click(cx.listener(move |this, _, window, cx| {
                        this.open_note(&id, None, window, cx);
                        this.refresh_connections(cx);
                    }));
                }
                list = list.child(
                    div().flex().flex_col().child(entry).child(
                        div()
                            .px_2()
                            .font_family(UI_FONT)
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(p.muted))
                            .child(facts.join(" · ")),
                    ),
                );
            }
        }
        list = list.child(metadata(
            "Only recorded links are shown; nothing is inferred. Relation names appear as written.",
        ));
        list.test_support().into_any_element()
    }

    /// The Topics section at the top of the notes tile.
    pub(super) fn topics_section(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let mut block = div().id("topics").flex().flex_col().gap_1();
        let Some(topics) = &self.topics else {
            return block
                .child(section("Topics"))
                .child(metadata(
                    "A Topics note links your topic pages, so you and BRN start from the same map.",
                ))
                .child(
                    pill("create-topics", "Create Topics note").on_click(
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
                row("topics-note", short(&topics.title), "open", false).on_click(
                    cx.listener(move |this, _, window, cx| this.open_note(&id, None, window, cx)),
                ),
            );
        if topics.entries.is_empty() {
            block = block.child(metadata(
                "Link topic pages from the Topics note to list them here.",
            ));
        }
        for entry in &topics.entries {
            let id = entry.note.clone();
            block = block.child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        row(
                            format!("topic-{}", entry.note),
                            short(&entry.title),
                            format!("{} linked", entry.linked_notes),
                            self.open_note_id() == Some(entry.note.as_str()),
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| this.open_note(&id, None, window, cx),
                        )),
                    )
                    .when(!entry.description.is_empty(), |d| {
                        d.child(
                            div()
                                .px_2()
                                .text_size(SMALL_TEXT)
                                .text_color(rgb(p.muted))
                                .child(entry.description.clone()),
                        )
                    }),
            );
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
        self.knowledge.running = true;
        self.knowledge.error = None;
        // A separate read view keeps the check off the UI thread.
        self.knowledge.task = Some(cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move { Workspace::open(&data)?.knowledge_check() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.knowledge.running = false;
                match answer {
                    Ok(check) => this.knowledge.result = Some(check),
                    Err(error) => this.knowledge.error = Some(error.to_string()),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn review_knowledge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(check) = self.knowledge.result.clone() else {
            return;
        };
        match self.workspace.seed_knowledge_review(&check) {
            Ok(chat) => {
                self.refresh();
                self.open_thread(&chat, window, cx);
            }
            Err(error) => self.fail(error.to_string()),
        }
        cx.notify();
    }

    /// The overview panel for the owner-triggered knowledge check.
    pub(super) fn knowledge_panel(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let knowledge = &self.knowledge;
        let mut panel = panel()
            .id("knowledge-check")
            .child(subheading("Knowledge check"))
            .child(metadata(
                "Look for broken links, outdated inputs, gaps, orphans, duplicates and unresolved conflicts. A clean check is not proof of correctness.",
            ))
            .child(
                button("knowledge-run")
                    .label(if knowledge.running {
                        "Checking…"
                    } else {
                        "Check knowledge"
                    })
                    .disabled(knowledge.running)
                    .on_click(cx.listener(|this, _, _, cx| this.run_knowledge_check(cx))),
            );
        if let Some(error) = &knowledge.error {
            panel = panel.child(empty("knowledge-error", format!("Check failed: {error}")));
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
            for (kind, count) in &check.counts {
                panel = panel.child(metadata(format!("{kind:?} · {count}")));
            }
            for (index, finding) in check.findings.iter().take(8).enumerate() {
                panel = panel.child(
                    div()
                        .id(format!("knowledge-finding-{index}"))
                        .text_size(UI_TEXT)
                        .child(finding.detail.clone()),
                );
            }
            if total > 0 {
                panel = panel.child(
                    button("knowledge-review")
                        .label("Review findings in a chat")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.review_knowledge(window, cx)),
                        ),
                );
            }
        }
        panel.test_support().into_any_element()
    }
}
