//! Retained operational Actions through the shared application boundary.
//! Current reads require durable-change reconciliation, but no vault or provider.
use crate::{ErrorKind, Result, WorkflowError, app::App};
pub use brn_store::work::actions::{
    ActionCursor, ActionData, ActionListRequest, ActionOrigin, ActionPage, ActionPriority,
    ActionRecord, ActionStamp, ActionState,
};
use uuid::Uuid;

impl App {
    /// Return the exact current baseline, including its immutable approved origin.
    pub fn action(&self, id: Uuid) -> Result<ActionRecord> {
        if id.is_nil() {
            return Err(WorkflowError::typed(
                ErrorKind::ToolRejected,
                "Action UUID must not be nil",
            ));
        }
        self.require_current_evidence()?;
        self.store
            .action(id)?
            .ok_or_else(|| WorkflowError::typed(ErrorKind::NotFound, "Action does not exist"))
    }

    /// Page by immutable creation time and UUID; edits do not reorder retained work.
    pub fn actions(&self, request: &ActionListRequest) -> Result<ActionPage> {
        request
            .validate()
            .map_err(|error| WorkflowError::typed(ErrorKind::ToolRejected, error.to_string()))?;
        self.require_current_evidence()?;
        Ok(self.store.action_list(request)?)
    }
}

// Qualified privately before the existing Action producers are opened.
impl App {
    pub(crate) fn validate_action_references(
        &mut self,
        draft: &crate::proposals::ProposalDraft,
    ) -> Result<()> {
        use crate::proposals::ActionChange;
        use std::collections::{BTreeMap, BTreeSet};

        let mut overlay = BTreeMap::new();
        let mut added = BTreeSet::new();
        for change in &draft.action_changes {
            change
                .validate()
                .map_err(|error| rejected(error.to_string()))?;
            if overlay.insert(change.id(), change.data().clone()).is_some() {
                return Err(rejected(
                    "Draft changes the same Action UUID more than once",
                ));
            }
            let after = change.data();
            let before = match change {
                ActionChange::Create { .. } => None,
                ActionChange::Replace { before, .. } => Some(&before.data),
            };
            // A reference retained in one slot does not grant authority to add
            // that UUID in another slot. Source ordering has no semantic effect.
            for (new, old) in [
                (
                    after.related_person,
                    before.and_then(|data| data.related_person),
                ),
                (
                    after.related_project,
                    before.and_then(|data| data.related_project),
                ),
                (after.thread, before.and_then(|data| data.thread)),
            ] {
                if new != old
                    && let Some(id) = new
                {
                    added.insert(id);
                }
            }
            added.extend(
                after
                    .sources
                    .iter()
                    .filter(|id| !before.is_some_and(|data| data.sources.contains(id)))
                    .copied(),
            );
        }
        if overlay.is_empty() {
            return Ok(());
        }
        self.require_current_evidence()?;
        self.validate_proposal_note_targets(draft, &added)?;

        let roots: Vec<_> = overlay.keys().copied().collect();
        // Follow-up relates retained work; it is not a dependency or hierarchy
        // edge, and neither a Completed-only nor cycle rule is implied.
        let followups: Vec<_> = overlay
            .values()
            .filter_map(|data| data.follows_up)
            .collect();
        for id in followups {
            resolve(self, &mut overlay, id)?;
        }
        check_graph(self, &mut overlay, &roots, false)?;
        check_graph(self, &mut overlay, &roots, true)
    }
}

fn rejected(message: impl Into<String>) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ToolRejected, message)
}

/// Resolve only explicit IDs, using reviewed after-data ahead of checked Store
/// reads. No unrelated stored graph scan or frozen target revisions are needed.
fn resolve<'a>(
    app: &App,
    overlay: &'a mut std::collections::BTreeMap<Uuid, ActionData>,
    id: Uuid,
) -> Result<&'a ActionData> {
    if let std::collections::btree_map::Entry::Vacant(entry) = overlay.entry(id) {
        let record = app
            .store
            .action(id)?
            .ok_or_else(|| rejected("Action reference target does not exist"))?;
        entry.insert(record.data);
    }
    Ok(&overlay[&id])
}

/// Separate iterative DFS traversals avoid stack overflow and refuse reachable
/// missing edges. Grey/black colors are local to the one governed edge relation.
fn check_graph(
    app: &App,
    overlay: &mut std::collections::BTreeMap<Uuid, ActionData>,
    roots: &[Uuid],
    parents: bool,
) -> Result<()> {
    let mut colors = std::collections::BTreeMap::new();
    for root in roots {
        let mut stack = vec![(*root, false)];
        while let Some((id, exiting)) = stack.pop() {
            if exiting {
                colors.insert(id, true);
                continue;
            }
            match colors.get(&id) {
                Some(true) => continue,
                Some(false) => {
                    return Err(rejected(if parents {
                        "Action parent hierarchy contains a cycle"
                    } else {
                        "Action dependencies contain a cycle"
                    }));
                }
                None => {}
            }
            let data = resolve(app, overlay, id)?;
            let edges = if parents {
                data.parent.into_iter().collect()
            } else {
                data.dependencies.clone()
            };
            colors.insert(id, false);
            stack.push((id, true));
            stack.extend(edges.into_iter().rev().map(|target| (target, false)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod reference_tests;
