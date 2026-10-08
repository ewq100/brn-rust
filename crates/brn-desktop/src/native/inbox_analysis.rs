//! Saved Source analysis presentation over the shared worker's retained proof.
use super::*;
use crate::ai::{AiState, provider_name, turn_label};
use brn_workflow::ReasoningEffort;
use gpui_kit::{AnyElement, TestSupportExt, base::Disableable, component::input::Textarea};

fn readonly(editor: &Entity<EditorState>, label: &'static str) -> Editor {
    Editor::new(editor)
        .h_full()
        .readonly(true)
        .aria_label(label)
}

fn copy(id: &'static str, label: &'static str, text: String) -> Button {
    Button::new(id).label(label).on_click(move |_, _, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text.clone()));
    })
}

fn fingerprint_hash(hash: &[u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Only the selected analysis owns this answer; changing inspection never shows
/// a different active turn's provisional or unfinalized text.
fn answer_text(ai: &AiState) -> &str {
    let id = ai.inbox_analysis.analysis_id;
    if let Some(active) = ai
        .active
        .as_ref()
        .filter(|active| active.request.inbox().is_some() && Some(active.request.id()) == id)
    {
        return &active.partial;
    }
    if let Some(turn) = ai.unsaved.as_ref().filter(|turn| Some(turn.id) == id) {
        return &turn.answer;
    }
    ai.inbox_analysis
        .record
        .as_ref()
        .and_then(|record| record.turn.as_ref())
        .map_or("", |turn| turn.answer.as_str())
}

impl Desktop {
    pub(super) fn sync_inbox_analysis_widgets(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(path) = self.inbox.analysis_path_target.take() {
            self.inbox
                .analysis_source_path
                .update(cx, |input, cx| input.set_value(path, window, cx));
        }
        let ai = self.ai.as_ref().unwrap();
        let source = ai
            .inbox_analysis
            .source
            .as_ref()
            .map_or("", |source| source.text.as_str());
        if self.inbox.analysis_source.read(cx).value().as_ref() != source {
            self.inbox
                .analysis_source
                .update(cx, |editor, cx| editor.set_value(source, window, cx));
        }
        let extraction = ai
            .inbox_analysis
            .retained_extraction
            .as_ref()
            .map_or("", |snapshot| snapshot.extraction.markdown.as_str());
        if self.inbox.retained_extraction.read(cx).value().as_ref() != extraction {
            self.inbox
                .retained_extraction
                .update(cx, |editor, cx| editor.set_value(extraction, window, cx));
        }
        let retained_source = ai
            .inbox_analysis
            .record
            .as_ref()
            .map_or("", |record| record.job.capture.source_text.as_str());
        if self
            .inbox
            .analysis_retained_source
            .read(cx)
            .value()
            .as_ref()
            != retained_source
        {
            self.inbox
                .analysis_retained_source
                .update(cx, |editor, cx| {
                    editor.set_value(retained_source, window, cx)
                });
        }
        let annotation = ai
            .inbox_analysis
            .annotation
            .as_ref()
            .and_then(|draft| match draft.changes.first() {
                Some(brn_workflow::proposals::DraftNoteChange::Replace { text, .. }) => {
                    Some(text.as_str())
                }
                _ => None,
            })
            .unwrap_or("");
        if self.inbox.visual_annotation.read(cx).value().as_ref() != annotation {
            self.inbox
                .visual_annotation
                .update(cx, |editor, cx| editor.set_value(annotation, window, cx));
        }
        let answer = answer_text(ai);
        if self.inbox.analysis_answer.read(cx).value().as_ref() != answer {
            self.inbox
                .analysis_answer
                .update(cx, |editor, cx| editor.set_value(answer, window, cx));
        }
    }

    fn analysis_source_matches_input(&self, cx: &App) -> bool {
        let ai = self.ai.as_ref().unwrap();
        ai.inbox_analysis.intake.is_some()
            || ai.inbox_analysis.source.as_ref().is_some_and(|source| {
                self.inbox.analysis_source_path.read(cx).value().as_ref() == source.source.path
            })
    }

    fn inspect_analysis_source(&mut self, cx: &mut Context<Self>) {
        if self.inbox_blocked() {
            return;
        }
        let path = self.inbox.analysis_source_path.read(cx).value().to_string();
        if let Some(command) = self
            .ai
            .as_mut()
            .unwrap()
            .inspect_inbox_analysis_source(path)
        {
            self.simple_send(command, cx);
        }
        cx.notify();
    }

    fn start_inbox_analysis(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.inbox_blocked() || !self.analysis_source_matches_input(cx) {
            return;
        }
        if let Some(command) = self.ai.as_mut().unwrap().analyze_inbox_source() {
            let id = command.0.to_string();
            self.inbox
                .analysis_id
                .update(cx, |input, cx| input.set_value(id, window, cx));
            self.simple_send(command, cx);
        }
        cx.notify();
    }

    fn inspect_analysis_input(&mut self, cx: &mut Context<Self>) {
        if self.inbox_blocked() {
            return;
        }
        let value = self.inbox.analysis_id.read(cx).value().to_string();
        let id = Uuid::parse_str(value.trim()).ok().filter(|id| !id.is_nil());
        let Some(id) = id else {
            self.ai.as_mut().unwrap().inbox_analysis.error =
                Some("Enter a nonempty, non-nil analysis UUID.".into());
            cx.notify();
            return;
        };
        self.inspect_retained_analysis(id, cx);
    }

    fn inspect_retained_analysis(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if self.inbox_blocked() {
            return;
        }
        if let Some(command) = self.ai.as_mut().unwrap().inspect_inbox_analysis(id) {
            self.simple_send(command, cx);
        }
        cx.notify();
    }

    pub(super) fn open_analysis_finding(
        &mut self,
        analysis: Uuid,
        id: Uuid,
        cx: &mut Context<Self>,
    ) {
        if self.inbox_blocked()
            || self
                .ai
                .as_ref()
                .unwrap()
                .inbox_analysis_finding(analysis, id)
                .is_none()
        {
            return;
        }
        self.simple_leave(
            super::simple::EditorTransition::Finding { analysis, id },
            cx,
        );
    }

    pub(super) fn render_inbox_analysis(&self, cx: &mut Context<Self>) -> AnyElement {
        let ai = self.ai.as_ref().unwrap();
        let view = &ai.inbox_analysis;
        let blocked = self.inbox_blocked();
        let mut panel = div()
            .id("inbox-analysis-panel")
            .flex()
            .flex_col()
            .gap_2()
            .child("Investigate captured evidence or a saved Source")
            .child("Investigation can start from an immutable private extraction before Source approval. Retain its Source proposal, then inspect the private binding below. Knowledge and Action drafts carry its exact Source prerequisite into review.")
            .child(Textarea::new(&self.inbox.analysis_source_path)
                .disabled(blocked)
                .aria_label("Saved Source path for analysis"))
            .child(Button::new("inbox-analysis-inspect-source")
                .label("Inspect Source")
                .disabled(blocked || !ai.vault_bound || ai.application_busy()
                    || ai.inbox_analysis_source_loading()
                    || self.inbox.analysis_source_path.read(cx).value().is_empty())
                .on_click(cx.listener(|this, _, _, cx| this.inspect_analysis_source(cx))));
        if let Some(prepared) = &ai.inbox_queue.prepared {
            let source_proposal_id = prepared.id;
            panel = panel.child(
                Button::new("inbox-analysis-inspect-private")
                    .label("Inspect private extraction before Source approval")
                    .disabled(
                        blocked || ai.application_busy() || ai.inbox_analysis_source_loading(),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.inbox_blocked()
                            && let Some(command) = this
                                .ai
                                .as_mut()
                                .unwrap()
                                .inspect_private_intake(source_proposal_id)
                        {
                            this.simple_send(command, cx);
                        }
                        cx.notify();
                    })),
            );
        }
        if let Some(intake) = &view.intake {
            panel = panel.child(div().id("inbox-analysis-private-binding").test_support()
                .child(format!("Private extraction {} · planned Source {} · proposal {} version {} · {} images / {} occurrences. Source approval is a prerequisite for applying dependent knowledge or Actions.", intake.snapshot_id, intake.source_path, intake.source_proposal.id, intake.source_proposal.version, intake.assets.len(), intake.occurrences.len())));
        }
        if ai.inbox_analysis_source_loading() {
            panel = panel.child("Inspecting saved Source…");
        }
        if let Some(error) = &view.source_error {
            panel = panel.child(format!("Source inspection: {error}"));
        }
        if let Some(source) = &view.source {
            panel = panel
                .child(format!(
                    "Inspected Source: {} · {} bytes",
                    source.source.path,
                    source.text.len()
                ))
                .child(format!(
                    "Saved SHA-256: {}",
                    fingerprint_hash(&source.source.fingerprint.sha256)
                ))
                .child(div().h(px(180.)).child(readonly(
                    &self.inbox.analysis_source,
                    "Complete inspected Source snapshot",
                )))
                .child(copy(
                    "inbox-analysis-copy-source",
                    "Copy inspected Source",
                    source.text.clone(),
                ));
            if !self.analysis_source_matches_input(cx) {
                panel = panel.child("The path input differs from this inspected snapshot. Inspect Source again before starting.");
            }
        }
        let selection = ai.selection.as_ref().map_or_else(
            || "Provider/model not acknowledged".to_owned(),
            |selection| {
                format!(
                    "{} / {}",
                    provider_name(selection.provider),
                    selection.model
                )
            },
        );
        let effort = ai
            .effort
            .map_or("not acknowledged", ReasoningEffort::as_str);
        panel = panel
            .child(format!("Selected for a new analysis: {selection} · effort: {effort}. Change these in Settings."))
            .child(Button::new("inbox-analysis-start")
                .label("Analyze knowledge + Actions")
                .disabled(blocked || !ai.can_analyze_inbox_source()
                    || !self.analysis_source_matches_input(cx))
                .on_click(cx.listener(|this, _, window, cx| this.start_inbox_analysis(window, cx))));
        panel = panel.child(
            Button::new("inbox-visual-inspect")
                .label("Inspect saved PNG")
                .disabled(
                    blocked
                        || !ai.ready
                        || !ai.vault_bound
                        || ai.application_busy()
                        || ai.inbox_analysis_source_loading()
                        || view.source.is_none()
                        || !self.analysis_source_matches_input(cx)
                        || ai.visual_analysis_pending(),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if this.inbox_blocked() || !this.analysis_source_matches_input(cx) {
                        return;
                    }
                    if let Some(command) = this.ai.as_mut().unwrap().inspect_inbox_visual() {
                        this.simple_send(command, cx);
                    }
                    cx.notify();
                })),
        );
        if let Some(visual) = &view.visual {
            if let Ok(proof) = visual.visual_proof() {
                panel = panel.child(super::visual::png_panel(
                    "inbox-saved-png",
                    &visual.bytes,
                    &proof,
                ));
            }
            panel = panel.child(div().id("inbox-visual-source-proof").test_support()
                .aria_label(super::visual::file_proof("Full Source proof", &visual.source.source))
                .child(super::visual::file_proof("Full Source proof", &visual.source.source)))
                .child(div().id("inbox-visual-asset-proof").test_support()
                .aria_label(super::visual::file_proof("Full PNG asset proof", &visual.asset))
                .child(super::visual::file_proof("Full PNG asset proof", &visual.asset)))
                .child(div().id("inbox-visual-interpretation-status").test_support()
                .aria_label("Source and asset approval preserves evidence. Interpretation remains pending or tentative until separately reviewed and exactly approved.")
                .child("Source and asset approval preserves evidence. Interpretation remains pending or tentative until separately reviewed and exactly approved."));
        }
        panel = panel
            .child(
                Button::new("inbox-visual-start")
                    .label("Interpret this PNG")
                    .disabled(
                        blocked
                            || !self.analysis_source_matches_input(cx)
                            || !ai.can_interpret_inbox_visual(),
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.inbox_blocked() || !this.analysis_source_matches_input(cx) {
                            return;
                        }
                        if let Some(command) = this.ai.as_mut().unwrap().interpret_inbox_visual() {
                            this.inbox.analysis_id.update(cx, |input, cx| {
                                input.set_value(command.0.to_string(), window, cx)
                            });
                            this.simple_send(command, cx);
                        }
                        cx.notify();
                    })),
            )
            .child(
                Button::new("inbox-visual-prepare")
                    .label("Prepare tentative annotation")
                    .disabled(
                        blocked
                            || !self.analysis_source_matches_input(cx)
                            || !ai.can_prepare_visual_annotation(),
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.inbox_blocked() || !this.analysis_source_matches_input(cx) {
                            return;
                        }
                        if let Some(command) = this.ai.as_mut().unwrap().prepare_visual_annotation()
                        {
                            this.simple_send(command, cx);
                        }
                        cx.notify();
                    })),
            );
        if let Some(error) = &view.visual_error {
            panel = panel.child(format!("Visual inspection or preparation: {error}"));
        }
        if view.annotation.is_some() {
            panel = panel.child("Tentative annotation candidate: complete Source text. No durable text changes before separate exact approval.")
                .child(div().h(px(220.)).child(readonly(&self.inbox.visual_annotation, "Complete tentative annotation Source candidate")))
                .child(Button::new("inbox-visual-create").label("Create annotation review")
                    .disabled(blocked || !self.analysis_source_matches_input(cx) || !ai.can_prepare_visual_annotation() || view.annotation_review.is_some())
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.inbox_blocked() || !this.analysis_source_matches_input(cx) { return; }
                        if let Some(command) = this.ai.as_mut().unwrap().create_visual_annotation() { this.simple_send(command, cx); }
                        cx.notify();
                    })));
        }
        if let Some(id) = view.annotation_review {
            panel = panel.child(
                Button::new("inbox-visual-open-review")
                    .label("Open annotation review for exact approval")
                    .disabled(blocked || !self.analysis_source_matches_input(cx))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.inbox_blocked()
                            || !this.analysis_source_matches_input(cx)
                            || this.ai.as_ref().unwrap().inbox_analysis.annotation_review
                                != Some(id)
                        {
                            return;
                        }
                        this.simple_leave(super::simple::EditorTransition::Review(id), cx);
                    })),
            );
        }
        if let Some(error) = &ai.selection_error {
            panel = panel.child(format!("Selection unavailable: {error}"));
        }
        if let Some(error) = &ai.effort_error {
            panel = panel.child(format!("Reasoning effort unavailable: {error}"));
        }
        panel = panel
            .child("Inspect a retained analysis")
            .child(
                Textarea::new(&self.inbox.analysis_id)
                    .disabled(blocked)
                    .aria_label("Retained Inbox analysis UUID"),
            )
            .child(
                Button::new("inbox-analysis-inspect")
                    .label("Inspect retained analysis")
                    .disabled(blocked || ai.inbox_analysis_loading())
                    .on_click(cx.listener(|this, _, _, cx| this.inspect_analysis_input(cx))),
            );
        if let Some(id) = view.analysis_id {
            panel = panel
                .child(format!("Selected analysis: {id}"))
                .child(copy(
                    "inbox-analysis-copy-id",
                    "Copy analysis UUID",
                    id.to_string(),
                ))
                .child(
                    Button::new("inbox-analysis-refresh")
                        .label("Refresh selected analysis")
                        .disabled(blocked || ai.inbox_analysis_loading())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.inspect_retained_analysis(id, cx)
                        })),
                );
        }
        if ai.inbox_analysis_loading() {
            panel = panel.child("Loading retained analysis…");
        }
        if let Some(error) = &view.error {
            panel = panel.child(format!("Analysis inspection: {error}"));
        }
        if let Some(active) = ai.active.as_ref().filter(|active| {
            active.request.inbox().is_some() && Some(active.request.id()) == view.analysis_id
        }) {
            panel = panel.child(format!(
                "{} / {} · effort: {} · {}",
                provider_name(active.request.selection().provider),
                active.request.selection().model,
                active
                    .request
                    .effort()
                    .map(ReasoningEffort::as_str)
                    .unwrap_or("unavailable"),
                if active.stopping {
                    "Stopping (not finalized)"
                } else {
                    "Streaming (provisional)"
                }
            ));
            if let Some(tool) = &active.tool {
                panel = panel.child(format!("Tool started: {tool}"));
            }
            panel = panel.child("Use the global Stop control to cancel the owned request.");
        }
        if let Some(turn) = ai
            .unsaved
            .as_ref()
            .filter(|turn| Some(turn.id) == view.analysis_id)
        {
            panel = panel.child(format!("{} / {} · effort: {} · Failed · in-memory partial · finalization not acknowledged",
                turn.provider, turn.model, turn.effort.as_deref().unwrap_or("unavailable")))
                .child("Copy this partial before closing or restarting. Further AI requests remain blocked until the workspace is reopened.");
        }
        if ai.retained_extraction_binding().is_some() {
            panel = panel.child(
                Button::new("inbox-inspect-retained-extraction")
                    .label("Inspect retained extraction")
                    .disabled(blocked || ai.application_busy() || ai.retained_extraction_loading())
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.inbox_blocked() {
                            return;
                        }
                        if let Some(command) =
                            this.ai.as_mut().unwrap().inspect_retained_extraction()
                        {
                            this.simple_send(command, cx);
                        }
                        cx.notify();
                    })),
            );
        }
        if ai.retained_extraction_loading() {
            panel = panel.child("Loading exact retained extraction…");
        }
        if let Some(error) = &view.extraction_error {
            panel = panel.child(format!("Retained extraction: {error}"));
        }
        if let Some(snapshot) = &view.retained_extraction {
            let checked_digest = ai
                .retained_extraction_binding()
                .filter(|(id, _)| *id == snapshot.id)
                .map(|(_, digest)| fingerprint_hash(&digest))
                .unwrap_or_else(|| "unavailable".into());
            panel = panel.child(div().id("retained-intake-extraction").test_support()
                .child(format!("Retained extraction {} · SHA-256 {} · original {}. Read-only historical evidence; original files and conversion are not required.", snapshot.id, checked_digest, snapshot.original.capture.title)))
                .child(div().h(px(300.)).flex_shrink_0().child(readonly(&self.inbox.retained_extraction, "Complete retained extraction Markdown")))
                .child(copy("copy-retained-extraction", "Copy full retained extraction", snapshot.extraction.markdown.clone()))
                .child(self.render_intake_extraction(&snapshot.extraction, true, blocked, cx));
        }
        if let Some(record) = &view.record {
            let capture = &record.job.capture;
            panel = panel
                .child(format!(
                    "Retained analysis: {} · {:?} · {} / {} · effort: {}",
                    capture.id, capture.purpose, capture.provider, capture.model, capture.effort
                ))
                .child(format!(
                    "Source captured for this analysis: {}",
                    capture.source_path()
                ))
                .child(format!(
                    "Captured SHA-256: {} · {} bytes",
                    fingerprint_hash(&capture.source_sha256()),
                    capture.source_text.len()
                ))
                .child(div().h(px(180.)).child(readonly(
                    &self.inbox.analysis_retained_source,
                    "Complete Source snapshot captured for retained analysis",
                )))
                .child(copy(
                    "inbox-analysis-copy-retained-source",
                    "Copy captured Source",
                    capture.source_text.clone(),
                ));
            if let Some(source) = &capture.source {
                panel = panel.child(super::visual::file_proof(
                    "Captured full Source proof",
                    source,
                ));
            } else if let Some(intake) = &capture.intake {
                panel = panel.child(format!(
                    "Immutable private snapshot {} · Source prerequisite {} version {}",
                    intake.snapshot_id, intake.source_proposal.id, intake.source_proposal.version
                ));
            }
            if let Some(asset) = &capture.visual_asset {
                let proof = super::visual::file_proof("Captured full PNG asset proof", asset);
                panel = panel.child(
                    div()
                        .id("inbox-visual-captured-asset-proof")
                        .test_support()
                        .aria_label(proof.clone())
                        .child(proof),
                );
            }
            if let Some(turn) = &record.turn {
                panel = panel.child(format!("Retained status: {}", turn_label(turn)));
                if let Some(code) = &turn.error_code {
                    panel = panel.child(format!("Safe failure category: {code}"));
                }
            } else {
                panel = panel.child("No retained turn recorded yet.");
            }
            if record.needs_semantic_review {
                panel = panel.child("Semantic review remains. This bounded analysis does not establish complete reconciliation.");
            }
            if record.proposals.is_empty() {
                panel = panel.child("No review proposals recorded. Semantic review remains.");
            }
            if !record.findings.is_empty() {
                panel = panel.child("Retained conflicts")
                    .child("These tentative findings retain opposing saved evidence. Resolve or Dismiss changes Needs Review only; knowledge changes require separate exact approval.");
            }
            for finding in &record.findings {
                let id = finding.draft.request.id;
                let analysis = capture.id;
                panel = panel.child(
                    div()
                        .id(format!("inbox-analysis-finding-{id}"))
                        .test_support()
                        .aria_label(format!(
                            "Conflict · {} · {} · {}",
                            super::findings::state_name(finding.state),
                            finding.draft.title,
                            finding.draft.summary
                        ))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(format!(
                            "Conflict · {}",
                            super::findings::state_name(finding.state)
                        ))
                        .child(finding.draft.title.clone())
                        .child(finding.draft.summary.clone())
                        .child(
                            Button::new(format!("inbox-analysis-open-finding-{id}"))
                                .label("Open in Needs Review")
                                .disabled(
                                    blocked || ai.inbox_analysis_finding(analysis, id).is_none(),
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.open_analysis_finding(analysis, id, cx)
                                })),
                        ),
                );
            }
            for proposal in &record.proposals {
                let id = proposal.draft.id;
                let kind = match (
                    proposal.draft.inbox_knowledge.is_some(),
                    !proposal.draft.changes.is_empty(),
                    !proposal.draft.action_changes.is_empty(),
                ) {
                    (true, _, true) => "Knowledge + Actions",
                    (true, _, false) => "Knowledge",
                    (false, true, true) => "Note + Actions",
                    (false, true, false) => "Note",
                    (false, false, _) => "Actions",
                };
                panel = panel.child(
                    div()
                        .id(format!("inbox-analysis-proposal-{id}"))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(format!(
                            "{kind} proposal · {} · {:?} · {id}",
                            proposal.draft.title, proposal.state
                        ))
                        .child(
                            Button::new(format!("inbox-analysis-open-proposal-{id}"))
                                .label("Open proposal review")
                                .disabled(blocked)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.simple_leave(
                                        super::simple::EditorTransition::Review(id),
                                        cx,
                                    );
                                })),
                        ),
                );
            }
        }
        if view.record.is_some()
            || ai.active.as_ref().is_some_and(|active| {
                active.request.inbox().is_some() && Some(active.request.id()) == view.analysis_id
            })
            || ai
                .unsaved
                .as_ref()
                .is_some_and(|turn| Some(turn.id) == view.analysis_id)
        {
            panel = panel
                .child(div().h(px(180.)).child(readonly(
                    &self.inbox.analysis_answer,
                    "Complete analysis answer or provisional partial",
                )))
                .child(copy(
                    "inbox-analysis-copy-answer",
                    "Copy analysis answer or partial",
                    answer_text(ai).to_owned(),
                ));
        }
        panel.test_support().into_any_element()
    }
}
