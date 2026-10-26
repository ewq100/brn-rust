//! Capture confirmation and typed dispatch; existing Run and review lifecycles own results.
use super::*;
#[cfg(all(test, feature = "native-test-support"))]
#[path = "threads_capture_tests.rs"]
mod tests;
impl ThreadsUi {
    pub(super) fn capture_outcomes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(worker) = &self.worker {
            self.warn(worker.kind().busy_notice());
            cx.notify();
            return;
        }
        let Some(chat) = self
            .chat_thread()
            .map(str::to_owned)
            .filter(|id| !self.thread_resolved(id))
        else {
            self.warn("Choose an open chat first.");
            cx.notify();
            return;
        };
        let preview = match self.workspace.preview_capture_request(&chat) {
            Ok(preview) => preview,
            Err(error) => {
                self.fail(error.to_string());
                cx.notify();
                return;
            }
        };
        let destination = if preview.private_context {
            "Proposals stay in this chat's private/project context."
        } else {
            "Proposals use this chat's General knowledge context."
        };
        let target = cx.entity().downgrade();
        self.open_dialog(window,cx,move|dialog,window,_cx|{
            let target=target.clone();let preview=preview.clone();
            dialog.title("Capture useful outcomes?").on_ok(|_,_,_|false)
                .child(components::dialog_body("capture-outcomes-body",window).child(div().flex().flex_col().gap_2()
                    .child("Review this posted conversation for useful notes, decisions and follow-ups. Existing saved knowledge and pending proposals are checked before new proposals are made.")
                    .child(destination).child(format!("{} posted messages captured now. Later messages belong to a fresh Capture request.",preview.messages.len()))
                    .child("This starts an AI request using your current model. New outcomes remain proposals: choose which to Apply in Changes. Everything else stays only in chat. Archive is a separate action.")))
                .footer(components::dialog_footer()
                    .child(button("capture-outcomes-cancel").label("Cancel").outline().on_click(|_,window,cx|window.close_dialog(cx)))
                    .child(button("capture-outcomes-confirm").label("Propose outcomes").primary().on_click(move|_,window,cx|{
                        let mut close=false;
                        let _=target.update(cx,|this,cx|{
                            if this.worker.is_some() {this.warn("Finish or stop the current work first.");cx.notify();return;}
                            match this.workspace.save_capture_request(&format!("capture:{}",uuid::Uuid::new_v4()),&preview) {
                                Ok(snapshot)=>{this.start_capture_request(snapshot,cx);close=true;},
                                Err(error)=>{this.fail(error.to_string());cx.notify();},
                            }
                        });
                        if close {window.close_dialog(cx);}
                    })))
        });
    }
    fn start_capture_request(&mut self, snapshot: CaptureSnapshot, cx: &mut Context<Self>) {
        if !self.begin_run_start(cx) {
            return;
        }
        self.start_request(ThreadRunStart {
            thread:snapshot.thread,prompt:"Propose useful outcomes from the captured posted conversation for owner review; do not apply or archive.".into(),
            request_message:None,queue_claim:None,owner_request:Some(brn_ai::threads::OwnerRequest::CaptureOutcomes{request_message:snapshot.request_message}),
            sources:vec![],max_tool_rounds:None,trace:brn_ai::threads::TraceLevel::Off,offered_copilot_models:Some(self.copilot_models.clone()),
        },false,None,"Continue Capture from this chat","Capture finished. Review proposed outcomes in Changes; unapplied information remains in chat.",cx);
    }
    pub(super) fn retry_capture(&mut self, request: String, cx: &mut Context<Self>) {
        match self
            .workspace
            .store
            .record(&request)
            .map_err(AppError::from)
            .and_then(|r| {
                r.ok_or_else(|| AppError::Invalid("Capture request is unavailable".into()))
            })
            .and_then(|r| CaptureSnapshot::from_message(&r).map_err(AppError::from))
        {
            Ok(snapshot) => self.start_capture_request(snapshot, cx),
            Err(error) => {
                self.fail(error.to_string());
                cx.notify();
            }
        }
    }
}

// Continue belongs to the latest durable Run, never merely to a newer marker.
fn capture_continuation(
    path: ContinuePath,
    messages: &[Record],
    latest: Option<&Record>,
) -> ContinuePath {
    if matches!(
        path,
        ContinuePath::ResumeQueue | ContinuePath::RetryRewrite(_)
    ) {
        return path;
    }
    let after_owner = messages
        .iter()
        .rposition(|r| matches!(&r.data,RecordData::Message(m) if m.role=="owner"))
        .map_or(0, |i| i + 1);
    let captures = messages[after_owner..]
        .iter()
        .filter(|r| matches!(&r.data,RecordData::Message(m) if m.role==CAPTURE_REQUEST_ROLE))
        .collect::<Vec<_>>();
    if let Some(Record {
        data: RecordData::Run(run),
        ..
    }) = latest
        && let Some(record) = captures.iter().find(|r| {
            run.instruction.as_ref() == Some(&r.id)
                && matches!(&r.data,RecordData::Message(m) if m.thread==run.thread)
        })
    {
        return ContinuePath::RetryCapture(record.id.clone());
    }
    if !captures.is_empty() {
        ContinuePath::CaptureNeedsConfirmation
    } else {
        path
    }
}
impl ThreadsUi {
    pub(super) fn continue_chat_path(&self, messages: &[Record], thread: &str) -> ContinuePath {
        capture_continuation(
            continue_path(&self.queue_state(), messages, thread),
            messages,
            latest_run(&self.records, &self.history, thread),
        )
    }
}
