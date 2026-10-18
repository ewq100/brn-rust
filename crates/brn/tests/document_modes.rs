//! Real CLI processes, fixed public fixtures and synthetic data only.
mod support;
#[cfg(target_os = "macos")]
use brn_threads_app::RecordData;
use brn_threads_app::{DocumentConversionSettings, DocumentConverter, Workspace};
use serde_json::json;
use support::*;

#[cfg(target_os = "macos")]
fn capture(name: &str, value: &impl serde::Serialize) {
    if let Some(path) = std::env::var_os("BRN_TASK3_CAPTURES") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join(format!("{name}.json")),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
    }
}
#[cfg(target_os = "macos")]
fn root() -> Root {
    let mut root = Root::new();
    for name in ["BRN_XBERG_HELPER", "BRN_XBERG_HELPER_BRN"] {
        root.env.push((
            name.into(),
            std::env::var(name).expect("declared helper artifact required for macOS qualification"),
        ));
    }
    root.env.extend(support::stub::proxy_env());
    root
}
fn select(root: &Root, data: &std::path::Path, converter: &str, ai: &str) {
    ok(
        root,
        data,
        &["document-conversion", "--converter", converter, "--ai", ai],
    );
}
#[cfg(target_os = "macos")]
fn corpus() -> Vec<(&'static str, &'static [u8])> {
    vec![
        (
            "pdf",
            include_bytes!("../../brn-threads-intake/tests/fixtures/harbor-study.pdf"),
        ),
        (
            "docx",
            include_bytes!("../../brn-intake/tests/fixtures/harbor.docx"),
        ),
        (
            "pptx",
            include_bytes!("../../brn-intake/tests/fixtures/quay.pptx"),
        ),
        (
            "xlsx",
            include_bytes!("../../brn-intake/tests/fixtures/forecast.xlsx"),
        ),
        (
            "eml",
            include_bytes!("../../brn-intake/tests/fixtures/single.eml"),
        ),
        ("html", b"<p>Synthetic HTML source</p>"),
        ("txt", b"Exact text\r\n"),
        ("md", b"# Exact Markdown\n"),
    ]
}
#[test]
fn documents_cli_defaults_checked_settings_and_reopen_without_note_effect() {
    let root = Root::new();
    let data = root.workspace("data");
    let original = note(&root, &data, "Unchanged", "Original body");
    let before = ok(&root, &data, &["read", "--id", &original]);
    assert_eq!(
        ok(&root, &data, &["settings"])["document_conversion"],
        json!({"converter":"xberg","ai":false})
    );
    for (converter, ai, expected) in [
        ("xberg-brn", "on", DocumentConverter::XbergBrn),
        ("xberg", "on", DocumentConverter::Xberg),
        ("xberg", "off", DocumentConverter::Xberg),
    ] {
        select(&root, &data, converter, ai);
        assert_eq!(
            Workspace::open_existing_no_upgrade(&data)
                .unwrap()
                .settings()
                .unwrap()
                .document_conversion,
            DocumentConversionSettings {
                converter: expected,
                ai: ai == "on"
            }
        );
    }
    assert_eq!(ok(&root, &data, &["read", "--id", &original]), before);
    let refused = call(
        &root,
        &data,
        &[
            "document-conversion",
            "--converter",
            "legacy",
            "--ai",
            "off",
        ],
    );
    assert_error(&refused, "usage", "invalid_value");
}
#[test]
#[cfg(target_os = "macos")]
fn documents_cli_all_eight_both_engines_off_import_without_auth() {
    let root = root();
    let data = root.workspace("data");
    for converter in ["xberg", "xberg-brn"] {
        select(&root, &data, converter, "off");
        for (extension, bytes) in corpus() {
            let file = root.file(&format!("source.{extension}"), bytes);
            let result = call(
                &root,
                &data,
                &[
                    "intake",
                    "--source",
                    &file,
                    "--intent",
                    "full",
                    "--title",
                    extension,
                    "--key",
                    &format!("{converter}-{extension}"),
                ],
            );
            capture(&format!("off-{converter}-{extension}"), &result);
            assert_eq!(
                result["outcome"], "Applied",
                "{converter}/{extension}: {result}"
            );
        }
    }
    let records = Workspace::open_existing_no_upgrade(&data)
        .unwrap()
        .records()
        .unwrap();
    capture("off-all-eight-records", &records);
    let notes = records
        .iter()
        .filter_map(|r| match &r.data {
            RecordData::Note(n) if n.import.is_some() => Some(n),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(notes.len(), 16);
    for note in notes {
        assert!(note.protected);
        let report = note.import.as_ref().unwrap().report.as_ref().unwrap();
        assert!(!report.conversion.as_ref().unwrap().settings.ai);
        assert!(!note.markdown.trim().is_empty());
    }
    assert_eq!(
        records
            .iter()
            .filter(|r| matches!(r.data, RecordData::Source(_)))
            .count(),
        8
    );
}
#[test]
#[cfg(target_os = "macos")]
fn documents_cli_replay_checks_variant_before_auth_or_helper_and_preserves_old_note() {
    let mut root = root();
    let data = root.workspace("data");
    let file = root.file("source.txt", b"Source snapshot\r\n");
    let args = [
        "intake", "--source", &file, "--intent", "full", "--key", "same",
    ];
    assert_eq!(call(&root, &data, &args)["outcome"], "Applied");
    let before = Workspace::open_existing_no_upgrade(&data)
        .unwrap()
        .records()
        .unwrap();
    for pair in &mut root.env {
        if pair.0.starts_with("BRN_XBERG_HELPER") {
            pair.1 = "/missing/helper".into();
        }
    }
    assert_eq!(call(&root, &data, &args)["retry"], "replay");
    select(&root, &data, "xberg-brn", "on");
    let mismatch = call(&root, &data, &args);
    assert_eq!(mismatch["error"]["code"], "key_conflict", "{mismatch}");
    let missing_auth = call(
        &root,
        &data,
        &[
            "intake", "--source", &file, "--intent", "full", "--key", "other",
        ],
    );
    assert_error(&missing_auth, "rejected", "credentials_not_configured");
    // Restore trusted artifacts; a distinct variant creates a separate protected Note.
    for pair in &mut root.env {
        if pair.0.starts_with("BRN_XBERG_HELPER") {
            pair.1 = std::env::var(&pair.0).unwrap();
        }
    }
    select(&root, &data, "xberg-brn", "off");
    assert_eq!(
        call(
            &root,
            &data,
            &["intake", "--source", &file, "--intent", "full", "--key", "second"]
        )["outcome"],
        "Applied"
    );
    let after = Workspace::open_existing_no_upgrade(&data)
        .unwrap()
        .records()
        .unwrap();
    assert_eq!(
        after
            .iter()
            .filter(|r| matches!(r.data, RecordData::Source(_)))
            .count(),
        1
    );
    assert_eq!(
        after
            .iter()
            .filter(|r| matches!(r.data, RecordData::Note(_)))
            .count(),
        2
    );
    for old in before
        .iter()
        .filter(|r| matches!(r.data, RecordData::Note(_)))
    {
        assert!(after.contains(old));
    }
}
#[test]
#[cfg(target_os = "macos")]
fn documents_cli_missing_or_wrong_selected_helper_never_falls_back_or_saves() {
    for wrong in ["/missing/helper", "stock-as-brn", "relative/helper"] {
        let mut root = root();
        let data = root.workspace("data");
        select(&root, &data, "xberg-brn", "off");
        let stock = std::env::var("BRN_XBERG_HELPER").unwrap();
        root.env
            .iter_mut()
            .find(|p| p.0 == "BRN_XBERG_HELPER_BRN")
            .unwrap()
            .1 = if wrong == "stock-as-brn" {
            stock
        } else {
            wrong.into()
        };
        let file = root.file("source.md", b"# No fallback\n");
        let result = call(
            &root,
            &data,
            &["intake", "--source", &file, "--intent", "full"],
        );
        assert_ne!(result["status"], "ok", "{result}");
        assert!(!Workspace::open_existing_no_upgrade(&data)
            .unwrap()
            .records()
            .unwrap()
            .iter()
            .any(|r| matches!(r.data, RecordData::Note(_) | RecordData::Source(_))));
    }
}
#[test]
#[cfg(target_os = "macos")]
fn documents_cli_useful_handoff_stores_captured_report_before_agent() {
    for converter in ["xberg", "xberg-brn"] {
        let root = root();
        let data = root.workspace("data");
        select(&root, &data, converter, "off");
        let stub =
            support::stub::Stub::start(vec![support::stub::answer("Synthetic useful result.")]);
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
        let file = root.file("source.txt", b"Exact useful source\n");
        let result = call(
            &root,
            &data,
            &["intake", "--source", &file, "--intent", "useful"],
        );
        assert_eq!(result["outcome"], "Completed", "{result}");
        assert_eq!(stub.requests().len(), 1);
        let records = Workspace::open_existing_no_upgrade(&data)
            .unwrap()
            .records()
            .unwrap();
        let source = records
            .iter()
            .find_map(|r| match &r.data {
                RecordData::Source(s) => Some(s),
                _ => None,
            })
            .unwrap();
        let metadata = source.report.as_ref().unwrap().conversion.as_ref().unwrap();
        assert_eq!(
            metadata.settings.converter,
            if converter == "xberg" {
                DocumentConverter::Xberg
            } else {
                DocumentConverter::XbergBrn
            }
        );
        assert!(!metadata.settings.ai);
    }
}
