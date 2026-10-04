//! One checked operational snapshot for dashboard clients; no stored projection.
use super::{
    ActionCursor, ActionListRequest, ActionRecord, ActionState, WorkStore, check_schema, date, read,
};
use crate::{Result, invalid};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionDashboardFilter {
    #[default]
    Active,
    Open,
    Waiting,
    Blocked,
    Completed,
    Overdue,
    FollowUp,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDashboardRequest {
    pub as_of: String,
    pub filter: ActionDashboardFilter,
    pub limit: usize,
    pub before: Option<ActionCursor>,
}
impl ActionDashboardRequest {
    pub fn validate(&self) -> Result<()> {
        date(&self.as_of)?;
        ActionListRequest {
            state: None,
            limit: self.limit,
            before: self.before,
        }
        .validate()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDashboardCounts {
    pub open: u64,
    pub waiting: u64,
    pub blocked: u64,
    pub completed: u64,
    pub overdue: u64,
    pub follow_up: u64,
}
impl ActionDashboardCounts {
    fn include(&mut self, record: &ActionRecord, overdue: bool, follow_up: bool) {
        match record.data.state {
            ActionState::Open => self.open += 1,
            ActionState::Waiting => self.waiting += 1,
            ActionState::Blocked => self.blocked += 1,
            ActionState::Completed => self.completed += 1,
        }
        self.overdue += u64::from(overdue);
        self.follow_up += u64::from(follow_up);
    }
    fn contains(&self, visible: &Self) -> bool {
        self.open >= visible.open
            && self.waiting >= visible.waiting
            && self.blocked >= visible.blocked
            && self.completed >= visible.completed
            && self.overdue >= visible.overdue
            && self.follow_up >= visible.follow_up
    }
    fn validate(&self) -> Result<()> {
        let active = self
            .open
            .checked_add(self.waiting)
            .and_then(|n| n.checked_add(self.blocked))
            .ok_or_else(|| invalid("Dashboard counts exceed their range"))?;
        active
            .checked_add(self.completed)
            .ok_or_else(|| invalid("Dashboard counts exceed their range"))?;
        if self.overdue > active || self.follow_up > active {
            return Err(invalid("Dashboard date counts exceed unfinished work"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDependencyObservation {
    pub id: Uuid,
    /// Absent retained target is explicit, never inferred as complete.
    pub state: Option<ActionState>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDashboardEntry {
    pub action: ActionRecord,
    pub overdue: bool,
    pub follow_up: bool,
    pub dependency_blocked: bool,
    pub dependencies: Vec<ActionDependencyObservation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDashboardPage {
    pub as_of: String,
    pub counts: ActionDashboardCounts,
    pub entries: Vec<ActionDashboardEntry>,
    pub next_before: Option<ActionCursor>,
}

fn signals(record: &ActionRecord, as_of: &str) -> (bool, bool) {
    let unfinished = record.data.state != ActionState::Completed;
    (
        unfinished && record.data.due_on.as_deref().is_some_and(|day| day < as_of),
        unfinished
            && record
                .data
                .follow_up_on
                .as_deref()
                .is_some_and(|day| day <= as_of),
    )
}
fn matches(
    filter: ActionDashboardFilter,
    record: &ActionRecord,
    overdue: bool,
    follow_up: bool,
) -> bool {
    match filter {
        ActionDashboardFilter::Active => record.data.state != ActionState::Completed,
        ActionDashboardFilter::Open => record.data.state == ActionState::Open,
        ActionDashboardFilter::Waiting => record.data.state == ActionState::Waiting,
        ActionDashboardFilter::Blocked => record.data.state == ActionState::Blocked,
        ActionDashboardFilter::Completed => record.data.state == ActionState::Completed,
        ActionDashboardFilter::Overdue => overdue,
        ActionDashboardFilter::FollowUp => follow_up,
        ActionDashboardFilter::All => true,
    }
}
fn cursor(record: &ActionRecord) -> ActionCursor {
    ActionCursor {
        created_at_ms: record.origin.created_at_ms,
        id: record.origin.id,
    }
}
fn order(cursor: ActionCursor) -> (u64, Uuid) {
    (cursor.created_at_ms, cursor.id)
}

impl ActionDashboardPage {
    /// Check transport shape and captured query binding without recomputing data
    /// authority in a client. Counts are global, while visible rows are bounded.
    pub fn validate(&self, request: &ActionDashboardRequest) -> Result<()> {
        request.validate()?;
        self.counts.validate()?;
        if self.as_of != request.as_of || self.entries.len() > request.limit {
            return Err(invalid(
                "Dashboard reply does not match its date/page request",
            ));
        }
        let mut previous = request.before;
        let mut visible = ActionDashboardCounts::default();
        for entry in &self.entries {
            entry.action.validate()?;
            let current = cursor(&entry.action);
            let (overdue, follow_up) = signals(&entry.action, &self.as_of);
            let dependency_blocked = entry.action.data.state != ActionState::Completed
                && entry
                    .dependencies
                    .iter()
                    .any(|item| item.state != Some(ActionState::Completed));
            if previous.is_some_and(|before| order(current) >= order(before))
                || !matches(request.filter, &entry.action, overdue, follow_up)
                || entry.overdue != overdue
                || entry.follow_up != follow_up
                || entry.dependency_blocked != dependency_blocked
                || entry.dependencies.len() != entry.action.data.dependencies.len()
                || entry
                    .dependencies
                    .iter()
                    .zip(&entry.action.data.dependencies)
                    .any(|(item, id)| item.id != *id)
            {
                return Err(invalid(
                    "Dashboard entry is inconsistent with its captured query or record",
                ));
            }
            visible.include(&entry.action, overdue, follow_up);
            previous = Some(current);
        }
        if !self.counts.contains(&visible)
            || self
                .next_before
                .is_some_and(|next| self.entries.len() != request.limit || Some(next) != previous)
        {
            return Err(invalid(
                "Dashboard counts/cursor do not contain the visible page",
            ));
        }
        Ok(())
    }
}

impl WorkStore {
    pub fn action_dashboard(
        &self,
        request: &ActionDashboardRequest,
    ) -> Result<ActionDashboardPage> {
        request.validate()?;
        let tx = self.conn.unchecked_transaction()?;
        check_schema(&tx)?;
        let ids = {
            let mut statement =
                tx.prepare("SELECT id FROM actions ORDER BY created_at_ms DESC,id DESC")?;
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let mut states = BTreeMap::new();
        let mut counts = ActionDashboardCounts::default();
        let mut selected = Vec::new();
        // Validate every retained row, including hidden/filter-excluded work.
        // Keep scalar target states and only limit+1 bounded complete records.
        for raw in ids {
            let id = crate::parse_id(raw.clone())?;
            if raw != id.to_string() {
                return Err(invalid("Invalid dashboard Action UUID"));
            }
            let record = read(&tx, id)?.ok_or_else(|| invalid("Dashboard Action disappeared"))?;
            let (overdue, follow_up) = signals(&record, &request.as_of);
            counts.include(&record, overdue, follow_up);
            states.insert(id, record.data.state);
            if selected.len() <= request.limit
                && request
                    .before
                    .is_none_or(|before| order(cursor(&record)) < order(before))
                && matches(request.filter, &record, overdue, follow_up)
            {
                selected.push(record);
            }
        }
        let more = selected.len() > request.limit;
        selected.truncate(request.limit);
        let next_before = more.then(|| cursor(selected.last().expect("positive dashboard limit")));
        let entries = selected
            .into_iter()
            .map(|action| {
                let (overdue, follow_up) = signals(&action, &request.as_of);
                let dependencies: Vec<_> = action
                    .data
                    .dependencies
                    .iter()
                    .map(|id| ActionDependencyObservation {
                        id: *id,
                        state: states.get(id).copied(),
                    })
                    .collect();
                let dependency_blocked = action.data.state != ActionState::Completed
                    && dependencies
                        .iter()
                        .any(|item| item.state != Some(ActionState::Completed));
                ActionDashboardEntry {
                    action,
                    overdue,
                    follow_up,
                    dependency_blocked,
                    dependencies,
                }
            })
            .collect();
        let page = ActionDashboardPage {
            as_of: request.as_of.clone(),
            counts,
            entries,
            next_before,
        };
        page.validate(request)?;
        tx.commit()?;
        Ok(page)
    }
}
