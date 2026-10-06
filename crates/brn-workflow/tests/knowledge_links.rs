#![cfg(target_os = "macos")]
use brn_workflow::{
    ErrorKind,
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
    knowledge::{IdentityOutcome, NoteLinkOutcome},
    proposal_apply::ApprovalRequest,
    proposals::{DraftNoteChange, DraftRequest},
};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

const SOURCE_ID: &str = "11111111-1111-4111-8111-111111111111";
const TARGET_ID: &str = "22222222-2222-4222-8222-222222222222";
const SOURCE: &str = "folder/current.md";
const TARGET: &str = "archive/tõend.md";

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    credentials: PathBuf,
    original: String,
}
impl Fixture {
    fn new(body: &str) -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        let credentials = owner.path().join("task.credentials");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::create_dir(vault.join("folder")).unwrap();
        fs::create_dir(vault.join("archive")).unwrap();
        let original = format!(
            "\u{feff}---\r\nbrn_id: {SOURCE_ID}\r\ncustom: '[hidden](bad.md)'\r\n...\r\n{body}"
        );
        fs::write(vault.join(SOURCE), &original).unwrap();
        fs::write(
            vault.join(TARGET),
            format!("---\nbrn_id: {TARGET_ID}\nbrn_kind: source\n---\nExact source õ\r\n"),
        )
        .unwrap();
        Self {
            _owner: owner,
            data,
            vault,
            credentials,
            original,
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

#[test]
fn saved_links_resolve_exact_inline_reference_and_uuid_evidence_without_mutation() {
    let body = format!(
        "# Õ\r\n[inline](../archive/t%C3%B5end.md#section)\r\n[ref][Õ]\r\n[stable](brn://note/{TARGET_ID})\r\n\r\n[õ]: ../archive/t%C3%B5end.md\r\n\r\n`[code](bad.md)`\r\n![image](bad.md)\r\n<div><a href=\"bad.md\">HTML</a></div>\r\n"
    );
    let fixture = Fixture::new(&body);
    let app = fixture.app();
    let result = app.note_links(SOURCE).unwrap();
    assert_eq!(result.source.path, SOURCE);
    assert_eq!(result.source.note_id, Some(SOURCE_ID.parse().unwrap()));
    assert_eq!(result.source_outcome, Some(IdentityOutcome::Unique));
    assert!(result.issues.is_empty());
    assert_eq!(result.links.len(), 3);
    for link in &result.links {
        assert_eq!(link.outcome, NoteLinkOutcome::Resolved);
        assert_eq!(link.target_path.as_deref(), Some(TARGET));
        assert_eq!(link.matches.len(), 1);
        assert_eq!(link.matches[0].note_id, Some(TARGET_ID.parse().unwrap()));
        for proof in &link.evidence {
            assert_eq!(
                fixture.original.get(proof.start_byte..proof.end_byte),
                Some(proof.quote.as_str())
            );
            assert!(proof.start_byte >= fixture.original.find("# Õ").unwrap());
        }
    }
    assert_eq!(result.links[1].evidence.len(), 2);
    assert!(result.links[1].evidence[1].quote.starts_with("[õ]:"));
    assert_eq!(
        fs::read(fixture.vault.join(SOURCE)).unwrap(),
        fixture.original.as_bytes()
    );
    assert!(app.work_store().proposals(None).unwrap().is_empty());
    assert_eq!(fs::read_dir(&fixture.credentials).unwrap().count(), 0);
}

#[test]
fn uuid_links_follow_moves_while_path_links_never_guess_and_duplicates_stay_ambiguous() {
    let fixture = Fixture::new(&format!(
        "[path](../archive/t%C3%B5end.md)\n[uuid](brn://note/{TARGET_ID})\n"
    ));
    let app = fixture.app();
    assert_eq!(
        app.note_links(SOURCE).unwrap().links[1].outcome,
        NoteLinkOutcome::Resolved
    );
    fs::rename(fixture.vault.join(TARGET), fixture.vault.join("moved.md")).unwrap();
    let moved = app.note_links(SOURCE).unwrap();
    assert_eq!(moved.links[0].outcome, NoteLinkOutcome::Absent);
    assert_eq!(moved.links[1].outcome, NoteLinkOutcome::Resolved);
    assert_eq!(moved.links[1].target_path.as_deref(), Some("moved.md"));
    fs::copy(
        fixture.vault.join("moved.md"),
        fixture.vault.join("duplicate.md"),
    )
    .unwrap();
    let duplicate = app.note_links(SOURCE).unwrap();
    assert_eq!(duplicate.links[1].outcome, NoteLinkOutcome::Ambiguous);
    assert_eq!(duplicate.links[1].matches.len(), 2);
    assert!(duplicate.links[1].target_path.is_none());
    fs::copy(
        fixture.vault.join(SOURCE),
        fixture.vault.join("duplicate-source.md"),
    )
    .unwrap();
    assert_eq!(
        app.note_links(SOURCE).unwrap().source_outcome,
        Some(IdentityOutcome::Ambiguous)
    );
}

#[test]
fn local_uri_rules_preserve_percent_filenames_and_refuse_escaping_or_external_targets() {
    let fixture = Fixture::new(&format!(
        "[self](#heading)\n[external](https://example.invalid/a.md)\n[network](//example.invalid/a.md)\n[escape](../../outside.md)\n[absolute](/outside.md)\n[invalid](bad%FF.md)\n[separator](..%2Farchive/t%C3%B5end.md)\n[asset](file.pdf)\n[nil](brn://note/00000000-0000-0000-0000-000000000000)\n[plus](<../plus+%23%3F.md?query#heading>)\n[known](brn://note/{TARGET_ID}#heading)\n"
    ));
    fs::copy(fixture.vault.join(TARGET), fixture.vault.join("plus+#?.md")).unwrap();
    // Distinct identity: the plus filename fixture is not a duplicate target.
    let plus = fs::read_to_string(fixture.vault.join("plus+#?.md"))
        .unwrap()
        .replace(TARGET_ID, "33333333-3333-4333-8333-333333333333");
    fs::write(fixture.vault.join("plus+#?.md"), plus).unwrap();
    let result = fixture.app().note_links(SOURCE).unwrap();
    let outcomes: Vec<_> = result.links.iter().map(|link| link.outcome).collect();
    assert_eq!(
        outcomes,
        [
            NoteLinkOutcome::Resolved,
            NoteLinkOutcome::External,
            NoteLinkOutcome::External,
            NoteLinkOutcome::Unsupported,
            NoteLinkOutcome::Unsupported,
            NoteLinkOutcome::Unsupported,
            NoteLinkOutcome::Unsupported,
            NoteLinkOutcome::NonNote,
            NoteLinkOutcome::Unsupported,
            NoteLinkOutcome::Resolved,
            NoteLinkOutcome::Resolved
        ]
    );
    assert_eq!(result.links[0].target_path.as_deref(), Some(SOURCE));
    assert_eq!(result.links[9].target_path.as_deref(), Some("plus+#?.md"));
}

#[test]
fn saved_same_size_edits_and_incomplete_or_unmanaged_identity_are_not_cached_guesses() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new(&format!(
        "[uuid](brn://note/{TARGET_ID})\n[path](../archive/t%C3%B5end.md)\n"
    ));
    let app = fixture.app();
    let before = app.note_links(SOURCE).unwrap();
    let path = fixture.vault.join(TARGET);
    let old = fs::read_to_string(&path).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let replacement = old.replace(TARGET_ID, "33333333-3333-4333-8333-333333333333");
    assert_eq!(old.len(), replacement.len());
    fs::write(&path, replacement).unwrap();
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let fresh = app.note_links(SOURCE).unwrap();
    assert_eq!(fresh.links[0].outcome, NoteLinkOutcome::Absent);
    assert_eq!(fresh.links[1].outcome, NoteLinkOutcome::Resolved);
    assert_ne!(
        fresh.links[1].matches[0].sha256,
        before.links[1].matches[0].sha256
    );
    let unknown = fixture.vault.join("unknown.md");
    fs::write(&unknown, "ordinary unmanaged note").unwrap();
    fs::set_permissions(&unknown, fs::Permissions::from_mode(0o000)).unwrap();
    let incomplete = app.note_links(SOURCE).unwrap();
    fs::set_permissions(&unknown, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(incomplete.source_outcome, Some(IdentityOutcome::Incomplete));
    assert_eq!(incomplete.links[0].outcome, NoteLinkOutcome::Incomplete);
    assert_eq!(incomplete.links[1].outcome, NoteLinkOutcome::Incomplete);
    assert!(!incomplete.issues.is_empty());
    fs::write(&path, "Unmanaged target").unwrap();
    assert_eq!(
        app.note_links(SOURCE).unwrap().links[1].outcome,
        NoteLinkOutcome::Unmanaged
    );
}

#[test]
fn pending_authoritative_changes_fence_link_observation_before_any_effect() {
    let fixture = Fixture::new(&format!("[uuid](brn://note/{TARGET_ID})\n"));
    let mut app = fixture.app();
    let source = app.proposal_source(SOURCE).unwrap();
    let draft = DraftRequest {
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        action_changes: Vec::new(),
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Synthetic pending replacement".into(),
        changes: vec![DraftNoteChange::Replace {
            path: SOURCE.into(),
            expected: source.source.fingerprint.clone(),
            text: source.text,
        }],
        sources: vec![source.source],
    };
    let record = app.create_proposal(&draft).unwrap();
    app.work_store_mut()
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        })
        .unwrap();
    assert_eq!(
        app.note_links(SOURCE).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    assert_eq!(
        fs::read(fixture.vault.join(SOURCE)).unwrap(),
        fixture.original.as_bytes()
    );
}

#[test]
fn actual_worker_rebuilds_links_from_markdown_after_restart_index_loss_and_new_store() {
    let fixture = Fixture::new(&format!("[source](brn://note/{TARGET_ID})\r\n"));
    let mut expected = None;
    let fresh_data = fixture.data.parent().unwrap().join("fresh-data");
    fs::create_dir(&fresh_data).unwrap();
    for data in [&fixture.data, &fixture.data, &fresh_data] {
        let mut worker = AppWorker::start(data.clone(), fixture.config()).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(20))
                .unwrap()
                .1,
            AppEvent::Ready {
                vault_bound: true,
                model_installed: false
            }
        ));
        let id = Uuid::new_v4();
        worker
            .submit(id, AppCommand::NoteLinks(SOURCE.into()))
            .unwrap();
        let (actual, result) = worker.recv_event_timeout(Duration::from_secs(20)).unwrap();
        assert_eq!(actual, id);
        let AppEvent::NoteLinks(links) = result else {
            panic!("expected saved links")
        };
        assert_eq!(links.links[0].outcome, NoteLinkOutcome::Resolved);
        if let Some(expected) = &expected {
            assert_eq!(&*links, expected);
        }
        expected = Some(*links);
        worker.shutdown().unwrap();
        fs::remove_file(data.join("index.sqlite")).unwrap();
    }
    assert_eq!(
        fs::read(fixture.vault.join(SOURCE)).unwrap(),
        fixture.original.as_bytes()
    );
}

#[test]
fn leading_ordinary_thematic_break_does_not_turn_links_into_incomplete_frontmatter() {
    let fixture = Fixture::new("");
    let ordinary = "---\n[ordinary](../archive/t%C3%B5end.md)\n";
    fs::write(fixture.vault.join(SOURCE), ordinary).unwrap();
    let result = fixture.app().note_links(SOURCE).unwrap();
    assert!(result.source.note_id.is_none());
    assert!(result.source_outcome.is_none());
    assert_eq!(result.links.len(), 1);
    assert_eq!(result.links[0].outcome, NoteLinkOutcome::Resolved);
    assert_eq!(
        result.links[0].evidence[0].quote,
        "[ordinary](../archive/t%C3%B5end.md)"
    );
    assert_eq!(
        fs::read(fixture.vault.join(SOURCE)).unwrap(),
        ordinary.as_bytes()
    );
}
