use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{findings::*, proposals::SourceVersion},
};
use sha2::{Digest, Sha256};
use uuid::Uuid;
const TEXT: &str =
    "\u{feff}---\r\nbrn_id: 11111111-1111-4111-8111-111111111111\r\n---\r\n日本語 λ\r\n";
fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn draft() -> FindingDraft {
    let id = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
    FindingDraft {
        request: CaptureFindingRequest {
            id: Uuid::new_v4(),
            origin: FindingOrigin::IdentityAmbiguity { note_id: id },
        },
        vault: VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: VaultIdentity {
                device: 1,
                inode: 1,
            },
        },
        title: "Identity conflict 日本語".into(),
        summary: "Two saved notes have the same identity. λ\r\n".into(),
        evidence: ["current.md", "archive/資料.md"]
            .iter()
            .enumerate()
            .map(|(i, path)| FindingEvidence {
                source: SourceVersion {
                    path: (*path).into(),
                    fingerprint: FileFingerprint {
                        device: 1,
                        inode: 2 + i as u64,
                        len: TEXT.len() as u64,
                        sha256: digest(TEXT.as_bytes()),
                    },
                },
                note_id: Some(id),
                quote: Some(FindingQuote {
                    start_byte: 0,
                    end_byte: TEXT.len(),
                    quote: TEXT.into(),
                }),
            })
            .collect(),
    }
}
#[test]
fn creation_and_terminal_replay_retain_exact_evidence_and_never_reopen() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let input = draft();
    input.validate().unwrap();
    let first = store
        .create_finding(&input)
        .expect("admit tentative finding");
    assert_eq!(first.draft, input);
    assert_eq!(first.version, 1);
    assert_eq!(first.state, FindingState::Open);
    assert_eq!(first.created_at_ms, first.updated_at_ms);
    let close = CloseFindingRequest {
        expected: first.stamp(),
        state: FindingState::Resolved,
    };
    let terminal = store.close_finding(&close).unwrap();
    assert_eq!(terminal.version, 2);
    assert_eq!(terminal.state, FindingState::Resolved);
    assert_eq!(terminal.draft, input);
    assert_eq!(store.close_finding(&close).unwrap(), terminal);
    assert_eq!(store.create_finding(&input).unwrap(), terminal);
    let mut changed = input.clone();
    changed.summary.push_str("changed");
    assert!(matches!(
        store.create_finding(&changed),
        Err(Error::OperationConflict(_))
    ));
    let competing = CloseFindingRequest {
        state: FindingState::Dismissed,
        ..close.clone()
    };
    assert!(matches!(
        store.close_finding(&competing),
        Err(Error::StateChanged(_))
    ));
    drop(store);
    let (store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.finding(input.request.id).unwrap(), Some(terminal));
    assert_eq!(
        store
            .findings(&FindingListRequest::default())
            .unwrap()
            .open_count,
        0
    );
}

#[test]
fn distinct_case_sensitive_paths_retain_their_exact_saved_file_proofs() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut input = draft();
    input.evidence[0].source.path = "Note.md".into();
    input.evidence[1].source.path = "note.md".into();
    assert_ne!(
        input.evidence[0].source.fingerprint.inode,
        input.evidence[1].source.fingerprint.inode
    );
    input
        .validate()
        .expect("distinct saved paths are not case-folded aliases");
    let record = store.create_finding(&input).unwrap();
    drop(store);
    let (store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.finding(input.request.id).unwrap(), Some(record));
}

fn raw(data: &std::path::Path) -> rusqlite::Connection {
    rusqlite::Connection::open(data.join("brn.sqlite")).unwrap()
}
fn rewrite(conn: &rusqlite::Connection, record: &FindingRecord) {
    let bytes = serde_json::to_vec(record).unwrap();
    conn.execute(
        "UPDATE findings SET state=?2,created_at_ms=?3,record_json=?4,record_sha256=?5 WHERE id=?1",
        rusqlite::params![
            record.draft.request.id.to_string(),
            match record.state {
                FindingState::Open => "open",
                FindingState::Resolved => "resolved",
                FindingState::Dismissed => "dismissed",
            },
            record.created_at_ms as i64,
            bytes,
            digest(&bytes).as_slice()
        ],
    )
    .unwrap();
}
fn link_draft() -> FindingDraft {
    let text = "\u{feff}[λ][proof]\r\n\r\n[proof]: missing.md\r\n";
    let mut draft = draft();
    let source = SourceVersion {
        path: "archive/source.md".into(),
        fingerprint: FileFingerprint {
            device: 1,
            inode: 2,
            len: text.len() as u64,
            sha256: digest(text.as_bytes()),
        },
    };
    let occurrence = text.find("[λ]").unwrap();
    draft.request.origin = FindingOrigin::UnresolvedLink {
        path: source.path.clone(),
        source_sha256: source.fingerprint.sha256,
        destination: "missing.md".into(),
        start_byte: occurrence,
    };
    draft.evidence = vec![
        FindingEvidence {
            source: source.clone(),
            note_id: None,
            quote: Some(FindingQuote {
                start_byte: occurrence,
                end_byte: text.find('\r').unwrap(),
                quote: "[λ][proof]".into(),
            }),
        },
        FindingEvidence {
            source,
            note_id: None,
            quote: Some(FindingQuote {
                start_byte: text.find("[proof]:").unwrap(),
                end_byte: text.len() - 2,
                quote: "[proof]: missing.md".into(),
            }),
        },
    ];
    draft
}
#[test]
fn replay_does_not_refresh_known_creation_or_terminal_times_and_both_outcomes_close_once() {
    for outcome in [FindingState::Resolved, FindingState::Dismissed] {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let input = draft();
        let mut old = store.create_finding(&input).unwrap();
        old.created_at_ms = 1;
        old.updated_at_ms = 1;
        rewrite(&raw(data.path()), &old);
        assert_eq!(store.create_finding(&input).unwrap(), old);
        let close = CloseFindingRequest {
            expected: old.stamp(),
            state: outcome,
        };
        let mut terminal = store.close_finding(&close).unwrap();
        assert!(terminal.updated_at_ms > 1);
        terminal.updated_at_ms = 2;
        rewrite(&raw(data.path()), &terminal);
        assert_eq!(store.close_finding(&close).unwrap(), terminal);
        assert_eq!(store.create_finding(&input).unwrap(), terminal);
        assert!(matches!(
            store.close_finding(&CloseFindingRequest {
                expected: terminal.stamp(),
                state: outcome
            }),
            Err(Error::StateChanged(_))
        ));
        let bad = CloseFindingRequest {
            expected: old.stamp(),
            state: FindingState::Open,
        };
        assert!(matches!(store.close_finding(&bad), Err(Error::Invalid(_))));
    }
}
#[test]
fn unresolved_reference_keeps_occurrence_and_used_definition_from_one_source() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let input = link_draft();
    input.validate().unwrap();
    let first = store.create_finding(&input).unwrap();
    assert_eq!(first.draft.evidence, input.evidence);
    assert_eq!(
        first.draft.evidence[0].source,
        first.draft.evidence[1].source
    );
    for change in 0..6 {
        let mut bad = input.clone();
        match change {
            0 => bad.evidence[1].source.fingerprint.inode += 1,
            1 => bad.evidence[1].source.fingerprint.sha256 = [0; 32],
            2 => bad.evidence[1].source.path = "other.md".into(),
            3 => bad.evidence[0].quote = None,
            4 => bad.evidence[0].quote.as_mut().unwrap().start_byte += 1,
            _ => bad.evidence[1] = bad.evidence[0].clone(),
        };
        assert!(bad.validate().is_err(), "change {change}");
    }
    // Range shape is pure validation; original UTF-8 boundary qualification is
    // intentionally owned by workflow's coordinated full-source observation.
    let mut shape = input.clone();
    shape.evidence.truncate(1);
    shape.evidence[0].quote = Some(FindingQuote {
        start_byte: 3,
        end_byte: 5,
        quote: "λ".into(),
    });
    shape.validate().unwrap();
}
#[test]
fn strict_json_refuses_unknown_and_duplicate_fields_including_nested_old_proof_types() {
    let input = draft();
    let mut value = serde_json::to_value(&input).unwrap();
    for pointer in [
        "/",
        "/request",
        "/request/origin",
        "/vault",
        "/vault/identity",
        "/evidence/0",
        "/evidence/0/source",
        "/evidence/0/source/fingerprint",
        "/evidence/0/quote",
    ] {
        let mut bad = value.clone();
        let object = if pointer == "/" {
            bad.as_object_mut().unwrap()
        } else {
            bad.pointer_mut(pointer).unwrap().as_object_mut().unwrap()
        };
        object.insert("unexpected".into(), true.into());
        assert!(
            serde_json::from_value::<FindingDraft>(bad).is_err(),
            "{pointer}"
        );
    }
    let encoded = serde_json::to_string(&input).unwrap();
    let duplicate = encoded.replacen("\"summary\":", "\"summary\":\"duplicate\",\"summary\":", 1);
    assert!(serde_json::from_str::<FindingDraft>(&duplicate).is_err());
    value["request"]["origin"]["kind"] = "unknown".into();
    assert!(serde_json::from_value::<FindingDraft>(value).is_err());
    for text in [
        r#"{"state":null,"limit":25,"before":null,"unknown":1}"#,
        r#"{"state":"open","limit":25,"limit":25,"before":null}"#,
    ] {
        assert!(serde_json::from_str::<FindingListRequest>(text).is_err());
    }
    assert!(serde_json::from_value::<CloseFindingRequest>(serde_json::json!({"expected":{"id":input.request.id,"version":1,"extra":true},"state":"resolved"})).is_err());
}
#[test]
fn shape_limits_refuse_without_inserting_or_clipping_evidence() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let input = draft();
    for change in 0..16 {
        let mut bad = input.clone();
        match change {
            0 => bad.request.id = Uuid::nil(),
            1 => bad.vault.id = Uuid::nil(),
            2 => bad.title = " ".into(),
            3 => bad.title = "x".repeat(513),
            4 => bad.summary = "x".repeat(16 * 1024 + 1),
            5 => bad.evidence.clear(),
            6 => bad.evidence.truncate(1),
            7 => bad.evidence[1].note_id = Some(Uuid::nil()),
            8 => bad.evidence[1].note_id = Some(Uuid::new_v4()),
            9 => bad.evidence[1].source.path = bad.evidence[0].source.path.clone(),
            10 => bad.evidence[0].source.fingerprint.len = brn_store::MAX_NOTE_BYTES as u64 + 1,
            11 => bad.evidence[0].quote.as_mut().unwrap().quote.clear(),
            12 => bad.evidence[0].quote.as_mut().unwrap().end_byte += 1,
            13 => bad.evidence[0].quote.as_mut().unwrap().quote = "x".repeat(16 * 1024 + 1),
            14 => bad.vault.root = "relative".into(),
            _ => bad.vault.root = "/bad\0root".into(),
        };
        assert!(bad.validate().is_err(), "change {change}");
        assert!(store.create_finding(&bad).is_err());
    }
    for invalid in [
        "/absolute.md",
        "../bad.md",
        "a/../bad.md",
        "a//bad.md",
        ".hidden.md",
        "a/.hidden/bad.md",
        ".brn-1.stage",
        "note.txt",
        "note\n.md",
        "note\r.md",
        "note\0.md",
        "a\\b.md",
    ] {
        let mut bad = input.clone();
        bad.evidence[0].source.path = invalid.into();
        assert!(bad.validate().is_err(), "path {invalid:?}");
    }
    let mut capped = input.clone();
    capped.title = "x".repeat(512);
    capped.summary = "s".repeat(16 * 1024);
    for evidence in &mut capped.evidence {
        evidence.quote = None;
        evidence.source.fingerprint.len = 0;
        evidence.source.fingerprint.sha256 = digest(b"");
    }
    capped.validate().unwrap();
    let id = capped.evidence[0].note_id;
    capped.evidence = (0..64)
        .map(|i| FindingEvidence {
            source: SourceVersion {
                path: format!("{i}.md"),
                fingerprint: FileFingerprint {
                    device: 1,
                    inode: 2 + i,
                    len: brn_store::MAX_NOTE_BYTES as u64,
                    sha256: [1; 32],
                },
            },
            note_id: id,
            quote: Some(FindingQuote {
                start_byte: 0,
                end_byte: 16 * 1024,
                quote: "x".repeat(16 * 1024),
            }),
        })
        .collect();
    assert!(capped.validate().is_err());
    capped.evidence.truncate(62);
    capped.validate().unwrap();
    capped.evidence.extend((62..65).map(|i| FindingEvidence {
        source: SourceVersion {
            path: format!("{i}.md"),
            fingerprint: FileFingerprint {
                device: 1,
                inode: 2 + i,
                len: 0,
                sha256: digest(b""),
            },
        },
        note_id: id,
        quote: None,
    }));
    assert!(capped.validate().is_err());
    assert!(
        store
            .findings(&FindingListRequest::default())
            .unwrap()
            .entries
            .is_empty()
    );
}
#[test]
fn list_filters_have_exact_descending_cursors_even_after_cursor_closure() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let mut retained = Vec::new();
    for i in 1..=6 {
        let mut input = draft();
        input.request.id = Uuid::from_u128(i);
        let mut record = store.create_finding(&input).unwrap();
        record.created_at_ms = if i == 1 { 100 } else { 200 };
        record.updated_at_ms = record.created_at_ms;
        rewrite(&raw(data.path()), &record);
        retained.push(record);
    }
    let request = FindingListRequest {
        limit: 2,
        ..Default::default()
    };
    let first = store.findings(&request).unwrap();
    assert_eq!(
        first
            .entries
            .iter()
            .map(|r| r.draft.request.id)
            .collect::<Vec<_>>(),
        vec![Uuid::from_u128(6), Uuid::from_u128(5)]
    );
    assert_eq!(first.open_count, 6);
    assert_eq!(first.next_before, Some(Uuid::from_u128(5)));
    store
        .close_finding(&CloseFindingRequest {
            expected: retained[4].stamp(),
            state: FindingState::Dismissed,
        })
        .unwrap();
    let second = store
        .findings(&FindingListRequest {
            before: first.next_before,
            ..request.clone()
        })
        .unwrap();
    assert_eq!(
        second
            .entries
            .iter()
            .map(|r| r.draft.request.id)
            .collect::<Vec<_>>(),
        vec![Uuid::from_u128(4), Uuid::from_u128(3)]
    );
    assert_eq!(second.open_count, 5);
    let last = store
        .findings(&FindingListRequest {
            before: second.next_before,
            ..request.clone()
        })
        .unwrap();
    assert_eq!(last.entries.len(), 2);
    assert_eq!(last.entries[1].draft.request.id, Uuid::from_u128(1));
    assert_eq!(last.next_before, None);
    let closed = store
        .findings(&FindingListRequest {
            state: Some(FindingState::Dismissed),
            limit: 100,
            before: None,
        })
        .unwrap();
    assert_eq!(closed.entries.len(), 1);
    assert_eq!(closed.open_count, 5);
    let all = store
        .findings(&FindingListRequest {
            state: None,
            ..request.clone()
        })
        .unwrap();
    assert_eq!(all.entries.len(), 2);
    assert!(matches!(
        store.findings(&FindingListRequest {
            before: Some(Uuid::new_v4()),
            ..request
        }),
        Err(Error::NotFound(_))
    ));
    for limit in [0, 101, usize::MAX] {
        assert!(
            FindingListRequest {
                limit,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        FindingListRequest {
            before: Some(Uuid::nil()),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}
#[test]
fn sql_failures_rollback_creation_and_terminal_state_together() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let conn = raw(data.path());
    let input = draft();
    conn.execute_batch("CREATE TRIGGER refuse_finding_insert AFTER INSERT ON findings BEGIN SELECT RAISE(ABORT,'synthetic admission failure'); END;").unwrap();
    assert!(store.create_finding(&input).is_err());
    assert_eq!(store.finding(input.request.id).unwrap(), None);
    conn.execute_batch("DROP TRIGGER refuse_finding_insert;")
        .unwrap();
    let first = store.create_finding(&input).unwrap();
    conn.execute_batch("CREATE TRIGGER refuse_finding_update AFTER UPDATE ON findings BEGIN SELECT RAISE(ABORT,'synthetic closure failure'); END;").unwrap();
    assert!(
        store
            .close_finding(&CloseFindingRequest {
                expected: first.stamp(),
                state: FindingState::Resolved
            })
            .is_err()
    );
    assert_eq!(store.finding(input.request.id).unwrap(), Some(first));
}
#[test]
fn malformed_hash_bound_or_rehashed_structural_rows_refuse_read_list_and_startup() {
    for problem in [
        "hash", "creation", "version", "time", "state", "row_time", "quote", "path", "unknown",
        "nested", "encoded",
    ] {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let first = store.create_finding(&draft()).unwrap();
        let terminal = store
            .close_finding(&CloseFindingRequest {
                expected: first.stamp(),
                state: FindingState::Resolved,
            })
            .unwrap();
        let conn = raw(data.path());
        let mut value = serde_json::to_value(&terminal).unwrap();
        match problem {
            "version" => value["version"] = 3.into(),
            "time" => value["updated_at_ms"] = 0.into(),
            "quote" => value["draft"]["evidence"][0]["quote"]["end_byte"] = 1.into(),
            "path" => value["draft"]["evidence"][0]["source"]["path"] = "../bad.md".into(),
            "unknown" => value["raw_prompt"] = true.into(),
            "nested" => {
                value["draft"]["evidence"][0]["source"]["fingerprint"]["extra"] = true.into()
            }
            _ => {}
        }
        let mut bytes = serde_json::to_vec(&value).unwrap();
        if problem == "encoded" {
            bytes = vec![b' '; brn_store::MAX_NOTE_BYTES * 6 + 128 * 1024 + 1];
        }
        let checksum = if problem == "hash" {
            [0; 32]
        } else {
            digest(&bytes)
        };
        conn.execute(
            "UPDATE findings SET record_json=?2,record_sha256=?3 WHERE id=?1",
            rusqlite::params![
                first.draft.request.id.to_string(),
                bytes,
                checksum.as_slice()
            ],
        )
        .unwrap();
        match problem {
            "creation" => {
                conn.execute("UPDATE findings SET creation_sha256=zeroblob(32)", [])
                    .unwrap();
            }
            "state" => {
                conn.execute("UPDATE findings SET state='dismissed'", [])
                    .unwrap();
            }
            "row_time" => {
                conn.execute("UPDATE findings SET created_at_ms=-1", [])
                    .unwrap();
            }
            _ => {}
        }
        assert!(
            store.finding(first.draft.request.id).is_err(),
            "read {problem}"
        );
        assert!(
            store.findings(&FindingListRequest::default()).is_err(),
            "list {problem}"
        );
        drop(conn);
        drop(store);
        assert!(WorkStore::open(data.path()).is_err(), "startup {problem}");
        assert!(
            !fs_names(data.path())
                .iter()
                .any(|name| name.contains("corrupt")),
            "semantic refusal must not trigger physical corruption restore"
        );
    }
}
fn fs_names(path: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn finding_check_damage_refuses_startup_without_reopening_terminal_work() {
    use rusqlite::OptionalExtension;

    #[derive(Debug, PartialEq, Eq)]
    struct Row {
        id: String,
        state: String,
        created_at_ms: i64,
        creation_sha256: Vec<u8>,
        record_json: Vec<u8>,
        record_sha256: Vec<u8>,
    }
    fn row(conn: &rusqlite::Connection, id: Uuid) -> Option<Row> {
        conn.query_row(
            "SELECT id,state,created_at_ms,creation_sha256,record_json,record_sha256 FROM findings WHERE id=?1",
            [id.to_string()],
            |r| {
                Ok(Row {
                    id: r.get(0)?,
                    state: r.get(1)?,
                    created_at_ms: r.get(2)?,
                    creation_sha256: r.get(3)?,
                    record_json: r.get(4)?,
                    record_sha256: r.get(5)?,
                })
            },
        )
        .optional()
        .unwrap()
    }
    fn backups(data: &std::path::Path) -> Vec<(std::ffi::OsString, Vec<u8>)> {
        let mut entries: Vec<_> = std::fs::read_dir(data.join("backups"))
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (entry.file_name(), std::fs::read(entry.path()).unwrap())
            })
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }

    let mut failures = Vec::new();
    for (case, sql) in [
        (
            "unknown indexed state",
            "UPDATE findings SET state='not-a-state'",
        ),
        (
            "31-byte creation digest",
            "UPDATE findings SET creation_sha256=zeroblob(31)",
        ),
        (
            "31-byte record digest",
            "UPDATE findings SET record_sha256=zeroblob(31)",
        ),
    ] {
        let data = fixture();
        let vault = fixture();
        std::fs::create_dir(vault.path().join("archive")).unwrap();
        let mut input = draft();
        input.vault.root = vault
            .path()
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .into();
        for evidence in &input.evidence {
            std::fs::write(vault.path().join(&evidence.source.path), TEXT.as_bytes()).unwrap();
        }
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let first = store.create_finding(&input).unwrap();
        drop(store);

        // This startup backup predates the legitimate terminal transition.
        let (mut store, report) = WorkStore::open(data.path()).unwrap();
        let backup = rusqlite::Connection::open_with_flags(
            &report.backup,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let backed_up = row(&backup, input.request.id).unwrap();
        assert_eq!(
            serde_json::from_slice::<FindingRecord>(&backed_up.record_json).unwrap(),
            first
        );
        drop(backup);
        let close = CloseFindingRequest {
            expected: first.stamp(),
            state: FindingState::Resolved,
        };
        let terminal = store.close_finding(&close).unwrap();
        assert_eq!(terminal.state, FindingState::Resolved);
        assert_eq!(terminal.version, 2);
        assert_eq!(store.close_finding(&close).unwrap(), terminal);
        drop(store);

        let conn = raw(data.path());
        conn.execute_batch("PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        assert_eq!(conn.execute(sql, []).unwrap(), 1);
        conn.execute_batch("PRAGMA ignore_check_constraints=OFF;")
            .unwrap();
        drop(conn);
        let conn = raw(data.path());
        assert_eq!(
            conn.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "CHECK constraint failed in findings",
            "{case} must reach the physical integrity classifier"
        );
        let damaged = row(&conn, input.request.id).unwrap();
        assert_eq!(
            serde_json::from_slice::<FindingRecord>(&damaged.record_json).unwrap(),
            terminal,
            "the legitimate full terminal receipt survives indexed metadata damage"
        );
        let schema: String = conn
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE type='table' AND name='findings'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        drop(conn);
        let prior_backups = backups(data.path());

        if let Ok((store, report)) = WorkStore::open(data.path()) {
            failures.push(format!(
                "{case}: startup accepted semantic damage (restored={}, moved_corrupt={})",
                report.restored_from.is_some(),
                report.corrupt_moved_to.is_some()
            ));
            drop(store);
        }
        let conn = raw(data.path());
        let after = row(&conn, input.request.id);
        if after.as_ref() != Some(&damaged) {
            failures.push(format!("{case}: the complete damaged row was replaced"));
        }
        if after.as_ref().map(|r| &r.record_json) != Some(&damaged.record_json) {
            failures.push(format!(
                "{case}: the exact terminal closure receipt was replaced"
            ));
        }
        let after_schema: String = conn
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE type='table' AND name='findings'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            after_schema, schema,
            "{case}: retain the compatible V9 schema"
        );
        drop(conn);
        if backups(data.path()) != prior_backups {
            failures.push(format!("{case}: backup names or exact file bytes changed"));
        }
        if fs_names(data.path())
            .iter()
            .any(|name| name.contains("corrupt"))
        {
            failures.push(format!("{case}: semantic damage moved the database aside"));
        }
        for evidence in &input.evidence {
            assert_eq!(
                std::fs::read(vault.path().join(&evidence.source.path)).unwrap(),
                TEXT.as_bytes(),
                "{case}: saved vault evidence must remain untouched"
            );
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn finding_check_damage_backup_is_skipped_during_physical_recovery() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let input = draft();
    let first = store.create_finding(&input).unwrap();
    let terminal = store
        .close_finding(&CloseFindingRequest {
            expected: first.stamp(),
            state: FindingState::Resolved,
        })
        .unwrap();
    drop(store);
    let (store, report) = WorkStore::open(data.path()).unwrap();
    let healthy_backup = report.backup;
    drop(store);
    let healthy_bytes = std::fs::read(&healthy_backup).unwrap();
    let damaged_backup = data.path().join("backups/brn-9999999999999.sqlite");
    let conn = raw(data.path());
    conn.backup("main", &damaged_backup, None).unwrap();
    drop(conn);
    let conn = rusqlite::Connection::open(&damaged_backup).unwrap();
    conn.execute_batch(
        "PRAGMA ignore_check_constraints=ON; UPDATE findings SET state='not-a-state'; PRAGMA ignore_check_constraints=OFF;",
    )
    .unwrap();
    drop(conn);
    let conn = rusqlite::Connection::open(&damaged_backup).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "CHECK constraint failed in findings"
    );
    drop(conn);
    let damaged_bytes = std::fs::read(&damaged_backup).unwrap();
    let physical_damage = b"synthetic physical corruption, not semantic finding damage";
    std::fs::write(data.path().join("brn.sqlite"), physical_damage).unwrap();

    let (store, report) = WorkStore::open(data.path()).unwrap();
    assert_eq!(report.restored_from, Some(healthy_backup.clone()));
    assert_eq!(store.finding(input.request.id).unwrap(), Some(terminal));
    assert_eq!(std::fs::read(&damaged_backup).unwrap(), damaged_bytes);
    assert_eq!(std::fs::read(&healthy_backup).unwrap(), healthy_bytes);
    assert_eq!(
        std::fs::read(
            report
                .corrupt_moved_to
                .expect("retain physical main database")
        )
        .unwrap(),
        physical_damage
    );
}

#[test]
fn immutable_creation_digest_is_checked_after_closure_even_when_record_hash_is_recomputed() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let first = store.create_finding(&draft()).unwrap();
    let mut closed = store
        .close_finding(&CloseFindingRequest {
            expected: first.stamp(),
            state: FindingState::Dismissed,
        })
        .unwrap();
    closed.draft.summary.push_str("mutated retained creation");
    rewrite(&raw(data.path()), &closed);
    assert!(store.finding(first.draft.request.id).is_err());
}
#[test]
fn v8_upgrade_and_backup_restore_preserve_findings_and_existing_operational_work() {
    use brn_store::work::{
        EditRequest, WorkTurnStatus,
        proposals::{NoteChange, ProposalDraft},
    };
    for restored_v8 in [false, true] {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let turn = store
            .begin_turn(
                Uuid::new_v4(),
                None,
                "Question λ\r\n",
                "chatgpt",
                "synthetic",
            )
            .unwrap();
        let turn = store
            .finish_turn(
                turn.id,
                WorkTurnStatus::Completed,
                "Answer 日本語\r\n",
                None,
            )
            .unwrap();
        let fp = FileFingerprint {
            device: 1,
            inode: 2,
            len: TEXT.len() as u64,
            sha256: digest(TEXT.as_bytes()),
        };
        let editor = store.open_editor("current.md", &fp, TEXT).unwrap();
        let editor = store
            .recover_editor(&EditRequest {
                path: editor.path.clone(),
                expected: editor.stamp,
                generation: editor.stamp.generation + 1,
                text: "\u{feff}Unfinished 日本語\r\n".into(),
            })
            .unwrap();
        let input = draft();
        let proposal = store
            .create_proposal(&ProposalDraft {
                inbox_visual: None,
                inbox_knowledge: None,
                inbox_source: None,
                id: Uuid::new_v4(),
                group_id: None,
                session_id: Some(turn.conversation_id),
                vault: Some(input.vault.clone()),
                title: "Protected proposal".into(),
                changes: vec![NoteChange::Create {
                    path: "new.md".into(),
                    parent: input.vault.identity.clone(),
                    text: TEXT.into(),
                }],
                sources: vec![],
                action_changes: Vec::new(),
            })
            .unwrap();
        store.set_setting("synthetic", "retained").unwrap();
        drop(store);
        let conn = raw(data.path());
        conn.execute_batch("DROP TABLE inbox_original_operations; DROP TABLE inbox_actions; DROP TABLE inbox_processing; DROP TABLE inbox_items; DROP TABLE action_completions; DROP TABLE actions; DROP TABLE findings; PRAGMA user_version=8;")
            .unwrap();
        let old_backup = data.path().join("backups/brn-9999999999999.sqlite");
        if restored_v8 {
            conn.backup("main", &old_backup, None).unwrap();
        }
        drop(conn);
        if restored_v8 {
            std::fs::write(data.path().join("brn.sqlite"), b"synthetic V8 corruption").unwrap();
        }
        let (mut store, report) = WorkStore::open(data.path()).unwrap();
        assert_eq!(report.restored_from, restored_v8.then_some(old_backup));
        assert_eq!(
            raw(data.path())
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            15
        );
        assert!(
            store
                .findings(&Default::default())
                .unwrap()
                .entries
                .is_empty()
        );
        assert_eq!(
            serde_json::to_value(store.turn(turn.id).unwrap()).unwrap(),
            serde_json::to_value(Some(turn.clone())).unwrap()
        );
        assert_eq!(store.editor(&editor.path).unwrap(), Some(editor.clone()));
        assert_eq!(
            store.proposal(proposal.draft.id).unwrap(),
            Some(proposal.clone())
        );
        let record = store.create_finding(&input).unwrap();
        let record = store
            .close_finding(&CloseFindingRequest {
                expected: record.stamp(),
                state: FindingState::Resolved,
            })
            .unwrap();
        drop(store);
        let (store, report) = WorkStore::open(data.path()).unwrap();
        assert_eq!(
            store.finding(input.request.id).unwrap(),
            Some(record.clone())
        );
        let backup = report.backup;
        drop(store);
        std::fs::write(data.path().join("brn.sqlite"), b"synthetic corruption").unwrap();
        let (store, report) = WorkStore::open(data.path()).unwrap();
        assert_eq!(report.restored_from, Some(backup));
        assert!(report.corrupt_moved_to.is_some());
        assert_eq!(store.finding(input.request.id).unwrap(), Some(record));
        assert_eq!(
            serde_json::to_value(store.turn(turn.id).unwrap()).unwrap(),
            serde_json::to_value(Some(turn)).unwrap()
        );
        assert_eq!(store.editor(&editor.path).unwrap(), Some(editor));
        assert_eq!(store.proposal(proposal.draft.id).unwrap(), Some(proposal));
        assert_eq!(
            store.setting("synthetic").unwrap().as_deref(),
            Some("retained")
        );
    }
}

#[test]
fn indexed_row_identity_and_unknown_or_invalid_operations_never_expose_guessed_work() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let first = store.create_finding(&draft()).unwrap();
    assert_eq!(store.finding(Uuid::new_v4()).unwrap(), None);
    assert!(matches!(store.finding(Uuid::nil()), Err(Error::Invalid(_))));
    assert!(matches!(
        store.close_finding(&CloseFindingRequest {
            expected: FindingStamp {
                id: Uuid::new_v4(),
                version: 1
            },
            state: FindingState::Resolved
        }),
        Err(Error::NotFound(_))
    ));
    assert!(
        CloseFindingRequest {
            expected: FindingStamp {
                id: first.draft.request.id,
                version: 0
            },
            state: FindingState::Resolved
        }
        .validate()
        .is_err()
    );
    let moved = Uuid::new_v4();
    let conn = raw(data.path());
    conn.execute(
        "UPDATE findings SET id=?1 WHERE id=?2",
        rusqlite::params![moved.to_string(), first.draft.request.id.to_string()],
    )
    .unwrap();
    assert!(store.finding(moved).is_err());
    assert!(store.findings(&Default::default()).is_err());
    drop(conn);
    drop(store);
    assert!(WorkStore::open(data.path()).is_err());
}
#[test]
fn capture_intent_validation_is_bounded_and_keeps_archived_unmanaged_link_proof() {
    let mut input = link_draft();
    input.validate().unwrap();
    for change in 0..5 {
        let mut bad = input.request.clone();
        if let FindingOrigin::UnresolvedLink {
            path,
            destination,
            start_byte,
            ..
        } = &mut bad.origin
        {
            match change {
                0 => *path = "../source.md".into(),
                1 => *path = "source\n.md".into(),
                2 => destination.clear(),
                3 => *start_byte = brn_store::MAX_NOTE_BYTES,
                _ => *destination = "x".repeat(brn_store::MAX_NOTE_BYTES + 1),
            }
        }
        assert!(bad.validate().is_err());
    }
    input.request.origin = FindingOrigin::IdentityAmbiguity {
        note_id: Uuid::nil(),
    };
    assert!(input.request.validate().is_err());
}

#[test]
fn repeated_source_range_is_refused_even_when_quote_bytes_claim_a_different_value() {
    let mut input = link_draft();
    let mut repeated = input.evidence[0].clone();
    repeated.quote.as_mut().unwrap().quote =
        repeated.quote.as_ref().unwrap().quote.replace('λ', "μ");
    input.evidence.push(repeated);
    assert!(
        input.validate().is_err(),
        "a saved byte range cannot carry two contradictory proofs"
    );
}
