//! Local choices and recorded run metadata; workflow owns budget enforcement.
use super::{ActiveTurn, AiState, Pending};
use brn_workflow::{WorkBudget, app_worker::AppCommand};
use uuid::Uuid;

pub const ROUND_PRESETS: [u16; 4] = [4, 8, 16, 32];
pub const TIME_PRESETS: [u32; 4] = [60, 180, 300, 600];

pub fn budget_label(budget: Option<WorkBudget>) -> String {
    budget.map_or_else(
        || "Budget unavailable in recorded history".into(),
        |budget| {
            format!(
                "Budget: at most {} tool rounds · {} seconds",
                budget.max_tool_rounds, budget.timeout_seconds
            )
        },
    )
}

impl ActiveTurn {
    pub fn budget_label(&self) -> String {
        let ceiling = budget_label(self.request.budget());
        match self.budget_progress {
            Some((model_turns, tool_rounds)) => format!(
                "{ceiling} · {model_turns} model requests admitted · {tool_rounds} tool rounds admitted"
            ),
            None => format!("{ceiling} · awaiting admission"),
        }
    }
    pub fn status_label(&self) -> &'static str {
        if self.time_limit_reached {
            "Time limit reached; stopping and finalizing"
        } else if self.stopping {
            "Stopping (not finalized)"
        } else {
            "Streaming (provisional)"
        }
    }
}

impl AiState {
    pub fn can_select_work_budget(&self) -> bool {
        self.ready && self.active.is_none()
    }
    /// Native controls deliberately offer only the labelled bounded presets.
    pub fn select_work_rounds(&mut self, rounds: u16) -> bool {
        if !self.can_select_work_budget() || !ROUND_PRESETS.contains(&rounds) {
            return false;
        }
        self.work_budget.max_tool_rounds = rounds;
        true
    }
    pub fn select_work_seconds(&mut self, seconds: u32) -> bool {
        if !self.can_select_work_budget() || !TIME_PRESETS.contains(&seconds) {
            return false;
        }
        self.work_budget.timeout_seconds = seconds;
        true
    }
    pub fn selected_budget_label(&self) -> String {
        budget_label(Some(self.work_budget))
    }
    pub fn recorded_budget_label(&self, turn: Uuid) -> String {
        match self.run_budgets.get(&turn) {
            Some(budget) => budget_label(*budget),
            None => "Budget metadata pending".into(),
        }
    }
    pub(super) fn request_run_budget(&mut self, turn: Uuid) -> (Uuid, AppCommand) {
        self.command(
            Pending::RunBudget {
                turn,
                generation: self.generation,
            },
            AppCommand::RunBudget(turn),
        )
    }
}
