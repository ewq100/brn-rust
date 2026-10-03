//! Owned AI suggestion generation. Results remain review work until exact approval.
use crate::{ErrorKind, Result, Selection, WorkflowError, chat_worker::provider_name};
pub use brn_ai::ReasoningEffort;
pub use brn_store::work::proposal_rewrite::{RewriteJob, RewriteSpec, RewriteStatus};
use brn_store::work::proposals::{ProposalEdit, ProposalRecord, ProposalStamp};
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

    pub(crate) fn check_replay(&self, job: &RewriteJob) -> Result<()> {
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

pub(crate) fn prompt(record: &ProposalRecord) -> Result<String> {
    // Serialize the complete snapshot, including unresolved exact quote anchors.
    // Never truncate a member/comment or silently drop evidence to fit a model.
    let prompt = serde_json::to_string(record)
        .map_err(|_| WorkflowError::typed(ErrorKind::ToolRejected, "could not capture Rewrite"))?;
    if prompt.len() > brn_ai::MAX_REWRITE_BYTES {
        return Err(WorkflowError::typed(
            ErrorKind::ToolRejected,
            "full Rewrite capture exceeds its limit",
        ));
    }
    Ok(prompt)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RewriteText {
    title: String,
    texts: Vec<Option<String>>,
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
