//! Exact client capture for the dedicated structural transition.
use super::*;
use brn_workflow::proposals::{
    KnowledgePredecessorRequest, validate_knowledge_predecessor_transition,
};

pub(super) struct SubmittedPredecessor {
    id: Uuid,
    request: KnowledgePredecessorRequest,
    before: ProposalRecord,
    generation: u64,
}

impl ProposalReview {
    pub fn predecessor_pending(&self) -> bool {
        self.predecessor.is_some()
    }

    pub fn predecessor_eligible(&self) -> bool {
        self.record.state == ProposalState::Draft
            && self
                .record
                .draft
                .inbox_knowledge
                .as_ref()
                .is_some_and(|binding| binding.supersedes.is_none())
            && matches!(
                self.record.draft.changes.as_slice(),
                [NoteChange::Create { .. }]
            )
            && self.record.draft.action_changes.is_empty()
    }
    pub fn can_attach_predecessor(&self) -> bool {
        self.predecessor_eligible() && self.can_mutate()
    }
    pub fn history_member(&self, index: usize) -> bool {
        index == 1
            && self
                .record
                .draft
                .inbox_knowledge
                .as_ref()
                .is_some_and(|binding| binding.supersedes.is_some())
    }
    pub fn member_readonly(&self, index: usize) -> bool {
        self.record.draft.inbox_source.is_some() || self.history_member(index)
    }
    pub fn prepare_predecessor(
        &mut self,
        predecessor_path: String,
    ) -> Option<(Uuid, KnowledgePredecessorRequest)> {
        if !self.can_attach_predecessor() {
            return None;
        }
        if predecessor_path.trim().is_empty() || predecessor_path.len() > 4096 {
            self.error = Some("Enter a Current knowledge path of at most 4096 UTF-8 bytes.".into());
            return None;
        }
        let request = KnowledgePredecessorRequest {
            expected: self.record.stamp(),
            predecessor_path,
        };
        if let Err(error) = request.validate() {
            self.error = Some(error.message);
            return None;
        }
        let id = Uuid::new_v4();
        self.predecessor = Some(SubmittedPredecessor {
            id,
            request: request.clone(),
            before: self.record.clone(),
            generation: self.generation,
        });
        self.error = None;
        Some((id, request))
    }
    fn transition_matches(submitted: &SubmittedPredecessor, record: &ProposalRecord) -> bool {
        record
            .draft
            .inbox_knowledge
            .as_ref()
            .and_then(|binding| binding.supersedes.as_ref())
            .is_some_and(|binding| binding.source.path == submitted.request.predecessor_path)
            && validate_knowledge_predecessor_transition(&submitted.before, record).is_ok()
    }
    pub(super) fn valid_predecessor_observation(&self, record: &ProposalRecord) -> bool {
        self.predecessor
            .as_ref()
            .is_some_and(|submitted| Self::transition_matches(submitted, record))
    }
    pub fn acknowledge_predecessor(&mut self, id: Uuid, record: ProposalRecord) -> bool {
        let Some(submitted) = self
            .predecessor
            .as_ref()
            .filter(|submitted| submitted.id == id)
        else {
            return false;
        };
        if self.record != submitted.before || !Self::transition_matches(submitted, &record) {
            self.fail_predecessor(id, "Predecessor acknowledgement did not match the captured transition; local review is retained.".into());
            return false;
        }
        let submitted = self.predecessor.take().expect("matched attachment");
        let local_matches = self.generation == submitted.generation
            && self.title == submitted.before.draft.title
            && texts_match(&submitted.before, &self.texts)
            && actions_match(&submitted.before, &self.action_data);
        if !local_matches {
            self.observed = Some(record);
            self.error = Some("Review changed during predecessor attachment; local text is retained. Copy or explicitly discard it before continuing.".into());
            self.failed = false;
            return false;
        }
        let observed = self
            .observed
            .take()
            .filter(|observed| observed.version >= record.version && observed != &record);
        self.adopt(record);
        if let Some(observed) = observed {
            self.observed = Some(observed);
            self.error = Some("A conflicting review observation is retained after predecessor attachment; inspect or explicitly discard local text.".into());
        }
        true
    }
    pub fn fail_predecessor(&mut self, id: Uuid, error: String) -> bool {
        if !self
            .predecessor
            .as_ref()
            .is_some_and(|submitted| submitted.id == id)
        {
            return false;
        }
        self.predecessor = None;
        self.error = Some(error);
        self.failed = true;
        true
    }
}
