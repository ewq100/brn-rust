//! Inbox-bound tentative findings admitted on the existing application lane.
use super::action_proposals::{active_turn, rejected, safe};
use super::*;
use crate::inbox_actions::{InboxActionJob, InboxAnalysisPurpose};
use brn_ai::{AiError, AiErrorKind, AiResult, ConflictArgs};
use serde_json::Value;
use sha2::{Digest, Sha256};

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
        let id = conflict_id(job.capture.id, &self.args)?;
        if app
            .work_store()
            .finding(id)
            .map_err(|e| safe(e.into()))?
            .is_none()
            && super::action_proposals::inbox_consequence_count(app, job)?
                >= crate::inbox_actions::MAX_INBOX_ACTION_PROPOSALS
        {
            return Err(rejected());
        }
        let record = app
            .capture_selected_conflict(job.capture.id, id, &self.args)
            .map_err(safe)?;
        let receipt =
            serde_json::to_value(record).map_err(|_| AiError::new(AiErrorKind::Storage))?;
        if serde_json::to_vec(&receipt).map_err(|_| rejected())?.len() > brn_ai::READ_ACTION_BYTES {
            return Err(rejected());
        }
        Ok(receipt)
    }
}

// A candidate identity is application-owned and stable only within this exact
// analysis/intent. It grants no approval or saved-state authority. Domain and
// fixed-length analysis separate inputs; encoded struct order is deterministic.
fn conflict_id(analysis: Uuid, args: &ConflictArgs) -> AiResult<Uuid> {
    let input = serde_json::to_vec(args).map_err(|_| rejected())?;
    let mut hash = Sha256::new();
    hash.update(b"brn/inbox-conflict/v1\0");
    hash.update(analysis.as_bytes());
    hash.update(input);
    let digest = hash.finalize();
    let mut bytes: [u8; 16] = digest[..16].try_into().expect("SHA-256 prefix");
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(Uuid::from_bytes(bytes))
}
