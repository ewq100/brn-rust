//! Derived read-only review of saved citation evidence, never a semantic Finding.
use super::{CitationOutcome, IdentityInventory, IdentityIssue, NoteProvenance};
use crate::{
    ErrorKind, Result, WorkflowError,
    ai_tools::AiTools,
    app::App,
    library::{saved_metadata, title},
    vault::{self, EvidencePath, VaultPath},
};
use brn_store::files::{VaultIdentity, VaultRecord};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io, os::unix::fs::MetadataExt, sync::Arc};
use uuid::Uuid;

/// Complete serialized pages/details must fit; neither quotations nor proofs are truncated.
pub const CITATION_REVIEW_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const DIAGNOSTICS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationReviewRequest {
    pub limit: usize,
    pub cursor: Option<CitationReviewCursor>,
}
impl Default for CitationReviewRequest {
    fn default() -> Self {
        Self {
            limit: 25,
            cursor: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationReviewCursor {
    #[serde(with = "strict_vault")]
    pub vault: VaultRecord,
    pub observation_digest: [u8; 32],
    pub after_path: String,
}
// The reused storage record predates strict external request DTOs. Keep this
// public field unchanged while rejecting unknown nested cursor fields here.
mod strict_vault {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        value: &VaultRecord,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        value.serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<VaultRecord, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Identity {
            device: u64,
            inode: u64,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Record {
            id: Uuid,
            root: std::path::PathBuf,
            identity: Identity,
        }
        let value = Record::deserialize(deserializer)?;
        Ok(VaultRecord {
            id: value.id,
            root: value.root,
            identity: VaultIdentity {
                device: value.identity.device,
                inode: value.identity.inode,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationReviewIssue {
    pub index: usize,
    pub outcome: CitationOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationReviewEntry {
    pub path: String,
    pub title: String,
    pub note_id: Option<Uuid>,
    pub sha256: [u8; 32],
    pub citations: Vec<CitationReviewIssue>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationReviewCoverage {
    pub incomplete: bool,
    pub diagnostics: Vec<IdentityIssue>,
    pub diagnostic_count: usize,
    pub diagnostics_truncated: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationReviewPage {
    pub entries: Vec<CitationReviewEntry>,
    pub next_cursor: Option<CitationReviewCursor>,
    pub inspected_count: usize,
    pub coverage: CitationReviewCoverage,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationReviewDetailRequest {
    pub path: String,
    pub expected_sha256: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationReviewDetail {
    pub path: String,
    pub title: String,
    pub note_id: Option<Uuid>,
    pub sha256: [u8; 32],
    pub text: String,
    pub provenance: NoteProvenance,
}

fn rejected(message: impl std::fmt::Display) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ToolRejected, message.to_string())
}

fn validate_cursor(cursor: &CitationReviewCursor) -> Result<()> {
    VaultPath::parse(&cursor.after_path).map_err(rejected)?;
    if cursor.vault.id.is_nil()
        || !cursor.vault.root.is_absolute()
        || cursor.vault.root.components().any(|part| {
            matches!(
                part,
                std::path::Component::CurDir | std::path::Component::ParentDir
            )
        })
    {
        return Err(rejected(
            "Citation review cursor needs a contained path and absolute vault identity.",
        ));
    }
    Ok(())
}
impl CitationReviewRequest {
    pub fn validate(&self) -> Result<()> {
        if !(1..=100).contains(&self.limit) {
            return Err(rejected("Citation review limit must be between 1 and 100."));
        }
        if let Some(cursor) = &self.cursor {
            validate_cursor(cursor)?;
        }
        Ok(())
    }
}
impl CitationReviewDetailRequest {
    pub fn validate(&self) -> Result<()> {
        VaultPath::parse(&self.path).map_err(rejected)?;
        Ok(())
    }
}
impl CitationReviewPage {
    pub fn validate_for(&self, request: &CitationReviewRequest) -> Result<()> {
        request.validate()?;
        let coverage = &self.coverage;
        if self.inspected_count > request.limit
            || self.entries.len() > self.inspected_count
            || coverage.diagnostics.len() > DIAGNOSTICS
            || coverage.diagnostic_count < coverage.diagnostics.len()
            || coverage.diagnostics.len() != coverage.diagnostic_count.min(DIAGNOSTICS)
            || coverage.diagnostics_truncated != (coverage.diagnostic_count > DIAGNOSTICS)
            || coverage.incomplete != (coverage.diagnostic_count > 0)
            || coverage
                .diagnostics
                .iter()
                .any(|issue| issue.reason.is_empty())
            || !coverage
                .diagnostics
                .windows(2)
                .all(|pair| (&pair[0].path, &pair[0].reason) < (&pair[1].path, &pair[1].reason))
        {
            return Err(rejected(
                "Citation review reply has invalid counts or coverage.",
            ));
        }
        let mut previous = request
            .cursor
            .as_ref()
            .map(|cursor| cursor.after_path.as_str());
        for entry in &self.entries {
            VaultPath::parse(&entry.path).map_err(rejected)?;
            if previous.is_some_and(|path| entry.path.as_str() <= path)
                || entry.title.is_empty()
                || entry.note_id.is_some_and(|id| id.is_nil())
                || entry.citations.is_empty()
                || entry.citations.iter().any(|issue| {
                    issue.outcome == CitationOutcome::Matched
                        || issue.index >= brn_store::note_provenance::MAX_CITATIONS
                })
                || !entry
                    .citations
                    .windows(2)
                    .all(|pair| pair[0].index < pair[1].index)
            {
                return Err(rejected(
                    "Citation review reply has invalid entry order or issues.",
                ));
            }
            previous = Some(&entry.path);
        }
        if let Some(next) = &self.next_cursor {
            validate_cursor(next)?;
            if self.inspected_count != request.limit
                || previous.is_some_and(|path| next.after_path.as_str() < path)
                || request.cursor.as_ref().is_some_and(|cursor| {
                    next.after_path <= cursor.after_path
                        || next.vault != cursor.vault
                        || next.observation_digest != cursor.observation_digest
                })
            {
                return Err(rejected(
                    "Citation review continuation did not preserve scope and progress.",
                ));
            }
        }
        response_budget(self)
    }
}
impl CitationReviewDetail {
    pub fn validate_for(&self, request: &CitationReviewDetailRequest) -> Result<()> {
        request.validate()?;
        if self.path != request.path
            || self.provenance.path != request.path
            || self.sha256 != request.expected_sha256
            || <[u8; 32]>::from(Sha256::digest(self.text.as_bytes())) != self.sha256
            || self.text.len() > crate::MAX_NOTE_BYTES
            || self.title != title(&self.text, &self.path)
            || saved_metadata(&self.text, &self.path).issue.is_some()
            || !is_current(&self.text, &self.path)
            || brn_store::note_identity::read(&self.text).map_err(rejected)? != self.note_id
            || brn_store::note_provenance::read(&self.text).map_err(rejected)?
                != self
                    .provenance
                    .citations
                    .iter()
                    .map(|item| item.citation.clone())
                    .collect::<Vec<_>>()
            || brn_store::work::inbox_source::read_provenance(&self.text)?
                != self.provenance.inbox_source
        {
            return Err(rejected(
                "Citation review detail is not bound to the exact saved consumer.",
            ));
        }
        response_budget(self)
    }
}

fn stale() -> WorkflowError {
    WorkflowError::typed(
        ErrorKind::ContextStale,
        "Citation evidence changed or is unavailable. Refresh citation review before continuing.",
    )
}
fn is_current(text: &str, path: &str) -> bool {
    let metadata = saved_metadata(text, path);
    metadata.issue.is_none()
        && !metadata.source
        && !metadata.history
        && VaultPath::parse(path).is_ok()
}

struct Observation {
    inventory: IdentityInventory,
    candidates: Vec<CitationReviewEntry>,
    diagnostics: Vec<IdentityIssue>,
    digest: [u8; 32],
}
impl App {
    /// Bounds resolved consumers per page; the inherited identity scan remains O(vault).
    pub fn citation_review(
        &mut self,
        request: &CitationReviewRequest,
    ) -> Result<CitationReviewPage> {
        request.validate()?;
        let (tools, epoch, vault) = self.citation_review_boundary()?;
        // refresh() currently changes only disposable metadata, not the evidence epoch.
        self.refresh().map_err(|_| stale())?;
        tools.check_current_epoch(epoch).map_err(|_| stale())?;
        let observed = self.citation_review_observation()?;
        if request.cursor.as_ref().is_some_and(|cursor| {
            cursor.vault != vault
                || cursor.observation_digest != observed.digest
                || !observed
                    .candidates
                    .iter()
                    .any(|note| note.path == cursor.after_path)
        }) {
            return Err(stale());
        }
        let candidates = observed
            .candidates
            .iter()
            .filter(|note| {
                request
                    .cursor
                    .as_ref()
                    .is_none_or(|cursor| note.path > cursor.after_path)
            })
            .collect::<Vec<_>>();
        let mut entries = Vec::new();
        let mut proofs = Vec::new();
        let inspected = candidates
            .iter()
            .take(request.limit)
            .copied()
            .collect::<Vec<_>>();
        for candidate in &inspected {
            let note = self.citation_review_read(&candidate.path)?;
            if note.sha256 != candidate.sha256 || !is_current(&note.text, &candidate.path) {
                return Err(stale());
            }
            let provenance =
                self.provenance_from_saved(&candidate.path, &note, &observed.inventory)?;
            let citations = provenance
                .citations
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    (item.outcome != CitationOutcome::Matched).then_some(CitationReviewIssue {
                        index,
                        outcome: item.outcome,
                    })
                })
                .collect::<Vec<_>>();
            proofs.push(provenance);
            if !citations.is_empty() {
                let mut entry = (*candidate).clone();
                entry.citations = citations;
                entries.push(entry);
            }
        }
        let next_cursor = (candidates.len() > inspected.len()).then(|| CitationReviewCursor {
            vault: vault.clone(),
            observation_digest: observed.digest,
            after_path: inspected
                .last()
                .expect("nonempty bounded page")
                .path
                .clone(),
        });
        let diagnostic_count = observed.diagnostics.len();
        let page = CitationReviewPage {
            entries,
            next_cursor,
            inspected_count: inspected.len(),
            coverage: CitationReviewCoverage {
                incomplete: diagnostic_count > 0,
                diagnostics: observed
                    .diagnostics
                    .iter()
                    .take(DIAGNOSTICS)
                    .cloned()
                    .collect(),
                diagnostic_count,
                diagnostics_truncated: diagnostic_count > DIAGNOSTICS,
            },
        };
        let fresh = self.citation_review_recheck(&observed, &tools, epoch, &vault)?;
        for proof in proofs {
            let note = self.citation_review_read(&proof.path)?;
            if !fresh
                .inventory
                .notes
                .iter()
                .any(|identity| identity.path == proof.path && identity.sha256 == note.sha256)
                || self.provenance_from_saved(&proof.path, &note, &fresh.inventory)? != proof
            {
                return Err(stale());
            }
            self.citation_review_source_proofs(&proof)?;
        }
        self.citation_review_end(&tools, epoch, &vault)?;
        page.validate_for(request)?;
        Ok(page)
    }

    /// One bound, complete read of consumer and durable citation proofs.
    pub fn citation_review_detail(
        &self,
        request: &CitationReviewDetailRequest,
    ) -> Result<CitationReviewDetail> {
        request.validate()?;
        let (tools, epoch, vault) = self.citation_review_boundary()?;
        let observed = self.citation_review_observation()?;
        let note = self.citation_review_read(&request.path)?;
        if note.sha256 != request.expected_sha256
            || !is_current(&note.text, &request.path)
            || !observed
                .candidates
                .iter()
                .any(|candidate| candidate.path == request.path && candidate.sha256 == note.sha256)
        {
            return Err(stale());
        }
        let detail = CitationReviewDetail {
            path: request.path.clone(),
            title: title(&note.text, &request.path),
            note_id: brn_store::note_identity::read(&note.text).map_err(rejected)?,
            sha256: note.sha256,
            provenance: self.provenance_from_saved(&request.path, &note, &observed.inventory)?,
            text: note.text,
        };
        let fresh = self.citation_review_recheck(&observed, &tools, epoch, &vault)?;
        let note = self.citation_review_read(&request.path)?;
        if note.sha256 != detail.sha256
            || self.provenance_from_saved(&request.path, &note, &fresh.inventory)?
                != detail.provenance
        {
            return Err(stale());
        }
        self.citation_review_source_proofs(&detail.provenance)?;
        self.citation_review_end(&tools, epoch, &vault)?;
        detail.validate_for(request)?;
        Ok(detail)
    }

    pub(super) fn citation_review_boundary(&self) -> Result<(Arc<AiTools>, u64, VaultRecord)> {
        self.require_current_evidence().map_err(|_| stale())?;
        let tools = self.guarded_tools()?;
        let epoch = tools.check_root().map_err(|_| stale())?;
        Ok((tools, epoch, self.citation_review_vault()?))
    }

    fn citation_review_vault(&self) -> Result<VaultRecord> {
        let root = self.require_vault().map_err(|_| stale())?;
        let meta = root.symlink_metadata().map_err(|_| stale())?;
        if !meta.is_dir()
            || meta.file_type().is_symlink()
            || root.canonicalize().map_err(|_| stale())? != root
        {
            return Err(stale());
        }
        let identity = VaultIdentity {
            device: meta.dev(),
            inode: meta.ino(),
        };
        // Deterministic read-only scope identity survives process/index loss and is
        // independent of any later writer registration. It grants no write authority.
        let mut hasher = Sha256::new();
        hasher.update(b"brn/citation-review/vault/v1\0");
        hasher.update(root.as_os_str().as_encoded_bytes());
        hasher.update(identity.device.to_be_bytes());
        hasher.update(identity.inode.to_be_bytes());
        let digest = hasher.finalize();
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&digest[..16]);
        Ok(VaultRecord {
            id: Uuid::from_bytes(bytes),
            root: root.to_owned(),
            identity,
        })
    }

    fn citation_review_read(&self, path: &str) -> Result<vault::NoteText> {
        let path = EvidencePath::parse(path).map_err(rejected)?;
        vault::read_evidence(self.require_vault().map_err(|_| stale())?, &path).map_err(|_| stale())
    }

    fn citation_review_observation(&self) -> Result<Observation> {
        let inventory = self.identity_inventory().map_err(|_| stale())?;
        let mut diagnostics = inventory.issues.clone();
        let mut candidates = Vec::new();
        for identity in &inventory.notes {
            let note = self.citation_review_read(&identity.path)?;
            if note.sha256 != identity.sha256 {
                return Err(stale());
            }
            let metadata = saved_metadata(&note.text, &identity.path);
            let issue = metadata.issue.or_else(|| {
                brn_store::work::inbox_source::read_provenance(&note.text)
                    .err()
                    .map(|error| error.to_string())
            });
            if let Some(reason) = issue {
                diagnostics.push(IdentityIssue {
                    path: identity.path.clone(),
                    reason,
                });
            } else if is_current(&note.text, &identity.path) {
                candidates.push(CitationReviewEntry {
                    path: identity.path.clone(),
                    title: title(&note.text, &identity.path),
                    note_id: metadata.note_id,
                    sha256: note.sha256,
                    citations: Vec::new(),
                });
            }
        }
        candidates.sort_by(|a, b| a.path.cmp(&b.path));
        diagnostics.sort_by(|a, b| a.path.cmp(&b.path).then(a.reason.cmp(&b.reason)));
        diagnostics.dedup();
        // Whole-byte hashes bind all managed classification/provenance metadata.
        // Diagnostic changes and candidate membership also invalidate continuation.
        struct Hasher(Sha256);
        impl io::Write for Hasher {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.0.update(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut hasher = Hasher(Sha256::new());
        serde_json::to_writer(&mut hasher, &(&inventory, &candidates, &diagnostics))
            .map_err(|_| stale())?;
        let digest = hasher.0.finalize().into();
        Ok(Observation {
            inventory,
            candidates,
            diagnostics,
            digest,
        })
    }

    fn citation_review_recheck(
        &self,
        observed: &Observation,
        tools: &AiTools,
        epoch: u64,
        vault: &VaultRecord,
    ) -> Result<Observation> {
        self.require_current_evidence().map_err(|_| stale())?;
        tools.check_current_epoch(epoch).map_err(|_| stale())?;
        let fresh = self.citation_review_observation()?;
        if self.citation_review_vault()? != *vault || fresh.digest != observed.digest {
            return Err(stale());
        }
        // Reinspection above freshly reads every consumer/source complete hash,
        // identity and metadata; no size/mtime observation substitutes for bytes.
        self.citation_review_end(tools, epoch, vault)?;
        Ok(fresh)
    }

    fn citation_review_source_proofs(&self, proof: &NoteProvenance) -> Result<()> {
        for citation in &proof.citations {
            for matched in &citation.matches {
                let note = self.citation_review_read(&matched.path)?;
                if note.sha256 != matched.sha256
                    || brn_store::note_identity::read(&note.text).map_err(|_| stale())?
                        != matched.note_id
                {
                    return Err(stale());
                }
            }
        }
        Ok(())
    }

    pub(super) fn citation_review_end(
        &self,
        tools: &AiTools,
        epoch: u64,
        vault: &VaultRecord,
    ) -> Result<()> {
        tools.check_current_epoch(epoch).map_err(|_| stale())?;
        self.require_current_evidence().map_err(|_| stale())?;
        if self.citation_review_vault()? != *vault {
            return Err(stale());
        }
        Ok(())
    }
}

fn response_budget(value: &impl Serialize) -> Result<()> {
    struct Budget(usize);
    impl io::Write for Budget {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0 = self
                .0
                .checked_add(bytes.len())
                .filter(|size| *size <= CITATION_REVIEW_RESPONSE_BYTES)
                .ok_or_else(|| io::Error::other("citation review response exceeds 4 MiB"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Budget(0), value).map_err(|_| {
        rejected(
            "Complete citation review response exceeds the 4 MiB budget; no proof was truncated.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::AppConfig,
        app_worker::{AppCommand, AppEvent, AppWorker},
    };
    use brn_store::{note_identity, note_provenance};
    use std::{fs, path::PathBuf, time::Duration};

    struct Fixture {
        _owner: tempfile::TempDir,
        data: PathBuf,
        vault: PathBuf,
        credentials: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            let data = owner.path().join("data");
            let vault = owner.path().join("vault");
            fs::create_dir(&data).unwrap();
            fs::create_dir(&vault).unwrap();
            Self {
                credentials: owner.path().join("credentials"),
                _owner: owner,
                data,
                vault,
            }
        }
        fn config(&self) -> AppConfig {
            AppConfig {
                vault_root: Some(self.vault.clone()),
                credentials_dir: Some(self.credentials.clone()),
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
        fn source(&self, path: &str) -> super::super::VaultCitation {
            let id = Uuid::new_v4();
            let text = note_identity::assign("# Source\r\nSaved õ 原文 🦀\r\n", id).unwrap();
            let start_byte = text.find("Saved").unwrap();
            let quote = "Saved õ 原文 🦀\r\n";
            self.write(path, &text);
            super::super::VaultCitation {
                note_id: id,
                sha256: Sha256::digest(text.as_bytes()).into(),
                start_byte,
                end_byte: start_byte + quote.len(),
                quote: quote.into(),
            }
        }
        fn consumer(&self, path: &str, citations: &[super::super::VaultCitation]) -> String {
            let text = note_identity::assign("# Consumer\nExact interpretation\n", Uuid::new_v4())
                .unwrap();
            let text = note_provenance::write(&text, citations).unwrap();
            self.write(path, &text);
            text
        }
    }

    #[test]
    fn malformed_inbox_consumer_is_coverage_not_a_page_blocker() {
        let fixture = Fixture::new();
        let broken = "---\nbrn_inbox_source: invalid\n---\n# Broken no-citation Current\n";
        fixture.write("00-broken.md", broken);
        fixture.write("01-healthy.md", "# No citation\n");
        let mut missing = fixture.source("archive/source.md");
        missing.note_id = Uuid::new_v4();
        let original = fixture.consumer("z-affected.md", &[missing]);
        let mut app = fixture.app();
        let first = app
            .citation_review(&CitationReviewRequest {
                limit: 1,
                cursor: None,
            })
            .unwrap();
        assert!(first.entries.is_empty());
        assert_eq!(first.inspected_count, 1);
        assert!(first.coverage.incomplete);
        assert_eq!(first.coverage.diagnostic_count, 1);
        assert_eq!(first.coverage.diagnostics[0].path, "00-broken.md");
        assert!(
            first.coverage.diagnostics[0]
                .reason
                .contains("Inbox provenance")
        );
        let cursor = first.next_cursor.unwrap();
        assert_eq!(cursor.after_path, "01-healthy.md");
        let next = app
            .citation_review(&CitationReviewRequest {
                limit: 1,
                cursor: Some(cursor),
            })
            .unwrap();
        assert_eq!(next.entries.len(), 1);
        assert_eq!(next.entries[0].path, "z-affected.md");
        assert_eq!(
            next.entries[0].citations[0].outcome,
            CitationOutcome::Absent
        );
        assert!(next.next_cursor.is_none());
        let detail = app
            .citation_review_detail(&CitationReviewDetailRequest {
                path: next.entries[0].path.clone(),
                expected_sha256: next.entries[0].sha256,
            })
            .unwrap();
        assert_eq!(detail.text, original);
        assert_eq!(
            fs::read_to_string(fixture.vault.join("00-broken.md")).unwrap(),
            broken
        );
        assert!(app.work_store().proposals(None).unwrap().is_empty());
        assert!(
            app.work_store()
                .findings(&Default::default())
                .unwrap()
                .entries
                .is_empty()
        );
    }

    #[test]
    fn all_nonmatched_outcomes_and_exact_detail_are_read_only() {
        let fixture = Fixture::new();
        let matched = fixture.source("archive/moved.md");
        let changed = fixture.source("changed.md");
        let absent = fixture.source("absent.md");
        let ambiguous = fixture.source("ambiguous.md");
        fs::remove_file(fixture.vault.join("absent.md")).unwrap();
        let text = fs::read_to_string(fixture.vault.join("ambiguous.md")).unwrap();
        fixture.write("duplicate.md", &text);
        let text = fs::read_to_string(fixture.vault.join("changed.md"))
            .unwrap()
            .replace("Saved", "Other");
        fixture.write("changed.md", &text);
        let original = fixture.consumer("current.md", &[matched, changed, absent, ambiguous]);
        let mut app = fixture.app();
        let page = app
            .citation_review(&CitationReviewRequest::default())
            .unwrap();
        let row = page
            .entries
            .iter()
            .find(|row| row.path == "current.md")
            .unwrap();
        assert_eq!(
            row.citations,
            vec![
                CitationReviewIssue {
                    index: 1,
                    outcome: CitationOutcome::Changed
                },
                CitationReviewIssue {
                    index: 2,
                    outcome: CitationOutcome::Absent
                },
                CitationReviewIssue {
                    index: 3,
                    outcome: CitationOutcome::Ambiguous
                },
            ]
        );
        assert!(!page.coverage.incomplete);
        let request = CitationReviewDetailRequest {
            path: row.path.clone(),
            expected_sha256: row.sha256,
        };
        let detail = app.citation_review_detail(&request).unwrap();
        assert_eq!(detail.text, original);
        assert_eq!(
            detail.provenance.citations[0].outcome,
            CitationOutcome::Matched
        );
        detail.validate_for(&request).unwrap();
        assert_eq!(
            fs::read_to_string(fixture.vault.join("current.md")).unwrap(),
            original
        );
        assert!(app.work_store().proposals(None).unwrap().is_empty());
        assert!(
            app.work_store()
                .findings(&Default::default())
                .unwrap()
                .entries
                .is_empty()
        );
    }

    #[test]
    fn current_scope_and_moved_source_reuse_existing_semantics() {
        let fixture = Fixture::new();
        let citation = fixture.source("source.md");
        fs::create_dir(fixture.vault.join("ArChIvE")).unwrap();
        fs::rename(
            fixture.vault.join("source.md"),
            fixture.vault.join("ArChIvE/moved.md"),
        )
        .unwrap();
        fixture.consumer("matched.md", std::slice::from_ref(&citation));
        fixture.consumer("empty.md", &[]);
        let mut absent = citation.clone();
        absent.note_id = Uuid::new_v4();
        fixture.consumer("ordinary.md", std::slice::from_ref(&absent));
        for (path, field) in [
            ("source-consumer.md", "brn_kind: source"),
            ("history-consumer.md", "brn_state: history"),
            ("ArChIvE/consumer.md", ""),
        ] {
            let text = fixture.consumer(path, std::slice::from_ref(&absent));
            let text = text.replacen("---\n", &format!("---\n{field}\n"), 1);
            fixture.write(path, &text);
        }
        let mut app = fixture.app();
        let page = app
            .citation_review(&CitationReviewRequest::default())
            .unwrap();
        assert_eq!(
            page.entries
                .iter()
                .map(|row| row.path.as_str())
                .collect::<Vec<_>>(),
            ["ordinary.md"]
        );
        assert_eq!(page.inspected_count, 3);
        assert!(!page.coverage.incomplete);
    }

    #[test]
    fn unknown_identity_coverage_is_incomplete_not_absent_and_diagnostics_are_bounded() {
        let fixture = Fixture::new();
        let citation = fixture.source("source.md");
        fixture.consumer("current.md", &[citation]);
        fixture.write("broken.md", "---\nbrn_id: invalid\n---\nUnknown identity\n");
        for index in 0..40 {
            fixture.write(
                &format!("bad-{index:02}.md"),
                "---\nbrn_state: invalid\n---\n",
            );
        }
        let mut app = fixture.app();
        let page = app
            .citation_review(&CitationReviewRequest::default())
            .unwrap();
        assert_eq!(
            page.entries[0].citations[0].outcome,
            CitationOutcome::Incomplete
        );
        assert!(page.coverage.incomplete);
        assert_eq!(page.coverage.diagnostics.len(), 32);
        assert_eq!(page.coverage.diagnostic_count, 41);
        assert!(page.coverage.diagnostics_truncated);
        assert_eq!(page.inspected_count, 2);
    }

    #[test]
    fn unrelated_unreadable_evidence_prevents_certified_identity_absence() {
        let fixture = Fixture::new();
        let citation = fixture.source("archive/removed.md");
        fs::remove_file(fixture.vault.join("archive/removed.md")).unwrap();
        fixture.consumer("current.md", &[citation]);
        fixture.write("broken-consumer.md", "---\nbrn_provenance: invalid\n---\n");
        fixture.write("unrelated.md", &"x".repeat(crate::MAX_NOTE_BYTES + 1));
        let mut app = fixture.app();
        let page = app
            .citation_review(&CitationReviewRequest::default())
            .unwrap();
        assert_eq!(page.inspected_count, 1);
        assert_eq!(
            page.entries[0].citations[0].outcome,
            CitationOutcome::Incomplete
        );
        assert!(page.coverage.incomplete);
        assert_eq!(page.coverage.diagnostic_count, 2);
        assert!(
            page.coverage
                .diagnostics
                .iter()
                .any(|issue| issue.path == "broken-consumer.md")
        );
        assert!(
            page.coverage
                .diagnostics
                .iter()
                .any(|issue| issue.path == "unrelated.md")
        );
    }

    #[test]
    fn sparse_pages_advance_last_inspected_and_survive_restart_index_loss() {
        let fixture = Fixture::new();
        fixture.consumer("a.md", &[]);
        fixture.consumer("b.md", &[]);
        let citation = fixture.source("archive/source.md");
        fs::remove_file(fixture.vault.join("archive/source.md")).unwrap();
        fixture.consumer("c.md", &[citation]);
        let mut app = fixture.app();
        let request = CitationReviewRequest {
            limit: 1,
            cursor: None,
        };
        let first = app.citation_review(&request).unwrap();
        assert!(first.entries.is_empty());
        assert_eq!(first.next_cursor.as_ref().unwrap().after_path, "a.md");
        drop(app);
        fs::remove_file(fixture.data.join("index.sqlite")).unwrap();
        let mut app = fixture.app();
        let second = app
            .citation_review(&CitationReviewRequest {
                limit: 1,
                cursor: first.next_cursor,
            })
            .unwrap();
        assert!(second.entries.is_empty());
        assert_eq!(second.next_cursor.as_ref().unwrap().after_path, "b.md");
        let third = app
            .citation_review(&CitationReviewRequest {
                limit: 1,
                cursor: second.next_cursor,
            })
            .unwrap();
        assert_eq!(third.entries[0].path, "c.md");
        assert!(third.next_cursor.is_none());
    }

    #[test]
    fn retained_mtime_same_size_edits_refuse_cursor_and_exact_detail() {
        let fixture = Fixture::new();
        let citation = fixture.source("source.md");
        fixture.consumer("a.md", std::slice::from_ref(&citation));
        fixture.consumer("b.md", &[citation]);
        let mut app = fixture.app();
        let page = app
            .citation_review(&CitationReviewRequest {
                limit: 1,
                cursor: None,
            })
            .unwrap();
        let path = fixture.vault.join("source.md");
        let metadata = fs::metadata(&path).unwrap();
        let original = fs::read_to_string(&path).unwrap();
        fs::write(&path, original.replace("Saved", "Other")).unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(metadata.modified().unwrap()))
            .unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len(), metadata.len());
        assert_eq!(
            app.citation_review(&CitationReviewRequest {
                limit: 1,
                cursor: page.next_cursor
            })
            .unwrap_err()
            .kind,
            ErrorKind::ContextStale
        );
        let page = app
            .citation_review(&CitationReviewRequest::default())
            .unwrap();
        let row = &page.entries[0];
        let request = CitationReviewDetailRequest {
            path: row.path.clone(),
            expected_sha256: row.sha256,
        };
        let path = fixture.vault.join(&row.path);
        let text = fs::read_to_string(&path).unwrap().replace("Exact", "Other");
        fs::write(path, text).unwrap();
        assert_eq!(
            app.citation_review_detail(&request).unwrap_err().kind,
            ErrorKind::ContextStale
        );
    }

    #[test]
    fn reobservation_refuses_source_consumer_epoch_root_and_unknown_durable_changes() {
        for change in ["source", "consumer", "epoch", "root", "uncertain"] {
            let fixture = Fixture::new();
            let citation = fixture.source("source.md");
            fixture.consumer("current.md", &[citation]);
            let mut app = fixture.app();
            let (tools, epoch, vault) = app.citation_review_boundary().unwrap();
            let observed = app.citation_review_observation().unwrap();
            match change {
                "source" => fixture.write("source.md", "# Changed source\n"),
                "consumer" => fixture.write("current.md", "# Changed consumer\n"),
                "epoch" => {
                    tools.set_current_blocked(true);
                    tools.set_current_blocked(false);
                }
                "root" => {
                    fs::rename(&fixture.vault, fixture.vault.with_extension("old")).unwrap();
                    fs::create_dir(&fixture.vault).unwrap();
                }
                "uncertain" => app.completion_uncertain = true,
                _ => unreachable!(),
            }
            assert_eq!(
                app.citation_review_recheck(&observed, &tools, epoch, &vault)
                    .err()
                    .unwrap()
                    .kind,
                ErrorKind::ContextStale,
                "{change}"
            );
        }
    }

    #[test]
    fn cursor_root_digest_and_missing_candidate_mismatch_refuse() {
        let fixture = Fixture::new();
        fixture.consumer("a.md", &[]);
        fixture.consumer("b.md", &[]);
        let mut app = fixture.app();
        let page = app
            .citation_review(&CitationReviewRequest {
                limit: 1,
                cursor: None,
            })
            .unwrap();
        let mut encoded = serde_json::to_value(page.next_cursor.clone().unwrap()).unwrap();
        encoded["vault"]["unexpected"] = true.into();
        assert!(serde_json::from_value::<CitationReviewCursor>(encoded).is_err());
        for change in ["digest", "root", "candidate"] {
            let mut cursor = page.next_cursor.clone().unwrap();
            match change {
                "digest" => cursor.observation_digest[0] ^= 1,
                "root" => cursor.vault.identity.inode += 1,
                "candidate" => cursor.after_path = "absent.md".into(),
                _ => unreachable!(),
            }
            assert_eq!(
                app.citation_review(&CitationReviewRequest {
                    limit: 1,
                    cursor: Some(cursor)
                })
                .unwrap_err()
                .kind,
                ErrorKind::ContextStale
            );
        }
    }

    #[test]
    fn pure_validators_reject_unbound_counts_issues_detail_and_oversized_complete_proof() {
        let fixture = Fixture::new();
        let citation = fixture.source("source.md");
        fixture.consumer("current.md", &[citation]);
        fixture.write("source.md", "changed");
        let mut app = fixture.app();
        let request = CitationReviewRequest::default();
        let page = app.citation_review(&request).unwrap();
        for change in ["count", "matched", "index", "coverage"] {
            let mut bad = page.clone();
            match change {
                "count" => bad.inspected_count = 101,
                "matched" => bad.entries[0].citations[0].outcome = CitationOutcome::Matched,
                "index" => bad.entries[0].citations[0].index = 32,
                "coverage" => bad.coverage.incomplete = true,
                _ => unreachable!(),
            }
            assert!(bad.validate_for(&request).is_err(), "{change}");
        }
        assert!(
            CitationReviewRequest {
                limit: 0,
                cursor: None
            }
            .validate()
            .is_err()
        );
        assert!(
            CitationReviewDetailRequest {
                path: "archive/current.md".into(),
                expected_sha256: [0; 32]
            }
            .validate()
            .is_err()
        );
        let detail_request = CitationReviewDetailRequest {
            path: page.entries[0].path.clone(),
            expected_sha256: page.entries[0].sha256,
        };
        let detail = app.citation_review_detail(&detail_request).unwrap();
        let mut bad = detail.clone();
        bad.text.push('!');
        assert!(bad.validate_for(&detail_request).is_err());
        let mut oversized = detail.clone();
        oversized.provenance.citations[0]
            .issues
            .push(IdentityIssue {
                path: "source.md".into(),
                reason: "x".repeat(CITATION_REVIEW_RESPONSE_BYTES),
            });
        assert!(oversized.validate_for(&detail_request).is_err());
        let mut bad = detail;
        bad.provenance.path = "other.md".into();
        assert!(bad.validate_for(&detail_request).is_err());
        assert!(response_budget(&"x".repeat(CITATION_REVIEW_RESPONSE_BYTES)).is_err());
    }

    #[test]
    fn worker_dispatch_returns_bound_page_and_detail() {
        let fixture = Fixture::new();
        let citation = fixture.source("source.md");
        fixture.consumer("current.md", &[citation]);
        fixture.write("source.md", "changed");
        let mut worker = AppWorker::start(fixture.data.clone(), fixture.config()).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(20))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        let id = Uuid::new_v4();
        worker
            .submit(
                id,
                AppCommand::CitationReview(CitationReviewRequest::default()),
            )
            .unwrap();
        let (reply, event) = worker.recv_event_timeout(Duration::from_secs(20)).unwrap();
        assert_eq!(reply, id);
        let AppEvent::CitationReview(page) = event else {
            panic!("expected citation page")
        };
        let request = CitationReviewDetailRequest {
            path: page.entries[0].path.clone(),
            expected_sha256: page.entries[0].sha256,
        };
        let id = Uuid::new_v4();
        worker
            .submit(id, AppCommand::CitationReviewDetail(request.clone()))
            .unwrap();
        let (reply, event) = worker.recv_event_timeout(Duration::from_secs(20)).unwrap();
        assert_eq!(reply, id);
        let AppEvent::CitationReviewDetail(detail) = event else {
            panic!("expected citation detail")
        };
        detail.validate_for(&request).unwrap();
        worker.shutdown().unwrap();
    }
}
