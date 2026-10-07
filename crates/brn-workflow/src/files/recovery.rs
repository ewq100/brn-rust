//! Ordinary approval recovery receipts. This helper never applies vault changes.
mod completion;
mod inbox_capture;
pub(crate) use completion::CompletionRecoverySnapshot;

use super::{FileFailure, FileResult, note_unsupported};
#[cfg(target_os = "macos")]
use super::{FileOutcome, failure};
use brn_store::{files::RetainedArtifact, work::proposal_apply::ApplyJournal};
#[cfg(target_os = "macos")]
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[cfg(target_os = "macos")]
const PREFIX: &str = ".brn-apply-";
#[cfg(target_os = "macos")]
const SUFFIX: &str = ".receipt";
#[cfg(target_os = "macos")]
const TEMP_PREFIX: &str = ".brn-apply-temp-";
#[cfg(target_os = "macos")]
const TEMP_SUFFIX: &str = ".stage";
#[cfg(target_os = "macos")]
const MAX_RECORD_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecoverySnapshot {
    pub journal: ApplyJournal,
    pub proof: RetainedArtifact,
}

#[cfg(target_os = "macos")]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: u8,
    sha256: [u8; 32],
    journal: ApplyJournal,
}

fn invalid(message: &str) -> FileFailure {
    note_unsupported(message)
}

#[cfg(target_os = "macos")]
fn name(id: Uuid) -> FileResult<String> {
    if id.is_nil() {
        return Err(invalid("approval recovery UUID must not be nil"));
    }
    Ok(format!("{PREFIX}{id}{SUFFIX}"))
}

#[cfg(target_os = "macos")]
fn parse_id(value: &str) -> FileResult<Uuid> {
    let id = Uuid::parse_str(value).map_err(|_| invalid("malformed approval recovery filename"))?;
    if id.is_nil() || id.to_string() != value {
        return Err(invalid("approval recovery filename needs a canonical UUID"));
    }
    Ok(id)
}

#[cfg(target_os = "macos")]
use {
    super::{
        FileErrorCode, cstring, full_sync, identity, note_io_failure, open_at, open_directory,
        rename_flags, sync_directory,
    },
    brn_store::files::{ArtifactIdentity, ArtifactKind, VaultIdentity},
    brn_store::work::{
        proposal_apply::ApplyOutcome,
        proposals::{ActionChange, NoteChange},
    },
    sha2::{Digest, Sha256},
    std::{
        ffi::{CStr, OsStr},
        fs::{File, Metadata},
        io::{Read, Write},
        os::{
            fd::{AsRawFd, IntoRawFd},
            unix::fs::MetadataExt,
        },
        path::{Component, PathBuf},
    },
};

#[cfg(target_os = "macos")]
pub(crate) struct ApplyRecoveryFiles {
    path: PathBuf,
    canonical: PathBuf,
    directory: File,
    identity: VaultIdentity,
}

#[cfg(target_os = "macos")]
fn current_user() -> u32 {
    // SAFETY: geteuid has no preconditions.
    unsafe { libc::geteuid() }
}

#[cfg(target_os = "macos")]
fn checked_directory(path: &Path) -> FileResult<File> {
    if !path.is_absolute() {
        return Err(invalid("approval recovery directory must be absolute"));
    }
    let mut directory = open_directory(Path::new("/"))?;
    for component in path.components().skip(1) {
        let Component::Normal(part) = component else {
            return Err(invalid("invalid approval recovery directory"));
        };
        directory = open_at(&directory, part, libc::O_DIRECTORY, 0)?;
    }
    let metadata = directory.metadata().map_err(note_io_failure)?;
    if !metadata.is_dir() || metadata.uid() != current_user() {
        return Err(invalid(
            "approval recovery directory is not owned by the current user",
        ));
    }
    Ok(directory)
}

#[cfg(target_os = "macos")]
fn check_file(metadata: &Metadata) -> FileResult<()> {
    if !metadata.is_file() || metadata.nlink() != 1 || metadata.uid() != current_user() {
        return Err(invalid(
            "approval recovery needs an owned single-link regular file",
        ));
    }
    if metadata.len() > MAX_RECORD_BYTES as u64 {
        return Err(invalid("approval recovery record exceeds 64 MiB"));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(target_os = "macos")]
fn encode(journal: &ApplyJournal) -> FileResult<Vec<u8>> {
    journal
        .validate()
        .map_err(|_| invalid("invalid approval recovery journal"))?;
    let encoded = serde_json::to_vec(journal)
        .map_err(|_| invalid("could not encode approval recovery journal"))?;
    let envelope = Envelope {
        format: 1,
        sha256: digest(&encoded),
        journal: journal.clone(),
    };
    let bytes = serde_json::to_vec(&envelope)
        .map_err(|_| invalid("could not encode approval recovery record"))?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(invalid("approval recovery record exceeds 64 MiB"));
    }
    Ok(bytes)
}

#[cfg(target_os = "macos")]
fn unchanged(before: &Metadata, after: &Metadata) -> bool {
    identity(before) == identity(after)
        && before.len() == after.len()
        && (
            before.mtime(),
            before.mtime_nsec(),
            before.ctime(),
            before.ctime_nsec(),
        ) == (
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec(),
        )
}

#[cfg(target_os = "macos")]
fn relocated(proof: &RetainedArtifact, relative: &Path) -> RetainedArtifact {
    RetainedArtifact {
        relative: relative.to_owned(),
        ..proof.clone()
    }
}

#[cfg(target_os = "macos")]
fn uncertain(mut error: FileFailure) -> FileFailure {
    error.filesystem_outcome = FileOutcome::Unknown;
    error
}

#[cfg(target_os = "macos")]
fn covered_review(older: &ApplyJournal, applied: &ApplyJournal) -> bool {
    let before = &older.approved;
    let after = &applied.approved;
    before.draft.id == after.draft.id
        && older.creation_sha256 == applied.creation_sha256
        && older.undo == applied.undo
        && before.version <= after.version
        && before.created_at_ms == after.created_at_ms
        && before.draft.vault == after.draft.vault
        && before.draft.group_id == after.draft.group_id
        && before.draft.session_id == after.draft.session_id
        && before.draft.sources == after.draft.sources
        && before.draft.inbox_source == after.draft.inbox_source
        && before.draft.action_changes.len() == after.draft.action_changes.len()
        && before
            .draft
            .action_changes
            .iter()
            .zip(&after.draft.action_changes)
            .all(|(before, after)| match (before, after) {
                (
                    ActionChange::Create { id: before, .. },
                    ActionChange::Create { id: after, .. },
                ) => before == after,
                (
                    ActionChange::Replace { before, .. },
                    ActionChange::Replace { before: after, .. },
                ) => before == after,
                _ => false,
            })
        && before.draft.changes.len() == after.draft.changes.len()
        && before
            .draft
            .changes
            .iter()
            .zip(&after.draft.changes)
            .all(|(before, after)| match (before, after) {
                (
                    NoteChange::Create {
                        path: before_path,
                        parent: before_parent,
                        ..
                    },
                    NoteChange::Create {
                        path: after_path,
                        parent: after_parent,
                        ..
                    },
                ) => before_path == after_path && before_parent == after_parent,
                (
                    NoteChange::Replace {
                        path: before_path,
                        parent: before_parent,
                        before: before_proof,
                        before_text,
                        ..
                    },
                    NoteChange::Replace {
                        path: after_path,
                        parent: after_parent,
                        before: after_proof,
                        before_text: after_text,
                        ..
                    },
                )
                | (
                    NoteChange::Trash {
                        path: before_path,
                        parent: before_parent,
                        before: before_proof,
                        before_text,
                    },
                    NoteChange::Trash {
                        path: after_path,
                        parent: after_parent,
                        before: after_proof,
                        before_text: after_text,
                    },
                ) => {
                    before_path == after_path
                        && before_parent == after_parent
                        && before_proof == after_proof
                        && before_text == after_text
                }
                (NoteChange::CreateAsset { .. }, NoteChange::CreateAsset { .. })
                | (NoteChange::ReplaceAsset { .. }, NoteChange::ReplaceAsset { .. })
                | (NoteChange::TrashAsset { .. }, NoteChange::TrashAsset { .. }) => before == after,
                _ => false,
            })
}

#[cfg(target_os = "macos")]
impl ApplyRecoveryFiles {
    pub(crate) fn open(path: &Path) -> FileResult<Self> {
        let directory = checked_directory(path)?;
        let canonical = path.canonicalize().map_err(note_io_failure)?;
        let identity = identity(&directory.metadata().map_err(note_io_failure)?);
        let files = Self {
            path: path.to_owned(),
            canonical,
            directory,
            identity,
        };
        files.validate_directory()?;
        Ok(files)
    }

    fn validate_directory(&self) -> FileResult<()> {
        let changed = || {
            failure(
                FileErrorCode::Conflict,
                "approval recovery directory changed",
            )
        };
        if self.path.canonicalize().map_err(|_| changed())? != self.canonical {
            return Err(changed());
        }
        let current = checked_directory(&self.path).map_err(|_| changed())?;
        if identity(&current.metadata().map_err(note_io_failure)?) != self.identity
            || identity(&self.directory.metadata().map_err(note_io_failure)?) != self.identity
        {
            return Err(changed());
        }
        Ok(())
    }

    fn enumerate(&self, mut visit: impl FnMut(&[u8]) -> FileResult<()>) -> FileResult<()> {
        self.validate_directory()?;
        // A new description has its own enumeration offset; dup would share it.
        let enumeration = open_at(&self.directory, OsStr::new("."), libc::O_DIRECTORY, 0)?;
        if identity(&enumeration.metadata().map_err(note_io_failure)?) != self.identity {
            return Err(failure(
                FileErrorCode::Conflict,
                "approval recovery directory changed",
            ));
        }
        let fd = enumeration.into_raw_fd();
        // SAFETY: fd is a newly opened directory descriptor.
        let directory = unsafe { libc::fdopendir(fd) };
        if directory.is_null() {
            let error = std::io::Error::last_os_error();
            // SAFETY: fdopendir failed without taking ownership of fd.
            unsafe { libc::close(fd) };
            return Err(note_io_failure(error));
        }
        struct Entries(*mut libc::DIR);
        impl Drop for Entries {
            fn drop(&mut self) {
                // SAFETY: this is the unique owner of the successful fdopendir result.
                unsafe { libc::closedir(self.0) };
            }
        }
        let entries = Entries(directory);
        loop {
            // SAFETY: macOS exposes the calling thread's errno cell.
            unsafe { *libc::__error() = 0 };
            // SAFETY: entries retains the live DIR; the name is consumed before the next call.
            let entry = unsafe { libc::readdir(entries.0) };
            if entry.is_null() {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() != Some(0) {
                    return Err(note_io_failure(error));
                }
                break;
            }
            // SAFETY: readdir returns a NUL-terminated d_name in this live entry.
            let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            visit(bytes)?;
        }
        self.validate_directory()
    }

    /// Lists names through the retained descriptor without loading journal bodies.
    pub(crate) fn ids(&self) -> FileResult<Vec<Uuid>> {
        let mut ids = Vec::new();
        self.enumerate(|bytes| {
            if !bytes.starts_with(PREFIX.as_bytes()) {
                return Ok(());
            }
            let filename = std::str::from_utf8(bytes)
                .map_err(|_| invalid("malformed approval recovery filename"))?;
            if let Some(id) = filename
                .strip_prefix(TEMP_PREFIX)
                .and_then(|rest| rest.strip_suffix(TEMP_SUFFIX))
            {
                parse_id(id)?;
                return Ok(());
            }
            if self.check_capture_name(filename)? {
                return Ok(());
            }
            let id = filename
                .strip_prefix(PREFIX)
                .and_then(|rest| rest.strip_suffix(SUFFIX))
                .ok_or_else(|| invalid("malformed approval recovery filename"))?;
            ids.push(parse_id(id)?);
            Ok(())
        })?;
        ids.sort_unstable();
        Ok(ids)
    }

    fn path_metadata(&self, relative: &Path) -> FileResult<Option<libc::stat>> {
        let filename = cstring(relative)?;
        let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: relative is an internal basename; metadata is initialized on success.
        if unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                filename.as_ptr(),
                metadata.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            let error = std::io::Error::last_os_error();
            return if error.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(note_io_failure(error))
            };
        }
        // SAFETY: successful fstatat initialized metadata.
        Ok(Some(unsafe { metadata.assume_init() }))
    }

    fn path_matches(&self, relative: &Path, metadata: &Metadata) -> FileResult<()> {
        let current = self.path_metadata(relative)?.ok_or_else(|| {
            failure(
                FileErrorCode::Conflict,
                "approval recovery file disappeared",
            )
        })?;
        if current.st_mode & libc::S_IFMT != libc::S_IFREG
            || current.st_nlink != 1
            || current.st_uid != current_user()
            || current.st_dev as u64 != metadata.dev()
            || current.st_ino != metadata.ino()
            || current.st_size as u64 != metadata.len()
        {
            return Err(failure(
                FileErrorCode::Conflict,
                "approval recovery file changed",
            ));
        }
        Ok(())
    }

    fn read_named(&self, relative: &Path, operation: Uuid) -> FileResult<Option<RecoverySnapshot>> {
        self.read_envelope(relative, Some(operation))
    }

    fn read_envelope(
        &self,
        relative: &Path,
        operation: Option<Uuid>,
    ) -> FileResult<Option<RecoverySnapshot>> {
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
        let mut bytes = Vec::with_capacity(before.len() as usize);
        step("read")?;
        Read::by_ref(&mut file)
            .take((MAX_RECORD_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(note_io_failure)?;
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(invalid("approval recovery record exceeds 64 MiB"));
        }
        let after = file.metadata().map_err(note_io_failure)?;
        check_file(&after)?;
        if !unchanged(&before, &after) || after.len() != bytes.len() as u64 {
            return Err(failure(
                FileErrorCode::Conflict,
                "approval recovery changed while reading",
            ));
        }
        self.path_matches(relative, &after)?;
        std::str::from_utf8(&bytes)
            .map_err(|_| invalid("approval recovery record is not UTF-8"))?;
        let envelope: Envelope = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("invalid approval recovery envelope"))?;
        if envelope.format != 1
            || operation.is_some_and(|operation| envelope.journal.request.operation_id != operation)
        {
            return Err(invalid(
                "approval recovery format or filename binding differs",
            ));
        }
        envelope
            .journal
            .validate()
            .map_err(|_| invalid("invalid approval recovery journal"))?;
        let journal = serde_json::to_vec(&envelope.journal)
            .map_err(|_| invalid("could not encode approval recovery journal"))?;
        if envelope.sha256 != digest(&journal) {
            return Err(invalid("approval recovery journal failed its hash check"));
        }
        self.validate_directory()?;
        Ok(Some(RecoverySnapshot {
            journal: envelope.journal,
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

    pub(crate) fn read(&self, id: Uuid) -> FileResult<Option<RecoverySnapshot>> {
        self.read_named(Path::new(&name(id)?), id)
    }

    /// Removes only proven temporary review snapshots covered by this Applied
    /// proposal. Canonical receipts and unproven occupants remain untouched.
    pub(crate) fn retire_review_temporaries(&self, approved: &ApplyJournal) -> FileResult<()> {
        approved
            .validate()
            .map_err(|_| invalid("invalid Applied approval recovery journal"))?;
        if !approved
            .receipt
            .as_ref()
            .is_some_and(|receipt| receipt.outcome == ApplyOutcome::Applied)
        {
            return Err(invalid(
                "temporary review retirement requires an Applied receipt",
            ));
        }
        let mut names = Vec::new();
        self.enumerate(|bytes| {
            if let Ok(filename) = std::str::from_utf8(bytes)
                && let Some(id) = filename
                    .strip_prefix(TEMP_PREFIX)
                    .and_then(|rest| rest.strip_suffix(TEMP_SUFFIX))
                && parse_id(id).is_ok()
            {
                names.push(PathBuf::from(filename));
            }
            Ok(())
        })?;
        for relative in names {
            let snapshot = match self.read_envelope(&relative, None) {
                Ok(Some(snapshot)) => snapshot,
                Err(error) if error.code == FileErrorCode::Io => return Err(error),
                Ok(None) | Err(_) => {
                    // Malformed, raced or unrelated occupants do not grant
                    // permission to unlink. A replaced directory still fences
                    // completion rather than being mistaken for an occupant.
                    self.validate_directory()?;
                    continue;
                }
            };
            let compatible_repair = if snapshot.journal.repair.is_some() {
                self.read(snapshot.journal.request.operation_id)?
                    .is_some_and(|canonical| {
                        canonical.journal.repair_history_covers(&snapshot.journal)
                    })
            } else {
                true
            };
            if compatible_repair && covered_review(&snapshot.journal, approved) {
                before_retire();
                self.remove_known(&snapshot).map_err(uncertain)?;
                step("cleanup_directory_sync").map_err(uncertain)?;
                sync_directory(&self.directory).map_err(uncertain)?;
                self.validate_directory().map_err(uncertain)?;
            }
        }
        // A retry after an earlier unlink/sync failure must establish the
        // deletion's durability even when that temporary name is now absent.
        step("cleanup_directory_sync").map_err(uncertain)?;
        sync_directory(&self.directory).map_err(uncertain)?;
        self.validate_directory().map_err(uncertain)
    }

    fn check_expected(
        &self,
        operation: Uuid,
        expected: Option<&RecoverySnapshot>,
    ) -> FileResult<Option<RecoverySnapshot>> {
        let current = self.read(operation)?;
        match (expected, current) {
            (None, None) => Ok(None),
            (Some(expected), Some(current)) if *expected == current => Ok(Some(current)),
            _ => Err(failure(
                FileErrorCode::Conflict,
                "approval recovery replacement token changed",
            )),
        }
    }

    pub(crate) fn write(
        &self,
        journal: &ApplyJournal,
        expected: Option<&RecoverySnapshot>,
    ) -> FileResult<RecoverySnapshot> {
        let bytes = encode(journal)?;
        let operation = journal.request.operation_id;
        let target = PathBuf::from(name(operation)?);
        if let Some(expected) = expected
            && (expected.proof.relative != target
                || expected.journal.request.operation_id != operation)
        {
            return Err(failure(
                FileErrorCode::Conflict,
                "approval recovery token belongs to another filename",
            ));
        }
        let current = self.check_expected(operation, expected)?;
        if let Some(current) = current
            && current.journal == *journal
        {
            // The caller may be recovering an earlier post-rename sync
            // failure. An identical record must re-establish durability before
            // it can authorize SQLite completion after restart.
            let file = open_at(&self.directory, target.as_os_str(), 0, 0)?;
            check_file(&file.metadata().map_err(note_io_failure)?)?;
            step("file_sync")?;
            full_sync(&file)?;
            step("directory_sync")?;
            sync_directory(&self.directory)?;
            self.validate_directory()?;
            self.check_expected(operation, Some(&current))?;
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
        let prepared = self.read_named(&temporary, operation)?.ok_or_else(|| {
            failure(
                FileErrorCode::Conflict,
                "approval recovery staging disappeared",
            )
        })?;
        if prepared.journal != *journal || prepared.proof.sha256 != Some(digest(&bytes)) {
            return Err(failure(
                FileErrorCode::Conflict,
                "approval recovery staging changed",
            ));
        }
        self.check_expected(operation, expected)?;
        self.validate_directory()?;
        before_rename();
        // From this boundary onward any failure is conservatively uncertain.
        let result = (|| {
            step("rename")?;
            rename_flags(
                &self.directory,
                &cstring(&temporary)?,
                &cstring(&target)?,
                if expected.is_some() {
                    libc::RENAME_SWAP
                } else {
                    libc::RENAME_EXCL
                },
            )?;
            step("directory_sync")?;
            sync_directory(&self.directory)?;
            step("postproof")?;
            let installed = self.read(operation)?.ok_or_else(|| {
                failure(
                    FileErrorCode::Conflict,
                    "installed approval recovery record disappeared",
                )
            })?;
            if installed.journal != *journal
                || installed.proof != relocated(&prepared.proof, &target)
            {
                return Err(failure(
                    FileErrorCode::Conflict,
                    "installed approval recovery proof changed",
                ));
            }
            if let Some(expected) = expected {
                let displaced = self.read_named(&temporary, operation)?.ok_or_else(|| {
                    failure(
                        FileErrorCode::Conflict,
                        "displaced approval recovery record disappeared",
                    )
                })?;
                if displaced.journal != expected.journal
                    || displaced.proof != relocated(&expected.proof, &temporary)
                {
                    return Err(failure(
                        FileErrorCode::Conflict,
                        "unexpected displaced approval recovery record retained",
                    ));
                }
                self.remove_known(&displaced)?;
                step("cleanup_directory_sync")?;
                sync_directory(&self.directory)?;
            } else if self.path_metadata(&temporary)?.is_some() {
                return Err(failure(
                    FileErrorCode::Conflict,
                    "unexpected approval recovery staging occupant retained",
                ));
            }
            self.validate_directory()?;
            if self.read(operation)?.as_ref() != Some(&installed) {
                return Err(failure(
                    FileErrorCode::Conflict,
                    "approval recovery record changed after cleanup",
                ));
            }
            Ok(installed)
        })();
        result.map_err(uncertain)
    }

    fn remove_known(&self, snapshot: &RecoverySnapshot) -> FileResult<()> {
        if self
            .read_named(
                &snapshot.proof.relative,
                snapshot.journal.request.operation_id,
            )?
            .as_ref()
            != Some(snapshot)
        {
            return Err(failure(
                FileErrorCode::Conflict,
                "approval recovery cleanup occupant changed",
            ));
        }
        self.validate_directory()?;
        step("cleanup")?;
        let filename = cstring(&snapshot.proof.relative)?;
        // SAFETY: an internal basename was just validated against its exact owned file proof.
        if unsafe { libc::unlinkat(self.directory.as_raw_fd(), filename.as_ptr(), 0) } != 0 {
            return Err(note_io_failure(std::io::Error::last_os_error()));
        }
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use brn_store::{
        WorkStore,
        files::{FileFingerprint, VaultIdentity, VaultRecord},
        work::{
            actions::{ActionData, ActionRecord, ActionState},
            proposal_apply::{ApplyMemberProof, ApplyReceipt, ApprovalRequest},
            proposals::{
                ActionChange, CommentRequest, CommentTarget, NoteChange, ProposalDraft,
                ProposalEdit, ReviewComment, SourceVersion,
            },
        },
    };
    use std::fs;

    struct Fixture {
        _base: tempfile::TempDir,
        data: PathBuf,
        files: ApplyRecoveryFiles,
        journal: ApplyJournal,
    }

    impl Fixture {
        fn new() -> Self {
            let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            let data = base.path().join("data");
            fs::create_dir(&data).unwrap();
            let vault = base.path().join("vault");
            fs::create_dir(&vault).unwrap();
            let (mut store, _) = WorkStore::open(&data).unwrap();
            let parent = VaultIdentity {
                device: 1,
                inode: 1,
            };
            let before = "\u{feff}旧い 🦀\r\n";
            let record = store
                .create_proposal(&ProposalDraft {
                    intake: None,
                    inbox_visual: None,
                    inbox_knowledge: None,
                    inbox_source: None,
                    action_changes: Vec::new(),
                    id: Uuid::new_v4(),
                    group_id: None,
                    session_id: None,
                    vault: Some(VaultRecord {
                        id: Uuid::new_v4(),
                        root: vault,
                        identity: parent.clone(),
                    }),
                    title: "Exact synthetic approval recovery".into(),
                    changes: vec![
                        NoteChange::Create {
                            path: "new.md".into(),
                            parent: parent.clone(),
                            text: "\u{feff}日本語 🦀\r\n".into(),
                        },
                        NoteChange::Replace {
                            path: "existing.md".into(),
                            parent,
                            before: FileFingerprint {
                                device: 1,
                                inode: 2,
                                len: before.len() as u64,
                                sha256: digest(before.as_bytes()),
                            },
                            before_text: before.into(),
                            text: "\u{feff}новое\r\n".into(),
                        },
                    ],
                    sources: Vec::new(),
                })
                .unwrap();
            let journal = store
                .begin_proposal_apply(&ApprovalRequest {
                    operation_id: Uuid::new_v4(),
                    expected: record.stamp(),
                })
                .unwrap();
            drop(store);
            let files = ApplyRecoveryFiles::open(&data).unwrap();
            Self {
                _base: base,
                data,
                files,
                journal,
            }
        }

        fn path(&self) -> PathBuf {
            self.data
                .join(name(self.journal.request.operation_id).unwrap())
        }
        fn changed(&self) -> ApplyJournal {
            let mut changed = self.journal.clone();
            changed.started_at_ms += 1;
            changed
        }
        fn temps(&self) -> Vec<PathBuf> {
            fs::read_dir(&self.data)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with(TEMP_PREFIX)
                })
                .collect()
        }

        fn commented(&self) -> ApplyJournal {
            let mut journal = self.journal.clone();
            journal.approved.version += 1;
            journal.approved.updated_at_ms += 1;
            journal.started_at_ms = journal.approved.updated_at_ms + 1;
            journal.request.expected = journal.approved.stamp();
            journal.approved.comments.push(ReviewComment {
                id: Uuid::new_v4(),
                text: "Temporary review annotation 日本語\r\n".into(),
                target: CommentTarget::Proposal,
            });
            journal.validate().unwrap();
            journal
        }

        fn temporary(&self, bytes: &[u8]) -> PathBuf {
            let path = self
                .data
                .join(format!("{TEMP_PREFIX}{}{TEMP_SUFFIX}", Uuid::new_v4()));
            fs::write(&path, bytes).unwrap();
            path
        }
    }

    fn applied(journal: &ApplyJournal) -> ApplyJournal {
        let mut journal = journal.clone();
        journal.approved.comments.clear();
        let prepared: Vec<_> = journal
            .approved
            .draft
            .changes
            .iter()
            .enumerate()
            .map(|(index, change)| {
                if let Some(original) = journal
                    .undo
                    .as_ref()
                    .and_then(|binding| binding.originals[index].as_ref())
                {
                    return original.fingerprint.clone();
                }
                match change {
                    NoteChange::Create { text, .. } | NoteChange::Replace { text, .. } => {
                        FileFingerprint {
                            device: 1,
                            inode: 100 + index as u64,
                            len: text.len() as u64,
                            sha256: digest(text.as_bytes()),
                        }
                    }
                    NoteChange::Trash { before, .. } => before.clone(),
                    NoteChange::CreateAsset { .. }
                    | NoteChange::ReplaceAsset { .. }
                    | NoteChange::TrashAsset { .. } => {
                        unreachable!("Markdown-only historical fixture")
                    }
                }
            })
            .collect();
        journal.observations = Some(
            journal
                .approved
                .draft
                .changes
                .iter()
                .zip(&prepared)
                .map(|(change, prepared)| match change {
                    NoteChange::Create { .. } => ApplyMemberProof {
                        destination: Some(prepared.clone()),
                        staging: None,
                    },
                    NoteChange::Replace { before, .. } => ApplyMemberProof {
                        destination: Some(prepared.clone()),
                        staging: Some(before.clone()),
                    },
                    NoteChange::Trash { before, .. } => ApplyMemberProof {
                        destination: None,
                        staging: Some(before.clone()),
                    },
                    NoteChange::CreateAsset { .. }
                    | NoteChange::ReplaceAsset { .. }
                    | NoteChange::TrashAsset { .. } => {
                        unreachable!("Markdown-only historical fixture")
                    }
                })
                .collect(),
        );
        journal.prepared = Some(prepared);
        journal.receipt = Some(ApplyReceipt {
            operation_id: journal.request.operation_id,
            proposal_id: journal.approved.draft.id,
            approved_version: journal.approved.version,
            stamp: brn_store::work::proposals::ProposalStamp {
                id: journal.approved.draft.id,
                version: journal.approved.version + 2,
            },
            outcome: ApplyOutcome::Applied,
        });
        if let Some(binding) = &mut journal.repair {
            binding.attempts.last_mut().unwrap().outcome = Some(ApplyOutcome::Applied);
        }
        journal.validate().unwrap();
        journal
    }

    fn action_data(state: ActionState) -> ActionData {
        ActionData {
            title: "Täpne tegevus λ\r\n".into(),
            description: "Exact English ja eesti\r\n".into(),
            state,
            owner: Some("Anna Õun".into()),
            related_person: None,
            related_project: None,
            sources: Vec::new(),
            thread: None,
            due_on: None,
            follow_up_on: None,
            dependencies: Vec::new(),
            parent: None,
            follows_up: None,
            priority: None,
        }
    }

    // Build real checked Store baselines and a reviewed Create/Replace snapshot.
    // The helper exercises only synthetic Store work and ordinary metadata files.
    fn action_review(fixture: &Fixture, mixed: bool) -> ApplyJournal {
        let (mut store, _) = WorkStore::open(&fixture.data).unwrap();
        store
            .refuse_proposal_before_effects(fixture.journal.request.operation_id, None)
            .unwrap();
        let ids = [Uuid::new_v4(), Uuid::new_v4()];
        let initial = ProposalDraft {
            intake: None,
            inbox_visual: None,
            inbox_knowledge: None,
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            vault: None,
            title: "Retained Action baselines".into(),
            changes: Vec::new(),
            sources: Vec::new(),
            action_changes: ids
                .iter()
                .map(|id| ActionChange::Create {
                    id: *id,
                    data: action_data(ActionState::Open),
                })
                .collect(),
        };
        let initial = store.create_proposal(&initial).unwrap();
        let initial = store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: initial.stamp(),
            })
            .unwrap();
        store
            .record_proposal_prepared(initial.request.operation_id, &[])
            .unwrap();
        store
            .finish_proposal_apply(
                initial.request.operation_id,
                ApplyOutcome::Applied,
                Some(&[]),
            )
            .unwrap();
        let mut updated = initial.approved.draft.clone();
        updated.id = Uuid::new_v4();
        updated.action_changes = ids
            .iter()
            .map(|id| ActionChange::Replace {
                before: Box::new(store.action(*id).unwrap().unwrap()),
                data: action_data(ActionState::Waiting),
            })
            .collect();
        let updated = store.create_proposal(&updated).unwrap();
        let updated = store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: updated.stamp(),
            })
            .unwrap();
        store
            .record_proposal_prepared(updated.request.operation_id, &[])
            .unwrap();
        store
            .finish_proposal_apply(
                updated.request.operation_id,
                ApplyOutcome::Applied,
                Some(&[]),
            )
            .unwrap();
        let mut draft = fixture.journal.approved.draft.clone();
        draft.id = Uuid::new_v4();
        if !mixed {
            draft.vault = None;
            draft.changes.clear();
        }
        draft.action_changes = vec![ActionChange::Create {
            id: Uuid::new_v4(),
            data: action_data(ActionState::Open),
        }];
        draft
            .action_changes
            .extend(ids.iter().map(|id| ActionChange::Replace {
                before: Box::new(store.action(*id).unwrap().unwrap()),
                data: action_data(ActionState::Blocked),
            }));
        let record = store.create_proposal(&draft).unwrap();
        let record = store
            .add_proposal_comment(&CommentRequest {
                expected: record.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "Temporary Action review\r\n".into(),
                    target: CommentTarget::Proposal,
                },
            })
            .unwrap();
        store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: record.stamp(),
            })
            .unwrap()
    }

    #[test]
    fn action_retirement_preserves_foreign_immutable_bindings_in_real_envelopes() {
        for mixed in [false, true] {
            let fixture = Fixture::new();
            let old = action_review(&fixture, mixed);
            let terminal = applied(&old);
            let canonical = fixture.files.write(&terminal, None).unwrap();
            let compatible = fixture.temporary(&encode(&old).unwrap());
            let mut preserved = Vec::new();
            for binding in 0..8 {
                let mut foreign = old.clone();
                match binding {
                    0 => {
                        let id = Uuid::new_v4();
                        if let ActionChange::Create { id: target, .. } =
                            &mut foreign.approved.draft.action_changes[0]
                        {
                            *target = id;
                        }
                        foreign.action_records[0].origin.id = id;
                    }
                    1 => {
                        let id = Uuid::new_v4();
                        if let ActionChange::Replace { before, .. } =
                            &mut foreign.approved.draft.action_changes[1]
                        {
                            before.origin.id = id;
                        }
                        foreign.action_records[1].origin.id = id;
                    }
                    2 => {
                        if let ActionChange::Replace { before, .. } =
                            &mut foreign.approved.draft.action_changes[1]
                        {
                            before.data.description.push_str("Different exact baseline");
                        }
                    }
                    3 => {
                        let proposal = Uuid::new_v4();
                        if let ActionChange::Replace { before, .. } =
                            &mut foreign.approved.draft.action_changes[1]
                        {
                            before.origin.proposal.id = proposal;
                        }
                        foreign.action_records[1].origin.proposal.id = proposal;
                    }
                    4 => {
                        if let ActionChange::Replace { before, .. } =
                            &mut foreign.approved.draft.action_changes[1]
                        {
                            before.version += 1;
                        }
                        foreign.action_records[1].version += 1;
                    }
                    5 => {
                        foreign.approved.draft.action_changes.pop();
                        foreign.action_records.pop();
                    }
                    6 => {
                        foreign.approved.draft.action_changes.swap(1, 2);
                        foreign.action_records.swap(1, 2);
                    }
                    7 => {
                        let mut before = match &foreign.approved.draft.action_changes[1] {
                            ActionChange::Replace { before, .. } => before.clone(),
                            _ => unreachable!(),
                        };
                        before.origin.id = foreign.approved.draft.action_changes[0].id();
                        let data = foreign.approved.draft.action_changes[0].data().clone();
                        foreign.action_records[0] = ActionRecord {
                            origin: before.origin.clone(),
                            version: before.version + 1,
                            data: data.clone(),
                            updated_at_ms: foreign.started_at_ms,
                            waiting_since_ms: None,
                            completed_at_ms: None,
                        };
                        foreign.approved.draft.action_changes[0] =
                            ActionChange::Replace { before, data };
                    }
                    _ => unreachable!(),
                }
                foreign.validate().unwrap();
                let bytes = encode(&foreign).unwrap();
                let path = fixture.temporary(&bytes);
                preserved.push((binding, path, bytes));
            }
            fixture.files.retire_review_temporaries(&terminal).unwrap();
            let removed: Vec<_> = preserved
                .iter()
                .filter(|(_, path, _)| !path.exists())
                .map(|(binding, _, _)| *binding)
                .collect();
            assert!(
                removed.is_empty(),
                "mixed={mixed}: foreign Action bindings retired: {removed:?}"
            );
            for (_, path, bytes) in preserved {
                assert_eq!(fs::read(path).unwrap(), bytes);
            }
            assert!(!compatible.exists());
            assert_eq!(
                fixture.files.read(terminal.request.operation_id).unwrap(),
                Some(canonical)
            );
        }
    }

    #[test]
    fn action_retirement_accepts_exact_reviewed_edits_after_failed_staging_and_restart() {
        let _reset = HookReset;
        for mixed in [false, true] {
            let fixture = Fixture::new();
            let old = action_review(&fixture, mixed);
            FAILURE.with(|failure| failure.set(Some("prepare_directory_sync")));
            assert_eq!(
                fixture
                    .files
                    .write(&old, None)
                    .unwrap_err()
                    .filesystem_outcome,
                FileOutcome::NotApplied
            );
            FAILURE.with(|failure| failure.set(None));
            let temporary = fixture.temps().pop().unwrap();
            assert_eq!(fs::read(&temporary).unwrap(), encode(&old).unwrap());
            let (mut store, _) = WorkStore::open(&fixture.data).unwrap();
            store
                .refuse_proposal_before_effects(old.request.operation_id, None)
                .unwrap();
            let record = store.proposal(old.approved.draft.id).unwrap().unwrap();
            let mut candidate = record
                .draft
                .action_changes
                .iter()
                .map(|change| change.data().clone())
                .collect::<Vec<_>>();
            candidate[0].description = "Accepted current English ja eesti λ\r\n".into();
            candidate[1].state = ActionState::Waiting;
            let record = store
                .edit_proposal(&ProposalEdit {
                    expected: record.stamp(),
                    title: "Accepted later title".into(),
                    texts: record
                        .draft
                        .changes
                        .iter()
                        .map(|change| change.text().map(|_| "Accepted later note\r\nλ".into()))
                        .collect(),
                    action_data: candidate,
                })
                .unwrap();
            let revised = store
                .begin_proposal_apply(&ApprovalRequest {
                    operation_id: Uuid::new_v4(),
                    expected: record.stamp(),
                })
                .unwrap();
            let terminal = applied(&revised);
            let canonical = fixture.files.write(&terminal, None).unwrap();
            drop(store);
            let restarted = ApplyRecoveryFiles::open(&fixture.data).unwrap();
            restarted.retire_review_temporaries(&terminal).unwrap();
            assert!(!temporary.exists());
            assert_eq!(
                restarted.read(terminal.request.operation_id).unwrap(),
                Some(canonical)
            );
            restarted.retire_review_temporaries(&terminal).unwrap();
            assert!(fixture.temps().is_empty());
        }
    }

    #[test]
    fn temporary_retirement_requires_known_forward_repair_history() {
        use brn_store::work::proposal_apply::{
            RepairAttempt, RepairBinding, RepairDirection, RepairRequest,
        };
        let fixture = Fixture::new();
        let old = fixture.commented();
        let mut prepared = applied(&old);
        prepared.approved.comments = old.approved.comments.clone();
        prepared.receipt = None;
        prepared.observations = None;
        let before: Vec<_> = prepared
            .approved
            .draft
            .changes
            .iter()
            .zip(prepared.prepared.as_ref().unwrap())
            .map(|(change, staged)| match change {
                NoteChange::Create { .. } => ApplyMemberProof {
                    destination: None,
                    staging: Some(staged.clone()),
                },
                NoteChange::Replace { before, .. } => ApplyMemberProof {
                    destination: Some(before.clone()),
                    staging: Some(staged.clone()),
                },
                _ => unreachable!(),
            })
            .collect();
        let preview = prepared.repair_preview(&before).unwrap();
        prepared.repair = Some(RepairBinding {
            attempts: vec![RepairAttempt {
                request: RepairRequest {
                    id: Uuid::new_v4(),
                    operation_id: prepared.request.operation_id,
                    expected: preview.expected,
                    direction: RepairDirection::Finish,
                },
                started_at_ms: prepared.started_at_ms,
                outcome: None,
            }],
            observations: before,
        });
        prepared.validate().unwrap();
        let terminal = applied(&prepared);
        fixture.files.write(&terminal, None).unwrap();
        let compatible = fixture.temporary(&encode(&prepared).unwrap());
        let mut foreign = prepared.clone();
        foreign.repair.as_mut().unwrap().attempts[0].request.id = Uuid::new_v4();
        foreign.validate().unwrap();
        let retained = fixture.temporary(&encode(&foreign).unwrap());
        fixture.files.retire_review_temporaries(&terminal).unwrap();
        assert!(!compatible.exists());
        assert_eq!(fs::read(&retained).unwrap(), encode(&foreign).unwrap());

        // A later approval may retire a known prior operation's annotations,
        // including compatible displaced repair snapshots from that operation.
        let historical = fixture.temporary(&encode(&prepared).unwrap());
        let mut later = applied(&old);
        later.request.operation_id = Uuid::new_v4();
        later.approved.version += 3;
        later.request.expected = later.approved.stamp();
        later.receipt.as_mut().unwrap().operation_id = later.request.operation_id;
        later.receipt.as_mut().unwrap().approved_version = later.approved.version;
        later.receipt.as_mut().unwrap().stamp.version = later.approved.version + 2;
        later.validate().unwrap();
        fixture.files.retire_review_temporaries(&later).unwrap();
        assert!(!historical.exists());
        assert_eq!(fs::read(retained).unwrap(), encode(&foreign).unwrap());
    }

    #[test]
    fn temporary_retirement_requires_the_exact_undo_binding() {
        use brn_store::work::proposal_apply::UndoRequest;
        let fixture = Fixture::new();
        let source = applied(&fixture.journal);
        let (mut store, _) = WorkStore::open(&fixture.data).unwrap();
        store.restore_proposal_apply(&source).unwrap();
        let undo = store
            .begin_proposal_undo(&UndoRequest {
                operation_id: Uuid::new_v4(),
                target_operation_id: source.request.operation_id,
                trash_member: None,
            })
            .unwrap();
        let terminal = applied(&undo);
        let compatible = fixture.temporary(&encode(&undo).unwrap());
        let mut foreign = undo.clone();
        foreign.undo.as_mut().unwrap().operation_id = Uuid::new_v4();
        foreign.validate().unwrap();
        let retained = fixture.temporary(&encode(&foreign).unwrap());
        fixture.files.retire_review_temporaries(&terminal).unwrap();
        assert!(!compatible.exists());
        assert_eq!(fs::read(retained).unwrap(), encode(&foreign).unwrap());
    }

    struct HookReset;
    impl Drop for HookReset {
        fn drop(&mut self) {
            FAILURE.with(|failure| failure.set(None));
            BEFORE_RENAME.with(|hook| *hook.borrow_mut() = None);
            BEFORE_RETIRE.with(|hook| *hook.borrow_mut() = None);
        }
    }

    #[test]
    fn exact_journal_new_noop_restart_and_exclusive_replay() {
        let fixture = Fixture::new();
        assert!(
            fixture
                .files
                .read(fixture.journal.request.operation_id)
                .unwrap()
                .is_none()
        );
        let snapshot = fixture.files.write(&fixture.journal, None).unwrap();
        assert_eq!(snapshot.journal, fixture.journal);
        assert_eq!(
            snapshot.proof.sha256,
            Some(digest(&fs::read(fixture.path()).unwrap()))
        );
        assert_eq!(fs::metadata(fixture.path()).unwrap().mode() & 0o777, 0o600);
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(fixture.path()).unwrap()).unwrap();
        assert!(value["journal"].is_object());
        assert_eq!(
            fixture
                .files
                .write(&fixture.journal, Some(&snapshot))
                .unwrap(),
            snapshot
        );
        assert_eq!(
            fixture
                .files
                .write(&fixture.journal, None)
                .unwrap_err()
                .filesystem_outcome,
            FileOutcome::NotApplied
        );
        assert_eq!(
            ApplyRecoveryFiles::open(&fixture.data)
                .unwrap()
                .read(fixture.journal.request.operation_id)
                .unwrap(),
            Some(snapshot)
        );
        assert!(fixture.temps().is_empty());
    }

    #[test]
    fn atomic_update_requires_exact_token_and_retires_only_known_displacement() {
        let fixture = Fixture::new();
        let before = fixture.files.write(&fixture.journal, None).unwrap();
        let after = fixture
            .files
            .write(&fixture.changed(), Some(&before))
            .unwrap();
        assert_ne!(before.proof.identity.inode, after.proof.identity.inode);
        assert_eq!(after.journal, fixture.changed());
        assert!(fixture.temps().is_empty());
        let bytes = fs::read(fixture.path()).unwrap();
        assert_eq!(
            fixture
                .files
                .write(&fixture.journal, Some(&before))
                .unwrap_err()
                .filesystem_outcome,
            FileOutcome::NotApplied
        );
        assert_eq!(fs::read(fixture.path()).unwrap(), bytes);
        let mut wrong = after.clone();
        wrong.journal.started_at_ms += 1;
        assert!(fixture.files.write(&fixture.journal, Some(&wrong)).is_err());
        wrong = after.clone();
        wrong.proof.relative = PathBuf::from(name(Uuid::new_v4()).unwrap());
        assert!(fixture.files.write(&fixture.journal, Some(&wrong)).is_err());
        let mut foreign = fixture.journal.clone();
        foreign.request.operation_id = Uuid::new_v4();
        assert!(fixture.files.write(&foreign, Some(&after)).is_err());
        assert_eq!(
            fixture.files.ids().unwrap(),
            vec![fixture.journal.request.operation_id]
        );
        assert_eq!(fs::read(fixture.path()).unwrap(), bytes);
    }

    #[test]
    fn metadata_length_and_hash_tokens_reject_in_place_changes() {
        let fixture = Fixture::new();
        let snapshot = fixture.files.write(&fixture.journal, None).unwrap();
        let bytes = encode(&fixture.changed()).unwrap();
        fs::write(fixture.path(), &bytes).unwrap();
        assert!(
            fixture
                .files
                .write(&fixture.journal, Some(&snapshot))
                .is_err()
        );
        assert_eq!(fs::read(fixture.path()).unwrap(), bytes);
    }

    #[test]
    fn malformed_envelopes_domain_uuid_format_hash_and_utf8_are_refused() {
        let fixture = Fixture::new();
        let original: serde_json::Value =
            serde_json::from_slice(&encode(&fixture.journal).unwrap()).unwrap();
        for malformed in [
            {
                let mut value = original.clone();
                value["format"] = 2.into();
                value
            },
            {
                let mut value = original.clone();
                value["unknown"] = true.into();
                value
            },
            {
                let mut value = original.clone();
                value["sha256"][0] = ((value["sha256"][0].as_u64().unwrap() + 1) % 256).into();
                value
            },
        ] {
            fs::write(fixture.path(), serde_json::to_vec(&malformed).unwrap()).unwrap();
            assert!(
                fixture
                    .files
                    .read(fixture.journal.request.operation_id)
                    .is_err()
            );
        }
        let mut wrong_uuid = fixture.journal.clone();
        wrong_uuid.request.operation_id = Uuid::new_v4();
        fs::write(fixture.path(), encode(&wrong_uuid).unwrap()).unwrap();
        assert!(
            fixture
                .files
                .read(fixture.journal.request.operation_id)
                .is_err()
        );
        let mut invalid_domain = fixture.journal.clone();
        invalid_domain.members.clear();
        let envelope = Envelope {
            format: 1,
            sha256: digest(&serde_json::to_vec(&invalid_domain).unwrap()),
            journal: invalid_domain,
        };
        fs::write(fixture.path(), serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert!(
            fixture
                .files
                .read(fixture.journal.request.operation_id)
                .is_err()
        );
        assert!(fixture.files.write(&envelope.journal, None).is_err());
        fs::write(fixture.path(), [0xff]).unwrap();
        assert!(
            fixture
                .files
                .read(fixture.journal.request.operation_id)
                .is_err()
        );
        File::create(fixture.path())
            .unwrap()
            .set_len((MAX_RECORD_BYTES + 1) as u64)
            .unwrap();
        assert!(
            fixture
                .files
                .read(fixture.journal.request.operation_id)
                .is_err()
        );
    }

    #[test]
    fn receipt_symlink_hardlink_fifo_and_symlinked_directory_are_refused() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new();
        let other = fixture._base.path().join("other");
        fs::write(&other, b"unrelated sentinel").unwrap();
        symlink(&other, fixture.path()).unwrap();
        assert!(
            fixture
                .files
                .read(fixture.journal.request.operation_id)
                .is_err()
        );
        assert!(fixture.files.write(&fixture.journal, None).is_err());
        fs::remove_file(fixture.path()).unwrap();
        fs::hard_link(&other, fixture.path()).unwrap();
        assert!(
            fixture
                .files
                .read(fixture.journal.request.operation_id)
                .is_err()
        );
        fs::remove_file(fixture.path()).unwrap();
        // SAFETY: the C string names a fresh synthetic fixture path.
        assert_eq!(
            unsafe { libc::mkfifo(cstring(&fixture.path()).unwrap().as_ptr(), 0o600) },
            0
        );
        let started = std::time::Instant::now();
        assert!(
            fixture
                .files
                .read(fixture.journal.request.operation_id)
                .is_err()
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        assert_eq!(fs::read(&other).unwrap(), b"unrelated sentinel");
        let alias = fixture._base.path().join("alias");
        symlink(&fixture.data, &alias).unwrap();
        assert!(ApplyRecoveryFiles::open(&alias).is_err());
    }

    #[test]
    fn root_replacement_refuses_reads_listing_and_writes() {
        let fixture = Fixture::new();
        let snapshot = fixture.files.write(&fixture.journal, None).unwrap();
        let parked = fixture._base.path().join("parked");
        fs::rename(&fixture.data, &parked).unwrap();
        fs::create_dir(&fixture.data).unwrap();
        assert!(
            fixture
                .files
                .read(fixture.journal.request.operation_id)
                .is_err()
        );
        assert!(fixture.files.ids().is_err());
        assert!(
            fixture
                .files
                .retire_review_temporaries(&applied(&fixture.journal))
                .is_err()
        );
        assert!(
            fixture
                .files
                .write(&fixture.changed(), Some(&snapshot))
                .is_err()
        );
        assert!(fs::read_dir(&fixture.data).unwrap().next().is_none());
        assert_eq!(
            fs::read(parked.join(&snapshot.proof.relative)).unwrap(),
            encode(&fixture.journal).unwrap()
        );
    }

    #[test]
    fn enumeration_is_names_only_repeatable_sorted_and_namespace_strict() {
        let fixture = Fixture::new();
        let mut ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        for id in &ids {
            fs::write(fixture.data.join(name(*id).unwrap()), b"not a journal").unwrap();
        }
        fs::write(fixture.data.join("unrelated.txt"), b"sentinel").unwrap();
        fs::write(
            fixture
                .data
                .join(format!("{TEMP_PREFIX}{}{TEMP_SUFFIX}", Uuid::new_v4())),
            b"incomplete",
        )
        .unwrap();
        ids.sort_unstable();
        let invalid_body = ids[0];
        assert_eq!(fixture.files.ids().unwrap(), ids);
        assert_eq!(fixture.files.ids().unwrap(), ids);
        fixture.files.write(&fixture.journal, None).unwrap();
        ids.push(fixture.journal.request.operation_id);
        ids.sort_unstable();
        assert_eq!(fixture.files.ids().unwrap(), ids);
        assert!(fixture.files.read(invalid_body).is_err());
        let bad = fixture.data.join(format!(
            "{PREFIX}{}{SUFFIX}",
            Uuid::new_v4().to_string().to_uppercase()
        ));
        fs::write(&bad, b"unknown").unwrap();
        assert!(fixture.files.ids().is_err());
        fs::remove_file(bad).unwrap();
        fs::write(fixture.data.join(".brn-apply-invalid.receipt"), b"unknown").unwrap();
        assert!(fixture.files.ids().is_err());
    }

    #[test]
    fn required_failures_distinguish_before_and_after_rename_and_retain_proofs() {
        let _reset = HookReset;
        for phase in [
            "write",
            "file_sync",
            "prepare_directory_sync",
            "rename",
            "directory_sync",
            "postproof",
        ] {
            let fixture = Fixture::new();
            FAILURE.with(|failure| failure.set(Some(phase)));
            let error = fixture.files.write(&fixture.journal, None).unwrap_err();
            FAILURE.with(|failure| failure.set(None));
            assert_eq!(
                error.filesystem_outcome,
                if matches!(phase, "write" | "file_sync" | "prepare_directory_sync") {
                    FileOutcome::NotApplied
                } else {
                    FileOutcome::Unknown
                },
                "{phase}"
            );
            if matches!(phase, "directory_sync" | "postproof") {
                assert_eq!(
                    fixture
                        .files
                        .read(fixture.journal.request.operation_id)
                        .unwrap()
                        .unwrap()
                        .journal,
                    fixture.journal
                );
            } else {
                assert!(
                    fixture
                        .files
                        .read(fixture.journal.request.operation_id)
                        .unwrap()
                        .is_none()
                );
                assert_eq!(fixture.temps().len(), 1);
            }
        }
        let fixture = Fixture::new();
        let before = fixture.files.write(&fixture.journal, None).unwrap();
        FAILURE.with(|failure| failure.set(Some("cleanup_directory_sync")));
        let error = fixture
            .files
            .write(&fixture.changed(), Some(&before))
            .unwrap_err();
        FAILURE.with(|failure| failure.set(None));
        assert_eq!(error.filesystem_outcome, FileOutcome::Unknown);
        assert_eq!(
            fixture
                .files
                .read(fixture.journal.request.operation_id)
                .unwrap()
                .unwrap()
                .journal,
            fixture.changed()
        );
        let fixture = Fixture::new();
        let before = fixture.files.write(&fixture.journal, None).unwrap();
        FAILURE.with(|failure| failure.set(Some("cleanup")));
        let error = fixture
            .files
            .write(&fixture.changed(), Some(&before))
            .unwrap_err();
        FAILURE.with(|failure| failure.set(None));
        assert_eq!(error.filesystem_outcome, FileOutcome::Unknown);
        assert_eq!(fixture.temps().len(), 1);
        assert_eq!(
            fs::read(&fixture.temps()[0]).unwrap(),
            encode(&fixture.journal).unwrap()
        );
    }

    #[test]
    fn unexpected_displacement_and_exclusive_collision_are_retained() {
        let _reset = HookReset;
        let fixture = Fixture::new();
        let snapshot = fixture.files.write(&fixture.journal, None).unwrap();
        let target = fixture.path();
        let external = fixture.data.join("external");
        BEFORE_RENAME.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move || {
                fs::write(&external, b"unexpected displacement").unwrap();
                fs::rename(external, target).unwrap();
            }))
        });
        assert_eq!(
            fixture
                .files
                .write(&fixture.changed(), Some(&snapshot))
                .unwrap_err()
                .filesystem_outcome,
            FileOutcome::Unknown
        );
        let temps = fixture.temps();
        assert_eq!(temps.len(), 1);
        assert_eq!(fs::read(&temps[0]).unwrap(), b"unexpected displacement");
        assert_eq!(
            fixture
                .files
                .read(fixture.journal.request.operation_id)
                .unwrap()
                .unwrap()
                .journal,
            fixture.changed()
        );
        let fixture = Fixture::new();
        let target = fixture.path();
        BEFORE_RENAME.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move || {
                fs::write(target, b"unexpected occupant").unwrap()
            }))
        });
        assert_eq!(
            fixture
                .files
                .write(&fixture.journal, None)
                .unwrap_err()
                .filesystem_outcome,
            FileOutcome::Unknown
        );
        assert_eq!(fs::read(fixture.path()).unwrap(), b"unexpected occupant");
        assert_eq!(fixture.temps().len(), 1);
    }

    #[test]
    fn aggregate_journal_can_exceed_the_individual_note_limit() {
        let fixture = Fixture::new();
        let mut journal = fixture.journal.clone();
        for change in &mut journal.approved.draft.changes {
            match change {
                NoteChange::Create { text, .. } | NoteChange::Replace { text, .. } => {
                    *text = "x".repeat(crate::MAX_NOTE_BYTES)
                }
                NoteChange::Trash { .. }
                | NoteChange::CreateAsset { .. }
                | NoteChange::ReplaceAsset { .. }
                | NoteChange::TrashAsset { .. } => unreachable!("Create/Replace Markdown fixture"),
            }
        }
        journal.creation_sha256 = digest(&serde_json::to_vec(&journal.approved.draft).unwrap());
        let snapshot = fixture.files.write(&journal, None).unwrap();
        assert!(snapshot.proof.identity.len > crate::MAX_NOTE_BYTES as u64);
        assert_eq!(
            fixture.files.read(journal.request.operation_id).unwrap(),
            Some(snapshot)
        );
    }

    #[test]
    fn applied_retirement_recovers_displaced_review_comments_after_cleanup_failure() {
        let _reset = HookReset;
        let fixture = Fixture::new();
        let old = fixture.commented();
        let snapshot = fixture.files.write(&old, None).unwrap();
        let final_journal = applied(&old);
        FAILURE.with(|failure| failure.set(Some("cleanup")));
        assert_eq!(
            fixture
                .files
                .write(&final_journal, Some(&snapshot))
                .unwrap_err()
                .filesystem_outcome,
            FileOutcome::Unknown
        );
        FAILURE.with(|failure| failure.set(None));
        let temporary = fixture.temps().pop().unwrap();
        assert_eq!(fs::read(&temporary).unwrap(), encode(&old).unwrap());
        assert_eq!(fixture.files.ids().unwrap(), vec![old.request.operation_id]);
        let canonical = fs::read(fixture.path()).unwrap();
        let restarted = ApplyRecoveryFiles::open(&fixture.data).unwrap();
        restarted.retire_review_temporaries(&final_journal).unwrap();
        assert!(!temporary.exists());
        assert_eq!(fs::read(fixture.path()).unwrap(), canonical);
        restarted.retire_review_temporaries(&final_journal).unwrap();
        assert!(fixture.temps().is_empty());
    }

    #[test]
    fn applied_retirement_covers_failed_new_staging_and_accepted_review_edits() {
        let _reset = HookReset;
        let fixture = Fixture::new();
        let old = fixture.commented();
        FAILURE.with(|failure| failure.set(Some("prepare_directory_sync")));
        assert_eq!(
            fixture
                .files
                .write(&old, None)
                .unwrap_err()
                .filesystem_outcome,
            FileOutcome::NotApplied
        );
        FAILURE.with(|failure| failure.set(None));
        let temporary = fixture.temps().pop().unwrap();
        assert_eq!(fs::read(&temporary).unwrap(), encode(&old).unwrap());
        let mut revised = old.clone();
        revised.approved.version += 1;
        revised.approved.updated_at_ms += 1;
        revised.started_at_ms += 1;
        revised.approved.draft.title = "Accepted later title".into();
        for change in &mut revised.approved.draft.changes {
            if let NoteChange::Create { text, .. } | NoteChange::Replace { text, .. } = change {
                *text = "Accepted later bytes\r\n🦀".into();
            }
        }
        revised.request.operation_id = Uuid::new_v4();
        revised.request.expected = revised.approved.stamp();
        let final_journal = applied(&revised);
        let final_snapshot = fixture.files.write(&final_journal, None).unwrap();
        fixture
            .files
            .retire_review_temporaries(&final_journal)
            .unwrap();
        assert!(!temporary.exists());
        assert_eq!(
            fixture
                .files
                .read(final_journal.request.operation_id)
                .unwrap(),
            Some(final_snapshot)
        );
        assert!(
            fixture
                .files
                .read(old.request.operation_id)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn retirement_preserves_malformed_tampered_unrelated_and_incompatible_temporaries() {
        let fixture = Fixture::new();
        let old = fixture.commented();
        let final_journal = applied(&old);
        let canonical = fixture.files.write(&final_journal, None).unwrap();
        let mut preserved = vec![(
            fixture.temporary(b"malformed occupant"),
            b"malformed occupant".to_vec(),
        )];
        let mut tampered: serde_json::Value =
            serde_json::from_slice(&encode(&old).unwrap()).unwrap();
        tampered["sha256"][0] = ((tampered["sha256"][0].as_u64().unwrap() + 1) % 256).into();
        let bytes = serde_json::to_vec(&tampered).unwrap();
        preserved.push((fixture.temporary(&bytes), bytes));
        let malformed_name = fixture.data.join(".brn-apply-temp-invalid.stage");
        fs::write(&malformed_name, encode(&old).unwrap()).unwrap();
        preserved.push((malformed_name, encode(&old).unwrap()));
        let unrelated = fixture.data.join("unrelated.stage");
        fs::write(&unrelated, encode(&old).unwrap()).unwrap();
        preserved.push((unrelated, encode(&old).unwrap()));
        for binding in 0..13 {
            let mut foreign = old.clone();
            match binding {
                0 => foreign.approved.draft.id = Uuid::new_v4(),
                1 => foreign.creation_sha256[0] ^= 1,
                2 => foreign.approved.draft.vault.as_mut().unwrap().id = Uuid::new_v4(),
                3 => foreign.approved.created_at_ms += 1,
                4 => foreign.approved.draft.group_id = Some(Uuid::new_v4()),
                5 => foreign.approved.draft.session_id = Some(Uuid::new_v4()),
                6 => foreign.approved.version += 1,
                7 => foreign.approved.draft.sources.push(SourceVersion {
                    path: "source.md".into(),
                    fingerprint: FileFingerprint {
                        device: 1,
                        inode: 9,
                        len: 0,
                        sha256: digest(b""),
                    },
                }),
                8 => {
                    if let NoteChange::Create { path, .. } = &mut foreign.approved.draft.changes[0]
                    {
                        *path = "other.md".into();
                    }
                }
                9 => {
                    if let NoteChange::Create { parent, .. } =
                        &mut foreign.approved.draft.changes[0]
                    {
                        parent.inode += 1;
                    }
                }
                10 => {
                    if let NoteChange::Replace { before, .. } =
                        &mut foreign.approved.draft.changes[1]
                    {
                        before.inode += 1;
                    }
                }
                11 => {
                    if let NoteChange::Replace {
                        before,
                        before_text,
                        ..
                    } = &mut foreign.approved.draft.changes[1]
                    {
                        *before_text = "different original bytes".into();
                        before.len = before_text.len() as u64;
                        before.sha256 = digest(before_text.as_bytes());
                    }
                }
                12 => {
                    if let NoteChange::Replace {
                        path,
                        parent,
                        before,
                        before_text,
                        ..
                    } = &foreign.approved.draft.changes[1]
                    {
                        foreign.approved.draft.changes[1] = NoteChange::Trash {
                            path: path.clone(),
                            parent: parent.clone(),
                            before: before.clone(),
                            before_text: before_text.clone(),
                        };
                    }
                }
                _ => unreachable!(),
            }
            foreign.request.expected = foreign.approved.stamp();
            let bytes = encode(&foreign).unwrap();
            preserved.push((fixture.temporary(&bytes), bytes));
        }
        fixture
            .files
            .retire_review_temporaries(&final_journal)
            .unwrap();
        for (path, bytes) in preserved {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
        assert_eq!(
            fixture
                .files
                .read(final_journal.request.operation_id)
                .unwrap(),
            Some(canonical)
        );
        assert!(fixture.files.retire_review_temporaries(&old).is_err());
    }

    #[test]
    fn matching_retirement_failure_fences_completion_and_retry_syncs_absent_names() {
        let _reset = HookReset;
        for phase in ["cleanup", "cleanup_directory_sync"] {
            let fixture = Fixture::new();
            let old = fixture.commented();
            let final_journal = applied(&old);
            let canonical = fixture.files.write(&final_journal, None).unwrap();
            let temporary = fixture.temporary(&encode(&old).unwrap());
            FAILURE.with(|failure| failure.set(Some(phase)));
            assert_eq!(
                fixture
                    .files
                    .retire_review_temporaries(&final_journal)
                    .unwrap_err()
                    .filesystem_outcome,
                FileOutcome::Unknown
            );
            assert_eq!(temporary.exists(), phase == "cleanup");
            FAILURE.with(|failure| failure.set(Some("cleanup_directory_sync")));
            assert!(
                fixture
                    .files
                    .retire_review_temporaries(&final_journal)
                    .is_err()
            );
            FAILURE.with(|failure| failure.set(None));
            fixture
                .files
                .retire_review_temporaries(&final_journal)
                .unwrap();
            assert!(!temporary.exists());
            assert_eq!(
                fixture
                    .files
                    .read(final_journal.request.operation_id)
                    .unwrap(),
                Some(canonical)
            );
        }
    }

    #[test]
    fn retirement_preserves_an_unexpected_occupant_after_matching_snapshot_was_read() {
        let _reset = HookReset;
        let fixture = Fixture::new();
        let old = fixture.commented();
        let final_journal = applied(&old);
        let canonical = fixture.files.write(&final_journal, None).unwrap();
        let temporary = fixture.temporary(&encode(&old).unwrap());
        let changed = temporary.clone();
        BEFORE_RETIRE.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move || {
                fs::write(changed, b"unexpected replacement occupant").unwrap();
            }));
        });
        assert_eq!(
            fixture
                .files
                .retire_review_temporaries(&final_journal)
                .unwrap_err()
                .filesystem_outcome,
            FileOutcome::Unknown
        );
        assert_eq!(
            fs::read(temporary).unwrap(),
            b"unexpected replacement occupant"
        );
        assert_eq!(
            fixture
                .files
                .read(final_journal.request.operation_id)
                .unwrap(),
            Some(canonical)
        );
    }

    #[test]
    fn temporary_read_io_failure_preserves_snapshot_and_fences_retirement() {
        let _reset = HookReset;
        let fixture = Fixture::new();
        let old = fixture.commented();
        let final_journal = applied(&old);
        let canonical = fixture.files.write(&final_journal, None).unwrap();
        let bytes = encode(&old).unwrap();
        let temporary = fixture.temporary(&bytes);
        FAILURE.with(|failure| failure.set(Some("read")));
        let error = fixture
            .files
            .retire_review_temporaries(&final_journal)
            .unwrap_err();
        FAILURE.with(|failure| failure.set(None));
        assert_eq!(error.code, FileErrorCode::Io);
        assert_eq!(fs::read(&temporary).unwrap(), bytes);
        fixture
            .files
            .retire_review_temporaries(&final_journal)
            .unwrap();
        assert!(!temporary.exists());
        assert_eq!(
            fixture
                .files
                .read(final_journal.request.operation_id)
                .unwrap(),
            Some(canonical)
        );
    }
}

#[cfg(all(test, target_os = "macos"))]
thread_local! {
    pub(crate) static FAILURE: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
    static BEFORE_RENAME: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
    static BEFORE_RETIRE: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
}

#[cfg(target_os = "macos")]
fn step(_step: &str) -> FileResult<()> {
    #[cfg(test)]
    if FAILURE.with(|selected| selected.get() == Some(_step)) {
        return Err(failure(
            super::FileErrorCode::Io,
            "injected approval recovery I/O failure",
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn before_rename() {
    #[cfg(test)]
    BEFORE_RENAME.with(|hook| {
        if let Some(hook) = hook.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(target_os = "macos")]
fn before_retire() {
    #[cfg(test)]
    BEFORE_RETIRE.with(|hook| {
        if let Some(hook) = hook.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub(crate) struct ApplyRecoveryFiles;

#[cfg(not(target_os = "macos"))]
impl ApplyRecoveryFiles {
    pub(crate) fn open(_: &Path) -> FileResult<Self> {
        Err(invalid("approval recovery persistence requires macOS"))
    }
    pub(crate) fn ids(&self) -> FileResult<Vec<Uuid>> {
        Err(invalid("approval recovery persistence requires macOS"))
    }
    pub(crate) fn read(&self, _: Uuid) -> FileResult<Option<RecoverySnapshot>> {
        Err(invalid("approval recovery persistence requires macOS"))
    }
    pub(crate) fn write(
        &self,
        _: &ApplyJournal,
        _: Option<&RecoverySnapshot>,
    ) -> FileResult<RecoverySnapshot> {
        Err(invalid("approval recovery persistence requires macOS"))
    }
    pub(crate) fn retire_review_temporaries(&self, _: &ApplyJournal) -> FileResult<()> {
        Err(invalid("approval recovery persistence requires macOS"))
    }
}
