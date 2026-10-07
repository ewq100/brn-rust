use super::*;
use crate::cli::{Command, Invocation, Outcome};
use std::fs;

fn arguments(parts: &[&str], data: &std::path::Path) -> Vec<String> {
    parts
        .iter()
        .copied()
        .chain(["--data-dir", data.to_str().unwrap()])
        .map(str::to_owned)
        .collect()
}

#[test]
fn visual_parser_requires_explicit_bounded_commands() {
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let id = Uuid::new_v4().to_string();
    for (tokens, name) in [
        (
            vec!["inbox", "visual", "sources/document.md"],
            "inbox.visual",
        ),
        (
            vec!["inbox", "visual-annotation", &id],
            "inbox.visual-annotation",
        ),
        (
            vec![
                "inbox",
                "interpret-visual",
                "--file",
                "request.json",
                "--timeout-seconds",
                "17",
            ],
            "inbox.interpret-visual",
        ),
    ] {
        let Outcome::Run(invocation) = crate::cli::parse(&arguments(&tokens, owner.path()))
            .unwrap_or_else(|_| panic!("visual command"))
        else {
            panic!("run")
        };
        let Command::Inbox(command) = invocation.command else {
            panic!("Inbox")
        };
        assert_eq!(command.name(), name);
        if let InboxCommand::InterpretVisual {
            input,
            timeout_seconds,
        } = command
        {
            assert_eq!(input, PathBuf::from("request.json"));
            assert_eq!(timeout_seconds, 17);
        }
    }
    for tokens in [
        vec!["inbox", "visual"],
        vec!["inbox", "visual", "../source.md"],
        vec!["inbox", "visual", "image.png"],
        vec!["inbox", "visual", "source.md", "--approve"],
        vec![
            "inbox",
            "visual-annotation",
            "00000000-0000-0000-0000-000000000000",
        ],
        vec!["inbox", "visual-annotation", &id, "extra"],
        vec!["inbox", "interpret-visual"],
        vec![
            "inbox",
            "interpret-visual",
            "--file",
            "request.json",
            "--timeout-seconds",
            "0",
        ],
        vec![
            "inbox",
            "interpret-visual",
            "--file",
            "request.json",
            "--timeout-seconds",
            "3601",
        ],
        vec![
            "inbox",
            "interpret-visual",
            "--file",
            "request.json",
            "--model",
            "automatic",
        ],
    ] {
        assert!(crate::cli::parse(&arguments(&tokens, owner.path())).is_err());
    }
    assert_eq!(fs::read_dir(owner.path()).unwrap().count(), 0);
}

#[test]
fn visual_preflight_refuses_before_startup_without_rewriting_request() {
    let _cancel = crate::tests::CancelTestGuard::with(false);
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    fs::create_dir(&data).unwrap();
    let credentials = owner.path().join("credentials");
    let input = owner.path().join("request.json");
    for purpose in [
        None,
        Some("actions"),
        Some("knowledge_and_actions"),
        Some("visual_interpretation"),
    ] {
        let mut request = serde_json::json!({
            "id": Uuid::new_v4(), "conversation": null,
            "source": {"source": {"path": "source.md", "fingerprint": {"device":1,"inode":2,"len":0,"sha256":vec![0u8;32]}}, "text":""},
            "selection": {"provider":"chatgpt","model":"gpt-6-luna"},
            "effort":"medium", "generation":11
        });
        if let Some(purpose) = purpose {
            request["purpose"] = purpose.into();
        }
        let exact = serde_json::to_vec(&request).unwrap();
        fs::write(&input, &exact).unwrap();
        let invocation = Invocation {
            json: true,
            data_dir: data.clone(),
            vault: None,
            credentials_dir: Some(credentials.clone()),
            model_dir: None,
            command: Command::Inbox(InboxCommand::InterpretVisual {
                input: input.clone(),
                timeout_seconds: 17,
            }),
        };
        let error = crate::cli::execute(&invocation).err().unwrap();
        if purpose != Some("visual_interpretation") {
            assert_eq!(error.error.code(), "USAGE");
            assert!(error.error.message().contains("purpose"));
        }
        assert_eq!(fs::read(&input).unwrap(), exact);
        assert_eq!(fs::read_dir(&data).unwrap().count(), 0);
        assert!(!credentials.exists());
    }
    for command in [
        InboxCommand::Visual("../outside.md".into()),
        InboxCommand::VisualAnnotation(Uuid::nil()),
    ] {
        let invocation = Invocation {
            json: true,
            data_dir: data.clone(),
            vault: None,
            credentials_dir: Some(credentials.clone()),
            model_dir: None,
            command: Command::Inbox(command),
        };
        assert!(crate::cli::execute(&invocation).is_err());
        assert_eq!(fs::read_dir(&data).unwrap().count(), 0);
        assert!(!credentials.exists());
    }
}

#[cfg(target_os = "macos")]
mod saved {
    use super::*;
    use crate::cli::proposals::ProposalCommand;
    use brn_workflow::{
        inbox::InboxItem,
        inbox_actions::{InboxActionCapture, InboxVisualEvidence},
        proposals::{DraftNoteChange, DraftRequest},
    };
    const DOCX: &[u8] =
        include_bytes!("../../../brn-workflow/src/inbox_processing/fixtures/inline-png.docx");
    const PNG: &[u8] =
        include_bytes!("../../../brn-workflow/src/inbox_processing/fixtures/inline.png");
    struct Fixture {
        base: tempfile::TempDir,
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
            let f = Self {
                base,
                data,
                vault,
                credentials,
            };
            let original = f.base.path().join("original.docx");
            fs::write(&original, DOCX).unwrap();
            let result = f
                .inbox(InboxCommand::AddBinary {
                    id: Uuid::new_v4(),
                    title: "Synthetic illustration".into(),
                    original_name: Some("original.docx".into()),
                    input: original,
                })
                .unwrap();
            let item: InboxItem = serde_json::from_value(result.data).unwrap();
            // Seed a canonical historical saved singleton profile; new imports must not
            // dispatch to the retired converter merely to exercise historical readers.
            let asset_name = brn_store::work::inbox_visual::asset_name(&item.capture.copy.sha256);
            let payload: serde_json::Value = serde_json::from_str(include_str!(
                "../../../brn-workflow/src/inbox_processing/fixtures/inline-png.legacy.json"
            ))
            .unwrap();
            let body = payload["body"].as_str().unwrap().to_owned();
            let mut metadata = payload["visual"].clone();
            metadata.as_object_mut().unwrap().remove("bytes");
            metadata["converted_byte_len"] = body.len().into();
            metadata["converted_sha256"] = serde_json::json!(brn_intake::digest(body.as_bytes()));
            let proof: brn_workflow::inbox_processing::InboxSourceVisual =
                serde_json::from_value(metadata).unwrap();
            let binding = brn_workflow::inbox_processing::InboxSourceBinding {
                extraction: None,
                visual: Some(proof),
                batch_id: Uuid::new_v4(),
                index: 0,
                original: item,
                format: brn_workflow::inbox_processing::InboxConversionFormat::DocxInlinePngV1,
                byte_len: body.len() as u64,
                sha256: brn_intake::digest(body.as_bytes()),
                note_id: Uuid::new_v4(),
            };
            fs::write(f.vault.join("source.md"), binding.markdown(&body).unwrap()).unwrap();
            fs::write(f.vault.join(asset_name), PNG).unwrap();
            f
        }
        fn json_file(&self, name: &str, value: &impl serde::Serialize) -> PathBuf {
            let file = self.base.path().join(name);
            fs::write(&file, serde_json::to_vec(value).unwrap()).unwrap();
            file
        }
        fn run(&self, command: Command) -> Result<Output, CliFailure> {
            crate::cli::execute(&Invocation {
                json: true,
                data_dir: self.data.clone(),
                vault: Some(self.vault.clone()),
                credentials_dir: Some(self.credentials.clone()),
                model_dir: None,
                command,
            })
        }
        fn inbox(&self, command: InboxCommand) -> Result<Output, CliFailure> {
            self.run(Command::Inbox(command))
        }
        fn evidence(&self) -> InboxVisualEvidence {
            serde_json::from_value(
                self.inbox(InboxCommand::Visual("source.md".into()))
                    .unwrap()
                    .data,
            )
            .unwrap()
        }
        fn request(&self, evidence: &InboxVisualEvidence) -> InboxActionRequest {
            InboxActionRequest {
                id: Uuid::new_v4(),
                conversation: None,
                source: Some(evidence.source.clone()),
                intake: None,
                selection: brn_workflow::Selection {
                    provider: brn_workflow::Provider::Chatgpt,
                    model: "gpt-6-luna".into(),
                },
                effort: brn_workflow::ReasoningEffort::Medium,
                generation: 73,
                purpose: InboxAnalysisPurpose::VisualInterpretation,
                visual_asset: Some(evidence.asset.clone()),
            }
        }
        fn finish(&self, request: &InboxActionRequest, status: brn_workflow::WorkTurnStatus) {
            let capture = InboxActionCapture {
                id: request.id,
                conversation: request.conversation,
                source: request.source.as_ref().map(|s| s.source.clone()),
                intake: None,
                source_text: request.source.as_ref().unwrap().text.clone(),
                provider: "chatgpt".into(),
                model: request.selection.model.clone(),
                effort: "medium".into(),
                purpose: request.purpose,
                visual_asset: request.visual_asset.clone(),
            };
            let (mut store, _) = brn_store::WorkStore::open(&self.data).unwrap();
            let job = store
                .reserve_inbox_action(&capture, "Synthetic retained interpretation question")
                .unwrap();
            store.begin_inbox_action_turn(&job).unwrap();
            store.finish_turn(request.id,status,r#"{"description":"Tentative illustration õ","uncertainty":"Meaning remains uncertain."}"#,None).unwrap();
        }
    }
    #[test]
    fn visual_archive_inspection_keeps_read_only_evidence_scope() {
        let _cancel = crate::tests::CancelTestGuard::with(false);
        let f = Fixture::new();
        let old = f.evidence();
        fs::create_dir(f.vault.join("archive")).unwrap();
        fs::rename(f.vault.join("source.md"), f.vault.join("archive/source.md")).unwrap();
        let asset_path = format!("archive/{}", old.asset.path);
        fs::rename(f.vault.join(&old.asset.path), f.vault.join(&asset_path)).unwrap();
        let mut app = brn_workflow::app::App::open(
            &f.data,
            brn_workflow::app::AppConfig {
                vault_root: Some(f.vault.clone()),
                credentials_dir: Some(f.credentials.clone()),
                model_dir: None,
            },
        )
        .unwrap();
        let expected = app.inbox_visual_evidence("archive/source.md").unwrap();
        assert_eq!(expected.source.text, old.source.text);
        assert_eq!(expected.asset.path, asset_path);
        assert_eq!(expected.bytes, PNG);
        drop(app);
        let output = f
            .inbox(InboxCommand::Visual("archive/source.md".into()))
            .unwrap();
        let observed: InboxVisualEvidence = serde_json::from_value(output.data).unwrap();
        assert_eq!(observed, expected);
        let request = f.request(&observed);
        f.finish(&request, brn_workflow::WorkTurnStatus::Completed);
        assert!(f.inbox(InboxCommand::VisualAnnotation(request.id)).is_err());
        assert_eq!(
            fs::read_to_string(f.vault.join("archive/source.md")).unwrap(),
            old.source.text
        );
        assert_eq!(fs::read(f.vault.join(asset_path)).unwrap(), PNG);
        for path in [
            "archive/../source.md",
            "archive/.hidden.md",
            "archive/image.png",
        ] {
            assert!(super::super::metadata(&InboxCommand::Visual(path.into())).is_err());
        }
    }
    #[test]
    fn visual_complete_bytes_proofs_and_annotation_are_correlated_and_provider_free() {
        let _cancel = crate::tests::CancelTestGuard::with(false);
        let f = Fixture::new();
        let output = f.inbox(InboxCommand::Visual("source.md".into())).unwrap();
        let evidence: InboxVisualEvidence = serde_json::from_value(output.data.clone()).unwrap();
        evidence.validate().unwrap();
        assert_eq!(evidence.bytes, PNG);
        assert!(output.data["bytes"].is_string());
        assert!(output.text.contains("use --json"));
        assert!(!output.text.contains(output.data["bytes"].as_str().unwrap()));
        assert_eq!(
            super::super::output(
                &AppCommand::InboxVisualEvidence("source.md".into()),
                AppEvent::InboxVisualEvidence(Box::new(evidence.clone()))
            )
            .unwrap()
            .data,
            output.data
        );
        assert!(super::super::output(
            &AppCommand::InboxVisualEvidence("another.md".into()),
            AppEvent::InboxVisualEvidence(Box::new(evidence.clone()))
        )
        .is_err());
        for field in 0..4 {
            let mut forged = evidence.clone();
            match field {
                0 => forged.bytes[20] ^= 1,
                1 => forged.asset.path = "another.png".into(),
                2 => forged.asset.fingerprint.sha256[0] ^= 1,
                _ => forged.source.text.push_str("changed"),
            }
            assert!(super::super::output(
                &AppCommand::InboxVisualEvidence("source.md".into()),
                AppEvent::InboxVisualEvidence(Box::new(forged))
            )
            .is_err());
        }
        let before = fs::read(f.vault.join("source.md")).unwrap();
        let image = fs::read(f.vault.join(&evidence.asset.path)).unwrap();
        let request = f.request(&evidence);
        request.validate().unwrap();
        f.finish(&request, brn_workflow::WorkTurnStatus::Completed);
        let input = f.json_file("interpret.json", &request);
        let AppCommand::AnalyzeInboxActions(prepared) = prepare(&InboxCommand::InterpretVisual {
            input: input.clone(),
            timeout_seconds: 30,
        })
        .unwrap() else {
            panic!("explicit image request")
        };
        assert_eq!(*prepared, request);
        let replay = f
            .inbox(InboxCommand::InterpretVisual {
                input: input.clone(),
                timeout_seconds: 30,
            })
            .unwrap();
        assert_eq!(replay.data["status"], "completed");
        assert_eq!(replay.data["operation_id"], serde_json::json!(request.id));
        let prepared = f.inbox(InboxCommand::VisualAnnotation(request.id)).unwrap();
        let draft: DraftRequest = serde_json::from_value(prepared.data.clone()).unwrap();
        draft.validate().unwrap();
        assert_eq!(draft.group_id, Some(request.id));
        let binding = draft.inbox_visual.as_ref().unwrap();
        assert_eq!(binding.source, evidence.source.source);
        assert_eq!(binding.asset, evidence.asset);
        let [DraftNoteChange::Replace { text, .. }] = draft.changes.as_slice() else {
            panic!("single protected Replace")
        };
        assert_eq!(*text, binding.candidate_text().unwrap());
        assert!(text.contains("Tentative model interpretation"));
        assert!(prepared.text.contains("Meaning remains uncertain."));
        assert!(super::super::output(
            &AppCommand::PrepareInboxVisualAnnotation(Uuid::new_v4()),
            AppEvent::InboxVisualDraft(Box::new(draft.clone()))
        )
        .is_err());
        let mut missing_session = draft.clone();
        missing_session.session_id = None;
        missing_session.validate().unwrap();
        assert!(super::super::output(
            &AppCommand::PrepareInboxVisualAnnotation(request.id),
            AppEvent::InboxVisualDraft(Box::new(missing_session))
        )
        .is_err());
        let mut edited = draft.clone();
        let DraftNoteChange::Replace { text, .. } = &mut edited.changes[0] else {
            unreachable!()
        };
        *text = text.replace("Tentative illustration õ", "Owner revised description");
        // Valid review wording edits are allowed by the domain. Preparation
        // must still return the original exact candidate, not an edited review.
        edited.validate().unwrap();
        assert!(super::super::output(
            &AppCommand::PrepareInboxVisualAnnotation(request.id),
            AppEvent::InboxVisualDraft(Box::new(edited))
        )
        .is_err());
        for field in 0..4 {
            let mut forged = draft.clone();
            match field {
                0 => forged.group_id = Some(Uuid::new_v4()),
                1 => forged.sources[0].fingerprint.sha256[0] ^= 1,
                2 => {
                    forged
                        .inbox_visual
                        .as_mut()
                        .unwrap()
                        .asset
                        .fingerprint
                        .sha256[0] ^= 1
                }
                _ => {
                    let DraftNoteChange::Replace { text, .. } = &mut forged.changes[0] else {
                        unreachable!()
                    };
                    text.push_str("forged");
                }
            }
            assert!(super::super::output(
                &AppCommand::PrepareInboxVisualAnnotation(request.id),
                AppEvent::InboxVisualDraft(Box::new(forged))
            )
            .is_err());
        }
        assert_eq!(
            f.inbox(InboxCommand::VisualAnnotation(request.id))
                .unwrap()
                .data,
            prepared.data
        );
        let grouped = f
            .run(Command::Proposals(ProposalCommand::List(Some(request.id))))
            .unwrap();
        assert!(grouped.data.as_array().unwrap().is_empty());
        assert_eq!(fs::read(f.vault.join("source.md")).unwrap(), before);
        assert_eq!(fs::read(f.vault.join(&evidence.asset.path)).unwrap(), image);
        assert_eq!(fs::read_dir(&f.vault).unwrap().count(), 2);
        assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
        // Terminal replay remains local after Source/asset loss; fresh reads and
        // new admissions refuse. No default provider/model is queried.
        fs::remove_file(f.vault.join("source.md")).unwrap();
        fs::remove_file(f.vault.join(&evidence.asset.path)).unwrap();
        assert_eq!(
            f.inbox(InboxCommand::InterpretVisual {
                input,
                timeout_seconds: 30
            })
            .unwrap()
            .data,
            replay.data
        );
        assert!(f.inbox(InboxCommand::Visual("source.md".into())).is_err());
    }
    #[test]
    fn visual_stale_png_and_interrupted_analysis_never_prepare_or_install_annotation() {
        let _cancel = crate::tests::CancelTestGuard::with(false);
        let f = Fixture::new();
        let evidence = f.evidence();
        let before = fs::read(f.vault.join("source.md")).unwrap();
        let request = f.request(&evidence);
        f.finish(&request, brn_workflow::WorkTurnStatus::Interrupted);
        let error = f
            .inbox(InboxCommand::InterpretVisual {
                input: f.json_file("interrupted.json", &request),
                timeout_seconds: 30,
            })
            .err()
            .unwrap();
        assert_eq!(error.error.exit_code(), 130);
        assert!(f.inbox(InboxCommand::VisualAnnotation(request.id)).is_err());
        fs::write(f.vault.join(&evidence.asset.path), b"changed").unwrap();
        assert!(f.inbox(InboxCommand::Visual("source.md".into())).is_err());
        let fresh = f.request(&evidence);
        let failure = f
            .inbox(InboxCommand::InterpretVisual {
                input: f.json_file("stale.json", &fresh),
                timeout_seconds: 30,
            })
            .err()
            .unwrap();
        assert_eq!(failure.error.code(), "CONTEXT_STALE");
        assert_eq!(fs::read(f.vault.join("source.md")).unwrap(), before);
        assert_eq!(
            fs::read(f.vault.join(&evidence.asset.path)).unwrap(),
            b"changed"
        );
        assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
    }
}
