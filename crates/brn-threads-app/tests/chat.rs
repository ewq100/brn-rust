use brn_threads_app::*;
use brn_threads_intake::{ConversionResult, Coverage, CoverageStatus, ImportIntent, RetainedAsset};
use sha2::{Digest, Sha256};
fn state(w: &Workspace) -> ChatControl {
    let RecordData::ChatControl(c) = w.chat_control().unwrap().data else {
        panic!()
    };
    c
}
fn id_for_run(op: &OperationId) -> String {
    op.creation_id(0)
}
fn put_run(w: &mut Workspace, q: &QueuedMessage, key: &str) -> Record {
    let op = w.store.allocate_operation(key).unwrap();
    let authority = HostAuthority::owner("synthetic queued run");
    w.store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: key.into(),
                inputs: vec![],
                writes: vec![
                    Put {
                        id: op.creation_id(0),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Run(Run {
                            loaded_skill_details: vec![],
                            instruction: q.message.clone(),
                            queue_claim: q.dispatch.clone(),
                            thread: q.thread.clone(),
                            provider: "Codex".into(),
                            model: "gpt-6.1-sol".into(),
                            guide_identity: "fixture".into(),
                            budget: 8,
                            effort: "medium".into(),
                            loaded_skills: vec![],
                            fence: 1,
                            state: RunState::Working,
                            progress: String::new(),
                            operations: vec![],
                            sources: vec![],
                        }),
                    },
                    {
                        let control = w.chat_control().unwrap();
                        let RecordData::ChatControl(mut c) = control.data else {
                            panic!()
                        };
                        c.pending.as_mut().unwrap().run = Some(id_for_run(&op));
                        Put {
                            id: control.id,
                            expected_version: Some(control.version),
                            archived: false,
                            data: RecordData::ChatControl(c),
                        }
                    },
                ],
            },
            &authority,
        )
        .unwrap();
    let ApplyOutcome::Applied(r) = w.store.apply(&op, &authority).unwrap() else {
        panic!()
    };
    r.writes[0].after.clone()
}
fn terminal(w: &mut Workspace, record: &Record, key: &str, outcome: RunState) -> u64 {
    let RecordData::Run(mut r) = w.store.record(&record.id).unwrap().unwrap().data else {
        panic!()
    };
    r.state = outcome;
    r.fence += 1;
    let fence = r.fence;
    let current = w.store.record(&record.id).unwrap().unwrap();
    assert!(matches!(
        w.owner_change(
            key,
            &ChangeRequest {
                reason: key.into(),
                inputs: vec![],
                writes: vec![Put {
                    id: record.id.clone(),
                    expected_version: Some(current.version),
                    archived: false,
                    data: RecordData::Run(r)
                }]
            }
        )
        .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    fence
}
#[test]
fn dispatch_claim_replay_restart_and_competing_process_cannot_repost_instruction() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().canonicalize().unwrap().join("data");
    let mut a = Workspace::open(&dir).unwrap();
    let chat = a.new_thread("Queue").unwrap().id;
    let first = a.queue_message("first", &chat, "One").unwrap();
    assert_eq!(a.queue_message("first", &chat, "One").unwrap(), first);
    assert!(a.queue_message("first", &chat, "Changed").is_err());
    a.queue_message("second", &chat, "Two").unwrap();
    let mut b = Workspace::open(&dir).unwrap();
    let q = a.dispatch_queued("claim").unwrap().unwrap();
    assert_eq!(a.dispatch_queued("claim").unwrap().unwrap(), q);
    assert!(b.dispatch_queued("competitor").is_err());
    assert_eq!(a.messages(&chat).unwrap().len(), 1);
    drop(a);
    drop(b);
    let mut a = Workspace::open(&dir).unwrap();
    a.recover_queue("restart").unwrap();
    assert!(a.dispatch_queued("paused").unwrap().is_none());
    a.resume_queue("resume").unwrap();
    let retried = a.dispatch_queued("retry").unwrap().unwrap();
    assert_eq!(retried.message, q.message);
    assert_ne!(retried.dispatch, q.dispatch);
    assert_eq!(a.messages(&chat).unwrap().len(), 1);
    assert_eq!(state(&a).queue[0].text, "Two");
}
#[test]
fn late_or_forged_completion_does_not_advance_and_stop_preserves_saved_partial() {
    let root = tempfile::tempdir().unwrap();
    let mut w = Workspace::open(root.path().canonicalize().unwrap().join("data")).unwrap();
    let chat = w.new_thread("Queue").unwrap().id;
    w.queue_message("one", &chat, "One").unwrap();
    w.queue_message("two", &chat, "Two").unwrap();
    let q = w.dispatch_queued("dispatch").unwrap().unwrap();
    let r = put_run(&mut w, &q, "run");
    w.bind_queued_run("bind", &q.id, q.message.as_ref().unwrap(), &r.id)
        .unwrap();
    assert!(
        w.finish_queued("forged", &q.id, q.message.as_ref().unwrap(), &r.id, 1, true)
            .is_err()
    );
    w.stop_chat_run("stop", &r.id, Some("Useful unfinished text"))
        .unwrap();
    let c = state(&w);
    assert!(c.paused);
    assert_eq!(c.queue.len(), 1);
    assert_eq!(c.pending.unwrap().text, "One");
    let RecordData::Run(stopped) = w.store.record(&r.id).unwrap().unwrap().data else {
        panic!()
    };
    assert!(
        w.finish_queued(
            "late-success",
            &q.id,
            q.message.as_ref().unwrap(),
            &r.id,
            stopped.fence,
            true
        )
        .is_err()
    );
    assert!(w.messages(&chat).unwrap().iter().any(|r|matches!(&r.data,RecordData::Message(m) if m.role=="assistant-partial" && m.text=="Useful unfinished text")));
    w.resume_queue("resume").unwrap();
    let new = w.dispatch_queued("retry").unwrap().unwrap();
    let next = put_run(&mut w, &new, "next-run");
    w.bind_queued_run(
        "next-bind",
        &new.id,
        new.message.as_ref().unwrap(),
        &next.id,
    )
    .unwrap();
    assert!(
        w.finish_queued(
            "old-failure",
            &q.id,
            q.message.as_ref().unwrap(),
            &r.id,
            stopped.fence,
            false
        )
        .is_err()
    );
    assert!(!state(&w).paused);
    let fence = terminal(&mut w, &next, "completed", RunState::Completed);
    w.pause_queue("pause-race").unwrap();
    w.finish_queued(
        "complete",
        &new.id,
        new.message.as_ref().unwrap(),
        &next.id,
        fence,
        true,
    )
    .unwrap();
    assert!(state(&w).paused);
    assert!(state(&w).pending.is_none());
    assert_eq!(state(&w).queue.len(), 1);
}
#[test]
fn queue_edit_reorder_and_archive_preserve_attention() {
    let root = tempfile::tempdir().unwrap();
    let mut w = Workspace::open(root.path().canonicalize().unwrap().join("data")).unwrap();
    let chat = w.new_thread("Queue").unwrap();
    let mut data = chat.data.clone();
    let RecordData::Thread(ref mut t) = data else {
        panic!()
    };
    t.attention.push(Attention {
        kind: AttentionKind::Question,
        reason: "Which date?".into(),
        record: None,
    });
    w.owner_change(
        "attention",
        &ChangeRequest {
            reason: "attention".into(),
            inputs: vec![],
            writes: vec![Put {
                id: chat.id.clone(),
                expected_version: Some(chat.version),
                archived: false,
                data,
            }],
        },
    )
    .unwrap();
    w.queue_message("a", &chat.id, "A").unwrap();
    w.queue_message("b", &chat.id, "B").unwrap();
    let c = state(&w);
    w.move_queued("move", &c.queue[1].id, Some(&c.queue[0].id))
        .unwrap();
    w.edit_queued("edit", &c.queue[1].id, "Edited B").unwrap();
    assert_eq!(state(&w).queue[0].text, "Edited B");
    w.remove_queued("remove", &c.queue[0].id).unwrap();
    w.archive_chat("archive", &chat.id).unwrap();
    assert!(state(&w).paused);
    let archived = w.store.record(&chat.id).unwrap().unwrap();
    assert!(archived.archived);
    assert!(matches!(archived.data,RecordData::Thread(t) if t.attention.len()==1));
    w.unarchive_chat("unarchive", &chat.id).unwrap();
    assert!(state(&w).paused);
    assert_eq!(state(&w).queue.len(), 1);
}
fn converted() -> ConversionResult {
    let bytes = vec![1, 2, 3];
    ConversionResult {
        intent: ImportIntent::FullNote,
        markdown: "# Private\n\nsecret attachment phrase\n\n![Figure](figure.png)".into(),
        assets: vec![RetainedAsset {
            name: "figure.png".into(),
            media_type: "image/png".into(),
            sha256: Sha256::digest(&bytes).into(),
            width: 1,
            height: 1,
            bytes,
        }],
        source_sha256: Sha256::digest(b"original").into(),
        source_reference: "/missing/original.docx".into(),
        converter: "synthetic".into(),
        sources: vec![],
        coverage: Coverage {
            status: CoverageStatus::Unchecked,
            checked_items: vec![],
            gaps: vec![],
        },
    }
}
#[test]
fn retained_attachment_is_private_after_restart_then_keep_and_undo_change_same_note() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("data");
    let mut w = Workspace::open(&path).unwrap();
    let chat = w.new_thread("Private").unwrap().id;
    let other = w.new_thread("Other").unwrap().id;
    let ApplyOutcome::Applied(r) = w
        .attach_full("attach", &chat, "Private file", &converted())
        .unwrap()
    else {
        panic!()
    };
    let note = r
        .writes
        .iter()
        .find(|w| matches!(w.after.data, RecordData::Note(_)))
        .unwrap()
        .after
        .clone();
    let asset = r
        .writes
        .iter()
        .find_map(|write| match &write.after.data {
            RecordData::Asset(asset) => Some(asset),
            _ => None,
        })
        .expect("attachment receipt retains its figure asset");
    assert!(!asset.bytes.is_empty());
    assert!(r
        .writes
        .iter()
        .any(|write| matches!(write.after.data, RecordData::Source(_))));
    assert!(r.writes.iter().any(|write| matches!(
        &write.after.data,
        RecordData::Link(link) if link.relation == "attachment"
    )));
    w.attach_full("other", &other, "Other file", &converted())
        .unwrap();
    assert!(w.notes().unwrap().is_empty());
    assert!(w.search("secret").unwrap().is_empty());
    let export = w
        .export_now(&root.path().canonicalize().unwrap().join("export"))
        .unwrap();
    assert!(export.files.is_empty());
    let backup = root.path().canonicalize().unwrap().join("backup.sqlite3");
    w.store.backup(&backup).unwrap();
    drop(w);
    let mut w = Workspace::open(&path).unwrap();
    assert!(
        w.store
            .read_note_range(&note.id, note.version, 0, 4096, false)
            .is_ok()
    );
    assert!(
        w.store
            .record_visible_in_chat(&note.id, Some(&chat))
            .unwrap()
    );
    assert!(
        !w.store
            .record_visible_in_chat(&note.id, Some(&other))
            .unwrap()
    );
    let restored = Store::restore(
        &backup,
        root.path().canonicalize().unwrap().join("restored"),
    )
    .unwrap();
    for write in &r.writes {
        assert_eq!(
            w.store.record(&write.after.id).unwrap().as_ref(),
            Some(&write.after),
            "source reopen must preserve attachment receipt record {}",
            write.after.id
        );
        assert_eq!(
            restored.record(&write.after.id).unwrap().as_ref(),
            Some(&write.after),
            "restore must preserve attachment receipt record {}",
            write.after.id
        );
    }
    assert!(!restored.record_visible_in_chat(&note.id, None).unwrap());
    let ApplyOutcome::Applied(kept) = w.keep_in_brn("keep", &note.id, note.version).unwrap() else {
        panic!()
    };
    assert!(
        matches!(&w.notes().unwrap()[0].data,RecordData::Note(n) if n.protected && n.chat.is_none())
    );
    assert_eq!(w.notes().unwrap()[0].id, note.id);
    assert!(!w.search("secret").unwrap().is_empty());
    assert_eq!(
        w.keep_in_brn("keep", &note.id, note.version).unwrap(),
        ApplyOutcome::Applied(kept.clone())
    );
    w.undo(kept.operation.as_str()).unwrap();
    assert!(w.notes().unwrap().is_empty());
    assert!(w.search("secret").unwrap().is_empty());
}

#[test]
fn old_global_import_binding_replays_identical_receipt() {
    let root = tempfile::tempdir().unwrap();
    let mut w = Workspace::open(root.path().canonicalize().unwrap().join("data")).unwrap();
    let converted = converted();
    let hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&("Original import", &converted)).unwrap())
    );
    w.store
        .allocate_bound_operation("full-import:legacy", &hash)
        .unwrap();
    let first = w
        .import_full("legacy", "Original import", &converted)
        .unwrap();
    assert_eq!(
        w.import_full("legacy", "Original import", &converted)
            .unwrap(),
        first
    );
}

#[test]
fn archived_attachment_replays_before_mutable_destination_checks() {
    let root = tempfile::tempdir().unwrap();
    let mut w = Workspace::open(root.path().canonicalize().unwrap().join("data")).unwrap();
    let thread = w.new_thread("Private").unwrap().id;
    let attached = w
        .attach_full("stable-attachment", &thread, "Private", &converted())
        .unwrap();
    w.archive_chat("archive", &thread).unwrap();
    assert_eq!(
        w.attach_full("stable-attachment", &thread, "Private", &converted())
            .unwrap(),
        attached
    );
    assert!(
        w.attach_full("new-attachment", &thread, "Private", &converted())
            .is_err()
    );
}
#[test]
fn competing_dispatchers_claim_exactly_one_instruction() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("data");
    let mut w = Workspace::open(&path).unwrap();
    let thread = w.new_thread("Queue").unwrap().id;
    w.queue_message("q", &thread, "Question").unwrap();
    drop(w);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers = (0..2)
        .map(|i| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut w = Workspace::open(path).unwrap();
                barrier.wait();
                w.dispatch_queued(&format!("claim-{i}"))
            })
        })
        .collect::<Vec<_>>();
    let outcomes = workers
        .into_iter()
        .map(|w| w.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        outcomes.iter().filter(|r| matches!(r, Ok(Some(_)))).count(),
        1
    );
    assert_eq!(
        Workspace::open(&path)
            .unwrap()
            .messages(&thread)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn keeping_a_chat_note_copies_its_referenced_private_figure_without_promoting_the_source_document()
{
    let root = tempfile::tempdir().unwrap();
    let mut w = Workspace::open(root.path().canonicalize().unwrap().join("data")).unwrap();
    let thread = w.new_thread("Private").unwrap().id;
    let ApplyOutcome::Applied(r) = w
        .attach_full("attach", &thread, "Original private file", &converted())
        .unwrap()
    else {
        panic!()
    };
    let original = r
        .writes
        .iter()
        .find(|w| matches!(w.after.data, RecordData::Note(_)))
        .unwrap()
        .after
        .clone();
    let asset = r
        .writes
        .iter()
        .find(|w| matches!(w.after.data, RecordData::Asset(_)))
        .unwrap()
        .after
        .id
        .clone();
    let op = w.store.allocate_operation("derived-private-note").unwrap();
    let id = op.creation_id(0);
    let mut n = Note::working(
        "Selected finding",
        format!("Supported finding\n![Figure](brn-asset://{asset})"),
    );
    n.chat = Some(thread);
    let ApplyOutcome::Applied(r) = w
        .owner_change(
            "derived-private-note",
            &ChangeRequest {
                reason: "Selected finding".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: id.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(n),
                }],
            },
        )
        .unwrap()
    else {
        panic!()
    };
    let ApplyOutcome::Applied(kept) = w
        .keep_in_brn("keep-selected", &id, r.writes[0].after.version)
        .unwrap()
    else {
        panic!()
    };
    assert!(!w.store.record_visible_in_chat(&original.id, None).unwrap());
    assert!(w.store.record_visible_in_chat(&id, None).unwrap());
    let exported = w
        .export_now(&root.path().canonicalize().unwrap().join("kept-export"))
        .unwrap();
    assert_eq!(exported.files.len(), 2);
    w.undo(kept.operation.as_str()).unwrap();
    assert!(!w.store.record_visible_in_chat(&id, None).unwrap());
}

#[test]
fn keep_globalizes_each_owned_figure_source_without_promoting_other_private_documents() {
    let root = tempfile::tempdir().unwrap();
    let mut w = Workspace::open(root.path().canonicalize().unwrap().join("data")).unwrap();
    let chat = w.new_thread("Private").unwrap().id;
    let mut imported = Vec::new();
    for i in 0..2 {
        let mut c = converted();
        c.source_reference = format!("private://file-{i}");
        let ApplyOutcome::Applied(r) = w
            .attach_full(&format!("attach-{i}"), &chat, &format!("Source {i}"), &c)
            .unwrap()
        else {
            panic!()
        };
        imported.push(r);
    }
    let op = w.store.allocate_operation("two-source-note").unwrap();
    let id = op.creation_id(0);
    let mut writes = Vec::new();
    let mut images = Vec::new();
    let mut sources = Vec::new();
    for (i, r) in imported.iter().enumerate() {
        let RecordData::Asset(mut a) = r
            .writes
            .iter()
            .find(|w| matches!(w.after.data, RecordData::Asset(_)))
            .unwrap()
            .after
            .data
            .clone()
        else {
            panic!()
        };
        a.note = id.clone();
        sources.push(a.source.clone().unwrap());
        let asset = op.creation_id(i + 1);
        images.push(format!("![Figure {i}](brn-asset://{asset})"));
        writes.push(Put {
            id: asset,
            expected_version: None,
            archived: false,
            data: RecordData::Asset(a),
        });
    }
    let mut note = Note::working("Combined private finding", images.join("\n"));
    note.chat = Some(chat.clone());
    note.import = Some(ImportCoverage {
        source: sources[0].clone(),
        full_note: false,
        gaps: vec![],
        report: None,
    });
    writes.push(Put {
        id: id.clone(),
        expected_version: None,
        archived: false,
        data: RecordData::Note(note),
    });
    let ApplyOutcome::Applied(r) = w
        .owner_change(
            "two-source-note",
            &ChangeRequest {
                reason: "Combined finding".into(),
                inputs: vec![],
                writes,
            },
        )
        .unwrap()
    else {
        panic!()
    };
    let version = r
        .writes
        .iter()
        .find(|w| w.after.id == id)
        .unwrap()
        .after
        .version;
    assert!(matches!(
        w.keep_in_brn("keep-combined", &id, version).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    for i in 0..2 {
        let asset = op.creation_id(i + 1);
        assert!(w.store.record_visible_in_chat(&asset, None).unwrap());
        let RecordData::Asset(a) = w.store.record(&asset).unwrap().unwrap().data else {
            panic!()
        };
        assert_eq!(a.note, id);
        assert!(
            matches!(w.store.record(a.source.as_ref().unwrap()).unwrap().unwrap().data,RecordData::Source(s) if s.chat.is_none())
        );
    }
    assert_eq!(w.notes().unwrap().len(), 1);
    assert_eq!(
        w.export_now(&root.path().canonicalize().unwrap().join("combined-export"))
            .unwrap()
            .files
            .len(),
        3
    );
}
