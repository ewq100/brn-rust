//! Deliberate private text copies through the shared application boundary.
use crate::{
    ErrorKind, Result, WorkflowError,
    app::App,
    files::inbox::{InboxFiles, InboxRoot, receipt_id},
};
pub use brn_store::work::inbox::{InboxItem, InboxKind, InboxListRequest, MAX_INBOX_BINARY_BYTES};
use brn_store::{WorkStore, work::inbox::InboxCapture};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const ROOT_SETTING: &str = "inbox.root";
const MAX_ISSUES: usize = 100;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureInboxRequest {
    pub id: Uuid,
    pub kind: InboxKind,
    pub title: String,
    pub original_name: Option<String>,
    pub text: String,
}
impl CaptureInboxRequest {
    pub fn validate(&self) -> Result<()> {
        if self.kind == InboxKind::Binary {
            return Err(WorkflowError::msg(
                "Binary Inbox capture requires an explicit byte request",
            ));
        }
        if self.id.is_nil() || self.text.len() > crate::MAX_NOTE_BYTES {
            return Err(WorkflowError::msg(
                "Inbox needs a nonnil UUID and exact UTF-8 text up to 1 MiB",
            ));
        }
        for value in std::iter::once(&self.title).chain(self.original_name.iter()) {
            if value.trim().is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
                return Err(WorkflowError::msg(
                    "Inbox labels must be visible and at most 512 UTF-8 bytes",
                ));
            }
        }
        Ok(())
    }
    pub fn validate_receipt(&self, item: &InboxItem) -> Result<()> {
        self.validate()?;
        item.validate()?;
        if !self.matches(item) {
            return Err(WorkflowError::typed(
                ErrorKind::OperationConflict,
                "Inbox receipt differs from the complete explicit input",
            ));
        }
        Ok(())
    }
    fn matches(&self, item: &InboxItem) -> bool {
        let c = &item.capture;
        self.id == c.id
            && self.kind == c.kind
            && self.title == c.title
            && self.original_name == c.original_name
            && self.text.len() as u64 == c.copy.byte_len
            && <[u8; 32]>::from(Sha256::digest(self.text.as_bytes())) == c.copy.sha256
    }
}
/// Explicit byte retention, without conversion or an inferred document type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureBinaryInboxRequest {
    pub id: Uuid,
    pub title: String,
    pub original_name: Option<String>,
    pub bytes: Vec<u8>,
}
impl CaptureBinaryInboxRequest {
    pub fn validate(&self) -> Result<()> {
        if self.id.is_nil() || self.bytes.len() > MAX_INBOX_BINARY_BYTES {
            return Err(WorkflowError::msg(
                "Binary Inbox needs a nonnil UUID and exact bytes up to 16 MiB",
            ));
        }
        for value in std::iter::once(&self.title).chain(self.original_name.iter()) {
            if value.trim().is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
                return Err(WorkflowError::msg(
                    "Inbox labels must be visible and at most 512 UTF-8 bytes",
                ));
            }
        }
        Ok(())
    }
    pub fn validate_receipt(&self, item: &InboxItem) -> Result<()> {
        self.validate()?;
        item.validate()?;
        if !matches_capture(
            item,
            self.id,
            InboxKind::Binary,
            &self.title,
            &self.original_name,
            &self.bytes,
        ) {
            return Err(WorkflowError::typed(
                ErrorKind::OperationConflict,
                "Inbox receipt differs from the complete explicit input",
            ));
        }
        Ok(())
    }
}
fn matches_capture(
    item: &InboxItem,
    id: Uuid,
    kind: InboxKind,
    title: &str,
    original_name: &Option<String>,
    bytes: &[u8],
) -> bool {
    let c = &item.capture;
    id == c.id
        && kind == c.kind
        && title == c.title
        && original_name == &c.original_name
        && bytes.len() as u64 == c.copy.byte_len
        && <[u8; 32]>::from(Sha256::digest(bytes)) == c.copy.sha256
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxAvailability {
    Available,
    RemovedRetained,
    Missing,
    Changed,
    Unavailable,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum InboxOriginal {
    Available { text: String },
    AvailableBinary { byte_len: u64, sha256: [u8; 32] },
    RemovedRetained { operation_id: Uuid },
    Missing,
    Changed { reason: String },
    Unavailable { reason: String },
}
impl InboxOriginal {
    fn availability(&self) -> InboxAvailability {
        match self {
            Self::Available { .. } | Self::AvailableBinary { .. } => InboxAvailability::Available,
            Self::RemovedRetained { .. } => InboxAvailability::RemovedRetained,
            Self::Missing => InboxAvailability::Missing,
            Self::Changed { .. } => InboxAvailability::Changed,
            Self::Unavailable { .. } => InboxAvailability::Unavailable,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxRead {
    pub item: InboxItem,
    pub original: InboxOriginal,
}
impl InboxRead {
    /// DTO consistency only; fresh filesystem availability is established by App.
    pub fn validate_receipt(&self) -> Result<()> {
        self.item.validate()?;
        let c = &self.item.capture;
        match &self.original {
            InboxOriginal::Available { text } => CaptureInboxRequest {
                id: c.id,
                kind: c.kind,
                title: c.title.clone(),
                original_name: c.original_name.clone(),
                text: text.clone(),
            }
            .validate_receipt(&self.item),
            InboxOriginal::AvailableBinary { byte_len, sha256 } => {
                if c.kind != InboxKind::Binary
                    || *byte_len != c.copy.byte_len
                    || *sha256 != c.copy.sha256
                {
                    return Err(WorkflowError::typed(
                        ErrorKind::OperationConflict,
                        "Binary original response differs from its complete capture",
                    ));
                }
                Ok(())
            }
            InboxOriginal::RemovedRetained { .. } if c.kind == InboxKind::Binary => Err(
                WorkflowError::msg("Binary Inbox original removal is not supported"),
            ),
            _ => Ok(()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxEntry {
    pub item: InboxItem,
    pub availability: InboxAvailability,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxIssue {
    pub item_id: Option<Uuid>,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxInventory {
    pub entries: Vec<InboxEntry>,
    pub next_after: Option<Uuid>,
    pub total_count: usize,
    pub issues: Vec<InboxIssue>,
    pub issues_truncated: bool,
}
#[derive(Default)]
pub(crate) struct InboxState {
    pub(crate) files: Option<InboxFiles>,
    removed: std::collections::HashMap<Uuid, crate::inbox_original_operations::RemovedOriginal>,
    issues: Vec<InboxIssue>,
    truncated: bool,
}
impl InboxState {
    fn issue(&mut self, id: Option<Uuid>, message: impl Into<String>) {
        if self.issues.len() == MAX_ISSUES {
            self.truncated = true;
            return;
        }
        let mut message = message.into();
        if message.len() > 512 {
            let mut end = 512;
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            message.truncate(end);
        }
        let issue = InboxIssue {
            item_id: id,
            message,
        };
        if !self.issues.contains(&issue) {
            self.issues.push(issue);
        }
    }
    pub(crate) fn original(&self, item: &InboxItem) -> InboxOriginal {
        let Some(files) = &self.files else {
            return InboxOriginal::Unavailable {
                reason: "owned Inbox originals are unavailable; inspect Inbox issues".into(),
            };
        };
        if let Some(removed) = self.removed.get(&item.capture.id) {
            return match files.retained_copy(item, removed.operation_id, &removed.namespace) {
                Ok(()) => match files.original_occupied(item) {
                    Ok(false) => InboxOriginal::RemovedRetained {
                        operation_id: removed.operation_id,
                    },
                    Ok(true) => InboxOriginal::Changed {
                        reason:
                            "removed original endpoint is occupied; retained copy remains untouched"
                                .into(),
                    },
                    Err(error) => InboxOriginal::Unavailable {
                        reason: error.message,
                    },
                },
                Err(error) => InboxOriginal::Unavailable {
                    reason: error.message,
                },
            };
        }
        if item.capture.kind == InboxKind::Binary {
            return match files.read_binary(item) {
                Ok(Some(proof)) => InboxOriginal::AvailableBinary {
                    byte_len: proof.len,
                    sha256: proof.sha256,
                },
                Ok(None) => InboxOriginal::Missing,
                Err(e) if e.kind == ErrorKind::ContextStale => {
                    InboxOriginal::Changed { reason: e.message }
                }
                Err(e) => InboxOriginal::Unavailable { reason: e.message },
            };
        }
        match files.read(item) {
            Ok(Some(text)) => InboxOriginal::Available { text },
            Ok(None) => InboxOriginal::Missing,
            Err(e) if e.kind == ErrorKind::ContextStale => {
                InboxOriginal::Changed { reason: e.message }
            }
            Err(e) => InboxOriginal::Unavailable { reason: e.message },
        }
    }
}
fn bind(store: &mut WorkStore, root: &InboxRoot) -> Result<()> {
    let bytes = serde_json::to_string(root)
        .map_err(|_| WorkflowError::msg("could not encode Inbox directory binding"))?;
    store.set_setting(ROOT_SETTING, &bytes)?;
    Ok(())
}
/// Startup recovers only exact known explicit-user captures. Namespace failures
/// stay visible in Inbox and do not fence unrelated current vault knowledge.
/// Restore immutable evidence before analysis/application companions that reference it.
/// This never recovers/removes an original or asserts an approval endpoint.
pub(crate) fn restore_intake_snapshot_mirrors(store: &mut WorkStore) -> Result<()> {
    let raw = store.setting(ROOT_SETTING)?;
    let bound = match raw
        .as_deref()
        .map(serde_json::from_str::<InboxRoot>)
        .transpose()
    {
        Ok(bound) => bound,
        Err(_) => return Ok(()),
    };
    if let (Some(raw), Some(bound)) = (&raw, &bound) {
        if serde_json::to_string(bound).ok().as_ref() != Some(raw) {
            return Ok(());
        }
    }
    let files = match InboxFiles::open(store.data_dir(), bound.as_ref(), false) {
        Ok(Some(files)) => files,
        _ => return Ok(()),
    };
    // Inventory admission failures remain Inbox-local, as in capture recovery.
    // Referenced snapshots are still required by contextual Store validation.
    let names = match files.names() {
        Ok(names) => names,
        Err(_) => return Ok(()),
    };
    for name in names {
        let Some(raw) = name
            .strip_prefix(".brn-intake-")
            .and_then(|name| name.strip_suffix(".snapshot"))
        else {
            continue;
        };
        let id = Uuid::parse_str(raw)
            .map_err(|_| WorkflowError::msg("invalid extraction snapshot filename"))?;
        if id.to_string() != raw {
            return Err(WorkflowError::msg("noncanonical extraction filename"));
        }
        store.restore_intake_snapshot(&files.read_snapshot(id)?)?;
    }
    Ok(())
}

pub(crate) fn restore_inbox_captures(store: &mut WorkStore) -> Result<InboxState> {
    let mut state = InboxState::default();
    let raw = store.setting(ROOT_SETTING)?;
    let bound = match raw
        .as_deref()
        .map(serde_json::from_str::<InboxRoot>)
        .transpose()
    {
        Ok(bound) => bound,
        Err(_) => {
            state.issue(
                None,
                "Inbox directory binding is malformed; originals remain retained",
            );
            return Ok(state);
        }
    };
    if let (Some(raw), Some(bound)) = (&raw, &bound)
        && serde_json::to_string(bound).ok().as_ref() != Some(raw)
    {
        state.issue(
            None,
            "Inbox directory binding differs from its complete canonical shape",
        );
        return Ok(state);
    }
    let files = match InboxFiles::open(store.data_dir(), bound.as_ref(), false) {
        Ok(Some(files)) => files,
        Ok(None) => return Ok(state),
        Err(e) => {
            state.issue(None, e.message);
            return Ok(state);
        }
    };
    let names = match files.names() {
        Ok(names) => names,
        Err(e) => {
            state.issue(None, e.message);
            return Ok(state);
        }
    };
    let operations = match crate::inbox_original_operations::restore_records(store, &files, &names)
    {
        Ok(operations) => operations,
        Err(error) => {
            state.issue(None, error.message);
            // No ordinary capture recovery may resurrect an original while its
            // checked operation family or mirror is damaged or ambiguous.
            return Ok(state);
        }
    };
    let mut qualified = bound.is_some() || !operations.heads.is_empty();
    if bound.is_none() && qualified {
        bind(store, files.root())?;
    }
    let mut known = operations.known;
    state.removed = operations.removed;
    for name in &names {
        let Some(id) = receipt_id(name) else {
            continue;
        };
        let recovered = (|| -> Result<()> {
            let item = files.mirror(id)?;
            if store
                .inbox_item(id)?
                .is_some_and(|existing| existing != item)
            {
                return Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "Inbox mirror conflicts with the retained immutable capture",
                ));
            }
            // A pending or settled original-operation head always fences stage
            // installation. Missing ordinary bytes alone never undo removal.
            if !operations.heads.contains_key(&id) {
                files.recover(&item)?;
            }
            store.restore_inbox(&item)?;
            known.insert(name.clone());
            known.insert(item.capture.copy_name());
            if !qualified {
                bind(store, files.root())?;
                qualified = true;
            }
            Ok(())
        })();
        if let Err(e) = recovered {
            state.issue(Some(id), e.message);
        }
    }
    for name in &names {
        let Some(raw) = name
            .strip_prefix(".brn-intake-")
            .and_then(|name| name.strip_suffix(".snapshot"))
        else {
            continue;
        };
        let restored = (|| -> Result<()> {
            let id = Uuid::parse_str(raw)
                .map_err(|_| WorkflowError::msg("invalid retained snapshot filename"))?;
            if id.to_string() != raw {
                return Err(WorkflowError::msg("noncanonical snapshot filename"));
            }
            let snapshot = files.read_snapshot(id)?;
            store.restore_intake_snapshot(&snapshot)?;
            known.insert(name.clone());
            Ok(())
        })();
        if let Err(error) = restored {
            state.issue(None, error.message);
        }
    }
    let final_names = match files.names() {
        Ok(names) => names,
        Err(e) => {
            state.issue(None, e.message);
            names
        }
    };
    for name in final_names {
        if !known.contains(&name) {
            // Only names are observed here; unknown artifact contents stay unread.
            state.issue(
                receipt_id(&name),
                format!("retained unqualified Inbox artifact: {name}"),
            );
        }
    }
    if qualified {
        state.files = Some(files);
    } else {
        state.issue(
            None,
            "unbound Inbox namespace has no fully qualified capture; artifacts remain retained",
        );
    }
    Ok(state)
}
/// Complete retained review evidence and a fresh observation of the original.
/// This read never approves semantic completeness or authorizes removal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxReview {
    pub manifest: brn_store::work::inbox_review::InboxReviewManifest,
    pub digest: [u8; 32],
    pub original: InboxOriginal,
    pub needs_semantic_review: bool,
}
impl App {
    /// Explicit user copy, independent of a vault/model/approval. Retained replay
    /// is an immutable receipt, not a claim that original bytes remain available.
    pub fn capture_inbox(&mut self, request: &CaptureInboxRequest) -> Result<InboxItem> {
        request.validate()?;
        self.capture_inbox_bytes(
            request.id,
            request.kind,
            &request.title,
            &request.original_name,
            request.text.as_bytes(),
        )
    }
    pub fn capture_binary_inbox(
        &mut self,
        request: &CaptureBinaryInboxRequest,
    ) -> Result<InboxItem> {
        request.validate()?;
        self.capture_inbox_bytes(
            request.id,
            InboxKind::Binary,
            &request.title,
            &request.original_name,
            &request.bytes,
        )
    }
    fn capture_inbox_bytes(
        &mut self,
        id: Uuid,
        kind: InboxKind,
        title: &str,
        original_name: &Option<String>,
        bytes: &[u8],
    ) -> Result<InboxItem> {
        let retained = |item: InboxItem| {
            if matches_capture(&item, id, kind, title, original_name, bytes) {
                Ok(item)
            } else {
                Err(WorkflowError::typed(
                    ErrorKind::OperationConflict,
                    "Inbox UUID already identifies another exact copy request",
                ))
            }
        };
        if let Some(item) = self.store.inbox_item(id)? {
            return retained(item);
        }
        self.inbox = restore_inbox_captures(&mut self.store)?;
        if let Some(item) = self.store.inbox_item(id)? {
            return retained(item);
        }
        if self.inbox.files.is_none() {
            if !self.inbox.issues.is_empty() {
                return Err(WorkflowError::typed(
                    ErrorKind::InboxUnavailable,
                    "retained Inbox artifacts require inspection before fresh capture",
                ));
            }
            let files = InboxFiles::open(self.store.data_dir(), None, true)?.ok_or_else(|| {
                WorkflowError::typed(ErrorKind::InboxUnavailable, "Inbox namespace did not open")
            })?;
            if let Err(e) = bind(&mut self.store, files.root()) {
                return Err(WorkflowError::typed(
                    ErrorKind::InboxUncertain,
                    format!("Inbox directory was prepared but not acknowledged: {e}"),
                ));
            }
            self.inbox.files = Some(files);
        }
        let files = self
            .inbox
            .files
            .as_ref()
            .expect("capture owns its checked namespace");
        let prepared = if kind == InboxKind::Binary {
            files.prepare_binary(id, bytes)
        } else {
            files.prepare(
                id,
                std::str::from_utf8(bytes).expect("validated text capture"),
            )
        };
        let copy = match prepared {
            Ok(copy) => copy,
            Err(e) => {
                self.inbox.issue(Some(id), e.message.clone());
                return Err(WorkflowError::typed(
                    ErrorKind::InboxUncertain,
                    format!("Inbox capture was not acknowledged; artifacts remain retained: {e}"),
                ));
            }
        };
        let capture = InboxCapture {
            id,
            kind,
            title: title.to_owned(),
            original_name: original_name.clone(),
            copy,
        };
        let result = self.store.capture_inbox_with(&capture, |item| {
            files
                .publish(item)
                .map_err(|e| brn_store::Error::Invalid(e.message))
        });
        match result {
            Ok(item) => {
                crate::files::inbox::checkpoint("catalog_settled")?;
                Ok(item)
            }
            Err(e) => {
                self.inbox.issue(Some(id), e.to_string());
                Err(WorkflowError::typed(
                    ErrorKind::InboxUncertain,
                    format!(
                        "Inbox capture was not acknowledged; exact original and recovery artifacts remain retained: {e}"
                    ),
                ))
            }
        }
    }
    pub fn inbox_review(&self, id: Uuid) -> Result<InboxReview> {
        let manifest = self.store.inbox_review_manifest(id)?;
        let review = InboxReview {
            digest: manifest.digest()?,
            original: self.inbox.original(&manifest.original),
            manifest,
            needs_semantic_review: true,
        };
        if serde_json::to_vec(&review)
            .map_err(|_| WorkflowError::msg("could not encode complete Inbox review"))?
            .len()
            > brn_store::work::inbox_review::MAX_INBOX_REVIEW_BYTES
        {
            return Err(WorkflowError::msg(
                "complete Inbox review exceeds its encoded bound",
            ));
        }
        Ok(review)
    }
    pub fn inbox_item(&self, id: Uuid) -> Result<InboxRead> {
        if id.is_nil() {
            return Err(WorkflowError::msg("Inbox UUID must not be nil"));
        }
        let item = self.store.inbox_item(id)?.ok_or_else(|| {
            WorkflowError::typed(ErrorKind::NotFound, "Inbox item does not exist")
        })?;
        let original = self.inbox.original(&item);
        Ok(InboxRead { item, original })
    }
    pub fn inbox_items(&self, request: &InboxListRequest) -> Result<InboxInventory> {
        let page = self.store.inbox_items(request)?;
        let entries = page
            .entries
            .into_iter()
            .map(|item| InboxEntry {
                availability: self.inbox.original(&item).availability(),
                item,
            })
            .collect();
        Ok(InboxInventory {
            entries,
            next_after: page.next_after,
            total_count: page.total_count,
            issues: self.inbox.issues.clone(),
            issues_truncated: self.inbox.truncated,
        })
    }
}
#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::{app::AppConfig, files::inbox::FAULT};
    use std::{fs, path::PathBuf};
    fn request() -> CaptureInboxRequest {
        CaptureInboxRequest {
            id: Uuid::new_v4(),
            kind: InboxKind::Teams,
            title: "Retain exact original λ".into(),
            original_name: None,
            text: "\u{feff}Source õ\r\n日本語".into(),
        }
    }
    fn config(data: &std::path::Path) -> AppConfig {
        AppConfig {
            vault_root: None,
            credentials_dir: Some(data.parent().unwrap().join("credentials")),
            model_dir: None,
        }
    }
    #[test]
    fn publication_failures_reconcile_known_proof_without_republishing_or_losing_original_time() {
        for step in ["mirror_durable", "original_installed", "catalog_settled"] {
            let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            let data = base.path().join("data");
            fs::create_dir(&data).unwrap();
            let r = request();
            let mut app = App::open(&data, config(&data)).unwrap();
            FAULT.with(|f| f.set(Some((step, false))));
            let failure = app.capture_inbox(&r).unwrap_err();
            FAULT.with(|f| f.set(None));
            assert_eq!(failure.kind, ErrorKind::InboxUncertain);
            let mirror = fs::read(
                data.join("inbox")
                    .join(format!(".brn-inbox-{}.receipt", r.id)),
            )
            .unwrap();
            let retained = app.inbox.files.as_ref().unwrap().mirror(r.id).unwrap();
            if step == "catalog_settled" {
                assert_eq!(app.store.inbox_item(r.id).unwrap(), Some(retained.clone()));
            } else {
                assert!(app.store.inbox_item(r.id).unwrap().is_none());
            }
            drop(app);
            let mut app = App::open(&data, config(&data)).unwrap();
            assert_eq!(app.capture_inbox(&r).unwrap(), retained);
            assert_eq!(
                app.inbox_item(r.id).unwrap().original,
                InboxOriginal::Available {
                    text: r.text.clone()
                }
            );
            assert_eq!(
                fs::read(
                    data.join("inbox")
                        .join(format!(".brn-inbox-{}.receipt", r.id))
                )
                .unwrap(),
                mirror
            );
            assert!(
                app.inbox_items(&InboxListRequest::default())
                    .unwrap()
                    .issues
                    .is_empty()
            );
        }
    }
    #[test]
    fn unproved_stage_after_failure_stays_retained_and_another_explicit_copy_can_progress() {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = base.path().join("data");
        fs::create_dir(&data).unwrap();
        let r = request();
        let mut app = App::open(&data, config(&data)).unwrap();
        FAULT.with(|f| f.set(Some(("original_durable", false))));
        assert_eq!(
            app.capture_inbox(&r).unwrap_err().kind,
            ErrorKind::InboxUncertain
        );
        FAULT.with(|f| f.set(None));
        drop(app);
        let path = data
            .join("inbox")
            .join(format!(".brn-inbox-{}.stage", r.id));
        let bytes = fs::read(&path).unwrap();
        let mut app = App::open(&data, config(&data)).unwrap();
        assert!(app.inbox_item(r.id).is_err());
        assert!(
            !app.inbox_items(&InboxListRequest::default())
                .unwrap()
                .issues
                .is_empty()
        );
        assert_eq!(
            app.capture_inbox(&r).unwrap_err().kind,
            ErrorKind::InboxUncertain
        );
        let other = request();
        let item = app.capture_inbox(&other).unwrap();
        assert_ne!(item.capture.id, r.id);
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    #[test]
    fn equal_bytes_in_an_unknown_stage_never_authorize_installation() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = base.path().join("data");
        fs::create_dir(&data).unwrap();
        let r = request();
        let mut app = App::open(&data, config(&data)).unwrap();
        FAULT.with(|f| f.set(Some(("mirror_durable", false))));
        assert!(app.capture_inbox(&r).is_err());
        FAULT.with(|f| f.set(None));
        let item = app.inbox.files.as_ref().unwrap().mirror(r.id).unwrap();
        drop(app);
        let root = data.join("inbox");
        let stage = root.join(format!(".brn-inbox-{}.stage", r.id));
        let retained = root.join("retained-unknown-original");
        fs::rename(&stage, &retained).unwrap();
        fs::write(&stage, &r.text).unwrap();
        fs::set_permissions(&stage, fs::Permissions::from_mode(0o600)).unwrap();
        assert_ne!(
            fs::metadata(&stage).unwrap().ino(),
            item.capture.copy.file_inode
        );
        let mut app = App::open(&data, config(&data)).unwrap();
        assert!(app.inbox_item(r.id).is_err());
        assert_eq!(
            app.capture_inbox(&r).unwrap_err().kind,
            ErrorKind::InboxUncertain
        );
        assert!(!root.join(item.capture.copy_name()).exists());
        assert_eq!(fs::read(stage).unwrap(), r.text.as_bytes());
        assert_eq!(fs::read(retained).unwrap(), r.text.as_bytes());
        assert!(
            !app.inbox_items(&InboxListRequest::default())
                .unwrap()
                .issues
                .is_empty()
        );
    }
    #[test]
    fn malformed_or_forked_mirrors_keep_catalog_original_bytes_and_the_damaged_artifact() {
        for kind in [
            "hash",
            "time_fork",
            "unknown_field",
            "missing_field",
            "format",
        ] {
            let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            let data = base.path().join("data");
            fs::create_dir(&data).unwrap();
            let r = request();
            let mut app = App::open(&data, config(&data)).unwrap();
            let item = app.capture_inbox(&r).unwrap();
            drop(app);
            let path = data
                .join("inbox")
                .join(format!(".brn-inbox-{}.receipt", r.id));
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            match kind {
                "hash" => value["sha256"] = serde_json::json!(vec![0; 32]),
                "time_fork" => {
                    value["item"]["received_at_ms"] = serde_json::json!(item.received_at_ms + 1);
                    let changed: InboxItem = serde_json::from_value(value["item"].clone()).unwrap();
                    value["sha256"] = serde_json::json!(<[u8; 32]>::from(Sha256::digest(
                        serde_json::to_vec(&changed).unwrap()
                    )));
                }
                "unknown_field" => value["other"] = serde_json::json!(true),
                "missing_field" => {
                    value["item"]["capture"]
                        .as_object_mut()
                        .unwrap()
                        .remove("original_name");
                }
                "format" => value["format"] = serde_json::json!(2),
                _ => unreachable!(),
            }
            #[derive(Serialize)]
            struct CanonicalMirror {
                format: u8,
                sha256: [u8; 32],
                item: InboxItem,
            }
            let damaged = if kind == "time_fork" {
                let changed: InboxItem = serde_json::from_value(value["item"].clone()).unwrap();
                let sha256 = Sha256::digest(serde_json::to_vec(&changed).unwrap()).into();
                serde_json::to_vec(&CanonicalMirror {
                    format: 1,
                    sha256,
                    item: changed,
                })
                .unwrap()
            } else {
                serde_json::to_vec(&value).unwrap()
            };
            fs::write(&path, &damaged).unwrap();
            let mut app = App::open(&data, config(&data)).unwrap();
            assert_eq!(app.capture_inbox(&r).unwrap(), item);
            assert_eq!(
                app.inbox_item(r.id).unwrap().original,
                InboxOriginal::Available {
                    text: r.text.clone()
                }
            );
            let issues = app
                .inbox_items(&InboxListRequest::default())
                .unwrap()
                .issues;
            assert!(!issues.is_empty());
            if kind == "time_fork" {
                assert!(issues.iter().any(|issue| {
                    issue
                        .message
                        .contains("conflicts with the retained immutable capture")
                }));
            }
            assert_eq!(fs::read(path).unwrap(), damaged);
            assert_eq!(
                fs::read(data.join("inbox").join(item.capture.copy_name())).unwrap(),
                r.text.as_bytes()
            );
        }
    }
    #[test]
    fn unknown_artifacts_are_bounded_visible_and_never_read_adopted_or_deleted() {
        use std::os::unix::fs::PermissionsExt;
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = base.path().join("data");
        fs::create_dir(&data).unwrap();
        let root = data.join("inbox");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let unknown = root.join("unknown-original");
        fs::write(&unknown, b"unknown body remains unread").unwrap();
        let mut app = App::open(&data, config(&data)).unwrap();
        assert_eq!(
            app.capture_inbox(&request()).unwrap_err().kind,
            ErrorKind::InboxUnavailable
        );
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        assert_eq!(fs::read(&unknown).unwrap(), b"unknown body remains unread");
        drop(app);
        // Separate already-bound namespace can keep processing unrelated explicit copies.
        let other = base.path().join("other");
        fs::create_dir(&other).unwrap();
        let mut app = App::open(&other, config(&other)).unwrap();
        let r = request();
        let item = app.capture_inbox(&r).unwrap();
        drop(app);
        for index in 0..101 {
            fs::write(
                other.join("inbox").join(format!("unknown-{index:03}")),
                b"unparsed content",
            )
            .unwrap();
        }
        let app = App::open(&other, config(&other)).unwrap();
        let page = app.inbox_items(&InboxListRequest::default()).unwrap();
        assert_eq!(page.issues.len(), 100);
        assert!(page.issues_truncated);
        assert_eq!(page.total_count, 1);
        assert_eq!(page.entries[0].item, item);
        assert_eq!(page.entries[0].availability, InboxAvailability::Available);
        for index in 0..101 {
            assert_eq!(
                fs::read(other.join("inbox").join(format!("unknown-{index:03}"))).unwrap(),
                b"unparsed content"
            );
        }
    }
    #[test]
    fn process_crash_windows_reconcile_only_the_known_whole_capture() {
        let _fixtures = crate::SUBPROCESS_FIXTURES.lock().unwrap();
        for step in [
            "original_durable",
            "mirror_durable",
            "original_installed",
            "catalog_settled",
        ] {
            let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            let data = base.path().join("data");
            fs::create_dir(&data).unwrap();
            let r = request();
            let input = base.path().join("input.json");
            fs::write(&input, serde_json::to_vec(&r).unwrap()).unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "inbox::tests::crash_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("BRN_INBOX_SYNTHETIC_DATA", &data)
                .env("BRN_INBOX_SYNTHETIC_INPUT", &input)
                .env("BRN_INBOX_SYNTHETIC_STEP", step)
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(91),
                "{step}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let mut app = App::open(&data, config(&data)).unwrap();
            if step == "original_durable" {
                assert!(app.inbox_item(r.id).is_err());
                assert!(
                    !app.inbox_items(&InboxListRequest::default())
                        .unwrap()
                        .issues
                        .is_empty()
                );
                assert_eq!(
                    fs::read(
                        data.join("inbox")
                            .join(format!(".brn-inbox-{}.stage", r.id))
                    )
                    .unwrap(),
                    r.text.as_bytes()
                );
            } else {
                let item = app.inbox_item(r.id).unwrap();
                let retained = app.inbox.files.as_ref().unwrap().mirror(r.id).unwrap();
                assert_eq!(item.item, retained);
                assert_eq!(
                    item.original,
                    InboxOriginal::Available {
                        text: r.text.clone()
                    }
                );
                assert_eq!(app.capture_inbox(&r).unwrap(), retained);
                assert!(
                    !data
                        .join("inbox")
                        .join(format!(".brn-inbox-{}.stage", r.id))
                        .exists()
                );
            }
        }
    }
    #[test]
    #[ignore = "invoked by the parent process-crash witness with an exclusive synthetic fixture"]
    fn crash_child() {
        let Some(data) = std::env::var_os("BRN_INBOX_SYNTHETIC_DATA") else {
            return;
        };
        let data = PathBuf::from(data);
        let input = PathBuf::from(std::env::var_os("BRN_INBOX_SYNTHETIC_INPUT").unwrap());
        assert_eq!(input.parent(), data.parent());
        assert_eq!(data.file_name().unwrap(), "data");
        let request: CaptureInboxRequest =
            serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
        let step = match std::env::var("BRN_INBOX_SYNTHETIC_STEP").unwrap().as_str() {
            "original_durable" => "original_durable",
            "mirror_durable" => "mirror_durable",
            "original_installed" => "original_installed",
            "catalog_settled" => "catalog_settled",
            _ => panic!("unknown private test checkpoint"),
        };
        let mut app = App::open(&data, config(&data)).unwrap();
        FAULT.with(|f| f.set(Some((step, true))));
        let _ = app.capture_inbox(&request);
        panic!("checkpoint did not terminate the child");
    }
}

#[cfg(test)]
#[path = "inbox_binary_tests.rs"]
mod binary_tests;
