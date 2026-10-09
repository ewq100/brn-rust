#![cfg(target_os = "macos")]
use brn_store::note_provenance::{self, VaultCitation};
use brn_workflow::{
    ErrorKind,
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
    knowledge::{EdgeEvidence, EdgeOrigin, EvidenceEndpoint, RelationshipRequest},
    library::KnowledgeScope,
    proposal_apply::ApprovalRequest,
    proposals::{DraftNoteChange, DraftRequest},
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

const A: &str = "11111111-1111-4111-8111-111111111111";
const B: &str = "22222222-2222-4222-8222-222222222222";
const C: &str = "33333333-3333-4333-8333-333333333333";

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    credentials: PathBuf,
    original: String,
    target: String,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        let credentials = owner.path().join("task.credentials");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::create_dir(vault.join("archive")).unwrap();
        let target = format!("---\nbrn_id: {B}\nbrn_kind: source\n---\nExact tõend õ\r\n");
        let start = target.find("Exact").unwrap();
        let original = note_provenance::write(
            &format!("\u{feff}---\r\nbrn_id: {A}\r\n...\r\n# Õ\r\n[one](brn://note/{B})\r\n[ref][proof]\r\n[current](brn://note/{C})\r\n[self](#heading)\r\n[web](https://example.invalid/n.md)\r\n[unmanaged](ordinary.md)\r\n\r\n[proof]: archive/source.md\r\n"),
            &[VaultCitation {
                note_id: B.parse().unwrap(),
                sha256: Sha256::digest(target.as_bytes()).into(),
                start_byte: start,
                end_byte: target.len(),
                quote: target[start..].into(),
            }],
        ).unwrap();
        fs::write(vault.join("a.md"), &original).unwrap();
        fs::write(vault.join("archive/source.md"), &target).unwrap();
        fs::write(
            vault.join("c.md"),
            format!("---\nbrn_id: {C}\n---\nCurrent õ\n"),
        )
        .unwrap();
        fs::write(vault.join("ordinary.md"), "ordinary unmanaged note").unwrap();
        Self {
            _owner: owner,
            data,
            vault,
            credentials,
            original,
            target,
        }
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
}
fn request(scope: KnowledgeScope, offset: usize, limit: usize) -> RelationshipRequest {
    RelationshipRequest {
        scope,
        offset,
        limit,
    }
}

#[test]
fn exact_coalesced_explicit_links_and_inferred_provenance_remain_distinct_without_writes() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    let page = app
        .relationships(&request(KnowledgeScope::All, 0, 50))
        .unwrap();
    assert_eq!(page.total, 3);
    assert_eq!(page.edges.len(), 3);
    assert!(page.issues.is_empty() && page.duplicates.is_empty());
    let explicit = page
        .edges
        .iter()
        .find(|edge| {
            edge.target.note_id == B.parse::<Uuid>().unwrap()
                && edge.origin == EdgeOrigin::ExplicitLink
        })
        .unwrap();
    assert_eq!(explicit.evidence.len(), 3);
    for edge in &page.edges {
        assert_eq!(edge.source.note_id, A.parse::<Uuid>().unwrap());
        assert_eq!(
            edge.source.sha256,
            Sha256::digest(fixture.original.as_bytes()).as_slice()
        );
        for proof in &edge.evidence {
            let text = match proof.endpoint {
                EvidenceEndpoint::Source => &fixture.original,
                EvidenceEndpoint::Target => &fixture.target,
            };
            assert_eq!(
                text.get(proof.start_byte..proof.end_byte),
                Some(proof.quote.as_str())
            );
        }
    }
    let inferred = page
        .edges
        .iter()
        .find(|edge| edge.origin == EdgeOrigin::InferredProvenance)
        .unwrap();
    assert_eq!(inferred.evidence[0].endpoint, EvidenceEndpoint::Target);
    assert_eq!(
        fs::read(fixture.vault.join("a.md")).unwrap(),
        fixture.original.as_bytes()
    );
    assert_eq!(
        fs::read(fixture.vault.join("archive/source.md")).unwrap(),
        fixture.target.as_bytes()
    );
    assert!(app.work_store().proposals(None).unwrap().is_empty());
    assert_eq!(fs::read_dir(&fixture.credentials).unwrap().count(), 0);
}

#[test]
fn accepted_maximum_reference_link_count_fits_coalesced_relationship_proofs() {
    let fixture = Fixture::new();
    let text = format!(
        "---\nbrn_id: {A}\n---\n{}\n[r]: archive/source.md\n",
        "[x][r]\n".repeat(4096)
    );
    fs::write(fixture.vault.join("a.md"), &text).unwrap();
    let mut app = fixture.app();
    let links = app.note_links("a.md").unwrap();
    assert_eq!(links.links.len(), 4096);
    let page = app
        .relationships(&request(KnowledgeScope::All, 0, 50))
        .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.edges[0].evidence.len(), 4097);
    let occurrence = "[x][r]";
    let definition = "[r]: archive/source.md";
    let proof = |start: usize, quote: &str| EdgeEvidence {
        endpoint: EvidenceEndpoint::Source,
        start_byte: start,
        end_byte: start + quote.len(),
        quote: quote.into(),
    };
    let starts = text
        .match_indices(occurrence)
        .map(|(start, _)| start)
        .collect::<Vec<_>>();
    let mut expected = vec![
        proof(starts[0], occurrence),
        proof(text.find(definition).unwrap(), definition),
    ];
    expected.extend(starts[1..].iter().map(|start| proof(*start, occurrence)));
    assert_eq!(page.edges[0].evidence, expected);
    assert_eq!(
        fs::read(fixture.vault.join("a.md")).unwrap(),
        text.as_bytes()
    );
}

#[test]
fn maximum_distinct_reference_proofs_preserve_first_seen_unicode_byte_order() {
    let fixture = Fixture::new();
    let mut text = format!("\u{feff}---\r\nbrn_id: {A}\r\n---\r\n");
    let mut occurrences = Vec::new();
    for index in (0..4096).rev() {
        let quote = format!("[õ{index}][r{index}]");
        occurrences.push((text.len(), quote.clone()));
        text.push_str(&quote);
        text.push_str("\r\n");
    }
    text.push_str("\r\n");
    let mut definitions = Vec::new();
    for index in (0..4096).rev() {
        let quote = format!("[r{index}]: archive/source.md");
        definitions.push((text.len(), quote.clone()));
        text.push_str(&quote);
        text.push_str("\r\n");
    }
    let expected = occurrences
        .into_iter()
        .zip(definitions)
        .flat_map(|(occurrence, definition)| {
            [occurrence, definition].map(|(start, quote)| EdgeEvidence {
                endpoint: EvidenceEndpoint::Source,
                start_byte: start,
                end_byte: start + quote.len(),
                quote,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 8192);
    fs::write(fixture.vault.join("a.md"), &text).unwrap();
    let mut app = fixture.app();
    for _ in 0..2 {
        let page = app
            .relationships(&request(KnowledgeScope::All, 0, 50))
            .unwrap();
        assert!(page.issues.is_empty() && page.duplicates.is_empty());
        assert_eq!(page.total, 1);
        assert_eq!(page.edges[0].evidence, expected);
        assert_eq!(
            page.edges[0].source.sha256,
            <[u8; 32]>::from(Sha256::digest(text.as_bytes()))
        );
        for proof in &page.edges[0].evidence {
            assert_eq!(
                text.get(proof.start_byte..proof.end_byte),
                Some(proof.quote.as_str())
            );
        }
    }
    assert_eq!(
        fs::read(fixture.vault.join("a.md")).unwrap(),
        text.as_bytes()
    );
    assert!(app.work_store().proposals(None).unwrap().is_empty());
}

#[test]
fn scope_filters_both_endpoints_before_pagination_and_reports_matching_total() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    let current = app
        .relationships(&request(KnowledgeScope::Current, 0, 1))
        .unwrap();
    assert_eq!(current.total, 1);
    assert_eq!(current.edges[0].target.path, "c.md");
    assert!(
        app.relationships(&request(KnowledgeScope::Current, 1, 1))
            .unwrap()
            .edges
            .is_empty()
    );
    for scope in [KnowledgeScope::Source, KnowledgeScope::History] {
        assert_eq!(app.relationships(&request(scope, 0, 1)).unwrap().total, 0);
    }
    let all = app
        .relationships(&request(KnowledgeScope::All, 1, 1))
        .unwrap();
    assert_eq!(all.total, 3);
    assert_eq!(all.edges.len(), 1);
    assert_eq!(all.offset, 1);
    assert_eq!(all.scope, KnowledgeScope::All);
    assert!(
        app.relationships(&request(KnowledgeScope::All, usize::MAX, 1))
            .unwrap()
            .edges
            .is_empty()
    );
}

#[test]
fn unchanged_source_does_not_cache_target_changes_moves_or_duplicate_identities() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    assert_eq!(
        app.relationships(&request(KnowledgeScope::All, 0, 50))
            .unwrap()
            .total,
        3
    );
    let path = fixture.vault.join("archive/source.md");
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let replacement = fixture.target.replace("Exact", "Other");
    assert_eq!(replacement.len(), fixture.target.len());
    fs::write(&path, &replacement).unwrap();
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let changed = app
        .relationships(&request(KnowledgeScope::All, 0, 50))
        .unwrap();
    assert_eq!(changed.total, 2);
    assert!(
        changed
            .issues
            .iter()
            .any(|issue| issue.path == "a.md" && issue.reason.contains("changed source bytes"))
    );
    assert!(
        changed
            .edges
            .iter()
            .all(|edge| edge.origin == EdgeOrigin::ExplicitLink)
    );
    let proof = changed
        .edges
        .iter()
        .find(|edge| edge.target.note_id == B.parse::<Uuid>().unwrap())
        .unwrap();
    assert_eq!(
        proof.target.sha256,
        Sha256::digest(replacement.as_bytes()).as_slice()
    );
    fs::rename(&path, fixture.vault.join("moved.md")).unwrap();
    let moved = app
        .relationships(&request(KnowledgeScope::All, 0, 50))
        .unwrap();
    assert_eq!(moved.total, 2);
    let moved = moved
        .edges
        .iter()
        .find(|edge| edge.target.note_id == B.parse::<Uuid>().unwrap())
        .unwrap();
    assert_eq!(moved.target.path, "moved.md");
    assert_eq!(moved.evidence.len(), 1);
    fs::copy(
        fixture.vault.join("moved.md"),
        fixture.vault.join("duplicate.md"),
    )
    .unwrap();
    let duplicate = app
        .relationships(&request(KnowledgeScope::All, 0, 50))
        .unwrap();
    assert_eq!(duplicate.total, 1);
    assert_eq!(duplicate.duplicates.len(), 1);
    fs::copy(
        fixture.vault.join("a.md"),
        fixture.vault.join("duplicate-consumer.md"),
    )
    .unwrap();
    let duplicate_source = app
        .relationships(&request(KnowledgeScope::All, 0, 50))
        .unwrap();
    assert_eq!(duplicate_source.total, 0);
    assert_eq!(duplicate_source.duplicates.len(), 2);
    assert_eq!(
        fs::read(fixture.vault.join("a.md")).unwrap(),
        fixture.original.as_bytes()
    );
}

#[test]
fn incomplete_identity_inspection_clears_prior_edges_without_guessing() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let mut app = fixture.app();
    assert_eq!(
        app.relationships(&request(KnowledgeScope::All, 0, 50))
            .unwrap()
            .total,
        3
    );
    let unknown = fixture.vault.join("ordinary.md");
    fs::set_permissions(&unknown, fs::Permissions::from_mode(0o000)).unwrap();
    let incomplete = app
        .relationships(&request(KnowledgeScope::All, 0, 50))
        .unwrap();
    fs::set_permissions(&unknown, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(incomplete.total, 0);
    assert!(!incomplete.issues.is_empty());
    assert_eq!(
        app.relationships(&request(KnowledgeScope::All, 0, 50))
            .unwrap()
            .total,
        3
    );
}

#[test]
fn invalid_managed_classification_cannot_enter_edges_and_other_knowledge_stays_usable() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    assert_eq!(
        app.relationships(&request(KnowledgeScope::All, 0, 50))
            .unwrap()
            .total,
        3
    );
    let invalid = fixture
        .target
        .replace("brn_kind: source", "brn_kind: invented");
    fs::write(fixture.vault.join("archive/source.md"), &invalid).unwrap();
    let page = app
        .relationships(&request(KnowledgeScope::All, 0, 50))
        .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.edges[0].target.path, "c.md");
    assert!(
        page.issues
            .iter()
            .any(|issue| issue.path == "archive/source.md")
    );
    assert_eq!(
        fs::read(fixture.vault.join("archive/source.md")).unwrap(),
        invalid.as_bytes()
    );
    assert_eq!(
        fs::read(fixture.vault.join("a.md")).unwrap(),
        fixture.original.as_bytes()
    );
}

#[test]
fn real_worker_restart_and_index_loss_reconstruct_same_markdown_relationships() {
    let fixture = Fixture::new();
    let mut expected = None;
    for _ in 0..2 {
        let mut worker = AppWorker::start(fixture.data.clone(), fixture.config()).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(20))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let id = Uuid::new_v4();
        worker
            .submit(
                id,
                AppCommand::Relationships(request(KnowledgeScope::All, 0, 50)),
            )
            .unwrap();
        let (actual, result) = worker.recv_event_timeout(Duration::from_secs(20)).unwrap();
        assert_eq!(actual, id);
        let AppEvent::Relationships(page) = result else {
            panic!("expected relationships");
        };
        assert_eq!(page.total, 3);
        if let Some(expected) = &expected {
            assert_eq!(&*page, expected);
        }
        expected = Some(*page);
        worker.shutdown().unwrap();
        fs::remove_file(fixture.data.join("index.sqlite")).unwrap();
    }
}

#[test]
fn invalid_requests_and_uncertain_work_refuse_before_observation() {
    let fixture = Fixture::new();
    let mut app = fixture.app();
    for limit in [0, 201] {
        assert_eq!(
            app.relationships(&request(KnowledgeScope::All, 0, limit))
                .unwrap_err()
                .kind,
            ErrorKind::ToolRejected
        );
    }
    let observed = app.proposal_source("a.md").unwrap();
    let draft = DraftRequest {
        intake: None,
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        action_changes: Vec::new(),
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Synthetic pending replacement".into(),
        changes: vec![DraftNoteChange::Replace {
            path: "a.md".into(),
            expected: observed.source.fingerprint.clone(),
            text: observed.text,
        }],
        sources: vec![observed.source],
    };
    let record = app.create_proposal(&draft).unwrap();
    app.work_store_mut()
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        })
        .unwrap();
    assert_eq!(
        app.relationships(&request(KnowledgeScope::All, 0, 50))
            .unwrap_err()
            .kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        fs::read(fixture.vault.join("a.md")).unwrap(),
        fixture.original.as_bytes()
    );
}
