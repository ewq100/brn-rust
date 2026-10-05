//! Narrow proposal protocol; workflow owns all domain and approval decisions.
use crate::AiResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Complete, bounded protocol input; workflow checks domain and authority.
pub struct ActionProposalArgs {
    pub id: String,
    pub title: String,
    pub source_paths: Vec<String>,
    pub action_changes: Vec<Value>,
}

/// One review-only capability, separate from read tools and real Action writes.
pub trait ActionProposalTools: Send + Sync {
    /// Return a whole bounded review receipt; approval remains a separate operation.
    fn propose_actions(&self, args: ActionProposalArgs) -> AiResult<Value>;
}

use crate::{AiError, AiErrorKind, READ_ACTION_BYTES};
use rig::tool::{Tool, ToolContext};
use serde_json::json;
use std::sync::Arc;

/// Whole encoded proposal input limit, including JSON escaping.
pub const ACTION_PROPOSAL_BYTES: usize = 8 * 1024 * 1024;
impl ActionProposalArgs {
    /// Protocol bounds only. Workflow owns identities, full records and authority.
    pub fn validate(&self) -> AiResult<()> {
        if !(1..=64).contains(&self.id.len())
            || !(1..=512).contains(&self.title.len())
            || self.source_paths.len() > 64
            || self
                .source_paths
                .iter()
                .any(|p| !(1..=512).contains(&p.len()))
            || !(1..=20).contains(&self.action_changes.len())
            || serde_json::to_vec(self).map_err(|_| rejected())?.len() > ACTION_PROPOSAL_BYTES
        {
            return Err(rejected());
        }
        Ok(())
    }
}
fn rejected() -> AiError {
    AiError::new(AiErrorKind::ToolRejected)
}
pub(crate) struct ProposeActions(pub(crate) Arc<dyn ActionProposalTools>);

fn action_data_schema() -> Value {
    let nullable_uuid = json!({"type":["string","null"],"format":"uuid"});
    let uuid_array =
        json!({"type":"array","maxItems":64,"items":{"type":"string","format":"uuid"}});
    json!({"type":"object","additionalProperties":false,"properties":{
        "title":{"type":"string","minLength":1,"maxLength":512},
        "description":{"type":"string","maxLength":65536},
        "state":{"type":"string","enum":["open","waiting","blocked"]},
        "owner":{"type":["string","null"],"maxLength":512},
        "related_person":nullable_uuid,"related_project":nullable_uuid,
        "sources":uuid_array,"thread":nullable_uuid,
        "due_on":{"type":["string","null"],"format":"date"},
        "follow_up_on":{"type":["string","null"],"format":"date"},
        "dependencies":uuid_array,"parent":nullable_uuid,"follows_up":nullable_uuid,
        "priority":{"type":["string","null"],"enum":[null,"low","normal","high"]}
    },"required":["title","description","state","owner","related_person","related_project","sources","thread","due_on","follow_up_on","dependencies","parent","follows_up","priority"]})
}
fn record_schema() -> Value {
    let data = action_data_schema();
    json!({"type":"object","additionalProperties":false,"properties":{
        "origin":{"type":"object","additionalProperties":false,"properties":{
            "id":{"type":"string","format":"uuid"},
            "proposal":{"type":"object","additionalProperties":false,"properties":{"id":{"type":"string","format":"uuid"},"version":{"type":"integer","minimum":1}},"required":["id","version"]},
            "data":data,"created_at_ms":{"type":"integer","minimum":0}
        },"required":["id","proposal","data","created_at_ms"]},
        "version":{"type":"integer","minimum":1},"data":data,
        "updated_at_ms":{"type":"integer","minimum":0},
        "waiting_since_ms":{"type":["integer","null"],"minimum":0},
        "completed_at_ms":{"type":["integer","null"],"minimum":0}
    },"required":["origin","version","data","updated_at_ms","waiting_since_ms","completed_at_ms"]})
}
impl Tool for ProposeActions {
    const NAME: &'static str = "propose_actions";
    type Args = ActionProposalArgs;
    type Output = Value;
    type Error = AiError;
    fn description(&self) -> String {
        "Create an Action-only review proposal, never real Actions or approval. Supply stable proposal/member UUIDs, all14 candidate fields including explicit nulls, and the entire fresh read_action record for Replace. Use new related Actions for completed work. Explicit ordered source_paths capture full saved evidence, including historical evidence when intended; never derive a proof from a truncated read. Retry only identical original input/UUID. Returns an exact review receipt; human review and separate exact approval are required.".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","additionalProperties":false,"properties":{
            "id":{"type":"string","format":"uuid","minLength":1,"maxLength":64},
            "title":{"type":"string","minLength":1,"maxLength":512},
            "source_paths":{"type":"array","maxItems":64,"items":{"type":"string","minLength":1,"maxLength":512}},
            "action_changes":{"type":"array","minItems":1,"maxItems":20,"items":{"anyOf":[
                {"type":"object","additionalProperties":false,"properties":{"kind":{"type":"string","enum":["create"]},"id":{"type":"string","format":"uuid"},"data":action_data_schema()},"required":["kind","id","data"]},
                {"type":"object","additionalProperties":false,"properties":{"kind":{"type":"string","enum":["replace"]},"before":record_schema(),"data":action_data_schema()},"required":["kind","before","data"]}
            ]}}
        },"required":["id","title","source_paths","action_changes"]})
    }
    async fn call(&self, _: &mut ToolContext, args: ActionProposalArgs) -> AiResult<Value> {
        args.validate()?;
        let proposals = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let receipt = proposals.propose_actions(args)?;
            if serde_json::to_vec(&receipt).map_err(|_| rejected())?.len() > READ_ACTION_BYTES {
                return Err(rejected());
            }
            Ok(receipt)
        })
        .await
        .map_err(|_| AiError::new(AiErrorKind::Other))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Backend {
        calls: AtomicUsize,
        receipt: Value,
    }
    impl ActionProposalTools for Backend {
        fn propose_actions(&self, _: ActionProposalArgs) -> AiResult<Value> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.receipt.clone())
        }
    }
    fn args() -> ActionProposalArgs {
        ActionProposalArgs {
            id: "a".into(),
            title: "Exact õ\r\n".into(),
            source_paths: vec![],
            action_changes: vec![json!({})],
        }
    }
    #[test]
    fn outer_protocol_is_closed_and_requires_all_four_explicit_fields() {
        let whole = serde_json::to_value(args()).unwrap();
        for field in ["id", "title", "source_paths", "action_changes"] {
            let mut partial = whole.clone();
            partial.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<ActionProposalArgs>(partial).is_err());
        }
        for field in [
            "session_id",
            "group_id",
            "conversation",
            "provider",
            "approve",
        ] {
            let mut unknown = whole.clone();
            unknown[field] = json!("model cannot bind this");
            assert!(serde_json::from_value::<ActionProposalArgs>(unknown).is_err());
        }
    }
    #[tokio::test]
    async fn bounded_input_refuses_before_dispatch_using_utf8_and_whole_json_bytes() {
        let backend = Arc::new(Backend {
            calls: AtomicUsize::new(0),
            receipt: json!({"stamp":"ok"}),
        });
        let tool = ProposeActions(backend.clone());
        let mut context = ToolContext::default();
        let mut bad = vec![];
        let mut input = args();
        input.id = "x".repeat(65);
        bad.push(input);
        let mut input = args();
        input.title = "õ".repeat(257);
        bad.push(input);
        let mut input = args();
        input.source_paths = vec!["p.md".into(); 65];
        bad.push(input);
        let mut input = args();
        input.source_paths = vec!["õ".repeat(257)];
        bad.push(input);
        let mut input = args();
        input.source_paths = vec!["".into()];
        bad.push(input);
        let mut input = args();
        input.action_changes = vec![];
        bad.push(input);
        let mut input = args();
        input.action_changes = vec![json!({}); 21];
        bad.push(input);
        let mut input = args();
        input.action_changes = vec![json!("\u{1}".repeat(ACTION_PROPOSAL_BYTES / 6))];
        bad.push(input);
        for input in bad {
            assert_eq!(
                tool.call(&mut context, input).await.unwrap_err().kind,
                AiErrorKind::ToolRejected
            );
        }
        assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
        let mut exact = args();
        exact.action_changes = vec![json!("")];
        let overhead = serde_json::to_vec(&exact).unwrap().len();
        exact.action_changes[0] = json!("x".repeat(ACTION_PROPOSAL_BYTES - overhead));
        assert_eq!(
            serde_json::to_vec(&exact).unwrap().len(),
            ACTION_PROPOSAL_BYTES
        );
        assert!(tool.call(&mut context, exact.clone()).await.is_ok());
        exact.action_changes[0] = json!(format!("{}x", exact.action_changes[0].as_str().unwrap()));
        assert_eq!(
            tool.call(&mut context, exact).await.unwrap_err().kind,
            AiErrorKind::ToolRejected
        );
        assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn complete_receipt_limit_refuses_oversize_without_truncation() {
        for accepted in [true, false] {
            let receipt = json!({"stamp":"\u{1}".repeat(READ_ACTION_BYTES/6)});
            // Use one exact whole-JSON boundary, then one additional byte.
            let overhead = serde_json::to_vec(&json!({"stamp":""})).unwrap().len();
            let receipt = if accepted {
                json!({"stamp":"x".repeat(READ_ACTION_BYTES-overhead)})
            } else {
                receipt
            };
            let backend = Arc::new(Backend {
                calls: AtomicUsize::new(0),
                receipt: receipt.clone(),
            });
            let result = ProposeActions(backend.clone())
                .call(&mut ToolContext::default(), args())
                .await;
            if accepted {
                assert_eq!(result.unwrap(), receipt);
            } else {
                assert_eq!(result.unwrap_err().kind, AiErrorKind::ToolRejected);
            }
            assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
        }
    }
}
