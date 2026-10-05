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

/// Review-only capabilities, separate from read tools and authoritative writes.
pub trait ProposalTools: Send + Sync {
    /// Return a whole bounded review receipt; approval remains a separate operation.
    fn propose_actions(&self, args: ActionProposalArgs) -> AiResult<Value>;

    /// Record tentative unresolved evidence only; workflow owns saved proofs and identity.
    fn report_conflict(&self, _: ConflictArgs) -> AiResult<Value> {
        Err(rejected())
    }

    /// Opt in only for an application-owned, explicitly selected source job.
    fn knowledge_enabled(&self) -> bool {
        false
    }

    /// Return a bounded review receipt; existing backends safely refuse knowledge.
    fn propose_knowledge(&self, _: KnowledgeProposalArgs) -> AiResult<Value> {
        Err(rejected())
    }
}

/// One complete Current knowledge consequence; workflow owns source and citation rules.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeProposalArgs {
    /// Optional saved Current predecessor; workflow captures it and protects History.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    pub id: String,
    pub title: String,
    pub path: String,
    pub note_id: String,
    pub text: String,
    pub quotes: Vec<KnowledgeQuoteArgs>,
    /// Additional explicit evidence paths in caller order; selected Source is first automatically.
    #[serde(default)]
    pub source_paths: Vec<String>,
}

/// Exact saved body wording; workflow resolves its unique or selected occurrence.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeQuoteArgs {
    pub quote: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurrence: Option<usize>,
}

/// Exact saved body quotation; workflow verifies bytes against full captured evidence.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictQuote {
    pub quote: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurrence: Option<usize>,
}

/// Tentative unresolved finding, with no knowledge, Action or approval authority.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictArgs {
    pub title: String,
    pub summary: String,
    pub source_quote: ConflictQuote,
    pub other_path: String,
    pub other_quote: ConflictQuote,
}

/// Complete encoded conflict report limit, including JSON escaping.
pub const CONFLICT_REPORT_BYTES: usize = 512 * 1024;
impl ConflictArgs {
    /// Protocol bounds only; workflow owns UUIDs, paths, body boundaries and proofs.
    pub fn validate(&self) -> AiResult<()> {
        if self.title.trim().is_empty()
            || self.title.len() > 512
            || self.summary.trim().is_empty()
            || self.summary.len() > 16 * 1024
            || !(1..=512).contains(&self.other_path.len())
            || [&self.source_quote, &self.other_quote]
                .iter()
                .any(|q| !(1..=16 * 1024).contains(&q.quote.len()))
            || serde_json::to_vec(self).map_err(|_| rejected())?.len() > CONFLICT_REPORT_BYTES
        {
            return Err(rejected());
        }
        Ok(())
    }
}

use crate::{AiError, AiErrorKind, READ_ACTION_BYTES};
use rig::tool::{Tool, ToolContext};
use serde_json::json;
use std::sync::Arc;

/// Whole encoded proposal input limit, including JSON escaping.
pub const ACTION_PROPOSAL_BYTES: usize = 8 * 1024 * 1024;
/// Whole encoded Knowledge proposal input limit, including JSON escaping.
pub const KNOWLEDGE_PROPOSAL_BYTES: usize = 8 * 1024 * 1024;
impl KnowledgeProposalArgs {
    /// Protocol bounds only. Workflow owns UUIDs, paths, source identity and citations.
    pub fn validate(&self) -> AiResult<()> {
        if !(1..=64).contains(&self.id.len())
            || !(1..=512).contains(&self.title.len())
            || !(1..=512).contains(&self.path.len())
            || !(1..=64).contains(&self.note_id.len())
            || !(1..=1024 * 1024).contains(&self.text.len())
            || !(1..=32).contains(&self.quotes.len())
            || self
                .supersedes
                .as_ref()
                .is_some_and(|path| !(1..=512).contains(&path.len()))
            || self.source_paths.len() > if self.supersedes.is_some() { 62 } else { 63 }
            || self
                .source_paths
                .iter()
                .any(|path| !(1..=512).contains(&path.len()))
            || self
                .quotes
                .iter()
                .any(|quote| !(1..=16 * 1024).contains(&quote.quote.len()))
            || serde_json::to_vec(self).map_err(|_| rejected())?.len() > KNOWLEDGE_PROPOSAL_BYTES
        {
            return Err(rejected());
        }
        Ok(())
    }
}
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

// Rig hides Tool::call errors. Only safe quotation selectors get correction
// feedback as data; callbacks still return their original typed application error.
fn quote_feedback(result: AiResult<Value>) -> AiResult<Value> {
    match result {
        Err(error)
            if matches!(
                error.kind,
                AiErrorKind::QuoteNotFound
                    | AiErrorKind::QuoteAmbiguous
                    | AiErrorKind::QuoteOccurrenceInvalid
            ) =>
        {
            Ok(json!({"error":{"kind":error.kind,"message":error.to_string()}}))
        }
        other => other,
    }
}
pub(crate) struct ProposeActions(pub(crate) Arc<dyn ProposalTools>);
pub(crate) struct ProposeKnowledge(pub(crate) Arc<dyn ProposalTools>);
pub(crate) struct ReportConflict(pub(crate) Arc<dyn ProposalTools>);

impl Tool for ReportConflict {
    const NAME: &'static str = "report_conflict";
    type Args = ConflictArgs;
    type Output = Value;
    type Error = AiError;
    fn description(&self) -> String {
        "Report a tentative unresolved finding between the explicitly selected approved Inbox Source and one other saved Current knowledge or Source note. Supply two exact opposing saved body quotations, title, summary and other_path. Each quote may specify an optional 1-based occurrence in the saved body; omit it only for unique wording. Workflow resolves exact byte ranges, captures and verifies full saved proofs, and assigns the finding UUID returned in the receipt. Do not choose a winner. This creates no knowledge effects, real Actions or deletion authority; separate exact proposals and human approval still govern those. Retry only identical original input; changed input creates a separate finding draft. Whole receipts are bounded and never clipped.".into()
    }
    fn parameters(&self) -> Value {
        let quote = json!({"type":"object","additionalProperties":false,"properties":{
            "quote":{"type":"string","minLength":1,"maxLength":16384},
            "occurrence":{"type":["integer","null"],"minimum":1,"maximum":1048576}
        },"required":["quote"]});
        json!({"type":"object","additionalProperties":false,"properties":{
            "title":{"type":"string","minLength":1,"maxLength":512},
            "summary":{"type":"string","minLength":1,"maxLength":16384},
            "source_quote":quote,"other_path":{"type":"string","minLength":1,"maxLength":512},
            "other_quote":quote
        },"required":["title","summary","source_quote","other_path","other_quote"]})
    }
    async fn call(&self, _: &mut ToolContext, args: ConflictArgs) -> AiResult<Value> {
        args.validate()?;
        let proposals = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let receipt = quote_feedback(proposals.report_conflict(args))?;
            if serde_json::to_vec(&receipt).map_err(|_| rejected())?.len() > READ_ACTION_BYTES {
                return Err(rejected());
            }
            Ok(receipt)
        })
        .await
        .map_err(|_| AiError::new(AiErrorKind::Other))?
    }
}

impl Tool for ProposeKnowledge {
    const NAME: &'static str = "propose_knowledge";
    type Args = KnowledgeProposalArgs;
    type Output = Value;
    type Error = AiError;
    fn description(&self) -> String {
        "Create one independent current Knowledge review draft from the explicitly selected approved Inbox Source. Supply complete candidate Markdown, stable proposal and note UUIDs, a relative destination path, exact saved body quotations, and ordered additional source_paths. Each quote may specify an optional 1-based occurrence in the saved body; omit it only for unique wording. Workflow resolves exact byte ranges. The selected Inbox Source is automatically the mandatory first proof; do not include it again. Stable brn://note/UUID relationships require exact named target evidence. Read tools default to Current; explicitly named extra Source or History paths are evidence, never truth or deletion approval. Optional supersedes names one saved Current knowledge path: workflow captures it as the second proof, adds a Previous version link and a protected History member to this same exact proposal. Do not repeat that path in source_paths or use a Source/History predecessor. Workflow captures complete saved proofs and adds exact saved citations. This tool never approves or writes knowledge. Retry only identical original input and UUIDs; human review and separate exact approval are required.".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","additionalProperties":false,"properties":{
            "id":{"type":"string","format":"uuid","minLength":1,"maxLength":64},
            "title":{"type":"string","minLength":1,"maxLength":512},
            "path":{"type":"string","minLength":1,"maxLength":512},
            "note_id":{"type":"string","format":"uuid","minLength":1,"maxLength":64},
            "text":{"type":"string","minLength":1,"maxLength":1048576},
            "supersedes":{"type":["string","null"],"minLength":1,"maxLength":512},
            "source_paths":{"type":"array","maxItems":63,"items":{"type":"string","minLength":1,"maxLength":512}},
            "quotes":{"type":"array","minItems":1,"maxItems":32,"items":{
                "type":"object","additionalProperties":false,"properties":{
                    "quote":{"type":"string","minLength":1,"maxLength":16384},
                    "occurrence":{"type":["integer","null"],"minimum":1,"maximum":1048576}
                },"required":["quote"]
            }}
        },"required":["id","title","path","note_id","text","quotes","source_paths","supersedes"]})
    }
    async fn call(&self, _: &mut ToolContext, args: KnowledgeProposalArgs) -> AiResult<Value> {
        args.validate()?;
        let proposals = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let receipt = quote_feedback(proposals.propose_knowledge(args))?;
            if serde_json::to_vec(&receipt).map_err(|_| rejected())?.len() > READ_ACTION_BYTES {
                return Err(rejected());
            }
            Ok(receipt)
        })
        .await
        .map_err(|_| AiError::new(AiErrorKind::Other))?
    }
}

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

    #[test]
    fn quote_feedback_is_allowlisted_and_never_serializes_error_metadata() {
        for kind in [
            AiErrorKind::QuoteNotFound,
            AiErrorKind::QuoteAmbiguous,
            AiErrorKind::QuoteOccurrenceInvalid,
        ] {
            let error = AiError {
                kind,
                retry_after_seconds: Some(u64::MAX),
            };
            let result = quote_feedback(Err(error.clone())).unwrap();
            assert_eq!(
                result,
                json!({"error":{"kind":kind,"message":error.to_string()}})
            );
            let bytes = serde_json::to_vec(&result).unwrap();
            assert!(bytes.len() <= READ_ACTION_BYTES);
            assert!(!result.to_string().contains("retry_after_seconds"));
        }
        for kind in [
            AiErrorKind::ReconnectNeeded,
            AiErrorKind::CodeExpired,
            AiErrorKind::RateLimited,
            AiErrorKind::Network,
            AiErrorKind::ModelRefused,
            AiErrorKind::InvalidToolUse,
            AiErrorKind::ToolLimitReached,
            AiErrorKind::UnsafeCredentials,
            AiErrorKind::ToolRejected,
            AiErrorKind::IndexStale,
            AiErrorKind::Storage,
            AiErrorKind::Other,
        ] {
            let error = AiError {
                kind,
                retry_after_seconds: Some(42),
            };
            assert_eq!(quote_feedback(Err(error.clone())), Err(error));
        }
        let receipt = json!({"stamp":{"id":"synthetic draft","version":1}});
        assert_eq!(quote_feedback(Ok(receipt.clone())), Ok(receipt));
    }

    struct QuoteRefusal;
    impl ProposalTools for QuoteRefusal {
        fn propose_actions(&self, _: ActionProposalArgs) -> AiResult<Value> {
            Err(AiError::new(AiErrorKind::QuoteAmbiguous))
        }
    }

    #[tokio::test]
    async fn quote_errors_on_action_tools_keep_the_existing_error_boundary() {
        assert_eq!(
            ProposeActions(Arc::new(QuoteRefusal))
                .call(&mut ToolContext::default(), args())
                .await
                .unwrap_err()
                .kind,
            AiErrorKind::QuoteAmbiguous
        );
    }
    struct Backend {
        calls: AtomicUsize,
        receipt: Value,
    }
    impl ProposalTools for Backend {
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

    fn knowledge_args() -> KnowledgeProposalArgs {
        KnowledgeProposalArgs {
            supersedes: None,
            id: "workflow parses this UUID".into(),
            title: "Exact õ\r\n".into(),
            path: "workflow checks the destination".into(),
            note_id: "workflow checks stable identity".into(),
            text: "\u{feff}Whole candidate 🦀\r\n".into(),
            quotes: vec![KnowledgeQuoteArgs {
                quote: "õ🦀\r\n".into(),
                occurrence: None,
            }],
            source_paths: vec!["workflow interprets this path".into()],
        }
    }

    struct KnowledgeBackend {
        calls: AtomicUsize,
        receipt: Value,
    }
    impl ProposalTools for KnowledgeBackend {
        fn propose_actions(&self, _: ActionProposalArgs) -> AiResult<Value> {
            panic!("unexpected Action proposal")
        }
        fn knowledge_enabled(&self) -> bool {
            true
        }
        fn propose_knowledge(&self, _: KnowledgeProposalArgs) -> AiResult<Value> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.receipt.clone())
        }
    }

    #[test]
    fn knowledge_protocol_is_closed_and_requires_every_candidate_and_quote_field() {
        let whole = serde_json::to_value(knowledge_args()).unwrap();
        for field in ["id", "title", "path", "note_id", "text", "quotes"] {
            let mut partial = whole.clone();
            partial.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<KnowledgeProposalArgs>(partial).is_err());
        }
        let mut legacy = whole.clone();
        legacy.as_object_mut().unwrap().remove("source_paths");
        let legacy = serde_json::from_value::<KnowledgeProposalArgs>(legacy).unwrap();
        assert!(legacy.source_paths.is_empty());
        assert!(legacy.validate().is_ok());
        for field in ["source_path", "session_id", "approve", "citations"] {
            let mut unknown = whole.clone();
            unknown[field] = json!("workflow owns this");
            assert!(serde_json::from_value::<KnowledgeProposalArgs>(unknown).is_err());
        }
        let mut partial = whole.clone();
        partial["quotes"][0]
            .as_object_mut()
            .unwrap()
            .remove("quote");
        assert!(serde_json::from_value::<KnowledgeProposalArgs>(partial).is_err());
        for field in ["text", "start_byte", "end_byte"] {
            let mut unknown = whole.clone();
            unknown["quotes"][0][field] = json!(0);
            assert!(serde_json::from_value::<KnowledgeProposalArgs>(unknown).is_err());
        }
        for value in [json!(-1), json!(1.5), json!("1")] {
            let mut malformed = whole.clone();
            malformed["quotes"][0]["occurrence"] = value;
            assert!(serde_json::from_value::<KnowledgeProposalArgs>(malformed).is_err());
        }
        let mut explicit_null = whole.clone();
        explicit_null["quotes"][0]["occurrence"] = Value::Null;
        let parsed = serde_json::from_value::<KnowledgeProposalArgs>(explicit_null).unwrap();
        assert!(parsed.quotes[0].occurrence.is_none());
        assert_eq!(serde_json::to_value(parsed).unwrap(), whole);
    }

    #[tokio::test]
    async fn knowledge_bounds_refuse_before_dispatch_and_leave_domain_rules_to_workflow() {
        let backend = Arc::new(KnowledgeBackend {
            calls: AtomicUsize::new(0),
            receipt: json!({"stamp":"ok"}),
        });
        let tool = ProposeKnowledge(backend.clone());
        let mutations: [fn(&mut KnowledgeProposalArgs); 22] = [
            |v| v.id.clear(),
            |v| v.id = "x".repeat(65),
            |v| v.title.clear(),
            |v| v.title = "õ".repeat(257),
            |v| v.path.clear(),
            |v| v.path = "õ".repeat(257),
            |v| v.note_id.clear(),
            |v| v.note_id = "õ".repeat(33),
            |v| v.text.clear(),
            |v| v.text = "x".repeat(1024 * 1024 + 1),
            |v| v.text = "õ".repeat(512 * 1024 + 1),
            |v| v.quotes.clear(),
            |v| v.quotes = vec![v.quotes[0].clone(); 33],
            |v| v.quotes[0].quote.clear(),
            |v| v.quotes[0].quote = "x".repeat(16 * 1024 + 1),
            |v| v.quotes[0].quote = "õ".repeat(8193),
            |v| v.source_paths = vec!["explicit.md".into(); 64],
            |v| v.source_paths = vec![String::new()],
            |v| v.source_paths = vec!["x".repeat(513)],
            |v| v.source_paths = vec!["õ".repeat(257)],
            |v| {
                v.text = "\u{1}".repeat(KNOWLEDGE_PROPOSAL_BYTES / 6 + 1);
                assert!(serde_json::to_vec(v).unwrap().len() > KNOWLEDGE_PROPOSAL_BYTES);
            },
            |v| {
                // Every individual text fits, but complete JSON escaping does not.
                v.text = "\u{1}".repeat(1024 * 1024);
                v.quotes = vec![
                    KnowledgeQuoteArgs {
                        quote: "\u{1}".repeat(16 * 1024),
                        occurrence: None
                    };
                    32
                ];
                assert!(serde_json::to_vec(v).unwrap().len() > KNOWLEDGE_PROPOSAL_BYTES);
            },
        ];
        for mutation in mutations {
            let mut input = knowledge_args();
            mutation(&mut input);
            assert_eq!(
                tool.call(&mut ToolContext::default(), input)
                    .await
                    .unwrap_err()
                    .kind,
                AiErrorKind::ToolRejected
            );
        }
        assert_eq!(backend.calls.load(Ordering::SeqCst), 0);

        // Non-UUID strings, destination semantics and occurrence resolution
        // are deliberately delegated. Only protocol byte bounds live here.
        let input = knowledge_args();
        assert!(tool.call(&mut ToolContext::default(), input).await.is_ok());
        let mut maximum = knowledge_args();
        maximum.id = "x".repeat(64);
        maximum.title = "õ".repeat(256);
        maximum.path = "õ".repeat(256);
        maximum.note_id = "x".repeat(64);
        maximum.source_paths = vec!["\u{1}".repeat(512); 63];
        maximum.text = "\u{1}".repeat(1024 * 1024);
        maximum.quotes = vec![
            KnowledgeQuoteArgs {
                quote: "x".repeat(16 * 1024),
                occurrence: Some(1024 * 1024),
            };
            32
        ];
        let encoded = serde_json::to_vec(&maximum).unwrap();
        assert!(encoded.len() > maximum.text.len());
        assert!(encoded.len() < KNOWLEDGE_PROPOSAL_BYTES);
        assert!(
            tool.call(&mut ToolContext::default(), maximum)
                .await
                .is_ok()
        );
        assert_eq!(backend.calls.load(Ordering::SeqCst), 2);
        for occurrence in [0, usize::MAX] {
            let mut input = knowledge_args();
            input.quotes[0].occurrence = Some(occurrence);
            assert!(tool.call(&mut ToolContext::default(), input).await.is_ok());
        }
        assert_eq!(backend.calls.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn quote_tool_schemas_leave_nullable_occurrence_optional_and_conflict_ids_owned() {
        let backend = Arc::new(KnowledgeBackend {
            calls: AtomicUsize::new(0),
            receipt: json!({}),
        });
        let knowledge = ProposeKnowledge(backend.clone()).parameters();
        let conflict = ReportConflict(backend).parameters();
        let expected = json!({"type":"object","additionalProperties":false,"properties":{
            "quote":{"type":"string","minLength":1,"maxLength":16384},
            "occurrence":{"type":["integer","null"],"minimum":1,"maximum":1048576}
        },"required":["quote"]});
        assert_eq!(knowledge["properties"]["quotes"]["items"], expected);
        for side in ["source_quote", "other_quote"] {
            assert_eq!(conflict["properties"][side], expected);
        }
        assert!(conflict["properties"].get("id").is_none());
        assert_eq!(conflict["required"].as_array().unwrap().len(), 5);
    }

    #[tokio::test]
    async fn knowledge_defaults_refuse_and_whole_receipts_are_bounded_without_truncation() {
        let ordinary = Arc::new(Backend {
            calls: AtomicUsize::new(0),
            receipt: json!({"stamp":"ordinary"}),
        });
        assert!(!ordinary.knowledge_enabled());
        assert_eq!(
            ProposeKnowledge(ordinary.clone())
                .call(&mut ToolContext::default(), knowledge_args())
                .await
                .unwrap_err()
                .kind,
            AiErrorKind::ToolRejected
        );
        assert_eq!(ordinary.calls.load(Ordering::SeqCst), 0);
        let overhead = serde_json::to_vec(&json!({"stamp":""})).unwrap().len();
        for extra in [0, 1] {
            let receipt = json!({"stamp":"x".repeat(READ_ACTION_BYTES - overhead + extra)});
            let backend = Arc::new(KnowledgeBackend {
                calls: AtomicUsize::new(0),
                receipt: receipt.clone(),
            });
            let result = ProposeKnowledge(backend.clone())
                .call(&mut ToolContext::default(), knowledge_args())
                .await;
            if extra == 0 {
                assert_eq!(result.unwrap(), receipt);
            } else {
                assert_eq!(result.unwrap_err().kind, AiErrorKind::ToolRejected);
            }
            assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
        }
    }
}
