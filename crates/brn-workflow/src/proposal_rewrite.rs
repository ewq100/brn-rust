//! Owned AI suggestion generation. Results remain review work until exact approval.
use crate::{ErrorKind, Result, Selection, WorkflowError, chat_worker::provider_name};
pub use brn_ai::ReasoningEffort;
pub use brn_store::work::proposal_rewrite::{RewriteJob, RewriteSpec, RewriteStatus};
use brn_store::work::{
    actions::{ActionData, ActionPriority, ActionState},
    proposals::{ProposalEdit, ProposalStamp},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewriteRequest {
    pub id: Uuid,
    pub expected: ProposalStamp,
    #[serde(deserialize_with = "strict_selection")]
    pub selection: Selection,
    pub effort: ReasoningEffort,
    pub generation: u64,
}

fn strict_selection<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Selection, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Selected {
        provider: brn_ai::Provider,
        model: String,
    }
    let selected = Selected::deserialize(deserializer)?;
    Ok(Selection {
        provider: selected.provider,
        model: selected.model,
    })
}

impl RewriteRequest {
    pub fn validate(&self) -> Result<()> {
        if self.id.is_nil() || self.expected.id.is_nil() || self.expected.version == 0 {
            return Err(WorkflowError::typed(
                ErrorKind::ToolRejected,
                "invalid Rewrite identity",
            ));
        }
        self.selection.validate()?;
        Ok(())
    }

    pub(crate) fn spec(&self) -> RewriteSpec {
        RewriteSpec {
            id: self.id,
            expected: self.expected,
            provider: provider_name(self.selection.provider).into(),
            model: self.selection.model.clone(),
            effort: self.effort.as_str().into(),
        }
    }

    /// Checks immutable job bindings, excluding presentation generation. This
    /// does not grant admission, authorize a provider call or retry saved work.
    pub fn check_replay(&self, job: &RewriteJob) -> Result<()> {
        if job.spec != self.spec() {
            return Err(crate::chat_worker::conflict());
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum RewriteEvent {
    Started {
        id: Uuid,
        generation: u64,
        job: RewriteJob,
    },
    AlreadyRunning {
        id: Uuid,
        generation: u64,
        job: RewriteJob,
    },
    ToolStarted {
        id: Uuid,
        generation: u64,
        name: String,
    },
    Finished {
        id: Uuid,
        generation: u64,
        job: RewriteJob,
    },
    Rejected {
        id: Uuid,
        generation: u64,
        error: WorkflowError,
    },
    PersistenceFailed {
        id: Uuid,
        generation: u64,
        error: WorkflowError,
    },
}

impl RewriteEvent {
    pub fn id(&self) -> Uuid {
        match self {
            Self::Started { id, .. }
            | Self::AlreadyRunning { id, .. }
            | Self::ToolStarted { id, .. }
            | Self::Finished { id, .. }
            | Self::Rejected { id, .. }
            | Self::PersistenceFailed { id, .. } => *id,
        }
    }
    pub fn generation(&self) -> u64 {
        match self {
            Self::Started { generation, .. }
            | Self::AlreadyRunning { generation, .. }
            | Self::ToolStarted { generation, .. }
            | Self::Finished { generation, .. }
            | Self::Rejected { generation, .. }
            | Self::PersistenceFailed { generation, .. } => *generation,
        }
    }
    pub(crate) fn rejected(request: &RewriteRequest, error: WorkflowError) -> Self {
        Self::Rejected {
            id: request.id,
            generation: request.generation,
            error,
        }
    }
    pub(crate) fn replay(request: &RewriteRequest, job: RewriteJob) -> Self {
        if job.status == RewriteStatus::Running {
            Self::AlreadyRunning {
                id: request.id,
                generation: request.generation,
                job,
            }
        } else {
            Self::Finished {
                id: request.id,
                generation: request.generation,
                job,
            }
        }
    }
    pub(crate) fn started(request: &RewriteRequest, job: RewriteJob) -> Self {
        Self::Started {
            id: request.id,
            generation: request.generation,
            job,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RewriteText {
    title: String,
    texts: Vec<Option<String>>,
    #[serde(default)]
    action_data: Vec<CompleteAction>,
}

// Every field must be explicit, including null optional values: an omitted
// provider field is partial output, not permission to clear retained evidence.
#[derive(Deserialize)]
#[serde(remote = "ActionData", deny_unknown_fields)]
struct ActionResult {
    title: String,
    description: String,
    state: ActionState,
    #[serde(deserialize_with = "required_optional")]
    owner: Option<String>,
    #[serde(deserialize_with = "required_optional")]
    related_person: Option<Uuid>,
    #[serde(deserialize_with = "required_optional")]
    related_project: Option<Uuid>,
    sources: Vec<Uuid>,
    #[serde(deserialize_with = "required_optional")]
    thread: Option<Uuid>,
    #[serde(deserialize_with = "required_optional")]
    due_on: Option<String>,
    #[serde(deserialize_with = "required_optional")]
    follow_up_on: Option<String>,
    dependencies: Vec<Uuid>,
    #[serde(deserialize_with = "required_optional")]
    parent: Option<Uuid>,
    #[serde(deserialize_with = "required_optional")]
    follows_up: Option<Uuid>,
    #[serde(deserialize_with = "required_optional")]
    priority: Option<ActionPriority>,
}

fn required_optional<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

struct CompleteAction(ActionData);
impl<'de> Deserialize<'de> for CompleteAction {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        ActionResult::deserialize(deserializer).map(Self)
    }
}

pub(crate) fn decode(request: &RewriteRequest, text: &str) -> Result<ProposalEdit> {
    if text.len() > brn_ai::MAX_REWRITE_BYTES {
        return Err(WorkflowError::typed(
            ErrorKind::ToolRejected,
            "Rewrite result exceeds its limit",
        ));
    }
    let result: RewriteText = serde_json::from_str(text).map_err(|_| {
        WorkflowError::typed(
            ErrorKind::ToolRejected,
            "Rewrite did not return a complete typed result",
        )
    })?;
    Ok(ProposalEdit {
        action_data: result
            .action_data
            .into_iter()
            .map(|action| action.0)
            .collect(),
        expected: request.expected,
        title: result.title,
        texts: result.texts,
    })
}

#[cfg(test)]
pub(crate) type RewriteHook = std::sync::Arc<
    dyn Fn(
            RewriteRequest,
            String,
            std::sync::Arc<dyn brn_ai::ReadTools>,
            tokio_util::sync::CancellationToken,
            std::sync::Arc<dyn Fn(brn_ai::AiEvent) + Send + Sync>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = brn_ai::AiAnswer> + Send>>
        + Send
        + Sync,
>;

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> RewriteRequest {
        RewriteRequest {
            id: Uuid::new_v4(),
            expected: ProposalStamp {
                id: Uuid::new_v4(),
                version: 1,
            },
            selection: Selection {
                provider: brn_ai::Provider::Chatgpt,
                model: "gpt-6-luna".into(),
            },
            effort: ReasoningEffort::High,
            generation: 3,
        }
    }
    fn action_json() -> serde_json::Value {
        serde_json::json!({"title":"\u{feff}Full õ\r\n", "description":"Exact 🦀\r\n", "state":"waiting", "owner":null, "related_person":null, "related_project":null, "sources":[], "thread":null, "due_on":null, "follow_up_on":null, "dependencies":[], "parent":null, "follows_up":null, "priority":null})
    }

    #[test]
    fn rewrite_decoder_accepts_complete_actions_and_legacy_markdown_without_rebinding() {
        let request = request();
        let action: brn_store::work::actions::ActionData =
            serde_json::from_value(action_json()).unwrap();
        let text = serde_json::json!({"title":"Whole", "texts":["Full õ\r\n",null], "action_data":[action.clone(),action.clone()]}).to_string();
        let edit = decode(&request, &text).unwrap();
        assert_eq!(edit.expected, request.expected);
        assert_eq!(edit.action_data, vec![action.clone(), action]);
        assert_eq!(edit.texts, vec![Some("Full õ\r\n".into()), None]);
        let old = r#"{"title":"Old","texts":["exact\r\n"]}"#;
        assert_eq!(
            serde_json::to_string(&decode(&request, old).unwrap()).unwrap(),
            format!(
                "{{\"expected\":{},\"title\":\"Old\",\"texts\":[\"exact\\r\\n\"]}}",
                serde_json::to_string(&request.expected).unwrap()
            )
        );
        let empty = r#"{"title":"Old","texts":["exact\r\n"],"action_data":[]}"#;
        assert_eq!(
            decode(&request, old).unwrap(),
            decode(&request, empty).unwrap()
        );
    }

    #[test]
    fn rewrite_action_decoder_refuses_missing_unknown_duplicate_and_invalid_fields() {
        let request = request();
        for key in action_json().as_object().unwrap().keys() {
            let mut action = action_json();
            action.as_object_mut().unwrap().remove(key);
            let text =
                serde_json::json!({"title":"Incomplete", "texts":[], "action_data":[action]})
                    .to_string();
            assert!(decode(&request, &text).is_err(), "missing {key}");
        }
        let mut unknown = action_json();
        unknown["before"] = serde_json::json!({});
        let mut invalid = action_json();
        invalid["state"] = serde_json::json!("pending");
        for action in [unknown, invalid] {
            let text = serde_json::json!({"title":"Invalid", "texts":[], "action_data":[action]})
                .to_string();
            assert!(decode(&request, &text).is_err());
        }
        let duplicate = serde_json::to_string(&action_json()).unwrap().replacen(
            "\"owner\":null",
            "\"owner\":null,\"owner\":null",
            1,
        );
        assert!(
            decode(
                &request,
                &format!("{{\"title\":\"Duplicate\",\"texts\":[],\"action_data\":[{duplicate}]}}")
            )
            .is_err()
        );
        assert!(
            decode(
                &request,
                r#"{"title":"Null","texts":[],"action_data":null}"#
            )
            .is_err()
        );
    }

    #[test]
    fn nested_rewrite_selection_rejects_unknown_fields() {
        let request = serde_json::json!({
            "id": Uuid::new_v4(),
            "expected": {"id": Uuid::new_v4(), "version": 1},
            "selection": {"provider":"chatgpt", "model":"gpt-5.5", "unexpected":true},
            "effort":"high", "generation":0,
        });
        assert!(serde_json::from_value::<RewriteRequest>(request).is_err());
    }
}
