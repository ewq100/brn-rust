use brn_threads_app::*;
use std::collections::BTreeMap;
fn fixture() -> (tempfile::TempDir, Workspace) {
    let root = tempfile::TempDir::new_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let w = Workspace::open(root.path().join("data")).unwrap();
    (root, w)
}
fn command(w: &mut Workspace, c: ProjectCommand) -> Receipt {
    w.project_command(&uuid::Uuid::new_v4().to_string(), c)
        .unwrap()
}
fn project(w: &mut Workspace, title: &str) -> String {
    command(
        w,
        ProjectCommand::Create {
            title: title.into(),
        },
    )
    .writes[0]
        .after
        .id
        .clone()
}
fn chat(w: &mut Workspace, p: &str) -> String {
    command(
        w,
        ProjectCommand::CreateChat {
            project: p.into(),
            title: "Trip chat".into(),
        },
    )
    .writes
    .into_iter()
    .find(|r| matches!(r.after.data, RecordData::Thread(_)))
    .unwrap()
    .after
    .id
}
fn doc(w: &mut Workspace, p: &str, title: &str) -> String {
    command(
        w,
        ProjectCommand::CreateDocument {
            project: p.into(),
            title: title.into(),
            markdown: "Hakone itinerary mercury".into(),
        },
    )
    .writes
    .into_iter()
    .find(|r| matches!(r.after.data, RecordData::Note(_)))
    .unwrap()
    .after
    .id
}
fn knowledge(w: &mut Workspace, p: &str, enabled: bool) {
    let r = w.store.record(p).unwrap().unwrap();
    let RecordData::Project(v) = r.data else {
        panic!()
    };
    command(
        w,
        ProjectCommand::Configure {
            project: p.into(),
            version: r.version,
            title: v.title,
            instructions: "Use Estonian for beginners".into(),
            use_brn_knowledge: enabled,
        },
    );
}
fn seen(w: &Workspace, id: &str, c: &str) -> bool {
    w.store.record_visible_in_chat(id, Some(c)).unwrap()
}

#[test]
fn project_context_shared_identity_knowledge_setting_movement_and_replay() {
    let (_root, mut w) = fixture();
    let p = project(&mut w, "Japan");
    let other = project(&mut w, "Writing");
    let c = chat(&mut w, &p);
    let earlier = chat(&mut w, &p);
    let foreign = chat(&mut w, &other);
    let private = doc(&mut w, &p, "Mercury trip");
    let hidden = doc(&mut w, &p, "Private letter");
    let general = w.new_note("Mercury public", "mercury general").unwrap();
    assert!(seen(&w, &private, &c));
    assert!(!seen(&w, &private, &foreign));
    assert!(!w.store.record_visible_in_chat(&private, None).unwrap());
    let message = w
        .post_message(&earlier, "Hakone earlier discussion")
        .unwrap()
        .writes[0]
        .after
        .clone();
    assert!(seen(&w, &message.id, &c));
    assert!(!seen(&w, &message.id, &foreign));
    command(
        &mut w,
        ProjectCommand::LinkDocument {
            project: other.clone(),
            note: private.clone(),
        },
    );
    assert!(seen(&w, &private, &foreign));
    assert!(!seen(&w, &hidden, &foreign));
    assert!(!seen(&w, &message.id, &foreign));
    knowledge(&mut w, &p, false);
    assert!(!seen(&w, &general.id, &c));
    assert!(seen(&w, &private, &c));
    let hits = w.search_page("mercury", Some(&c), 12, None).unwrap();
    assert!(hits.hits.iter().any(|h| h.note == private));
    assert!(!hits.hits.iter().any(|h| h.note == general.id));
    let foreign_hits = w.search_page("mercury", Some(&foreign), 12, None).unwrap();
    assert!(foreign_hits.hits.iter().any(|h| h.note == private));
    assert!(
        !w.search("mercury")
            .unwrap()
            .iter()
            .any(|h| h.note == private)
    );
    let move_command = ProjectCommand::MoveChat {
        chat: c.clone(),
        project: Some(other.clone()),
    };
    let r = w
        .project_command("move-once", move_command.clone())
        .unwrap();
    assert_eq!(r, w.project_command("move-once", move_command).unwrap());
    assert!(
        w.project_command(
            "move-once",
            ProjectCommand::MoveChat {
                chat: c.clone(),
                project: None
            }
        )
        .is_err()
    );
    assert!(seen(&w, &private, &c));
    assert!(!seen(&w, &hidden, &c));
    command(
        &mut w,
        ProjectCommand::MoveChat {
            chat: c.clone(),
            project: None,
        },
    );
    assert!(!seen(&w, &private, &c));
    assert!(seen(&w, &general.id, &c));
    assert_eq!(w.store.record(&message.id).unwrap().unwrap(), message);
}
#[test]
fn project_document_links_reject_superseded_notes_but_allow_unlinking_existing_links() {
    let (_root, mut w) = fixture();
    let origin = project(&mut w, "Origin");
    let destination = project(&mut w, "Destination");
    let retired = w.new_note("Guide", "Original saved guide").unwrap();
    let current = w
        .new_note("Guide current", "Replacement saved guide")
        .unwrap();

    // Both Notes are current at link time, so linking them to separate Projects
    // establishes the positive control and a pre-existing link to preserve.
    command(
        &mut w,
        ProjectCommand::LinkDocument {
            project: origin.clone(),
            note: retired.id.clone(),
        },
    );
    command(
        &mut w,
        ProjectCommand::LinkDocument {
            project: destination.clone(),
            note: current.id.clone(),
        },
    );

    let mut retired_data = retired.data.clone();
    let RecordData::Note(note) = &mut retired_data else {
        panic!("new_note must return a Note")
    };
    note.superseded_by = Some(current.id.clone());
    w.owner_change(
        "supersede-project-document",
        &ChangeRequest {
            reason: "Supersede linked document".into(),
            inputs: vec![],
            writes: vec![Put {
                id: retired.id.clone(),
                expected_version: Some(retired.version),
                archived: false,
                data: retired_data,
            }],
        },
    )
    .unwrap();

    let records_before_refusal = w.records().unwrap();
    let history_before_refusal = w.history().unwrap();
    assert!(
        w.project_command(
            "link-superseded-project-document",
            ProjectCommand::LinkDocument {
                project: destination.clone(),
                note: retired.id.clone(),
            },
        )
        .is_err(),
        "a newly selected superseded document must be refused"
    );
    assert_eq!(w.records().unwrap(), records_before_refusal);
    assert_eq!(w.history().unwrap(), history_before_refusal);

    // Supersession does not silently retarget the pre-existing link. Its owner
    // can explicitly unlink the retired identity while the replacement remains
    // linked to the other Project.
    command(
        &mut w,
        ProjectCommand::UnlinkDocument {
            project: origin.clone(),
            note: retired.id.clone(),
        },
    );
    let RecordData::Project(origin_project) = w.store.record(&origin).unwrap().unwrap().data else {
        panic!("origin remains a Project")
    };
    assert!(!origin_project.documents.contains(&retired.id));
    let RecordData::Project(destination_project) =
        w.store.record(&destination).unwrap().unwrap().data
    else {
        panic!("destination remains a Project")
    };
    assert_eq!(destination_project.documents, [current.id]);
}

#[test]
fn delete_preview_guards_choices_and_restore_preserve_later_shared_work() {
    let (root, mut w) = fixture();
    let p = project(&mut w, "Japan");
    let other = project(&mut w, "Travel writing");
    let c = chat(&mut w, &p);
    let other_c = chat(&mut w, &other);
    let exclusive = doc(&mut w, &p, "Exclusive");
    let shared = doc(&mut w, &p, "Shared");
    command(
        &mut w,
        ProjectCommand::LinkDocument {
            project: other.clone(),
            note: shared.clone(),
        },
    );
    let comment_op = w.store.allocate_operation("project-passage").unwrap();
    let comment_id = comment_op.creation_id(0);
    w.owner_change(
        "project-passage",
        &ChangeRequest {
            reason: "Anchored feedback".into(),
            inputs: vec![],
            writes: vec![Put {
                id: comment_id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Comment(Comment {
                    note: exclusive.clone(),
                    base_version: 1,
                    quote: "itinerary".into(),
                    range: Some((7, 16)),
                    mapped_version: None,
                    mapped_range: None,
                    body: "Check this route".into(),
                    unresolved: false,
                }),
            }],
        },
    )
    .unwrap();
    let mut action_ids = vec![];
    for description in ["Send Mari's reply", "Prepare packing list"] {
        let op = w.store.allocate_operation(description).unwrap();
        let id = op.creation_id(0);
        let request = ChangeRequest {
            reason: "Owner tasks".into(),
            inputs: vec![],
            writes: vec![
                Put {
                    id: id.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Action(Action {
                        kept_outside_project: false,
                        owner: None,
                        waiting_on: None,
                        follow_up: None,
                        chat: None,
                        description: description.into(),
                        state: ActionState::Open,
                        internal: false,
                        evidence: None,
                        due: None,
                    }),
                },
                Put {
                    id: op.creation_id(1),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Link(Link {
                        from: c.clone(),
                        to: id.clone(),
                        relation: "task".into(),
                    }),
                },
            ],
        };
        w.store
            .prepare_owner(&op, &request, &HostAuthority::owner("test"))
            .unwrap();
        assert!(matches!(
            w.store.apply(&op, &HostAuthority::owner("test")).unwrap(),
            ApplyOutcome::Applied(_)
        ));
        action_ids.push(id);
    }
    let preview = w.project_delete_preview(&p).unwrap();
    assert!(preview.removed.iter().any(|r| r.id == exclusive));
    assert!(preview.retained.iter().any(|r| r.id == shared));
    assert_eq!(preview.unfinished_actions.len(), 2);
    assert!(
        w.project_command(
            "missing-choices",
            ProjectCommand::Delete {
                preview: preview.clone(),
                actions: BTreeMap::new()
            }
        )
        .is_err()
    );
    let choices = BTreeMap::from([
        (action_ids[0].clone(), ProjectActionChoice::Cancel),
        (action_ids[1].clone(), ProjectActionChoice::Keep),
    ]);
    let session = w.store.begin_edit(&exclusive, 1).unwrap();
    assert!(
        w.project_command(
            "guarded",
            ProjectCommand::Delete {
                preview: preview.clone(),
                actions: choices.clone()
            }
        )
        .is_err()
    );
    assert!(!w.store.record(&p).unwrap().unwrap().archived);
    w.store.discard(&session.id, session.generation).unwrap();
    let r = w
        .project_command(
            "delete",
            ProjectCommand::Delete {
                preview: preview.clone(),
                actions: choices.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        r,
        w.project_command(
            "delete",
            ProjectCommand::Delete {
                preview,
                actions: choices
            }
        )
        .unwrap()
    );
    assert!(w.store.record(&action_ids[0]).unwrap().unwrap().archived);
    assert!(
        !w.store
            .records_for_chat(Some(&other_c))
            .unwrap()
            .iter()
            .any(|r| r.id == action_ids[0])
    );
    assert!(w.store.record(&exclusive).unwrap().unwrap().archived);
    assert!(!w.store.record(&shared).unwrap().unwrap().archived);
    assert!(w.unarchive_chat("unarchive-trash-chat", &c).is_err());
    assert!(!seen(&w, &exclusive, &other_c));
    assert!(seen(&w, &shared, &other_c));
    let old = w.store.record(&shared).unwrap().unwrap();
    let mut n = match old.data.clone() {
        RecordData::Note(n) => n,
        _ => panic!(),
    };
    n.markdown = "Later adopted shared writing".into();
    assert!(matches!(
        w.owner_change(
            "later-shared-edit",
            &ChangeRequest {
                reason: "Later writing".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: shared.clone(),
                    expected_version: Some(old.version),
                    archived: false,
                    data: RecordData::Note(n)
                }]
            }
        )
        .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let backup = root.path().join("complete.sqlite3");
    w.store.backup(&backup).unwrap();
    drop(w);
    let mut w = Workspace::open(root.path().join("data")).unwrap();
    let deleted = w.store.record(&p).unwrap().unwrap();
    command(
        &mut w,
        ProjectCommand::Restore {
            project: p.clone(),
            version: deleted.version,
        },
    );
    assert!(!w.store.record(&exclusive).unwrap().unwrap().archived);
    let current_note = w.store.record(&exclusive).unwrap().unwrap();
    let current_comment = w.store.record(&comment_id).unwrap().unwrap();
    if let (RecordData::Note(n), RecordData::Comment(c)) =
        (&current_note.data, &current_comment.data)
    {
        assert_eq!(
            exact_writing_range(c, current_note.version, &n.markdown),
            Some((7, 16))
        );
    } else {
        panic!()
    }
    assert!(seen(&w, &shared, &c));
    assert!(
        matches!(w.store.record(&action_ids[0]).unwrap().unwrap().data, RecordData::Action(a) if a.state == ActionState::Cancelled)
    );
    assert!(
        matches!(w.store.record(&action_ids[1]).unwrap().unwrap().data, RecordData::Action(a) if a.state == ActionState::Open)
    );
    let restored_preview = w.project_delete_preview(&p).unwrap();
    assert!(restored_preview.unfinished_actions.is_empty());
    assert!(
        matches!(w.store.record(&shared).unwrap().unwrap().data, RecordData::Note(n) if n.markdown == "Later adopted shared writing")
    );
    let restored = Store::restore(&backup, root.path().join("restored")).unwrap();
    assert!(restored.record(&p).unwrap().unwrap().archived);
}
#[test]
fn stale_preview_and_changed_trash_fail_atomically_without_losing_records() {
    let (_root, mut w) = fixture();
    let p = project(&mut w, "Japan");
    let c = chat(&mut w, &p);
    let n = doc(&mut w, &p, "Itinerary");
    let stale = w.project_delete_preview(&p).unwrap();
    w.post_message(&c, "new relevant discussion").unwrap();
    assert!(
        w.project_command(
            "stale",
            ProjectCommand::Delete {
                preview: stale,
                actions: BTreeMap::new()
            }
        )
        .is_err()
    );
    assert!(!w.store.record(&p).unwrap().unwrap().archived);
    let fresh = w.project_delete_preview(&p).unwrap();
    command(
        &mut w,
        ProjectCommand::Delete {
            preview: fresh,
            actions: BTreeMap::new(),
        },
    );
    let r = w.store.record(&n).unwrap().unwrap();
    let mut data = r.data.clone();
    if let RecordData::Note(n) = &mut data {
        n.markdown = "Later content".into();
    }
    w.owner_change(
        "explicit-later-trash-edit",
        &ChangeRequest {
            reason: "later".into(),
            inputs: vec![],
            writes: vec![Put {
                id: n,
                expected_version: Some(r.version),
                archived: true,
                data,
            }],
        },
    )
    .unwrap();
    let r = w.store.record(&p).unwrap().unwrap();
    assert!(
        w.project_command(
            "restore-conflict",
            ProjectCommand::Restore {
                project: p.clone(),
                version: r.version
            }
        )
        .is_err()
    );
    assert!(w.store.record(&p).unwrap().unwrap().archived);
    assert!(w.store.record(&c).unwrap().unwrap().archived);
}

#[test]
fn unlink_preserves_private_retention_elsewhere_and_last_link_requires_explicit_keep() {
    let (_root, mut w) = fixture();
    let a = project(&mut w, "Origin");
    let b = project(&mut w, "Destination");
    let ca = chat(&mut w, &a);
    let cb = chat(&mut w, &b);
    let note = doc(&mut w, &a, "Shared itinerary");
    let before = w.store.record(&note).unwrap().unwrap();
    command(
        &mut w,
        ProjectCommand::LinkDocument {
            project: b.clone(),
            note: note.clone(),
        },
    );
    command(
        &mut w,
        ProjectCommand::UnlinkDocument {
            project: a.clone(),
            note: note.clone(),
        },
    );
    assert!(!seen(&w, &note, &ca));
    assert!(seen(&w, &note, &cb));
    assert!(!w.store.record_visible_in_chat(&note, None).unwrap());
    assert_eq!(w.store.record(&note).unwrap().unwrap(), before);
    assert!(
        w.project_command(
            "last-private-link",
            ProjectCommand::UnlinkDocument {
                project: b.clone(),
                note: note.clone()
            }
        )
        .is_err()
    );
    assert!(matches!(
        w.keep_in_brn("explicit-keep", &note, before.version)
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    command(
        &mut w,
        ProjectCommand::UnlinkDocument {
            project: b,
            note: note.clone(),
        },
    );
    assert!(w.store.record_visible_in_chat(&note, None).unwrap());
}

#[test]
fn moved_chat_keeps_private_provenance_and_undo_does_not_erase_it() {
    let (_root, mut w) = fixture();
    let p = project(&mut w, "Private project");
    let c = w.new_thread("Existing standalone").unwrap();
    let _receipt = command(
        &mut w,
        ProjectCommand::MoveChat {
            chat: c.id.clone(),
            project: Some(p),
        },
    );
    w.post_message(&c.id, "Saved project-only discussion")
        .unwrap();
    command(
        &mut w,
        ProjectCommand::MoveChat {
            chat: c.id.clone(),
            project: None,
        },
    );
    assert!(w.store.chat_has_private_context(&c.id).unwrap());
    let record = w.store.record(&c.id).unwrap().unwrap();
    assert!(
        matches!(record.data, RecordData::Thread(t) if t.private_context && t.context_epoch == 2)
    );
    // Fresh separate chat with no later membership edit qualifies exact receipt Undo.
    let (_other, mut other) = fixture();
    let p = project(&mut other, "Private");
    let c = other.new_thread("Standalone").unwrap();
    let receipt = command(
        &mut other,
        ProjectCommand::MoveChat {
            chat: c.id.clone(),
            project: Some(p),
        },
    );
    other
        .post_message(&c.id, "Private retained conversation")
        .unwrap();
    assert!(matches!(
        other.undo(receipt.operation.as_str()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(other.store.chat_has_private_context(&c.id).unwrap());
    let rows = other.records().unwrap();
    assert!(project_for_chat(&rows, &c.id).is_none());
    let _ = receipt;
}

#[test]
fn project_import_retains_source_figures_and_private_history_after_shared_origin_deletion() {
    use brn_threads_intake::{
        ConversionResult, Coverage, CoverageStatus, ImportIntent, RetainedAsset,
    };
    use sha2::{Digest, Sha256};
    let (root, mut w) = fixture();
    let a = project(&mut w, "Original project");
    let b = project(&mut w, "Shared destination");
    let ca = chat(&mut w, &a);
    let cb = chat(&mut w, &b);
    let bytes = b"retained synthetic figure".to_vec();
    let converted = ConversionResult {
        intent: ImportIntent::FullNote,
        source_reference: "synthetic:///missing-original.md".into(),
        source_sha256: Sha256::digest(b"Original supplied content").into(),
        markdown: "# Mercury reference\n\nComplete supplied passage\n\n![figure](figure.png)"
            .into(),
        assets: vec![RetainedAsset {
            name: "figure.png".into(),
            media_type: "image/png".into(),
            bytes: bytes.clone(),
            sha256: Sha256::digest(&bytes).into(),
            width: 1,
            height: 1,
        }],
        converter: "synthetic fixture".into(),
        sources: vec![],
        coverage: Coverage {
            status: CoverageStatus::Complete,
            checked_items: vec!["supplied paragraph and figure".into()],
            gaps: vec![],
        },
    };
    let ApplyOutcome::Applied(receipt) = w
        .import_full_scoped("project-import", "Mercury reference", &converted, Some(&a))
        .unwrap()
    else {
        panic!()
    };
    let note = receipt
        .writes
        .iter()
        .find(|w| matches!(w.after.data, RecordData::Note(_)))
        .unwrap()
        .after
        .clone();
    let source = receipt
        .writes
        .iter()
        .find(|w| matches!(w.after.data, RecordData::Source(_)))
        .unwrap()
        .after
        .clone();
    let asset = receipt
        .writes
        .iter()
        .find(|w| matches!(w.after.data, RecordData::Asset(_)))
        .unwrap()
        .after
        .clone();
    assert!(seen(&w, &note.id, &ca));
    assert!(!seen(&w, &source.id, &cb));
    assert!(!seen(&w, &asset.id, &cb));
    command(
        &mut w,
        ProjectCommand::LinkDocument {
            project: b.clone(),
            note: note.id.clone(),
        },
    );
    knowledge(&mut w, &b, false);
    assert!(seen(&w, &note.id, &cb));
    assert!(seen(&w, &source.id, &cb));
    assert!(seen(&w, &asset.id, &cb));
    assert!(
        w.store
            .revision_visible_in_chat(&note.id, 1, Some(&cb))
            .unwrap()
    );
    command(
        &mut w,
        ProjectCommand::UnlinkDocument {
            project: a.clone(),
            note: note.id.clone(),
        },
    );
    assert!(!seen(&w, &note.id, &ca));
    assert!(!seen(&w, &source.id, &ca));
    assert!(!seen(&w, &asset.id, &ca));
    assert!(
        !w.store
            .revision_visible_in_chat(&note.id, 1, Some(&ca))
            .unwrap()
    );
    let graph = w
        .search_handle()
        .unwrap()
        .connections(&note.id, Some(&cb), 1, true)
        .unwrap();
    assert!(
        graph
            .hops
            .iter()
            .flat_map(|h| h.shown.iter())
            .any(|n| n.id == source.id)
    );
    let preview = w.project_delete_preview(&a).unwrap();
    command(
        &mut w,
        ProjectCommand::Delete {
            preview,
            actions: BTreeMap::new(),
        },
    );
    drop(w);
    let mut w = Workspace::open(root.path().join("data")).unwrap();
    assert!(seen(&w, &note.id, &cb));
    assert_eq!(w.store.record(&asset.id).unwrap().unwrap(), asset);
    assert!(
        matches!(w.store.record(&note.id).unwrap().unwrap().data, RecordData::Note(n) if n.protected && n.markdown.contains("Complete supplied passage"))
    );
    assert!(matches!(
        w.keep_in_brn("keep-project-document", &note.id, note.version)
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(w.notes().unwrap().iter().any(|r| r.id == note.id));
    let kept = w
        .store
        .receipts()
        .unwrap()
        .into_iter()
        .find(|r| r.reason.contains("Keep") || r.reason.contains("Retain"))
        .unwrap();
    assert!(matches!(
        w.undo(kept.operation.as_str()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(!w.notes().unwrap().iter().any(|r| r.id == note.id));
    assert!(seen(&w, &note.id, &cb));
}

#[test]
fn existing_terminal_private_actions_go_to_trash_without_becoming_general_and_restore_terminal() {
    let (_root, mut w) = fixture();
    let p = project(&mut w, "Private obligations");
    let c = chat(&mut w, &p);
    let mut actions = vec![];
    for state in [ActionState::Done, ActionState::Cancelled] {
        let key = uuid::Uuid::new_v4().to_string();
        let op = w.store.allocate_operation(&key).unwrap();
        let id = op.creation_id(0);
        let r = w
            .owner_change(
                &key,
                &ChangeRequest {
                    reason: "Synthetic terminal obligation".into(),
                    inputs: vec![],
                    writes: vec![
                        Put {
                            id: id.clone(),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Action(Action {
                                kept_outside_project: false,
                                owner: None,
                                waiting_on: None,
                                follow_up: None,
                                chat: None,
                                description: "Private Mari reply".into(),
                                state: state.clone(),
                                internal: false,
                                evidence: Some("Synthetic prior owner-confirmed outcome".into()),
                                due: None,
                            }),
                        },
                        Put {
                            id: op.creation_id(1),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Link(Link {
                                from: c.clone(),
                                to: id.clone(),
                                relation: "task".into(),
                            }),
                        },
                    ],
                },
            )
            .unwrap();
        assert!(matches!(r, ApplyOutcome::Applied(_)));
        actions.push((id, state));
    }
    let preview = w.project_delete_preview(&p).unwrap();
    assert!(preview.unfinished_actions.is_empty());
    assert!(
        actions
            .iter()
            .all(|(id, _)| preview.removed.iter().any(|r| r.id == *id))
    );
    command(
        &mut w,
        ProjectCommand::Delete {
            preview,
            actions: BTreeMap::new(),
        },
    );
    for (id, _) in &actions {
        assert!(w.store.record(id).unwrap().unwrap().archived);
        assert!(!w.store.record_visible_in_chat(id, None).unwrap());
    }
    let r = w.store.record(&p).unwrap().unwrap();
    command(
        &mut w,
        ProjectCommand::Restore {
            project: p,
            version: r.version,
        },
    );
    for (id, expected) in actions {
        let r = w.store.record(&id).unwrap().unwrap();
        assert!(!r.archived);
        assert!(matches!(r.data, RecordData::Action(a) if a.state == expected));
        assert!(!w.store.record_visible_in_chat(&id, None).unwrap());
    }
}

#[test]
fn private_actions_on_shared_documents_need_explicit_choices_and_never_leak() {
    let (_root, mut w) = fixture();
    let p = project(&mut w, "Private tasks");
    let b = project(&mut w, "Other project");
    let c = chat(&mut w, &p);
    let sibling = chat(&mut w, &p);
    let cb = chat(&mut w, &b);
    let d = doc(&mut w, &p, "Shared document");
    command(
        &mut w,
        ProjectCommand::LinkDocument {
            project: b.clone(),
            note: d.clone(),
        },
    );
    let key = "private-action-shared-document";
    let op = w.store.allocate_operation(key).unwrap();
    let mut ids = vec![];
    let mut writes = vec![];
    for (index, state) in [
        ActionState::Open,
        ActionState::Waiting,
        ActionState::Done,
        ActionState::Open,
    ]
    .into_iter()
    .enumerate()
    {
        let id = op.creation_id(index * 2);
        ids.push(id.clone());
        writes.push(Put {
            id: id.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Action(Action {
                kept_outside_project: false,
                owner: Some("Private contact Mari".into()),
                waiting_on: Some("Confidential supplier approval".into()),
                follow_up: Some("2028-03-02".into()),
                chat: (index != 3).then_some(c.clone()),
                description: format!("Private obligation {index}"),
                state,
                internal: false,
                evidence: Some("Synthetic completed outcome".into()),
                due: None,
            }),
        });
        writes.push(Put {
            id: op.creation_id(index * 2 + 1),
            expected_version: None,
            archived: false,
            data: RecordData::Link(Link {
                from: d.clone(),
                to: id,
                relation: "task".into(),
            }),
        });
    }
    assert!(matches!(
        w.owner_change(
            key,
            &ChangeRequest {
                reason: "Synthetic private obligations".into(),
                inputs: vec![],
                writes
            }
        )
        .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let other_key = "foreign-private-action-on-shared-document";
    let other_op = w.store.allocate_operation(other_key).unwrap();
    let foreign_id = other_op.creation_id(0);
    assert!(matches!(
        w.owner_change(
            other_key,
            &ChangeRequest {
                reason: "Foreign private obligation".into(),
                inputs: vec![],
                writes: vec![
                    Put {
                        id: foreign_id.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Action(Action {
                            chat: Some(cb.clone()),
                            kept_outside_project: false,
                            owner: None,
                            waiting_on: None,
                            follow_up: None,
                            description: "Other project private task".into(),
                            state: ActionState::Open,
                            internal: false,
                            evidence: None,
                            due: None
                        })
                    },
                    Put {
                        id: other_op.creation_id(1),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Link(Link {
                            from: d.clone(),
                            to: foreign_id.clone(),
                            relation: "task".into()
                        })
                    },
                ]
            }
        )
        .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let foreign_before = w.store.record(&foreign_id).unwrap().unwrap();
    for id in &ids[..3] {
        assert!(seen(&w, id, &c));
        assert!(!seen(&w, id, &cb));
        assert!(!w.store.record_visible_in_chat(id, None).unwrap());
    }
    w.archive_chat("archive-action-origin", &c).unwrap();
    assert!(seen(&w, &ids[0], &sibling));
    let preview = w.project_delete_preview(&p).unwrap();
    assert_eq!(preview.unfinished_actions.len(), 3);
    assert!(
        !preview
            .unfinished_actions
            .iter()
            .any(|r| r.id == foreign_id)
    );
    assert!(preview.retained.iter().any(|r| r.id == d));
    assert!(preview.removed.iter().any(|r| r.id == ids[2]));
    assert!(
        w.project_command(
            "missing-shared-action-choice",
            ProjectCommand::Delete {
                preview: preview.clone(),
                actions: BTreeMap::new()
            }
        )
        .is_err()
    );
    command(
        &mut w,
        ProjectCommand::Delete {
            preview,
            actions: BTreeMap::from([
                (ids[0].clone(), ProjectActionChoice::Keep),
                (ids[1].clone(), ProjectActionChoice::Cancel),
                (ids[3].clone(), ProjectActionChoice::Keep),
            ]),
        },
    );
    assert!(w.store.record_visible_in_chat(&ids[0], None).unwrap());
    assert!(w.store.record_visible_in_chat(&ids[3], None).unwrap());
    assert!(!w.store.record_visible_in_chat(&d, None).unwrap());
    assert!(!seen(&w, &ids[1], &cb));
    assert!(w.store.record(&ids[1]).unwrap().unwrap().archived);
    assert!(w.store.record(&ids[2]).unwrap().unwrap().archived);
    let r = w.store.record(&p).unwrap().unwrap();
    command(
        &mut w,
        ProjectCommand::Restore {
            project: p,
            version: r.version,
        },
    );
    assert!(w.store.record_visible_in_chat(&ids[0], None).unwrap());
    assert!(w.store.record_visible_in_chat(&ids[3], None).unwrap());
    assert!(!seen(&w, &ids[1], &sibling));
    assert!(
        matches!(w.store.record(&ids[1]).unwrap().unwrap().data, RecordData::Action(a) if a.state == ActionState::Cancelled)
    );
    assert!(seen(&w, &ids[2], &sibling));
    assert!(!seen(&w, &ids[2], &cb));
    assert!(!w.store.record_visible_in_chat(&ids[2], None).unwrap());
    assert_eq!(
        w.store.record(&foreign_id).unwrap().unwrap(),
        foreign_before
    );
    assert!(seen(&w, &foreign_id, &cb));
    let second = w
        .project_delete_preview(
            &w.store
                .project_context_record(&sibling)
                .unwrap()
                .unwrap()
                .id,
        )
        .unwrap();
    assert!(second.unfinished_actions.is_empty());
}
