use super::search::{HitRow, to_hit};
use super::{NoteHit, NoteIndex};
use crate::{Error, Result};
use rusqlite::{OptionalExtension, params};

/// Turns text into vectors. Implemented by the local model and by test fakes.
pub trait Embedder {
    /// Stable identity of the model and its settings; a change rebuilds all embeddings.
    fn identity(&self) -> &str;
    fn dimension(&self) -> usize;
    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddingProgress {
    pub embedded: usize,
    pub total: usize,
}

fn unit(vector: &[f32]) -> Result<Vec<f32>> {
    if vector.iter().any(|x| !x.is_finite()) {
        return Err(Error::Invalid("embedding is not finite"));
    }
    let norm = vector
        .iter()
        .map(|x| f64::from(*x).powi(2))
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() {
        return Err(Error::Invalid("embedding is not finite"));
    }
    if norm == 0.0 {
        return Err(Error::Invalid("embedding is all zeros"));
    }
    let normalised: Vec<f32> = vector
        .iter()
        .map(|x| (f64::from(*x) / norm) as f32)
        .collect();
    if normalised.iter().any(|x| !x.is_finite()) {
        return Err(Error::Invalid("embedding is not finite"));
    }
    Ok(normalised)
}

fn to_blob(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn from_blob(bytes: &[u8], dimension: usize) -> Result<Vec<f32>> {
    if bytes.len() != dimension * 4 {
        return Err(Error::Corrupt("stored embedding size"));
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect())
}

impl NoteIndex {
    fn embedding_model(&self) -> Result<Option<(String, usize)>> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'embedding_model'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        value
            .map(|value| {
                let (identity, dimension) = value
                    .rsplit_once('|')
                    .ok_or(Error::Corrupt("embedding model record"))?;
                let dimension = dimension
                    .parse()
                    .map_err(|_| Error::Corrupt("embedding model record"))?;
                Ok((identity.to_owned(), dimension))
            })
            .transpose()
    }

    /// Records the embedding model in use. A different identity or dimension
    /// deletes every stored embedding.
    pub fn use_embedding_model(&mut self, identity: &str, dimension: usize) -> Result<()> {
        if identity.is_empty() || identity.contains('|') || dimension == 0 {
            return Err(Error::Invalid("embedding model identity or dimension"));
        }
        if self
            .embedding_model()?
            .is_some_and(|(i, d)| i == identity && d == dimension)
        {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM embeddings", [])?;
        tx.execute(
            "INSERT INTO meta(key, value) VALUES ('embedding_model', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [format!("{identity}|{dimension}")],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn embedding_progress(&self) -> Result<EmbeddingProgress> {
        let total: i64 = self
            .conn
            .query_row("SELECT count(*) FROM passages", [], |r| r.get(0))?;
        let embedded: i64 = self
            .conn
            .query_row("SELECT count(*) FROM embeddings", [], |r| r.get(0))?;
        Ok(EmbeddingProgress {
            embedded: embedded as usize,
            total: total as usize,
        })
    }

    /// Embeds up to `batch` passages that have no embedding yet.
    pub fn embed_pending(
        &mut self,
        embedder: &mut dyn Embedder,
        batch: usize,
    ) -> Result<EmbeddingProgress> {
        let dimension = embedder.dimension();
        let model = format!("{}|{dimension}", embedder.identity());
        self.use_embedding_model(embedder.identity(), dimension)?;
        let pending: Vec<(i64, String)> = {
            let mut statement = self.conn.prepare(
                "SELECT p.id, p.text FROM passages p
                 LEFT JOIN embeddings e ON e.passage_id = p.id
                 WHERE e.passage_id IS NULL ORDER BY p.id LIMIT ?1",
            )?;
            let rows =
                statement.query_map([batch.max(1) as i64], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        if !pending.is_empty() {
            let texts: Vec<&str> = pending.iter().map(|(_, text)| text.as_str()).collect();
            let vectors = embedder.embed(&texts)?;
            if vectors.len() != pending.len() || vectors.iter().any(|v| v.len() != dimension) {
                return Err(Error::Invalid(
                    "embedder returned the wrong count or dimension",
                ));
            }
            let tx = self.conn.transaction()?;
            let current: Option<String> = tx
                .query_row(
                    "SELECT value FROM meta WHERE key = 'embedding_model'",
                    [],
                    |r| r.get(0),
                )
                .optional()?;
            if current.as_deref() == Some(model.as_str()) {
                let mut insert = tx.prepare(
                    "INSERT OR REPLACE INTO embeddings(passage_id, vector)
                     SELECT ?1, ?2 WHERE EXISTS
                     (SELECT 1 FROM passages WHERE id = ?1 AND text = ?3)",
                )?;
                for ((id, text), vector) in pending.iter().zip(&vectors) {
                    insert.execute(params![id, to_blob(&unit(vector)?), text])?;
                }
            }
            tx.commit()?;
        }
        self.embedding_progress()
    }

    /// Embedded passages closest to `query_vector` by cosine similarity, best first.
    /// Returns nothing before any embedding model has been recorded.
    pub fn semantic(&self, query_vector: &[f32], limit: usize) -> Result<Vec<NoteHit>> {
        if !(1..=50).contains(&limit) {
            return Err(Error::Invalid("query length or limit"));
        }
        let Some((_, dimension)) = self.embedding_model()? else {
            return Ok(Vec::new());
        };
        if query_vector.len() != dimension {
            return Err(Error::Invalid("query embedding dimension"));
        }
        let query = unit(query_vector)?;
        let mut scored: Vec<(f32, HitRow)> = Vec::with_capacity(limit);
        let mut statement = self.conn.prepare(
            "SELECT p.id, p.path, n.sha256, p.start_byte, p.end_byte, e.vector, p.text
             FROM embeddings e JOIN passages p ON p.id = e.passage_id
             JOIN notes n ON n.path = p.path",
        )?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let bytes: Vec<u8> = row.get(5)?;
            let vector = from_blob(&bytes, dimension)?;
            let score = vector.iter().zip(&query).map(|(a, b)| a * b).sum::<f32>();
            if !score.is_finite() {
                continue;
            }
            let position = scored.partition_point(|(kept_score, hit)| {
                kept_score
                    .total_cmp(&score)
                    .reverse()
                    .then(hit.0.cmp(&id))
                    .is_lt()
            });
            if position >= limit {
                continue;
            }
            let hit = (
                id,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(6)?,
            );
            if scored.len() == limit {
                scored.pop();
            }
            scored.insert(position, (score, hit));
        }
        scored
            .into_iter()
            .map(|(score, hit)| to_hit(hit, score))
            .collect()
    }
}
