//! Initial Action fields over the existing exact proposal submission lifecycle.
use super::*;
use crate::review::action_fields::ActionFields;
use brn_workflow::{
    actions::ActionState,
    proposals::{ActionChange, MAX_PROPOSAL_CHANGES},
};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct InitialAction {
    pub id: Uuid,
    pub fields: ActionFields,
    pub sources: Vec<ProposalSource>,
}
impl InitialAction {
    pub fn pristine(&self) -> bool {
        self.fields.values.iter().all(String::is_empty)
            && self.fields.state == ActionState::Open
            && self.fields.priority.is_none()
            && self.sources.is_empty()
    }
}
impl DraftForm {
    pub fn new_action(follows_up: Option<Uuid>) -> Option<Self> {
        if follows_up.is_some_and(|id| id.is_nil()) {
            return None;
        }
        let mut form = Self::new(None)?;
        let mut fields = ActionFields {
            values: std::array::from_fn(|_| String::new()),
            state: ActionState::Open,
            priority: None,
        };
        fields.values[11] = follows_up.map(|id| id.to_string()).unwrap_or_default();
        form.action = Some(Box::new(InitialAction {
            id: Uuid::new_v4(),
            fields,
            sources: vec![],
        }));
        Some(form)
    }
    pub fn edit_action_fields(&mut self, fields: ActionFields) {
        let Some(action) = self.action.as_mut() else {
            return;
        };
        if action.fields == fields {
            return;
        }
        let Some(generation) = self.generation.checked_add(1) else {
            self.error = Some("Draft form generation exhausted; copy retained input.".into());
            return;
        };
        self.generation = generation;
        self.title = fields.values[0].clone();
        action.fields = fields;
    }
    pub fn edit_action_source_path(&mut self, path: String) {
        if self.action.is_none() || self.path == path {
            return;
        }
        let Some((generation, binding)) = self
            .generation
            .checked_add(1)
            .zip(self.binding_generation.checked_add(1))
        else {
            self.error = Some("Draft binding generation exhausted; copy retained input.".into());
            return;
        };
        self.generation = generation;
        self.binding_generation = binding;
        self.path = path;
        self.source_operation = None;
        self.source_error = None;
    }
    pub fn can_capture_action_source(&self) -> bool {
        self.action.as_ref().is_some_and(|action| {
            action.sources.len() < MAX_PROPOSAL_CHANGES
                || action
                    .sources
                    .iter()
                    .any(|source| source.source.path == self.path)
        })
    }
    pub fn retain_action_source(&mut self, capture: ProposalSource) -> bool {
        if capture.source.path != self.path
            || capture.validate().is_err()
            || !self.can_capture_action_source()
        {
            return false;
        }
        let Some(action) = self.action.as_mut() else {
            return false;
        };
        let existing = action
            .sources
            .iter()
            .position(|source| source.source.path == capture.source.path);
        if existing.is_some_and(|index| action.sources[index] == capture) {
            return true;
        }
        let Some(generation) = self.generation.checked_add(1) else {
            return false;
        };
        self.generation = generation;
        if let Some(index) = existing {
            action.sources[index] = capture;
        } else {
            action.sources.push(capture);
        }
        true
    }
    pub(super) fn action_request(
        &self,
        action: &InitialAction,
    ) -> brn_workflow::Result<DraftRequest> {
        let data = action
            .fields
            .data()
            .map_err(brn_workflow::WorkflowError::msg)?;
        let request = DraftRequest {
            inbox_source: None,
            id: self.id,
            group_id: None,
            session_id: self.session_id,
            title: self.title.clone(),
            changes: vec![],
            sources: action
                .sources
                .iter()
                .map(|capture| capture.source.clone())
                .collect(),
            action_changes: vec![ActionChange::Create {
                id: action.id,
                data,
            }],
        };
        request.validate()?;
        Ok(request)
    }
}
