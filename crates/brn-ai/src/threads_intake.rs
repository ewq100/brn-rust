//! PROPOSED host-driven intake context. Before the first model turn of a run
//! with supplied sources, the host runs the deterministic part of intake: it
//! searches the run's chat context with queries derived from the source text
//! and puts the source, the closest related pages (exact versions) and their
//! link neighbours into the first prompt. No model is involved; the model
//! still decides what the information means and what to write, and every tool
//! stays available. Retrieval uses the same chat-scoped host search as the
//! `search_notes` tool, and page text is read through the run's chat context.
use super::*;
use std::{collections::BTreeMap, time::Instant};

const MAX_QUERIES: usize = 6;
const SEARCH_LIMIT: usize = 8;
const MAX_PAGES: usize = 3;
const PAGE_BYTES: usize = 8 * 1024;
const PAGES_BYTES: usize = 20 * 1024;
const SOURCE_BYTES: usize = 24 * 1024;
const ALSO_FOUND: usize = 5;
const SNIPPET_BYTES: usize = 160;
const NEIGHBOURS: usize = 8;
const QUERY_BYTES: usize = 480;
const LEAD_BYTES: usize = 300;
const DERIVE_BYTES: usize = 64 * 1024;
const RRF_K: f64 = 60.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Query {
    pub text: String,
    /// The lead is natural prose, so it also uses semantic matching.
    pub hybrid: bool,
}

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "can", "had", "her", "was", "one",
    "our", "out", "has", "have", "from", "with", "this", "that", "they", "will", "would", "there",
    "their", "what", "about", "which", "when", "your", "been", "were", "more", "also", "into",
    "than", "then", "them", "some", "could", "other", "only", "its", "who", "how", "any", "may",
    "per", "via", "ja", "on", "ei", "see", "sest", "kui", "ning", "aga", "ka", "oli", "mis", "kes",
    "siis", "veel", "nii", "või", "vaid", "oma", "need", "ole",
];

struct Stat {
    first: usize,
    count: usize,
    proper: bool,
    headline: bool,
    digits: bool,
    len: usize,
}

fn sentence_initial(text: &str, start: usize) -> bool {
    text[..start]
        .trim_end_matches([' ', '\t'])
        .chars()
        .next_back()
        .is_none_or(|c| matches!(c, '\n' | '.' | '!' | '?' | ':'))
}

/// Deterministic queries from a source's text: a prose lead for semantic
/// matching, the most salient distinct terms, and the names in it. Ranking
/// weighs terms in the first line and capitalised names, so a subject or
/// sender weighs more than words repeated in a long body.
pub(super) fn derive_queries(text: &str) -> Vec<Query> {
    let text = truncate(text, DERIVE_BYTES);
    let headline_end = text
        .lines()
        .position(|line| !line.trim().is_empty())
        .map(|n| {
            text.split_inclusive('\n')
                .take(n + 1)
                .map(str::len)
                .sum::<usize>()
        })
        .unwrap_or(0);
    let mut stats: BTreeMap<String, Stat> = BTreeMap::new();
    let mut start = None;
    let mut spans = Vec::new();
    for (i, c) in text.char_indices() {
        match (c.is_alphanumeric(), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                spans.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        spans.push((s, text.len()));
    }
    for (s, e) in spans {
        let word = &text[s..e];
        let len = word.chars().count();
        let lower = word.to_lowercase();
        if len < 3 || STOPWORDS.contains(&lower.as_str()) {
            continue;
        }
        let capital = word.chars().next().is_some_and(char::is_uppercase);
        let stat = stats.entry(lower).or_insert(Stat {
            first: s,
            count: 0,
            proper: false,
            headline: false,
            digits: word.chars().all(|c| c.is_ascii_digit()),
            len,
        });
        stat.count += 1;
        stat.proper |= capital && !sentence_initial(text, s);
        stat.headline |= s < headline_end && capital;
    }
    let mut out = Vec::new();
    let lead = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let lead = truncate(&lead, LEAD_BYTES);
    let lead = match lead.len() < LEAD_BYTES {
        true => lead,
        false => lead.rsplit_once(' ').map_or(lead, |(head, _)| head),
    };
    if !stats.is_empty() {
        out.push(Query {
            text: lead.to_owned(),
            hybrid: true,
        });
    }
    let mut ranked: Vec<(&String, &Stat)> = stats.iter().collect();
    ranked.sort_by(|(_, a), (_, b)| {
        let score = |s: &Stat| {
            (1.0 + s.count as f64).ln()
                + 0.5 * s.len.min(12) as f64 / 12.0
                + if s.proper { 1.5 } else { 0.0 }
                + if s.headline { 2.0 } else { 0.0 }
                - if s.digits { 0.8 } else { 0.0 }
        };
        score(b)
            .total_cmp(&score(a))
            .then_with(|| a.first.cmp(&b.first))
    });
    let join = |words: Vec<&String>| {
        let mut query = String::new();
        for word in words {
            if query.len() + word.len() + 1 > QUERY_BYTES {
                break;
            }
            if !query.is_empty() {
                query.push(' ');
            }
            query.push_str(word);
        }
        query
    };
    let terms = join(ranked.iter().take(24).map(|(word, _)| *word).collect());
    let mut names: Vec<(&String, &Stat)> = stats
        .iter()
        .filter(|(_, s)| s.proper || s.headline)
        .collect();
    names.sort_by_key(|(_, s)| s.first);
    let names = join(names.into_iter().take(10).map(|(word, _)| word).collect());
    for query in [terms, names] {
        if !query.is_empty() && out.iter().all(|q| q.text != query) {
            out.push(Query {
                text: query,
                hybrid: false,
            });
        }
    }
    out
}

struct Found {
    result: NoteSearchResult,
    score: f64,
    hits: usize,
}

struct Retrieval {
    queries: Vec<Query>,
    ranked: Vec<Found>,
    semantic: String,
    failures: usize,
}

/// Searches every derived query in the run's chat and fuses the rankings by
/// reciprocal rank, so a note several queries agree on leads.
fn retrieve(scope: &Scope, search: &Arc<dyn RunNoteSearch>) -> Retrieval {
    let mut queries: Vec<Query> = Vec::new();
    for source in scope.sources.values() {
        for query in derive_queries(&source.text) {
            if queries.len() < MAX_QUERIES && queries.iter().all(|q| q.text != query.text) {
                queries.push(query);
            }
        }
    }
    let mut found: BTreeMap<String, Found> = BTreeMap::new();
    let (mut semantic, mut failures) = (String::new(), 0);
    for query in &queries {
        match search.search(&scope.chat, &query.text, SEARCH_LIMIT, None, query.hybrid) {
            Ok(page) => {
                if query.hybrid && semantic.is_empty() {
                    semantic = page.semantic.clone();
                }
                for (rank, result) in page.results.into_iter().enumerate() {
                    let score = 1.0 / (RRF_K + rank as f64 + 1.0);
                    let entry = found.entry(result.id.clone()).or_insert(Found {
                        result,
                        score: 0.0,
                        hits: 0,
                    });
                    entry.score += score;
                    entry.hits += 1;
                }
            }
            Err(_) => failures += 1,
        }
    }
    let mut ranked: Vec<Found> = found.into_values().collect();
    ranked.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.result.title.cmp(&b.result.title))
            .then_with(|| a.result.id.cmp(&b.result.id))
    });
    Retrieval {
        queries,
        ranked,
        semantic,
        failures,
    }
}

fn source_context(scope: &Scope, store: &Store) -> AiResult<Vec<Value>> {
    let each = (SOURCE_BYTES / scope.sources.len().max(1)).max(4 * 1024);
    let mut out = Vec::new();
    for reference in scope.sources.keys() {
        // The same value read_source returns, so figure links and the Source
        // record version are exactly what the tool would show.
        let mut value = sources::source_metadata(
            &scope.sources,
            store,
            reference,
            Some(&scope.chat),
            &scope.source_ids,
        )?;
        let text = value["text"].as_str().unwrap_or_default().to_owned();
        if text.len() > each {
            value["text"] = Value::String(truncate(&text, each).to_owned());
            value["text_truncated"] = json!({"bytes_shown":truncate(&text, each).len(),"bytes_total":text.len(),"rest":"call read_source for the full text"});
        }
        out.push(value);
    }
    Ok(out)
}

fn neighbour(node: ConnectionNode) -> Value {
    json!({"id":node.id,"version":node.version,"title":node.title,"kind":node.kind,"relations":node.relations,"inbound_links":node.inbound_links})
}

/// The retrieved context as the first prompt carries it, and its trace.
pub(super) struct Context {
    pub prompt: Value,
    pub trace: Value,
}

fn build(scope: &Scope, search: Option<&Arc<dyn RunNoteSearch>>) -> AiResult<Context> {
    let started = Instant::now();
    let sources = {
        let store = scope.store.lock().map_err(storage)?;
        live(scope, &store)?;
        source_context(scope, &store)?
    };
    let Some(search) = search else {
        return Ok(Context {
            prompt: json!({"retrieval":"unavailable: this run has no note search; use read_records to find related notes","sources":sources}),
            trace: json!({"retrieval":"unavailable","pages":[],"ms":started.elapsed().as_millis() as u64}),
        });
    };
    let retrieval = retrieve(scope, search);
    // Page text is read through this run's chat context, whatever the index
    // returned; a note the chat cannot see never reaches the prompt.
    let visible: BTreeMap<String, Record> = {
        let store = scope.store.lock().map_err(storage)?;
        store
            .records_for_chat(Some(&scope.chat))
            .map_err(storage)?
            .into_iter()
            .filter(|record| {
                !record.archived
                    && matches!(&record.data, RecordData::Note(n) if n.superseded_by.is_none())
            })
            .filter(|record| {
                retrieval
                    .ranked
                    .iter()
                    .any(|found| found.result.id == record.id)
            })
            .map(|record| (record.id.clone(), record))
            .collect()
    };
    let (mut pages, mut also, mut budget) = (Vec::new(), Vec::new(), PAGES_BYTES);
    let mut page_ids = Vec::new();
    for (position, found) in retrieval.ranked.iter().enumerate() {
        let Some(record) = visible.get(&found.result.id) else {
            continue;
        };
        let RecordData::Note(note) = &record.data else {
            continue;
        };
        if pages.len() >= MAX_PAGES || budget < 1024 {
            if also.len() < ALSO_FOUND {
                also.push(json!({"id":record.id,"version":record.version,"title":note.title,"heading_path":found.result.heading_path,"snippet":truncate(&found.result.snippet, SNIPPET_BYTES)}));
            }
            continue;
        }
        let shown = truncate(&note.markdown, PAGE_BYTES.min(budget));
        budget -= shown.len();
        let mut page = json!({
            "rank": position + 1,
            "queries_matching": found.hits,
            "id": record.id,
            "version": record.version,
            "title": note.title,
            "matched_in": found.result.heading_path,
            "protected": note.protected,
            "imported": note.import.is_some(),
            "inbound_links": found.result.inbound_links,
            "bytes": note.markdown.len(),
            "markdown": shown,
        });
        if shown.len() < note.markdown.len() {
            let headings = note_headings(&note.markdown);
            page["truncated"] = json!({"next_start":shown.len(),"rest":"use read_note at this version and start","headings":headings.iter().take(24).collect::<Vec<_>>()});
        }
        if let Ok(around) = search.connections(&scope.chat, &record.id, 1, true) {
            let nodes: Vec<Value> = around
                .hops
                .into_iter()
                .flat_map(|hop| hop.neighbours)
                .take(NEIGHBOURS)
                .map(neighbour)
                .collect();
            page["links"] = Value::Array(nodes);
        }
        page_ids.push(json!({"id":record.id,"version":record.version,"bytes":shown.len()}));
        pages.push(page);
    }
    let queries: Vec<Value> = retrieval
        .queries
        .iter()
        .map(|q| json!({"mode":if q.hybrid {"hybrid"} else {"keyword"},"query":q.text}))
        .collect();
    let mut trace = json!({
        "retrieval": if retrieval.failures == 0 { "ok" } else { "partial" },
        "queries": retrieval.queries.len(),
        "failures": retrieval.failures,
        "semantic": retrieval.semantic,
        "pages": page_ids,
        "also_found": also.iter().filter_map(|a| a["id"].as_str().map(str::to_owned)).collect::<Vec<_>>(),
        "ms": started.elapsed().as_millis() as u64,
    });
    trace["query_text"] = Value::Array(queries.clone());
    Ok(Context {
        prompt: json!({
            "retrieval": trace["retrieval"],
            "semantic": retrieval.semantic,
            "queries_run": queries,
            "sources": sources,
            "related_pages": pages,
            "also_found": also,
        }),
        trace,
    })
}

const HEADER: &str = "Host intake context. The host ran these chat-scoped searches from the source text before this run and attached the source, the closest related notes at their exact current versions, and their direct link neighbours. It is retrieval only: decide what the source means, which pages to update or create, or keep it in the chat. Cite a page you rely on as a prepare_changes input at the id and version shown. The search is not exhaustive: when the answer depends on something not shown, use search_notes, note_connections or read_records. A source text marked truncated continues in read_source.";

/// The first prompt with the host intake context appended, or unchanged when
/// the run has no supplied sources. A failure leaves the plain prompt and the
/// tools, so the run falls back to the open loop in the same session.
pub(super) async fn with_context(scope: &Arc<Scope>, input: String) -> String {
    if scope.sources.is_empty() || scope.writing_snapshot.is_some() {
        return input;
    }
    let worker = scope.clone();
    let built = tokio::task::spawn_blocking(move || build(&worker, worker.search.as_ref())).await;
    match built {
        Ok(Ok(context)) => {
            let text = context.prompt.to_string();
            if let Some(trace) = &scope.trace {
                let mut summary = context.trace;
                summary["prompt_bytes"] = json!(text.len());
                trace.host_context(summary);
            }
            format!("{input}\n{HEADER}\n{text}")
        }
        _ => {
            if let Some(trace) = &scope.trace {
                trace.host_context(json!({"retrieval":"failed"}));
            }
            input
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(queries: &[Query]) -> Vec<&str> {
        queries.iter().map(|q| q.text.as_str()).collect()
    }

    #[test]
    fn queries_prefer_names_and_the_first_line_over_filler() {
        let source = "Email from Tõnis Kask (electrician), 5 October 2026: Rewiring the kitchen and hallway will cost 3,800 EUR. The fuse board must be replaced first; this is included in the price.";
        let queries = derive_queries(source);
        assert!(queries[0].hybrid && queries[0].text.starts_with("Email from Tõnis"));
        let terms = &queries[1].text;
        for wanted in ["tõnis", "kask", "kitchen", "rewiring", "electrician"] {
            assert!(terms.contains(wanted), "{terms}");
        }
        for filler in [" the ", " and ", " will "] {
            assert!(!format!(" {terms} ").contains(filler), "{terms}");
        }
        assert!(queries.iter().all(|q| q.text.len() <= 512));
    }

    #[test]
    fn queries_are_deterministic_bounded_and_skip_empty_text() {
        let long = "Harbour berth ".repeat(10_000) + &"quay depth ".repeat(5_000);
        let a = derive_queries(&long);
        assert_eq!(texts(&a), texts(&derive_queries(&long)));
        assert!(
            a.iter()
                .all(|q| q.text.len() <= LEAD_BYTES.max(QUERY_BYTES))
        );
        assert!(derive_queries("  \n ... -- ").is_empty());
        assert!(derive_queries("to be or").is_empty());
    }

    #[test]
    fn one_line_without_names_gives_lead_and_terms() {
        let queries =
            derive_queries("elektritööde algus lükkub novembrile sest kilp saabub hiljem");
        assert!(queries[0].hybrid);
        assert!(queries[1].text.contains("elektritööde"));
        assert!(!queries[1].text.split(' ').any(|w| w == "sest"));
    }
}
