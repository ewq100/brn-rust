//! Inbox-bound tentative findings admitted on the existing application lane.
use super::action_proposals::{active_turn, rejected, safe};
use super::*;
use crate::findings::{CaptureFindingRequest, FindingOrigin, FindingQuote};
use crate::inbox_actions::{InboxActionJob, InboxAnalysisPurpose};
use brn_ai::{AiError, AiErrorKind, AiResult, ConflictArgs};
use serde_json::Value;

pub(super) struct ConflictReport {
    pub args: ConflictArgs,
    pub request: AskRequest,
    pub turn: WorkTurn,
    pub reply: mpsc::Sender<AiResult<Value>>,
    pub inbox: Box<InboxActionJob>,
}
impl ConflictReport {
    pub(super) fn refuse(self, kind: AiErrorKind) {
        let _ = self.reply.send(Err(AiError::new(kind)));
    }
    pub(super) fn settle(self, app: &mut App) {
        let result = self.run(app);
        let _ = self.reply.send(result);
    }
    fn run(&self, app: &mut App) -> AiResult<Value> {
        self.args.validate()?;
        active_turn(app, &self.request, &self.turn)?;
        let job = &self.inbox;
        if job.capture.purpose != InboxAnalysisPurpose::KnowledgeAndActions
            || job.capture.id != self.request.id
            || job.question != self.request.question
            || app
                .work_store()
                .inbox_action(self.request.id)
                .map_err(|e| safe(e.into()))?
                .as_ref()
                != Some(job.as_ref())
        {
            return Err(rejected());
        }
        let request = CaptureFindingRequest {
            id: Uuid::parse_str(&self.args.id).map_err(|_| rejected())?,
            origin: FindingOrigin::InboxConflict {
                analysis_id: job.capture.id,
                title: self.args.title.clone(),
                summary: self.args.summary.clone(),
                source_quote: FindingQuote {
                    start_byte: self.args.source_quote.start_byte,
                    end_byte: self.args.source_quote.end_byte,
                    quote: self.args.source_quote.quote.clone(),
                },
                other_path: self.args.other_path.clone(),
                other_quote: FindingQuote {
                    start_byte: self.args.other_quote.start_byte,
                    end_byte: self.args.other_quote.end_byte,
                    quote: self.args.other_quote.quote.clone(),
                },
            },
        };
        if app
            .work_store()
            .finding(request.id)
            .map_err(|e| safe(e.into()))?
            .is_none()
            && super::action_proposals::inbox_consequence_count(app, job)?
                >= crate::inbox_actions::MAX_INBOX_ACTION_PROPOSALS
        {
            return Err(rejected());
        }
        let record = app.capture_finding(&request).map_err(safe)?;
        let receipt =
            serde_json::to_value(record).map_err(|_| AiError::new(AiErrorKind::Storage))?;
        if serde_json::to_vec(&receipt).map_err(|_| rejected())?.len() > brn_ai::READ_ACTION_BYTES {
            return Err(rejected());
        }
        Ok(receipt)
    }
}
