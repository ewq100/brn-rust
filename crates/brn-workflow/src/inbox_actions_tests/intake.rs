//! Integrated paired synthetic stories: retained EML -> private investigation -> review -> restart.
//! Hooks prove lifecycle/authority, not live-model semantic quality.
use super::*;
use crate::proposal_rewrite::{RewriteEvent, RewriteRequest, RewriteStatus};
use crate::proposals::{NoteChange, ProposalRecord};
use crate::{
    inbox::CaptureBinaryInboxRequest,
    proposal_apply::GroupApprovalRequest,
    proposals::{CommentRequest, CommentTarget, ReviewComment},
};
use brn_store::note_metadata;

#[track_caller]
fn intake(
    worker: &AppWorker,
    bytes: &[u8],
    name: &str,
) -> (
    crate::inbox_processing::InboxConversionPreview,
    ProposalRecord,
) {
    intake_at(worker, bytes, name, "harbor-source.md")
}
#[track_caller]
fn intake_at(
    worker: &AppWorker,
    bytes: &[u8],
    name: &str,
    path: &str,
) -> (
    crate::inbox_processing::InboxConversionPreview,
    ProposalRecord,
) {
    let capture = CaptureBinaryInboxRequest {
        id: Uuid::new_v4(),
        title: "Harbor synthetic email".into(),
        original_name: Some(name.into()),
        bytes: bytes.to_vec(),
    };
    let AppEvent::InboxCaptured(original) =
        reply_at(worker, capture.id, AppCommand::CaptureBinaryInbox(capture))
    else {
        panic!("capture")
    };
    let process = ProcessInboxRequest {
        limits: None,
        id: Uuid::new_v4(),
        items: vec![*original],
    };
    let mut wait = OperationWait::new(process.id, "ProcessInbox");
    worker
        .submit(process.id, AppCommand::ProcessInbox(process.clone()))
        .unwrap();
    loop {
        let (id, value) = wait.event(worker);
        if id == process.id
            && let AppEvent::InboxProcessing(batch) = value
            && batch.pending_count() == 0
        {
            assert!(
                matches!(
                    batch.entries[0].outcome,
                    InboxProcessOutcome::Converted { .. }
                ),
                "{batch:?}"
            );
            break;
        }
    }

    let candidate = InboxCandidateRequest {
        batch_id: process.id,
        index: 0,
    };
    let AppEvent::InboxCandidate(preview) =
        reply(worker, AppCommand::InboxCandidate(candidate.clone()))
    else {
        panic!("preview")
    };
    let source = InboxSourceRequest {
        candidate,
        proposal_id: Uuid::new_v4(),
        note_id: Uuid::new_v4(),
        path: path.into(),
        title: "Preserve Harbor source and images".into(),
    };
    let AppEvent::InboxSourceDraft(draft) = reply(worker, AppCommand::PrepareInboxSource(source))
    else {
        panic!("source draft")
    };
    let AppEvent::Proposal(record) = reply(worker, AppCommand::CreateProposal(*draft)) else {
        panic!("source review")
    };
    (*preview, record)
}

#[test]
fn p2_single_and_plural_private_investigation_rewrite_exact_group_and_restart() {
    for (bytes,plural) in [(include_bytes!("../../../../experiments/architecture-reassessment/p1-office-mime/fixtures/single.eml").as_slice(),false),(include_bytes!("../../../../experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml").as_slice(),true)] {
        eprintln!("P2 paired fixture phase={}",if plural {"plural"} else {"single"});
        let f=Fixture::new();
        let previous_id=Uuid::new_v4();
        let previous=note_identity::assign("# Harbor pilot\n\nExtension decision pending.\n",previous_id).unwrap();
        std::fs::write(f.base.path().join("vault/pilot.md"),&previous).unwrap();
        let script=Arc::new(Mutex::new(vec![]));
        let rewrite:crate::proposal_rewrite::RewriteHook=Arc::new(|_,prompt,_,_,_|Box::pin(async move {
            let record:ProposalRecord=serde_json::from_str(&prompt).unwrap();
            assert!(record.comments.iter().any(|c|c.text.contains("inspection")&&c.text.contains("4,000")));
            let texts:Vec<_>=record.draft.changes.iter().map(|c|c.text().map(|text|text.replace("Extension is proposed.","Extension is proposed after inspection, within the EUR 4,000 cap."))).collect();
            AiAnswer{text:json!({"title":"Harbor pilot with owner constraints","texts":texts}).to_string(),terminal:AiTerminal::Completed}
        }));
        let mut worker=f.start(Hooks{proposal_answer:Some(scripted(script.clone())),rewrite:Some(rewrite),..Hooks::default()});
        let (preview,source)=intake(&worker,bytes,"harbor.eml");
        let extraction=preview.extraction.as_ref().unwrap();
        assert_eq!(extraction.sources[0].bytes,bytes);
        let docx=extraction.sources.iter().find(|s|s.name=="harbor.docx").unwrap();
        assert!(docx.parent.is_some());assert!(docx.text.contains("15 October 2026"));assert!(docx.text.contains("Mira"));assert!(docx.text.contains("EUR 4,000"));
        let pictures:Vec<_>=extraction.occurrences.iter().filter(|o|o.source_id==docx.id).collect();
        assert_eq!(pictures.len(),2);assert_ne!(pictures[0].id,pictures[1].id);assert_ne!(pictures[0].locator,pictures[1].locator);assert_eq!(pictures[0].asset_id,pictures[1].asset_id);
        if plural {assert!(extraction.sources.iter().any(|s|s.name=="forecast.xlsx"&&s.status=="unprocessed"&&s.text.is_empty()&&!s.bytes.is_empty()));assert!(extraction.occurrences.iter().any(|o|o.source_id!=docx.id));}
        let AppEvent::InboxIntakeBinding(binding)=reply(&worker,AppCommand::InboxIntakeBinding{source_proposal_id:source.draft.id}) else {panic!("private binder")};
        let request=InboxActionRequest{intake:Some(*binding),source:None,visual_asset:None,purpose:InboxAnalysisPurpose::KnowledgeAndActions,id:Uuid::new_v4(),conversation:None,selection:Selection{provider:Provider::Chatgpt,model:"gpt-6-luna".into()},effort:crate::ReasoningEffort::Medium,generation:713};
        let knowledge:KnowledgeProposalArgs=serde_json::from_value(json!({"supersedes":"pilot.md","title":"Harbor extension proposal","path":"harbor-current.md","source_paths":[],"text":"# Harbor pilot\n\nExtension is proposed. The document records Pier B on 15 October 2026, owned by Mira. Interpreting the retained chart: 120 to 72 L/day is a 40% reduction.\n","quotes":[{"quote":"Decision: extend the pilot to Pier B on 15 October 2026. Owner: Mira.","source_id":docx.id}]})).unwrap();
        let mut action=args();action.title="Arrange Harbor inspection".into();let data=candidate_data_mut(&mut action);data.title="Arrange inspection with Mira".into();data.description="Confirm the inspection prerequisite before extending the Harbor pilot.".into();data.owner=Some("Mira".into());data.state=ActionCandidateState::Open;data.due_on=None;data.follow_up_on=None;
        *script.lock().unwrap()=vec![Step::Knowledge(knowledge),Step::Action(action)];
        let turn=analyze(&worker,&request).unwrap();assert_eq!(turn.status,WorkTurnStatus::Completed);
        assert!(results(&turn).iter().all(|r|r.get("ok").is_some()),"{}",turn.answer);
        assert!(!f.base.path().join("vault/harbor-source.md").exists());
        let proposals=analysis(&worker,request.id).proposals;assert_eq!(proposals.len(),2);
        let knowledge=proposals.iter().find(|p|p.draft.inbox_knowledge.is_some()).unwrap();
        let action=proposals.iter().find(|p|!p.draft.action_changes.is_empty()).unwrap();
        assert!(knowledge.draft.sources.iter().all(|s|s.path!="harbor-source.md"));
        assert!(action.draft.intake.is_some());
        let attempted=ApprovalRequest{operation_id:Uuid::new_v4(),expected:knowledge.stamp()};
        assert!(matches!(reply_at(&worker,attempted.operation_id,AppCommand::ApproveProposal(attempted)),AppEvent::Failed(_)));
        let comment=CommentRequest{expected:knowledge.stamp(),comment:ReviewComment{id:Uuid::new_v4(),text:"Retain the inspection prerequisite and EUR 4,000 budget cap.".into(),target:CommentTarget::Proposal}};
        let AppEvent::Proposal(commented)=reply(&worker,AppCommand::AddProposalComment(comment)) else {panic!("comment")};
        let rewrite=RewriteRequest{id:Uuid::new_v4(),expected:commented.stamp(),selection:request.selection.clone(),effort:crate::ReasoningEffort::Medium,generation:714};
        let mut rewrite_wait=OperationWait::new(rewrite.id,"StartProposalRewrite");
        worker.submit(rewrite.id,AppCommand::StartProposalRewrite(rewrite.clone())).unwrap();
        loop {let (id,value)=rewrite_wait.event(&worker);if id!=rewrite.id {continue;}match value {AppEvent::Rewrite(RewriteEvent::Finished{job,..})=>{assert_eq!(job.status,RewriteStatus::Completed,"{job:?}");break;},AppEvent::Failed(error)=>panic!("rewrite {error:?}"),_=>{}}}
        let AppEvent::Proposal(revised)=reply(&worker,AppCommand::Proposal(knowledge.draft.id)) else {panic!("revised")};
        assert!(revised.draft.changes[0].text().unwrap().contains("after inspection, within the EUR 4,000 cap"));
        let stale=ApprovalRequest{operation_id:Uuid::new_v4(),expected:knowledge.stamp()};assert!(matches!(reply_at(&worker,stale.operation_id,AppCommand::ApproveProposal(stale)),AppEvent::Failed(_)));
        let group=GroupApprovalRequest{group_id:request.id,approvals:vec![ApprovalRequest{operation_id:Uuid::new_v4(),expected:revised.stamp()},ApprovalRequest{operation_id:Uuid::new_v4(),expected:action.stamp()},ApprovalRequest{operation_id:Uuid::new_v4(),expected:source.stamp()}]};
        let AppEvent::ProposalGroupApplied(applied)=reply(&worker,AppCommand::ApproveProposalGroup(group.clone())) else {panic!("group")};
        assert!(applied.stopped.is_none(),"{applied:?}");assert_eq!(applied.receipts.len(),3);assert!(applied.receipts.iter().all(|r|r.outcome==ApplyOutcome::Applied));
        let action_id=action.draft.action_changes[0].id();
        let source_text=std::fs::read(f.base.path().join("vault/harbor-source.md")).unwrap();
        let current=std::fs::read(f.base.path().join("vault/harbor-current.md")).unwrap();
        assert!(note_metadata::classify(&std::fs::read_to_string(f.base.path().join("vault/pilot.md")).unwrap()).unwrap().history);
        worker.shutdown().unwrap();
        let mut restarted=f.start(Hooks{proposal_answer:Some(Arc::new(|_,_,_,_,_,_|panic!("history reran model"))),..Hooks::default()});
        let replay=analysis(&restarted,request.id);assert_eq!(replay.proposals.len(),2);assert_eq!(replay.turn.unwrap().answer,turn.answer);
        let AppEvent::InboxCandidate(retained)=reply(&restarted,AppCommand::InboxCandidate(preview.request.clone())) else {panic!("retained preview")};assert_eq!(*retained,preview);
        assert_eq!(std::fs::read(f.base.path().join("vault/harbor-source.md")).unwrap(),source_text);assert_eq!(std::fs::read(f.base.path().join("vault/harbor-current.md")).unwrap(),current);
        let AppEvent::Action(retained_action)=reply(&restarted,AppCommand::Action(action_id)) else {panic!("retained action")};assert_eq!(retained_action.data.owner.as_deref(),Some("Mira"));assert_eq!(retained_action.data.due_on,None);
        restarted.shutdown().unwrap();
    }
}

#[test]
fn private_action_refuses_changed_original_or_applied_image() {
    for damage_asset in [false, true] {
        let f = Fixture::new();
        let mut worker = f.start(Hooks {
            proposal_answer: Some(scripted(Arc::new(Mutex::new(vec![Step::Action(args())])))),
            ..Hooks::default()
        });
        let (preview, source) = intake(
            &worker,
            include_bytes!(
                "../../../../experiments/architecture-reassessment/p1-office-mime/fixtures/single.eml"
            ),
            "harbor.eml",
        );
        let AppEvent::InboxIntakeBinding(binding) = reply(
            &worker,
            AppCommand::InboxIntakeBinding {
                source_proposal_id: source.draft.id,
            },
        ) else {
            panic!("binding")
        };
        let request = InboxActionRequest {
            intake: Some(*binding),
            source: None,
            visual_asset: None,
            purpose: InboxAnalysisPurpose::Actions,
            id: Uuid::new_v4(),
            conversation: None,
            selection: Selection {
                provider: Provider::Chatgpt,
                model: "gpt-6-luna".into(),
            },
            effort: crate::ReasoningEffort::Medium,
            generation: 715,
        };
        let turn = analyze(&worker, &request).unwrap();
        assert_eq!(turn.status, WorkTurnStatus::Completed);
        let action = analysis(&worker, request.id).proposals.remove(0);
        let approve = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: source.stamp(),
        };
        assert!(matches!(
            reply_at(
                &worker,
                approve.operation_id,
                AppCommand::ApproveProposal(approve)
            ),
            AppEvent::ProposalApplied(_)
        ));
        if damage_asset {
            let asset = source
                .draft
                .changes
                .iter()
                .find_map(|c| {
                    if let NoteChange::CreateAsset { path, .. } = c {
                        Some(path)
                    } else {
                        None
                    }
                })
                .unwrap();
            std::fs::write(f.base.path().join("vault").join(asset), b"changed image").unwrap();
        } else {
            let original = f
                .base
                .path()
                .join("data/inbox")
                .join(format!("{}.bin", preview.original.capture.id));
            std::fs::write(original, b"changed original").unwrap();
        }
        let approve = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: action.stamp(),
        };
        assert!(matches!(
            reply_at(
                &worker,
                approve.operation_id,
                AppCommand::ApproveProposal(approve)
            ),
            AppEvent::Failed(_)
        ));
        let AppEvent::Proposal(retained) = reply(&worker, AppCommand::Proposal(action.draft.id))
        else {
            panic!("retained")
        };
        assert_eq!(retained.state, crate::proposals::ProposalState::Draft);
        worker.shutdown().unwrap();
    }
}

#[test]
fn repeated_imports_with_shared_images_apply_to_one_source_directory() {
    let f = Fixture::new();
    let mut worker = f.start(Hooks::default());
    let (preview, first) = intake_at(
        &worker,
        include_bytes!(
            "../../../../experiments/architecture-reassessment/p1-office-mime/fixtures/single.eml"
        ),
        "single.eml",
        "first.md",
    );
    let (_, second) = intake_at(
        &worker,
        include_bytes!(
            "../../../../experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml"
        ),
        "plural.eml",
        "second.md",
    );
    let request = InboxSourceRequest {
        candidate: preview.request,
        proposal_id: Uuid::new_v4(),
        note_id: Uuid::new_v4(),
        path: "same-snapshot.md".into(),
        title: "Preserve again".into(),
    };
    let AppEvent::InboxSourceDraft(draft) = reply(&worker, AppCommand::PrepareInboxSource(request))
    else {
        panic!("new Source")
    };
    let AppEvent::Proposal(third) = reply(&worker, AppCommand::CreateProposal(*draft)) else {
        panic!("new review")
    };
    let mut paths = std::collections::HashSet::new();
    for source in [first, second, third] {
        for change in &source.draft.changes[1..] {
            assert!(paths.insert(change.path().to_owned()));
        }
        let approve = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: source.stamp(),
        };
        assert!(matches!(
            reply_at(
                &worker,
                approve.operation_id,
                AppCommand::ApproveProposal(approve)
            ),
            AppEvent::ProposalApplied(_)
        ));
    }
    worker.shutdown().unwrap();
}

#[test]
fn short_valid_eml_does_not_inherit_the_legacy_zip_size_floor() {
    let f = Fixture::new();
    let mut worker = f.start(Hooks::default());
    let (_, source) = intake_at(
        &worker,
        b"Subject: a\r\n\r\nb",
        "short.eml",
        "short-source.md",
    );
    let AppEvent::InboxIntakeBinding(_) = reply(
        &worker,
        AppCommand::InboxIntakeBinding {
            source_proposal_id: source.draft.id,
        },
    ) else {
        panic!("short MIME Source must support private review");
    };
    worker.shutdown().unwrap();
}
