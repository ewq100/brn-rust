//! Immutable analysis evidence inside an existing approval operation's family.
//! This companion grants neither file effects nor another provider execution.
use super::{ApplyJournal, ApplyRecoveryFiles, FileResult, invalid};
use brn_store::work::inbox_actions::InboxActionJob;

#[cfg(target_os = "macos")]
use super::{
    ArtifactIdentity, ArtifactKind, FileErrorCode, Path, PathBuf, Read, RetainedArtifact, Write,
    check_file, cstring, digest, failure, full_sync, note_io_failure, open_at, parse_id, relocated,
    rename_flags, step, sync_directory, uncertain, unchanged,
};
#[cfg(target_os = "macos")]
use brn_store::work::{inbox_actions::InboxAnalysisPurpose, proposal_apply::ApprovalRequest};
#[cfg(target_os = "macos")]
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use std::os::unix::fs::MetadataExt;

#[cfg(target_os = "macos")]
const SUFFIX: &str = ".inbox-capture";
#[cfg(target_os = "macos")]
const TEMP_SUFFIX: &str = ".capture-stage";
// Retain the already bounded 1 MiB job plus fixed request/digest metadata.
// The binding digest avoids repeating citations and predecessor paths.
#[cfg(target_os = "macos")]
const MAX_BYTES: usize = 1024 * 1024 + 4096;

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    request: ApprovalRequest,
    binding_sha256: [u8; 32],
    job: InboxActionJob,
}

#[cfg(target_os = "macos")]
impl Capture {
    fn validate(&self) -> FileResult<()> {
        crate::proposal_apply::validate_approval_request(&self.request)
            .map_err(|_| invalid("invalid captured approval request"))?;
        self.job
            .validate()
            .map_err(|_| invalid("invalid approval analysis capture"))?;
        if self.job.capture.purpose != InboxAnalysisPurpose::KnowledgeAndActions {
            return Err(invalid("approval needs a Knowledge analysis capture"));
        }
        Ok(())
    }

    fn check_journal(&self, journal: &ApplyJournal) -> FileResult<()> {
        self.validate()?;
        let binding = journal
            .approved
            .draft
            .inbox_knowledge
            .as_deref()
            .ok_or_else(|| invalid("approval has no Knowledge capture binding"))?;
        binding
            .validate_capture(&self.job)
            .map_err(|_| invalid("approval analysis capture binding differs"))?;
        let bytes = serde_json::to_vec(binding)
            .map_err(|_| invalid("could not encode approval Knowledge binding"))?;
        if self.request != journal.request || self.binding_sha256 != digest(&bytes) {
            return Err(invalid(
                "approval companion belongs to another exact request",
            ));
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: u8,
    sha256: [u8; 32],
    capture: Capture,
}

#[cfg(target_os = "macos")]
fn encode(capture: &Capture) -> FileResult<Vec<u8>> {
    capture.validate()?;
    let payload = serde_json::to_vec(capture)
        .map_err(|_| invalid("could not encode approval analysis capture"))?;
    let bytes = serde_json::to_vec(&Envelope {
        format: 1,
        sha256: digest(&payload),
        capture: capture.clone(),
    })
    .map_err(|_| invalid("could not encode approval analysis companion"))?;
    if bytes.len() > MAX_BYTES {
        return Err(invalid(
            "approval analysis companion exceeds its fixed bound",
        ));
    }
    Ok(bytes)
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    capture: Capture,
    proof: RetainedArtifact,
}

#[cfg(target_os = "macos")]
impl ApplyRecoveryFiles {
    // Canonical companions are checked even without a receipt. Publication can
    // stop before intent mirroring; that valid orphan remains inert and retained.
    pub(super) fn check_capture_name(&self, filename: &str) -> FileResult<bool> {
        if let Some(raw) = filename
            .strip_prefix(super::TEMP_PREFIX)
            .and_then(|rest| rest.strip_suffix(TEMP_SUFFIX))
        {
            parse_id(raw)?;
            return Ok(true);
        }
        let Some(raw) = filename
            .strip_prefix(super::PREFIX)
            .and_then(|rest| rest.strip_suffix(SUFFIX))
        else {
            return Ok(false);
        };
        let id = parse_id(raw)?;
        let snapshot = self
            .read_capture_named(Path::new(filename), id)?
            .ok_or_else(|| invalid("listed approval analysis companion disappeared"))?;
        if let Some(receipt) = self.read(id)? {
            snapshot.capture.check_journal(&receipt.journal)?;
        }
        Ok(true)
    }

    fn read_capture_named(&self, relative: &Path, id: uuid::Uuid) -> FileResult<Option<Snapshot>> {
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
        self.path_matches(relative, &before)?;
        if before.len() > MAX_BYTES as u64 {
            return Err(invalid(
                "approval analysis companion exceeds its fixed bound",
            ));
        }
        let mut bytes = Vec::with_capacity(before.len() as usize);
        Read::by_ref(&mut file)
            .take((MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(note_io_failure)?;
        let after = file.metadata().map_err(note_io_failure)?;
        check_file(&after)?;
        self.path_matches(relative, &after)?;
        if !unchanged(&before, &after) || after.len() != bytes.len() as u64 {
            return Err(failure(
                FileErrorCode::Conflict,
                "approval analysis companion changed",
            ));
        }
        let envelope: Envelope = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("invalid approval analysis companion"))?;
        if envelope.format != 1
            || envelope.capture.request.operation_id != id
            || encode(&envelope.capture)? != bytes
        {
            return Err(invalid(
                "approval analysis companion format/hash/binding differs",
            ));
        }
        self.validate_directory()?;
        Ok(Some(Snapshot {
            capture: envelope.capture,
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

    pub(crate) fn read_inbox_capture(
        &self,
        journal: &ApplyJournal,
    ) -> FileResult<Option<InboxActionJob>> {
        let id = journal.request.operation_id;
        let path = PathBuf::from(format!("{}{id}{SUFFIX}", super::PREFIX));
        let Some(snapshot) = self.read_capture_named(&path, id)? else {
            return Ok(None);
        };
        snapshot.capture.check_journal(journal)?;
        Ok(Some(snapshot.capture.job))
    }

    pub(crate) fn write_inbox_capture(
        &self,
        journal: &ApplyJournal,
        job: &InboxActionJob,
    ) -> FileResult<()> {
        journal
            .validate()
            .map_err(|_| invalid("invalid captured approval journal"))?;
        let capture = Capture {
            request: journal.request.clone(),
            binding_sha256: digest(
                &serde_json::to_vec(
                    journal
                        .approved
                        .draft
                        .inbox_knowledge
                        .as_deref()
                        .ok_or_else(|| invalid("approval has no Knowledge capture binding"))?,
                )
                .map_err(|_| invalid("could not encode approval Knowledge binding"))?,
            ),
            job: job.clone(),
        };
        capture.check_journal(journal)?;
        let bytes = encode(&capture)?;
        let id = journal.request.operation_id;
        let target = PathBuf::from(format!("{}{id}{SUFFIX}", super::PREFIX));
        if let Some(current) = self.read_capture_named(&target, id)? {
            if current.capture != capture {
                return Err(failure(
                    FileErrorCode::Conflict,
                    "approval analysis companion has another payload",
                ));
            }
            let file = open_at(&self.directory, target.as_os_str(), libc::O_RDONLY, 0)?;
            check_file(&file.metadata().map_err(note_io_failure)?)?;
            step("capture_file_sync")?;
            full_sync(&file)?;
            step("capture_directory_sync")?;
            sync_directory(&self.directory)?;
            if self.read_capture_named(&target, id)?.as_ref() != Some(&current) {
                return Err(failure(
                    FileErrorCode::Conflict,
                    "approval analysis companion changed during sync",
                ));
            }
            return Ok(());
        }
        let temporary = PathBuf::from(format!(
            "{}{}{TEMP_SUFFIX}",
            super::TEMP_PREFIX,
            uuid::Uuid::new_v4()
        ));
        let mut file = open_at(
            &self.directory,
            temporary.as_os_str(),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )?;
        check_file(&file.metadata().map_err(note_io_failure)?)?;
        step("capture_write")?;
        file.write_all(&bytes).map_err(note_io_failure)?;
        step("capture_file_sync")?;
        full_sync(&file)?;
        step("capture_prepare_sync")?;
        sync_directory(&self.directory)?;
        let prepared = self
            .read_capture_named(&temporary, id)?
            .ok_or_else(|| invalid("prepared approval analysis companion disappeared"))?;
        if prepared.capture != capture || prepared.proof.sha256 != Some(digest(&bytes)) {
            return Err(failure(
                FileErrorCode::Conflict,
                "prepared approval analysis companion changed",
            ));
        }
        self.validate_directory()?;
        let result = (|| {
            step("capture_rename")?;
            rename_flags(
                &self.directory,
                &cstring(&temporary)?,
                &cstring(&target)?,
                libc::RENAME_EXCL,
            )?;
            step("capture_directory_sync")?;
            sync_directory(&self.directory)?;
            step("capture_postproof")?;
            let installed = self
                .read_capture_named(&target, id)?
                .ok_or_else(|| invalid("installed approval analysis companion disappeared"))?;
            if installed.capture != capture
                || installed.proof != relocated(&prepared.proof, &target)
                || self.path_metadata(&temporary)?.is_some()
            {
                return Err(failure(
                    FileErrorCode::Conflict,
                    "installed approval analysis companion proof changed",
                ));
            }
            self.validate_directory()?;
            Ok(())
        })();
        result.map_err(uncertain)
    }
}

#[cfg(not(target_os = "macos"))]
impl ApplyRecoveryFiles {
    pub(crate) fn read_inbox_capture(
        &self,
        _: &ApplyJournal,
    ) -> FileResult<Option<InboxActionJob>> {
        Err(invalid("approval recovery persistence requires macOS"))
    }
    pub(crate) fn write_inbox_capture(
        &self,
        _: &ApplyJournal,
        _: &InboxActionJob,
    ) -> FileResult<()> {
        Err(invalid("approval recovery persistence requires macOS"))
    }
}
