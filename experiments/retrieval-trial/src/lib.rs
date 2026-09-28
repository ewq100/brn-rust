//! Disposable, synthetic-only retrieval contract. Source versions are authoritative;
//! all ranking backends receive the same prefiltered immutable passages.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid request: {0}")]
    InvalidRequest(&'static str),
    #[error("evidence provenance mismatch: {0}")]
    Provenance(&'static str),
    #[error("native retrieval failure: {0}")]
    Native(String),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Keyword,
    Semantic,
    Hybrid,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Approval {
    Approved,
    Draft,
    Withdrawn,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub source_id: String,
    pub version_id: String,
    pub approval: Approval,
    pub current: bool,
    pub text: String,
    pub source_hash: String,
}
impl Document {
    pub fn new(
        source_id: &str,
        version_id: &str,
        approval: Approval,
        current: bool,
        text: &str,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            version_id: version_id.into(),
            approval,
            current,
            text: text.into(),
            source_hash: hash(text.as_bytes()),
        }
    }
    pub fn verify(&self) -> Result<()> {
        if self.source_id.is_empty() || self.version_id.is_empty() {
            return Err(Error::Provenance("empty source/version id"));
        }
        if hash(self.text.as_bytes()) != self.source_hash {
            return Err(Error::Provenance("source hash"));
        }
        Ok(())
    }
}
pub fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filters {
    pub source_ids: Option<Vec<String>>,
    pub version_ids: Option<Vec<String>>,
    /// Default true; historical versions require explicit opt-in.
    pub current_only: bool,
    pub approvals: Vec<Approval>,
}
impl Default for Filters {
    fn default() -> Self {
        Self {
            source_ids: None,
            version_ids: None,
            current_only: true,
            approvals: vec![Approval::Approved],
        }
    }
}
impl Filters {
    pub fn validate(&self) -> Result<()> {
        for (name, list) in [
            ("source_ids", &self.source_ids),
            ("version_ids", &self.version_ids),
        ] {
            if let Some(ids) = list {
                if ids.is_empty() || ids.iter().any(|s| s.trim().is_empty()) {
                    return Err(Error::InvalidRequest("empty source/version filter"));
                }
                let distinct: HashSet<_> = ids.iter().collect();
                if distinct.len() != ids.len() {
                    return Err(Error::InvalidRequest("duplicate source/version filter"));
                }
                let _ = name;
            }
        }
        if self.approvals.is_empty() {
            return Err(Error::InvalidRequest("approval filter must be nonempty"));
        }
        if self.approvals.iter().collect::<HashSet<_>>().len() != self.approvals.len() {
            return Err(Error::InvalidRequest("duplicate approval filter"));
        }
        Ok(())
    }
    pub fn allows(&self, doc: &Document) -> bool {
        (!self.current_only || doc.current)
            && self.approvals.contains(&doc.approval)
            && self
                .source_ids
                .as_ref()
                .is_none_or(|ids| ids.contains(&doc.source_id))
            && self
                .version_ids
                .as_ref()
                .is_none_or(|ids| ids.contains(&doc.version_id))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub query: String,
    pub profile: Profile,
    pub limit: usize,
    pub filters: Filters,
}
impl Request {
    pub fn new(query: &str, profile: Profile, limit: usize, filters: Filters) -> Result<Self> {
        let request = Self {
            query: query.trim().into(),
            profile,
            limit,
            filters,
        };
        request.validate()?;
        Ok(request)
    }
    pub fn validate(&self) -> Result<()> {
        if self.query.trim().is_empty() || self.query.len() > 512 {
            return Err(Error::InvalidRequest("query length must be 1..512 bytes"));
        }
        if !(1..=100).contains(&self.limit) {
            return Err(Error::InvalidRequest("limit must be 1..100"));
        }
        self.filters.validate()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub passage_id: String,
    pub score_kind: ScoreKind,
    pub source_id: String,
    pub version_id: String,
    pub source_hash: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
    pub score: f32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreKind {
    KeywordFraction,
    CosineSimilarity,
    ReciprocalRankFusion,
}
impl Evidence {
    pub fn from_document(doc: &Document, score: f32, score_kind: ScoreKind) -> Self {
        Self {
            passage_id: "p0".into(),
            score_kind,
            source_id: doc.source_id.clone(),
            version_id: doc.version_id.clone(),
            source_hash: doc.source_hash.clone(),
            start_byte: 0,
            end_byte: doc.text.len(),
            quote: doc.text.clone(),
            score,
        }
    }
    pub fn verify(&self, docs: &[Document]) -> Result<()> {
        let doc = docs
            .iter()
            .find(|d| d.source_id == self.source_id && d.version_id == self.version_id)
            .ok_or(Error::Provenance("source/version missing"))?;
        doc.verify()?;
        if self.passage_id != "p0" || !self.score.is_finite() {
            return Err(Error::Provenance("passage or score"));
        }
        if self.source_hash != doc.source_hash {
            return Err(Error::Provenance("source hash"));
        }
        if !doc.text.is_char_boundary(self.start_byte)
            || !doc.text.is_char_boundary(self.end_byte)
            || self.start_byte >= self.end_byte
        {
            return Err(Error::Provenance("quote offsets"));
        }
        if doc.text.get(self.start_byte..self.end_byte) != Some(self.quote.as_str()) {
            return Err(Error::Provenance("quote bytes"));
        }
        Ok(())
    }
}
pub fn eligible<'a>(docs: &'a [Document], filters: &Filters) -> Result<Vec<&'a Document>> {
    filters.validate()?;
    docs.iter()
        .map(Document::verify)
        .collect::<Result<Vec<_>>>()?;
    let mut seen = HashSet::new();
    if docs
        .iter()
        .any(|d| !seen.insert((d.source_id.as_str(), d.version_id.as_str())))
    {
        return Err(Error::Provenance("duplicate source/version identity"));
    }
    Ok(docs.iter().filter(|d| filters.allows(d)).collect())
}
pub fn search_keyword(docs: &[Document], request: &Request) -> Result<Vec<Evidence>> {
    request.validate()?;
    if request.profile != Profile::Keyword {
        return Err(Error::InvalidRequest(
            "keyword adapter requires keyword profile",
        ));
    }
    let terms: Vec<String> = request
        .query
        .split_whitespace()
        .map(str::to_lowercase)
        .collect();
    let mut hits: Vec<Evidence> = eligible(docs, &request.filters)?
        .into_iter()
        .filter_map(|doc| {
            let lower = doc.text.to_lowercase();
            let matches = terms
                .iter()
                .filter(|term| lower.contains(term.as_str()))
                .count();
            (matches > 0).then(|| {
                Evidence::from_document(
                    doc,
                    matches as f32 / terms.len() as f32,
                    ScoreKind::KeywordFraction,
                )
            })
        })
        .collect();
    hits.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(a.source_id.cmp(&b.source_id))
            .then(a.version_id.cmp(&b.version_id))
    });
    hits.truncate(request.limit);
    Ok(hits)
}
pub fn fuse(a: &[Evidence], b: &[Evidence], limit: usize) -> Vec<Evidence> {
    let mut merged: Vec<Evidence> = Vec::new();
    for (list, _) in [(a, 0), (b, 1)] {
        for (rank, hit) in list.iter().enumerate() {
            let contribution = 1.0 / (60.0 + rank as f32 + 1.0);
            if let Some(existing) = merged.iter_mut().find(|x| {
                x.source_id == hit.source_id
                    && x.version_id == hit.version_id
                    && x.source_hash == hit.source_hash
                    && x.start_byte == hit.start_byte
                    && x.end_byte == hit.end_byte
            }) {
                existing.score += contribution;
                existing.score_kind = ScoreKind::ReciprocalRankFusion;
            } else {
                let mut new = hit.clone();
                new.score = contribution;
                new.score_kind = ScoreKind::ReciprocalRankFusion;
                merged.push(new);
            }
        }
    }
    merged.sort_by(|x, y| {
        y.score
            .total_cmp(&x.score)
            .then(x.source_id.cmp(&y.source_id))
    });
    merged.truncate(limit);
    merged
}
pub fn fixtures() -> Vec<Document> {
    vec![
        Document::new(
            "launch-current",
            "v1",
            Approval::Approved,
            false,
            "The launch window was March. BRN-482 remained open.",
        ),
        Document::new(
            "launch-current",
            "v2",
            Approval::Approved,
            true,
            "The launch window is October. BRN-482 is closed.",
        ),
        Document::new(
            "draft-brief",
            "v1",
            Approval::Draft,
            true,
            "Launch window is January. BRN-482 draft.",
        ),
        Document::new(
            "withdrawn-brief",
            "v1",
            Approval::Withdrawn,
            true,
            "Launch window is February. BRN-482 withdrawn.",
        ),
        Document::new(
            "unicode-note",
            "v1",
            Approval::Approved,
            true,
            "The café note uses naïve résumé text and emoji 🧭.",
        ),
        Document::new(
            "identifier-note",
            "v1",
            Approval::Approved,
            true,
            "BRN-482 is the exact identifier for the release checklist.",
        ),
        Document::new(
            "duplicate-note",
            "v1",
            Approval::Approved,
            true,
            "A duplicate release checklist mentions BRN-482.",
        ),
    ]
}

#[cfg(feature = "native")]
pub mod native;
