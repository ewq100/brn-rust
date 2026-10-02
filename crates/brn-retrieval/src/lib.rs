//! Immutable, rebuildable retrieval generations. Callers supply the authoritative
//! eligible revisions at build time and must revalidate every hit before use.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid retrieval input: {0}")]
    Invalid(&'static str),
    #[error("retrieval generation is incomplete or corrupt: {0}")]
    Corrupt(&'static str),
    #[error("retrieval profile unavailable: {0}")]
    Unavailable(&'static str),
    #[error("retrieval build cancelled")]
    Cancelled,
    #[error("retrieval I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("retrieval format: {0}")]
    Format(#[from] serde_json::Error),
    #[error("native retrieval: {0}")]
    Native(String),
    #[error("index database: {0}")]
    Sql(#[from] rusqlite::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Keyword,
    Semantic,
    Hybrid,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub source_id: String,
    pub version_id: String,
    pub title: String,
    pub text: String,
    pub source_hash: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub source_id: String,
    pub version_id: String,
    pub source_hash: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
    pub passage_id: String,
    pub generation: String,
    pub score: f32,
    pub score_kind: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Chunk {
    doc: usize,
    start: usize,
    end: usize,
    passage_id: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    format: u32,
    chunker: String,
    generation: String,
    fingerprint: String,
    documents_hash: String,
    chunks_hash: String,
    native: bool,
    model_identity: Option<String>,
    model_files: BTreeMap<String, String>,
    db_files: BTreeMap<String, String>,
}
const CHUNKER: &str = "utf8-1600-v1";
const MAX_DOCUMENT: usize = 1_048_576;
const FORMAT: u32 = 1;
const SEMANTIC_MOVED: &str = "semantic search moved to the note index";

pub fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn validate_documents(docs: &[Document]) -> Result<()> {
    let mut seen = HashSet::new();
    for doc in docs {
        if doc.source_id.is_empty() || doc.version_id.is_empty() || doc.text.len() > MAX_DOCUMENT {
            return Err(Error::Invalid("document identity or length"));
        }
        if hash(doc.text.as_bytes()) != doc.source_hash {
            return Err(Error::Invalid("document source hash"));
        }
        if !seen.insert((&doc.source_id, &doc.version_id)) {
            return Err(Error::Invalid("duplicate source/version"));
        }
    }
    Ok(())
}
fn chunk_documents(docs: &[Document], cancel: &AtomicBool) -> Result<Vec<Chunk>> {
    let mut chunks = Vec::new();
    for (doc_index, doc) in docs.iter().enumerate() {
        check_cancel(cancel)?;
        for (start, end) in chunk::passages(&doc.text) {
            chunks.push(Chunk {
                doc: doc_index,
                start,
                end,
                passage_id: format!(
                    "{CHUNKER}:{}:{}:{start}:{end}",
                    doc.source_id, doc.version_id
                ),
            });
        }
    }
    Ok(chunks)
}
fn write_sync(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = File::create(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}
fn sync_dir(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}
fn validate_chunks(docs: &[Document], chunks: &[Chunk]) -> Result<()> {
    let expected = chunk_documents(docs, &AtomicBool::new(false))?;
    if expected != chunks {
        return Err(Error::Corrupt("passage map mismatch"));
    }
    Ok(())
}

pub struct Index {
    path: PathBuf,
    docs: Vec<Document>,
    chunks: Vec<Chunk>,
    manifest: Manifest,
}
impl Index {
    /// Builds in a new directory only; `COMPLETE` is written and synced last.
    /// Each document is limited to 1 MiB. Semantic search moved to
    /// [`note_index`], so `model_dir` must be `None`.
    pub fn build(
        path: &Path,
        documents: &[Document],
        model_dir: Option<&Path>,
        cancel: &AtomicBool,
        mut on_progress: impl FnMut(&str),
    ) -> Result<Self> {
        validate_documents(documents)?;
        check_cancel(cancel)?;
        if model_dir.is_some() {
            return Err(Error::Unavailable(SEMANTIC_MOVED));
        }
        fs::create_dir(path)?;
        on_progress("chunking");
        let chunks = chunk_documents(documents, cancel)?;
        let docs_bytes = serde_json::to_vec(documents)?;
        let chunks_bytes = serde_json::to_vec(&chunks)?;
        write_sync(&path.join("documents.json"), &docs_bytes)?;
        write_sync(&path.join("chunks.json"), &chunks_bytes)?;
        check_cancel(cancel)?;
        let manifest = Manifest {
            format: FORMAT,
            chunker: CHUNKER.into(),
            generation: Uuid::new_v4().to_string(),
            fingerprint: hash(&docs_bytes),
            documents_hash: hash(&docs_bytes),
            chunks_hash: hash(&chunks_bytes),
            native: false,
            model_identity: None,
            model_files: BTreeMap::new(),
            db_files: BTreeMap::new(),
        };
        let manifest_bytes = serde_json::to_vec(&manifest)?;
        write_sync(&path.join("manifest.json"), &manifest_bytes)?;
        sync_dir(path)?;
        check_cancel(cancel)?;
        on_progress("publishing");
        write_sync(&path.join("COMPLETE"), hash(&manifest_bytes).as_bytes())?;
        sync_dir(path)?;
        if let Some(parent) = path.parent() {
            sync_dir(parent)?;
        }
        Self::open(path)
    }
    /// Opens only a fully published, hash-verified generation.
    pub fn open(path: &Path) -> Result<Self> {
        if !path.is_dir() || !path.join("COMPLETE").is_file() {
            return Err(Error::Corrupt("missing COMPLETE marker"));
        }
        let manifest_bytes = fs::read(path.join("manifest.json"))?;
        if fs::read(path.join("COMPLETE"))? != hash(&manifest_bytes).as_bytes() {
            return Err(Error::Corrupt("manifest marker hash"));
        }
        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
        if manifest.format != FORMAT
            || manifest.chunker != CHUNKER
            || Uuid::parse_str(&manifest.generation).is_err()
        {
            return Err(Error::Corrupt("incompatible manifest"));
        }
        let docs_bytes = fs::read(path.join("documents.json"))?;
        let chunks_bytes = fs::read(path.join("chunks.json"))?;
        if hash(&docs_bytes) != manifest.documents_hash
            || hash(&chunks_bytes) != manifest.chunks_hash
            || manifest.fingerprint != manifest.documents_hash
        {
            return Err(Error::Corrupt("snapshot hash mismatch"));
        }
        let docs: Vec<Document> = serde_json::from_slice(&docs_bytes)?;
        validate_documents(&docs).map_err(|_| Error::Corrupt("document provenance"))?;
        let chunks: Vec<Chunk> = serde_json::from_slice(&chunks_bytes)?;
        validate_chunks(&docs, &chunks)?;
        // The document snapshot is independently usable. Native assets are
        // checked when a semantic query first needs them.
        if !manifest.native
            && (manifest.model_identity.is_some()
                || !manifest.model_files.is_empty()
                || !manifest.db_files.is_empty())
        {
            return Err(Error::Corrupt("unexpected native manifest"));
        }
        Ok(Self {
            path: path.to_path_buf(),
            docs,
            chunks,
            manifest,
        })
    }
    pub fn generation(&self) -> &str {
        &self.manifest.generation
    }
    pub fn fingerprint(&self) -> &str {
        &self.manifest.fingerprint
    }
    pub fn documents(&self) -> &[Document] {
        &self.docs
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn search(&mut self, query: &str, profile: Profile, limit: usize) -> Result<Vec<Evidence>> {
        if query.trim().is_empty() || query.len() > 512 || !(1..=100).contains(&limit) {
            return Err(Error::Invalid("query length or limit"));
        }
        let terms = keyword_terms(query);
        if terms.is_empty() {
            return Err(Error::Invalid("query has no search terms"));
        }
        let hits = match profile {
            Profile::Keyword => self.keyword(&terms, limit),
            Profile::Semantic => self.semantic(query, limit)?,
            Profile::Hybrid => {
                let lexical = self.keyword(&terms, 100);
                let semantic = self.semantic(query, 100)?;
                fuse(&lexical, &semantic, limit)
            }
        };
        for hit in &hits {
            self.verify_hit(hit)?;
        }
        Ok(hits)
    }
    fn keyword(&self, terms: &[String], limit: usize) -> Vec<Evidence> {
        let mut hits = Vec::new();
        for chunk in &self.chunks {
            let doc = &self.docs[chunk.doc];
            let quote = &doc.text[chunk.start..chunk.end];
            let lower = quote.to_lowercase();
            let matches = terms
                .iter()
                .filter(|term| lower.contains(term.as_str()))
                .count();
            if matches > 0 {
                hits.push(self.hit(
                    chunk,
                    matches as f32 / terms.len() as f32,
                    "keyword_fraction",
                ));
            }
        }
        hits.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then(a.passage_id.cmp(&b.passage_id))
        });
        hits.truncate(limit);
        hits
    }
    fn semantic(&mut self, _query: &str, _limit: usize) -> Result<Vec<Evidence>> {
        Err(Error::Unavailable(SEMANTIC_MOVED))
    }
    fn hit(&self, chunk: &Chunk, score: f32, kind: &str) -> Evidence {
        let doc = &self.docs[chunk.doc];
        Evidence {
            source_id: doc.source_id.clone(),
            version_id: doc.version_id.clone(),
            source_hash: doc.source_hash.clone(),
            start_byte: chunk.start,
            end_byte: chunk.end,
            quote: doc.text[chunk.start..chunk.end].into(),
            passage_id: chunk.passage_id.clone(),
            generation: self.manifest.generation.clone(),
            score,
            score_kind: kind.into(),
        }
    }
    pub fn verify_hit(&self, hit: &Evidence) -> Result<()> {
        if !hit.score.is_finite() || hit.generation != self.manifest.generation {
            return Err(Error::Corrupt("hit score or generation"));
        }
        let chunk = self
            .chunks
            .iter()
            .find(|c| c.passage_id == hit.passage_id)
            .ok_or(Error::Corrupt("missing hit passage"))?;
        let doc = &self.docs[chunk.doc];
        if hit.source_id != doc.source_id
            || hit.version_id != doc.version_id
            || hit.source_hash != doc.source_hash
            || hit.start_byte != chunk.start
            || hit.end_byte != chunk.end
            || doc.text.get(chunk.start..chunk.end) != Some(hit.quote.as_str())
        {
            return Err(Error::Corrupt("hit provenance mismatch"));
        }
        Ok(())
    }
}
fn keyword_terms(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_lowercase)
        .collect()
}
fn fuse(a: &[Evidence], b: &[Evidence], limit: usize) -> Vec<Evidence> {
    let mut merged: Vec<Evidence> = Vec::new();
    for list in [a, b] {
        for (rank, hit) in list.iter().enumerate() {
            let contribution = 1.0 / (60.0 + rank as f32 + 1.0);
            if let Some(existing) = merged.iter_mut().find(|x| x.passage_id == hit.passage_id) {
                existing.score += contribution;
                existing.score_kind = "rrf".into();
            } else {
                let mut item = hit.clone();
                item.score = contribution;
                item.score_kind = "rrf".into();
                merged.push(item);
            }
        }
    }
    merged.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(a.passage_id.cmp(&b.passage_id))
    });
    merged.truncate(limit);
    merged
}
#[cfg(feature = "native")]
pub mod native;

mod chunk;
pub mod note_index;
