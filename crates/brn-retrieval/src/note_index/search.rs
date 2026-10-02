use super::NoteIndex;
use crate::{Error, Result};
use rusqlite::{Connection, Row, params};

/// One matching passage, with its note's path and content hash at indexing time.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteHit {
    pub passage_id: i64,
    pub path: String,
    pub note_sha256: [u8; 32],
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
    /// Higher is better; comparable only within one result list.
    pub score: f32,
}

pub(super) const HIT_COLUMNS: &str = "p.id, p.path, n.sha256, p.start_byte, p.end_byte, p.text";
pub(super) type HitRow = (i64, String, Vec<u8>, i64, i64, String);

pub(super) fn hit_row(row: &Row<'_>) -> rusqlite::Result<HitRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

pub(super) fn to_hit(
    (passage_id, path, sha, start, end, quote): HitRow,
    score: f32,
) -> Result<NoteHit> {
    if start < 0 || end < start {
        return Err(Error::Corrupt("index passage byte range"));
    }
    let note_sha256 = sha
        .try_into()
        .map_err(|_| Error::Corrupt("index note hash"))?;
    Ok(NoteHit {
        passage_id,
        path,
        note_sha256,
        start_byte: usize::try_from(start)
            .map_err(|_| Error::Corrupt("index passage byte range"))?,
        end_byte: usize::try_from(end).map_err(|_| Error::Corrupt("index passage byte range"))?,
        quote,
        score,
    })
}

/// Rejects empty or over-long (> 512 bytes) queries and limits outside 1..=50.
pub fn check_query(query: &str, limit: usize) -> Result<()> {
    if query.trim().is_empty() || query.len() > 512 || !(1..=50).contains(&limit) {
        return Err(Error::Invalid("query length or limit"));
    }
    Ok(())
}

/// An FTS5 query matching any of the query's words. Each word is quoted, so
/// user text never becomes FTS5 syntax.
fn fts_query(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.to_lowercase()))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" OR "))
}

impl NoteIndex {
    /// Passages containing any query word, best BM25 score first.
    pub fn keyword(&self, query: &str, limit: usize) -> Result<Vec<NoteHit>> {
        keyword(&self.conn, query, limit)
    }
}

pub(super) fn keyword(conn: &Connection, query: &str, limit: usize) -> Result<Vec<NoteHit>> {
    check_query(query, limit)?;
    let fts = fts_query(query).ok_or(Error::Invalid("query has no search terms"))?;
    let mut statement = conn.prepare(&format!(
        "SELECT {HIT_COLUMNS}, bm25(passages_fts) AS rank FROM passages_fts \
             JOIN passages p ON p.id = passages_fts.rowid \
             JOIN notes n ON n.path = p.path \
             WHERE passages_fts MATCH ?1 ORDER BY rank, p.id LIMIT ?2"
    ))?;
    let rows = statement.query_map(params![fts, limit as i64], |row| {
        Ok((hit_row(row)?, row.get::<_, f64>(6)?))
    })?;
    rows.map(|row| {
        let (hit, rank) = row?;
        to_hit(hit, -rank as f32)
    })
    .collect()
}

/// Reciprocal-rank fusion (k = 60) of ranked lists; ties are broken by passage id.
pub fn fuse_hits(lists: &[&[NoteHit]], limit: usize) -> Vec<NoteHit> {
    let mut merged: Vec<NoteHit> = Vec::new();
    for list in lists {
        for (rank, hit) in list.iter().enumerate() {
            let contribution = 1.0 / (60.0 + rank as f32 + 1.0);
            match merged.iter_mut().find(|m| m.passage_id == hit.passage_id) {
                Some(existing) => existing.score += contribution,
                None => {
                    let mut fused = hit.clone();
                    fused.score = contribution;
                    merged.push(fused);
                }
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
