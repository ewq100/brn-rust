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
/// Body-free identity of an Action affected by this recorded approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityActionChangeKind {
    Created,
    Replaced,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityActionChange {
    pub kind: ActivityActionChangeKind,
    pub action_id: Uuid,
    /// Complete title in the historical approved after-data, never a current lookup.
    pub title: String,
}
impl From<&ActionChange> for ActivityActionChange {
    fn from(change: &ActionChange) -> Self {
        Self {
            kind: match change {
                ActionChange::Create { .. } => ActivityActionChangeKind::Created,
                ActionChange::Replace { .. } => ActivityActionChangeKind::Replaced,
            },
            action_id: change.id(),
            title: change.data().title.clone(),
        }
    }
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
    /// Ordered members from the same checked Applied approval as the note changes.
    #[serde(default)]
    pub action_changes: Vec<ActivityActionChange>,
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
                            NoteChange::Create { .. } | NoteChange::CreateAsset { .. } => {
                                ActivityChangeKind::Created
                            }
                            NoteChange::Replace { .. } | NoteChange::ReplaceAsset { .. } => {
                                ActivityChangeKind::Replaced
                            }
                            NoteChange::Trash { .. } | NoteChange::TrashAsset { .. } => {
                                ActivityChangeKind::Trashed
                            }
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
                    action_changes: draft
                        .action_changes
                        .iter()
                        .map(ActivityActionChange::from)
                        .collect(),
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

    fn action_change(index: usize) -> ActionChange {
        use brn_store::work::{
            actions::{ActionData, ActionOrigin, ActionRecord, ActionState},
            proposals::ProposalStamp,
        };
        let id = Uuid::from_u128(index as u128 + 1);
        let data = ActionData {
            title: format!("{index:02} õ 日本語\r\n{}", "λ".repeat(230)),
            description: "PRIVATE FULL ACTION BODY".repeat(2000),
            state: ActionState::Open,
            owner: None,
            related_person: None,
            related_project: None,
            sources: vec![],
            thread: None,
            due_on: None,
            follow_up_on: None,
            dependencies: vec![],
            parent: None,
            follows_up: None,
            priority: None,
        };
        data.validate(id).unwrap();
        if index.is_multiple_of(2) {
            ActionChange::Create { id, data }
        } else {
            let before = ActionRecord {
                origin: ActionOrigin {
                    id,
                    proposal: ProposalStamp {
                        id: Uuid::from_u128(1000 + index as u128),
                        version: 1,
                    },
                    data: data.clone(),
                    created_at_ms: 1,
                },
                version: 1,
                data: data.clone(),
                updated_at_ms: 1,
                waiting_since_ms: None,
                completed_at_ms: None,
            };
            before.validate().unwrap();
            ActionChange::Replace {
                before: Box::new(before),
                data,
            }
        }
    }
    #[test]
    fn activity_action_inventory_portable_projection_retains_all_64_ordered_members_without_bodies()
    {
        let changes: Vec<_> = (0..brn_store::work::proposals::MAX_PROPOSAL_CHANGES)
            .map(action_change)
            .collect();
        let inventory: Vec<_> = changes.iter().map(ActivityActionChange::from).collect();
        assert_eq!(inventory.len(), 64);
        for (index, (original, item)) in changes.iter().zip(&inventory).enumerate() {
            assert_eq!(item.action_id, original.id());
            assert_eq!(item.title, original.data().title);
            assert_eq!(
                item.kind,
                if index.is_multiple_of(2) {
                    ActivityActionChangeKind::Created
                } else {
                    ActivityActionChangeKind::Replaced
                }
            );
        }
        let encoded = serde_json::to_string(&inventory).unwrap();
        for private in [
            "PRIVATE FULL ACTION BODY",
            "description",
            "origin",
            "before",
            "state",
        ] {
            assert!(!encoded.contains(private));
        }
        assert_eq!(
            serde_json::from_str::<Vec<ActivityActionChange>>(&encoded).unwrap(),
            inventory
        );
        assert!(
            serde_json::from_value::<ActivityActionChange>(
                serde_json::json!({"kind":"trashed","action_id":Uuid::new_v4(),"title":"invalid"})
            )
            .is_err()
        );
    }
    #[test]
    fn activity_action_inventory_legacy_dto_defaults_to_consistently_serialized_empty_list() {
        let old = serde_json::json!({"operation_id":Uuid::new_v4(),"proposal_id":Uuid::new_v4(),"group_id":null,"session_id":null,"title":"Historical note only","approved_at_ms":1,"approved_at_utc":null,"summary":"Created 1 note.","changes":[]});
        let entry: ActivityEntry = serde_json::from_value(old).unwrap();
        assert!(entry.action_changes.is_empty());
        assert_eq!(
            serde_json::to_value(entry).unwrap()["action_changes"],
            serde_json::json!([])
        );
    }

    #[test]
    fn timestamps_render_utc_without_wrapping_unrepresentable_values() {
        assert_eq!(utc_time(0).as_deref(), Some("1970-01-01T00:00:00.000Z"));
        assert_eq!(utc_time(1001).as_deref(), Some("1970-01-01T00:00:01.001Z"));
        assert!(utc_time(u64::MAX).is_none());
        assert!(utc_time(i64::MAX as u64).is_none());
    }
}
