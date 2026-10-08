//! Exact client capture for one new-note destination revision.
use super::*;
use brn_workflow::proposals::{CreateRenameRequest, validate_create_rename_transition};

pub(super) struct SubmittedCreateRename {
    id: Uuid,
    request: CreateRenameRequest,
    before: ProposalRecord,
    generation: u64,
    action_fields: Vec<ActionFields>,
}

impl ProposalReview {
    pub fn create_rename_pending(&self) -> bool {
        self.create_rename.is_some()
    }

    pub fn create_rename_eligible(&self, index: usize) -> bool {
        // Presentation eligibility only. Full records, bindings and path policy
        // are checked at capture and acknowledgement, without encoding retained
        // asset payloads again on every native render.
        self.record.state == ProposalState::Draft
            && self.record.draft.intake.is_none()
            && self.record.draft.inbox_source.is_none()
            && self.record.draft.inbox_visual.is_none()
            && matches!(
                self.record.draft.changes.get(index),
                Some(NoteChange::Create { .. })
            )
    }

    pub fn can_rename_create(&self, index: usize) -> bool {
        self.can_mutate() && self.create_rename_eligible(index)
    }

    pub fn prepare_create_rename(
        &mut self,
        change_index: usize,
        path: String,
    ) -> Option<(Uuid, CreateRenameRequest)> {
        if !self.can_rename_create(change_index) {
            return None;
        }
        let request = CreateRenameRequest {
            expected: self.record.stamp(),
            change_index,
            path,
        };
        if let Err(error) = request.validate() {
            self.error = Some(error.message);
            return None;
        }
        // Check the complete proposed shape with the same pure validator used
        // for the returned record. The host alone captures filesystem evidence.
        let mut after = self.record.clone();
        if let NoteChange::Create { path, .. } = &mut after.draft.changes[change_index]
            && *path != request.path
        {
            *path = request.path.clone();
            after.version = after.version.checked_add(1)?;
        }
        if let Err(error) =
            validate_create_rename_transition(&self.record, &after, change_index, &request.path)
        {
            self.error = Some(error.message);
            return None;
        }
        let id = Uuid::new_v4();
        self.create_rename = Some(SubmittedCreateRename {
            id,
            request: request.clone(),
            before: self.record.clone(),
            generation: self.generation,
            action_fields: self.action_fields.clone(),
        });
        self.error = None;
        Some((id, request))
    }

    fn create_rename_matches(submitted: &SubmittedCreateRename, record: &ProposalRecord) -> bool {
        validate_create_rename_transition(
            &submitted.before,
            record,
            submitted.request.change_index,
            &submitted.request.path,
        )
        .is_ok()
    }

    pub(super) fn valid_create_rename_observation(&self, record: &ProposalRecord) -> bool {
        self.create_rename
            .as_ref()
            .is_some_and(|submitted| Self::create_rename_matches(submitted, record))
    }

    pub fn acknowledge_create_rename(&mut self, id: Uuid, record: ProposalRecord) -> bool {
        let Some(submitted) = self
            .create_rename
            .as_ref()
            .filter(|submitted| submitted.id == id)
        else {
            return false;
        };
        if self.record != submitted.before || !Self::create_rename_matches(submitted, &record) {
            self.fail_create_rename(id, "Destination acknowledgement did not match the captured revision; local review is retained.".into());
            return false;
        }
        let submitted = self
            .create_rename
            .take()
            .expect("matching destination revision");
        let local_matches = self.generation == submitted.generation
            && self.title == submitted.before.draft.title
            && texts_match(&submitted.before, &self.texts)
            && actions_match(&submitted.before, &self.action_data)
            && self.action_fields == submitted.action_fields;
        if !local_matches {
            if self
                .observed
                .as_ref()
                .is_none_or(|observed| observed.version < record.version || observed == &record)
            {
                self.observed = Some(record);
            }
            self.error = Some("Review changed during destination revision; local text is retained. Copy or explicitly discard it before continuing.".into());
            self.failed = false;
            return false;
        }
        let observed = self
            .observed
            .take()
            .filter(|observed| observed.version >= record.version && observed != &record);
        self.adopt(record);
        self.action_fields = submitted.action_fields;
        if let Some(observed) = observed {
            self.observed = Some(observed);
            self.error = Some("A conflicting review observation is retained after destination revision; inspect or explicitly discard local text.".into());
        }
        true
    }

    pub fn fail_create_rename(&mut self, id: Uuid, error: String) -> bool {
        if !self
            .create_rename
            .as_ref()
            .is_some_and(|submitted| submitted.id == id)
        {
            return false;
        }
        self.create_rename = None;
        // A queued native navigation must not discard the retained destination
        // after failure. Existing explicit retry/discard clears this fence.
        self.create_rename_failed = true;
        self.error = Some(error);
        self.failed = true;
        true
    }
}
