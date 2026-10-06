use super::*;
use crate::cli::{Command, Invocation, Outcome};
use std::fs;

fn tokens(parts: &[&str], data: &std::path::Path) -> Vec<String> {
    parts
        .iter()
        .copied()
        .chain(["--data-dir", data.to_str().unwrap()])
        .map(str::to_owned)
        .collect()
}

#[test]
fn binary_parser_keeps_explicit_metadata_and_the_text_command_text_only() {
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let id = Uuid::new_v4().to_string();
    let args = tokens(
        &[
            "inbox",
            "add-binary",
            "--id",
            &id,
            "--title",
            "Exact binary õ",
            "--file",
            "synthetic.pdf",
            "--original-name",
            "Label.pdf",
        ],
        owner.path(),
    );
    let Outcome::Run(parsed) =
        crate::cli::parse(&args).unwrap_or_else(|_| panic!("binary arguments"))
    else {
        panic!("binary invocation")
    };
    let Command::Inbox(command) = parsed.command else {
        panic!("Inbox command")
    };
    assert_eq!(command.name(), "inbox.add-binary");
    let InboxCommand::AddBinary {
        id: actual,
        title,
        original_name,
        input,
    } = command
    else {
        panic!("binary input")
    };
    assert_eq!(actual.to_string(), id);
    assert_eq!(title, "Exact binary õ");
    assert_eq!(original_name.as_deref(), Some("Label.pdf"));
    assert_eq!(input, PathBuf::from("synthetic.pdf"));
    for args in [
        vec!["inbox", "add-binary", "--title", "x", "--file", "x"],
        vec!["inbox", "add-binary", "--id", &id, "--file", "x"],
        vec!["inbox", "add-binary", "--id", &id, "--title", "x"],
        vec![
            "inbox",
            "add-binary",
            "--id",
            &id,
            "--title",
            " ",
            "--file",
            "x",
        ],
        vec![
            "inbox",
            "add-binary",
            "--id",
            "00000000-0000-0000-0000-000000000000",
            "--title",
            "x",
            "--file",
            "x",
        ],
        vec![
            "inbox",
            "add-binary",
            "--id",
            &id,
            "--title",
            "x",
            "--file",
            "x",
            "--kind",
            "binary",
        ],
        vec![
            "inbox",
            "add-binary",
            "unexpected",
            "--id",
            &id,
            "--title",
            "x",
            "--file",
            "x",
        ],
        vec![
            "inbox", "add", "--id", &id, "--title", "x", "--file", "x", "--kind", "binary",
        ],
    ] {
        assert!(crate::cli::parse(&tokens(&args, owner.path())).is_err());
    }
    assert_eq!(fs::read_dir(owner.path()).unwrap().count(), 0);
}

#[test]
fn invalid_binary_input_refuses_before_application_startup() {
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    fs::create_dir(&data).unwrap();
    let credentials = owner.path().join("credentials");
    let oversized = owner.path().join("oversized.bin");
    fs::File::create(&oversized)
        .unwrap()
        .set_len((brn_workflow::inbox::MAX_INBOX_BINARY_BYTES + 1) as u64)
        .unwrap();
    let valid = owner.path().join("valid.bin");
    fs::write(&valid, [0, 0xff, 0xfe]).unwrap();
    for (id, title, input) in [
        (Uuid::new_v4(), "Binary", oversized),
        (Uuid::new_v4(), "Binary", owner.path().join("missing")),
        (Uuid::new_v4(), "Binary", owner.path().to_owned()),
        (Uuid::nil(), "Binary", valid.clone()),
        (Uuid::new_v4(), " ", valid),
    ] {
        let invocation = Invocation {
            json: true,
            data_dir: data.clone(),
            vault: None,
            credentials_dir: Some(credentials.clone()),
            model_dir: None,
            command: Command::Inbox(InboxCommand::AddBinary {
                id,
                title: title.into(),
                original_name: None,
                input,
            }),
        };
        assert!(crate::cli::execute(&invocation).is_err());
        assert_eq!(fs::read_dir(&data).unwrap().count(), 0);
        assert!(!credentials.exists());
    }
}

#[cfg(target_os = "macos")]
#[test]
fn binary_cli_retains_exact_bytes_replays_and_rejects_forged_proofs_and_conversion() {
    use brn_workflow::inbox::{InboxItem, InboxOriginal, InboxRead, InboxReview};
    let _cancel = crate::tests::CancelTestGuard::with(false);
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    let vault = owner.path().join("vault");
    let credentials = owner.path().join("credentials");
    fs::create_dir(&data).unwrap();
    fs::create_dir(&vault).unwrap();
    fs::write(vault.join("untouched.md"), "\u{feff}Untouched õ\r\n").unwrap();
    let input = owner.path().join("synthetic.pdf");
    let mut bytes = vec![b'x'; brn_workflow::MAX_NOTE_BYTES + 5];
    bytes[..5].copy_from_slice(&[0, 0xff, 0xfe, b'\r', b'\n']);
    fs::write(&input, &bytes).unwrap();
    let id = Uuid::new_v4();
    let invoke = |command| Invocation {
        json: true,
        data_dir: data.clone(),
        vault: Some(vault.clone()),
        credentials_dir: Some(credentials.clone()),
        model_dir: None,
        command: Command::Inbox(command),
    };
    let add = || InboxCommand::AddBinary {
        id,
        title: "Retained binary õ".into(),
        original_name: Some("Synthetic.pdf".into()),
        input: input.clone(),
    };
    let captured = crate::cli::execute(&invoke(add())).unwrap();
    let item: InboxItem = serde_json::from_value(captured.data.clone()).unwrap();
    let request = CaptureBinaryInboxRequest {
        id,
        title: item.capture.title.clone(),
        original_name: item.capture.original_name.clone(),
        bytes: bytes.clone(),
    };
    request.validate_receipt(&item).unwrap();
    assert_eq!(item.capture.kind, InboxKind::Binary);
    assert_eq!(item.capture.copy_name(), format!("{id}.bin"));
    assert!(captured.data.to_string().len() < 8192);
    let owned_copy = data.join("inbox").join(item.capture.copy_name());
    assert_eq!(fs::read(&owned_copy).unwrap(), bytes);
    assert!(!data.join("inbox").join(format!("{id}.txt")).exists());
    assert_eq!(
        crate::cli::execute(&invoke(add())).unwrap().data,
        captured.data
    );
    let shown = crate::cli::execute(&invoke(InboxCommand::Show(id))).unwrap();
    assert!(shown.data["original"].get("text").is_none());
    let read: InboxRead = serde_json::from_value(shown.data).unwrap();
    read.validate_receipt().unwrap();
    assert!(
        matches!(&read.original, InboxOriginal::AvailableBinary { byte_len, sha256 } if *byte_len == bytes.len() as u64 && *sha256 == item.capture.copy.sha256)
    );
    for alter in 0..3 {
        let mut forged = read.clone();
        if let InboxOriginal::AvailableBinary { byte_len, sha256 } = &mut forged.original {
            match alter {
                0 => *byte_len += 1,
                1 => sha256[0] ^= 1,
                _ => forged.item.capture.kind = InboxKind::Text,
            }
        }
        assert!(output(
            &AppCommand::InboxItem(id),
            AppEvent::InboxItem(Box::new(forged))
        )
        .is_err());
    }
    let review = crate::cli::execute(&invoke(InboxCommand::Review(id))).unwrap();
    let mut review: InboxReview = serde_json::from_value(review.data).unwrap();
    if let InboxOriginal::AvailableBinary { sha256, .. } = &mut review.original {
        sha256[0] ^= 1;
    }
    assert!(output(
        &AppCommand::InboxReview(id),
        AppEvent::InboxReview(Box::new(review))
    )
    .is_err());
    let process = ProcessInboxRequest {
        id: Uuid::new_v4(),
        items: vec![item.clone()],
    };
    let process_file = owner.path().join("process.json");
    fs::write(&process_file, serde_json::to_vec(&process).unwrap()).unwrap();
    assert!(crate::cli::execute(&invoke(InboxCommand::Process(process_file))).is_err());
    assert!(crate::cli::execute(&invoke(InboxCommand::RemovalPreview(id))).is_err());
    bytes[0] ^= 1;
    fs::write(&input, &bytes).unwrap();
    assert!(crate::cli::execute(&invoke(add())).is_err());
    assert_eq!(fs::read(&owned_copy).unwrap(), request.bytes);
    assert_eq!(
        fs::read(vault.join("untouched.md")).unwrap(),
        "\u{feff}Untouched õ\r\n".as_bytes()
    );
    assert_eq!(fs::read_dir(&vault).unwrap().count(), 1);
    assert_eq!(fs::read_dir(&credentials).unwrap().count(), 0);
}
