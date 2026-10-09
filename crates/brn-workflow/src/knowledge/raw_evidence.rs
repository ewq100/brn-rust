//! Explicit raw saved evidence grants no Current classification or write authority.
use crate::{Result, app::App};
use brn_ai::{RawEvidence, RawEvidenceRequest, ReadTools};

impl App {
    pub fn raw_evidence(&self, request: &RawEvidenceRequest) -> Result<RawEvidence> {
        request.validate()?;
        self.require_current_evidence()?;
        Ok(self.guarded_tools()?.read_raw_evidence(request)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::AppConfig,
        app_worker::{AppCommand, AppEvent, AppWorker},
    };
    use brn_ai::ReadScope;
    use sha2::{Digest, Sha256};
    use std::{fs, path::PathBuf, time::Duration};
    use uuid::Uuid;

    struct Fixture {
        _owner: tempfile::TempDir,
        data: PathBuf,
        vault: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            let data = owner.path().join("data");
            let vault = owner.path().join("vault");
            fs::create_dir(&data).unwrap();
            fs::create_dir(&vault).unwrap();
            Self {
                _owner: owner,
                data,
                vault,
            }
        }
        fn config(&self) -> AppConfig {
            AppConfig {
                vault_root: Some(self.vault.clone()),
                credentials_dir: Some(self.data.with_file_name("synthetic.credentials")),
                model_dir: None,
            }
        }
        fn app(&self) -> App {
            App::open(&self.data, self.config()).unwrap()
        }
        fn write(&self, path: &str, text: &str) {
            let path = self.vault.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
    }
    fn request(path: &str) -> RawEvidenceRequest {
        RawEvidenceRequest {
            path: path.into(),
            start_byte: 0,
            end_byte: None,
            expected_sha256: None,
        }
    }

    #[test]
    fn malformed_saved_fields_are_raw_without_current_facts_or_effects() {
        let fixture = Fixture::new();
        let fields = [
            "brn_id: invalid",
            "brn_kind: invalid",
            "brn_state: invalid",
            "brn_provenance: invalid",
            "brn_kind: source\r\nbrn_inbox_source: invalid",
        ];
        for (i, field) in fields.iter().enumerate() {
            fixture.write(
                &format!("raw-{i}.md"),
                &format!("\u{feff}---\r\n{field}\r\n---\r\n# Original õ 日本語 🦀\r\n"),
            );
        }
        let app = fixture.app();
        for i in 0..fields.len() {
            let path = format!("raw-{i}.md");
            let original = fs::read(fixture.vault.join(&path)).unwrap();
            let reply = app.raw_evidence(&request(&path)).unwrap();
            assert_eq!(reply.text.as_bytes(), original);
            assert_eq!(reply.sha256, <[u8; 32]>::from(Sha256::digest(&original)));
            assert!(reply.facts.is_none());
            assert!(!reply.metadata_issue.as_ref().unwrap().is_empty());
            assert!(!reply.partial);
            assert!(
                app.tools()
                    .unwrap()
                    .read_note_scoped(&path, ReadScope::Current)
                    .is_err()
            );
            assert_eq!(fs::read(fixture.vault.join(&path)).unwrap(), original);
        }
        assert!(app.work_store().proposals(None).unwrap().is_empty());
        assert!(
            app.work_store()
                .findings(&Default::default())
                .unwrap()
                .entries
                .is_empty()
        );
        assert!(
            app.work_store()
                .action_list(&Default::default())
                .unwrap()
                .entries
                .is_empty()
        );
    }

    #[test]
    fn raw_prefix_and_near_megabyte_tail_preserve_bytes_and_refuse_changed_full_hash() {
        let fixture = Fixture::new();
        let mut text = "\u{feff}---\r\nbrn_id: invalid\r\n---\r\n# Raw\r\n".to_owned();
        text.push_str(&"x".repeat(49_999 - text.len()));
        text.push('🦀');
        text.push_str(&"x".repeat(949_000 - text.len()));
        text.push_str("\r\nTAIL õ 日本語 🦀\r\n");
        fixture.write("long.md", &text);
        let app = fixture.app();
        let prefix = app.raw_evidence(&request("long.md")).unwrap();
        assert_eq!(prefix.end_byte, 49_999);
        assert!(prefix.partial);
        assert_eq!(prefix.total_bytes, text.len());
        assert_eq!(prefix.text, &text[..49_999]);
        let start = text.find("TAIL").unwrap();
        let range = RawEvidenceRequest {
            path: "long.md".into(),
            start_byte: start,
            end_byte: Some(text.len()),
            expected_sha256: Some(prefix.sha256),
        };
        let tail = app.raw_evidence(&range).unwrap();
        assert_eq!(tail.text, &text[start..]);
        assert!(tail.facts.is_none());
        assert!(tail.partial);
        let modified = fs::metadata(fixture.vault.join("long.md"))
            .unwrap()
            .modified()
            .unwrap();
        let changed = text.replacen("xxxxx", "yyyyy", 1);
        fixture.write("long.md", &changed);
        fs::File::options()
            .write(true)
            .open(fixture.vault.join("long.md"))
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(modified))
            .unwrap();
        assert_eq!(&changed[start..], &text[start..]);
        assert_eq!(changed.len(), text.len());
        assert!(app.raw_evidence(&range).is_err());
        assert_ne!(
            app.raw_evidence(&request("long.md")).unwrap().sha256,
            prefix.sha256
        );
        assert_eq!(
            fs::read_to_string(fixture.vault.join("long.md")).unwrap(),
            changed
        );
    }

    #[test]
    fn valid_unmanaged_source_history_and_archive_metadata_remain_observations() {
        let fixture = Fixture::new();
        for (path, text) in [
            ("plain.md", "# Unmanaged\n"),
            ("source.md", "---\nbrn_kind: source\n---\n# Source\n"),
            ("history.md", "---\nbrn_state: history\n---\n# History\n"),
            ("ArChIvE/moved.md", "# Archived\n"),
            ("empty.md", ""),
        ] {
            fixture.write(path, text);
        }
        let app = fixture.app();
        for (path, source, history) in [
            ("plain.md", false, false),
            ("source.md", true, false),
            ("history.md", false, true),
            ("ArChIvE/moved.md", false, true),
            ("empty.md", false, false),
        ] {
            let reply = app.raw_evidence(&request(path)).unwrap();
            let facts = reply.facts.as_ref().unwrap();
            assert_eq!((facts.source, facts.history), (source, history));
            assert_eq!(facts.sha256, reply.sha256);
            assert!(reply.metadata_issue.is_none());
        }
    }

    #[test]
    fn exact_ranges_boundaries_root_and_epoch_refuse_without_fallback() {
        let fixture = Fixture::new();
        fixture.write("raw.md", "\u{feff}õ🦀\r\n");
        let app = fixture.app();
        let whole = app.raw_evidence(&request("raw.md")).unwrap();
        for (start, end) in [
            (0, 0),
            (3, 5),
            (5, 9),
            (whole.total_bytes, whole.total_bytes),
        ] {
            let req = RawEvidenceRequest {
                path: "raw.md".into(),
                start_byte: start,
                end_byte: Some(end),
                expected_sha256: Some(whole.sha256),
            };
            assert_eq!(
                app.raw_evidence(&req).unwrap().text,
                &whole.text[start..end]
            );
        }
        for (start, end) in [(1, 3), (3, 4), (6, 9), (0, whole.total_bytes + 1)] {
            assert!(
                app.raw_evidence(&RawEvidenceRequest {
                    path: "raw.md".into(),
                    start_byte: start,
                    end_byte: Some(end),
                    expected_sha256: Some(whole.sha256)
                })
                .is_err()
            );
        }
        let tools = app.tools().unwrap();
        tools.set_current_blocked(true);
        assert!(app.raw_evidence(&request("raw.md")).is_err());
        tools.set_current_blocked(false);
        assert_eq!(
            app.raw_evidence(&request("raw.md")).unwrap().sha256,
            whole.sha256
        );
        fs::rename(
            &fixture.vault,
            fixture.vault.with_file_name("retained-vault"),
        )
        .unwrap();
        fs::create_dir(&fixture.vault).unwrap();
        fixture.write("raw.md", "Foreign replacement; do not read\n");
        assert!(app.raw_evidence(&request("raw.md")).is_err());
    }

    #[test]
    fn raw_reader_keeps_containment_regular_utf8_and_size_guards() {
        let fixture = Fixture::new();
        fixture.write("valid.md", "# Valid\n");
        fs::create_dir(fixture.vault.join("directory.md")).unwrap();
        fs::write(fixture.vault.join("invalid.md"), [0xff]).unwrap();
        fs::write(
            fixture.vault.join("large.md"),
            vec![b'x'; crate::MAX_NOTE_BYTES + 1],
        )
        .unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            fixture.vault.join("valid.md"),
            fixture.vault.join("link.md"),
        )
        .unwrap();
        let app = fixture.app();
        for path in [
            "../outside.md",
            "/absolute.md",
            ".hidden.md",
            "folder/../valid.md",
            "directory.md",
            "invalid.md",
            "large.md",
            "missing.md",
            "valid.txt",
        ] {
            assert!(app.raw_evidence(&request(path)).is_err(), "{path}");
        }
        #[cfg(unix)]
        assert!(app.raw_evidence(&request("link.md")).is_err());
        assert_eq!(
            app.raw_evidence(&request("valid.md")).unwrap().text,
            "# Valid\n"
        );
    }

    #[test]
    fn worker_returns_exact_bound_raw_reply_and_preserves_files() {
        let fixture = Fixture::new();
        fixture.write("raw.md", "---\nbrn_id: invalid\n---\nOriginal õ\n");
        let worker = AppWorker::start(fixture.data.clone(), fixture.config()).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(20))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let id = Uuid::new_v4();
        let req = request("raw.md");
        worker
            .submit(id, AppCommand::RawEvidence(req.clone()))
            .unwrap();
        let (reply, event) = worker.recv_event_timeout(Duration::from_secs(20)).unwrap();
        assert_eq!(reply, id);
        let AppEvent::RawEvidence(raw) = event else {
            panic!("expected raw reply")
        };
        raw.validate_for(&req).unwrap();
        assert!(raw.facts.is_none());
        assert_eq!(
            raw.text,
            fs::read_to_string(fixture.vault.join("raw.md")).unwrap()
        );
    }
}
