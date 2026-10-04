//! Private qualification: real App/checked Store and disposable exact Markdown.
use super::*;
use crate::{
    app::AppConfig,
    proposal_apply::{ApplyOutcome, ApprovalRequest},
    proposals::{ActionChange, NoteChange, ProposalDraft},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

struct Fixture {
    _base: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    credentials: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = base.path().join("data");
        let vault = base.path().join("vault");
        let credentials = base.path().join("credentials");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        Self {
            _base: base,
            data,
            vault,
            credentials,
        }
    }
    fn app(&self, vault: bool) -> App {
        App::open(
            &self.data,
            AppConfig {
                vault_root: vault.then(|| self.vault.clone()),
                credentials_dir: Some(self.credentials.clone()),
                model_dir: None,
            },
        )
        .unwrap()
    }
    fn note(&self, path: &str, id: Uuid, metadata: &str) {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(self.vault.join(parent)).unwrap();
        }
        fs::write(
            self.vault.join(path),
            format!("---\nbrn_id: {id}\n{metadata}---\nExact synthetic õ\r\n"),
        )
        .unwrap();
    }
    fn quiet(&self, app: &App) {
        assert!(app.selection().unwrap().is_none());
        assert!(app.store.conversations().unwrap().is_empty());
        assert_eq!(fs::read_dir(&self.credentials).unwrap().count(), 0);
    }
}
fn data() -> ActionData {
    ActionData {
        title: "Synthetic action".into(),
        description: String::new(),
        state: ActionState::Open,
        owner: None,
        related_person: None,
        related_project: None,
        sources: Vec::new(),
        thread: None,
        due_on: None,
        follow_up_on: None,
        dependencies: Vec::new(),
        parent: None,
        follows_up: None,
        priority: None,
    }
}
fn create(id: Uuid) -> ActionChange {
    ActionChange::Create { id, data: data() }
}
fn draft(changes: Vec<ActionChange>) -> ProposalDraft {
    ProposalDraft {
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: None,
        title: "Private validation".into(),
        changes: Vec::new(),
        sources: Vec::new(),
        action_changes: changes,
    }
}
fn seed(app: &mut App, changes: Vec<ActionChange>) {
    let record = app.store.create_proposal(&draft(changes)).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    app.store.begin_proposal_apply(&request).unwrap();
    app.store
        .record_proposal_prepared(request.operation_id, &[])
        .unwrap();
    app.store
        .finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&[]))
        .unwrap();
}
fn replace(app: &App, id: Uuid) -> ActionChange {
    let before = app.store.action(id).unwrap().unwrap();
    ActionChange::Replace {
        data: before.data.clone(),
        before: Box::new(before),
    }
}
fn note_change(app: &mut App, path: &str, text: Option<String>) -> NoteChange {
    let files = app.editor_files().unwrap();
    let observed = files.observe(Path::new(path)).unwrap();
    let parent = files.parent_identity(Path::new(path)).unwrap();
    match text {
        Some(text) => NoteChange::Replace {
            path: path.into(),
            parent,
            before: observed.fingerprint,
            before_text: observed.text,
            text,
        },
        None => NoteChange::Trash {
            path: path.into(),
            parent,
            before: observed.fingerprint,
            before_text: observed.text,
        },
    }
}
#[test]
fn vaultless_same_draft_and_existing_action_targets_pass_without_side_effects() {
    let f = Fixture::new();
    let mut app = f.app(false);
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    seed(&mut app, vec![create(a)]);
    let mut changed = create(b);
    changed.data_mut().dependencies = vec![a, c];
    changed.data_mut().parent = Some(a);
    changed.data_mut().follows_up = Some(c);
    assert!(
        app.validate_action_references(&draft(vec![changed, create(c)]))
            .is_ok()
    );
    assert!(app.store.action(b).unwrap().is_none());
    assert!(fs::read_dir(&f.vault).unwrap().next().is_none());
    f.quiet(&app);
}
#[test]
fn missing_action_slots_and_reachable_missing_dependency_refuse() {
    let f = Fixture::new();
    let mut app = f.app(false);
    for slot in 0..3 {
        let mut c = create(Uuid::new_v4());
        let missing = Uuid::new_v4();
        match slot {
            0 => c.data_mut().dependencies = vec![missing],
            1 => c.data_mut().parent = Some(missing),
            _ => c.data_mut().follows_up = Some(missing),
        };
        assert!(
            app.validate_action_references(&draft(vec![c])).is_err(),
            "slot {slot}"
        );
    }
    let id = Uuid::new_v4();
    let mut broken = create(id);
    broken.data_mut().dependencies = vec![Uuid::new_v4()];
    seed(&mut app, vec![broken]);
    let mut c = create(Uuid::new_v4());
    c.data_mut().dependencies = vec![id];
    assert!(app.validate_action_references(&draft(vec![c])).is_err());
    assert!(
        app.validate_action_references(&draft(vec![create(Uuid::new_v4())]))
            .is_ok()
    );
    f.quiet(&app);
}
#[test]
fn malformed_nil_duplicate_and_self_action_references_refuse() {
    let f = Fixture::new();
    let mut app = f.app(false);
    let id = Uuid::new_v4();
    let mut invalid = Vec::new();
    invalid.push(create(Uuid::nil()));
    for slot in 0..7 {
        let mut c = create(id);
        match slot {
            0 => c.data_mut().title.clear(),
            1 => c.data_mut().dependencies = vec![id],
            2 => c.data_mut().parent = Some(id),
            3 => c.data_mut().follows_up = Some(id),
            4 => c.data_mut().sources = vec![Uuid::nil()],
            5 => c.data_mut().sources = vec![id, id],
            _ => c.data_mut().dependencies = vec![Uuid::nil()],
        };
        invalid.push(c);
    }
    for c in invalid {
        assert!(app.validate_action_references(&draft(vec![c])).is_err());
    }
    assert!(
        app.validate_action_references(&draft(vec![create(id), create(id)]))
            .is_err()
    );
    let mut c = create(id);
    let target = Uuid::new_v4();
    c.data_mut().dependencies = vec![target, target];
    assert!(
        app.validate_action_references(&draft(vec![c, create(target)]))
            .is_err()
    );
}
#[test]
fn two_three_node_and_existing_dependency_cycles_refuse() {
    let f = Fixture::new();
    let mut app = f.app(false);
    for count in [2, 3] {
        let ids: Vec<_> = (0..count).map(|_| Uuid::new_v4()).collect();
        let changes: Vec<_> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                let mut c = create(*id);
                c.data_mut().dependencies = vec![ids[(i + 1) % count]];
                c
            })
            .collect();
        assert!(
            app.validate_action_references(&draft(changes.clone()))
                .is_err()
        );
        seed(&mut app, changes);
        let mut root = create(Uuid::new_v4());
        root.data_mut().dependencies = vec![ids[0]];
        assert!(app.validate_action_references(&draft(vec![root])).is_err());
    }
}
#[test]
fn parent_hierarchy_is_acyclic_separately_from_dependencies_and_followups() {
    let f = Fixture::new();
    let mut app = f.app(false);
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let mut ca = create(a);
    let mut cb = create(b);
    let mut cc = create(c);
    ca.data_mut().parent = Some(b);
    cb.data_mut().parent = Some(c);
    cc.data_mut().parent = Some(a);
    assert!(
        app.validate_action_references(&draft(vec![ca.clone(), cb.clone(), cc]))
            .is_err()
    );
    // The union has a cycle, but each governed graph is acyclic.
    cb.data_mut().parent = None;
    cb.data_mut().dependencies = vec![a];
    ca.data_mut().follows_up = Some(b);
    cb.data_mut().follows_up = Some(a);
    assert!(app.validate_action_references(&draft(vec![ca, cb])).is_ok());
}
#[test]
fn existing_parent_cycle_refuses_and_replace_overlay_can_break_existing_cycles() {
    let f = Fixture::new();
    let mut app = f.app(false);
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut ca = create(a);
    let mut cb = create(b);
    ca.data_mut().parent = Some(b);
    cb.data_mut().parent = Some(a);
    ca.data_mut().dependencies = vec![b];
    cb.data_mut().dependencies = vec![a];
    seed(&mut app, vec![ca, cb]);
    let mut root = create(Uuid::new_v4());
    root.data_mut().parent = Some(a);
    assert!(app.validate_action_references(&draft(vec![root])).is_err());
    let mut changed = replace(&app, b);
    changed.data_mut().parent = None;
    changed.data_mut().dependencies.clear();
    assert!(
        app.validate_action_references(&draft(vec![replace(&app, a), changed]))
            .is_ok()
    );
}
#[test]
fn long_retained_chain_uses_iterative_traversal() {
    let f = Fixture::new();
    let mut app = f.app(false);
    let mut next = None;
    for _ in 0..16 {
        let mut batch = Vec::new();
        for _ in 0..32 {
            let id = Uuid::new_v4();
            let mut c = create(id);
            if let Some(next) = next {
                c.data_mut().dependencies = vec![next];
                c.data_mut().parent = Some(next);
            }
            batch.push(c);
            next = Some(id);
        }
        seed(&mut app, batch);
    }
    let mut root = create(Uuid::new_v4());
    root.data_mut().dependencies = vec![next.unwrap()];
    root.data_mut().parent = next;
    assert!(app.validate_action_references(&draft(vec![root])).is_ok());
    f.quiet(&app);
}
#[test]
fn completed_followup_target_is_allowed() {
    let f = Fixture::new();
    let mut app = f.app(false);
    let id = Uuid::new_v4();
    seed(&mut app, vec![create(id)]);
    let mut completed = app.store.action(id).unwrap().unwrap();
    completed.version = 2;
    completed.data.state = ActionState::Completed;
    completed.completed_at_ms = Some(completed.updated_at_ms);
    completed.validate().unwrap();
    // Private synthetic SQL fixture supplies a valid Completed baseline to the
    // checked Store reader without opening a production completion command.
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(&completed).unwrap();
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    let conn = rusqlite::Connection::open(f.data.join("brn.sqlite")).unwrap();
    conn.execute("UPDATE actions SET version=?2,state='completed',record_json=?3,record_sha256=?4 WHERE id=?1", rusqlite::params![id.to_string(), completed.version as i64, bytes, digest.as_slice()]).unwrap();
    assert_eq!(app.store.action(id).unwrap(), Some(completed));
    let mut c = create(Uuid::new_v4());
    c.data_mut().follows_up = Some(id);
    assert!(app.validate_action_references(&draft(vec![c])).is_ok());
}
#[test]
fn newly_introduced_knowledge_slots_need_exact_sources_and_source_drift_refuses() {
    let f = Fixture::new();
    let id = Uuid::new_v4();
    f.note("archive/source.md", id, "brn_kind: source\n");
    let mut app = f.app(true);
    let source = app
        .proposal_evidence_source("archive/source.md")
        .unwrap()
        .source;
    for slot in 0..4 {
        let mut c = create(Uuid::new_v4());
        match slot {
            0 => c.data_mut().related_person = Some(id),
            1 => c.data_mut().related_project = Some(id),
            2 => c.data_mut().thread = Some(id),
            _ => c.data_mut().sources = vec![id],
        };
        let mut d = draft(vec![c]);
        assert!(app.validate_action_references(&d).is_err(), "slot {slot}");
        d.sources = vec![source.clone()];
        assert!(app.validate_action_references(&d).is_ok());
        fs::write(f.vault.join("archive/source.md"), "changed").unwrap();
        assert!(app.validate_action_references(&d).is_err());
        f.note("archive/source.md", id, "brn_kind: source\n");
    }
    f.quiet(&app);
}
#[test]
fn unchanged_historical_refs_are_preserved_but_moving_a_ref_between_fields_needs_binding() {
    let f = Fixture::new();
    let mut app = f.app(false);
    let id = Uuid::new_v4();
    let absent = Uuid::new_v4();
    let mut c = create(id);
    c.data_mut().related_person = Some(absent);
    c.data_mut().related_project = Some(absent);
    c.data_mut().thread = Some(absent);
    c.data_mut().sources = vec![absent];
    seed(&mut app, vec![c]);
    let mut changed = replace(&app, id);
    changed.data_mut().description = "Edited".into();
    assert!(
        app.validate_action_references(&draft(vec![changed.clone()]))
            .is_ok()
    );
    changed.data_mut().thread = Some(Uuid::new_v4());
    assert!(
        app.validate_action_references(&draft(vec![changed]))
            .is_err()
    );
    let other = Uuid::new_v4();
    let mut c = create(other);
    c.data_mut().related_person = Some(absent);
    seed(&mut app, vec![c]);
    let mut changed = replace(&app, other);
    changed.data_mut().related_project = Some(absent);
    assert!(
        app.validate_action_references(&draft(vec![changed]))
            .is_err()
    );
    f.quiet(&app);
}
#[test]
fn same_draft_managed_create_replace_and_trash_overlay_have_exact_authority() {
    let f = Fixture::new();
    let id = Uuid::new_v4();
    f.note("target.md", id, "");
    let mut app = f.app(true);
    let mut c = create(Uuid::new_v4());
    c.data_mut().sources = vec![id];
    let mut d = draft(vec![c]);
    let text = app.proposal_evidence_source("target.md").unwrap().text;
    d.changes = vec![note_change(
        &mut app,
        "target.md",
        Some(format!("{text}Reviewed new bytes")),
    )];
    assert!(app.validate_action_references(&d).is_ok());
    d.changes = vec![note_change(&mut app, "target.md", None)];
    assert!(app.validate_action_references(&d).is_err());
    let parent = app
        .editor_files()
        .unwrap()
        .parent_identity(Path::new("new.md"))
        .unwrap();
    d.changes = vec![
        note_change(&mut app, "target.md", None),
        NoteChange::Create {
            path: "new.md".into(),
            parent: parent.clone(),
            text: text.clone(),
        },
    ];
    assert!(app.validate_action_references(&d).is_ok());
    d.changes.push(NoteChange::Create {
        path: "duplicate.md".into(),
        parent,
        text,
    });
    assert!(app.validate_action_references(&d).is_err());
    f.quiet(&app);
}
#[test]
fn incomplete_duplicate_and_opaque_managed_targets_refuse_without_vault_writes() {
    let f = Fixture::new();
    let id = Uuid::new_v4();
    f.note("target.md", id, "");
    let mut app = f.app(true);
    let mut c = create(Uuid::new_v4());
    c.data_mut().sources = vec![id];
    let mut d = draft(vec![c]);
    d.sources = vec![app.proposal_evidence_source("target.md").unwrap().source];
    f.note("duplicate.md", id, "");
    assert!(app.validate_action_references(&d).is_err());
    fs::remove_file(f.vault.join("duplicate.md")).unwrap();
    fs::write(f.vault.join("broken.md"), b"\xff").unwrap();
    assert!(app.validate_action_references(&d).is_err());
    fs::remove_file(f.vault.join("broken.md")).unwrap();
    f.note("target.md", id, "brn_kind: [source]\n");
    d.sources = vec![app.proposal_evidence_source("target.md").unwrap().source];
    assert!(app.validate_action_references(&d).is_err());
    let before = fs::read(f.vault.join("target.md")).unwrap();
    assert!(app.validate_action_references(&d).is_err());
    assert_eq!(fs::read(f.vault.join("target.md")).unwrap(), before);
    f.quiet(&app);
}

#[test]
fn source_set_reordering_and_removal_need_no_vault_but_new_sources_do() {
    let f = Fixture::new();
    let mut app = f.app(false);
    let id = Uuid::new_v4();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut c = create(id);
    c.data_mut().sources = vec![a, b];
    seed(&mut app, vec![c]);
    let mut changed = replace(&app, id);
    changed.data_mut().sources = vec![b, a];
    assert!(
        app.validate_action_references(&draft(vec![changed.clone()]))
            .is_ok()
    );
    changed.data_mut().sources = vec![b];
    assert!(
        app.validate_action_references(&draft(vec![changed.clone()]))
            .is_ok()
    );
    changed.data_mut().sources.push(Uuid::new_v4());
    assert!(
        app.validate_action_references(&draft(vec![changed]))
            .is_err()
    );
    f.quiet(&app);
}

#[test]
fn same_draft_target_needs_managed_metadata_and_exact_observed_before() {
    let f = Fixture::new();
    let id = Uuid::new_v4();
    f.note("target.md", id, "");
    let mut app = f.app(true);
    let mut c = create(Uuid::new_v4());
    c.data_mut().related_project = Some(id);
    let text = app.proposal_evidence_source("target.md").unwrap().text;
    let mut d = draft(vec![c]);
    d.changes = vec![note_change(
        &mut app,
        "target.md",
        Some(text.replace("---\nExact", "brn_state: invalid\n---\nExact")),
    )];
    assert!(app.validate_action_references(&d).is_err());
    d.changes = vec![note_change(&mut app, "target.md", Some(text.clone()))];
    if let NoteChange::Replace { before, .. } = &mut d.changes[0] {
        before.sha256 = [0; 32];
    }
    assert!(app.validate_action_references(&d).is_err());
    d.changes = vec![note_change(&mut app, "target.md", Some(text))];
    assert!(app.validate_action_references(&d).is_ok());
    f.quiet(&app);
}

#[test]
fn empty_knowledge_reference_set_does_not_inspect_incomplete_vault() {
    let f = Fixture::new();
    let mut app = f.app(true);
    fs::write(f.vault.join("unreadable.md"), b"\xff").unwrap();
    assert!(
        app.validate_action_references(&draft(vec![create(Uuid::new_v4())]))
            .is_ok()
    );
    assert!(
        app.validate_proposal_note_targets(&draft(Vec::new()), &std::collections::BTreeSet::new())
            .is_ok()
    );
    assert_eq!(fs::read(f.vault.join("unreadable.md")).unwrap(), b"\xff");
    f.quiet(&app);
}

#[test]
fn shared_dependency_descendant_is_valid_and_checked_store_corruption_refuses() {
    let f = Fixture::new();
    let mut app = f.app(false);
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let d = Uuid::new_v4();
    seed(&mut app, vec![create(d)]);
    let mut ca = create(a);
    ca.data_mut().dependencies = vec![b, c];
    let mut cb = create(b);
    cb.data_mut().dependencies = vec![d];
    let mut cc = create(c);
    cc.data_mut().dependencies = vec![d];
    let proposal = draft(vec![ca, cb, cc]);
    assert!(app.validate_action_references(&proposal).is_ok());
    let conn = rusqlite::Connection::open(f.data.join("brn.sqlite")).unwrap();
    conn.execute(
        "UPDATE actions SET record_sha256=zeroblob(32) WHERE id=?1",
        [d.to_string()],
    )
    .unwrap();
    assert!(app.validate_action_references(&proposal).is_err());
    f.quiet(&app);
}
