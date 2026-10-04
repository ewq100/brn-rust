//! Shared dashboard meaning: local default date, explicit paging, checked state.
use crate::{ErrorKind, Result, WorkflowError, actions::ActionCursor, app::App};
use brn_store::work::actions::dashboard::ActionDashboardRequest as StoreRequest;
pub use brn_store::work::actions::dashboard::{
    ActionDashboardCounts as DashboardCounts, ActionDashboardEntry as DashboardEntry,
    ActionDashboardFilter as DashboardFilter, ActionDashboardPage as DashboardPage,
    ActionDependencyObservation,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DashboardRequest {
    /// Omitted first-page date is resolved by BRN to OS-local civil today.
    /// Continuations require the returned date, so midnight cannot change scope.
    pub as_of: Option<String>,
    pub filter: DashboardFilter,
    pub limit: usize,
    pub before: Option<ActionCursor>,
}
impl Default for DashboardRequest {
    fn default() -> Self {
        Self {
            as_of: None,
            filter: DashboardFilter::Active,
            limit: 25,
            before: None,
        }
    }
}
fn rejected(error: impl std::fmt::Display) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ToolRejected, error.to_string())
}
impl DashboardRequest {
    fn for_date(&self, as_of: String) -> StoreRequest {
        StoreRequest {
            as_of,
            filter: self.filter,
            limit: self.limit,
            before: self.before,
        }
    }
    /// Pure validation for clients before opening any workspace authority.
    pub fn validate(&self) -> Result<()> {
        if self.before.is_some() && self.as_of.is_none() {
            return Err(rejected(
                "Dashboard continuation needs the returned explicit --as-of date",
            ));
        }
        // The constant validates only shape when the application must resolve
        // today. Validation itself never reads a clock, workspace or provider.
        self.for_date(self.as_of.clone().unwrap_or_else(|| "2000-01-01".into()))
            .validate()
            .map_err(rejected)
    }
    pub fn validate_page(&self, page: &DashboardPage) -> Result<()> {
        self.validate()?;
        if self.as_of.as_ref().is_some_and(|date| date != &page.as_of) {
            return Err(rejected("Dashboard reply changed the captured date"));
        }
        page.validate(&self.for_date(page.as_of.clone()))
            .map_err(rejected)
    }
}
impl App {
    /// Read one operational snapshot without requiring a vault or an AI account.
    pub fn action_dashboard(&self, request: &DashboardRequest) -> Result<DashboardPage> {
        request.validate()?;
        self.require_current_evidence()?;
        let as_of = request
            .as_of
            .clone()
            .unwrap_or_else(|| chrono::Local::now().date_naive().to_string());
        Ok(self.store.action_dashboard(&request.for_date(as_of))?)
    }
}
