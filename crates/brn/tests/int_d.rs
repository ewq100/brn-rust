#![cfg(any(target_os = "macos", target_os = "linux"))]
//! Checked shared-corpus construction + real CLI roundtrip. With an explicit
//! fresh task-owned output, this SAME tested builder prepares qualification
//! fixtures; it never starts a provider or opens an owner credential source.
#[cfg(target_os = "macos")]
#[path = "support/int_d.rs"]
mod candidate_export;
mod support;
use brn_threads_app::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use support::{ok, Root};
const CORPUS: &str = include_str!("../../../docs/work/active/agent-capabilities/int-d/corpus.json");
fn project_row(receipt: Receipt, kind: &str) -> Record {
    receipt
        .writes
        .into_iter()
        .find(|w| {
            matches!(
                (&w.after.data, kind),
                (RecordData::Project(_), "project")
                    | (RecordData::Thread(_), "chat")
                    | (RecordData::Note(_), "note")
            )
        })
        .unwrap()
        .after
}
fn history(app: &mut Workspace, chat: &str, c: &Value) -> Vec<String> {
    let mut ids = vec![];
    for batch in (0..c["history_messages"].as_u64().unwrap())
        .collect::<Vec<_>>()
        .chunks(32)
    {
        let op = app
            .store
            .allocate_operation(&format!("int-d-history-{}", ids.len()))
            .unwrap();
        let writes = batch
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let id = op.creation_id(i);
                ids.push(id.clone());
                let text = match n {
                    0 => c["early_requirement"].as_str().unwrap().to_owned(),
                    40 => c["later_proposal"].as_str().unwrap().to_owned(),
                    _ => format!(
                        "Discussion {n}: {}",
                        c["history_filler"].as_str().unwrap().repeat(4)
                    ),
                };
                Put {
                    id,
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: chat.into(),
                        role: if *n != 40 && n % 2 == 0 {
                            "owner"
                        } else {
                            "assistant"
                        }
                        .into(),
                        text,
                    }),
                }
            })
            .collect();
        let owner = HostAuthority::owner("Synthetic fixed INT-D corpus");
        app.store
            .prepare_owner(
                &op,
                &ChangeRequest {
                    reason: "Source-labelled canonical discussion".into(),
                    writes,
                    inputs: vec![],
                },
                &owner,
            )
            .unwrap();
        assert!(matches!(
            app.store.apply(&op, &owner).unwrap(),
            ApplyOutcome::Applied(_)
        ));
    }
    ids
}
fn fresh_output(root: &Root) -> PathBuf {
    match std::env::var_os("BRN_INT_D_FIXTURE_ROOT") {
        None => root.join("paired-corpus"),
        Some(path) => {
            let path = PathBuf::from(path);
            assert!(path.is_absolute());
            assert_eq!(path.parent(), Some(Path::new("/private/tmp")));
            assert!(path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("brn-int-d-qualification-20261017-"));
            assert!(
                !path.exists(),
                "qualification fixture must be fresh; never reuse/overwrite"
            );
            path
        }
    }
}
#[test]
fn checked_source_labelled_pair_reopens_through_cli_with_explicit_scoped_selection() {
    let root = Root::new();
    let base = fresh_output(&root);
    std::fs::create_dir(&base).unwrap();
    let c: Value = serde_json::from_str(CORPUS).unwrap();
    let source = base.join("package");
    std::fs::create_dir_all(source.join("references")).unwrap();
    std::fs::write(
        source.join("SKILL.md"),
        format!(
            "---\nname: {}\ndescription: Review synthetic delivery evidence.\n---\n{}\n",
            c["selected_skill_name"].as_str().unwrap(),
            c["skill_body"].as_str().unwrap()
        ),
    )
    .unwrap();
    std::fs::write(
        source.join("references/check.md"),
        c["reference_body"].as_str().unwrap(),
    )
    .unwrap();
    let mut cases = vec![];
    for (name, selected) in [("control", false), ("candidate", true)] {
        let data = base.join(name);
        let mut app =
            Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
        app.choose_delegation(DelegationChoice::ReviewFirst)
            .unwrap();
        let mut settings = app.settings().unwrap();
        settings.provider = "Codex".into();
        settings.model = "gpt-6.1-sol".into();
        settings.effort = "medium".into();
        assert!(settings.auth_file.is_none() && settings.credentials_dir.is_none());
        app.configure(settings).unwrap();
        let project = project_row(
            app.project_command(
                "int-d-project",
                ProjectCommand::Create {
                    title: "Synthetic Aurora delivery".into(),
                },
            )
            .unwrap(),
            "project",
        );
        let chat = project_row(
            app.project_command(
                "int-d-chat",
                ProjectCommand::CreateChat {
                    project: project.id.clone(),
                    title: "Source-labelled Aurora discussion".into(),
                },
            )
            .unwrap(),
            "chat",
        );
        let note = project_row(
            app.project_command(
                "int-d-note",
                ProjectCommand::CreateDocument {
                    project: project.id.clone(),
                    title: "Aurora delivery draft".into(),
                    markdown: c["original_note"].as_str().unwrap().into(),
                },
            )
            .unwrap(),
            "note",
        );
        let RecordData::Note(mut protected) = note.data.clone() else {
            panic!()
        };
        protected.protected = true;
        app.owner_change(
            "int-d-protect",
            &ChangeRequest {
                reason: "Owner review required for this exact synthetic Note".into(),
                writes: vec![Put {
                    id: note.id.clone(),
                    expected_version: Some(note.version),
                    archived: false,
                    data: RecordData::Note(protected),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
        let messages = history(&mut app, &chat.id, &c);
        let current = app.store.record(&note.id).unwrap().unwrap();
        drop(app);
        let package = ok(
            &root,
            &data,
            &["skill-install", "--source", source.to_str().unwrap()],
        );
        let id = package["id"].as_str().unwrap();
        let digest = package["digest"].as_str().unwrap();
        if selected {
            ok(
                &root,
                &data,
                &[
                    "skill-enable",
                    "--id",
                    id,
                    "--digest",
                    digest,
                    "--project",
                    &project.id,
                ],
            );
            ok(
                &root,
                &data,
                &["skill-select", "--id", id, "--project", &project.id],
            );
        }
        let list = ok(
            &root,
            &data,
            &["messages", "--thread", &chat.id, "--limit", "100"],
        );
        let rows = list["messages"].as_array().unwrap();
        assert_eq!(rows.len(), 68);
        assert_eq!(rows[0]["id"], messages[0]);
        assert_eq!(rows[0]["version"], 1);
        assert_eq!(rows[0]["data"]["Message"]["text"], c["early_requirement"]);
        assert_eq!(rows[40]["data"]["Message"]["role"], "assistant");
        let inventory = ok(&root, &data, &["skills", "--project", &project.id]);
        assert!(inventory["skills"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == id));
        let policy = inventory["policy"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == id);
        if selected {
            let policy = policy.expect("explicit scoped enable/select must be visible");
            assert_eq!(policy["enabled"], true);
            assert_eq!(policy["selected"], true);
        } else {
            // Owner installation inventory is distinct from canonical scope
            // policy. Absence is disabled; no binding may be auto-created.
            assert!(policy.is_none_or(|p| p["enabled"] == false && p["selected"] == false));
        }
        let app = Workspace::open_existing_no_upgrade(&data).unwrap();
        assert_eq!(app.store.record(&note.id).unwrap().unwrap(), current);
        assert!(app.settings().unwrap().auth_file.is_none());
        if !selected {
            assert!(!app.settings().unwrap().skills.iter().any(|b| b.id == id));
        }
        assert!(app
            .store
            .records()
            .unwrap()
            .iter()
            .all(|r| !matches!(r.data, RecordData::Run(_))));
        cases.push(json!({"case":name,"data_dir":data,"project":project.id,"chat":chat.id,"note":current.id,"note_version":current.version,"early_message":messages[0],"early_version":1,"proposal_message":messages[40],"history_messages":messages,"selected":selected,"skill_id":id,"skill_digest":digest}));
    }
    assert_eq!(cases[0]["skill_digest"], cases[1]["skill_digest"]);
    std::fs::write(base.join("manifest.json"),serde_json::to_vec_pretty(&json!({"corpus_version":c["version"],"mode":"checked fixture only; zero provider starts; no auth source","package_source":source,"cases":cases})).unwrap()).unwrap();
}

/// Offline copy-reader interop only. The external branch never opens the
/// qualification originals; all paths must designate explicit closed copies.
#[cfg(target_os = "macos")]
#[test]
fn closed_copy_metadata_interoperates_for_both_history_arms() {
    if let Some(copy) = std::env::var_os("BRN_INT_D_METADATA_COPY_ROOT") {
        let expected = std::env::var_os("BRN_INT_D_METADATA_EXPECTED")
            .expect("explicit bounded expected JSON");
        let output = std::env::var_os("BRN_INT_D_METADATA_OUTPUT").expect("explicit absent output");
        candidate_export::metadata_copy_context(
            Path::new(&copy),
            Path::new(&expected),
            Path::new(&output),
        )
        .unwrap();
        return;
    }
    // Both real policies are checked without generating a qualification pair,
    // invoking a workflow or opening credentials. Python builds the independent
    // context from the normal CLI canonical-records serialization.
    let root = Root::new();
    let c: Value = serde_json::from_str(CORPUS).unwrap();
    for arm in ["control", "candidate"] {
        let copied = tempfile::Builder::new()
            .prefix("brn-int-d-metadata-copy-20261017-default-")
            .tempdir_in("/private/tmp")
            .unwrap();
        let data = copied.path().join("data");
        let mut app =
            Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
        app.choose_delegation(DelegationChoice::ReviewFirst)
            .unwrap();
        let mut s = app.settings().unwrap();
        s.provider = "Codex".into();
        s.model = "gpt-6.1-sol".into();
        s.effort = "medium".into();
        s.auth_file = Some("/Users/evokessler/.codex/auth.json".into());
        app.configure(s).unwrap();
        let p = project_row(
            app.project_command(
                "metadata-project",
                ProjectCommand::Create {
                    title: "Synthetic copy metadata".into(),
                },
            )
            .unwrap(),
            "project",
        );
        let t = project_row(
            app.project_command(
                "metadata-chat",
                ProjectCommand::CreateChat {
                    project: p.id.clone(),
                    title: "Synthetic history".into(),
                },
            )
            .unwrap(),
            "chat",
        );
        let n = project_row(
            app.project_command(
                "metadata-note",
                ProjectCommand::CreateDocument {
                    project: p.id.clone(),
                    title: "Synthetic protected target".into(),
                    markdown: c["original_note"].as_str().unwrap().into(),
                },
            )
            .unwrap(),
            "note",
        );
        let RecordData::Note(mut note) = n.data.clone() else {
            panic!()
        };
        note.protected = true;
        app.owner_change(
            "metadata-protect",
            &ChangeRequest {
                reason: "Synthetic owner protection".into(),
                writes: vec![Put {
                    id: n.id.clone(),
                    expected_version: Some(1),
                    archived: false,
                    data: RecordData::Note(note),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
        let ids = history(&mut app, &t.id, &c);
        let package = root.join(&format!("package-{arm}"));
        std::fs::create_dir(&package).unwrap();
        std::fs::write(package.join("SKILL.md"),format!("---\nname: int-d-review\ndescription: Review synthetic delivery evidence.\n---\n{}\n",c["skill_body"].as_str().unwrap())).unwrap();
        let info = app.install_skill(&package).unwrap();
        if arm == "candidate" {
            app.set_skill_enabled(&info.id, &info.digest, Some(&p.id), true)
                .unwrap();
            app.select_skill(Some(&info.id), Some(&p.id)).unwrap();
        }
        let facts = json!({"case":arm,"data_dir":format!("/private/tmp/brn-int-d-qualification-20261017-default/{arm}"),"project":p.id,"chat":t.id,"note":n.id,"note_version":2,"early_message":ids[0],"early_version":1,"proposal_message":ids[40],"history_messages":ids,"selected":arm=="candidate","skill_id":info.id,"skill_digest":info.digest});
        drop(app);
        let records = ok(&root, &data, &["records"]);
        let input = root.join(&format!("metadata-{arm}.json"));
        std::fs::write(&input,serde_json::to_vec(&json!({"envelope":{"status":"ok","authority":{"workspace_purpose":"test"},"result":records},"facts":facts})).unwrap()).unwrap();
        let expected = tempfile::Builder::new()
            .prefix("brn-int-d-metadata-expected-20261017-default-")
            .tempfile_in("/private/tmp")
            .unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/work/active/agent-capabilities/int-d/prepare-qualification.py");
        let result=std::process::Command::new("python3").args(["-c", "import importlib.util,json,sys; s=importlib.util.spec_from_file_location('preparation',sys.argv[1]);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);v=json.load(open(sys.argv[2]));c=json.load(open(sys.argv[3]));json.dump({'facts':v['facts'],'context':m.history_context(v['envelope'],v['facts'],c)},open(sys.argv[4],'w'),ensure_ascii=False)"]).arg(source).arg(&input).arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/work/active/agent-capabilities/int-d/corpus.json")).arg(expected.path()).status().unwrap();
        assert!(result.success());
        let output = tempfile::Builder::new()
            .prefix("brn-int-d-metadata-result-20261017-default-")
            .tempdir_in("/private/tmp")
            .unwrap();
        let output = output.path().to_owned();
        std::fs::remove_dir(&output).unwrap();
        let value =
            candidate_export::metadata_copy_context(copied.path(), expected.path(), &output)
                .unwrap();
        assert_eq!(value["context"]["package"]["selected"], arm == "candidate");
        let existing = std::fs::read(output.join("context.json")).unwrap();
        assert!(
            candidate_export::metadata_copy_context(copied.path(), expected.path(), &output)
                .is_err()
        );
        assert_eq!(
            std::fs::read(output.join("context.json")).unwrap(),
            existing
        );
        let mut bad: Value =
            serde_json::from_slice(&std::fs::read(expected.path()).unwrap()).unwrap();
        bad["context"]["settings"]["policy_sha256"] = json!("wrong");
        std::fs::write(expected.path(), serde_json::to_vec(&bad).unwrap()).unwrap();
        let refused = output.with_file_name(format!(
            "{}-refused",
            output.file_name().unwrap().to_str().unwrap()
        ));
        assert!(
            candidate_export::metadata_copy_context(copied.path(), expected.path(), &refused)
                .is_err()
        );
        assert!(!refused.exists());
        bad["facts"]["case"] = json!("other");
        std::fs::write(expected.path(), serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(
            candidate_export::metadata_copy_context(copied.path(), expected.path(), &refused)
                .is_err()
        );
        assert!(candidate_export::metadata_copy_context(
            Path::new("/private/tmp/brn-int-d-qualification-20261017-default"),
            expected.path(),
            &refused
        )
        .is_err());
        let link = copied.path().with_file_name(format!(
            "{}-link",
            copied.path().file_name().unwrap().to_str().unwrap()
        ));
        std::os::unix::fs::symlink(copied.path(), &link).unwrap();
        assert_eq!(
            candidate_export::metadata_copy_context(&link, expected.path(), &refused).unwrap_err(),
            "real existing fixture directory required"
        );
        std::fs::remove_file(link).unwrap();
        let unsafe_expected = output.join("context.json");
        assert_eq!(
            candidate_export::metadata_copy_context(copied.path(), &unsafe_expected, &refused)
                .unwrap_err(),
            "bounded explicit real expected-context file required"
        );
        std::fs::write(expected.path(), vec![b' '; 65537]).unwrap();
        assert_eq!(
            candidate_export::metadata_copy_context(copied.path(), expected.path(), &refused)
                .unwrap_err(),
            "bounded explicit real expected-context file required"
        );
        assert!(!refused.exists());
    }
}

/// Real normal-CLI/Rig prepare, not owner prepare: the runtime adds the exact
/// Thread revision ChatContext and current Project inputs before persistence.
#[cfg(target_os = "macos")]
#[test]
fn runtime_project_candidate_exports_host_context_inputs_without_apply() {
    use std::{ffi::OsStr, process::Stdio, time::Duration};
    use support::stub::{answer, proxy_env, tool, Stub};
    let mut root = Root::new();
    root.env = proxy_env();
    let fixture = tempfile::Builder::new()
        .prefix("brn-int-d-qualification-20261017-runtime-export-")
        .tempdir_in("/private/tmp")
        .unwrap();
    let data = fixture.path().join("candidate");
    let mut app =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    app.choose_delegation(DelegationChoice::ReviewFirst)
        .unwrap();
    let project = project_row(
        app.project_command(
            "export-project",
            ProjectCommand::Create {
                title: "Synthetic runtime export".into(),
            },
        )
        .unwrap(),
        "project",
    );
    let chat = project_row(
        app.project_command(
            "export-chat",
            ProjectCommand::CreateChat {
                project: project.id.clone(),
                title: "Synthetic discussion".into(),
            },
        )
        .unwrap(),
        "chat",
    );
    let note = project_row(
        app.project_command(
            "export-note",
            ProjectCommand::CreateDocument {
                project: project.id.clone(),
                title: "Synthetic protected draft".into(),
                markdown: "Monday remains pending exact review.\n".into(),
            },
        )
        .unwrap(),
        "note",
    );
    let RecordData::Note(mut content) = note.data.clone() else {
        panic!()
    };
    content.protected = true;
    app.owner_change(
        "export-protection",
        &ChangeRequest {
            reason: "Synthetic exact review".into(),
            writes: vec![Put {
                id: note.id.clone(),
                expected_version: Some(1),
                archived: false,
                data: RecordData::Note(content.clone()),
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    let receipt = app
        .post_message(&chat.id, "Delivery is Monday only / ainult esmaspäeval.")
        .unwrap();
    let message = receipt
        .writes
        .iter()
        .find(|w| matches!(w.after.data, RecordData::Message(_)))
        .unwrap()
        .after
        .clone();
    // Deliberately separate the revision number from epoch before Run start.
    let seed_chat = app.store.record(&chat.id).unwrap().unwrap();
    let RecordData::Thread(mut seed_title) = seed_chat.data.clone() else {
        panic!()
    };
    seed_title.title = "Seeded revision distinct from epoch".into();
    app.owner_change(
        "export-seed-chat-version",
        &ChangeRequest {
            reason: "Revision is not context epoch".into(),
            writes: vec![Put {
                id: chat.id.clone(),
                expected_version: Some(seed_chat.version),
                archived: false,
                data: RecordData::Thread(seed_title),
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    let original = app.store.record(&note.id).unwrap().unwrap();
    content.markdown =
        "Delivery: Monday only / ainult esmaspäeval. Prepared, not applied.\n".into();
    let request = json!({"reason":"Synthetic exact source revision","writes":[{"id":note.id,"expected_version":2,"archived":false,"data":{"Note":content}}],"inputs":[{"record":note.id,"version":2},{"record":message.id,"version":message.version}]});
    drop(app);
    let stub = Stub::start(vec![
        tool(
            "read-note",
            "read_note",
            json!({"id":note.id,"version":2,"start":0,"max_bytes":4096,"section":false}),
        ),
        tool(
            "read-message",
            "read_message",
            json!({"id":message.id,"version":1,"start":0,"max_bytes":4096}),
        ),
        tool(
            "prepare",
            "prepare_changes",
            json!({"request_key":"runtime-export","request":request}),
        ),
        answer("Prepared for exact owner review; nothing applied."),
    ]);
    let credentials = stub.credentials(&root);
    ok(
        &root,
        &data,
        &[
            "configure",
            "--provider",
            "Copilot",
            "--model",
            "synthetic",
            "--effort",
            "low",
            "--credentials-dir",
            credentials.to_str().unwrap(),
        ],
    );
    let asked = support::call(
        &root,
        &data,
        &[
            "ask",
            "--thread",
            &chat.id,
            "--prompt",
            "Prepare the exact protected Monday draft for review.",
            "--budget",
            "8",
        ],
    );
    assert_eq!(asked["outcome"], "Completed", "{asked}");
    assert_eq!(stub.requests().len(), 4);
    let mut app = Workspace::open_existing_no_upgrade(&data).unwrap();
    let candidates = app.candidates().unwrap();
    assert_eq!(candidates.len(), 1);
    let prepared = candidates[0].clone();
    let context = prepared
        .request
        .inputs
        .iter()
        .find(|i| i.record == chat.id)
        .unwrap();
    assert_eq!(context.expectation, InputExpectation::ChatContext);
    assert!(matches!(
        app.store
            .revision(&context.record, context.version)
            .unwrap()
            .unwrap()
            .data,
        RecordData::Thread(_)
    ));
    assert!(prepared
        .request
        .inputs
        .iter()
        .any(|i| i.record == project.id
            && i.expectation == InputExpectation::Current
            && i.version == app.store.record(&project.id).unwrap().unwrap().version));
    assert!(prepared
        .request
        .inputs
        .iter()
        .any(|i| i.record == message.id
            && i.expectation == InputExpectation::Current
            && i.version == 1));
    assert_eq!(
        prepared.request.writes[0].data,
        RecordData::Note(content.clone())
    );
    assert!(app.records().unwrap().iter().any(
        |r| matches!(&r.data,RecordData::Run(run) if run.operations.contains(&prepared.operation))
    ));
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), original);
    assert!(app
        .store
        .operation_info(prepared.operation.as_str())
        .unwrap()
        .receipt
        .is_none());
    // A real checked same-context title update advances Thread.version after
    // prepare; the original ChatContext revision must remain exportable.
    let same_context = app.store.record(&chat.id).unwrap().unwrap();
    let RecordData::Thread(mut renamed) = same_context.data.clone() else {
        panic!()
    };
    renamed.title = "Same synthetic context, revised title".into();
    app.owner_change(
        "export-same-context-title",
        &ChangeRequest {
            reason: "Exercise revision versus context epoch semantics".into(),
            writes: vec![Put {
                id: chat.id.clone(),
                expected_version: Some(same_context.version),
                archived: false,
                data: RecordData::Thread(renamed),
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    let current_chat = app.store.record(&chat.id).unwrap().unwrap();
    let old_chat = app
        .store
        .revision(&chat.id, context.version)
        .unwrap()
        .unwrap();
    let (RecordData::Thread(current), RecordData::Thread(old)) =
        (&current_chat.data, &old_chat.data)
    else {
        panic!()
    };
    assert_eq!(current.context_epoch, old.context_epoch);
    assert_ne!(context.version, old.context_epoch);
    assert!(current_chat.version > context.version);
    drop(app);
    let manifest = json!({"corpus_version":"BRN-INT-D-20261017-1","cases":[{"case":"candidate","data_dir":data,"note":note.id,"note_version":2,"chat":chat.id,"project":project.id,"history_messages":[message.id]}]});
    std::fs::write(
        fixture.path().join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let output = tempfile::Builder::new()
        .prefix("brn-int-d-candidate-export-20261017-runtime-")
        .tempdir_in("/private/tmp")
        .unwrap();
    let output_path = output.path().to_owned();
    output.close().unwrap();
    let mut command=root.command_with(&std::env::current_exe().unwrap(),&[OsStr::new("--exact"),OsStr::new("candidate_export::exact_synthetic_candidate_export_refuses_wrong_fixture_operation_and_preserves_content"),OsStr::new("--test-threads=1"),OsStr::new("--nocapture")]);
    command.env("BRN_INT_D_INSPECT_ROOT",fixture.path()).env("BRN_INT_D_INSPECT_ARM","candidate").env("BRN_INT_D_INSPECT_OPERATION",prepared.operation.as_str()).env("BRN_INT_D_INSPECT_OUTPUT",&output_path).env("BRN_INT_D_INSPECT_IDENTITY",json!({"source_pin":"0000000000000000000000000000000000000000","purpose":"offline real runtime witness; source association in command receipt"}).to_string()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let exported = support::wait_output(command.spawn().unwrap(), Duration::from_secs(15));
    assert_eq!(
        exported.code, 0,
        "real runtime export must accept host-added context: {} {}",
        exported.stdout, exported.stderr
    );
    let saved: Value =
        serde_json::from_slice(&std::fs::read(output_path.join("candidate.json")).unwrap())
            .unwrap();
    assert_eq!(saved["request_hash"], prepared.request_hash);
    assert_eq!(saved["prepared"], serde_json::to_value(&prepared).unwrap());
    assert_eq!(
        saved["prepared"]["request"]["writes"][0]["data"]["Note"]["markdown"],
        content.markdown
    );
    assert_eq!(
        saved["exact_inputs"].as_array().unwrap().len(),
        prepared.request.inputs.len()
    );
    assert_eq!(saved["automatic_apply"], false);
    let app = Workspace::open_existing_no_upgrade(&data).unwrap();
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), original);
    assert!(app
        .store
        .operation_info(prepared.operation.as_str())
        .unwrap()
        .receipt
        .is_none());
}
