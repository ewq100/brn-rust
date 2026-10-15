//! Explicit contextual actions, separate from ordinary chat drafts and queueing.
use super::*;

impl ThreadsUi {
    fn context_work_idle(&self) -> bool {
        let queue = self.queue_state();
        self.worker.is_none() && queue.pending.is_none() && queue.queue.is_empty()
    }
    pub(super) fn can_edit_note_context(&self) -> bool {
        self.edit_note_blocked().is_none()
    }
    /// Why Edit note is unavailable, in the words its disabled control shows;
    /// `None` when the open saved note may be edited.
    pub(super) fn edit_note_blocked(&self) -> Option<&'static str> {
        let Some(open) = &self.note else {
            return Some("no note is open");
        };
        if !self.context_work_idle() {
            Some("BRN is working")
        } else if self.review.is_some() {
            Some("finish the review first")
        } else if self.recovery_blocked() {
            Some("recover the kept edit first")
        } else if open.session.is_some() {
            Some("save or discard your edit first")
        } else if open.record.archived {
            Some("this note is archived")
        } else if self
            .workspace
            .store
            .record(&open.record.id)
            .ok()
            .flatten()
            .as_ref()
            != Some(&open.record)
        {
            Some("open the current version first")
        } else {
            None
        }
    }
    pub(super) fn edit_note_authority(
        &self,
        note: &Record,
    ) -> AppResult<brn_ai::threads::OwnerRequest> {
        if !self.can_edit_note_context()
            || self.note.as_ref().is_none_or(|open| &open.record != note)
        {
            return Err(AppError::Invalid(
                "Save or reload the note and finish queued work before editing it with BRN.".into(),
            ));
        }
        Ok(brn_ai::threads::OwnerRequest::EditNote {
            id: note.id.clone(),
            version: note.version,
        })
    }
    pub(super) fn edit_note_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_edit_note_context() {
            return;
        }
        let note = self.note.as_ref().unwrap().record.clone();
        let instruction = cx.new(|cx| {
            components::prose_editor_state(window, cx)
                .placeholder("What should change in this note?")
        });
        instruction.update(cx, |editor, cx| editor.focus(window, cx));
        let error = cx.new(|_| String::new());
        let target = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let (instruction, error, target, note) = (instruction.clone(), error.clone(), target.clone(), note.clone());
            let submit_input = instruction.clone();
            let submit_error = error.clone();
            dialog.title(format!("Edit {}", short(&title(&note)))).width(px(620.)).child(
                div().flex().flex_col().gap_3()
                    .child(metadata(format!("BRN may edit this saved note (v{}) for this request. Other notes keep their existing protection.", note.version)))
                    .child(div().id("context-edit-instruction").h(px(180.)).child(prose_editor(&instruction).h_full().aria_label("Note edit instruction")).test_support())
                    .child(metadata(error.read(cx).clone()).id("context-edit-error").aria_label(error.read(cx).clone()).test_support())
                    .child(div().flex().gap_2()
                        .child(button("context-edit-cancel").label("Cancel").on_click(|_, window, cx| window.close_dialog(cx)))
                        .child(button("context-edit-submit").label("Edit note").on_click(move |_, window, cx| {
                            let prompt = submit_input.read(cx).value().to_string();
                            let mut result = Err("This window is unavailable.".to_owned());
                            let _ = target.update(cx, |this, cx| {
                                result = this.submit_note_edit(&note, &prompt, window, cx).map_err(|e| e.to_string());
                            });
                            match result {
                                Ok(()) => window.close_dialog(cx),
                                Err(message) => submit_error.update(cx, |error, cx| { *error = message; cx.notify(); }),
                            }
                        }))),
            )
        });
    }
    pub(super) fn submit_note_edit(
        &mut self,
        note: &Record,
        prompt: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AppResult<()> {
        if prompt.trim().is_empty() {
            return Err(AppError::Invalid("Describe what should change.".into()));
        }
        let authority = self.edit_note_authority(note)?;
        let new_note_draft = matches!(&self.chat, ChatTarget::NewForNote(id) if id == &note.id)
            .then(|| {
                (
                    self.destination(),
                    self.composer.read(cx).value().to_string(),
                )
            });
        let thread = self.ensure_writing_thread(&note.id, note.version)?;
        if let Some((_, draft)) = &new_note_draft {
            // A first contextual edit creates the note's writing chat. Move its
            // unsent draft into that chat, including persisted recovery.
            let target = ComposerDestination::Thread(thread.clone());
            self.save_composer(target.clone(), draft.clone());
            let key = serde_json::to_string(&target)?;
            if !self
                .workspace
                .store
                .chat_drafts()?
                .iter()
                .any(|d| d.destination == key && d.text == *draft)
            {
                return Err(AppError::Invalid("The chat draft could not be preserved. Your original draft is still available.".into()));
            }
        }
        // All draft handoff checks precede posting, so a refused dialog retry
        // cannot duplicate an instruction already recorded by this action.
        self.workspace.post_message(&thread, prompt)?;
        if let Some((destination, _)) = new_note_draft {
            self.save_composer(destination, String::new());
            self.chat = ChatTarget::Thread(thread.clone());
        }
        self.open_thread(&thread, window, cx);
        self.start_run(prompt.into(), thread, vec![], Some(authority), cx);
        Ok(())
    }
    pub(super) fn refresh_context_edit(
        &mut self,
        note: &str,
        version: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let unchanged_editor = self.note_editor_visible() && self.note.as_ref().is_some_and(|open| {
            open.record.id == note && open.record.version == version && open.session.is_none()
                && matches!(&open.record.data, RecordData::Note(n) if self.note_editor.read(cx).value().as_ref() == n.markdown)
        });
        if unchanged_editor
            && self
                .workspace
                .store
                .record(note)
                .ok()
                .flatten()
                .is_some_and(|r| {
                    !r.archived && r.version > version && matches!(r.data, RecordData::Note(_))
                })
        {
            self.open_note(note, None, window, cx);
        }
    }
    pub(super) fn decision_dialog(
        &mut self,
        message: Record,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let RecordData::Message(reply) = &message.data else {
            return;
        };
        if reply.role != "assistant" || reply.text.trim().is_empty() || message.archived {
            return;
        }
        let title = cx.new(|cx| {
            InputState::new(window, cx).default_value(format!(
                "Decision · {}",
                short(&self.thread_title(&reply.thread))
            ))
        });
        let text = cx
            .new(|cx| components::prose_editor_state(window, cx).default_value(reply.text.clone()));
        title.update(cx, |input, cx| input.focus(window, cx));
        let error = cx.new(|_| String::new());
        let target = cx.entity().downgrade();
        let candidate = std::rc::Rc::new(std::cell::RefCell::new(None::<(String, String, String)>));
        window.open_dialog(cx, move |dialog, _, cx| {
            let (title, text, error, target, candidate, message) = (title.clone(), text.clone(), error.clone(), target.clone(), candidate.clone(), message.clone());
            let (submit_title, submit_text, submit_error) = (title.clone(), text.clone(), error.clone());
            dialog.title("Save as decision").width(px(680.)).child(
                div().flex().flex_col().gap_3()
                    .child(metadata("Review the title and text. Save retains this decision in BRN as protected and confirmed. Your chat and attachments stay where they are."))
                    .child(Input::new(&title).id("decision-title"))
                    .child(div().id("decision-text").h(px(260.)).child(prose_editor(&text).h_full().aria_label("Decision text")).test_support())
                    .child(metadata(error.read(cx).clone()).id("decision-error").aria_label(error.read(cx).clone()).test_support())
                    .child(div().flex().gap_2()
                        .child(button("decision-cancel").label("Cancel").on_click(|_, window, cx| window.close_dialog(cx)))
                        .child(button("decision-save").label("Save decision").on_click(move |_, window, cx| {
                            let name = submit_title.read(cx).value().to_string();
                            let markdown = submit_text.read(cx).value().to_string();
                            // Modified previews get fresh identities; unchanged retries reuse them.
                            let key = {
                                let mut c = candidate.borrow_mut();
                                if c.as_ref().is_none_or(|(t, body, _)| t != &name || body != &markdown) {
                                    *c = Some((name.clone(), markdown.clone(), format!("decision:{}", uuid::Uuid::new_v4())));
                                }
                                c.as_ref().unwrap().2.clone()
                            };
                            let mut result = Err("This window is unavailable.".to_owned());
                            let _ = target.update(cx, |this, cx| {
                                result = this.workspace.save_chat_decision(&key, &message, &name, &markdown).map(|note| {
                                    this.open_note(&note.id, None, window, cx);
                                    this.inform("Decision saved in BRN. Undo is available in What changed.");
                                    this.refresh();
                                    cx.notify();
                                }).map_err(|e| e.to_string());
                            });
                            match result {
                                Ok(()) => window.close_dialog(cx),
                                Err(message) => submit_error.update(cx, |error, cx| { *error = message; cx.notify(); }),
                            }
                        }))),
            )
        });
    }
}
