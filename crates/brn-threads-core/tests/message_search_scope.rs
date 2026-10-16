//! Owner search shows saved chat Messages where the chat's notes would
//! appear: general chats in general search, project chats only in their
//! project, private chats only in themselves. An archived or legacy Resolved
//! chat keeps that scope for its Messages (owner decision 3), as its
//! chat-only notes already did under the unchanged context predicate.
//! Trashed chats are hidden until restored, and partial progress is never
//! searchable. Synthetic data only.
use brn_threads_core::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

struct World {
    _root: tempfile::TempDir,
    data: PathBuf,
    store: Store,
}

fn world() -> World {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let store = Store::open(&data).unwrap();
    World {
        _root: root,
        data,
        store,
    }
}

/// One owner change: new records, or new versions of `prior` ones.
fn write(
    store: &mut Store,
    key: &str,
    puts: Vec<(Option<&Record>, RecordData, bool)>,
) -> Vec<Record> {
    let op = store.allocate_operation(key).unwrap();
    let owner = HostAuthority::owner("synthetic fixture");
    let writes = puts
        .into_iter()
        .enumerate()
        .map(|(i, (prior, data, archived))| Put {
            id: prior.map_or_else(|| op.creation_id(i), |r| r.id.clone()),
            expected_version: prior.map(|r| r.version),
            archived,
            data,
        })
        .collect();
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: key.into(),
                writes,
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    let ApplyOutcome::Applied(receipt) = store.apply(&op, &owner).unwrap() else {
        panic!("synthetic write did not apply")
    };
    receipt.writes.into_iter().map(|w| w.after).collect()
}

fn thread(title: &str) -> RecordData {
    RecordData::Thread(Thread {
        private_context: false,
        context_epoch: 0,
        title: title.into(),
        state: ThreadState::Open,
        attention: vec![],
    })
}

fn chat(store: &mut Store, title: &str) -> String {
    write(store, title, vec![(None, thread(title), false)])[0]
        .id
        .clone()
}

fn message(store: &mut Store, chat: &str, role: &str, text: &str) -> String {
    let data = RecordData::Message(Message {
        thread: chat.into(),
        role: role.into(),
        text: text.into(),
    });
    write(store, &format!("{chat}:{text}"), vec![(None, data, false)])[0]
        .id
        .clone()
}

/// An owner project command, bound to its exact content as the app binds it.
fn project_command(store: &mut Store, command: ProjectCommand) -> Receipt {
    use sha2::{Digest, Sha256};
    let binding = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&command).unwrap())
    );
    // Every command here is distinct, so its content is a unique key.
    let op = store
        .allocate_bound_operation(&format!("project:{binding}"), &binding)
        .unwrap();
    store
        .project_command(&op, &command, &HostAuthority::owner("owner"))
        .unwrap()
}

fn project(store: &mut Store, title: &str) -> String {
    project_command(
        store,
        ProjectCommand::Create {
            title: title.into(),
        },
    )
    .writes
    .into_iter()
    .find(|w| matches!(w.after.data, RecordData::Project(_)))
    .unwrap()
    .after
    .id
}

fn project_chat(store: &mut Store, project: &str, title: &str) -> String {
    project_command(
        store,
        ProjectCommand::CreateChat {
            project: project.into(),
            title: title.into(),
        },
    )
    .writes
    .into_iter()
    .find(|w| w.before.is_none() && matches!(w.after.data, RecordData::Thread(_)))
    .unwrap()
    .after
    .id
}

fn knowledge(store: &mut Store, project: &str, enabled: bool) {
    let record = store.record(project).unwrap().unwrap();
    let RecordData::Project(p) = record.data else {
        panic!("Project")
    };
    project_command(
        store,
        ProjectCommand::Configure {
            project: project.into(),
            version: record.version,
            title: p.title,
            instructions: p.instructions,
            use_brn_knowledge: enabled,
        },
    );
}

/// The ids owner search may show in `context`, from a fresh read-only snapshot.
fn searchable(data: &Path, ids: &[&str], context: Option<&str>) -> BTreeSet<String> {
    let ids: Vec<String> = ids.iter().map(|id| (*id).to_owned()).collect();
    let mut reader = Store::open_reader(data).unwrap();
    let snapshot = reader.snapshot().unwrap();
    snapshot.searchable_messages(&ids, context).unwrap()
}

fn set(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

fn run_data(chat: &str, state: RunState, fence: u64) -> RecordData {
    RecordData::Run(Run {
        instruction: None,
        queue_claim: None,
        thread: chat.into(),
        provider: "Codex".into(),
        model: "synthetic".into(),
        guide_identity: "synthetic".into(),
        budget: 4,
        effort: "medium".into(),
        loaded_skills: vec![],
        fence,
        state,
        progress: "retained synthetic progress".into(),
        operations: vec![],
        sources: vec![],
    })
}

fn assert_run(store: &Store, id: &str, state: RunState, fence: u64, archived: bool) {
    let record = store.record(id).unwrap().unwrap();
    assert_eq!(record.archived, archived);
    assert!(
        matches!(record.data, RecordData::Run(run) if run.state == state && run.fence == fence)
    );
}

#[test]
fn project_delete_advances_only_actual_terminal_and_live_archive_transitions_once() {
    for state in [
        RunState::Interrupted,
        RunState::Completed,
        RunState::Cancelled,
        RunState::Failed,
    ] {
        let mut w = world();
        let p = project(&mut w.store, "Run fences");
        let c = project_chat(&mut w.store, &p, "Conversation");
        let runs = write(
            &mut w.store,
            "mixed runs",
            vec![
                (None, run_data(&c, state.clone(), 7), false),
                (None, run_data(&c, RunState::Working, 11), false),
                // Already archived at MAX: deletion/restore must neither advance
                // this fence nor turn a no-op into an overflow failure.
                (None, run_data(&c, RunState::Completed, u64::MAX), true),
            ],
        );
        let delete = ProjectCommand::Delete {
            preview: w.store.project_delete_preview(&p).unwrap(),
            actions: BTreeMap::new(),
        };
        let receipt = project_command(&mut w.store, delete.clone());
        assert_run(&w.store, &runs[0].id, state.clone(), 8, true);
        assert_run(&w.store, &runs[1].id, RunState::Interrupted, 12, true);
        assert_run(&w.store, &runs[2].id, RunState::Completed, u64::MAX, true);
        let frozen = w.store.records().unwrap();
        assert_eq!(project_command(&mut w.store, delete), receipt);
        assert_eq!(
            w.store.records().unwrap(),
            frozen,
            "replay cannot advance any fence again"
        );
        let deleted = w.store.record(&p).unwrap().unwrap();
        project_command(
            &mut w.store,
            ProjectCommand::Restore {
                project: p,
                version: deleted.version,
            },
        );
        assert_run(&w.store, &runs[0].id, state, 9, false);
        assert_run(&w.store, &runs[1].id, RunState::Interrupted, 13, false);
        assert_run(&w.store, &runs[2].id, RunState::Completed, u64::MAX, true);
    }
}

#[test]
fn project_delete_terminal_fence_overflow_refuses_atomically() {
    let mut w = world();
    let p = project(&mut w.store, "Overflow");
    let c = project_chat(&mut w.store, &p, "Conversation");
    let runs = write(
        &mut w.store,
        "overflow runs",
        vec![
            (None, run_data(&c, RunState::Failed, u64::MAX), false),
            (None, run_data(&c, RunState::Working, 1), false),
        ],
    );
    let command = ProjectCommand::Delete {
        preview: w.store.project_delete_preview(&p).unwrap(),
        actions: BTreeMap::new(),
    };
    let before = w.store.records().unwrap();
    use sha2::{Digest, Sha256};
    let binding = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&command).unwrap())
    );
    let op = w
        .store
        .allocate_bound_operation("overflow deletion", &binding)
        .unwrap();
    let error = w
        .store
        .project_command(&op, &command, &HostAuthority::owner("synthetic fixture"))
        .unwrap_err();
    assert!(error.to_string().contains("run fence overflow"), "{error}");
    assert_eq!(
        w.store.records().unwrap(),
        before,
        "no member can be partly archived"
    );
    assert!(w.store.receipt(&op).unwrap().is_none());
    assert_run(&w.store, &runs[0].id, RunState::Failed, u64::MAX, false);
    assert_run(&w.store, &runs[1].id, RunState::Working, 1, false);
}

fn scope(data: &Path, chat: &str) -> MessageScope {
    let mut reader = Store::open_reader(data).unwrap();
    let snapshot = reader.snapshot().unwrap();
    let rows = snapshot.message_scope_rows(&[chat.to_owned()]).unwrap();
    let thread = rows.iter().find(|r| r.id == chat).unwrap();
    message_scope(thread, &rows)
}

fn scopes(data: &Path, context: Option<&str>) -> Vec<MessageScope> {
    let mut reader = Store::open_reader(data).unwrap();
    reader.snapshot().unwrap().message_scopes(context).unwrap()
}

#[test]
fn a_general_chat_is_searchable_wherever_general_notes_appear() {
    let mut w = world();
    let general = chat(&mut w.store, "Harbour replies");
    let other = chat(&mut w.store, "Other");
    let p = project(&mut w.store, "Crane");
    let m = message(&mut w.store, &general, "assistant", "Berth survey reply");
    assert_eq!(scope(&w.data, &general), MessageScope::General);
    assert!(!w.store.chat_has_private_context(&general).unwrap());
    for context in [None, Some(general.as_str()), Some(&other), Some(&p)] {
        assert_eq!(
            searchable(&w.data, &[&m], context),
            set(&[&m]),
            "{context:?}"
        );
    }
    assert_eq!(scopes(&w.data, None), [MessageScope::General]);
    assert_eq!(
        scopes(&w.data, Some(&other)),
        [MessageScope::General, MessageScope::Chat(other.clone())]
    );
}

#[test]
fn a_project_chat_is_searchable_only_in_its_project() {
    let mut w = world();
    let p = project(&mut w.store, "Crane");
    let pc = project_chat(&mut w.store, &p, "Crane chat");
    let sibling = project_chat(&mut w.store, &p, "Crane sibling");
    let q = project(&mut w.store, "Garden");
    let qc = project_chat(&mut w.store, &q, "Garden chat");
    let general = chat(&mut w.store, "General");
    let m = message(&mut w.store, &pc, "owner", "Berth survey question");
    assert_eq!(scope(&w.data, &pc), MessageScope::Project(p.clone()));
    assert!(w.store.chat_has_private_context(&pc).unwrap());
    for context in [Some(p.as_str()), Some(&pc), Some(&sibling)] {
        assert_eq!(
            searchable(&w.data, &[&m], context),
            set(&[&m]),
            "{context:?}"
        );
    }
    for context in [None, Some(q.as_str()), Some(&qc), Some(&general)] {
        assert!(
            searchable(&w.data, &[&m], context).is_empty(),
            "{context:?}"
        );
    }
    assert_eq!(
        scopes(&w.data, Some(&pc)),
        [MessageScope::Project(p.clone()), MessageScope::General]
    );
}

#[test]
fn a_project_without_brn_knowledge_shows_only_its_own_chats() {
    let mut w = world();
    let p = project(&mut w.store, "Crane");
    let pc = project_chat(&mut w.store, &p, "Crane chat");
    let general = chat(&mut w.store, "General");
    let mg = message(&mut w.store, &general, "owner", "Berth survey general");
    let mp = message(&mut w.store, &pc, "owner", "Berth survey project");
    assert_eq!(searchable(&w.data, &[&mg, &mp], Some(&p)), set(&[&mg, &mp]));
    knowledge(&mut w.store, &p, false);
    for context in [Some(p.as_str()), Some(&pc)] {
        assert_eq!(searchable(&w.data, &[&mg, &mp], context), set(&[&mp]));
    }
    assert_eq!(
        scopes(&w.data, Some(&p)),
        [MessageScope::Project(p.clone())]
    );
    // General search is unaffected.
    assert_eq!(searchable(&w.data, &[&mg, &mp], None), set(&[&mg]));
}

/// Saves chat `id` archived or not, and Open or legacy Resolved.
fn set_chat(store: &mut Store, id: &str, archived: bool, state: ThreadState) {
    let record = store.record(id).unwrap().unwrap();
    let RecordData::Thread(mut thread) = record.data.clone() else {
        panic!("Thread")
    };
    thread.state = state;
    write(
        store,
        &format!("{id}:{archived}:{:?}:{}", thread.state, record.version),
        vec![(Some(&record), RecordData::Thread(thread), archived)],
    );
}

fn chat_note(store: &mut Store, chat: &str, title: &str) -> String {
    let data = RecordData::Note(Note {
        chat: Some(chat.into()),
        ..Note::working(title, "Berth survey depth 1.4 m.")
    });
    write(store, title, vec![(None, data, false)])[0].id.clone()
}

/// The one note re-check owner and agent search share in `context`.
fn notes(data: &Path, ids: &[&str], context: Option<&str>) -> BTreeSet<String> {
    let ids: Vec<String> = ids.iter().map(|id| (*id).to_owned()).collect();
    let mut reader = Store::open_reader(data).unwrap();
    let snapshot = reader.snapshot().unwrap();
    snapshot.searchable_notes(&ids, context).unwrap()
}

#[test]
fn an_archived_or_resolved_chat_keeps_its_message_scope() {
    let mut w = world();
    let general = chat(&mut w.store, "Replies");
    let m = message(&mut w.store, &general, "assistant", "Berth survey reply");
    let p = project(&mut w.store, "Crane");
    let pc = project_chat(&mut w.store, &p, "Crane chat");
    let mp = message(&mut w.store, &pc, "owner", "Berth survey question");
    let label = |data: &Path, chat: &str| {
        let mut reader = Store::open_reader(data).unwrap();
        let snapshot = reader.snapshot().unwrap();
        chat_is_archived(&snapshot.record(chat).unwrap().unwrap())
    };
    for (archived, state) in [
        (true, ThreadState::Open),
        (false, ThreadState::Resolved),
        (true, ThreadState::Resolved),
    ] {
        set_chat(&mut w.store, &general, archived, state.clone());
        set_chat(&mut w.store, &pc, archived, state.clone());
        // Before: an archived chat was `Hidden` everywhere.
        assert_eq!(scope(&w.data, &general), MessageScope::General);
        assert_eq!(scope(&w.data, &pc), MessageScope::Project(p.clone()));
        for context in [None, Some(general.as_str())] {
            assert_eq!(searchable(&w.data, &[&m], context), set(&[&m]));
        }
        for context in [Some(p.as_str()), Some(&pc)] {
            assert_eq!(searchable(&w.data, &[&mp], context), set(&[&mp]));
        }
        assert!(searchable(&w.data, &[&mp], None).is_empty());
        assert!(
            label(&w.data, &general) && label(&w.data, &pc),
            "{archived} {state:?}"
        );
    }
    set_chat(&mut w.store, &general, false, ThreadState::Open);
    assert_eq!(scope(&w.data, &general), MessageScope::General);
    assert!(!label(&w.data, &general));
    assert_eq!(searchable(&w.data, &[&m], None), set(&[&m]));
}

/// The decision 3 caveat check, as a test: a project chat's chat-only note
/// is a project document (`project_document_ids`), so the one context
/// predicate admits it throughout its project whether its chat is open,
/// archived or legacy Resolved, and nowhere else. Nothing about the chat's
/// state excluded it, and this is unchanged.
#[test]
fn an_archived_project_chats_notes_stay_project_documents() {
    let mut w = world();
    let p = project(&mut w.store, "Crane");
    let pc = project_chat(&mut w.store, &p, "Crane chat");
    let sibling = project_chat(&mut w.store, &p, "Crane sibling");
    let q = project(&mut w.store, "Garden");
    let qc = project_chat(&mut w.store, &q, "Garden chat");
    let general = chat(&mut w.store, "General");
    let n = chat_note(&mut w.store, &pc, "Crane berth attachment");
    for (archived, state) in [
        (false, ThreadState::Open),
        (true, ThreadState::Open),
        (false, ThreadState::Resolved),
    ] {
        set_chat(&mut w.store, &pc, archived, state.clone());
        for context in [Some(p.as_str()), Some(&sibling), Some(&pc)] {
            assert_eq!(
                notes(&w.data, &[&n], context),
                set(&[&n]),
                "{archived} {state:?} {context:?}"
            );
        }
        for context in [None, Some(q.as_str()), Some(&qc), Some(&general)] {
            assert!(notes(&w.data, &[&n], context).is_empty(), "{context:?}");
        }
    }
}

#[test]
fn an_archived_general_chats_notes_stay_private_to_it() {
    let mut w = world();
    let private = chat(&mut w.store, "Attachments");
    let other = chat(&mut w.store, "Other");
    let p = project(&mut w.store, "Crane");
    let n = chat_note(&mut w.store, &private, "Survey attachment");
    set_chat(&mut w.store, &private, true, ThreadState::Open);
    assert_eq!(notes(&w.data, &[&n], Some(&private)), set(&[&n]));
    for context in [None, Some(other.as_str()), Some(&p)] {
        assert!(notes(&w.data, &[&n], context).is_empty(), "{context:?}");
    }
}

#[test]
fn a_trashed_chats_notes_stay_out_of_search_until_restore() {
    let mut w = world();
    let p = project(&mut w.store, "Crane");
    let pc = project_chat(&mut w.store, &p, "Crane chat");
    let n = chat_note(&mut w.store, &pc, "Crane berth attachment");
    set_chat(&mut w.store, &pc, true, ThreadState::Open);
    let preview = w.store.project_delete_preview(&p).unwrap();
    project_command(
        &mut w.store,
        ProjectCommand::Delete {
            preview,
            actions: BTreeMap::new(),
        },
    );
    for context in [None, Some(p.as_str()), Some(&pc)] {
        assert!(notes(&w.data, &[&n], context).is_empty(), "{context:?}");
    }
    let version = w.store.record(&p).unwrap().unwrap().version;
    project_command(
        &mut w.store,
        ProjectCommand::Restore {
            project: p.clone(),
            version,
        },
    );
    // Restore brings the chat back as it was: archived, so labelled.
    assert_eq!(notes(&w.data, &[&n], Some(&p)), set(&[&n]));
    let mut reader = Store::open_reader(&w.data).unwrap();
    let snapshot = reader.snapshot().unwrap();
    assert!(chat_is_archived(&snapshot.record(&pc).unwrap().unwrap()));
}

#[test]
fn a_deleted_project_hides_its_chats_and_restore_returns_them() {
    let mut w = world();
    let p = project(&mut w.store, "Crane");
    let pc = project_chat(&mut w.store, &p, "Crane chat");
    let m = message(&mut w.store, &pc, "assistant", "Berth survey reply");
    let preview = w.store.project_delete_preview(&p).unwrap();
    project_command(
        &mut w.store,
        ProjectCommand::Delete {
            preview,
            actions: BTreeMap::new(),
        },
    );
    assert_eq!(scope(&w.data, &pc), MessageScope::Hidden);
    assert!(scopes(&w.data, Some(&p)).is_empty());
    for context in [None, Some(p.as_str()), Some(&pc)] {
        assert!(
            searchable(&w.data, &[&m], context).is_empty(),
            "{context:?}"
        );
    }
    let version = w.store.record(&p).unwrap().unwrap().version;
    project_command(
        &mut w.store,
        ProjectCommand::Restore {
            project: p.clone(),
            version,
        },
    );
    assert_eq!(scope(&w.data, &pc), MessageScope::Project(p.clone()));
    assert_eq!(searchable(&w.data, &[&m], Some(&p)), set(&[&m]));
    assert!(searchable(&w.data, &[&m], None).is_empty());
}

#[test]
fn a_chat_moved_out_of_a_project_stays_searchable_only_from_itself() {
    let mut w = world();
    let p = project(&mut w.store, "Crane");
    let moved = chat(&mut w.store, "Moved");
    let other = chat(&mut w.store, "Other");
    let m = message(&mut w.store, &moved, "owner", "Berth survey question");
    assert_eq!(searchable(&w.data, &[&m], None), set(&[&m]));
    project_command(
        &mut w.store,
        ProjectCommand::MoveChat {
            chat: moved.clone(),
            project: Some(p.clone()),
        },
    );
    assert_eq!(scope(&w.data, &moved), MessageScope::Project(p.clone()));
    assert_eq!(searchable(&w.data, &[&m], Some(&p)), set(&[&m]));
    assert!(searchable(&w.data, &[&m], None).is_empty());
    project_command(
        &mut w.store,
        ProjectCommand::MoveChat {
            chat: moved.clone(),
            project: None,
        },
    );
    // Its discussion may hold project context, so it stays private.
    assert_eq!(scope(&w.data, &moved), MessageScope::Chat(moved.clone()));
    assert_eq!(searchable(&w.data, &[&m], Some(&moved)), set(&[&m]));
    for context in [None, Some(other.as_str()), Some(&p)] {
        assert!(
            searchable(&w.data, &[&m], context).is_empty(),
            "{context:?}"
        );
    }
}

#[test]
fn a_chat_with_its_own_attachments_is_searchable_only_from_itself() {
    let mut w = world();
    let private = chat(&mut w.store, "Attachments");
    let other = chat(&mut w.store, "Other");
    let m = message(&mut w.store, &private, "assistant", "Berth survey reply");
    write(
        &mut w.store,
        "attachment",
        vec![(
            None,
            RecordData::Note(Note {
                chat: Some(private.clone()),
                ..Note::working("Survey attachment", "Depth 1.4 m.")
            }),
            false,
        )],
    );
    assert!(w.store.chat_has_private_context(&private).unwrap());
    assert_eq!(
        scope(&w.data, &private),
        MessageScope::Chat(private.clone())
    );
    assert_eq!(searchable(&w.data, &[&m], Some(&private)), set(&[&m]));
    for context in [None, Some(other.as_str())] {
        assert!(
            searchable(&w.data, &[&m], context).is_empty(),
            "{context:?}"
        );
    }
}

#[test]
fn partial_progress_and_writing_requests_are_never_searchable() {
    let mut w = world();
    let general = chat(&mut w.store, "Replies");
    let owner = message(&mut w.store, &general, "owner", "Berth survey question");
    let assistant = message(&mut w.store, &general, "assistant", "Berth survey reply");
    let partial = message(
        &mut w.store,
        &general,
        "assistant-partial",
        "Berth survey half",
    );
    let request = message(
        &mut w.store,
        &general,
        WRITING_REQUEST_ROLE,
        "Berth survey rewrite",
    );
    assert_eq!(
        searchable(&w.data, &[&owner, &assistant, &partial, &request], None),
        set(&[&owner, &assistant])
    );
    for id in [&partial, &request] {
        assert!(!message_searchable(&w.store.record(id).unwrap().unwrap()));
    }
    // An archived Message is not current.
    let record = w.store.record(&owner).unwrap().unwrap();
    write(
        &mut w.store,
        "archive-message",
        vec![(Some(&record), record.data.clone(), true)],
    );
    assert_eq!(
        searchable(&w.data, &[&owner, &assistant], None),
        set(&[&assistant])
    );
}
