#![cfg(target_os = "macos")]
//! Saved profile observations compose checked records without granting new authority.
use brn_store::{
    WorkStore,
    note_provenance::{self, VaultCitation},
    work::{
        proposal_apply::{ApplyOutcome, ApprovalRequest},
        proposals::{ActionChange, ProposalDraft},
    },
};
use brn_workflow::{
    ErrorKind,
    action_completion::CompleteActionRequest,
    actions::{ActionData, ActionRecord, ActionState},
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
    knowledge::{EdgeOrigin, IdentityOutcome, ProfileContext, ProfileContextRequest, ProfileLens},
    library::KnowledgeScope,
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    credentials: PathBuf,
    profile: Uuid,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        let fixture = Self {
            credentials: owner.path().join("credentials"),
            _owner: owner,
            data,
            vault,
            profile: Uuid::new_v4(),
        };
        fixture.write(
            "z-profile.md",
            &managed(fixture.profile, "# Exact õ profile\r\n"),
        );
        fixture
    }
    fn write(&self, path: &str, text: &str) {
        let path = self.vault.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn config(&self) -> AppConfig {
        AppConfig {
            vault_root: Some(self.vault.clone()),
            credentials_dir: Some(self.credentials.clone()),
            model_dir: None,
        }
    }
    fn app(&self) -> App {
        App::open(&self.data, self.config()).unwrap()
    }
    fn request(&self, _app: &mut App, lens: ProfileLens, limit: usize) -> ProfileContextRequest {
        ProfileContextRequest {
            profile: {
                use std::os::unix::fs::MetadataExt;
                let path = self.vault.join("z-profile.md");
                let metadata = fs::metadata(&path).unwrap();
                let bytes = fs::read(&path).unwrap();
                brn_workflow::proposals::SourceVersion {
                    path: "z-profile.md".into(),
                    fingerprint: brn_store::files::FileFingerprint {
                        device: metadata.dev(),
                        inode: metadata.ino(),
                        len: bytes.len() as u64,
                        sha256: Sha256::digest(bytes).into(),
                    },
                }
            },
            note_id: self.profile,
            lens,
            action_offset: 0,
            relationship_offset: 0,
            limit,
        }
    }
    fn seed(&self, data: ActionData) -> ActionRecord {
        let (mut store, _) = WorkStore::open(&self.data).unwrap();
        let id = Uuid::new_v4();
        let draft = ProposalDraft {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            vault: None,
            title: "Exact synthetic approval".into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Create { id, data }],
        };
        let record = store.create_proposal(&draft).unwrap();
        let request = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        };
        store.begin_proposal_apply(&request).unwrap();
        store
            .record_proposal_prepared(request.operation_id, &[])
            .unwrap();
        store
            .finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&[]))
            .unwrap();
        store.action(id).unwrap().unwrap()
    }
}
fn managed(id: Uuid, body: &str) -> String {
    format!("---\nbrn_id: {id}\n---\n{body}")
}
fn action(
    person: Option<Uuid>,
    project: Option<Uuid>,
    sources: Vec<Uuid>,
    thread: Option<Uuid>,
) -> ActionData {
    ActionData {
        title: "Exact Action\r\nõ".into(),
        description: "Full retained description\twith proof".into(),
        state: ActionState::Open,
        owner: Some("Explicit owner".into()),
        related_person: person,
        related_project: project,
        sources,
        thread,
        due_on: None,
        follow_up_on: None,
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: None,
    }
}

#[test]
fn lenses_complete_records_cross_scope_proofs_and_independent_paging_ignore_first_200_rows() {
    let f = Fixture::new();
    let source = Uuid::new_v4();
    let history = Uuid::new_v4();
    let incoming = Uuid::new_v4();
    let source_text = format!("---\nbrn_id: {source}\nbrn_kind: source\n---\nExact email õ\r\n");
    f.write("source.md", &source_text);
    // Both source/history flags remain History, never Current.
    f.write(
        "archive/history.md",
        &format!("---\nbrn_id: {history}\nbrn_kind: source\n---\nHistorical exact text\n"),
    );
    f.write(
        "incoming.md",
        &managed(incoming, &format!("[profile](brn://note/{})", f.profile)),
    );
    let start = source_text.find("Exact").unwrap();
    let profile_text = note_provenance::write(
        &managed(
            f.profile,
            &format!(
                "# Exact profile\r\n[email](brn://note/{source})\n[history](brn://note/{history})\n"
            ),
        ),
        &[VaultCitation {
            note_id: source,
            sha256: Sha256::digest(source_text.as_bytes()).into(),
            start_byte: start,
            end_byte: source_text.len(),
            quote: source_text[start..].into(),
        }],
    )
    .unwrap();
    f.write("z-profile.md", &profile_text);
    let person = f.seed(action(Some(f.profile), None, vec![source], Some(history)));
    let project = f.seed(action(None, Some(f.profile), vec![history], Some(source)));
    // These newer rows fill the first underlying checked Action page.
    for _ in 0..205 {
        f.seed(action(None, None, vec![], None));
    }
    // These earlier-path unrelated edges fill the first All-scope edge page.
    let target = Uuid::new_v4();
    f.write("a-target.md", &managed(target, "Unrelated target"));
    for index in 0..205 {
        f.write(
            &format!("a-{index:03}.md"),
            &managed(Uuid::new_v4(), &format!("[other](brn://note/{target})")),
        );
    }
    let before = fs::read(f.vault.join("z-profile.md")).unwrap();
    let mut app = f.app();
    let req = f.request(&mut app, ProfileLens::Person, 2);
    let completed = app
        .complete_action(&CompleteActionRequest {
            operation_id: Uuid::new_v4(),
            before: Box::new(person.clone()),
        })
        .unwrap();
    let completed_record = app.action(person.origin.id).unwrap();
    assert_eq!(completed_record.data.state, ActionState::Completed);
    let applies = app.work_store().proposal_applies().unwrap();
    let actions_before = app
        .actions(&brn_workflow::actions::ActionListRequest {
            limit: 200,
            ..Default::default()
        })
        .unwrap();
    assert!(
        app.work_store()
            .setting("vault.editor_identity")
            .unwrap()
            .is_none()
    );
    let context = app.profile_context(&req).unwrap();
    assert!(
        app.work_store()
            .setting("vault.editor_identity")
            .unwrap()
            .is_none()
    );
    context.validate_for(&req).unwrap();
    assert_eq!(context.profile.text, profile_text);
    assert_eq!(context.action_total, 1);
    assert_eq!(context.actions, vec![completed_record]);
    assert_eq!(context.relationship_total, 4);
    assert_eq!(context.relationships.len(), 2);
    assert!(context.complete && context.issues.is_empty());
    assert_eq!(context.references.len(), 2);
    assert!(
        context
            .references
            .iter()
            .all(|reference| reference.resolution.outcome == IdentityOutcome::Unique)
    );
    let email = context
        .references
        .iter()
        .find(|reference| reference.resolution.note_id == source)
        .unwrap();
    assert_eq!(email.matches[0].scope, Some(KnowledgeScope::Source));
    let historical = context
        .references
        .iter()
        .find(|reference| reference.resolution.note_id == history)
        .unwrap();
    assert_eq!(historical.matches[0].scope, Some(KnowledgeScope::History));
    let next_req = ProfileContextRequest {
        relationship_offset: 2,
        action_offset: 1,
        ..req.clone()
    };
    let next = app.profile_context(&next_req).unwrap();
    assert!(next.actions.is_empty());
    assert_eq!(next.relationship_total, 4);
    assert_eq!(next.relationships.len(), 2);
    let edges = context
        .relationships
        .iter()
        .chain(&next.relationships)
        .collect::<Vec<_>>();
    assert!(
        edges.iter().any(|edge| edge.edge.source.note_id == incoming
            && edge.target_scope == KnowledgeScope::Current)
    );
    assert!(
        edges.iter().any(|edge| edge.edge.target.note_id == history
            && edge.target_scope == KnowledgeScope::History)
    );
    let provenance = edges
        .iter()
        .find(|edge| edge.edge.origin == EdgeOrigin::InferredProvenance)
        .unwrap();
    assert_eq!(provenance.edge.evidence[0].quote, source_text[start..]);
    assert_eq!(
        provenance.edge.target.sha256,
        <[u8; 32]>::from(Sha256::digest(source_text.as_bytes()))
    );
    let project_req = ProfileContextRequest {
        lens: ProfileLens::Project,
        ..req.clone()
    };
    assert_eq!(
        app.profile_context(&project_req).unwrap().actions,
        vec![project]
    );
    assert_eq!(app.work_store().proposal_applies().unwrap(), applies);
    assert_eq!(
        app.actions(&brn_workflow::actions::ActionListRequest {
            limit: 200,
            ..Default::default()
        })
        .unwrap(),
        actions_before
    );
    assert_eq!(
        app.work_store()
            .action_completion_for(&completed.request)
            .unwrap(),
        Some(completed)
    );
    assert!(app.selection().unwrap().is_none());
    assert!(app.work_store().conversations().unwrap().is_empty());
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
    assert_eq!(fs::read(f.vault.join("z-profile.md")).unwrap(), before);
}

#[test]
fn context_requires_exact_current_unique_complete_profile_identity() {
    let f = Fixture::new();
    let mut app = f.app();
    let req = f.request(&mut app, ProfileLens::Person, 25);
    f.write(
        "z-profile.md",
        &managed(f.profile, "Changed physical saved bytes"),
    );
    assert_eq!(
        app.profile_context(&req).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    let req = f.request(&mut app, ProfileLens::Person, 25);
    f.write("duplicate.md", &managed(f.profile, "Duplicate"));
    assert_eq!(
        app.profile_context(&req).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    fs::remove_file(f.vault.join("duplicate.md")).unwrap();
    f.write("bad.md", "---\nbrn_id: invalid\n---\nUnreadable identity");
    let error = app.profile_context(&req).unwrap_err();
    assert_eq!(error.kind, ErrorKind::ContextStale);
    assert!(error.message.contains("Incomplete"));
    fs::remove_file(f.vault.join("bad.md")).unwrap();
    f.write("z-profile.md", "Unmanaged saved note");
    let req = f.request(&mut app, ProfileLens::Person, 25);
    assert_eq!(
        app.profile_context(&req).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    f.write(
        "z-profile.md",
        &format!("---\nbrn_id: {}\nbrn_kind: source\n---\nSource", f.profile),
    );
    let req = f.request(&mut app, ProfileLens::Person, 25);
    assert_eq!(
        app.profile_context(&req).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    assert!(app.work_store().proposals(None).unwrap().is_empty());
}

#[test]
fn references_preserve_missing_ambiguous_and_unknown_classification_with_coverage() {
    let f = Fixture::new();
    let ambiguous = Uuid::new_v4();
    let missing = Uuid::new_v4();
    let invalid = Uuid::new_v4();
    f.write("a.md", &managed(ambiguous, "Alias A"));
    f.write("b.md", &managed(ambiguous, "Alias B"));
    f.write(
        "invalid.md",
        &format!("---\nbrn_id: {invalid}\nbrn_kind: invalid\n---\nUnknown scope"),
    );
    f.seed(action(
        Some(f.profile),
        None,
        vec![ambiguous, missing, invalid],
        Some(ambiguous),
    ));
    let mut app = f.app();
    let req = f.request(&mut app, ProfileLens::Person, 25);
    let context = app.profile_context(&req).unwrap();
    assert!(!context.complete);
    assert_eq!(context.references.len(), 3);
    assert_eq!(context.duplicates.len(), 1);
    assert_eq!(
        context
            .references
            .iter()
            .find(|r| r.resolution.note_id == ambiguous)
            .unwrap()
            .resolution
            .outcome,
        IdentityOutcome::Ambiguous
    );
    assert_eq!(
        context
            .references
            .iter()
            .find(|r| r.resolution.note_id == missing)
            .unwrap()
            .resolution
            .outcome,
        IdentityOutcome::Absent
    );
    let unknown = context
        .references
        .iter()
        .find(|r| r.resolution.note_id == invalid)
        .unwrap();
    assert_eq!(unknown.resolution.outcome, IdentityOutcome::Incomplete);
    assert_eq!(unknown.matches[0].scope, None);
    assert!(
        context
            .issues
            .iter()
            .any(|issue| issue.path == "invalid.md")
    );
    context.validate_for(&req).unwrap();
}

#[test]
fn real_worker_restart_and_index_loss_reconstruct_identical_context() {
    let f = Fixture::new();
    let linked = Uuid::new_v4();
    f.write("target.md", &managed(linked, "Saved support"));
    f.write(
        "z-profile.md",
        &managed(f.profile, &format!("[target](brn://note/{linked})")),
    );
    f.seed(action(Some(f.profile), None, vec![linked], Some(linked)));
    let req = f.request(&mut f.app(), ProfileLens::Person, 25);
    let mut expected: Option<ProfileContext> = None;
    for _ in 0..2 {
        let mut worker = AppWorker::start(f.data.clone(), f.config()).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let id = Uuid::new_v4();
        worker
            .submit(id, AppCommand::ProfileContext(req.clone()))
            .unwrap();
        let (actual, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(actual, id);
        let AppEvent::ProfileContext(context) = event else {
            panic!("expected context");
        };
        context.validate_for(&req).unwrap();
        if let Some(expected) = &expected {
            assert_eq!(&*context, expected);
        }
        expected = Some(*context);
        worker.shutdown().unwrap();
        fs::remove_file(f.data.join("index.sqlite")).unwrap();
    }
}

#[test]
fn pure_request_and_reply_validation_rejects_malformed_correlations_and_proofs() {
    let f = Fixture::new();
    let target = Uuid::new_v4();
    f.write("target.md", &managed(target, "Target"));
    f.write(
        "z-profile.md",
        &managed(f.profile, &format!("[link](brn://note/{target})")),
    );
    f.seed(action(Some(f.profile), None, vec![target], Some(target)));
    let mut app = f.app();
    let req = f.request(&mut app, ProfileLens::Person, 25);
    let context = app.profile_context(&req).unwrap();
    for mutate in [
        |r: &mut ProfileContextRequest| r.note_id = Uuid::nil(),
        |r: &mut ProfileContextRequest| r.limit = 0,
        |r: &mut ProfileContextRequest| r.limit = 201,
        |r: &mut ProfileContextRequest| r.action_offset = usize::MAX,
        |r: &mut ProfileContextRequest| r.relationship_offset = usize::MAX,
        |r: &mut ProfileContextRequest| r.profile.path = "../escape.md".into(),
        |r: &mut ProfileContextRequest| r.profile.path = "archive/old.md".into(),
        |r: &mut ProfileContextRequest| r.profile.fingerprint.len = 1024 * 1024 + 1,
    ] {
        let mut bad = req.clone();
        mutate(&mut bad);
        assert!(bad.validate().is_err());
    }
    let mut json = serde_json::to_value(&req).unwrap();
    json["profile"]["fingerprint"]["unknown"] = true.into();
    assert!(serde_json::from_value::<ProfileContextRequest>(json).is_err());
    for mutate in [
        |c: &mut ProfileContext| c.request.lens = ProfileLens::Project,
        |c: &mut ProfileContext| c.profile.text.push('x'),
        |c: &mut ProfileContext| c.action_total = 0,
        |c: &mut ProfileContext| c.relationship_total = 0,
        |c: &mut ProfileContext| c.actions[0].data.related_person = None,
        |c: &mut ProfileContext| c.relationships[0].target_scope = KnowledgeScope::All,
        |c: &mut ProfileContext| c.relationships[0].edge.source.sha256 = [0; 32],
        |c: &mut ProfileContext| c.relationships[0].edge.evidence[0].quote = "wrong".into(),
        |c: &mut ProfileContext| c.references.clear(),
        |c: &mut ProfileContext| c.references[0].matches.clear(),
        |c: &mut ProfileContext| c.references[0].resolution.outcome = IdentityOutcome::Absent,
        |c: &mut ProfileContext| c.references[0].matches[0].scope = None,
        |c: &mut ProfileContext| {
            c.issues.push(brn_workflow::knowledge::IdentityIssue {
                path: "x.md".into(),
                reason: "Incomplete".into(),
            })
        },
    ] {
        let mut bad = context.clone();
        mutate(&mut bad);
        assert!(bad.validate_for(&req).is_err());
    }
    context.validate_for(&req).unwrap();
}

#[test]
fn direct_relationship_pages_exceed_200_and_stale_citation_is_visible_uncertainty() {
    let f = Fixture::new();
    let mut body = String::new();
    for index in 0..205 {
        let id = Uuid::new_v4();
        f.write(
            &format!("targets/{index:03}.md"),
            &managed(id, "Exact target"),
        );
        body.push_str(&format!("[target{index}](brn://note/{id})\n"));
    }
    f.write("z-profile.md", &managed(f.profile, &body));
    let mut app = f.app();
    let request = f.request(&mut app, ProfileLens::Person, 200);
    let first = app.profile_context(&request).unwrap();
    assert_eq!(first.relationship_total, 205);
    assert_eq!(first.relationships.len(), 200);
    let last = app
        .profile_context(&ProfileContextRequest {
            relationship_offset: 200,
            ..request
        })
        .unwrap();
    assert_eq!(last.relationship_total, 205);
    assert_eq!(last.relationships.len(), 5);
    assert_eq!(last.relationships[0].edge.target.path, "targets/200.md");
    let source = Uuid::new_v4();
    let original = managed(source, "Exact citation text õ");
    f.write("source.md", &original);
    let start = original.find("Exact").unwrap();
    let profile = note_provenance::write(
        &managed(f.profile, "# Saved profile"),
        &[VaultCitation {
            note_id: source,
            sha256: Sha256::digest(original.as_bytes()).into(),
            start_byte: start,
            end_byte: original.len(),
            quote: original[start..].into(),
        }],
    )
    .unwrap();
    f.write("z-profile.md", &profile);
    f.write("source.md", &managed(source, "Changed citation text"));
    let request = f.request(&mut app, ProfileLens::Person, 25);
    let context = app.profile_context(&request).unwrap();
    assert_eq!(context.relationship_total, 0);
    assert!(!context.complete);
    assert!(
        context
            .issues
            .iter()
            .any(|issue| issue.path == "z-profile.md"
                && issue.reason.contains("changed source bytes"))
    );
}

#[test]
fn pending_and_uncertain_application_fence_context_and_invalid_shape_precedes_evidence() {
    for uncertain in [false, true] {
        let f = Fixture::new();
        let mut app = f.app();
        let request = f.request(&mut app, ProfileLens::Person, 25);
        let draft = ProposalDraft {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            vault: None,
            title: "Pending synthetic Action".into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Create {
                id: Uuid::new_v4(),
                data: action(None, None, vec![], None),
            }],
        };
        let proposal = app.work_store_mut().create_proposal(&draft).unwrap();
        let approval = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: proposal.stamp(),
        };
        app.work_store_mut()
            .begin_proposal_apply(&approval)
            .unwrap();
        if uncertain {
            app.work_store_mut()
                .finish_proposal_apply(approval.operation_id, ApplyOutcome::Uncertain, None)
                .unwrap();
        }
        assert_eq!(
            app.profile_context(&request).unwrap_err().kind,
            ErrorKind::SaveUncertain
        );
        assert_eq!(
            app.profile_context(&ProfileContextRequest {
                limit: 0,
                ..request
            })
            .unwrap_err()
            .kind,
            ErrorKind::ToolRejected
        );
        assert!(
            app.work_store()
                .setting("vault.editor_identity")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            fs::read_to_string(f.vault.join("z-profile.md")).unwrap(),
            managed(f.profile, "# Exact õ profile\r\n")
        );
    }
}
