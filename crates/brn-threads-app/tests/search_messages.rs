//! Saved chat replies are findable in owner search, as their own kind with
//! their chat. Project chats appear only in their project's search, general
//! search shows general chats, Trashed chats disappear until restored,
//! partial progress is never found, and agent tools never see chat
//! Messages. Archived (and legacy Resolved) chats' Messages and chat-only
//! notes stay in owner search, labelled `archived` (owner decision 3,
//! 16 October 2026); agent search is unchanged. Synthetic data and a
//! scripted model; no provider is contacted.
use brn_ai::threads::{RunNoteSearch, ThreadRunRequest, prepare_run, run_prepared_scripted};
use brn_ai::{AiTerminal, Provider, ReasoningEffort, Selection};
use brn_retrieval::search_index::{Changes, ChatScope, FILE_NAME, SearchIndex};
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const REPLY: &str = "Draft reply (not sent): the berth survey measured 1.4 m.";

struct World {
    _root: tempfile::TempDir,
    data: PathBuf,
    workspace: Workspace,
}

/// Offline settings: no sign-in file and no credentials, so a provider run
/// would fail before network access.
fn world() -> World {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut workspace = Workspace::open(&data).unwrap();
    let settings = workspace.settings().unwrap();
    workspace
        .configure(WorkspaceSettings {
            provider: "Codex".into(),
            model: "gpt-6.1-sol".into(),
            effort: "medium".into(),
            auth_file: None,
            credentials_dir: None,
            ..settings
        })
        .unwrap();
    World {
        _root: root,
        data,
        workspace,
    }
}

fn request(data: &Path, chat: &str, prompt: &str) -> ThreadRunRequest {
    ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: data.to_owned(),
        thread: chat.to_owned(),
        prompt: prompt.into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: ReasoningEffort::Medium,
        max_tool_rounds: 12,
        auth_file: None,
        credentials_dir: None,
        sources: vec![],
        search: None,
        trace: brn_ai::threads::TraceLevel::Off,
    }
}

fn finish(text: &str) -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::text(text),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]
}

/// The owner Message's ID.
fn post(workspace: &mut Workspace, chat: &str, text: &str) -> String {
    workspace.post_message(chat, text).unwrap().writes[0]
        .after
        .id
        .clone()
}

/// The Message IDs owner search lists in `context`.
fn found(workspace: &mut Workspace, query: &str, context: Option<&str>) -> BTreeSet<String> {
    let page = workspace.search_page(query, context, 12, None).unwrap();
    assert_eq!(page.message_total, page.messages.len(), "{context:?}");
    page.messages.into_iter().map(|m| m.message).collect()
}

fn set(ids: &[&String]) -> BTreeSet<String> {
    ids.iter().map(|id| (*id).clone()).collect()
}

fn project(workspace: &mut Workspace, title: &str) -> String {
    workspace
        .project_command(
            &format!("project:{title}"),
            ProjectCommand::Create {
                title: title.into(),
            },
        )
        .unwrap()
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Project(_)))
        .unwrap()
        .after
        .id
}

fn project_chat(workspace: &mut Workspace, project: &str, title: &str) -> String {
    workspace
        .project_command(
            &format!("chat:{title}"),
            ProjectCommand::CreateChat {
                project: project.into(),
                title: title.into(),
            },
        )
        .unwrap()
        .writes
        .into_iter()
        .find(|w| w.before.is_none() && matches!(w.after.data, RecordData::Thread(_)))
        .unwrap()
        .after
        .id
}

fn knowledge(workspace: &mut Workspace, project: &str, enabled: bool) {
    let record = workspace.store.record(project).unwrap().unwrap();
    let RecordData::Project(p) = record.data else {
        panic!("Project")
    };
    workspace
        .project_command(
            &format!("knowledge:{enabled}"),
            ProjectCommand::Configure {
                project: project.into(),
                version: record.version,
                title: p.title,
                instructions: p.instructions,
                use_brn_knowledge: enabled,
            },
        )
        .unwrap();
}

fn owner_change(workspace: &mut Workspace, key: &str, puts: Vec<Put>) {
    let outcome = workspace
        .owner_change(
            key,
            &ChangeRequest {
                reason: key.into(),
                writes: puts,
                inputs: vec![],
            },
        )
        .unwrap();
    assert!(matches!(outcome, ApplyOutcome::Applied(_)), "{outcome:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_saved_reply_is_found_with_its_chat() {
    let mut w = world();
    let chat = w.workspace.new_thread("Harbour replies").unwrap().id;
    w.workspace
        .new_note("Berth notes", "The berth survey is scheduled.")
        .unwrap();
    let before = w
        .workspace
        .search_page("berth survey", None, 12, None)
        .unwrap();
    assert!(before.messages.is_empty());
    let prepared =
        prepare_run(request(&w.data, &chat, "Please draft the harbour answer.")).unwrap();
    let result = run_prepared_scripted(
        prepared,
        MockCompletionModel::from_stream_turns([finish(REPLY)]),
    )
    .await
    .unwrap();
    assert!(matches!(result.answer.terminal, AiTerminal::Completed));
    let page = w
        .workspace
        .search_page("berth survey", None, 12, None)
        .unwrap();
    // Before: no messages; a saved reply was found only by browsing chats.
    assert_eq!(page.message_total, 1);
    let hit = &page.messages[0];
    assert_eq!(
        (
            hit.role.as_str(),
            hit.chat.as_str(),
            hit.chat_title.as_str()
        ),
        ("assistant", chat.as_str(), "Harbour replies")
    );
    assert_eq!(hit.snippet, REPLY);
    assert_eq!(&hit.snippet[hit.highlights[0].clone()], "berth");
    let Some(Record {
        version,
        data: RecordData::Message(saved),
        ..
    }) = w.workspace.store.record(&hit.message).unwrap()
    else {
        panic!("the hit names a saved Message")
    };
    assert_eq!((saved.text.as_str(), version), (REPLY, hit.version));
    // Notes are a separate kind: their hits and counts are unchanged.
    let notes = |p: &SearchPage| -> Vec<String> { p.hits.iter().map(|h| h.note.clone()).collect() };
    assert_eq!(notes(&page), notes(&before));
    assert_eq!((page.total, page.cursor.clone()), (before.total, None));
    assert_eq!(page.message_cursor, None);
}

#[test]
fn owner_messages_are_found_and_partial_progress_is_not() {
    let mut w = world();
    let chat = w.workspace.new_thread("Dredging").unwrap().id;
    let prepared = prepare_run(request(
        &w.data,
        &chat,
        "Check the dredging depth at berth 4.",
    ))
    .unwrap();
    let run = prepared.run_id().to_owned();
    drop(prepared);
    w.workspace
        .stop_chat_run(
            "stop:dredging",
            &run,
            Some("Partial: dredging depth unknown"),
        )
        .unwrap();
    let messages = w.workspace.messages(&chat).unwrap();
    let id = |role: &str| {
        messages
            .iter()
            .find(|m| matches!(&m.data, RecordData::Message(m) if m.role == role))
            .map(|m| m.id.clone())
            .unwrap()
    };
    let (owner, partial) = (id("owner"), id("assistant-partial"));
    let page = w.workspace.search_page("dredging", None, 12, None).unwrap();
    let listed: Vec<_> = page
        .messages
        .iter()
        .map(|m| (m.message.clone(), m.role.clone()))
        .collect();
    assert_eq!(listed, [(owner, "owner".to_string())]);
    assert_eq!(page.message_total, 1);
    assert!(!found(&mut w.workspace, "unknown", Some(&chat)).contains(&partial));
}

#[test]
fn project_chats_appear_only_in_their_project_search() {
    let mut w = world();
    let ws = &mut w.workspace;
    let p = project(ws, "Crane");
    let pc = project_chat(ws, &p, "Crane chat");
    let q = project(ws, "Garden");
    let qc = project_chat(ws, &q, "Garden chat");
    let g = ws.new_thread("General").unwrap().id;
    let mp = post(ws, &pc, "Pontoon mooring plan for the crane");
    let mq = post(ws, &qc, "Pontoon mooring plan for the garden");
    let mg = post(ws, &g, "Pontoon mooring plan in general");
    // Before: every context listed no messages.
    assert_eq!(found(ws, "pontoon", None), set(&[&mg]));
    assert_eq!(found(ws, "pontoon", Some(&g)), set(&[&mg]));
    for context in [&p, &pc] {
        assert_eq!(found(ws, "pontoon", Some(context)), set(&[&mp, &mg]));
    }
    for context in [&q, &qc] {
        assert_eq!(found(ws, "pontoon", Some(context)), set(&[&mq, &mg]));
    }
    // Without BRN knowledge a project shows only its own chats.
    knowledge(ws, &p, false);
    for context in [&p, &pc] {
        assert_eq!(found(ws, "pontoon", Some(context)), set(&[&mp]));
    }
    assert_eq!(found(ws, "pontoon", None), set(&[&mg]));
}

/// Message ID -> `archived` label of each Message owner search lists.
fn labelled(
    workspace: &mut Workspace,
    query: &str,
    context: Option<&str>,
) -> BTreeMap<String, bool> {
    let page = workspace.search_page(query, context, 12, None).unwrap();
    assert_eq!(page.message_total, page.messages.len(), "{context:?}");
    page.messages
        .into_iter()
        .map(|m| (m.message, m.archived))
        .collect()
}

/// Note ID -> `archived` label of each note owner search lists.
fn labelled_notes(
    workspace: &mut Workspace,
    query: &str,
    context: Option<&str>,
) -> BTreeMap<String, bool> {
    let page = workspace.search_page(query, context, 12, None).unwrap();
    assert_eq!(page.total, page.hits.len(), "{context:?}");
    page.hits
        .into_iter()
        .map(|h| (h.note, h.archived))
        .collect()
}

fn labels(pairs: &[(&String, bool)]) -> BTreeMap<String, bool> {
    pairs.iter().map(|(id, a)| ((*id).clone(), *a)).collect()
}

/// Saves chat `id` as a legacy Resolved chat: not archived, state Resolved.
fn resolve_legacy(workspace: &mut Workspace, id: &str) {
    let record = workspace.store.record(id).unwrap().unwrap();
    let RecordData::Thread(mut thread) = record.data.clone() else {
        panic!("Thread")
    };
    thread.state = ThreadState::Resolved;
    owner_change(
        workspace,
        &format!("legacy-resolve:{id}"),
        vec![Put {
            id: id.into(),
            expected_version: Some(record.version),
            archived: false,
            data: RecordData::Thread(thread),
        }],
    );
}

/// Owner decision 3: an archived chat's Messages stay in owner search under
/// the chat's normal scope, labelled; Trash hides them until Restore.
#[test]
fn archived_chats_stay_labelled_and_trashed_chats_return_on_restore() {
    let mut w = world();
    let ws = &mut w.workspace;
    let g = ws.new_thread("Replies").unwrap().id;
    let mg = post(ws, &g, "Pontoon reply saved here");
    let p = project(ws, "Crane");
    let pc = project_chat(ws, &p, "Crane chat");
    let mp = post(ws, &pc, "Pontoon reply in the project");
    assert_eq!(labelled(ws, "pontoon", None), labels(&[(&mg, false)]));
    ws.archive_chat("archive:replies", &g).unwrap();
    // Before: gone from every context until Unarchive.
    assert_eq!(labelled(ws, "pontoon", None), labels(&[(&mg, true)]));
    assert_eq!(labelled(ws, "pontoon", Some(&g)), labels(&[(&mg, true)]));
    assert_eq!(
        labelled(ws, "pontoon", Some(&p)),
        labels(&[(&mp, false), (&mg, true)])
    );
    ws.archive_chat("archive:crane-chat", &pc).unwrap();
    assert_eq!(
        labelled(ws, "pontoon", Some(&p)),
        labels(&[(&mp, true), (&mg, true)])
    );
    assert_eq!(labelled(ws, "pontoon", None), labels(&[(&mg, true)]));
    ws.unarchive_chat("unarchive:replies", &g).unwrap();
    ws.unarchive_chat("unarchive:crane-chat", &pc).unwrap();
    assert_eq!(labelled(ws, "pontoon", None), labels(&[(&mg, false)]));
    // A legacy Resolved chat counts as archived.
    resolve_legacy(ws, &g);
    assert_eq!(labelled(ws, "pontoon", None), labels(&[(&mg, true)]));
    ws.unarchive_chat("unarchive:legacy", &g).unwrap();
    assert_eq!(labelled(ws, "pontoon", None), labels(&[(&mg, false)]));
    let preview = ws.project_delete_preview(&p).unwrap();
    ws.project_command(
        "delete:crane",
        ProjectCommand::Delete {
            preview,
            actions: BTreeMap::new(),
        },
    )
    .unwrap();
    for context in [Some(p.as_str()), Some(&pc), None] {
        assert!(!found(ws, "pontoon", context).contains(&mp), "{context:?}");
    }
    let version = ws.store.record(&p).unwrap().unwrap().version;
    ws.project_command(
        "restore:crane",
        ProjectCommand::Restore {
            project: p.clone(),
            version,
        },
    )
    .unwrap();
    assert_eq!(found(ws, "pontoon", Some(&p)), set(&[&mp, &mg]));
    assert_eq!(found(ws, "pontoon", None), set(&[&mg]));
}

/// An archived chat's Messages stay counted and each listed hit carries its
/// own chat's label, beside a current chat's. Before decision 3 the index
/// dropped them (round 5 F-A5).
#[test]
fn an_archived_chats_messages_stay_counted_and_labelled() {
    let mut w = world();
    let ws = &mut w.workspace;
    let old = ws.new_thread("Old log").unwrap().id;
    let op = ws.store.allocate_operation("old-log").unwrap();
    let puts = (0..100)
        .map(|i| Put {
            id: op.creation_id(i),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: old.clone(),
                role: "owner".into(),
                text: format!("Pontoon {i:03}"),
            }),
        })
        .collect();
    owner_change(ws, "old-log", puts);
    let current = ws.new_thread("Current").unwrap().id;
    // The shortest text ranks first, so it is always within reach.
    let m = post(ws, &current, "Pontoon");
    assert_eq!(
        ws.search_page("pontoon", None, 12, None)
            .unwrap()
            .message_total,
        101
    );
    ws.archive_chat("archive:old", &old).unwrap();
    let page = ws.search_page("pontoon", None, 50, None).unwrap();
    assert_eq!(page.message_total, 101);
    let mut listed = page.messages.clone();
    listed.extend(
        ws.search_page("pontoon", None, 50, page.message_cursor.as_deref())
            .unwrap()
            .messages,
    );
    assert_eq!(listed.len(), 100, "reach stays capped at 100");
    for hit in &listed {
        assert_eq!(hit.archived, hit.chat == old, "{}", hit.message);
    }
    assert!(listed.iter().any(|hit| hit.message == m && !hit.archived));
}

/// Owner decision 3: an archived (or legacy Resolved) project chat's
/// chat-only note and Message show in that project's owner search, labelled,
/// and nowhere else. Agent retrieval is exactly as before: `search_notes`
/// finds the note (a project document, archived or not) and never the
/// Message, and exact reads from a sibling chat still refuse the archived
/// chat's Message.
#[test]
fn an_archived_project_chats_note_is_found_in_its_project_and_agents_keep_their_scope() {
    let mut w = world();
    let ws = &mut w.workspace;
    let p = project(ws, "Crane");
    let pc = project_chat(ws, &p, "Crane chat");
    let sibling = project_chat(ws, &p, "Crane sibling");
    let q = project(ws, "Garden");
    let qc = project_chat(ws, &q, "Garden chat");
    let general = ws.new_thread("General").unwrap().id;
    let op = ws.store.allocate_operation("crane-attachment").unwrap();
    let n = op.creation_id(0);
    owner_change(
        ws,
        "crane-attachment",
        vec![Put {
            id: n.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Note(Note {
                chat: Some(pc.clone()),
                ..Note::working("Crane attachment", "Pontoon depth 1.4 m at berth 4.")
            }),
        }],
    );
    let mp = post(ws, &pc, "Pontoon question in the crane chat");
    let handle = ws.search_handle().unwrap();
    let agent = |chat: &str| -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        for hybrid in [true, false] {
            let page =
                RunNoteSearch::search(&handle, Some(chat), "pontoon", 10, None, hybrid).unwrap();
            assert_eq!(page.total, page.results.len());
            seen.extend(page.results.into_iter().map(|r| r.id));
        }
        seen
    };
    for context in [&p, &sibling, &pc] {
        assert_eq!(
            labelled_notes(ws, "pontoon", Some(context)),
            labels(&[(&n, false)])
        );
    }
    assert_eq!(agent(&sibling), set(&[&n]));
    for leg in ["archive", "legacy-resolved"] {
        if leg == "archive" {
            ws.archive_chat("archive:crane-chat", &pc).unwrap();
        } else {
            resolve_legacy(ws, &pc);
        }
        // Before: gone from project search once the chat was archived.
        for context in [&p, &sibling] {
            assert_eq!(
                labelled_notes(ws, "pontoon", Some(context)),
                labels(&[(&n, true)]),
                "{leg}"
            );
            assert_eq!(
                labelled(ws, "pontoon", Some(context)),
                labels(&[(&mp, true)]),
                "{leg}"
            );
        }
        for context in [None, Some(q.as_str()), Some(&qc), Some(&general)] {
            assert!(
                labelled_notes(ws, "pontoon", context).is_empty(),
                "{leg} {context:?}"
            );
            assert!(
                labelled(ws, "pontoon", context).is_empty(),
                "{leg} {context:?}"
            );
        }
        // Agent retrieval scope is unchanged: the same note, with no label
        // (agent results carry none), and no exact read of the Message.
        assert_eq!(agent(&sibling), set(&[&n]), "{leg}");
        assert!(ws.store.record_visible_in_chat(&n, Some(&sibling)).unwrap());
        assert!(
            !ws.store
                .record_visible_in_chat(&mp, Some(&sibling))
                .unwrap()
        );
        ws.unarchive_chat(&format!("unarchive:{leg}"), &pc).unwrap();
        assert_eq!(
            labelled_notes(ws, "pontoon", Some(&p)),
            labels(&[(&n, false)])
        );
        assert_eq!(agent(&sibling), set(&[&n]), "{leg}");
        assert!(
            ws.store
                .record_visible_in_chat(&mp, Some(&sibling))
                .unwrap()
        );
    }
    // Trash hides the note and the Message from owner search; Restore
    // brings them back.
    let preview = ws.project_delete_preview(&p).unwrap();
    ws.project_command(
        "delete:crane",
        ProjectCommand::Delete {
            preview,
            actions: BTreeMap::new(),
        },
    )
    .unwrap();
    for context in [None, Some(pc.as_str())] {
        assert!(
            labelled_notes(ws, "pontoon", context).is_empty(),
            "{context:?}"
        );
        assert!(labelled(ws, "pontoon", context).is_empty(), "{context:?}");
    }
    let version = ws.store.record(&p).unwrap().unwrap().version;
    ws.project_command(
        "restore:crane",
        ProjectCommand::Restore {
            project: p.clone(),
            version,
        },
    )
    .unwrap();
    assert_eq!(
        labelled_notes(ws, "pontoon", Some(&p)),
        labels(&[(&n, false)])
    );
    assert_eq!(labelled(ws, "pontoon", Some(&p)), labels(&[(&mp, false)]));
}

#[test]
fn a_private_attachment_chat_is_searchable_only_from_itself() {
    let mut w = world();
    let ws = &mut w.workspace;
    let private = ws.new_thread("Attachments").unwrap().id;
    let other = ws.new_thread("Other").unwrap().id;
    let m = post(ws, &private, "Pontoon depth from the attachment");
    assert_eq!(found(ws, "pontoon", None), set(&[&m]));
    let op = ws.store.allocate_operation("attachment").unwrap();
    owner_change(
        ws,
        "attachment",
        vec![Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Note(Note {
                chat: Some(private.clone()),
                ..Note::working("Survey attachment", "Depth 1.4 m.")
            }),
        }],
    );
    assert_eq!(found(ws, "pontoon", Some(&private)), set(&[&m]));
    for context in [None, Some(other.as_str())] {
        assert!(found(ws, "pontoon", context).is_empty(), "{context:?}");
    }
    // Kept in BRN (no longer chat-only), the attachment stops making the chat private.
    let note = ws.store.record(&op.creation_id(0)).unwrap().unwrap();
    let RecordData::Note(mut kept) = note.data.clone() else {
        panic!("Note")
    };
    kept.chat = None;
    owner_change(
        ws,
        "keep",
        vec![Put {
            id: note.id.clone(),
            expected_version: Some(note.version),
            archived: false,
            data: RecordData::Note(kept),
        }],
    );
    assert_eq!(found(ws, "pontoon", None), set(&[&m]));
}

#[test]
fn rebuilding_the_index_backfills_messages() {
    let w = world();
    let mut ws = w.workspace;
    let chat = ws.new_thread("Replies").unwrap().id;
    let m = post(&mut ws, &chat, "Pontoon reply to keep");
    // Sync once with a worker this test can join, so the index file is closed
    // and checkpointed afterwards.
    let (handle, worker) = search::SearchHandle::spawn(&ws.store).unwrap();
    let watermark = handle.search("pontoon", None, 10).unwrap().watermark;
    drop(handle);
    worker.join().unwrap();
    drop(ws);
    // The state the previous build left: synced to the current watermark,
    // no messages, schema version 2.
    let derived = Store::open(&w.data).unwrap().derived_directory().unwrap();
    let (mut index, _) = SearchIndex::open(&derived).unwrap();
    let base = index.sync_state().unwrap();
    assert_eq!(base.as_ref().map(|s| s.watermark), Some(watermark));
    index
        .apply(&Changes {
            base,
            remove_messages: vec![m.clone()],
            watermark,
            ..Changes::default()
        })
        .unwrap();
    drop(index);
    let file = derived.join(FILE_NAME);
    // Closing the last connection checkpointed the WAL into the file.
    assert!(!derived.join(format!("{FILE_NAME}-wal")).exists());
    set_user_version(&file, 2);
    let mut view = Workspace::open(&w.data).unwrap();
    // Before: an index at the current watermark never saw older messages.
    assert_eq!(found(&mut view, "pontoon", None), set(&[&m]));
    drop(view);
    // A deleted index is re-created and backfilled too.
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", file.display()));
    }
    let mut view = Workspace::open(&w.data).unwrap();
    assert_eq!(found(&mut view, "pontoon", None), set(&[&m]));
}

/// A v3 index (round 5) tagged an archived chat `hidden`. Schema v4 rebuilds
/// it, so the archived chat's Message returns, labelled, without any new
/// write to the chat.
#[test]
fn a_v3_index_is_rebuilt_so_an_archived_chat_is_rescoped() {
    let w = world();
    let mut ws = w.workspace;
    let chat = ws.new_thread("Replies").unwrap().id;
    let m = post(&mut ws, &chat, "Pontoon reply to keep");
    ws.archive_chat("archive:replies", &chat).unwrap();
    let (handle, worker) = search::SearchHandle::spawn(&ws.store).unwrap();
    let watermark = handle.search("pontoon", None, 10).unwrap().watermark;
    drop(handle);
    worker.join().unwrap();
    drop(ws);
    // The state a round 5 build left: the archived chat tagged hidden, v3.
    let derived = Store::open(&w.data).unwrap().derived_directory().unwrap();
    let (mut index, _) = SearchIndex::open(&derived).unwrap();
    let base = index.sync_state().unwrap();
    assert_eq!(base.as_ref().map(|s| s.watermark), Some(watermark));
    index
        .apply(&Changes {
            base,
            chat_scopes: vec![(chat.clone(), ChatScope::Hidden)],
            watermark,
            ..Changes::default()
        })
        .unwrap();
    drop(index);
    let file = derived.join(FILE_NAME);
    assert!(!derived.join(format!("{FILE_NAME}-wal")).exists());
    set_user_version(&file, 3);
    let mut view = Workspace::open(&w.data).unwrap();
    // Before the bump the stale `hidden` tag stayed until the chat changed.
    assert_eq!(labelled(&mut view, "pontoon", None), labels(&[(&m, true)]));
}

/// Writes `version` into the SQLite header's `user_version` (offset 60) of a
/// closed, checkpointed database file.
fn set_user_version(path: &Path, version: u32) {
    let mut bytes = std::fs::read(path).unwrap();
    bytes[60..64].copy_from_slice(&version.to_be_bytes());
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn agent_search_never_returns_messages() {
    let mut w = world();
    let chat = w.workspace.new_thread("Replies").unwrap().id;
    let m = post(&mut w.workspace, &chat, "Pontoon reply only in chat");
    assert_eq!(found(&mut w.workspace, "pontoon", Some(&chat)), set(&[&m]));
    let handle = w.workspace.search_handle().unwrap();
    for hybrid in [true, false] {
        let page =
            RunNoteSearch::search(&handle, Some(&chat), "pontoon", 10, None, hybrid).unwrap();
        assert_eq!(
            (page.total, page.shown, page.results.len(), page.cursor),
            (0, 0, 0, None)
        );
    }
}

#[test]
fn message_pages_follow_the_cursor() {
    let mut w = world();
    let chat = w.workspace.new_thread("Log").unwrap().id;
    let op = w.workspace.store.allocate_operation("log").unwrap();
    let puts = (0..60)
        .map(|i| Put {
            id: op.creation_id(i),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: chat.clone(),
                role: "owner".into(),
                text: format!("Pontoon log entry {i:02}"),
            }),
        })
        .collect();
    owner_change(&mut w.workspace, "log", puts);
    let first = w.workspace.search_page("pontoon", None, 50, None).unwrap();
    assert_eq!(
        (
            first.messages.len(),
            first.message_total,
            first.message_cursor.as_deref(),
            first.cursor.as_deref(),
            first.hits.len(),
        ),
        (50, 60, Some("50"), None, 0)
    );
    let next = w
        .workspace
        .search_page("pontoon", None, 50, first.message_cursor.as_deref())
        .unwrap();
    assert_eq!(
        (
            next.offset,
            next.messages.len(),
            next.message_total,
            next.message_cursor
        ),
        (50, 10, 60, None)
    );
    let all: BTreeSet<_> = first
        .messages
        .iter()
        .chain(&next.messages)
        .map(|m| m.message.clone())
        .collect();
    assert_eq!(all.len(), 60);
}
