use brn_store::{
    Error, MAX_NOTE_BYTES, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{
        WorkTurnStatus,
        proposal_apply::{ApplyMemberProof, ApplyOutcome, ApprovalRequest},
        proposal_rewrite::{
            RewriteJob, RewriteOutcome, RewriteSpec, RewriteStatus, validate_result,
        },
        proposals::*,
    },
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::MetadataExt, path::Path};
use uuid::Uuid;

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn fingerprint(text: &str, inode: u64) -> FileFingerprint {
    FileFingerprint {
        device: 1,
        inode,
        len: text.len() as u64,
        sha256: digest(text.as_bytes()),
    }
}

struct Fixture {
    base: tempfile::TempDir,
    store: WorkStore,
    draft: ProposalDraft,
}

impl Fixture {
    fn new() -> Self {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = base.path().join("data");
        let vault = base.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::write(vault.join("old.md"), "\u{feff}old private baseline 🦀\r\n").unwrap();
        fs::write(vault.join("trash.md"), "kept archived bytes\r\n").unwrap();
        let meta = fs::metadata(&vault).unwrap();
        let parent = VaultIdentity {
            device: meta.dev(),
            inode: meta.ino(),
        };
        let draft = ProposalDraft {
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: Some(Uuid::new_v4()),
            session_id: Some(Uuid::new_v4()),
            vault: Some(VaultRecord {
                id: Uuid::new_v4(),
                root: vault,
                identity: parent.clone(),
            }),
            title: "Captured original title".into(),
            changes: vec![
                NoteChange::Create {
                    path: "new.md".into(),
                    parent: parent.clone(),
                    text: "α🦀日本語\r\nunique captured text".into(),
                },
                NoteChange::Replace {
                    path: "old.md".into(),
                    parent: parent.clone(),
                    before: fingerprint("\u{feff}old private baseline 🦀\r\n", 2),
                    before_text: "\u{feff}old private baseline 🦀\r\n".into(),
                    text: "proposed prior body".into(),
                },
                NoteChange::Trash {
                    path: "trash.md".into(),
                    parent,
                    before: fingerprint("kept archived bytes\r\n", 3),
                    before_text: "kept archived bytes\r\n".into(),
                },
            ],
            sources: vec![SourceVersion {
                path: "old.md".into(),
                fingerprint: fingerprint("\u{feff}old private baseline 🦀\r\n", 2),
            }],
            action_changes: Vec::new(),
        };
        let (store, _) = WorkStore::open(&data).unwrap();
        Self { base, store, draft }
    }

    fn data(&self) -> std::path::PathBuf {
        self.base.path().join("data")
    }

    fn review(&mut self) -> ProposalRecord {
        let first = self.store.create_proposal(&self.draft).unwrap();
        let anchored = self
            .store
            .add_proposal_comment(&CommentRequest {
                expected: first.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "unique temporary private comment 🦀\r\n".into(),
                    target: CommentTarget::Text(TextAnchor {
                        change_index: 0,
                        start: "α".len(),
                        end: "α🦀".len(),
                        quote: "🦀".into(),
                    }),
                },
            })
            .unwrap();
        self.store
            .add_proposal_comment(&CommentRequest {
                expected: anchored.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "unique whole proposal comment".into(),
                    target: CommentTarget::Proposal,
                },
            })
            .unwrap()
    }

    fn unchanged_vault(&self) {
        let vault = self.base.path().join("vault");
        assert!(!vault.join("new.md").exists());
        assert_eq!(
            fs::read(vault.join("old.md")).unwrap(),
            "\u{feff}old private baseline 🦀\r\n".as_bytes()
        );
        assert_eq!(
            fs::read(vault.join("trash.md")).unwrap(),
            b"kept archived bytes\r\n"
        );
    }
}

fn spec(record: &ProposalRecord) -> RewriteSpec {
    RewriteSpec {
        id: Uuid::new_v4(),
        expected: record.stamp(),
        provider: "chatgpt".into(),
        model: "gpt-5.5".into(),
        effort: "high".into(),
    }
}

fn edit(record: &ProposalRecord) -> ProposalEdit {
    ProposalEdit {
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: record
            .draft
            .changes
            .iter()
            .map(|change| change.text().map(str::to_owned))
            .collect(),
        action_data: Vec::new(),
    }
}

fn raw(data: &Path) -> Connection {
    Connection::open(data.join("brn.sqlite")).unwrap()
}

fn job_bytes(conn: &Connection, id: Uuid) -> Vec<u8> {
    conn.query_row(
        "SELECT job_json FROM proposal_rewrites WHERE id=?1",
        [id.to_string()],
        |row| row.get(0),
    )
    .unwrap()
}

#[test]
fn exact_full_capture_and_edit_are_atomic_and_jobs_never_copy_review_bodies() {
    let mut f = Fixture::new();
    let record = f.review();
    let request = spec(&record);
    let mut attached = f.store.chat_connection().unwrap();
    let (job, capture) = attached.begin_proposal_rewrite(&request).unwrap();
    assert_eq!(capture, Some(record.clone()));
    assert_eq!(
        job.capture_sha256,
        digest(&serde_json::to_vec(&record).unwrap())
    );
    assert_eq!(job.status, RewriteStatus::Running);
    assert_eq!(f.store.proposal_rewrite(request.id).unwrap(), Some(job));
    let mut result = edit(&record);
    result.title = "unique rewritten title".into();
    result.texts[0] =
        Some("\u{feff}---\r\ncustom: 🦀\r\n---\r\n日本語 русский\r\nunique result body".into());
    result.texts[1] = Some("\u{feff}русский результат\r\n".into());
    validate_result(&record, &result).unwrap();
    assert_eq!(
        f.store.proposal(record.draft.id).unwrap(),
        Some(record.clone())
    );
    let outcome = RewriteOutcome::Completed(result.clone());
    let finished = attached
        .finish_proposal_rewrite(request.id, &outcome)
        .unwrap();
    let rewritten = f.store.proposal(record.draft.id).unwrap().unwrap();
    assert_eq!(finished.status, RewriteStatus::Completed);
    assert_eq!(finished.result_stamp, Some(rewritten.stamp()));
    assert_eq!(rewritten.version, record.version + 1);
    assert_eq!(
        rewritten.draft.changes[0].text(),
        result.texts[0].as_deref()
    );
    assert_eq!(
        rewritten.draft.changes[1].text(),
        result.texts[1].as_deref()
    );
    assert_eq!(rewritten.draft.changes[2], record.draft.changes[2]);
    assert_eq!(rewritten.draft.vault, record.draft.vault);
    assert_eq!(rewritten.draft.sources, record.draft.sources);
    assert_eq!(rewritten.draft.group_id, record.draft.group_id);
    assert_eq!(rewritten.draft.session_id, record.draft.session_id);
    assert!(matches!(
        rewritten.comments[0].target,
        CommentTarget::Unresolved(_)
    ));
    assert_eq!(rewritten.comments[1], record.comments[1]);
    let conn = raw(&f.data());
    let bytes = job_bytes(&conn, request.id);
    assert!(bytes.len() <= 4096);
    let encoded = String::from_utf8(bytes).unwrap();
    for private in [
        "unique temporary private comment",
        "unique whole proposal comment",
        "unique captured text",
        "unique result body",
        "unique rewritten title",
        "old private baseline",
        "proposed prior body",
    ] {
        assert!(!encoded.contains(private), "job copied {private}");
    }
    assert_eq!(
        conn.query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    f.unchanged_vault();
}

#[test]
fn noop_completion_preserves_version_and_exact_anchor_and_replays_without_capture() {
    let mut f = Fixture::new();
    let record = f.review();
    let request = spec(&record);
    f.store.begin_proposal_rewrite(&request).unwrap();
    let outcome = RewriteOutcome::Completed(edit(&record));
    let finished = f
        .store
        .finish_proposal_rewrite(request.id, &outcome)
        .unwrap();
    assert_eq!(finished.status, RewriteStatus::Completed);
    assert_eq!(finished.result_stamp, Some(record.stamp()));
    assert_eq!(f.store.proposal(record.draft.id).unwrap(), Some(record));
    assert_eq!(
        f.store.begin_proposal_rewrite(&request).unwrap(),
        (finished.clone(), None)
    );
    assert_eq!(
        f.store
            .finish_proposal_rewrite(request.id, &outcome)
            .unwrap(),
        finished
    );
    f.unchanged_vault();
}

#[test]
fn late_results_after_comments_edits_rejection_application_or_missing_review_settle_stale() {
    for race in [
        "comment", "edit", "reject", "applying", "applied", "missing",
    ] {
        let mut f = Fixture::new();
        let record = f.review();
        let request = spec(&record);
        f.store.begin_proposal_rewrite(&request).unwrap();
        match race {
            "comment" => {
                f.store
                    .add_proposal_comment(&CommentRequest {
                        expected: record.stamp(),
                        comment: ReviewComment {
                            id: Uuid::new_v4(),
                            text: "later comment".into(),
                            target: CommentTarget::Proposal,
                        },
                    })
                    .unwrap();
            }
            "edit" => {
                let mut newer = edit(&record);
                newer.texts[0] = Some("later user typing".into());
                f.store.edit_proposal(&newer).unwrap();
            }
            "reject" => {
                f.store.reject_proposal(record.stamp()).unwrap();
            }
            "applying" | "applied" => {
                let journal = f
                    .store
                    .begin_proposal_apply(&ApprovalRequest {
                        operation_id: Uuid::new_v4(),
                        expected: record.stamp(),
                    })
                    .unwrap();
                if race == "applied" {
                    let prepared: Vec<_> = record
                        .draft
                        .changes
                        .iter()
                        .enumerate()
                        .map(|(i, change)| match change {
                            NoteChange::Create { text, .. } | NoteChange::Replace { text, .. } => {
                                fingerprint(text, 100 + i as u64)
                            }
                            NoteChange::Trash { before, .. } => before.clone(),
                        })
                        .collect();
                    f.store
                        .record_proposal_prepared(journal.request.operation_id, &prepared)
                        .unwrap();
                    let observations: Vec<_> = record
                        .draft
                        .changes
                        .iter()
                        .zip(prepared)
                        .map(|(change, new)| match change {
                            NoteChange::Create { .. } => ApplyMemberProof {
                                destination: Some(new),
                                staging: None,
                            },
                            NoteChange::Replace { before, .. } => ApplyMemberProof {
                                destination: Some(new),
                                staging: Some(before.clone()),
                            },
                            NoteChange::Trash { before, .. } => ApplyMemberProof {
                                destination: None,
                                staging: Some(before.clone()),
                            },
                        })
                        .collect();
                    f.store
                        .finish_proposal_apply(
                            journal.request.operation_id,
                            ApplyOutcome::Applied,
                            Some(&observations),
                        )
                        .unwrap();
                }
            }
            "missing" => {
                raw(&f.data())
                    .execute(
                        "DELETE FROM proposals WHERE id=?1",
                        [record.draft.id.to_string()],
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let newer = f.store.proposal(record.draft.id).unwrap();
        let mut late = edit(&record);
        late.texts[0] = Some("late provider result".into());
        let outcome = RewriteOutcome::Completed(late);
        let finished = f
            .store
            .finish_proposal_rewrite(request.id, &outcome)
            .unwrap();
        assert_eq!(finished.status, RewriteStatus::Stale, "{race}");
        assert!(finished.result_stamp.is_none());
        assert_eq!(f.store.proposal(record.draft.id).unwrap(), newer, "{race}");
        assert_eq!(
            f.store
                .finish_proposal_rewrite(request.id, &outcome)
                .unwrap(),
            finished
        );
        assert_eq!(
            f.store.begin_proposal_rewrite(&request).unwrap(),
            (finished, None)
        );
        f.unchanged_vault();
    }
}

#[test]
fn uuid_spec_and_terminal_outcome_are_bound_before_current_review_access() {
    let mut f = Fixture::new();
    let record = f.review();
    let request = spec(&record);
    let (running, _) = f.store.begin_proposal_rewrite(&request).unwrap();
    assert_eq!(
        f.store.begin_proposal_rewrite(&request).unwrap(),
        (running.clone(), None)
    );
    for field in ["provider", "model", "effort", "version", "proposal"] {
        let mut other = request.clone();
        match field {
            "provider" => other.provider = "copilot".into(),
            "model" => other.model = "other-model".into(),
            "effort" => other.effort = "low".into(),
            "version" => other.expected.version += 1,
            "proposal" => other.expected.id = Uuid::new_v4(),
            _ => unreachable!(),
        }
        assert!(matches!(
            f.store.begin_proposal_rewrite(&other),
            Err(Error::OperationConflict(_))
        ));
    }
    let outcome = RewriteOutcome::Failed("network".into());
    let finished = f
        .store
        .finish_proposal_rewrite(request.id, &outcome)
        .unwrap();
    raw(&f.data())
        .execute(
            "DELETE FROM proposals WHERE id=?1",
            [record.draft.id.to_string()],
        )
        .unwrap();
    assert_eq!(
        f.store.begin_proposal_rewrite(&request).unwrap(),
        (finished.clone(), None)
    );
    assert_eq!(
        f.store
            .finish_proposal_rewrite(request.id, &outcome)
            .unwrap(),
        finished
    );
    for other in [
        RewriteOutcome::Interrupted,
        RewriteOutcome::Failed("storage".into()),
        RewriteOutcome::Completed(edit(&record)),
    ] {
        assert!(matches!(
            f.store.finish_proposal_rewrite(request.id, &other),
            Err(Error::OperationConflict(_))
        ));
    }
}

#[test]
fn invalid_results_and_untrusted_failure_strings_cannot_mutate_or_settle_work() {
    let mut f = Fixture::new();
    let record = f.review();
    let request = spec(&record);
    let (running, _) = f.store.begin_proposal_rewrite(&request).unwrap();
    let mut invalid = vec![];
    let mut wrong_count = edit(&record);
    wrong_count.texts.pop();
    invalid.push(wrong_count);
    let mut wrong_kind = edit(&record);
    wrong_kind.texts[2] = Some("never change a Trash into Create".into());
    invalid.push(wrong_kind);
    let mut oversized = edit(&record);
    oversized.texts[0] = Some("x".repeat(MAX_NOTE_BYTES + 1));
    invalid.push(oversized);
    let mut blank = edit(&record);
    blank.title.clear();
    invalid.push(blank);
    for result in invalid {
        assert!(validate_result(&record, &result).is_err());
        assert!(
            f.store
                .finish_proposal_rewrite(request.id, &RewriteOutcome::Completed(result))
                .is_err()
        );
        assert_eq!(
            f.store.proposal_rewrite(request.id).unwrap(),
            Some(running.clone())
        );
        assert_eq!(
            f.store.proposal(record.draft.id).unwrap(),
            Some(record.clone())
        );
    }
    let mut mismatched = edit(&record);
    mismatched.expected.version += 1;
    assert!(matches!(
        f.store
            .finish_proposal_rewrite(request.id, &RewriteOutcome::Completed(mismatched)),
        Err(Error::OperationConflict(_))
    ));
    assert!(matches!(
        f.store.finish_proposal_rewrite(
            request.id,
            &RewriteOutcome::Failed("raw provider error secret".into())
        ),
        Err(Error::Invalid(_))
    ));
    let failed = f
        .store
        .finish_proposal_rewrite(request.id, &RewriteOutcome::Failed("tool_rejected".into()))
        .unwrap();
    assert_eq!(failed.status, RewriteStatus::Failed);
    assert_eq!(failed.error_code.as_deref(), Some("tool_rejected"));
    f.unchanged_vault();
}

#[test]
fn same_stamp_drift_of_the_full_capture_settles_stale_without_overwriting_it() {
    let mut f = Fixture::new();
    let record = f.review();
    let request = spec(&record);
    f.store.begin_proposal_rewrite(&request).unwrap();
    let conn = raw(&f.data());
    let bytes: Vec<u8> = conn
        .query_row(
            "SELECT record_json FROM proposals WHERE id=?1",
            [record.draft.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["record"]["comments"][0]["text"] = "restored same-stamp comment differs".into();
    let bytes = serde_json::to_vec(&value).unwrap();
    conn.execute(
        "UPDATE proposals SET record_json=?2,record_sha256=?3 WHERE id=?1",
        params![
            record.draft.id.to_string(),
            bytes,
            digest(&bytes).as_slice()
        ],
    )
    .unwrap();
    let restored = f.store.proposal(record.draft.id).unwrap().unwrap();
    assert_eq!(restored.stamp(), record.stamp());
    assert_ne!(restored.comments, record.comments);
    let mut late = edit(&record);
    late.texts[0] = Some("would overwrite the restored review".into());
    let settled = f
        .store
        .finish_proposal_rewrite(request.id, &RewriteOutcome::Completed(late))
        .unwrap();
    assert_eq!(settled.status, RewriteStatus::Stale);
    assert_eq!(f.store.proposal(record.draft.id).unwrap(), Some(restored));
    f.unchanged_vault();
}

#[test]
fn fresh_specs_are_validated_without_inserting_jobs_or_mutating_review() {
    let mut f = Fixture::new();
    let record = f.review();
    for invalid in ["nil", "nil_proposal", "zero", "provider", "model", "effort"] {
        let mut request = spec(&record);
        match invalid {
            "nil" => request.id = Uuid::nil(),
            "nil_proposal" => request.expected.id = Uuid::nil(),
            "zero" => request.expected.version = 0,
            "provider" => request.provider = "other-provider".into(),
            "model" => request.model = "unsafe\nmodel".into(),
            "effort" => request.effort = "ultra".into(),
            _ => unreachable!(),
        }
        assert!(matches!(
            f.store.begin_proposal_rewrite(&request),
            Err(Error::Invalid(_))
        ));
        assert!(f.store.proposal_rewrite(request.id).unwrap().is_none());
    }
    assert_eq!(f.store.proposal(record.draft.id).unwrap(), Some(record));
    assert_eq!(
        raw(&f.data())
            .query_row("SELECT count(*) FROM proposal_rewrites", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn pure_validation_counts_retained_before_text_and_comments_in_the_aggregate() {
    let mut f = Fixture::new();
    let parent = f.draft.vault.as_ref().unwrap().identity.clone();
    let before = "b".repeat(MAX_NOTE_BYTES);
    f.draft.changes = (0..7)
        .map(|i| NoteChange::Create {
            path: format!("new-{i}.md"),
            parent: parent.clone(),
            text: "small".into(),
        })
        .collect();
    f.draft.changes.push(NoteChange::Replace {
        path: "old.md".into(),
        parent,
        before: fingerprint(&before, 2),
        before_text: before,
        text: "small".into(),
    });
    let record = f.store.create_proposal(&f.draft).unwrap();
    let request = spec(&record);
    let (running, _) = f.store.begin_proposal_rewrite(&request).unwrap();
    let mut oversized = edit(&record);
    oversized.texts = vec![Some("x".repeat(MAX_NOTE_BYTES - 512)); 8];
    assert!(matches!(
        validate_result(&record, &oversized),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        f.store
            .finish_proposal_rewrite(request.id, &RewriteOutcome::Completed(oversized)),
        Err(Error::Invalid(_))
    ));
    assert_eq!(f.store.proposal(record.draft.id).unwrap(), Some(record));
    assert_eq!(f.store.proposal_rewrite(request.id).unwrap(), Some(running));
    f.unchanged_vault();
}

#[test]
fn result_and_job_roll_back_together_on_finalization_failure() {
    let mut f = Fixture::new();
    let record = f.review();
    let request = spec(&record);
    let (running, _) = f.store.begin_proposal_rewrite(&request).unwrap();
    let conn = raw(&f.data());
    conn.execute_batch("CREATE TRIGGER refuse_rewrite_finish BEFORE UPDATE ON proposal_rewrites BEGIN SELECT RAISE(ABORT, 'synthetic-finalization-failure'); END;").unwrap();
    let mut result = edit(&record);
    result.texts[0] = Some("should roll back".into());
    let outcome = RewriteOutcome::Completed(result);
    assert!(matches!(
        f.store.finish_proposal_rewrite(request.id, &outcome),
        Err(Error::Sql(_))
    ));
    assert_eq!(
        f.store.proposal(record.draft.id).unwrap(),
        Some(record.clone())
    );
    assert_eq!(f.store.proposal_rewrite(request.id).unwrap(), Some(running));
    conn.execute_batch("DROP TRIGGER refuse_rewrite_finish")
        .unwrap();
    assert_eq!(
        f.store
            .finish_proposal_rewrite(request.id, &outcome)
            .unwrap()
            .status,
        RewriteStatus::Completed
    );
    f.unchanged_vault();
}

#[test]
fn chat_and_rewrite_ids_cannot_alias_through_owner_or_attached_connections() {
    let mut f = Fixture::new();
    let record = f.review();
    let mut attached = f.store.chat_connection().unwrap();
    let request = spec(&record);
    attached.begin_proposal_rewrite(&request).unwrap();
    assert!(matches!(
        f.store
            .begin_turn(request.id, None, "chat", "chatgpt", "gpt-5.5"),
        Err(Error::OperationConflict(_))
    ));
    let mut exhausted = record.clone();
    exhausted.version = u64::MAX;
    let mut would_overflow = edit(&exhausted);
    would_overflow.title = "changed after last version".into();
    assert!(matches!(
        validate_result(&exhausted, &would_overflow),
        Err(Error::Invalid(_))
    ));
    validate_result(&exhausted, &edit(&exhausted)).unwrap();
    assert!(f.store.turn(request.id).unwrap().is_none());
    let mut second = spec(&record);
    let turn = attached
        .begin_turn(second.id, None, "ordinary turn", "chatgpt", "gpt-5.5")
        .unwrap();
    assert!(matches!(
        f.store.begin_proposal_rewrite(&second),
        Err(Error::OperationConflict(_))
    ));
    assert!(attached.proposal_rewrite(second.id).unwrap().is_none());
    attached
        .finish_turn(turn.id, WorkTurnStatus::Completed, "answer", None)
        .unwrap();
    assert!(matches!(
        attached.begin_proposal_rewrite(&second),
        Err(Error::OperationConflict(_))
    ));
    second.id = Uuid::new_v4();
    assert!(attached.begin_proposal_rewrite(&second).is_ok());
}

fn mutate_job(
    conn: &Connection,
    id: Uuid,
    change: impl FnOnce(&mut serde_json::Value),
    rehash: bool,
) {
    let mut value: serde_json::Value = serde_json::from_slice(&job_bytes(conn, id)).unwrap();
    change(&mut value);
    let bytes = serde_json::to_vec(&value).unwrap();
    if rehash {
        conn.execute(
            "UPDATE proposal_rewrites SET job_json=?2,job_sha256=?3 WHERE id=?1",
            params![id.to_string(), bytes, digest(&bytes).as_slice()],
        )
        .unwrap();
    } else {
        conn.execute(
            "UPDATE proposal_rewrites SET job_json=?2 WHERE id=?1",
            params![id.to_string(), bytes],
        )
        .unwrap();
    }
}

#[test]
fn startup_checks_every_bounded_hash_schema_identity_and_status_before_interruption() {
    for tamper in [
        "hash",
        "schema",
        "oversize",
        "uuid",
        "effort",
        "status",
        "timestamp",
        "result",
        "error",
        "outcome",
        "collision",
    ] {
        let mut f = Fixture::new();
        let record = f.review();
        let first = spec(&record);
        let second = spec(&record);
        f.store.begin_proposal_rewrite(&first).unwrap();
        f.store.begin_proposal_rewrite(&second).unwrap();
        let data = f.data();
        let base = f.base;
        drop(f.store);
        let conn = raw(&data);
        match tamper {
            "oversize" => {
                conn.execute(
                    "UPDATE proposal_rewrites SET job_json=?2,job_sha256=?3 WHERE id=?1",
                    params![
                        second.id.to_string(),
                        vec![b' '; 4097],
                        digest(&vec![b' '; 4097]).as_slice()
                    ],
                )
                .unwrap();
            }
            "collision" => {
                let conversation = Uuid::new_v4();
                conn.execute(
                    "INSERT INTO conversations(id,title,created_at_ms) VALUES(?1,'synthetic',1)",
                    [conversation.to_string()],
                )
                .unwrap();
                conn.execute("INSERT INTO messages(turn_id,conversation_id,sequence,role,text,provider,model,status) VALUES(?1,?2,1,'user','x','chatgpt','gpt-5.5','completed'),(?1,?2,1,'assistant','y','chatgpt','gpt-5.5','completed')", params![second.id.to_string(), conversation.to_string()]).unwrap();
            }
            _ => mutate_job(
                &conn,
                second.id,
                |value| match tamper {
                    "hash" => value["spec"]["effort"] = "low".into(),
                    "schema" => value["raw_prompt"] = "must refuse unknown fields".into(),
                    "uuid" => value["spec"]["id"] = Uuid::new_v4().to_string().into(),
                    "effort" => value["spec"]["effort"] = "ultra".into(),
                    "status" => value["status"] = "completed".into(),
                    "timestamp" => value["started_at_ms"] = 0.into(),
                    "result" => {
                        value["result_stamp"] = serde_json::to_value(record.stamp()).unwrap()
                    }
                    "error" => value["error_code"] = "network".into(),
                    "outcome" => value["outcome_sha256"] = serde_json::to_value([0u8; 32]).unwrap(),
                    _ => unreachable!(),
                },
                tamper != "hash",
            ),
        }
        let before = job_bytes(&conn, first.id);
        let backups = fs::read_dir(data.join("backups")).unwrap().count();
        assert!(
            matches!(WorkStore::open(&data), Err(Error::Invalid(_))),
            "{tamper}"
        );
        assert_eq!(job_bytes(&conn, first.id), before, "{tamper}");
        assert_eq!(
            fs::read_dir(data.join("backups")).unwrap().count(),
            backups,
            "{tamper}"
        );
        drop(conn);
        drop(base);
    }
}

#[test]
fn restart_interrupts_once_and_replay_never_returns_another_capture() {
    let mut f = Fixture::new();
    let record = f.review();
    let request = spec(&record);
    let (running, _) = f.store.begin_proposal_rewrite(&request).unwrap();
    let data = f.data();
    let _base = f.base;
    drop(f.store);
    let (mut store, report) = WorkStore::open(&data).unwrap();
    let interrupted = store.proposal_rewrite(request.id).unwrap().unwrap();
    assert_eq!(interrupted.status, RewriteStatus::Interrupted);
    assert_eq!(interrupted.capture_sha256, running.capture_sha256);
    assert!(interrupted.finished_at_ms.unwrap() >= running.started_at_ms);
    assert_eq!(
        store.proposal(record.draft.id).unwrap(),
        Some(record.clone())
    );
    assert_eq!(
        store.begin_proposal_rewrite(&request).unwrap(),
        (interrupted.clone(), None)
    );
    assert_eq!(
        store
            .finish_proposal_rewrite(request.id, &RewriteOutcome::Interrupted)
            .unwrap(),
        interrupted
    );
    let backup = Connection::open(report.backup).unwrap();
    let stored: RewriteJob = serde_json::from_slice(&job_bytes(&backup, request.id)).unwrap();
    assert_eq!(stored, interrupted, "open backup follows interruption");
    drop(backup);
    drop(store);
    let (store, _) = WorkStore::open(&data).unwrap();
    assert_eq!(
        store.proposal_rewrite(request.id).unwrap(),
        Some(interrupted)
    );
}

#[test]
fn additive_v5_migration_and_backup_restore_preserve_work_and_terminal_rewrite() {
    let mut f = Fixture::new();
    let record = f.review();
    f.store
        .put_unsaved_edit("legacy.md", [1; 32], "old recovered typing\r\n")
        .unwrap();
    f.store.set_setting("synthetic", "kept").unwrap();
    let data = f.data();
    let _base = f.base;
    drop(f.store);
    let conn = raw(&data);
    conn.execute_batch("DROP TABLE inbox_processing; DROP TABLE inbox_items; DROP TABLE action_completions; DROP TABLE actions; DROP TABLE findings; ALTER TABLE messages DROP COLUMN started_at_ms; ALTER TABLE messages DROP COLUMN finished_at_ms; ALTER TABLE conversations DROP COLUMN last_activity_at_ms; ALTER TABLE messages DROP COLUMN effort; DROP TABLE proposal_rewrites; PRAGMA user_version=5;")
        .unwrap();
    drop(conn);
    let (mut store, _) = WorkStore::open(&data).unwrap();
    assert_eq!(
        store.proposal(record.draft.id).unwrap(),
        Some(record.clone())
    );
    assert_eq!(
        store.unsaved_edit("legacy.md").unwrap().unwrap().text,
        "old recovered typing\r\n"
    );
    let request = spec(&record);
    store.begin_proposal_rewrite(&request).unwrap();
    let completed = store
        .finish_proposal_rewrite(request.id, &RewriteOutcome::Completed(edit(&record)))
        .unwrap();
    drop(store);
    drop(WorkStore::open(&data).unwrap()); // validated backup includes the terminal job
    fs::write(data.join("brn.sqlite"), b"synthetic corruption").unwrap();
    let (store, report) = WorkStore::open(&data).unwrap();
    assert!(report.restored_from.is_some());
    assert_eq!(store.proposal_rewrite(request.id).unwrap(), Some(completed));
    assert_eq!(store.proposal(record.draft.id).unwrap(), Some(record));
    assert_eq!(store.setting("synthetic").unwrap().as_deref(), Some("kept"));
    assert_eq!(
        store.unsaved_edit("legacy.md").unwrap().unwrap().text,
        "old recovered typing\r\n"
    );
    assert_eq!(
        raw(&data)
            .query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        13
    );
}
