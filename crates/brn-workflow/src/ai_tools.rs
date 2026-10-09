//! Fresh, read-only vault tools. Index text is never trusted without validation.
use crate::library::{
    KnowledgeScope, LibraryError, NoteHit, SearchMode, SharedEmbedder, saved_metadata,
    search_index_scoped,
};
use crate::vault::{self, EvidencePath, VaultPath};
use brn_ai::{
    AiError, AiErrorKind, AiResult, ConflictKnowledge, NoteEntry, NoteFacts, NotePage,
    NoteRangeRequest, Passage, RawEvidence, RawEvidenceRequest, ReadScope, ReadTools, ToolNote,
    ToolNoteRange, ToolSearch,
};
use brn_retrieval::note_index::{IndexedNote, NoteIndexReader};
use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::Mutex,
};

pub struct AiTools {
    root: PathBuf,
    raw_root_identity: Option<brn_store::files::VaultIdentity>,
    current_fence: Mutex<(u64, bool)>,
    reader: Mutex<NoteIndexReader>,
    embedder: Option<SharedEmbedder>,
}

fn rejected() -> AiError {
    AiError::new(AiErrorKind::ToolRejected)
}
fn stale() -> AiError {
    AiError::new(AiErrorKind::IndexStale)
}

impl AiTools {
    pub fn open(
        root: &Path,
        index: &Path,
        embedder: Option<SharedEmbedder>,
    ) -> crate::Result<Self> {
        Ok(Self {
            root: root.to_owned(),
            raw_root_identity: root
                .symlink_metadata()
                .ok()
                .filter(|meta| meta.is_dir() && !meta.file_type().is_symlink())
                .map(|meta| brn_store::files::VaultIdentity {
                    device: meta.dev(),
                    inode: meta.ino(),
                }),
            current_fence: Mutex::new((0, false)),
            reader: Mutex::new(NoteIndexReader::open(index)?),
            embedder,
        })
    }

    pub(crate) fn set_current_blocked(&self, blocked: bool) {
        let mut fence = self.current_fence.lock().expect("owned evidence fence");
        fence.0 = fence.0.checked_add(1).expect("evidence epoch exhausted");
        fence.1 = blocked;
    }

    pub(crate) fn check_current_epoch(&self, epoch: u64) -> AiResult<()> {
        let fence = self.current_fence.lock().map_err(|_| stale())?;
        if fence.1 || fence.0 != epoch {
            return Err(stale());
        }
        Ok(())
    }

    pub(crate) fn check_root(&self) -> AiResult<u64> {
        let epoch = {
            let fence = self.current_fence.lock().map_err(|_| stale())?;
            if fence.1 {
                return Err(stale());
            }
            fence.0
        };
        if self
            .root
            .symlink_metadata()
            .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
        {
            Ok(epoch)
        } else {
            Err(stale())
        }
    }

    fn check_raw_root(&self) -> AiResult<()> {
        let meta = self.root.symlink_metadata().map_err(|_| stale())?;
        let current = brn_store::files::VaultIdentity {
            device: meta.dev(),
            inode: meta.ino(),
        };
        if meta.is_dir()
            && !meta.file_type().is_symlink()
            && self.raw_root_identity.as_ref() == Some(&current)
        {
            Ok(())
        } else {
            Err(stale())
        }
    }

    fn entries(&self, scope: KnowledgeScope) -> AiResult<Vec<IndexedNote>> {
        self.reader
            .lock()
            .map_err(|_| AiError::new(AiErrorKind::Storage))?
            .notes_scoped(scope)
            .map_err(|_| stale())
    }
}

fn knowledge_scope(scope: ReadScope) -> KnowledgeScope {
    match scope {
        ReadScope::Current => KnowledgeScope::Current,
        ReadScope::Source => KnowledgeScope::Source,
        ReadScope::History => KnowledgeScope::History,
        ReadScope::All => KnowledgeScope::All,
    }
}

fn read_path(path: &str, scope: KnowledgeScope) -> AiResult<EvidencePath> {
    if scope == KnowledgeScope::Current {
        VaultPath::parse(path).map_err(|_| rejected())?;
    }
    EvidencePath::parse(path).map_err(|_| rejected())
}

fn checked_facts(
    text: &str,
    path: &str,
    scope: KnowledgeScope,
    sha256: [u8; 32],
) -> AiResult<NoteFacts> {
    let metadata = saved_metadata(text, path);
    if metadata.issue.is_some() || !scope.includes(metadata.source, metadata.history) {
        Err(stale())
    } else {
        Ok(NoteFacts {
            note_id: metadata.note_id.map(|id| id.to_string()),
            sha256,
            source: metadata.source,
            history: metadata.history,
            conflicts: ConflictKnowledge::Unknown,
        })
    }
}

pub(crate) fn validate_hits_scoped(
    root: &Path,
    hits: &[NoteHit],
    scope: KnowledgeScope,
) -> AiResult<()> {
    visit_hits_scoped(root, hits, scope, |_| {})
}

fn visit_hits_scoped(
    root: &Path,
    hits: &[NoteHit],
    scope: KnowledgeScope,
    mut visit: impl FnMut(NoteFacts),
) -> AiResult<()> {
    for hit in hits {
        let path = read_path(&hit.path, scope)?;
        let note = vault::read_evidence(root, &path).map_err(|_| stale())?;
        let facts = checked_facts(&note.text, &hit.path, scope, note.sha256)?;
        if note.sha256 != hit.note_sha256
            || note.text.get(hit.start_byte..hit.end_byte) != Some(hit.quote.as_str())
        {
            return Err(stale());
        }
        visit(facts);
    }
    Ok(())
}

pub(crate) fn note_page_scoped(
    root: &Path,
    entries: Vec<IndexedNote>,
    folder: Option<&str>,
    cursor: Option<&str>,
    scope: KnowledgeScope,
) -> AiResult<NotePage> {
    if let Some(folder) = folder {
        if scope == KnowledgeScope::Current {
            VaultPath::validate_folder(folder).map_err(|_| rejected())?;
        } else {
            EvidencePath::validate_folder(folder).map_err(|_| rejected())?;
        }
    }
    if let Some(cursor) = cursor {
        read_path(cursor, scope)?;
    }
    let mut notes = Vec::new();
    for entry in entries {
        let path = read_path(&entry.path, scope)?;
        if folder.is_some_and(|folder| {
            !entry
                .path
                .strip_prefix(folder)
                .is_some_and(|suffix| suffix.starts_with('/'))
        }) || cursor.is_some_and(|cursor| entry.path.as_str() <= cursor)
        {
            continue;
        }
        let current = vault::read_evidence(root, &path).map_err(|_| stale())?;
        let facts = checked_facts(&current.text, &entry.path, scope, current.sha256)?;
        if current.sha256 != entry.sha256 {
            return Err(stale());
        }
        notes.push(NoteEntry {
            path: entry.path,
            title: entry.title,
            facts,
        });
        if notes.len() == 201 {
            break;
        }
    }
    let next_cursor = if notes.len() > 200 {
        notes.pop();
        notes.last().map(|note| note.path.clone())
    } else {
        None
    };
    Ok(NotePage { notes, next_cursor })
}

impl ReadTools for AiTools {
    fn read_raw_evidence(&self, request: &RawEvidenceRequest) -> AiResult<RawEvidence> {
        request.validate()?;
        let path = EvidencePath::parse(&request.path).map_err(|_| rejected())?;
        let epoch = self.check_root()?;
        self.check_raw_root()?;
        let note = vault::read_evidence(&self.root, &path).map_err(|_| rejected())?;
        if request
            .expected_sha256
            .is_some_and(|hash| hash != note.sha256)
        {
            return Err(stale());
        }
        let remaining = note.text.get(request.start_byte..).ok_or_else(rejected)?;
        let end_byte = request
            .end_byte
            .unwrap_or_else(|| request.start_byte + brn_ai::capped_text(remaining).0.len());
        let text = note
            .text
            .get(request.start_byte..end_byte)
            .ok_or_else(rejected)?;
        let metadata_issue = saved_metadata(&note.text, &request.path).issue.or_else(|| {
            brn_store::work::inbox_source::read_provenance(&note.text)
                .err()
                .map(|error| error.to_string())
        });
        let facts = if metadata_issue.is_none() {
            Some(checked_facts(
                &note.text,
                &request.path,
                KnowledgeScope::All,
                note.sha256,
            )?)
        } else {
            None
        };
        let reply = RawEvidence {
            path: request.path.clone(),
            sha256: note.sha256,
            start_byte: request.start_byte,
            end_byte,
            total_bytes: note.text.len(),
            text: text.to_owned(),
            partial: request.start_byte > 0 || end_byte < note.text.len(),
            facts,
            metadata_issue,
        };
        self.check_current_epoch(epoch)?;
        self.check_raw_root()?;
        reply.validate_for(request)?;
        Ok(reply)
    }

    fn search_notes(&self, query: &str, limit: usize) -> AiResult<ToolSearch> {
        self.search_notes_scoped(query, limit, ReadScope::Current)
    }

    fn search_notes_scoped(
        &self,
        query: &str,
        limit: usize,
        scope: ReadScope,
    ) -> AiResult<ToolSearch> {
        if !(1..=10).contains(&limit) {
            return Err(rejected());
        }
        brn_retrieval::note_index::check_query(query, limit).map_err(|_| rejected())?;
        let scope = knowledge_scope(scope);
        let epoch = self.check_root()?;
        let mut embedder = self.embedder.clone();
        let results = {
            let reader = self
                .reader
                .lock()
                .map_err(|_| AiError::new(AiErrorKind::Storage))?;
            search_index_scoped(
                &*reader,
                embedder
                    .as_mut()
                    .map(|e| e as &mut dyn crate::library::Embedder),
                query,
                SearchMode::Hybrid,
                limit,
                scope,
            )
            .map_err(|error| match error {
                LibraryError::Index(brn_retrieval::Error::Invalid(_)) => rejected(),
                LibraryError::Index(brn_retrieval::Error::ModelMismatch) => stale(),
                LibraryError::Index(brn_retrieval::Error::Corrupt(_)) => stale(),
                _ => AiError::new(AiErrorKind::Storage),
            })?
        };
        let mut facts = Vec::with_capacity(results.hits.len());
        visit_hits_scoped(&self.root, &results.hits, scope, |fact| facts.push(fact))?;
        self.check_current_epoch(epoch)?;
        Ok(ToolSearch {
            hits: results
                .hits
                .into_iter()
                .zip(facts)
                .map(|(hit, facts)| Passage {
                    path: hit.path,
                    start_byte: hit.start_byte,
                    end_byte: hit.end_byte,
                    quote: hit.quote,
                    facts,
                })
                .collect(),
            keyword_only: results.keyword_only,
        })
    }

    fn read_note(&self, path: &str) -> AiResult<ToolNote> {
        self.read_note_scoped(path, ReadScope::Current)
    }

    fn read_note_scoped(&self, path: &str, scope: ReadScope) -> AiResult<ToolNote> {
        let scope = knowledge_scope(scope);
        let parsed = read_path(path, scope)?;
        let epoch = self.check_root()?;
        let note = vault::read_evidence(&self.root, &parsed).map_err(|_| rejected())?;
        let facts = checked_facts(&note.text, path, scope, note.sha256).map_err(|_| rejected())?;
        let (text, truncated) = brn_ai::capped_text(&note.text);
        self.check_current_epoch(epoch)?;
        Ok(ToolNote {
            path: path.to_owned(),
            text: text.to_owned(),
            truncated,
            facts,
        })
    }

    fn read_note_range(&self, request: &NoteRangeRequest) -> AiResult<ToolNoteRange> {
        request.validate()?;
        let scope = knowledge_scope(request.scope);
        let parsed = read_path(&request.path, scope)?;
        let epoch = self.check_root()?;
        let note = vault::read_evidence(&self.root, &parsed).map_err(|_| stale())?;
        if note.sha256 != request.expected_sha256 {
            return Err(stale());
        }
        let facts =
            checked_facts(&note.text, &request.path, scope, note.sha256).map_err(|_| rejected())?;
        let text = note
            .text
            .get(request.start_byte..request.end_byte)
            .ok_or_else(rejected)?;
        self.check_current_epoch(epoch)?;
        Ok(ToolNoteRange {
            path: request.path.clone(),
            start_byte: request.start_byte,
            end_byte: request.end_byte,
            total_bytes: note.text.len(),
            text: text.to_owned(),
            facts,
        })
    }

    fn list_notes(&self, folder: Option<&str>, cursor: Option<&str>) -> AiResult<NotePage> {
        self.list_notes_scoped(folder, cursor, ReadScope::Current)
    }

    fn list_notes_scoped(
        &self,
        folder: Option<&str>,
        cursor: Option<&str>,
        scope: ReadScope,
    ) -> AiResult<NotePage> {
        let scope = knowledge_scope(scope);
        let epoch = self.check_root()?;
        let page = note_page_scoped(&self.root, self.entries(scope)?, folder, cursor, scope)?;
        self.check_current_epoch(epoch)?;
        Ok(page)
    }
}
