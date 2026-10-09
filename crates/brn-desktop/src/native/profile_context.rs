//! Bounded exact saved-profile presentation; supporting notes use guarded navigation.
use super::*;
use brn_workflow::knowledge::{
    EdgeOrigin, EvidenceEndpoint, IdentityOutcome, ProfileContext, ProfileLens,
};
use brn_workflow::library::KnowledgeScope;
use gpui_kit::{AnyElement, TestSupportExt, base::Disableable};

pub(super) struct ProfileContextPane {
    pub(super) open: bool,
    snapshot: Option<ProfileContext>,
    action: usize,
    relationship: usize,
    proof: usize,
    pub(super) profile: Entity<EditorState>,
    pub(super) details: Entity<EditorState>,
    pub(super) quote: Entity<EditorState>,
    pub(super) scroll: ScrollHandle,
}
impl ProfileContextPane {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Desktop>) -> Self {
        Self {
            open: false,
            snapshot: None,
            action: 0,
            relationship: 0,
            proof: 0,
            profile: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            details: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            quote: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            scroll: ScrollHandle::new(),
        }
    }
}
fn exact_editor(editor: &Entity<EditorState>, label: &'static str) -> Editor {
    Editor::new(editor)
        .h_full()
        .readonly(true)
        .aria_label(label)
}
fn copy(id: &'static str, label: &'static str, editor: &Entity<EditorState>) -> Button {
    let editor = editor.clone();
    Button::new(id)
        .label(label)
        .compact()
        .on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                editor.read(cx).value().to_string(),
            ));
        })
}
fn digest(value: &[u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}
#[derive(Clone, Copy)]
enum Item {
    Action,
    Relationship,
    Proof,
}
fn item_controls(
    id: &'static str,
    label: &str,
    index: usize,
    total: usize,
    item: Item,
    cx: &mut Context<Desktop>,
) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .gap_1()
        .items_center()
        .child(format!(
            "{label} {} of {total}",
            if total == 0 { 0 } else { index + 1 }
        ))
        .child(
            Button::new(format!("profile-previous-{id}"))
                .label("Previous")
                .compact()
                .disabled(index == 0 || total == 0)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.step_profile_item(item, false, window, cx)
                })),
        )
        .child(
            Button::new(format!("profile-next-{id}"))
                .label("Next")
                .compact()
                .disabled(index.checked_add(1).is_none_or(|next| next >= total))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.step_profile_item(item, true, window, cx)
                })),
        )
        .into_any_element()
}
impl Desktop {
    pub(super) fn sync_profile_context_widgets(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let context = self.ai.as_ref().unwrap().profile_context.context.as_ref();
        let changed = self.profile_context.snapshot.as_ref() != context;
        if changed {
            self.profile_context.snapshot = context.cloned();
            self.profile_context.action = 0;
            self.profile_context.relationship = 0;
            self.profile_context.proof = 0;
            self.profile_context
                .scroll
                .set_offset(point(px(0.), px(0.)));
        }
        let profile = context.map_or("", |context| context.profile.text.as_str());
        if changed {
            let details = context.map_or_else(String::new, |context| {
                serde_json::to_string_pretty(context).expect("Profile context DTO serializes")
            });
            self.profile_context
                .details
                .update(cx, |editor, cx| editor.set_value(details, window, cx));
        }
        let quote = context
            .and_then(|context| context.relationships.get(self.profile_context.relationship))
            .and_then(|relationship| relationship.edge.evidence.get(self.profile_context.proof))
            .map_or("", |proof| proof.quote.as_str());
        for (editor, text) in [
            (&self.profile_context.profile, profile),
            (&self.profile_context.quote, quote),
        ] {
            if editor.read(cx).value().as_ref() != text {
                editor.update(cx, |editor, cx| editor.set_value(text, window, cx));
            }
        }
    }
    pub(super) fn inspect_profile(
        &mut self,
        lens: ProfileLens,
        actions: usize,
        relationships: usize,
        cx: &mut Context<Self>,
    ) {
        if self.inspection_blocked() || self.open_doc != Some(DocRef::SavedNote) {
            return;
        }
        if let Some(command) =
            self.ai
                .as_mut()
                .unwrap()
                .inspect_profile_context(lens, actions, relationships)
        {
            self.profile_context.open = true;
            self.simple_send(command, cx);
            cx.notify();
        }
    }
    pub(super) fn clear_profile_panel(&mut self) {
        self.profile_context.open = false;
        self.ai.as_mut().unwrap().clear_profile_context();
    }
    fn step_profile_item(
        &mut self,
        item: Item,
        next: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(context) = self.ai.as_ref().unwrap().profile_context.context.as_ref() else {
            return;
        };
        let pane = &mut self.profile_context;
        let (index, total) = match item {
            Item::Action => (&mut pane.action, context.actions.len()),
            Item::Relationship => (&mut pane.relationship, context.relationships.len()),
            Item::Proof => (
                &mut pane.proof,
                context
                    .relationships
                    .get(pane.relationship)
                    .map_or(0, |relationship| relationship.edge.evidence.len()),
            ),
        };
        let next = if next {
            index.checked_add(1)
        } else {
            index.checked_sub(1)
        };
        if let Some(next) = next.filter(|next| *next < total) {
            *index = next;
            if matches!(item, Item::Relationship) {
                pane.proof = 0;
            }
            self.sync_profile_context_widgets(window, cx);
            cx.notify();
        }
    }
    pub(super) fn profile_page(&mut self, actions: bool, next: bool, cx: &mut Context<Self>) {
        let ai = self.ai.as_ref().unwrap();
        let Some(context) = ai.profile_context.context.as_ref() else {
            return;
        };
        let lens = context.request.lens;
        let (action, relationship) = if actions {
            let Some(offset) = ai.profile_action_offset(next) else {
                return;
            };
            (offset, context.request.relationship_offset)
        } else {
            let Some(offset) = ai.profile_relationship_offset(next) else {
                return;
            };
            (context.request.action_offset, offset)
        };
        self.inspect_profile(lens, action, relationship, cx);
    }
    pub(super) fn open_profile_support(
        &mut self,
        path: String,
        scope: KnowledgeScope,
        cx: &mut Context<Self>,
    ) {
        if self.inspection_blocked() || !self.profile_context.open {
            return;
        }
        let Some(context) = self.ai.as_ref().unwrap().profile_context.context.as_ref() else {
            return;
        };
        let resolved = context.references.iter().any(|reference| {
            reference.resolution.outcome == IdentityOutcome::Unique
                && reference.matches.len() == 1
                && reference.matches[0].note.path == path
                && reference.matches[0].scope == Some(scope)
        });
        let endpoint = context.relationships.iter().any(|relationship| {
            relationship.edge.source.path == path && relationship.source_scope == scope
                || relationship.edge.target.path == path && relationship.target_scope == scope
        });
        if scope != KnowledgeScope::All && (resolved || endpoint) {
            self.simple_scoped_note(path, scope, cx);
        }
    }
    pub(super) fn render_profile_context(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.profile_context.open {
            return None;
        }
        let ai = self.ai.as_ref().unwrap();
        let pane = &self.profile_context;
        let blocked = self.inspection_blocked();
        let paging = blocked || ai.profile_context_loading() || !ai.profile_context_available();
        let mut content = div()
            .id("profile-context-scroll")
            .track_scroll(&pane.scroll)
            .overflow_y_scroll()
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .vertical_scrollbar(&pane.scroll)
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .gap_2()
            .p_2();
        if ai.profile_context_loading() {
            content = content
                .child("Reading saved profile context… Previous observation retained below.");
        }
        if let Some(error) = &ai.profile_context.error {
            content = content.child(format!("Context unavailable: {error}"));
        }
        if let Some(context) = &ai.profile_context.context {
            content = content.child(format!("{} context · {} · UUID {}", match context.request.lens { ProfileLens::Person => "Person", ProfileLens::Project => "Project" }, context.profile.source.path, context.request.note_id))
                .child("Explicit query lens; no saved profile type is inferred.")
                .child(format!("Saved profile SHA-256: {}", digest(&context.profile.source.fingerprint.sha256)))
                .child(div().h(px(160.)).flex_shrink_0().child(exact_editor(&pane.profile, "Full saved profile Markdown")))
                .child(copy("copy-profile-markdown", "Copy full saved profile", &pane.profile))
                .child(if context.complete { "Coverage: complete fresh observation; filesystem changes are not atomic." } else { "Coverage: incomplete; issues or duplicate identities prevent a certified empty result." })
                .child(format!("Actions: offset {} · {} displayed · {} matching retained Actions (including Completed)", context.request.action_offset, context.actions.len(), context.action_total))
                .child(item_controls("action", "Action", pane.action, context.actions.len(), Item::Action, cx));
            if let Some(action) = context.actions.get(pane.action) {
                content = content
                    .child(action.data.title.clone())
                    .child(format!(
                        "State: {:?} · UUID {} · version {}",
                        action.data.state, action.origin.id, action.version
                    ))
                    .child(format!(
                        "Related person: {:?} · Related project: {:?}",
                        action.data.related_person, action.data.related_project
                    ))
                    .child(format!(
                        "Explicit Sources: {:?} · Thread: {:?}",
                        action.data.sources, action.data.thread
                    ));
            }
            content = content.child("Displayed Action Source / thread resolutions (all exact records and diagnostics below)");
            for reference in &context.references {
                let roles = context
                    .actions
                    .iter()
                    .filter_map(|action| {
                        let source = action.data.sources.contains(&reference.resolution.note_id);
                        let thread = action.data.thread == Some(reference.resolution.note_id);
                        (source || thread).then(|| {
                            format!(
                                "{}: {}{}",
                                action.origin.id,
                                if source { "Source " } else { "" },
                                if thread { "Thread" } else { "" }
                            )
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                content = content.child(format!(
                    "{} · {:?} · {roles}",
                    reference.resolution.note_id, reference.resolution.outcome
                ));
                if reference.resolution.outcome == IdentityOutcome::Unique
                    && reference.matches.len() == 1
                {
                    let note = &reference.matches[0];
                    if let Some(scope) = note.scope.filter(|scope| *scope != KnowledgeScope::All) {
                        let path = note.note.path.clone();
                        content = content.child(
                            Button::new(format!(
                                "profile-reference-{}",
                                reference.resolution.note_id
                            ))
                            .label(format!(
                                "Open {} supporting note: {path}",
                                crate::ai::scope_name(scope)
                            ))
                            .compact()
                            .disabled(blocked)
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.open_profile_support(path.clone(), scope, cx)
                                },
                            )),
                        );
                    }
                }
                for note in &reference.matches {
                    content = content.child(format!(
                        "Observed {} · {} · SHA-256 {}",
                        note.note.path,
                        note.scope.map_or("Unknown scope", crate::ai::scope_name),
                        digest(&note.note.sha256)
                    ));
                }
                for issue in &reference.resolution.issues {
                    content =
                        content.child(format!("Reference issue {}: {}", issue.path, issue.reason));
                }
            }
            content = content
                .child(format!(
                    "Direct relationships: offset {} · {} displayed · {} total",
                    context.request.relationship_offset,
                    context.relationships.len(),
                    context.relationship_total
                ))
                .child(item_controls(
                    "relationship",
                    "Relationship",
                    pane.relationship,
                    context.relationships.len(),
                    Item::Relationship,
                    cx,
                ));
            if let Some(relationship) = context.relationships.get(pane.relationship) {
                let edge = &relationship.edge;
                let direction = match (
                    edge.source.note_id == context.request.note_id,
                    edge.target.note_id == context.request.note_id,
                ) {
                    (true, true) => "Self relationship",
                    (true, false) => "Outgoing from profile",
                    _ => "Incoming to profile",
                };
                content = content.child(direction).child(match edge.origin {
                    EdgeOrigin::ExplicitLink => "Explicit Markdown link",
                    EdgeOrigin::InferredProvenance => "Inferred provenance candidate",
                });
                for (role, endpoint, scope) in [
                    ("Source", &edge.source, relationship.source_scope),
                    ("Target", &edge.target, relationship.target_scope),
                ] {
                    let path = endpoint.path.clone();
                    content = content
                        .child(format!(
                            "{role}: {} · {} · UUID {} · SHA-256 {}",
                            crate::ai::scope_name(scope),
                            endpoint.path,
                            endpoint.note_id,
                            digest(&endpoint.sha256)
                        ))
                        .child(
                            Button::new(format!("profile-open-{role}"))
                                .label(format!(
                                    "Open {} {role} note: {path}",
                                    crate::ai::scope_name(scope)
                                ))
                                .compact()
                                .disabled(blocked)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.open_profile_support(path.clone(), scope, cx)
                                })),
                        );
                }
                content = content.child(item_controls(
                    "proof",
                    "Exact proof",
                    pane.proof,
                    edge.evidence.len(),
                    Item::Proof,
                    cx,
                ));
                if let Some(proof) = edge.evidence.get(pane.proof) {
                    content = content
                        .child(format!(
                            "Proof belongs to {} · saved byte range {}–{}",
                            match proof.endpoint {
                                EvidenceEndpoint::Source => "Source",
                                EvidenceEndpoint::Target => "Target",
                            },
                            proof.start_byte,
                            proof.end_byte
                        ))
                        .child(
                            div()
                                .h(px(140.))
                                .flex_shrink_0()
                                .child(relationships::quote_widget(&pane.quote)),
                        )
                        .child(copy("copy-profile-proof", "Copy exact quote", &pane.quote));
                }
            }
            for issue in &context.issues {
                content =
                    content.child(format!("Inspection issue {}: {}", issue.path, issue.reason));
            }
            for duplicate in &context.duplicates {
                content = content.child(format!(
                    "Duplicate UUID {}: {:?}",
                    duplicate.note_id, duplicate.paths
                ));
            }
            content = content.child("Complete context details: full Action records, references, relationship proofs and diagnostics")
                .child(div().h(px(240.)).flex_shrink_0().child(exact_editor(&pane.details, "Complete exact profile context JSON")))
                .child(copy("copy-profile-details", "Copy complete context details", &pane.details));
        } else if !ai.profile_context_loading() {
            content = content.child(
                "Choose Person context or Project context for a clean saved Current managed note.",
            );
        }
        let mut header = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .p_1()
            .child("Saved profile context");
        for (id, label, actions, next) in [
            (
                "profile-actions-previous",
                "Previous Actions page",
                true,
                false,
            ),
            ("profile-actions-next", "Next Actions page", true, true),
            (
                "profile-relationships-previous",
                "Previous relationships page",
                false,
                false,
            ),
            (
                "profile-relationships-next",
                "Next relationships page",
                false,
                true,
            ),
        ] {
            let available = if actions {
                ai.profile_action_offset(next)
            } else {
                ai.profile_relationship_offset(next)
            }
            .is_some();
            header = header.child(
                Button::new(id)
                    .label(label)
                    .compact()
                    .disabled(paging || !available)
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.profile_page(actions, next, cx)),
                    ),
            );
        }
        header = header
            .child(
                Button::new("profile-context-refresh")
                    .label("Refresh context")
                    .compact()
                    .disabled(paging || ai.profile_context.context.is_none())
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(context) =
                            this.ai.as_ref().unwrap().profile_context.context.as_ref()
                        {
                            this.inspect_profile(
                                context.request.lens,
                                context.request.action_offset,
                                context.request.relationship_offset,
                                cx,
                            );
                        }
                    })),
            )
            .child(
                Button::new("close-profile-context")
                    .label("Close context")
                    .compact()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.clear_profile_panel();
                        cx.notify();
                    })),
            );
        Some(
            div()
                .h(px(520.))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .border_t_1()
                .child(header)
                .child(content.test_support())
                .into_any_element(),
        )
    }
}
