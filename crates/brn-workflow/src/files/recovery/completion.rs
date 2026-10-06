//! Immutable recovery evidence for an exact, directly requested Action completion.
use super::{ApplyRecoveryFiles, FileResult, RetainedArtifact, Uuid, invalid};
use brn_store::work::action_completion::ActionCompletion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionRecoverySnapshot {
    pub(crate) completion: ActionCompletion,
    pub(crate) proof: RetainedArtifact,
}

#[cfg(target_os = "macos")]
use super::{
    ArtifactIdentity, ArtifactKind, FileErrorCode, Path, PathBuf, Read, Write, before_rename,
    check_file, cstring, digest, failure, full_sync, note_io_failure, open_at, relocated,
    rename_flags, step, sync_directory, uncertain, unchanged,
};
#[cfg(target_os = "macos")]
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use std::os::unix::fs::MetadataExt;

#[cfg(target_os = "macos")]
const PREFIX: &str = ".brn-complete-";
#[cfg(target_os = "macos")]
const SUFFIX: &str = ".receipt";
#[cfg(target_os = "macos")]
const TEMP_PREFIX: &str = ".brn-complete-temp-";
#[cfg(target_os = "macos")]
const TEMP_SUFFIX: &str = ".stage";
#[cfg(target_os = "macos")]
const MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;

#[cfg(target_os = "macos")]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: u8,
    sha256: [u8; 32],
    completion: ActionCompletion,
}
#[cfg(target_os = "macos")]
fn parse_id(raw: &str) -> FileResult<Uuid> {
    let id = Uuid::parse_str(raw).map_err(|_| invalid("malformed completion recovery filename"))?;
    if id.is_nil() || id.to_string() != raw {
        return Err(invalid(
            "completion recovery filename needs a canonical UUID",
        ));
    }
    Ok(id)
}
#[cfg(target_os = "macos")]
fn name(id: Uuid) -> FileResult<String> {
    if id.is_nil() {
        return Err(invalid("completion recovery UUID must not be nil"));
    }
    Ok(format!("{PREFIX}{id}{SUFFIX}"))
}
#[cfg(target_os = "macos")]
fn encode(completion: &ActionCompletion) -> FileResult<Vec<u8>> {
    completion
        .validate()
        .map_err(|_| invalid("invalid completion recovery snapshot"))?;
    let encoded = serde_json::to_vec(completion)
        .map_err(|_| invalid("could not encode completion recovery snapshot"))?;
    let bytes = serde_json::to_vec(&Envelope {
        format: 1,
        sha256: digest(&encoded),
        completion: completion.clone(),
    })
    .map_err(|_| invalid("could not encode completion recovery envelope"))?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(invalid("completion recovery envelope exceeds 4 MiB"));
    }
    Ok(bytes)
}

#[cfg(target_os = "macos")]
impl ApplyRecoveryFiles {
    pub(crate) fn completion_ids(&self) -> FileResult<Vec<Uuid>> {
        let mut ids = Vec::new();
        self.enumerate(|bytes| {
            if !bytes.starts_with(PREFIX.as_bytes()) {
                return Ok(());
            }
            let filename = std::str::from_utf8(bytes)
                .map_err(|_| invalid("malformed completion recovery filename"))?;
            if let Some(id) = filename
                .strip_prefix(TEMP_PREFIX)
                .and_then(|rest| rest.strip_suffix(TEMP_SUFFIX))
            {
                parse_id(id)?;
                return Ok(());
            }
            let id = filename
                .strip_prefix(PREFIX)
                .and_then(|rest| rest.strip_suffix(SUFFIX))
                .ok_or_else(|| invalid("malformed completion recovery filename"))?;
            ids.push(parse_id(id)?);
            Ok(())
        })?;
        ids.sort_unstable();
        Ok(ids)
    }

    fn read_completion_named(
        &self,
        relative: &Path,
        operation: Uuid,
    ) -> FileResult<Option<CompletionRecoverySnapshot>> {
        self.validate_directory()?;
        let mut file = match open_at(&self.directory, relative.as_os_str(), libc::O_RDONLY, 0) {
            Ok(file) => file,
            Err(error) if error.code == FileErrorCode::Missing => {
                self.validate_directory()?;
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let before = file.metadata().map_err(note_io_failure)?;
        check_file(&before)?;
        if before.len() > MAX_RECORD_BYTES as u64 {
            return Err(invalid("completion recovery envelope exceeds 4 MiB"));
        }
        self.path_matches(relative, &before)?;
        let mut bytes = Vec::with_capacity(before.len() as usize);
        step("read")?;
        Read::by_ref(&mut file)
            .take((MAX_RECORD_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(note_io_failure)?;
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(invalid("completion recovery envelope exceeds 4 MiB"));
        }
        let after = file.metadata().map_err(note_io_failure)?;
        check_file(&after)?;
        if !unchanged(&before, &after) || after.len() != bytes.len() as u64 {
            return Err(failure(
                FileErrorCode::Conflict,
                "completion recovery changed while reading",
            ));
        }
        self.path_matches(relative, &after)?;
        let envelope: Envelope = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("invalid completion recovery envelope"))?;
        if envelope.format != 1 || envelope.completion.request.operation_id != operation {
            return Err(invalid(
                "completion recovery format or filename binding differs",
            ));
        }
        envelope
            .completion
            .validate()
            .map_err(|_| invalid("invalid completion recovery snapshot"))?;
        let encoded = serde_json::to_vec(&envelope.completion)
            .map_err(|_| invalid("could not encode completion recovery snapshot"))?;
        if digest(&encoded) != envelope.sha256 {
            return Err(invalid(
                "completion recovery snapshot failed its hash check",
            ));
        }
        self.validate_directory()?;
        Ok(Some(CompletionRecoverySnapshot {
            completion: envelope.completion,
            proof: RetainedArtifact {
                relative: relative.to_owned(),
                identity: ArtifactIdentity {
                    device: after.dev(),
                    inode: after.ino(),
                    len: after.len(),
                    kind: ArtifactKind::Regular,
                },
                sha256: Some(digest(&bytes)),
            },
        }))
    }

    pub(crate) fn read_completion(
        &self,
        operation: Uuid,
    ) -> FileResult<Option<CompletionRecoverySnapshot>> {
        self.read_completion_named(Path::new(&name(operation)?), operation)
    }

    /// Publishes immutable operational evidence, without granting domain mutation authority.
    pub(crate) fn publish_completion(
        &self,
        completion: &ActionCompletion,
    ) -> FileResult<CompletionRecoverySnapshot> {
        let bytes = encode(completion)?;
        let operation = completion.request.operation_id;
        let target = PathBuf::from(name(operation)?);
        if let Some(current) = self.read_completion(operation)? {
            if current.completion != *completion {
                return Err(failure(
                    FileErrorCode::Conflict,
                    "completion recovery operation payload differs",
                ));
            }
            // A retry must repair a possible earlier post-install durability failure.
            let file = open_at(&self.directory, target.as_os_str(), libc::O_RDONLY, 0)?;
            let metadata = file.metadata().map_err(note_io_failure)?;
            check_file(&metadata)?;
            self.path_matches(&target, &metadata)?;
            if metadata.dev() != current.proof.identity.device
                || metadata.ino() != current.proof.identity.inode
            {
                return Err(failure(
                    FileErrorCode::Conflict,
                    "completion recovery file changed before sync",
                ));
            }
            step("file_sync")?;
            full_sync(&file)?;
            step("directory_sync")?;
            sync_directory(&self.directory)?;
            self.validate_directory()?;
            if self.read_completion(operation)? != Some(current.clone()) {
                return Err(failure(
                    FileErrorCode::Conflict,
                    "completion recovery retry proof changed",
                ));
            }
            return Ok(current);
        }
        self.validate_directory()?;
        let temporary = PathBuf::from(format!("{TEMP_PREFIX}{}{TEMP_SUFFIX}", Uuid::new_v4()));
        let mut file = open_at(
            &self.directory,
            temporary.as_os_str(),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )?;
        check_file(&file.metadata().map_err(note_io_failure)?)?;
        step("write")?;
        file.write_all(&bytes).map_err(note_io_failure)?;
        step("file_sync")?;
        full_sync(&file)?;
        step("prepare_directory_sync")?;
        sync_directory(&self.directory)?;
        let prepared = self
            .read_completion_named(&temporary, operation)?
            .ok_or_else(|| {
                failure(
                    FileErrorCode::Conflict,
                    "completion recovery staging disappeared",
                )
            })?;
        let metadata = file.metadata().map_err(note_io_failure)?;
        check_file(&metadata)?;
        self.path_matches(&temporary, &metadata)?;
        if prepared.completion != *completion
            || prepared.proof.sha256 != Some(digest(&bytes))
            || prepared.proof.identity.device != metadata.dev()
            || prepared.proof.identity.inode != metadata.ino()
        {
            return Err(failure(
                FileErrorCode::Conflict,
                "completion recovery staging changed",
            ));
        }
        if self.path_metadata(&target)?.is_some() {
            return Err(failure(
                FileErrorCode::Conflict,
                "completion recovery destination appeared",
            ));
        }
        self.validate_directory()?;
        before_rename();
        // From the rename boundary onward, no failure establishes non-publication.
        (|| {
            step("rename")?;
            self.validate_directory()?;
            if self.read_completion_named(&temporary, operation)? != Some(prepared.clone()) {
                return Err(failure(
                    FileErrorCode::Conflict,
                    "completion recovery staging proof changed before install",
                ));
            }
            rename_flags(
                &self.directory,
                &cstring(&temporary)?,
                &cstring(&target)?,
                libc::RENAME_EXCL,
            )?;
            step("directory_sync")?;
            sync_directory(&self.directory)?;
            step("postproof")?;
            let installed = self.read_completion(operation)?.ok_or_else(|| {
                failure(
                    FileErrorCode::Conflict,
                    "completion recovery install disappeared",
                )
            })?;
            if installed.completion != *completion
                || installed.proof != relocated(&prepared.proof, &target)
                || self.path_metadata(&temporary)?.is_some()
            {
                return Err(failure(
                    FileErrorCode::Conflict,
                    "completion recovery installed proof differs",
                ));
            }
            self.validate_directory()?;
            if self.read_completion(operation)? != Some(installed.clone()) {
                return Err(failure(
                    FileErrorCode::Conflict,
                    "completion recovery installed proof changed",
                ));
            }
            Ok(installed)
        })()
        .map_err(uncertain)
    }
}

#[cfg(not(target_os = "macos"))]
impl ApplyRecoveryFiles {
    pub(crate) fn completion_ids(&self) -> FileResult<Vec<Uuid>> {
        Err(invalid(
            "completion recovery requires native file coordination",
        ))
    }
    pub(crate) fn read_completion(
        &self,
        _: Uuid,
    ) -> FileResult<Option<CompletionRecoverySnapshot>> {
        Err(invalid(
            "completion recovery requires native file coordination",
        ))
    }
    pub(crate) fn publish_completion(
        &self,
        _: &ActionCompletion,
    ) -> FileResult<CompletionRecoverySnapshot> {
        Err(invalid(
            "completion recovery requires native file coordination",
        ))
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::files::{FileErrorCode, FileOutcome};
    use brn_store::{
        WorkStore,
        work::{
            action_completion::CompleteActionRequest,
            actions::{ActionData, ActionPriority, ActionState},
            proposal_apply::{ApplyOutcome, ApprovalRequest},
            proposals::{ActionChange, ProposalDraft},
        },
    };
    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
    };

    struct Fixture {
        _base: tempfile::TempDir,
        data: std::path::PathBuf,
        files: super::super::ApplyRecoveryFiles,
        completion: brn_store::work::action_completion::ActionCompletion,
    }
    impl Fixture {
        fn new() -> Self {
            let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            let data = base.path().join("data");
            fs::create_dir(&data).unwrap();
            let (mut store, _) = WorkStore::open(&data).unwrap();
            let id = Uuid::new_v4();
            let review = store
                .create_proposal(&ProposalDraft {
                    inbox_visual: None,
                    inbox_knowledge: None,
                    inbox_source: None,
                    id: Uuid::new_v4(),
                    group_id: None,
                    session_id: None,
                    vault: None,
                    title: "Approved action".into(),
                    changes: vec![],
                    sources: vec![],
                    action_changes: vec![ActionChange::Create {
                        id,
                        data: ActionData {
                            title: "Exact λ\r\n".into(),
                            description: "日本語 Õun\r\n".into(),
                            state: ActionState::Waiting,
                            owner: Some("Anna Õun".into()),
                            related_person: Some(Uuid::new_v4()),
                            related_project: Some(Uuid::new_v4()),
                            sources: vec![Uuid::new_v4()],
                            thread: Some(Uuid::new_v4()),
                            due_on: Some("2028-02-29".into()),
                            follow_up_on: Some("2028-03-01".into()),
                            dependencies: vec![Uuid::new_v4()],
                            parent: Some(Uuid::new_v4()),
                            follows_up: Some(Uuid::new_v4()),
                            priority: Some(ActionPriority::High),
                        },
                    }],
                })
                .unwrap();
            let op = Uuid::new_v4();
            store
                .begin_proposal_apply(&ApprovalRequest {
                    operation_id: op,
                    expected: review.stamp(),
                })
                .unwrap();
            store.record_proposal_prepared(op, &[]).unwrap();
            store
                .finish_proposal_apply(op, ApplyOutcome::Applied, Some(&[]))
                .unwrap();
            let request = CompleteActionRequest {
                operation_id: Uuid::new_v4(),
                before: Box::new(store.action(id).unwrap().unwrap()),
            };
            let completion = store.complete_action_with(&request, 0, |_| Ok(())).unwrap();
            drop(store);
            let files = super::super::ApplyRecoveryFiles::open(&data).unwrap();
            Self {
                _base: base,
                data,
                files,
                completion,
            }
        }
        fn path(&self) -> std::path::PathBuf {
            self.data
                .join(name(self.completion.request.operation_id).unwrap())
        }
        fn reset(&self) {
            super::super::FAILURE.with(|f| f.set(None));
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.reset();
            super::super::BEFORE_RENAME.with(|h| *h.borrow_mut() = None);
        }
    }

    #[test]
    fn exact_roundtrip_and_retry_preserve_proof_and_all_bytes() {
        let f = Fixture::new();
        let before: Vec<_> = fs::read_dir(&f.data)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(
            f.files
                .read_completion(f.completion.request.operation_id)
                .unwrap(),
            None
        );
        let proof = f.files.publish_completion(&f.completion).unwrap();
        assert_eq!(proof.completion, f.completion);
        assert_eq!(
            fs::metadata(f.path()).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let bytes = fs::read(f.path()).unwrap();
        assert_eq!(f.files.publish_completion(&f.completion).unwrap(), proof);
        assert_eq!(
            f.files
                .read_completion(f.completion.request.operation_id)
                .unwrap(),
            Some(proof)
        );
        assert_eq!(
            f.files.completion_ids().unwrap(),
            vec![f.completion.request.operation_id]
        );
        assert_eq!(fs::read(f.path()).unwrap(), bytes);
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), before.len() + 1);
        assert!(f.files.ids().unwrap().is_empty());
    }
    #[test]
    fn same_operation_changed_payload_never_overwrites() {
        let f = Fixture::new();
        let proof = f.files.publish_completion(&f.completion).unwrap();
        let bytes = fs::read(f.path()).unwrap();
        let mut changed = f.completion.clone();
        changed.after.updated_at_ms += 1;
        changed.after.completed_at_ms = Some(changed.after.updated_at_ms);
        changed.validate().unwrap();
        assert_eq!(
            f.files.publish_completion(&changed).unwrap_err().code,
            FileErrorCode::Conflict
        );
        assert_eq!(fs::read(f.path()).unwrap(), bytes);
        assert_eq!(
            f.files
                .read_completion(f.completion.request.operation_id)
                .unwrap(),
            Some(proof)
        );
        let mut invalid = f.completion.clone();
        invalid.request.operation_id = Uuid::nil();
        let count = fs::read_dir(&f.data).unwrap().count();
        assert!(f.files.publish_completion(&invalid).is_err());
        assert_eq!(fs::read_dir(&f.data).unwrap().count(), count);
        let mut other = f.completion.clone();
        other.request.operation_id = Uuid::new_v4();
        f.files.publish_completion(&other).unwrap();
        assert_eq!(f.files.completion_ids().unwrap().len(), 2);
    }
    #[test]
    fn malformed_envelopes_and_semantics_refuse_without_effects() {
        for case in [
            "hash", "format", "extra", "nested", "binding", "semantic", "oversize", "utf8",
        ] {
            let f = Fixture::new();
            let mut value: serde_json::Value =
                serde_json::from_slice(&encode(&f.completion).unwrap()).unwrap();
            match case {
                "hash" => {
                    value["sha256"][0] = serde_json::json!(value["sha256"][0].as_u64().unwrap() ^ 1)
                }
                "format" => value["format"] = serde_json::json!(2),
                "extra" => value["extra"] = serde_json::json!(true),
                "nested" => value["completion"]["request"]["extra"] = serde_json::json!(true),
                "binding" => {
                    value["completion"]["request"]["operation_id"] =
                        serde_json::json!(Uuid::new_v4())
                }
                "semantic" => {
                    let mut changed = f.completion.clone();
                    changed.after.data.title = "fork".into();
                    value["completion"] = serde_json::to_value(&changed).unwrap();
                    value["sha256"] = serde_json::json!(super::super::digest(
                        &serde_json::to_vec(&changed).unwrap()
                    ));
                }
                _ => {}
            }
            let bytes = match case {
                "oversize" => vec![b' '; MAX_RECORD_BYTES + 1],
                "utf8" => vec![255],
                _ => serde_json::to_vec(&value).unwrap(),
            };
            fs::write(f.path(), &bytes).unwrap();
            assert!(
                f.files
                    .read_completion(f.completion.request.operation_id)
                    .is_err(),
                "{case}"
            );
            assert!(f.files.publish_completion(&f.completion).is_err(), "{case}");
            assert_eq!(fs::read(f.path()).unwrap(), bytes);
        }
    }
    #[test]
    fn enumeration_binds_canonical_names_and_ignores_valid_temporaries() {
        let f = Fixture::new();
        fs::write(
            f.data
                .join(format!("{TEMP_PREFIX}{}{TEMP_SUFFIX}", Uuid::new_v4())),
            b"unfinished",
        )
        .unwrap();
        fs::write(f.data.join("unrelated"), b"leave").unwrap();
        assert!(f.files.completion_ids().unwrap().is_empty());
        for raw in [
            Uuid::new_v4().to_string().to_uppercase(),
            Uuid::nil().to_string(),
            "bad".into(),
        ] {
            let path = f.data.join(format!("{PREFIX}{raw}{SUFFIX}"));
            fs::write(&path, b"foreign").unwrap();
            assert!(f.files.completion_ids().is_err());
            fs::remove_file(path).unwrap();
        }
        let path = f.data.join(format!("{TEMP_PREFIX}bad{TEMP_SUFFIX}"));
        fs::write(path, b"foreign").unwrap();
        assert!(f.files.completion_ids().is_err());
        assert!(f.files.read_completion(Uuid::nil()).is_err());
    }
    #[test]
    fn foreign_occupants_and_replaced_directory_are_preserved() {
        for case in ["symlink", "hardlink", "directory"] {
            let f = Fixture::new();
            let foreign = f.data.join("foreign");
            fs::write(&foreign, b"do not change").unwrap();
            match case {
                "symlink" => symlink(&foreign, f.path()).unwrap(),
                "hardlink" => fs::hard_link(&foreign, f.path()).unwrap(),
                _ => fs::create_dir(f.path()).unwrap(),
            }
            assert!(f.files.publish_completion(&f.completion).is_err());
            assert_eq!(fs::read(foreign).unwrap(), b"do not change");
            assert!(fs::symlink_metadata(f.path()).is_ok());
        }
        let f = Fixture::new();
        let moved = f.data.with_extension("old");
        fs::rename(&f.data, &moved).unwrap();
        fs::create_dir(&f.data).unwrap();
        fs::write(f.path(), b"new directory foreign").unwrap();
        assert!(f.files.publish_completion(&f.completion).is_err());
        assert_eq!(fs::read(f.path()).unwrap(), b"new directory foreign");
        assert!(
            fs::read_dir(moved).unwrap().all(|e| !e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(PREFIX))
        );
    }
    #[test]
    fn publication_faults_classify_boundary_and_retry_reestablishes_durability() {
        for phase in [
            "write",
            "file_sync",
            "prepare_directory_sync",
            "rename",
            "directory_sync",
            "postproof",
        ] {
            let f = Fixture::new();
            super::super::FAILURE.with(|v| v.set(Some(phase)));
            let error = f.files.publish_completion(&f.completion).unwrap_err();
            f.reset();
            assert_eq!(
                error.filesystem_outcome,
                if ["rename", "directory_sync", "postproof"].contains(&phase) {
                    FileOutcome::Unknown
                } else {
                    FileOutcome::NotApplied
                },
                "{phase}"
            );
            if ["directory_sync", "postproof"].contains(&phase) {
                assert!(f.path().exists());
            } else {
                assert!(!f.path().exists());
            }
            let proof = f.files.publish_completion(&f.completion).unwrap();
            super::super::FAILURE.with(|v| v.set(Some("file_sync")));
            assert!(f.files.publish_completion(&f.completion).is_err());
            f.reset();
            assert_eq!(f.files.publish_completion(&f.completion).unwrap(), proof);
        }
    }
    #[test]
    fn replaced_stage_is_never_installed_or_deleted() {
        let f = Fixture::new();
        let data = f.data.clone();
        super::super::BEFORE_RENAME.with(|h| {
            *h.borrow_mut() = Some(Box::new(move || {
                let stage = fs::read_dir(data)
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .find(|p| {
                        p.file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with(TEMP_PREFIX)
                    })
                    .unwrap();
                fs::remove_file(&stage).unwrap();
                fs::write(stage, b"foreign stage").unwrap();
            }))
        });
        assert_eq!(
            f.files
                .publish_completion(&f.completion)
                .unwrap_err()
                .filesystem_outcome,
            FileOutcome::Unknown
        );
        assert!(!f.path().exists());
        let stage = fs::read_dir(&f.data)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(TEMP_PREFIX)
            })
            .unwrap();
        assert_eq!(fs::read(stage).unwrap(), b"foreign stage");
    }
    #[test]
    fn rename_race_never_clobbers_foreign_destination_or_swapped_directory() {
        for swap in [false, true] {
            let f = Fixture::new();
            let path = f.path();
            let data = f.data.clone();
            let moved = data.with_extension("old");
            super::super::BEFORE_RENAME.with(|h| {
                *h.borrow_mut() = Some(Box::new(move || {
                    if swap {
                        fs::rename(&data, moved).unwrap();
                        fs::create_dir(&data).unwrap();
                    }
                    fs::write(path, b"foreign at boundary").unwrap();
                }))
            });
            assert_eq!(
                f.files
                    .publish_completion(&f.completion)
                    .unwrap_err()
                    .filesystem_outcome,
                FileOutcome::Unknown
            );
            assert_eq!(fs::read(f.path()).unwrap(), b"foreign at boundary");
        }
    }
}
