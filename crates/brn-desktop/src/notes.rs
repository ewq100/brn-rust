use brn_workflow::notes::{
    FileOutcome, NoteAvailability, NoteBufferReceipt, NoteErrorCode, NoteFailure, NoteReceipt,
    NoteResult, NoteStamp, NoteSubmission, NoteView,
};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Default)]
pub struct NoteObservations {
    ids: BTreeSet<Uuid>,
    rescan: bool,
}
impl NoteObservations {
    pub fn insert(&mut self, id: Uuid) {
        self.ids.insert(id);
    }
    pub fn extend(&mut self, ids: impl IntoIterator<Item = Uuid>) {
        self.ids.extend(ids);
    }
    pub fn notice(&mut self, notice: &brn_workflow::notes::NoteFileNotice, views: &[NoteView]) {
        if notice.relative_path.is_none()
            || notice.kind == brn_workflow::notes::NoteNoticeKind::RescanRequired
        {
            self.rescan = true;
        } else {
            for view in views {
                if view.vault_id == notice.vault_id
                    && Some(&view.relative_path) == notice.relative_path.as_ref()
                {
                    self.ids.insert(view.id);
                }
            }
        }
    }
    pub fn take(&mut self, idle: bool, registered: &[Uuid]) -> Option<Vec<Uuid>> {
        if !idle {
            return None;
        }
        if self.rescan {
            self.ids.extend(registered.iter().copied());
            self.rescan = false;
        }
        if self.ids.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.ids).into_iter().collect())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseRoute {
    Window,
    Quit,
    Switch,
}

#[derive(Default)]
pub struct NoteScheduler {
    last_edit: Option<Instant>,
    close: Option<CloseRoute>,
    failed: bool,
}
impl NoteScheduler {
    pub fn edited(&mut self, now: Instant) {
        self.last_edit = Some(now);
    }
    pub fn request_close(&mut self, route: CloseRoute) {
        self.close = Some(route);
    }
    pub fn take_close(&mut self) -> Option<CloseRoute> {
        self.close.take()
    }
    pub fn closing(&self) -> bool {
        self.close.is_some()
    }
    pub fn recovery_failed(&mut self) {
        self.failed = true;
    }
    pub fn note_failed(&mut self, editor: &NoteEditor) {
        if editor.pending_kind == Some(PendingKind::Recovery) {
            self.recovery_failed();
        }
    }
    pub fn retry(&mut self) {
        self.failed = false;
        self.last_edit = Some(Instant::now() - Duration::from_millis(500));
    }
    pub fn can_finish(&self, editor: &NoteEditor, idle: bool) -> bool {
        self.closing() && idle && editor.can_close()
    }
    pub fn wants_recovery(&self, now: Instant, editor: &NoteEditor, idle: bool) -> bool {
        idle && !editor.pending()
            && editor.needs_recovery()
            && !self.failed
            && (self.closing()
                || self.last_edit.is_some_and(|last| {
                    now.saturating_duration_since(last) >= Duration::from_millis(500)
                }))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingKind {
    Save,
    Recovery,
    Copy,
    Discard,
    Rebase,
}

/// Durable acknowledgement is separate from both live typing and fresh disk observations.
pub struct NoteEditor {
    view: NoteView,
    acknowledged: NoteStamp,
    generation: u64,
    text: String,
    saved: Option<String>,
    pending: Option<NoteSubmission>,
    pending_kind: Option<PendingKind>,
    failure: Option<NoteFailure>,
    copy_failure: Option<NoteFailure>,
    queued_recovery: Option<NoteSubmission>,
    last_submission: Option<NoteSubmission>,
}

impl NoteEditor {
    pub fn new(view: NoteView) -> Self {
        Self {
            acknowledged: view.stamp,
            generation: view.stamp.generation,
            text: view.buffer.clone(),
            saved: view.saved.clone(),
            view,
            pending: None,
            pending_kind: None,
            failure: None,
            copy_failure: None,
            queued_recovery: None,
            last_submission: None,
        }
    }
    pub fn id(&self) -> Uuid {
        self.view.id
    }
    pub fn view(&self) -> &NoteView {
        &self.view
    }
    pub fn stamp(&self) -> NoteStamp {
        self.acknowledged
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn dirty(&self) -> bool {
        self.saved.as_deref() != Some(self.text())
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn discard_pending(&self) -> bool {
        self.pending_kind == Some(PendingKind::Discard)
    }
    pub fn needs_recovery(&self) -> bool {
        self.generation != self.acknowledged.generation
    }
    pub fn can_close(&self) -> bool {
        !self.pending() && !self.needs_recovery()
    }
    pub fn failure(&self) -> Option<&NoteFailure> {
        self.failure.as_ref()
    }
    pub fn copy_failure(&self) -> Option<&NoteFailure> {
        self.copy_failure.as_ref()
    }
    pub fn edit(&mut self, text: String) -> NoteResult<()> {
        if self.text == text {
            return Ok(());
        }
        if text.len() > brn_workflow::worker::MAX_DRAFT_BYTES {
            return Err(note_state_changed(
                "Note exceeds the 1 MiB UTF-8 byte limit",
            ));
        }
        let generation = self
            .generation
            .checked_add(1)
            .filter(|generation| *generation <= i64::MAX as u64)
            .ok_or_else(|| note_state_changed("note generation exhausted"))?;
        self.generation = generation;
        self.text = text;
        self.queue_recovery();
        Ok(())
    }
    pub fn input_text(&self) -> String {
        self.text.clone()
    }
    pub fn edit_input(&mut self, input: String) -> NoteResult<()> {
        self.edit(input)
    }
    fn begin(&mut self, op: Uuid, kind: PendingKind) -> NoteResult<NoteSubmission> {
        if self.pending() {
            return Err(note_state_changed("a note mutation is pending"));
        }
        let request = self
            .queued_recovery
            .clone()
            .filter(|_| kind == PendingKind::Recovery)
            .map(|mut request| {
                request.operation_id = op;
                request
            })
            .unwrap_or_else(|| NoteSubmission {
                operation_id: op,
                note_id: self.id(),
                expected: self.acknowledged,
                generation: self.generation,
                text: self.text.clone(),
            });
        self.pending = Some(request.clone());
        self.last_submission = Some(request.clone());
        self.pending_kind = Some(kind);
        Ok(request)
    }
    pub fn begin_save(&mut self, op: Uuid) -> NoteResult<NoteSubmission> {
        self.begin(op, PendingKind::Save)
    }
    pub fn begin_recovery(&mut self, op: Uuid) -> NoteResult<NoteSubmission> {
        self.begin(op, PendingKind::Recovery)
    }
    pub fn begin_copy(&mut self, op: Uuid) -> NoteResult<NoteSubmission> {
        self.begin(op, PendingKind::Copy)
    }
    pub fn begin_discard(&mut self, op: Uuid) -> NoteResult<NoteSubmission> {
        self.begin(op, PendingKind::Discard)
    }
    pub fn begin_rebase(&mut self, op: Uuid) -> NoteResult<NoteSubmission> {
        self.begin(op, PendingKind::Rebase)
    }
    fn submitted(&self, op: Uuid, kind: PendingKind) -> NoteResult<&NoteSubmission> {
        self.pending
            .as_ref()
            .filter(|request| request.operation_id == op && self.pending_kind == Some(kind))
            .ok_or_else(|| note_state_changed("no matching pending note operation"))
    }
    fn clear_pending(&mut self) {
        self.pending = None;
        self.pending_kind = None;
    }
    fn queue_recovery(&mut self) {
        self.queued_recovery = self.needs_recovery().then(|| NoteSubmission {
            operation_id: Uuid::new_v4(),
            note_id: self.id(),
            expected: self.acknowledged,
            generation: self.generation,
            text: self.text.clone(),
        });
    }
    pub fn acknowledge(&mut self, op: Uuid, receipt: NoteReceipt) -> NoteResult<()> {
        let pending = self.submitted(op, PendingKind::Save)?;
        if receipt.operation_id != pending.operation_id
            || receipt.source_note_id != pending.note_id
            || receipt.note_id != pending.note_id
            || receipt.submitted_generation != pending.generation
            || receipt.stamp.generation != pending.generation
            || receipt.filesystem_outcome == FileOutcome::Unknown
        {
            return Err(note_state_changed("receipt does not match submitted save"));
        }
        let text = pending.text.clone();
        self.saved = Some(text.clone());
        self.view.buffer = text;
        self.acknowledged = receipt.stamp;
        self.view.stamp = receipt.stamp;
        self.view.availability = NoteAvailability::Available;
        self.failure = None;
        self.clear_pending();
        self.queue_recovery();
        Ok(())
    }
    pub fn acknowledge_recovery(&mut self, op: Uuid, receipt: NoteBufferReceipt) -> NoteResult<()> {
        let pending = self.submitted(op, PendingKind::Recovery)?;
        if receipt.operation_id != op
            || receipt.note_id != pending.note_id
            || receipt.stamp.file_state != pending.expected.file_state
            || receipt.stamp.generation != pending.generation
        {
            return Err(note_state_changed(
                "receipt does not match submitted recovery",
            ));
        }
        let text = pending.text.clone();
        self.acknowledged = receipt.stamp;
        self.view.buffer = text;
        self.view.stamp = receipt.stamp;
        if self
            .failure
            .as_ref()
            .is_none_or(|failure| failure.filesystem_outcome != FileOutcome::Unknown)
            && !matches!(
                self.view.availability,
                NoteAvailability::Conflict | NoteAvailability::Uncertain
            )
        {
            self.failure = None;
        }
        self.clear_pending();
        self.queue_recovery();
        Ok(())
    }
    pub fn acknowledge_copy(
        &mut self,
        op: Uuid,
        receipt: NoteReceipt,
        destination: &NoteView,
    ) -> NoteResult<()> {
        let pending = self.submitted(op, PendingKind::Copy)?;
        if receipt.operation_id != op
            || receipt.source_note_id != pending.note_id
            || receipt.note_id == pending.note_id
            || receipt.submitted_generation != pending.generation
            || receipt.filesystem_outcome != FileOutcome::Applied
        {
            return Err(note_state_changed(
                "copy receipt does not match submitted copy",
            ));
        }
        if destination.id != receipt.note_id
            || destination.vault_id != self.view.vault_id
            || destination.stamp != receipt.stamp
            || destination.buffer != pending.text
            || destination.saved.as_deref() != Some(pending.text.as_str())
            || destination.current_file_state != Some(receipt.stamp.file_state)
            || destination.availability != NoteAvailability::Available
        {
            let failure = NoteFailure {
                code: NoteErrorCode::StateChanged,
                message: "Copy was verified at save completion, but its fresh destination no longer matches; original editor retained".into(),
                operation_id: Some(op),
                note_id: Some(receipt.note_id),
                phase: Some(brn_workflow::notes::SavePhase::Complete),
                filesystem_outcome: receipt.filesystem_outcome,
                recovery_available: receipt.recovery_available,
            };
            self.copy_failure = Some(failure.clone());
            self.clear_pending();
            return Err(failure);
        }
        // Copies do not acknowledge the source baseline or resolve an uncertain original.
        self.copy_failure = None;
        self.clear_pending();
        Ok(())
    }
    pub fn acknowledge_discard(&mut self, op: Uuid, view: NoteView) -> NoteResult<()> {
        let pending = self.submitted(op, PendingKind::Discard)?;
        if view.id != pending.note_id || view.vault_id != self.view.vault_id {
            return Err(note_state_changed("reload receipt belongs to another note"));
        }
        if self.generation != pending.generation || self.text != pending.text {
            self.acknowledged = view.stamp;
            self.saved = view.saved.clone();
            self.view = view;
            if self.generation <= self.acknowledged.generation {
                self.generation = self
                    .acknowledged
                    .generation
                    .checked_add(1)
                    .filter(|generation| *generation <= i64::MAX as u64)
                    .ok_or_else(|| note_state_changed("note generation exhausted after reload"))?;
            }
            self.clear_pending();
            self.queue_recovery();
            return Err(note_state_changed(
                "reload completed in storage; later typing retained and requires recovery",
            ));
        }
        *self = Self::new(view);
        Ok(())
    }
    pub fn discard_unrecovered(&mut self, op: Uuid) -> NoteResult<()> {
        let pending = self.submitted(op, PendingKind::Discard)?;
        if pending.expected != self.acknowledged
            || pending.generation != self.generation
            || pending.text != self.text
        {
            return Err(note_state_changed(
                "discard confirmation does not match current typing",
            ));
        }
        self.text = self.view.buffer.clone();
        self.generation = self.acknowledged.generation;
        self.clear_pending();
        self.queue_recovery();
        Ok(())
    }
    pub fn acknowledge_rebase(&mut self, op: Uuid, view: NoteView) -> NoteResult<()> {
        let pending = self.submitted(op, PendingKind::Rebase)?;
        if view.id != pending.note_id
            || view.vault_id != self.view.vault_id
            || view.buffer != pending.text
            || view.stamp.generation != pending.generation
        {
            return Err(note_state_changed(
                "decision receipt does not match submitted baseline",
            ));
        }
        self.acknowledged = view.stamp;
        self.saved = view.saved.clone();
        self.view = view;
        self.clear_pending();
        self.queue_recovery();
        Ok(())
    }
    pub fn fail(&mut self, op: Uuid, failure: NoteFailure) -> NoteResult<()> {
        if self
            .pending
            .as_ref()
            .is_none_or(|pending| pending.operation_id != op)
            || failure
                .operation_id
                .is_some_and(|failure_op| failure_op != op)
            || (self.pending_kind != Some(PendingKind::Copy)
                && failure.note_id.is_some_and(|id| id != self.id()))
        {
            return Err(note_state_changed(
                "failure does not match pending operation",
            ));
        }
        if self.pending_kind == Some(PendingKind::Copy) {
            self.copy_failure = Some(failure);
        } else {
            self.failure = Some(failure);
        }
        self.clear_pending();
        Ok(())
    }
    pub fn observe(&mut self, view: NoteView) -> NoteResult<()> {
        if view.id != self.id() || view.vault_id != self.view.vault_id || self.pending() {
            return Err(note_state_changed("observation does not match idle editor"));
        }
        // A failed save may have committed its input before the filesystem refusal.
        // Adopt only durable stamps that correspond to text this editor has submitted.
        if (view.stamp.generation == self.generation && view.buffer == self.text)
            || self.last_submission.as_ref().is_some_and(|request| {
                view.stamp.generation == request.generation && view.buffer == request.text
            })
            || view.stamp == self.acknowledged
        {
            self.acknowledged = view.stamp;
        } else {
            return Err(note_state_changed(
                "observation cannot rebase unacknowledged edits",
            ));
        }
        self.saved = view.saved.clone();
        self.view = view;
        self.queue_recovery();
        Ok(())
    }
    pub fn display_state(&self) -> &'static str {
        if self.view.availability == NoteAvailability::Uncertain
            || self
                .failure
                .as_ref()
                .is_some_and(|error| error.filesystem_outcome == FileOutcome::Unknown)
        {
            "Save outcome uncertain"
        } else if self.view.availability == NoteAvailability::Conflict {
            "External conflict"
        } else if self.pending_kind == Some(PendingKind::Save)
            || self.pending_kind == Some(PendingKind::Copy)
        {
            "Saving to Markdown"
        } else if self.needs_recovery() {
            "Unsaved"
        } else if self.dirty() {
            "Recoverable in BRN"
        } else {
            "Saved to Markdown"
        }
    }
}

fn note_state_changed(message: &str) -> NoteFailure {
    NoteFailure {
        code: NoteErrorCode::StateChanged,
        message: message.into(),
        operation_id: None,
        note_id: None,
        phase: None,
        filesystem_outcome: FileOutcome::NotApplied,
        recovery_available: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_workflow::notes::{
        FileOutcome, NoteAvailability, NoteBufferReceipt, NoteReceipt, NoteStamp, NoteSubmission,
        NoteView,
    };
    use brn_workflow::worker::Approval;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn opened(text: &str) -> NoteView {
        let file_state = Uuid::new_v4();
        NoteView {
            id: Uuid::new_v4(),
            vault_id: Uuid::new_v4(),
            relative_path: PathBuf::from("plan.md"),
            stamp: NoteStamp {
                file_state,
                generation: 0,
            },
            current_file_state: Some(file_state),
            saved: Some(text.into()),
            buffer: text.into(),
            availability: NoteAvailability::Available,
            availability_message: None,
            search_approval: Approval::Draft,
        }
    }

    fn receipt(request: &NoteSubmission) -> NoteReceipt {
        NoteReceipt {
            operation_id: request.operation_id,
            source_note_id: request.note_id,
            note_id: request.note_id,
            submitted_generation: request.generation,
            stamp: NoteStamp {
                file_state: Uuid::new_v4(),
                generation: request.generation,
            },
            filesystem_outcome: FileOutcome::Applied,
            recovery_available: true,
        }
    }

    #[test]
    fn typing_after_save_submission_survives_receipt() {
        let mut editor = NoteEditor::new(opened("base"));
        editor.edit("first".into()).unwrap();
        let submitted = editor.begin_save(Uuid::new_v4()).unwrap();
        editor.edit("later".into()).unwrap();
        editor
            .acknowledge(submitted.operation_id, receipt(&submitted))
            .unwrap();
        assert_eq!(editor.text(), "later");
        assert!(editor.dirty());
        assert!(!editor.can_close());
        let recovery = editor.begin_recovery(Uuid::new_v4()).unwrap();
        assert_eq!(recovery.text, "later");
        assert_eq!(recovery.generation, 2);
        assert_eq!(recovery.expected.generation, 1);
    }

    #[test]
    fn unrelated_and_duplicate_receipts_cannot_acknowledge_text() {
        let mut editor = NoteEditor::new(opened("base"));
        editor.edit("local".into()).unwrap();
        let request = editor.begin_save(Uuid::new_v4()).unwrap();
        let mut wrong = receipt(&request);
        wrong.source_note_id = Uuid::new_v4();
        assert!(editor.acknowledge(request.operation_id, wrong).is_err());
        assert!(editor.pending());
        let result = receipt(&request);
        editor
            .acknowledge(request.operation_id, result.clone())
            .unwrap();
        editor.edit("later".into()).unwrap();
        assert!(editor.acknowledge(request.operation_id, result).is_err());
        assert_eq!(editor.text(), "later");
        assert!(editor.dirty());
    }

    #[test]
    fn recovery_acknowledges_only_submitted_generation_not_markdown() {
        let mut editor = NoteEditor::new(opened("base"));
        editor.edit("one".into()).unwrap();
        let first = editor.begin_recovery(Uuid::new_v4()).unwrap();
        editor.edit("two".into()).unwrap();
        editor.edit("latest".into()).unwrap();
        editor
            .acknowledge_recovery(
                first.operation_id,
                NoteBufferReceipt {
                    operation_id: first.operation_id,
                    note_id: first.note_id,
                    stamp: NoteStamp {
                        file_state: first.expected.file_state,
                        generation: first.generation,
                    },
                },
            )
            .unwrap();
        assert!(!editor.can_close());
        let latest = editor.begin_recovery(Uuid::new_v4()).unwrap();
        assert_eq!(latest.text, "latest");
        assert_eq!(latest.generation, 3);
        editor
            .acknowledge_recovery(
                latest.operation_id,
                NoteBufferReceipt {
                    operation_id: latest.operation_id,
                    note_id: latest.note_id,
                    stamp: NoteStamp {
                        file_state: latest.expected.file_state,
                        generation: latest.generation,
                    },
                },
            )
            .unwrap();
        assert!(editor.can_close());
        assert!(editor.dirty());
        assert_eq!(editor.display_state(), "Recoverable in BRN");
    }

    #[test]
    fn copy_receipt_never_acknowledges_or_switches_original() {
        let mut editor = NoteEditor::new(opened("base"));
        editor.edit("copy input".into()).unwrap();
        let request = editor.begin_copy(Uuid::new_v4()).unwrap();
        editor.edit("later original".into()).unwrap();
        let mut result = receipt(&request);
        assert!(
            editor
                .acknowledge_copy(request.operation_id, result.clone(), &opened("copy input"))
                .is_err()
        );
        let mut destination = opened("copy input");
        destination.vault_id = editor.view().vault_id;
        result.note_id = destination.id;
        destination.stamp = result.stamp;
        destination.current_file_state = Some(result.stamp.file_state);
        editor
            .acknowledge_copy(request.operation_id, result, &destination)
            .unwrap();
        assert_eq!(editor.id(), request.note_id);
        assert_eq!(editor.text(), "later original");
        assert!(editor.dirty());
        assert!(!editor.can_close());
    }

    #[test]
    fn failed_flush_keeps_close_blocked_until_successful_retry() {
        let mut editor = NoteEditor::new(opened("base"));
        editor.edit("local".into()).unwrap();
        let request = editor.begin_recovery(Uuid::new_v4()).unwrap();
        editor
            .fail(request.operation_id, note_state_changed("fixture refusal"))
            .unwrap();
        assert!(!editor.can_close());
        assert_eq!(editor.text(), "local");
        let retry = editor.begin_recovery(Uuid::new_v4()).unwrap();
        editor
            .acknowledge_recovery(
                retry.operation_id,
                NoteBufferReceipt {
                    operation_id: retry.operation_id,
                    note_id: retry.note_id,
                    stamp: NoteStamp {
                        file_state: retry.expected.file_state,
                        generation: retry.generation,
                    },
                },
            )
            .unwrap();
        assert!(editor.can_close());
    }

    #[test]
    fn save_conflict_keeps_later_edits_scheduled_for_recovery() {
        let now = Instant::now();
        let mut schedule = NoteScheduler::default();
        let mut editor = NoteEditor::new(opened("base"));
        editor.edit("save input".into()).unwrap();
        let save = editor.begin_save(Uuid::new_v4()).unwrap();
        let mut failure = note_state_changed("external change");
        failure.code = NoteErrorCode::Conflict;
        schedule.note_failed(&editor);
        editor.fail(save.operation_id, failure).unwrap();
        editor.edit("later typing".into()).unwrap();
        schedule.edited(now);
        assert!(!schedule.wants_recovery(now + Duration::from_millis(499), &editor, true));
        assert!(schedule.wants_recovery(now + Duration::from_millis(500), &editor, true));
        let recovery = editor.begin_recovery(Uuid::new_v4()).unwrap();
        assert_eq!(recovery.text, "later typing");
        assert_eq!(recovery.generation, 2);
    }

    #[test]
    fn uncertain_reconcile_failure_keeps_edits_scheduled_for_recovery() {
        let now = Instant::now();
        let mut schedule = NoteScheduler::default();
        let mut view = opened("base");
        view.availability = NoteAvailability::Uncertain;
        let mut editor = NoteEditor::new(view);
        // Reconciliation has no pending editor submission to acknowledge.
        schedule.note_failed(&editor);
        editor
            .edit("protected after reconciliation".into())
            .unwrap();
        schedule.edited(now);
        assert!(!schedule.wants_recovery(now + Duration::from_millis(499), &editor, true));
        assert!(schedule.wants_recovery(now + Duration::from_millis(500), &editor, true));
        let recovery = editor.begin_recovery(Uuid::new_v4()).unwrap();
        assert_eq!(recovery.text, "protected after reconciliation");
        assert_eq!(editor.display_state(), "Save outcome uncertain");
    }

    #[test]
    fn close_after_save_failure_flushes_and_finishes_on_recovery_acknowledgement() {
        for route in [CloseRoute::Window, CloseRoute::Quit, CloseRoute::Switch] {
            let now = Instant::now();
            let mut schedule = NoteScheduler::default();
            let mut editor = NoteEditor::new(opened("base"));
            editor.edit("save input".into()).unwrap();
            let save = editor.begin_save(Uuid::new_v4()).unwrap();
            editor.edit("latest typing".into()).unwrap();
            schedule.edited(now);
            schedule.request_close(route);
            assert!(!schedule.can_finish(&editor, true));
            schedule.note_failed(&editor);
            editor
                .fail(save.operation_id, note_state_changed("save refused"))
                .unwrap();
            assert!(!schedule.wants_recovery(now, &editor, false));
            assert!(schedule.wants_recovery(now, &editor, true));
            let recovery = editor.begin_recovery(Uuid::new_v4()).unwrap();
            assert_eq!(recovery.text, "latest typing");
            assert!(!schedule.can_finish(&editor, true));
            editor
                .acknowledge_recovery(
                    recovery.operation_id,
                    NoteBufferReceipt {
                        operation_id: recovery.operation_id,
                        note_id: recovery.note_id,
                        stamp: NoteStamp {
                            file_state: recovery.expected.file_state,
                            generation: recovery.generation,
                        },
                    },
                )
                .unwrap();
            assert!(!schedule.can_finish(&editor, false));
            assert!(schedule.can_finish(&editor, true));
            assert_eq!(schedule.take_close(), Some(route));
            assert_eq!(editor.text(), "latest typing");
            assert!(editor.dirty());
        }
    }

    #[test]
    fn refused_reload_acknowledgement_keeps_later_text_scheduled() {
        let now = Instant::now();
        let mut schedule = NoteScheduler::default();
        let mut editor = NoteEditor::new(opened("base"));
        let reload = editor.begin_discard(Uuid::new_v4()).unwrap();
        editor.edit("later typing".into()).unwrap();
        schedule.edited(now);
        let mut reloaded = editor.view().clone();
        reloaded.saved = Some("new disk".into());
        reloaded.buffer = "new disk".into();
        reloaded.stamp.file_state = Uuid::new_v4();
        assert!(
            editor
                .acknowledge_discard(reload.operation_id, reloaded)
                .is_err()
        );
        schedule.note_failed(&editor);
        assert!(schedule.wants_recovery(now + Duration::from_millis(500), &editor, true));
        let recovery = editor.begin_recovery(Uuid::new_v4()).unwrap();
        assert_eq!(recovery.text, "later typing");
    }

    #[test]
    fn recovery_failure_still_requires_explicit_retry_after_later_typing() {
        let now = Instant::now();
        let mut schedule = NoteScheduler::default();
        let mut editor = NoteEditor::new(opened("base"));
        editor.edit("recovery input".into()).unwrap();
        let recovery = editor.begin_recovery(Uuid::new_v4()).unwrap();
        schedule.note_failed(&editor);
        editor
            .fail(
                recovery.operation_id,
                note_state_changed("recovery refused"),
            )
            .unwrap();
        editor.edit("later typing".into()).unwrap();
        schedule.edited(now);
        schedule.request_close(CloseRoute::Window);
        assert!(!schedule.wants_recovery(now + Duration::from_secs(1), &editor, true));
        assert!(!schedule.can_finish(&editor, true));
        schedule.retry();
        assert!(schedule.wants_recovery(Instant::now(), &editor, true));
        assert_eq!(
            editor.begin_recovery(Uuid::new_v4()).unwrap().text,
            "later typing"
        );
    }

    #[test]
    fn confirmed_reload_receipt_cannot_discard_typing_after_submission() {
        let mut editor = NoteEditor::new(opened("base"));
        editor.edit("local".into()).unwrap();
        let request = editor.begin_discard(Uuid::new_v4()).unwrap();
        editor.edit("later".into()).unwrap();
        let mut reloaded = opened("disk");
        reloaded.id = request.note_id;
        reloaded.vault_id = editor.view().vault_id;
        assert!(
            editor
                .acknowledge_discard(request.operation_id, reloaded)
                .is_err()
        );
        assert_eq!(editor.text(), "later");
        assert!(!editor.can_close());
        let recovery = editor.begin_recovery(Uuid::new_v4()).unwrap();
        assert_eq!(recovery.text, "later");
    }

    #[test]
    fn exact_native_text_path_preserves_bom_crlf_unicode_and_frontmatter() {
        let bytes = "\u{feff}---\r\ntitle: café 🧭\r\n---\r\n正文\r\n";
        let mut editor = NoteEditor::new(opened(bytes));
        let displayed =
            gpui_kit::component::input::Rope::from(editor.input_text().as_str()).to_string();
        editor.edit_input(displayed).unwrap();
        assert_eq!(editor.text().as_bytes(), bytes.as_bytes());
        assert!(!editor.dirty());
        editor
            .edit_input("\u{feff}---\r\ntitle: café 🧭\r\n---\r\n正文!\r\n".into())
            .unwrap();
        assert_eq!(
            editor.text(),
            "\u{feff}---\r\ntitle: café 🧭\r\n---\r\n正文!\r\n"
        );
    }

    #[test]
    fn all_close_routes_wait_for_idle_flush_and_later_edits() {
        use std::time::{Duration, Instant};
        for route in [CloseRoute::Window, CloseRoute::Quit, CloseRoute::Switch] {
            let now = Instant::now();
            let mut schedule = NoteScheduler::default();
            let mut editor = NoteEditor::new(opened("base"));
            editor.edit("first".into()).unwrap();
            schedule.edited(now);
            schedule.request_close(route);
            assert!(!schedule.can_finish(&editor, false));
            assert!(!schedule.wants_recovery(now, &editor, false));
            assert!(schedule.wants_recovery(now, &editor, true));
            let save = editor.begin_save(Uuid::new_v4()).unwrap();
            assert!(!schedule.can_finish(&editor, true));
            editor.edit("later".into()).unwrap();
            schedule.edited(now + Duration::from_millis(10));
            editor
                .acknowledge(save.operation_id, receipt(&save))
                .unwrap();
            assert!(!schedule.can_finish(&editor, true));
            let flush = editor.begin_recovery(Uuid::new_v4()).unwrap();
            editor
                .fail(flush.operation_id, note_state_changed("disk full"))
                .unwrap();
            schedule.recovery_failed();
            assert!(!schedule.can_finish(&editor, true));
            assert!(!schedule.wants_recovery(now, &editor, true));
            schedule.retry();
            assert!(schedule.wants_recovery(now, &editor, true));
            let flush = editor.begin_recovery(Uuid::new_v4()).unwrap();
            editor
                .acknowledge_recovery(
                    flush.operation_id,
                    NoteBufferReceipt {
                        operation_id: flush.operation_id,
                        note_id: flush.note_id,
                        stamp: NoteStamp {
                            file_state: flush.expected.file_state,
                            generation: flush.generation,
                        },
                    },
                )
                .unwrap();
            assert!(schedule.can_finish(&editor, true));
            assert_eq!(schedule.take_close(), Some(route));
            assert_eq!(editor.text(), "later");
            assert!(editor.dirty());
        }
    }

    #[test]
    fn recovery_debounces_for_500_ms_and_submits_only_latest_text() {
        use std::time::{Duration, Instant};
        let now = Instant::now();
        let mut schedule = NoteScheduler::default();
        let mut editor = NoteEditor::new(opened("base"));
        editor.edit("old".into()).unwrap();
        schedule.edited(now);
        editor.edit("latest".into()).unwrap();
        schedule.edited(now + Duration::from_millis(400));
        assert!(!schedule.wants_recovery(now + Duration::from_millis(899), &editor, true));
        assert!(schedule.wants_recovery(now + Duration::from_millis(900), &editor, true));
        assert_eq!(
            editor.begin_recovery(Uuid::new_v4()).unwrap().text,
            "latest"
        );
    }

    #[test]
    fn observation_rebases_failed_submitted_input_without_losing_later_typing() {
        let view = opened("base");
        let mut editor = NoteEditor::new(view.clone());
        editor.edit("submitted".into()).unwrap();
        let request = editor.begin_save(Uuid::new_v4()).unwrap();
        editor.edit("later".into()).unwrap();
        editor
            .fail(request.operation_id, note_state_changed("fixture conflict"))
            .unwrap();
        let mut observed = view;
        observed.buffer = "submitted".into();
        observed.stamp.generation = 1;
        editor.observe(observed).unwrap();
        let latest = editor.begin_recovery(Uuid::new_v4()).unwrap();
        assert_eq!(latest.expected.generation, 1);
        assert_eq!(latest.generation, 2);
        assert_eq!(latest.text, "later");
    }

    #[test]
    fn copied_rescue_failure_keeps_uncertain_original_outcome_visible() {
        let mut editor = NoteEditor::new(opened("base"));
        let save = editor.begin_save(Uuid::new_v4()).unwrap();
        let mut original = note_state_changed("original uncertain");
        original.operation_id = Some(save.operation_id);
        original.note_id = Some(save.note_id);
        original.filesystem_outcome = FileOutcome::Unknown;
        editor.fail(save.operation_id, original.clone()).unwrap();
        let copy = editor.begin_copy(Uuid::new_v4()).unwrap();
        editor
            .fail(copy.operation_id, note_state_changed("copy occupied"))
            .unwrap();
        assert_eq!(editor.failure(), Some(&original));
        assert_eq!(editor.display_state(), "Save outcome uncertain");
    }

    #[test]
    fn unrelated_failure_does_not_release_pending_generation() {
        let mut editor = NoteEditor::new(opened("base"));
        let request = editor.begin_save(Uuid::new_v4()).unwrap();
        let mut failure = note_state_changed("unrelated");
        failure.operation_id = Some(Uuid::new_v4());
        assert!(editor.fail(request.operation_id, failure).is_err());
        assert!(editor.pending());
        assert!(!editor.can_close());
    }

    #[test]
    fn confirmed_transient_discard_restores_only_acknowledged_recovery() {
        let mut editor = NoteEditor::new(opened("disk"));
        editor.edit("recovered".into()).unwrap();
        let recovered = editor.begin_recovery(Uuid::new_v4()).unwrap();
        editor
            .acknowledge_recovery(
                recovered.operation_id,
                NoteBufferReceipt {
                    operation_id: recovered.operation_id,
                    note_id: recovered.note_id,
                    stamp: NoteStamp {
                        file_state: recovered.expected.file_state,
                        generation: recovered.generation,
                    },
                },
            )
            .unwrap();
        editor.edit("unacknowledged".into()).unwrap();
        let discard = editor.begin_discard(Uuid::new_v4()).unwrap();
        assert!(editor.discard_unrecovered(Uuid::new_v4()).is_err());
        editor.discard_unrecovered(discard.operation_id).unwrap();
        assert_eq!(editor.text(), "recovered");
        assert!(editor.dirty());
        assert!(editor.can_close());
    }

    #[test]
    fn edit_limit_is_exact_utf8_bytes_and_empty_files_remain_valid() {
        let mut editor = NoteEditor::new(opened(""));
        assert!(!editor.dirty());
        let exact = "é".repeat(512 * 1024);
        editor.edit(exact.clone()).unwrap();
        assert_eq!(editor.text().len(), 1024 * 1024);
        assert!(editor.edit(format!("{exact}x")).is_err());
        assert_eq!(editor.text(), exact);
        editor.edit(String::new()).unwrap();
        assert_eq!(editor.text(), "");
    }

    #[test]
    fn mixed_line_endings_survive_gpui_rope_edit_without_normalization() {
        use gpui_kit::component::input::{Rope, RopeExt};
        let exact = "\u{feff}---\r\ntitle: e\u{301}\n---\r\n正文 🧭\r";
        let mut editor = NoteEditor::new(opened(exact));
        let mut text = Rope::from(editor.input_text().as_str());
        let start = exact.find("正文").unwrap();
        text.replace(start..start + "正文".len(), "文書");
        editor.edit_input(text.to_string()).unwrap();
        assert_eq!(
            editor.text(),
            "\u{feff}---\r\ntitle: e\u{301}\n---\r\n文書 🧭\r"
        );
        let submitted = editor.begin_save(Uuid::new_v4()).unwrap();
        assert_eq!(
            submitted.text,
            "\u{feff}---\r\ntitle: e\u{301}\n---\r\n文書 🧭\r"
        );
    }

    #[test]
    fn notices_coalesce_until_idle_and_rescan_observes_all_registered_notes() {
        use brn_workflow::notes::{NoteFileNotice, NoteNoticeKind};
        let first = opened("first");
        let mut second = opened("second");
        second.vault_id = first.vault_id;
        second.relative_path = "second.md".into();
        let views = vec![first.clone(), second.clone()];
        let mut observations = NoteObservations::default();
        let changed = NoteFileNotice {
            vault_id: first.vault_id,
            relative_path: Some(first.relative_path.clone()),
            kind: NoteNoticeKind::Moved,
        };
        observations.notice(&changed, &views);
        observations.notice(&changed, &views);
        assert!(observations.take(false, &[]).is_none());
        assert_eq!(observations.take(true, &[]), Some(vec![first.id]));
        observations.notice(
            &NoteFileNotice {
                kind: NoteNoticeKind::RescanRequired,
                relative_path: None,
                ..changed
            },
            &views,
        );
        let registered = vec![first.id, second.id, Uuid::new_v4()];
        let ready = observations.take(true, &registered).unwrap();
        assert_eq!(ready.len(), 3);
        for id in registered {
            assert!(ready.contains(&id));
        }
        assert!(observations.take(true, &[]).is_none());
        assert_eq!(views[0].relative_path, PathBuf::from("plan.md"));
    }

    #[test]
    fn acknowledged_recovery_does_not_erase_uncertain_original_outcome() {
        let mut editor = NoteEditor::new(opened("base"));
        let original = editor.begin_save(Uuid::new_v4()).unwrap();
        let mut failure = note_state_changed("original uncertain");
        failure.filesystem_outcome = FileOutcome::Unknown;
        editor.fail(original.operation_id, failure.clone()).unwrap();
        editor.edit("rescue edits".into()).unwrap();
        let recovery = editor.begin_recovery(Uuid::new_v4()).unwrap();
        editor
            .acknowledge_recovery(
                recovery.operation_id,
                NoteBufferReceipt {
                    operation_id: recovery.operation_id,
                    note_id: recovery.note_id,
                    stamp: NoteStamp {
                        file_state: recovery.expected.file_state,
                        generation: recovery.generation,
                    },
                },
            )
            .unwrap();
        assert_eq!(editor.failure(), Some(&failure));
        assert_eq!(editor.display_state(), "Save outcome uncertain");
        assert!(editor.can_close());
    }

    #[test]
    fn changed_copy_destination_releases_copy_job_without_switching_original() {
        let mut editor = NoteEditor::new(opened("base"));
        let request = editor.begin_copy(Uuid::new_v4()).unwrap();
        let mut result = receipt(&request);
        let mut destination = opened("changed externally");
        destination.vault_id = editor.view().vault_id;
        destination.stamp = result.stamp;
        result.note_id = destination.id;
        let failure = editor
            .acknowledge_copy(request.operation_id, result, &destination)
            .unwrap_err();
        assert_eq!(failure.filesystem_outcome, FileOutcome::Applied);
        assert!(failure.recovery_available);
        assert!(!editor.pending());
        assert_eq!(editor.text(), "base");
        assert_eq!(editor.id(), request.note_id);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn gpui_text_submission_round_trips_actual_markdown_through_owned_worker() {
        use brn_workflow::worker::{Action, Outcome, Terminal, Worker};
        use gpui_kit::component::input::{Rope, RopeExt};
        use std::{
            fs,
            os::unix::fs::MetadataExt,
            time::{Duration, Instant},
        };

        struct Fixture(PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                fs::remove_dir_all(&self.0).expect("remove this test's unique disposable fixture");
            }
        }
        fn terminal(worker: &Worker) -> Terminal {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                if let Some(terminal) = worker.take_terminal() {
                    return terminal;
                }
                assert!(Instant::now() < deadline, "owned note worker timed out");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let fixture =
            Fixture(std::env::temp_dir().join(format!("brn-note-editor-{}", Uuid::new_v4())));
        fs::create_dir(&fixture.0).unwrap();
        let data = fixture.0.join("data");
        let vault = fixture.0.join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        let path = vault.join("plan.md");
        let original = "\u{feff}---\r\ntitle: café 🧭\r\n---\r\n正文\r\n";
        fs::write(&path, original).unwrap();
        let before = fs::metadata(&path).unwrap();
        let mut worker = Worker::start(data, brn_workflow::Config::default());
        terminal(&worker).outcome.unwrap();
        worker
            .submit(Action::OpenNote {
                op: Uuid::new_v4(),
                vault,
                relative: PathBuf::from("plan.md"),
            })
            .unwrap();
        let view = match terminal(&worker).outcome.unwrap() {
            Outcome::NoteOpened { view, .. } => view,
            other => panic!("unexpected open: {other:?}"),
        };
        let mut editor = NoteEditor::new(view);
        let mut rope = Rope::from(editor.input_text().as_str());
        editor
            .edit_input(gpui_kit::SharedString::new(rope.to_string()).to_string())
            .unwrap();
        let unchanged = editor.begin_save(Uuid::new_v4()).unwrap();
        worker
            .submit(Action::SaveNote {
                request: unchanged.clone(),
            })
            .unwrap();
        let receipt = match terminal(&worker).outcome.unwrap() {
            Outcome::NoteSaved { receipt, .. } => receipt,
            other => panic!("unexpected save: {other:?}"),
        };
        assert_eq!(receipt.filesystem_outcome, FileOutcome::NotApplied);
        editor.acknowledge(unchanged.operation_id, receipt).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original.as_bytes());
        let after = fs::metadata(&path).unwrap();
        assert_eq!(after.ino(), before.ino());
        assert_eq!(after.modified().unwrap(), before.modified().unwrap());

        let start = original.find("正文").unwrap();
        rope.replace(start..start + "正文".len(), "文書 🦀");
        editor
            .edit_input(gpui_kit::SharedString::new(rope.to_string()).to_string())
            .unwrap();
        let changed = editor.begin_save(Uuid::new_v4()).unwrap();
        worker
            .submit(Action::SaveNote {
                request: changed.clone(),
            })
            .unwrap();
        let receipt = match terminal(&worker).outcome.unwrap() {
            Outcome::NoteSaved { receipt, .. } => receipt,
            other => panic!("unexpected save: {other:?}"),
        };
        assert_eq!(receipt.filesystem_outcome, FileOutcome::Applied);
        editor.acknowledge(changed.operation_id, receipt).unwrap();
        assert_eq!(
            fs::read(&path).unwrap(),
            "\u{feff}---\r\ntitle: café 🧭\r\n---\r\n文書 🦀\r\n".as_bytes()
        );
        assert!(!editor.dirty());
        worker.shutdown();
    }
}
