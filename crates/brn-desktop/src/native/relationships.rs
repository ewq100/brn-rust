//! Read-only native inspection over correlated worker observations.
use super::*;
use brn_workflow::knowledge::{
    EdgeOrigin, EvidenceEndpoint, IdentityIssue, IdentityOutcome, NoteLinkOutcome, NoteLinks,
    RelationshipPage,
};
use gpui_kit::{AnyElement, TestSupportExt, base::Disableable};

pub(super) struct SavedLinksPane {
    pub(super) open: bool,
    snapshot: Option<NoteLinks>,
    pub(super) item: usize,
    proof: usize,
    matched: usize,
    source_issue: usize,
    target_issue: usize,
    pub(super) quote: Entity<EditorState>,
    pub(super) scroll: ScrollHandle,
}
impl SavedLinksPane {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Desktop>) -> Self {
        Self {
            open: false,
            snapshot: None,
            item: 0,
            proof: 0,
            matched: 0,
            source_issue: 0,
            target_issue: 0,
            quote: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            scroll: ScrollHandle::new(),
        }
    }
    fn reset_selection(&mut self) {
        self.item = 0;
        self.proof = 0;
        self.matched = 0;
        self.source_issue = 0;
        self.target_issue = 0;
    }
}
pub(super) struct RelationshipsPane {
    pub(super) open: bool,
    snapshot: Option<RelationshipPage>,
    item: usize,
    proof: usize,
    issue: usize,
    duplicate: usize,
    duplicate_path: usize,
    pub(super) quote: Entity<EditorState>,
    pub(super) scroll: ScrollHandle,
}
impl RelationshipsPane {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Desktop>) -> Self {
        Self {
            open: false,
            snapshot: None,
            item: 0,
            proof: 0,
            issue: 0,
            duplicate: 0,
            duplicate_path: 0,
            quote: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            scroll: ScrollHandle::new(),
        }
    }
    fn reset_selection(&mut self) {
        self.item = 0;
        self.proof = 0;
        self.issue = 0;
        self.duplicate = 0;
        self.duplicate_path = 0;
    }
}
pub(super) fn quote_widget(editor: &Entity<EditorState>) -> Editor {
    Editor::new(editor)
        .h_full()
        .readonly(true)
        .aria_label("Exact saved relationship evidence")
}
pub(super) fn copy_button(id: &'static str, quote: &str) -> Button {
    let quote = quote.to_owned();
    Button::new(id)
        .label("Copy exact quote")
        .compact()
        .on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(quote.clone()));
        })
}
fn hash(value: &[u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn identity_status(outcome: Option<IdentityOutcome>) -> &'static str {
    match outcome {
        None => "Unmanaged source · no saved UUID",
        Some(IdentityOutcome::Unique) => "Unique source identity observed",
        Some(IdentityOutcome::Absent) => "Source identity absent",
        Some(IdentityOutcome::Ambiguous) => "Ambiguous source identity",
        Some(IdentityOutcome::Incomplete) => "Incomplete source identity inspection",
    }
}
fn link_status(outcome: NoteLinkOutcome) -> &'static str {
    match outcome {
        NoteLinkOutcome::Resolved => "Resolved · saved target observed",
        NoteLinkOutcome::Absent => "Absent · target was not found",
        NoteLinkOutcome::Unmanaged => "Unmanaged · target has no saved UUID",
        NoteLinkOutcome::Ambiguous => "Ambiguous · multiple target matches",
        NoteLinkOutcome::Incomplete => "Incomplete · target lookup could not be completed",
        NoteLinkOutcome::Changed => "Changed · target differs from observed saved version",
        NoteLinkOutcome::External => "External destination",
        NoteLinkOutcome::NonNote => "Not a Markdown note",
        NoteLinkOutcome::Unsupported => "Unsupported destination",
    }
}
fn origin_status(origin: EdgeOrigin) -> &'static str {
    match origin {
        EdgeOrigin::ExplicitLink => "Explicit Markdown link",
        EdgeOrigin::InferredProvenance => "Inferred provenance candidate",
    }
}
#[derive(Clone, Copy)]
enum Selection {
    Link,
    LinkProof,
    Match,
    SourceIssue,
    TargetIssue,
    Edge,
    EdgeProof,
    Issue,
    Duplicate,
    DuplicatePath,
}
fn controls(
    id: &'static str,
    label: &str,
    index: usize,
    total: usize,
    selection: Selection,
    cx: &mut Context<Desktop>,
) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_1()
        .child(format!(
            "{label} {} of {total}",
            if total == 0 { 0 } else { index + 1 }
        ))
        .child(
            Button::new(format!("previous-{id}"))
                .label("Previous")
                .compact()
                .disabled(index == 0 || total == 0)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.step_selection(selection, false, window, cx)
                })),
        )
        .child(
            Button::new(format!("next-{id}"))
                .label("Next")
                .compact()
                .disabled(index.checked_add(1).is_none_or(|next| next >= total))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.step_selection(selection, true, window, cx)
                })),
        )
        .into_any_element()
}
fn issue_view(issue: &IdentityIssue) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(format!("Inspection issue: {}", issue.path))
        .child(issue.reason.clone())
        .into_any_element()
}
impl Desktop {
    pub(super) fn sync_relationship_widgets(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ai = self.ai.as_ref().unwrap();
        if self.saved_links.snapshot.as_ref() != ai.links.as_ref() {
            self.saved_links.snapshot = ai.links.clone();
            self.saved_links.reset_selection();
            self.saved_links.scroll.set_offset(point(px(0.), px(0.)));
        }
        if self.relationships.snapshot.as_ref() != ai.relationships.as_ref() {
            self.relationships.snapshot = ai.relationships.clone();
            self.relationships.reset_selection();
            self.relationships.scroll.set_offset(point(px(0.), px(0.)));
        }
        let link_quote = ai
            .links
            .as_ref()
            .and_then(|links| links.links.get(self.saved_links.item))
            .and_then(|link| link.evidence.get(self.saved_links.proof))
            .map_or("", |proof| proof.quote.as_str());
        if self.saved_links.quote.read(cx).value().as_ref() != link_quote {
            self.saved_links
                .quote
                .update(cx, |editor, cx| editor.set_value(link_quote, window, cx));
        }
        let edge_quote = ai
            .relationships
            .as_ref()
            .and_then(|page| page.edges.get(self.relationships.item))
            .and_then(|edge| edge.evidence.get(self.relationships.proof))
            .map_or("", |proof| proof.quote.as_str());
        if self.relationships.quote.read(cx).value().as_ref() != edge_quote {
            self.relationships
                .quote
                .update(cx, |editor, cx| editor.set_value(edge_quote, window, cx));
        }
    }
    pub(super) fn inspection_blocked(&self) -> bool {
        let ai = self.ai.as_ref().unwrap();
        !ai.ready
            || !ai.vault_bound
            || ai.application_busy()
            || self.simple_transition.is_some()
            || self.closing.is_some()
            || self.closed
            || self.close_failed
    }
    pub(super) fn clear_saved_link_panel(&mut self) {
        self.ai.as_mut().unwrap().clear_links();
        self.saved_links.open = false;
    }
    pub(super) fn inspect_saved_links(&mut self, cx: &mut Context<Self>) {
        if self.inspection_blocked()
            || !matches!(self.open_doc, Some(DocRef::SavedNote | DocRef::Evidence))
        {
            return;
        }
        self.saved_links.open = true;
        if let Some(command) = self.ai.as_mut().unwrap().inspect_links() {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    pub(super) fn close_saved_links(&mut self, cx: &mut Context<Self>) {
        self.clear_saved_link_panel();
        cx.notify();
    }
    pub(super) fn refresh_relationship_page(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.inspection_blocked() {
            return;
        }
        self.relationships.open = true;
        if let Some(command) = self.ai.as_mut().unwrap().refresh_relationships(offset) {
            self.simple_send(command, cx);
        }
        cx.notify();
    }
    pub(super) fn close_relationship_page(&mut self, cx: &mut Context<Self>) {
        self.relationships.open = false;
        self.ai.as_mut().unwrap().clear_relationships();
        cx.notify();
    }
    fn step_selection(
        &mut self,
        selection: Selection,
        next: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ai = self.ai.as_ref().unwrap();
        let links = ai.links.as_ref();
        let link = links.and_then(|links| links.links.get(self.saved_links.item));
        let page = ai.relationships.as_ref();
        let edge = page.and_then(|page| page.edges.get(self.relationships.item));
        let (index, total) = match selection {
            Selection::Link => (
                &mut self.saved_links.item,
                links.map_or(0, |links| links.links.len()),
            ),
            Selection::LinkProof => (
                &mut self.saved_links.proof,
                link.map_or(0, |link| link.evidence.len()),
            ),
            Selection::Match => (
                &mut self.saved_links.matched,
                link.map_or(0, |link| link.matches.len()),
            ),
            Selection::SourceIssue => (
                &mut self.saved_links.source_issue,
                links.map_or(0, |links| links.issues.len()),
            ),
            Selection::TargetIssue => (
                &mut self.saved_links.target_issue,
                link.map_or(0, |link| link.issues.len()),
            ),
            Selection::Edge => (
                &mut self.relationships.item,
                page.map_or(0, |page| page.edges.len()),
            ),
            Selection::EdgeProof => (
                &mut self.relationships.proof,
                edge.map_or(0, |edge| edge.evidence.len()),
            ),
            Selection::Issue => (
                &mut self.relationships.issue,
                page.map_or(0, |page| page.issues.len()),
            ),
            Selection::Duplicate => (
                &mut self.relationships.duplicate,
                page.map_or(0, |page| page.duplicates.len()),
            ),
            Selection::DuplicatePath => (
                &mut self.relationships.duplicate_path,
                page.and_then(|page| page.duplicates.get(self.relationships.duplicate))
                    .map_or(0, |duplicate| duplicate.paths.len()),
            ),
        };
        let candidate = if next {
            index.checked_add(1)
        } else {
            index.checked_sub(1)
        };
        if let Some(candidate) = candidate.filter(|candidate| *candidate < total) {
            *index = candidate;
            match selection {
                Selection::Link => {
                    self.saved_links.proof = 0;
                    self.saved_links.matched = 0;
                    self.saved_links.target_issue = 0;
                }
                Selection::Edge => self.relationships.proof = 0,
                Selection::Duplicate => self.relationships.duplicate_path = 0,
                _ => {}
            }
            self.sync_relationship_widgets(window, cx);
            cx.notify();
        }
    }
    pub(super) fn render_saved_links(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.saved_links.open {
            return None;
        }
        let ai = self.ai.as_ref().unwrap();
        let mut content = div()
            .id("saved-links-scroll")
            .track_scroll(&self.saved_links.scroll)
            .overflow_y_scroll()
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .vertical_scrollbar(&self.saved_links.scroll)
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .gap_2()
            .p_2();
        if ai.links_loading() {
            content = content.child("Inspecting saved links…");
        } else if let Some(error) = &ai.links_error {
            content = content.child(format!("Saved links unavailable: {error}"));
        } else if let Some(links) = &ai.links {
            content = content
                .child(format!("Saved source: {}", links.source.path))
                .child(identity_status(links.source_outcome))
                .child(format!(
                    "Source UUID: {}",
                    links
                        .source
                        .note_id
                        .map_or("unmanaged".into(), |id| id.to_string())
                ))
                .child(format!("Source SHA-256: {}", hash(&links.source.sha256)));
            if links.source_outcome == Some(IdentityOutcome::Ambiguous) {
                content = content.child(
                    Button::new("keep-source-identity-finding")
                        .label("Keep identity finding")
                        .compact()
                        .disabled(self.inspection_blocked() || ai.finding_capture_pending())
                        .on_click(cx.listener(|this, _, _, cx| this.keep_identity_finding(cx))),
                );
            }
            if links.links.is_empty() {
                content = content.child("This saved note has no Markdown links.");
            } else {
                content = content.child(controls(
                    "saved-link",
                    "Link",
                    self.saved_links.item,
                    links.links.len(),
                    Selection::Link,
                    cx,
                ));
                if let Some(link) = links.links.get(self.saved_links.item) {
                    content = content
                        .child(link.destination.clone())
                        .child(link_status(link.outcome));
                    if !matches!(
                        link.outcome,
                        NoteLinkOutcome::Resolved
                            | NoteLinkOutcome::External
                            | NoteLinkOutcome::NonNote
                    ) {
                        content = content.child(
                            Button::new("keep-saved-link-finding")
                                .label("Keep saved-link finding")
                                .compact()
                                .disabled(self.inspection_blocked() || ai.finding_capture_pending())
                                .on_click(cx.listener(|this, _, _, cx| this.keep_link_finding(cx))),
                        );
                    }
                    if let Some(path) = &link.target_path {
                        content = content.child(format!("Observed target path: {path}"));
                    }
                    if !link.matches.is_empty() {
                        content = content.child(controls(
                            "link-match",
                            "Observed match",
                            self.saved_links.matched,
                            link.matches.len(),
                            Selection::Match,
                            cx,
                        ));
                        if let Some(matched) = link.matches.get(self.saved_links.matched) {
                            content = content
                                .child(format!("Match path: {}", matched.path))
                                .child(format!(
                                    "Match UUID: {}",
                                    matched
                                        .note_id
                                        .map_or("unmanaged".into(), |id| id.to_string())
                                ))
                                .child(format!("Match SHA-256: {}", hash(&matched.sha256)));
                        }
                    }
                    if !link.evidence.is_empty() {
                        content = content.child(controls(
                            "link-proof",
                            "Exact proof",
                            self.saved_links.proof,
                            link.evidence.len(),
                            Selection::LinkProof,
                            cx,
                        ));
                        if let Some(proof) = link.evidence.get(self.saved_links.proof) {
                            content = content
                                .child(if self.saved_links.proof == 0 {
                                    "Link occurrence"
                                } else {
                                    "Used reference definition"
                                })
                                .child(format!(
                                    "Saved byte range: {}–{}",
                                    proof.start_byte, proof.end_byte
                                ))
                                .child(
                                    div()
                                        .h(px(110.))
                                        .flex_shrink_0()
                                        .child(quote_widget(&self.saved_links.quote)),
                                )
                                .child(copy_button("copy-saved-link-proof", &proof.quote));
                        }
                    }
                    if !link.issues.is_empty() {
                        content = content.child(controls(
                            "link-target-issue",
                            "Target issue",
                            self.saved_links.target_issue,
                            link.issues.len(),
                            Selection::TargetIssue,
                            cx,
                        ));
                        if let Some(issue) = link.issues.get(self.saved_links.target_issue) {
                            content = content.child(issue_view(issue));
                        }
                    }
                }
            }
            if !links.issues.is_empty() {
                content = content.child(controls(
                    "link-source-issue",
                    "Inspection issue",
                    self.saved_links.source_issue,
                    links.issues.len(),
                    Selection::SourceIssue,
                    cx,
                ));
                if let Some(issue) = links.issues.get(self.saved_links.source_issue) {
                    content = content.child(issue_view(issue));
                }
            }
        } else {
            content = content.child("Select Refresh to inspect this note's saved links.");
        }
        Some(
            div()
                .h(px(300.))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .border_t_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .p_2()
                        .child(div().flex_1().child("Saved links"))
                        .child(
                            Button::new("refresh-saved-links")
                                .label("Refresh")
                                .compact()
                                .disabled(self.inspection_blocked() || ai.links_loading())
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.inspect_saved_links(cx)),
                                ),
                        )
                        .child(
                            Button::new("close-saved-links")
                                .label("Close links")
                                .compact()
                                .on_click(cx.listener(|this, _, _, cx| this.close_saved_links(cx))),
                        ),
                )
                .child(
                    div()
                        .px_2()
                        .child("Saved Markdown only; unsaved edits are not included."),
                )
                .child(content.test_support())
                .into_any_element(),
        )
    }
    pub(super) fn render_relationship_page(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.relationships.open {
            return None;
        }
        let ai = self.ai.as_ref().unwrap();
        let previous = ai.relationship_previous_offset();
        let next = ai.relationship_next_offset();
        let mut content = div()
            .id("relationships-scroll")
            .track_scroll(&self.relationships.scroll)
            .overflow_y_scroll()
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .vertical_scrollbar(&self.relationships.scroll)
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .child(format!(
                "{} scope · fresh saved observation",
                crate::ai::scope_name(ai.knowledge_scope)
            ));
        if ai.relationships_loading() {
            content = content.child("Inspecting relationships…");
        } else if let Some(error) = &ai.relationships_error {
            content = content.child(format!("Relationships unavailable: {error}"));
        } else if let Some(page) = &ai.relationships {
            content = content.child(format!(
                "Page offset {} · {} displayed · {} matching relationships",
                page.offset,
                page.edges.len(),
                page.total
            ));
            if page.edges.is_empty() {
                content = content.child("No relationships on this page.");
            } else {
                content = content.child(controls(
                    "relationship-edge",
                    "Relationship",
                    self.relationships.item,
                    page.edges.len(),
                    Selection::Edge,
                    cx,
                ));
                if let Some(edge) = page.edges.get(self.relationships.item) {
                    content = content
                        .child(origin_status(edge.origin))
                        .child(format!("Source: {}", edge.source.path))
                        .child(format!("Source UUID: {}", edge.source.note_id))
                        .child(format!("Source SHA-256: {}", hash(&edge.source.sha256)))
                        .child(format!("Target: {}", edge.target.path))
                        .child(format!("Target UUID: {}", edge.target.note_id))
                        .child(format!("Target SHA-256: {}", hash(&edge.target.sha256)));
                    if !edge.evidence.is_empty() {
                        content = content.child(controls(
                            "relationship-proof",
                            "Exact proof",
                            self.relationships.proof,
                            edge.evidence.len(),
                            Selection::EdgeProof,
                            cx,
                        ));
                        if let Some(proof) = edge.evidence.get(self.relationships.proof) {
                            content = content
                                .child(match proof.endpoint {
                                    EvidenceEndpoint::Source => "Proof belongs to source",
                                    EvidenceEndpoint::Target => "Proof belongs to target",
                                })
                                .child(format!(
                                    "Saved byte range: {}–{}",
                                    proof.start_byte, proof.end_byte
                                ))
                                .child(
                                    div()
                                        .h(px(110.))
                                        .flex_shrink_0()
                                        .child(quote_widget(&self.relationships.quote)),
                                )
                                .child(copy_button("copy-relationship-proof", &proof.quote));
                        }
                    }
                }
            }
            if !page.issues.is_empty() {
                content = content.child(controls(
                    "relationship-issue",
                    "Inspection issue",
                    self.relationships.issue,
                    page.issues.len(),
                    Selection::Issue,
                    cx,
                ));
                if let Some(issue) = page.issues.get(self.relationships.issue) {
                    content = content.child(issue_view(issue));
                }
            }
            if !page.duplicates.is_empty() {
                content = content.child(controls(
                    "relationship-duplicate",
                    "Duplicate identity",
                    self.relationships.duplicate,
                    page.duplicates.len(),
                    Selection::Duplicate,
                    cx,
                ));
                if let Some(duplicate) = page.duplicates.get(self.relationships.duplicate) {
                    content = content
                        .child(format!("Ambiguous UUID: {}", duplicate.note_id))
                        .child(controls(
                            "duplicate-path",
                            "Observed path",
                            self.relationships.duplicate_path,
                            duplicate.paths.len(),
                            Selection::DuplicatePath,
                            cx,
                        ));
                    if let Some(path) = duplicate.paths.get(self.relationships.duplicate_path) {
                        content = content.child(path.clone());
                    }
                }
            }
        } else {
            content = content.child("Select Refresh to inspect saved relationships.");
        }
        Some(
            div()
                .h(px(420.))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .border_t_1()
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_1()
                        .p_1()
                        .child("Relationships")
                        .child(
                            Button::new("relationship-page-previous")
                                .label("Previous page")
                                .compact()
                                .disabled(
                                    self.inspection_blocked()
                                        || ai.relationships_loading()
                                        || previous.is_none(),
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(offset) =
                                        this.ai.as_ref().unwrap().relationship_previous_offset()
                                    {
                                        this.refresh_relationship_page(offset, cx)
                                    }
                                })),
                        )
                        .child(
                            Button::new("relationship-page-next")
                                .label("Next page")
                                .compact()
                                .disabled(
                                    self.inspection_blocked()
                                        || ai.relationships_loading()
                                        || next.is_none(),
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(offset) =
                                        this.ai.as_ref().unwrap().relationship_next_offset()
                                    {
                                        this.refresh_relationship_page(offset, cx)
                                    }
                                })),
                        )
                        .child(
                            Button::new("relationship-page-refresh")
                                .label("Refresh")
                                .compact()
                                .disabled(self.inspection_blocked() || ai.relationships_loading())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.refresh_relationship_page(0, cx)
                                })),
                        )
                        .child(
                            Button::new("close-relationships")
                                .label("Close")
                                .compact()
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.close_relationship_page(cx)),
                                ),
                        ),
                )
                .child(content.test_support())
                .into_any_element(),
        )
    }
}
