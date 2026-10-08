//! Local review text and exact worker acknowledgements; no authority or I/O.

#[path = "action_fields.rs"]
pub mod action_fields;
use action_fields::ActionFields;

use brn_workflow::{
    MAX_NOTE_BYTES,
    actions::ActionData,
    proposals::{
        ActionChange, CommentTarget, NoteChange, ProposalEdit, ProposalRecord, ProposalState,
        TextAnchor, validate_review_edit,
    },
};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[path = "predecessor_review.rs"]
mod predecessor;
use predecessor::SubmittedPredecessor;
#[cfg(test)]
#[path = "predecessor_review_tests.rs"]
pub(crate) mod predecessor_tests;

struct SubmittedEdit {
    id: Uuid,
    edit: ProposalEdit,
    generation: u64,
}

/// The acknowledged full review and local typing have separate lifetimes.
pub struct ProposalReview {
    pub record: ProposalRecord,
    pub observed: Option<ProposalRecord>,
    pub error: Option<String>,
    title: String,
    texts: Vec<Option<String>>,
    action_data: Vec<ActionData>,
    action_fields: Vec<ActionFields>,
    generation: u64,
    acknowledged_generation: u64,
    submitted: Option<SubmittedEdit>,
    predecessor: Option<SubmittedPredecessor>,
    last_edit: Option<Instant>,
    failed: bool,
}

impl ProposalReview {
    pub fn new(record: ProposalRecord) -> Self {
        Self {
            title: record.draft.title.clone(),
            texts: record_texts(&record),
            action_data: record_actions(&record),
            action_fields: record_actions(&record)
                .iter()
                .map(ActionFields::from)
                .collect(),
            record,
            observed: None,
            error: None,
            generation: 0,
            acknowledged_generation: 0,
            submitted: None,
            predecessor: None,
            last_edit: None,
            failed: false,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn text(&self, index: usize) -> Option<&str> {
        self.texts.get(index).and_then(Option::as_deref)
    }

    pub fn texts(&self) -> &[Option<String>] {
        &self.texts
    }

    pub fn action_data(&self) -> &[ActionData] {
        &self.action_data
    }

    pub fn action_fields(&self) -> &[ActionFields] {
        &self.action_fields
    }

    // Typed editing entry point also used by headless presentation tests.
    #[allow(dead_code)]
    pub fn edit_action(
        &mut self,
        index: usize,
        data: ActionData,
        now: Instant,
    ) -> Result<(), &'static str> {
        self.can_type()?;
        let Some(current) = self.action_data.get(index) else {
            return Err("Action member does not exist");
        };
        let fields = ActionFields::from(&data);
        if current == &data && self.action_fields[index] == fields {
            return Ok(());
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or("Proposal edit generation exhausted")?;
        self.action_data[index] = data;
        self.action_fields[index] = fields;
        self.generation = generation;
        self.last_edit = Some(now);
        Ok(())
    }

    pub fn edit_action_fields(
        &mut self,
        index: usize,
        fields: ActionFields,
        now: Instant,
    ) -> Result<(), &'static str> {
        self.can_type()?;
        let Some(current) = self.action_fields.get(index) else {
            return Err("Action member does not exist");
        };
        if current == &fields {
            return Ok(());
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or("Proposal edit generation exhausted")?;
        if let Ok(data) = fields.data() {
            self.action_data[index] = data;
        }
        self.action_fields[index] = fields;
        self.generation = generation;
        self.last_edit = Some(now);
        Ok(())
    }

    pub fn copy_local(&self) -> Result<String, serde_json::Error> {
        let edit = ProposalEdit {
            action_data: self.action_data.clone(),
            expected: self.record.stamp(),
            title: self.title.clone(),
            texts: self.texts.clone(),
        };
        if self.action_fields.is_empty() {
            return serde_json::to_string_pretty(&edit);
        }
        serde_json::to_string_pretty(
            &serde_json::json!({ "edit": edit, "local_action_fields": self.action_fields }),
        )
    }

    pub fn dirty(&self) -> bool {
        self.generation != self.acknowledged_generation
            || self.title != self.record.draft.title
            || !texts_match(&self.record, &self.texts)
            || !actions_match(&self.record, &self.action_data)
            || self.action_fields.iter().any(|fields| fields.data().is_err())
            // A conflicting observation requires an explicit discard even when
            // an earlier acknowledgement catches up with the local bytes.
            || self.observed.is_some()
    }

    pub fn pending(&self) -> bool {
        self.submitted.is_some() || self.predecessor.is_some()
    }

    pub fn can_leave(&self) -> bool {
        !self.pending() && !self.dirty()
    }

    pub fn can_mutate(&self) -> bool {
        self.record.state == ProposalState::Draft
            && self.can_leave()
            && self.error.is_none()
            && !self.failed
    }

    fn can_type(&self) -> Result<(), &'static str> {
        if self.predecessor.is_some() {
            return Err("Waiting for predecessor attachment acknowledgement");
        }
        if self.record.state != ProposalState::Draft
            || self
                .observed
                .as_ref()
                .is_some_and(|record| record.state != ProposalState::Draft)
        {
            return Err("Only a draft proposal can be edited");
        }
        Ok(())
    }

    pub fn edit_title(&mut self, title: String, now: Instant) -> Result<(), &'static str> {
        self.can_type()?;
        if title.len() > 512 {
            return Err("Proposal title exceeds the 512-byte limit");
        }
        if title == self.title {
            return Ok(());
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or("Proposal edit generation exhausted")?;
        self.title = title;
        self.generation = generation;
        self.last_edit = Some(now);
        Ok(())
    }

    pub fn edit_text(
        &mut self,
        index: usize,
        text: String,
        now: Instant,
    ) -> Result<(), &'static str> {
        self.can_type()?;
        if self.history_member(index) {
            return Err("Generated predecessor History is read only");
        }
        let Some(Some(current)) = self.texts.get(index) else {
            return Err("This proposal member has no editable text");
        };
        if text.len() > MAX_NOTE_BYTES {
            return Err("Proposal text exceeds the 1 MiB UTF-8 byte limit");
        }
        if text == *current {
            return Ok(());
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or("Proposal edit generation exhausted")?;
        self.texts[index] = Some(text);
        self.generation = generation;
        self.last_edit = Some(now);
        Ok(())
    }

    fn can_recover(&self) -> bool {
        self.record.state == ProposalState::Draft
            && self.dirty()
            && !self.pending()
            && !self.failed
            && self.observed.is_none()
    }

    pub fn wants_recovery(&self, now: Instant, leaving: bool) -> bool {
        self.can_recover()
            && (leaving
                || self.last_edit.is_some_and(|last| {
                    now.saturating_duration_since(last) >= Duration::from_millis(500)
                }))
    }

    pub fn prepare_edit(&mut self) -> Option<(Uuid, ProposalEdit)> {
        if !self.can_recover() {
            return None;
        }
        for fields in &self.action_fields {
            if let Err(error) = fields.data() {
                self.error = Some(error);
                self.failed = true;
                return None;
            }
        }
        let edit = ProposalEdit {
            action_data: self.action_data.clone(),
            expected: self.record.stamp(),
            title: self.title.clone(),
            texts: self.texts.clone(),
        };
        if let Err(error) = validate_review_edit(&self.record, &edit) {
            self.error = Some(error.message);
            self.failed = true;
            return None;
        }
        let id = Uuid::new_v4();
        self.submitted = Some(SubmittedEdit {
            id,
            edit: edit.clone(),
            generation: self.generation,
        });
        self.error = None;
        Some((id, edit))
    }

    pub fn acknowledge_edit(&mut self, id: Uuid, record: ProposalRecord) -> bool {
        let Some(submitted) = self
            .submitted
            .as_ref()
            .filter(|submitted| submitted.id == id)
        else {
            return false;
        };
        let changed = submitted.edit.title != self.record.draft.title
            || !texts_match(&self.record, &submitted.edit.texts)
            || !actions_match(&self.record, &submitted.edit.action_data);
        let expected_version = if changed {
            submitted.edit.expected.version.checked_add(1)
        } else {
            Some(submitted.edit.expected.version)
        };
        let checked = ProposalEdit {
            action_data: record_actions(&record),
            expected: record.stamp(),
            title: record.draft.title.clone(),
            texts: record_texts(&record),
        };
        if !same_bindings(&self.record, &record)
            || record.state != ProposalState::Draft
            || expected_version != Some(record.version)
            || record.updated_at_ms < self.record.updated_at_ms
            || record.draft.title != submitted.edit.title
            || !texts_match(&record, &submitted.edit.texts)
            || !actions_match(&record, &submitted.edit.action_data)
            || validate_review_edit(&record, &checked).is_err()
        {
            self.fail_edit(
                id,
                "Review acknowledgement did not match the submitted full edit".into(),
            );
            return false;
        }
        let submitted = self.submitted.take().expect("matching edit");
        self.acknowledged_generation = submitted.generation;
        self.record = record;
        if self.observed.as_ref().is_some_and(|observed| {
            observed.version < self.record.version || observed == &self.record
        }) {
            self.observed = None;
        }
        self.error = self
            .observed
            .as_ref()
            .map(|_| conflict_message().to_owned());
        self.failed = false;
        if self.generation == self.acknowledged_generation {
            self.last_edit = None;
        }
        true
    }

    pub fn fail_edit(&mut self, id: Uuid, error: String) -> bool {
        if !self
            .submitted
            .as_ref()
            .is_some_and(|submitted| submitted.id == id)
        {
            return false;
        }
        self.submitted = None;
        self.error = Some(error);
        self.failed = true;
        true
    }

    pub fn retry(&mut self) -> bool {
        if self.pending() || self.observed.is_some() || !self.failed && self.error.is_none() {
            return false;
        }
        self.failed = false;
        self.error = None;
        true
    }

    pub fn observe(&mut self, record: ProposalRecord) -> bool {
        if (!same_bindings(&self.record, &record) && !self.valid_predecessor_observation(&record))
            || record.version < self.record.version
            || self
                .observed
                .as_ref()
                .is_some_and(|observed| record.version < observed.version)
        {
            return false;
        }
        if !self.dirty() && !self.pending() {
            self.adopt(record);
        } else if record.version != self.record.version
            || record.state != self.record.state
            || !same_contents(&record, &self.record)
            || record.comments != self.record.comments
        {
            self.observed = Some(record);
            self.error = Some(conflict_message().to_owned());
        }
        true
    }

    pub fn discard_local(&mut self) -> bool {
        if self.pending() {
            return false;
        }
        let record = self.observed.take().unwrap_or_else(|| self.record.clone());
        self.adopt(record);
        true
    }

    fn adopt(&mut self, record: ProposalRecord) {
        self.title = record.draft.title.clone();
        self.texts = record_texts(&record);
        self.action_data = record_actions(&record);
        self.action_fields = self.action_data.iter().map(ActionFields::from).collect();
        self.record = record;
        self.acknowledged_generation = self.generation;
        self.observed = None;
        self.error = None;
        self.failed = false;
        self.last_edit = None;
    }
}

fn conflict_message() -> &'static str {
    "Review changed while local edits were pending; local text is retained"
}

fn record_texts(record: &ProposalRecord) -> Vec<Option<String>> {
    record
        .draft
        .changes
        .iter()
        .map(|change| change.text().map(str::to_owned))
        .collect()
}

fn texts_match(record: &ProposalRecord, texts: &[Option<String>]) -> bool {
    record.draft.changes.len() == texts.len()
        && record
            .draft
            .changes
            .iter()
            .zip(texts)
            .all(|(change, text)| change.text() == text.as_deref())
}

pub(crate) fn record_actions(record: &ProposalRecord) -> Vec<ActionData> {
    record
        .draft
        .action_changes
        .iter()
        .map(|change| change.data().clone())
        .collect()
}

fn actions_match(record: &ProposalRecord, data: &[ActionData]) -> bool {
    record.draft.action_changes.len() == data.len()
        && record
            .draft
            .action_changes
            .iter()
            .zip(data)
            .all(|(change, data)| change.data() == data)
}

fn same_contents(left: &ProposalRecord, right: &ProposalRecord) -> bool {
    left.draft.action_changes == right.draft.action_changes
        && left.draft.title == right.draft.title
        && left.draft.changes.len() == right.draft.changes.len()
        && left
            .draft
            .changes
            .iter()
            .zip(&right.draft.changes)
            .all(|(a, b)| a.text() == b.text())
}

fn same_bindings(left: &ProposalRecord, right: &ProposalRecord) -> bool {
    let a = &left.draft;
    let b = &right.draft;
    a.id == b.id
        && a.group_id == b.group_id
        && a.session_id == b.session_id
        && a.vault == b.vault
        && a.sources == b.sources
        && left.created_at_ms == right.created_at_ms
        && a.action_changes.len() == b.action_changes.len()
        && a.action_changes
            .iter()
            .zip(&b.action_changes)
            .all(|(a, b)| match (a, b) {
                (ActionChange::Create { id: a, .. }, ActionChange::Create { id: b, .. }) => a == b,
                (
                    ActionChange::Replace { before: a, .. },
                    ActionChange::Replace { before: b, .. },
                ) => a == b,
                _ => false,
            })
        && a.changes.len() == b.changes.len()
        && a.changes.iter().zip(&b.changes).all(|(a, b)| match (a, b) {
            (
                NoteChange::Create {
                    path: a_path,
                    parent: a_parent,
                    ..
                },
                NoteChange::Create {
                    path: b_path,
                    parent: b_parent,
                    ..
                },
            ) => a_path == b_path && a_parent == b_parent,
            (
                NoteChange::Replace {
                    path: a_path,
                    parent: a_parent,
                    before: a_before,
                    before_text: a_text,
                    ..
                },
                NoteChange::Replace {
                    path: b_path,
                    parent: b_parent,
                    before: b_before,
                    before_text: b_text,
                    ..
                },
            )
            | (
                NoteChange::Trash {
                    path: a_path,
                    parent: a_parent,
                    before: a_before,
                    before_text: a_text,
                },
                NoteChange::Trash {
                    path: b_path,
                    parent: b_parent,
                    before: b_before,
                    before_text: b_text,
                },
            ) => {
                a_path == b_path && a_parent == b_parent && a_before == b_before && a_text == b_text
            }
            (NoteChange::CreateAsset { .. }, NoteChange::CreateAsset { .. })
            | (NoteChange::ReplaceAsset { .. }, NoteChange::ReplaceAsset { .. })
            | (NoteChange::TrashAsset { .. }, NoteChange::TrashAsset { .. }) => a == b,
            _ => false,
        })
}

/// Exact acknowledged UTF-8 byte selection; no trimming or inferred anchoring.
pub fn selection_target(
    review: &ProposalReview,
    index: usize,
    range: std::ops::Range<usize>,
) -> Option<CommentTarget> {
    if !review.can_mutate() || range.start >= range.end {
        return None;
    }
    let text = review.record.draft.changes.get(index)?.text()?;
    let quote = text.get(range.clone())?;
    Some(CommentTarget::Text(TextAnchor {
        change_index: index,
        start: range.start,
        end: range.end,
        quote: quote.to_owned(),
    }))
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "action_review_tests.rs"]
pub(crate) mod action_tests;

#[cfg(test)]
#[path = "review_asset_tests.rs"]
pub(crate) mod asset_tests;
