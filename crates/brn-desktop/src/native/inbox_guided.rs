//! Guided Inbox reading and explicit proposal gestures over the existing workflow.
use super::*;
use brn_workflow::{
    inbox::{InboxAvailability, InboxKind, InboxOriginal},
    inbox_processing::InboxSourceRequest,
    proposals::{ProposalRecord, ProposalState},
};
use gpui_kit::{
    AnyElement, TestSupportExt,
    base::Disableable,
    component::{Selectable, input::Textarea},
};

pub(super) fn inspection_source<'a>(
    extraction: &'a brn_intake::Extraction,
    selected: Option<&str>,
) -> Option<&'a brn_intake::SourceNode> {
    match selected {
        Some(id) => extraction.sources.iter().find(|source| source.id == id),
        None => extraction
            .sources
            .iter()
            .find(|source| source.parent.is_none()),
    }
}
fn status(state: ProposalState) -> &'static str {
    match state {
        ProposalState::Draft => "Needs review",
        ProposalState::Rejected => "Rejected",
        ProposalState::Applying => "Applying · finalization pending",
        ProposalState::Uncertain => "Needs recovery inspection",
        ProposalState::Applied => "Applied · saved version",
    }
}
fn excerpt(text: &str) -> String {
    let mut value: String = text.chars().take(320).collect();
    if text.chars().count() > 320 {
        value.push('…');
    }
    value
}
fn availability(value: &InboxAvailability) -> &'static str {
    match value {
        InboxAvailability::Available => "Original retained",
        InboxAvailability::RemovedRetained => "Removed · exact copy retained",
        InboxAvailability::Missing => "Original missing · saved reading may remain",
        InboxAvailability::Changed => "Original changed · saved reading is historical",
        InboxAvailability::Unavailable => "Original unavailable · saved reading may remain",
    }
}

impl Desktop {
    fn guided_owns_analysis(
        &self,
        intake: Option<&brn_workflow::inbox_actions::InboxIntakeBinding>,
        source: Option<&brn_workflow::proposals::SourceVersion>,
    ) -> bool {
        let ai = self.ai.as_ref().unwrap();
        ai.proposals.iter().any(|record| {
            let Some(binding) = &record.draft.inbox_source else {
                return false;
            };
            if Some(binding.original.capture.id) != ai.inbox_queue.guided.selected {
                return false;
            }
            if let Some(intake) = intake {
                return intake.source_proposal.id == record.draft.id
                    && binding.extraction.as_ref().is_some_and(|receipt| {
                        receipt.snapshot_id == intake.snapshot_id
                            && receipt.snapshot_sha256 == intake.snapshot_sha256
                    });
            }
            source.is_some_and(|source| {
                record.draft.changes.first().is_some_and(|change| {
                    change.path() == source.path
                        && change.text().is_some_and(|text| {
                            brn_intake::digest(text.as_bytes()) == source.fingerprint.sha256
                        })
                })
            })
        })
    }
    pub(super) fn sync_guided_inbox_defaults(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let item = self
            .ai
            .as_ref()
            .unwrap()
            .inbox_queue
            .selected
            .as_ref()
            .filter(|read| {
                Some(read.item.capture.id) == self.ai.as_ref().unwrap().inbox_queue.guided.selected
            })
            .map(|read| read.item.clone());
        let Some(item) = item else {
            return;
        };
        if self.inbox.guided_defaults_for == Some(item.capture.id) {
            return;
        }
        if let Some(previous) = self.inbox.guided_defaults_for {
            self.inbox.guided_source_inputs.insert(
                previous,
                (
                    self.inbox.source_title.read(cx).value().to_string(),
                    self.inbox.source_path.read(cx).value().to_string(),
                ),
            );
        }
        self.inbox.guided_defaults_for = Some(item.capture.id);
        self.inbox.guided_attachment = None;
        self.inbox.guided_proposals = false;
        let mut slug = String::new();
        for character in item.capture.title.chars() {
            if slug.len() >= 64 {
                break;
            }
            if character.is_ascii_alphanumeric() {
                slug.push(character.to_ascii_lowercase());
            } else if !slug.ends_with('-') {
                slug.push('-');
            }
        }
        let slug = slug.trim_matches('-');
        let slug = if slug.is_empty() { "original" } else { slug };
        let path = format!(
            "inbox-{slug}-{}.md",
            &item.capture.id.simple().to_string()[..8]
        );
        let (title, path) = self
            .inbox
            .guided_source_inputs
            .get(&item.capture.id)
            .cloned()
            .unwrap_or((item.capture.title, path));
        self.inbox
            .source_title
            .update(cx, |input, cx| input.set_value(title, window, cx));
        self.inbox
            .source_path
            .update(cx, |input, cx| input.set_value(path, window, cx));
    }

    fn guided_source_input(
        &mut self,
        expected: Option<(Uuid, brn_workflow::inbox_processing::InboxCandidateRequest)>,
        investigate: bool,
        cx: &mut Context<Self>,
    ) {
        if self.inbox_blocked() {
            return;
        }
        let ai = self.ai.as_ref().unwrap();
        let Some((expected_item, expected_candidate)) = expected else {
            return;
        };
        if ai.inbox_queue.guided.selected != Some(expected_item)
            || ai.inbox_queue.preview.as_ref().is_none_or(|preview| {
                preview.original.capture.id != expected_item
                    || preview.request != expected_candidate
            })
        {
            return;
        }
        if let Some(source_id) = ai.guided_source_record().map(|record| record.draft.id) {
            if investigate {
                if let Some(command) = self
                    .ai
                    .as_mut()
                    .unwrap()
                    .begin_guided_investigation(source_id)
                {
                    self.inbox.guided_proposals = true;
                    self.simple_send(command, cx);
                }
            } else {
                self.simple_leave(simple::EditorTransition::Review(source_id), cx);
            }
            cx.notify();
            return;
        }
        let Some(preview) =
            ai.inbox_queue.preview.as_ref().filter(|preview| {
                Some(preview.original.capture.id) == ai.inbox_queue.guided.selected
            })
        else {
            return;
        };
        let request = InboxSourceRequest {
            candidate: preview.request.clone(),
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: self.inbox.source_path.read(cx).value().to_string(),
            title: self.inbox.source_title.read(cx).value().to_string(),
        };
        if let Err(error) = request.validate() {
            self.ai.as_mut().unwrap().inbox_queue.source_error = Some(error.message);
            cx.notify();
            return;
        }
        if let Some(command) = self
            .ai
            .as_mut()
            .unwrap()
            .begin_guided_source(request, investigate)
        {
            self.inbox.guided_proposals = true;
            self.simple_send(command, cx);
        }
        cx.notify();
    }

    fn guided_proposal_card(&self, record: &ProposalRecord, cx: &mut Context<Self>) -> AnyElement {
        let id = record.draft.id;
        let kind = if record.draft.inbox_source.is_some() {
            "Source"
        } else if !record.draft.action_changes.is_empty() {
            "Action"
        } else if record.draft.inbox_knowledge.is_some() {
            "Knowledge"
        } else {
            "Note"
        };
        let reason = record.draft.action_changes.first().map(|change| excerpt(&change.data().description))
            .or_else(|| record.comments.first().map(|comment| excerpt(&comment.text)))
            .unwrap_or_else(|| if record.draft.inbox_source.is_some() {
                "Preserves the original wording and retained pictures for later inspection.".into()
            } else { "Suggested change based on the selected evidence. Review its full wording and quoted support.".into() });
        let mut card = div()
            .id(format!("guided-proposal-{id}"))
            .test_support()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .border_1()
            .border_color(theme::color(self.palette().line))
            .child(format!(
                "{kind} · {} · version {}",
                status(record.state),
                record.version
            ))
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(record.draft.title.clone()),
            )
            .child(reason);
        if let Some(binding) = &record.draft.inbox_knowledge {
            for citation in &binding.intake_citations {
                card = card.child(format!("Evidence excerpt: “{}”", excerpt(&citation.quote)));
            }
            for citation in &binding.citations {
                card = card.child(format!(
                    "Saved evidence excerpt: “{}”",
                    excerpt(&citation.quote)
                ));
            }
        }
        card.child(
            Button::new(format!("guided-review-{id}"))
                .label("Open exact review")
                .disabled(self.inbox_blocked())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.simple_leave(simple::EditorTransition::Review(id), cx)
                })),
        )
        .into_any_element()
    }

    fn render_guided_paste(&self, cx: &mut Context<Self>) -> AnyElement {
        let blocked = self.inbox_blocked();
        let ai = self.ai.as_ref().unwrap();
        let mut kinds = div().flex().flex_wrap().gap_1();
        for (kind, label) in [
            (InboxKind::Text, "Text"),
            (InboxKind::Markdown, "Markdown"),
            (InboxKind::Email, "Email text"),
            (InboxKind::Teams, "Teams copy"),
        ] {
            kinds = kinds.child(
                Button::new(format!("guided-paste-kind-{kind:?}"))
                    .label(label)
                    .selected(self.inbox.kind == kind)
                    .disabled(blocked || ai.capture_pending())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.inbox_blocked() {
                            this.inbox.kind = kind;
                            cx.notify();
                        }
                    })),
            );
        }
        div()
            .id("guided-paste-form")
            .test_support()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                Textarea::new(&self.inbox.title)
                    .aria_label("Title for pasted Inbox text")
                    .disabled(blocked),
            )
            .child(kinds)
            .child(
                div().h(px(160.)).child(
                    Editor::new(&self.inbox.body)
                        .h_full()
                        .aria_label("Paste exact Inbox text")
                        .disabled(blocked),
                ),
            )
            .child(
                Button::new("guided-import-text")
                    .label("Import and read text")
                    .disabled(blocked || ai.capture_pending())
                    .on_click(cx.listener(|this, _, _, cx| this.capture_inbox_input(cx))),
            )
            .into_any_element()
    }

    pub(super) fn render_inbox(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if self.inbox.guided_advanced {
            return div()
                .size_full()
                .flex()
                .flex_col()
                .child(
                    Button::new("guided-back-from-tools")
                        .label("Back to Inbox reading")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.inbox.guided_advanced = false;
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h(px(0.))
                        .child(self.render_inbox_advanced(cx)),
                )
                .into_any_element();
        }
        self.render_guided_inbox(cx)
    }
    fn render_guided_inbox(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let ai = self.ai.as_ref().unwrap();
        let queue = &ai.inbox_queue;
        let blocked = self.inbox_blocked();
        let busy = ai.guided_source_busy() || ai.source_pending();
        let mut list = div()
            .id("guided-inbox-items")
            .test_support()
            .track_scroll(&self.inbox.guided_list_scroll)
            .w(px(250.))
            .flex_shrink_0()
            .min_h(px(0.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .border_r_1()
            .border_color(theme::color(self.palette().line))
            .child("Retained originals");
        let mut items: Vec<_> = queue
            .page
            .as_ref()
            .into_iter()
            .flat_map(|page| &page.entries)
            .map(|entry| (&entry.item, availability(&entry.availability)))
            .collect();
        if let Some(captured) = &queue.capture_result
            && !items
                .iter()
                .any(|(item, _)| item.capture.id == captured.capture.id)
        {
            items.insert(0, (captured, "Imported original retained"));
        }
        if let Some(selected) = &queue.guided.selected_item
            && !items
                .iter()
                .any(|(item, _)| item.capture.id == selected.capture.id)
        {
            items.insert(0, (selected, "Selected retained original"));
        }
        if items.is_empty() {
            list = list.child("Import an email or document to begin.");
        }
        for (item, availability_label) in items {
            let id = item.capture.id;
            list = list.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        Button::new(format!("guided-item-{id}"))
                            .label(item.capture.title.clone())
                            .selected(queue.guided.selected == Some(id))
                            .disabled(blocked || busy)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.inbox_blocked() {
                                    return;
                                }
                                if let Some(command) =
                                    this.ai.as_mut().unwrap().select_guided_inbox(id)
                                {
                                    this.simple_send(command, cx);
                                }
                                cx.notify();
                            })),
                    )
                    .child(div().text_sm().child(availability_label)),
            );
        }
        if let Some(page) = &queue.page {
            for issue in &page.issues {
                list = list.child(issue.message.clone());
            }
        }
        list = list.child(
            Button::new("guided-inbox-refresh")
                .label("Refresh originals")
                .disabled(blocked || ai.inbox_loading() || busy)
                .on_click(cx.listener(|this, _, _, cx| {
                    if !this.inbox_blocked()
                        && let Some(command) = this.ai.as_mut().unwrap().refresh_inbox()
                    {
                        this.simple_send(command, cx);
                    }
                    cx.notify();
                })),
        );
        if queue
            .page
            .as_ref()
            .is_some_and(|page| page.next_after.is_some())
        {
            list = list.child(
                Button::new("guided-inbox-next")
                    .label("More originals")
                    .disabled(blocked || ai.inbox_loading() || busy)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !this.inbox_blocked()
                            && let Some(command) = this.ai.as_mut().unwrap().next_inbox_page()
                        {
                            this.simple_send(command, cx);
                        }
                        cx.notify();
                    })),
            );
        }
        let preview = queue
            .preview
            .as_ref()
            .filter(|preview| Some(preview.original.capture.id) == queue.guided.selected);
        let title = queue
            .selected
            .as_ref()
            .filter(|read| Some(read.item.capture.id) == queue.guided.selected)
            .map_or("Choose an original", |read| {
                read.item.capture.title.as_str()
            });
        let mut focus = div()
            .id("guided-inbox-focus")
            .test_support()
            .track_scroll(&self.inbox.guided_read_scroll)
            .flex_1()
            .min_w(px(0.))
            .min_h(px(0.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(
                div()
                    .text_xl()
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(title.to_owned()),
            );
        if ai.inbox_loading() || ai.guided_inbox_loading() {
            focus = focus.child("Loading retained evidence…");
        }
        if ai.processing_pending() {
            focus = focus.child("Reading the retained original locally…").child(
                Button::new("guided-cancel-reading")
                    .label("Cancel reading")
                    .disabled(blocked)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !this.inbox_blocked()
                            && let Some(command) = this.ai.as_mut().unwrap().cancel_inbox_batch()
                        {
                            this.simple_send(command, cx);
                        }
                        cx.notify();
                    })),
            );
        }
        if let Some(batch) = &queue.batch {
            for (item, entry) in batch
                .request
                .items
                .iter()
                .zip(&batch.entries)
                .filter(|(item, _)| Some(item.capture.id) == queue.guided.selected)
            {
                let _ = item;
                if let brn_workflow::inbox_processing::InboxProcessOutcome::Failed { code } =
                    &entry.outcome
                {
                    focus = focus.child(format!("Reading did not complete ({code}). The original remains retained; inspect details or try reading it again."));
                }
            }
        }
        if queue.guided.snapshots.len() > 1 {
            focus = focus.child("Choose an exact saved reading version. This does not reread or refresh the original.");
            for (index, snapshot) in queue.guided.snapshots.iter().enumerate() {
                let id = snapshot.id;
                focus = focus.child(
                    Button::new(format!("guided-saved-reading-{index}"))
                        .label(format!("Saved reading version {}", index + 1))
                        .selected(queue.guided.snapshot_id == Some(id))
                        .disabled(blocked || busy)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if !this.inbox_blocked()
                                && this.ai.as_mut().unwrap().select_saved_inbox_extraction(id)
                            {
                                this.sync_inbox_widgets(window, cx);
                            }
                            cx.notify();
                        })),
                );
            }
        } else if queue.guided.snapshot_id.is_some() {
            focus = focus.child("Exact saved reading · historical evidence");
        }
        let propose_context =
            preview.map(|preview| (preview.original.capture.id, preview.request.clone()));
        let save_context = propose_context.clone();
        let ready = preview.is_some();
        let retained_extraction = preview.is_some_and(|preview| preview.extraction.is_some());
        let ai_ready = ai.selection.is_some() && ai.effort.is_some();
        let existing_source = ai.guided_source_record().is_some();
        let source_unusable = ai.guided_source_record().is_some_and(|record| {
            !matches!(record.state, ProposalState::Draft | ProposalState::Applied)
        });
        let ambiguous_source = preview.is_some_and(|preview| {
            ai.guided_related_proposals()
                .iter()
                .filter(|record| {
                    record.draft.inbox_source.as_ref().is_some_and(|binding| {
                        binding.batch_id == preview.request.batch_id
                            && binding.index == preview.request.index
                    })
                })
                .count()
                > 1
        });
        focus = focus.child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(
                    Button::new("guided-propose-notes")
                        .label("Propose notes")
                        .disabled(
                            blocked
                                || !ready
                                || !retained_extraction
                                || busy
                                || !ai.vault_bound
                                || !ai.can_ask()
                                || source_unusable
                                || ambiguous_source,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.guided_source_input(propose_context.clone(), true, cx)
                        })),
                )
                .child(
                    Button::new("guided-save-source")
                        .label(if existing_source {
                            "Review Source"
                        } else {
                            "Save as Source only"
                        })
                        .disabled(
                            blocked
                                || !ready
                                || busy
                                || !ai.vault_bound
                                || ai.application_busy()
                                || ambiguous_source,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.guided_source_input(save_context.clone(), false, cx)
                        })),
                )
                .child(
                    Button::new("guided-source-options")
                        .label("Source title and destination")
                        .disabled(blocked || busy || existing_source)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.inbox.guided_source_options = !this.inbox.guided_source_options;
                            if this.inbox.guided_source_options {
                                this.inbox.guided_advanced = false;
                            }
                            cx.notify();
                        })),
                ),
        );
        if ready {
            focus = focus.child(
                "Both actions prepare review proposals. Files change only after exact approval.",
            );
            if let Some(source) = ai.guided_source_record() {
                let path = source
                    .draft
                    .changes
                    .first()
                    .map(|change| change.path())
                    .unwrap_or("Destination unavailable");
                focus = focus.child(format!(
                    "Source title: {} · destination: {path}. Edit retained Source in its exact review.",
                    source.draft.title
                ));
            } else {
                focus = focus.child(format!(
                    "Source title: {} · destination: {}",
                    self.inbox.source_title.read(cx).value(),
                    self.inbox.source_path.read(cx).value()
                ));
            }
        }
        if ready && !retained_extraction {
            focus = focus.child("Save this text as a Source first, then use saved Source investigation in advanced tools.");
        }
        if ai
            .guided_source_record()
            .is_some_and(|record| record.state == ProposalState::Applied)
        {
            focus = focus.child("This Source is already approved. Propose notes investigates its retained email and pictures and prepares new proposals for review.");
        }
        if ambiguous_source {
            focus = focus.child("More than one Source review matches this reading. Open the existing Source cards and resolve the exact review before proposing further changes.");
        }
        if !ai_ready {
            focus = focus.child("Choose a provider, model and reasoning effort in Settings to propose notes. Reading and saving a Source work offline.");
        }
        if !ai.vault_bound {
            focus = focus.child("Bind a vault before retaining Source or knowledge proposals.");
        }
        if self.inbox.guided_source_options && !existing_source {
            focus = focus
                .child(
                    Textarea::new(&self.inbox.source_title)
                        .aria_label("Source proposal title")
                        .disabled(blocked || busy),
                )
                .child(
                    Textarea::new(&self.inbox.source_path)
                        .aria_label("Source destination Markdown path")
                        .disabled(blocked || busy),
                );
        }
        if busy {
            focus = focus.child(
                "Preparing the Source review. Any investigation follows your explicit request.",
            );
        }
        let current_active = ai.active.as_ref().filter(|active| {
            active.request.inbox().is_some_and(|request| {
                self.guided_owns_analysis(
                    request.intake.as_ref(),
                    request.source.as_ref().map(|source| &source.source),
                )
            })
        });
        if let Some(active) = current_active {
            focus = focus
                .child(if active.stopping {
                    "Stopping investigation · local finalization pending"
                } else {
                    "Investigation running · suggestions require separate review"
                })
                .child(
                    Button::new("guided-stop-investigation")
                        .label("Stop investigation")
                        .disabled(blocked || active.stopping)
                        .on_click(cx.listener(|this, _, _, cx| this.cancel_running(cx))),
                );
        }
        for error in [
            &queue.guided.error,
            &queue.error,
            &queue.source_error,
            &queue.capture_error,
        ]
        .into_iter()
        .flatten()
        {
            focus = focus.child(error.clone());
        }
        let current_analysis = ai.inbox_analysis.record.as_ref().is_some_and(|record| {
            self.guided_owns_analysis(
                record.job.capture.intake.as_ref(),
                record.job.capture.source.as_ref(),
            )
        }) || ai
            .inbox_analysis
            .request
            .as_ref()
            .filter(|request| Some(request.id) == ai.inbox_analysis.analysis_id)
            .is_some_and(|request| {
                self.guided_owns_analysis(
                    request.intake.as_ref(),
                    request.source.as_ref().map(|source| &source.source),
                )
            });
        if current_analysis {
            for error in [&ai.inbox_analysis.source_error, &ai.inbox_analysis.error]
                .into_iter()
                .flatten()
            {
                focus = focus.child(error.clone());
            }
        }
        focus = focus.child(
            div()
                .flex()
                .gap_2()
                .child(
                    Button::new("guided-read-tab")
                        .label("Read")
                        .selected(!self.inbox.guided_proposals)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.inbox.guided_proposals = false;
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("guided-proposals-tab")
                        .label("Proposed notes")
                        .selected(self.inbox.guided_proposals)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.inbox.guided_proposals = true;
                            cx.notify();
                        })),
                ),
        );
        if self.inbox.guided_proposals {
            let cards = ai.guided_related_proposals();
            if cards.is_empty() {
                focus = focus.child("No suggestions retained for this original yet. Propose notes explicitly, or save only its Source.");
            }
            for record in cards {
                focus = focus.child(self.guided_proposal_card(record, cx));
            }
        } else if let Some(preview) = preview {
            if let Some(extraction) = &preview.extraction {
                let mut attachments = div().flex().flex_wrap().gap_1();
                attachments = attachments.child(
                    Button::new("guided-read-all")
                        .label("All extracted content")
                        .selected(self.inbox.guided_attachment.is_none())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.inbox.guided_attachment = None;
                            cx.notify();
                        })),
                );
                for (index, source) in
                    extraction.sources.iter().enumerate().filter(|(_, source)| {
                        source.parent.is_some()
                            && source.name != "unnamed MIME part"
                            && source.status != "container"
                            && source.status != "retained-inline"
                    })
                {
                    let id = source.id.clone();
                    attachments = attachments.child(
                        Button::new(format!("guided-attachment-{index}"))
                            .label(source.name.clone())
                            .selected(self.inbox.guided_attachment.as_ref() == Some(&id))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.inbox.guided_attachment = Some(id.clone());
                                cx.notify();
                            })),
                    );
                }
                focus = focus.child(attachments).child(super::inbox_reader::render(
                    extraction,
                    &self.inbox.intake_images,
                    self.inbox.guided_attachment.as_deref(),
                    cx,
                ));
                if let Some(original) =
                    inspection_source(extraction, self.inbox.guided_attachment.as_deref())
                {
                    let id = original.id.clone();
                    let hash = brn_intake::digest(&original.bytes);
                    focus = focus.child(Button::new("guided-inspect-original").label(format!("Inspect {} with Quick Look", original.name))
                        .disabled(blocked || !matches!(original.media_type.as_str(), "message/rfc822" | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"))
                        .on_click(cx.listener(move |this, _, _, cx| this.inspect_extraction_original(&id, hash, false, cx))));
                }
            } else {
                focus = focus.child(super::inbox_reader::safe_markdown(
                    "guided-text-reading".into(),
                    preview.markdown.clone(),
                    cx,
                ));
            }
        } else if let Some(read) = &queue.selected {
            focus = focus.child(match &read.original {
                InboxOriginal::Available { .. } | InboxOriginal::AvailableBinary { .. } => "No saved reading selected. Read this original locally to inspect its content.",
                InboxOriginal::Missing => "The original is missing. Choose an available saved reading above.",
                InboxOriginal::Changed { .. } => "The original has changed. Any saved reading remains historical evidence.",
                InboxOriginal::RemovedRetained { .. } => "The original was deliberately removed. Saved readings remain historical evidence.",
                InboxOriginal::Unavailable { .. } => "The original is unavailable. Choose an available saved reading above.",
            });
        } else {
            focus = focus.child("Import a file or choose a retained original. Reading does not start an AI request.");
        }
        if self.inbox.original_preview.is_some() {
            focus = focus.child(
                Button::new("guided-close-original-preview")
                    .label("Close original preview")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.inbox.original_preview = None;
                        cx.notify();
                    })),
            );
        }
        if queue.selected.is_some() {
            focus = focus.child(
                Button::new("guided-read-original")
                    .label(if ready {
                        "Read original again"
                    } else {
                        "Read original locally"
                    })
                    .disabled(
                        blocked || ai.processing_pending() || busy || ai.guided_inbox_loading(),
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !this.inbox_blocked()
                            && let Some(command) = this.ai.as_mut().unwrap().read_guided_original()
                        {
                            this.simple_send(command, cx);
                        }
                        cx.notify();
                    })),
            );
        }
        focus = focus.child(
            Button::new("guided-evidence-disclosure")
                .label(if self.inbox.guided_evidence {
                    "Hide evidence details"
                } else {
                    "Evidence details"
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.inbox.guided_evidence = !this.inbox.guided_evidence;
                    if this.inbox.guided_evidence {
                        this.inbox.guided_advanced = false;
                    }
                    cx.notify();
                })),
        );
        if self.inbox.guided_evidence
            && let Some(preview) = preview
        {
            focus = focus.child(
                div().h(px(260.)).child(
                    Editor::new(&self.inbox.preview)
                        .h_full()
                        .readonly(true)
                        .aria_label("Exact extraction Markdown evidence"),
                ),
            );
            if let Some(extraction) = &preview.extraction {
                focus = focus.child(self.render_intake_extraction(extraction, false, blocked, cx));
            }
        }
        focus = focus.child(
            Button::new("guided-manage-disclosure")
                .label(if self.inbox.guided_manage {
                    "Hide original management"
                } else {
                    "Manage original / Batch tools"
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.inbox.guided_manage = !this.inbox.guided_manage;
                    cx.notify();
                })),
        );
        if self.inbox.guided_manage {
            focus = focus.child(self.render_inbox_copy(cx)).child(
                Button::new("guided-advanced-disclosure")
                    .label(if self.inbox.guided_advanced {
                        "Hide advanced batch and analysis tools"
                    } else {
                        "Advanced batch and analysis tools"
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.inbox.guided_advanced = !this.inbox.guided_advanced;
                        if this.inbox.guided_advanced {
                            this.inbox.guided_paste = false;
                            this.inbox.guided_evidence = false;
                            this.inbox.guided_source_options = false;
                        }
                        cx.notify();
                    })),
            );
        }
        let mut root = div()
            .id("inbox-guided-content")
            .test_support()
            .size_full()
            .flex()
            .flex_col()
            .min_h(px(0.))
            .font_family(if cfg!(target_os = "macos") {
                "Helvetica Neue"
            } else if cfg!(target_os = "windows") {
                "Segoe UI"
            } else {
                "DejaVu Sans"
            })
            .text_size(px(14.))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .p_3()
                    .flex_shrink_0()
                    .child(div().text_xl().child("Inbox"))
                    .child(
                        Button::new("guided-import-file")
                            .label("Import EML or DOCX")
                            .disabled(blocked || self.choosing_file || ai.capture_pending())
                            .on_click(cx.listener(|this, _, _, cx| this.choose_intake_file(cx))),
                    )
                    .child(
                        Button::new("guided-paste-disclosure")
                            .label(if self.inbox.guided_paste {
                                "Hide paste text"
                            } else {
                                "Paste text"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.inbox.guided_paste = !this.inbox.guided_paste;
                                if this.inbox.guided_paste {
                                    this.inbox.guided_advanced = false;
                                }
                                cx.notify();
                            })),
                    ),
            );
        root = root.child(
            div()
                .px_3()
                .pb_2()
                .child("Import → Read email and attachments → Review proposed notes → Approve"),
        );
        if self.inbox.guided_paste {
            root = root.child(div().p_3().child(self.render_guided_paste(cx)));
        }
        if ai.capture_pending() {
            root = root.child("Importing exact original…");
        }
        root.child(div().flex().flex_1().min_h(px(0.)).child(list).child(focus))
            .into_any_element()
    }
}
