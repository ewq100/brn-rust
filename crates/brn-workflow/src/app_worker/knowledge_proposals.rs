//! One opt-in Inbox knowledge callback through the application owner.
use super::action_proposals::{active_turn, rejected, safe};
use super::*;
use crate::inbox_actions::{InboxActionJob, InboxAnalysisPurpose};
use crate::proposals::{ProposalStamp, ProposalState, SourceVersion};
use brn_ai::{AiError, AiErrorKind, AiResult, KnowledgeProposalArgs};
use serde::Serialize;
use serde_json::Value;

pub(super) struct KnowledgeProposal {
    pub args: KnowledgeProposalArgs,
    pub request: AskRequest,
    pub turn: WorkTurn,
    pub reply: mpsc::Sender<AiResult<Value>>,
    pub inbox: Box<InboxActionJob>,
}
impl KnowledgeProposal {
    pub(super) fn refuse(self, kind: AiErrorKind) {
        let _ = self.reply.send(Err(AiError::new(kind)));
    }
    pub(super) fn settle(self, app: &mut App) {
        let result = self.run(app);
        let _ = self.reply.send(result);
    }
    fn run(&self, app: &mut App) -> AiResult<Value> {
        self.args.validate()?;
        let actual = active_turn(app, &self.request, &self.turn)?;
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
        let request = app
            .prepare_inbox_knowledge(job, &self.args, actual.conversation_id)
            .map_err(safe)?;
        match app.proposal(request.id) {
            Ok(existing) => {
                if existing.draft.inbox_knowledge != request.inbox_knowledge {
                    return Err(rejected());
                }
            }
            Err(e) if e.kind == ErrorKind::NotFound => {
                if super::action_proposals::inbox_consequence_count(app, job)?
                    >= crate::inbox_actions::MAX_INBOX_ACTION_PROPOSALS
                {
                    return Err(rejected());
                }
            }
            Err(e) => return Err(safe(e)),
        }
        let mut receipt = Receipt {
            stamp: ProposalStamp {
                id: request.id,
                version: u64::MAX,
            },
            state: ProposalState::Uncertain,
            session_id: actual.conversation_id,
            group_id: job.capture.id,
            note_id: request
                .inbox_knowledge
                .as_ref()
                .expect("typed binding")
                .note_id,
            path: self.args.path.clone(),
            sources: request.sources.clone(),
        };
        if serde_json::to_vec(&receipt).map_err(|_| rejected())?.len() > brn_ai::READ_ACTION_BYTES {
            return Err(rejected());
        }
        let record = app.create_proposal(&request).map_err(safe)?;
        receipt.stamp = record.stamp();
        receipt.state = record.state;
        serde_json::to_value(receipt).map_err(|_| AiError::new(AiErrorKind::Storage))
    }
}
#[derive(Serialize)]
struct Receipt {
    stamp: ProposalStamp,
    state: ProposalState,
    session_id: Uuid,
    group_id: Uuid,
    note_id: Uuid,
    path: String,
    sources: Vec<SourceVersion>,
}
