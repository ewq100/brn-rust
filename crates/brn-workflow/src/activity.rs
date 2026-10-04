//! Readable approved-change history projected from operational receipts.
//! History is not current-vault evidence and does not require current reads.
use crate::{ErrorKind, Result, WorkflowError, app::App};
use brn_store::work::{
    proposal_apply::ApplyOutcome,
    proposals::{ActionChange, NoteChange},
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_ACTIVITY_PAGE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityRequest {
    pub limit: usize,
    /// Exclusive cursor identifying an existing successful approval operation.
    pub before: Option<Uuid>,
}
impl Default for ActivityRequest {
    fn default() -> Self {
        Self {
            limit: 20,
            before: None,
        }
    }
}
impl ActivityRequest {
    /// Pure input validation, also used before CLI workspace admission.
    pub fn validate(&self) -> Result<()> {
        if !(1..=MAX_ACTIVITY_PAGE).contains(&self.limit)
            || self.before.is_some_and(|id| id.is_nil())
        {
            return Err(WorkflowError::typed(
                ErrorKind::ToolRejected,
                "activity needs a limit of 1–100 and a non-nil operation cursor",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityChangeKind {
    Created,
    Replaced,
    Trashed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityChange {
    pub kind: ActivityChangeKind,
    pub path: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityUndo {
    pub operation_id: Uuid,
    pub trash_member: Option<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undo: Option<ActivityUndo>,
    pub operation_id: Uuid,
    pub proposal_id: Uuid,
    pub group_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub title: String,
    /// Durable admission time of the reviewed approval, not completion time.
    pub approved_at_ms: u64,
    /// UTC rendering when the recorded timestamp is representable.
    pub approved_at_utc: Option<String>,
    pub summary: String,
    pub changes: Vec<ActivityChange>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityPage {
    pub entries: Vec<ActivityEntry>,
    pub next_before: Option<Uuid>,
}

impl App {
    pub fn activity(&self, request: &ActivityRequest) -> Result<ActivityPage> {
        request.validate()?;
        // The checked journals are the single authority for successful
        // application. Later edits cannot rewrite their historical snapshots.
        // Retain scalar ordering metadata only, decoding one bounded journal at
        // a time even when history contains many full-size proposal bodies.
        let mut order = vec![];
        for id in self.store.proposal_apply_ids()? {
            let journal = self.store.proposal_apply(id)?.ok_or_else(|| {
                WorkflowError::typed(
                    ErrorKind::ContextStale,
                    "listed activity journal disappeared",
                )
            })?;
            if journal
                .receipt
                .as_ref()
                .is_some_and(|receipt| receipt.outcome == ApplyOutcome::Applied)
            {
                order.push((journal.started_at_ms, id));
            }
        }
        order.sort_unstable_by(|a, b| b.cmp(a));
        let offset = match request.before {
            None => 0,
            Some(id) => {
                order
                    .iter()
                    .position(|(_, operation_id)| *operation_id == id)
                    .ok_or_else(|| {
                        WorkflowError::typed(
                            ErrorKind::ContextStale,
                            "activity cursor no longer identifies an approved durable change",
                        )
                    })?
                    + 1
            }
        };
        let has_more = order.len() - offset > request.limit;
        let entries: Vec<_> = order
            .into_iter()
            .skip(offset)
            .take(request.limit)
            .map(|(_, id)| {
                let journal = self.store.proposal_apply(id)?.ok_or_else(|| {
                    WorkflowError::typed(
                        ErrorKind::ContextStale,
                        "paged activity journal disappeared",
                    )
                })?;
                let draft = journal.approved.draft;
                let changes: Vec<_> = draft
                    .changes
                    .into_iter()
                    .map(|change| {
                        let kind = match &change {
                            NoteChange::Create { .. } => ActivityChangeKind::Created,
                            NoteChange::Replace { .. } => ActivityChangeKind::Replaced,
                            NoteChange::Trash { .. } => ActivityChangeKind::Trashed,
                        };
                        ActivityChange {
                            kind,
                            path: change.path().into(),
                        }
                    })
                    .collect();
                Ok(ActivityEntry {
                    undo: journal.undo.map(|binding| ActivityUndo {
                        operation_id: binding.operation_id,
                        trash_member: binding.trash_member,
                    }),
                    operation_id: journal.request.operation_id,
                    proposal_id: draft.id,
                    group_id: draft.group_id,
                    session_id: draft.session_id,
                    title: draft.title,
                    approved_at_ms: journal.started_at_ms,
                    approved_at_utc: utc_time(journal.started_at_ms),
                    summary: summary(&changes, &draft.action_changes),
                    changes,
                })
            })
            .collect::<Result<_>>()?;
        let next_before = has_more.then(|| {
            entries
                .last()
                .expect("nonempty bounded activity page")
                .operation_id
        });
        Ok(ActivityPage {
            entries,
            next_before,
        })
    }
}

fn utc_time(ms: u64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_millis(i64::try_from(ms).ok()?)
        .map(|date| date.to_rfc3339_opts(SecondsFormat::Millis, true))
}
fn summary(changes: &[ActivityChange], actions: &[ActionChange]) -> String {
    let count = |kind| changes.iter().filter(|change| change.kind == kind).count();
    let mut parts = vec![];
    for (kind, first_verb, later_verb, suffix) in [
        (ActivityChangeKind::Created, "Created", "created", ""),
        (ActivityChangeKind::Replaced, "Replaced", "replaced", ""),
        (ActivityChangeKind::Trashed, "Moved", "moved", " to Trash"),
    ] {
        let n = count(kind);
        if n > 0 {
            let verb = if parts.is_empty() {
                first_verb
            } else {
                later_verb
            };
            parts.push(format!(
                "{verb} {n} note{}{suffix}",
                if n == 1 { "" } else { "s" }
            ));
        }
    }
    for (n, first_verb, later_verb) in [
        (
            actions
                .iter()
                .filter(|change| matches!(change, ActionChange::Create { .. }))
                .count(),
            "Created",
            "created",
        ),
        (
            actions
                .iter()
                .filter(|change| matches!(change, ActionChange::Replace { .. }))
                .count(),
            "Updated",
            "updated",
        ),
    ] {
        if n > 0 {
            let verb = if parts.is_empty() {
                first_verb
            } else {
                later_verb
            };
            parts.push(format!(
                "{verb} {n} action{}",
                if n == 1 { "" } else { "s" }
            ));
        }
    }
    format!("{}.", parts.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_render_utc_without_wrapping_unrepresentable_values() {
        assert_eq!(utc_time(0).as_deref(), Some("1970-01-01T00:00:00.000Z"));
        assert_eq!(utc_time(1001).as_deref(), Some("1970-01-01T00:00:01.001Z"));
        assert!(utc_time(u64::MAX).is_none());
        assert!(utc_time(i64::MAX as u64).is_none());
    }
}
