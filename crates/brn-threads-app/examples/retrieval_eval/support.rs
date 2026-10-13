//! Deterministic retrieval evaluation: a synthetic English corpus, the golden
//! query set and per-stratum ranking metrics. Shared by the opt-in example and
//! the integrity tests; never uses a default BRN data directory.
use brn_threads_app::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::Instant,
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const FORMAT: &str = "BRN retrieval evaluation 2";
pub const GOLDEN: &str = include_str!("../../tests/fixtures/retrieval_golden.json");
pub const DEFAULT_SEED: u64 = 0x4252_4e45; // "BRNE"
const BATCH: usize = 100;
/// Chat-only notes per synthetic chat; each is probed from every context.
pub const CHAT_NOTES_PER_CHAT: usize = 6;
const CHAT_LABELS: [&str; 2] = ["a", "b"];
const FUNCTION_WORDS: &[&str] = &[
    "the", "and", "to", "of", "for", "with", "on", "in", "we", "our", "is", "are", "was", "will",
    "this", "that", "next", "after", "before", "by", "from", "at", "as", "a", "it", "their",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stratum {
    Exact,
    Inflection,
    Paraphrase,
    Section,
    Topic,
    Abstention,
}

impl Stratum {
    pub const ALL: [Stratum; 6] = [
        Stratum::Exact,
        Stratum::Inflection,
        Stratum::Paraphrase,
        Stratum::Section,
        Stratum::Topic,
        Stratum::Abstention,
    ];
    pub fn expected_cases(self) -> usize {
        match self {
            Stratum::Exact | Stratum::Inflection | Stratum::Paraphrase => 20,
            Stratum::Section | Stratum::Abstention => 15,
            Stratum::Topic => 10,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Stratum::Exact => "exact",
            Stratum::Inflection => "inflection",
            Stratum::Paraphrase => "paraphrase",
            Stratum::Section => "section",
            Stratum::Topic => "topic",
            Stratum::Abstention => "abstention",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Golden {
    pub version: u32,
    pub description: String,
    pub vocabulary: Vec<String>,
    pub long_notes: Vec<LongNote>,
    pub topics: Vec<Topic>,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LongNote {
    pub title: String,
    pub sections: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Topic {
    pub title: String,
    pub summary: String,
    pub subjects: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub stratum: Stratum,
    pub query: String,
    pub plant: Option<Plant>,
    pub relevant: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plant {
    pub note: String,
    pub section: Option<String>,
    pub text: String,
}

fn tokens(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
}

const STOPWORDS: &[&str] = &[
    "about", "after", "also", "been", "before", "from", "have", "into", "more", "over", "that",
    "their", "them", "then", "they", "this", "what", "when", "where", "which", "while", "with",
    "will", "your", "does", "were", "there", "these", "those",
];

/// Lexical content tokens (four or more letters, not stopwords) used by the
/// fixture rules for strata that must not match literally.
fn content_tokens(text: &str) -> BTreeSet<String> {
    tokens(text)
        .filter(|t| t.chars().count() >= 4 && t.chars().all(char::is_alphabetic))
        .filter(|t| !STOPWORDS.contains(&t.as_str()))
        .collect()
}

/// Parses the checked-in golden set and enforces its structural and lexical rules.
pub fn golden() -> Result<Golden> {
    let golden: Golden = serde_json::from_str(GOLDEN)?;
    validate(&golden)?;
    Ok(golden)
}

pub fn validate(golden: &Golden) -> Result<()> {
    if golden.version != 1 {
        return Err("unsupported golden version".into());
    }
    if golden.vocabulary.len() < 100 {
        return Err("vocabulary must hold at least 100 words".into());
    }
    let long: BTreeMap<_, _> = golden
        .long_notes
        .iter()
        .map(|n| (n.title.as_str(), n))
        .collect();
    let topics: BTreeSet<_> = golden.topics.iter().map(|t| t.title.as_str()).collect();
    if long.len() != golden.long_notes.len() || topics.len() != golden.topics.len() {
        return Err("duplicate long note or topic title".into());
    }
    for stratum in Stratum::ALL {
        let count = golden.cases.iter().filter(|c| c.stratum == stratum).count();
        if count != stratum.expected_cases() {
            return Err(format!("{} has {count} cases", stratum.name()).into());
        }
    }
    let mut ids = BTreeSet::new();
    let mut subjects = BTreeSet::new();
    for case in &golden.cases {
        if !ids.insert(case.id.as_str()) {
            return Err(format!("duplicate case id {}", case.id).into());
        }
        if case.query.trim().is_empty() || case.query.len() > 512 {
            return Err(format!("{}: query length", case.id).into());
        }
        let expected: Vec<&str> = match (case.stratum, &case.plant) {
            (Stratum::Topic | Stratum::Abstention, Some(_)) => {
                return Err(format!("{}: unexpected plant", case.id).into());
            }
            (Stratum::Abstention, None) => vec![],
            (Stratum::Topic, None) => {
                if case.relevant.len() != 1 || !topics.contains(case.relevant[0].as_str()) {
                    return Err(format!("{}: topic relevance", case.id).into());
                }
                continue;
            }
            (_, None) => return Err(format!("{}: missing plant", case.id).into()),
            (Stratum::Section, Some(plant)) => {
                let note = long
                    .get(plant.note.as_str())
                    .ok_or(format!("{}: unknown long note", case.id))?;
                let section = plant
                    .section
                    .as_deref()
                    .ok_or(format!("{}: section plant needs a section", case.id))?;
                if !note.sections.iter().any(|s| s == section) {
                    return Err(format!("{}: unknown section", case.id).into());
                }
                vec![plant.note.as_str()]
            }
            (_, Some(plant)) => {
                if plant.section.is_some()
                    || long.contains_key(plant.note.as_str())
                    || topics.contains(plant.note.as_str())
                    || !subjects.insert(plant.note.as_str())
                {
                    return Err(format!("{}: subject note must be distinct", case.id).into());
                }
                vec![plant.note.as_str()]
            }
        };
        if case.relevant.iter().map(String::as_str).collect::<Vec<_>>() != expected {
            return Err(format!("{}: relevance must name the planted note", case.id).into());
        }
    }
    for topic in &golden.topics {
        if topic.subjects.is_empty()
            || topic
                .subjects
                .iter()
                .any(|s| !subjects.contains(s.as_str()))
        {
            return Err(format!("{}: topic subjects must be planted notes", topic.title).into());
        }
    }
    lexical_rules(golden)
}

fn lexical_rules(golden: &Golden) -> Result<()> {
    let mut corpus_words: BTreeSet<String> =
        golden.vocabulary.iter().map(|w| w.to_lowercase()).collect();
    for case in &golden.cases {
        if let Some(plant) = &case.plant {
            corpus_words.extend(tokens(&plant.text));
            corpus_words.extend(tokens(&plant.note));
        }
    }
    for note in &golden.long_notes {
        corpus_words.extend(tokens(&note.title));
        for section in &note.sections {
            corpus_words.extend(tokens(section));
        }
    }
    for topic in &golden.topics {
        corpus_words.extend(tokens(&topic.title));
        corpus_words.extend(tokens(&topic.summary));
    }
    corpus_words.extend(FUNCTION_WORDS.iter().map(|w| w.to_string()));
    for text in TITLE_SHAPES.iter().chain(BOILERPLATE) {
        corpus_words.extend(tokens(text));
    }
    for case in &golden.cases {
        match (case.stratum, &case.plant) {
            (Stratum::Inflection | Stratum::Paraphrase, Some(plant)) => {
                // The note's title is indexed as its heading, so it must not match literally either.
                let planted: BTreeSet<_> = tokens(&plant.text).chain(tokens(&plant.note)).collect();
                if let Some(word) = content_tokens(&case.query)
                    .into_iter()
                    .find(|w| planted.contains(w))
                {
                    return Err(
                        format!("{}: query word '{word}' appears literally", case.id).into(),
                    );
                }
            }
            (Stratum::Exact, Some(plant)) => {
                let holders = golden
                    .cases
                    .iter()
                    .filter_map(|c| c.plant.as_ref())
                    .filter(|p| p.text.contains(&case.query))
                    .count();
                if holders != 1 || !plant.text.contains(&case.query) {
                    return Err(
                        format!("{}: identifier must be planted exactly once", case.id).into(),
                    );
                }
            }
            (Stratum::Abstention, None) => {
                // Search ORs every token, so any shared token of any length returns hits.
                if let Some(word) = tokens(&case.query).find(|w| corpus_words.contains(w)) {
                    return Err(format!(
                        "{}: abstention word '{word}' exists in the corpus",
                        case.id
                    )
                    .into());
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// SplitMix64: small, deterministic and dependency-free.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a>(&mut self, items: &'a [String]) -> &'a str {
        &items[self.below(items.len())]
    }
}

fn sentence(rng: &mut Rng, vocabulary: &[String]) -> String {
    let length = 6 + rng.below(9);
    let mut words = Vec::with_capacity(length);
    for _ in 0..length {
        if rng.below(10) < 6 {
            words.push(rng.pick(vocabulary).to_string());
        } else {
            words.push(FUNCTION_WORDS[rng.below(FUNCTION_WORDS.len())].to_string());
        }
    }
    capitalized(&words.join(" ")) + "."
}

fn paragraph(rng: &mut Rng, vocabulary: &[String], sentences: usize) -> String {
    (0..sentences)
        .map(|_| sentence(rng, vocabulary))
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSpec {
    pub title: String,
    pub markdown: String,
    /// Titles of notes this note links to (topic pages only).
    pub links: Vec<String>,
}

/// Fixed words the generator writes besides vocabulary and golden text.
const BOILERPLATE: &[&str] = &["Imported reference document.", "Notes", "Item Owner Status"];

/// Background-note title shapes; the trailing counter keeps them distinct from golden titles.
const TITLE_SHAPES: [&str; 6] = [
    "Meeting notes",
    "Working notes",
    "Follow-up",
    "Reading notes",
    "Status update",
    "Planning draft",
];

/// The complete deterministic corpus: golden subject, long and topic notes plus
/// `total - specials` background notes, shuffled so golden notes are not
/// clustered in creation order.
pub fn corpus(golden: &Golden, total: usize, seed: u64) -> Result<Vec<NoteSpec>> {
    let mut rng = Rng(seed);
    let vocabulary = &golden.vocabulary;
    let mut subjects: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut sections: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
    for case in &golden.cases {
        if let Some(plant) = &case.plant {
            match &plant.section {
                Some(section) => sections
                    .entry((plant.note.as_str(), section.as_str()))
                    .or_default()
                    .push(&plant.text),
                None => subjects.entry(&plant.note).or_default().push(&plant.text),
            }
        }
    }
    let mut notes = Vec::new();
    for (title, plants) in &subjects {
        let count = 2 + rng.below(3);
        let mut paragraphs: Vec<String> = (0..count)
            .map(|_| {
                let sentences = 2 + rng.below(4);
                paragraph(&mut rng, vocabulary, sentences)
            })
            .collect();
        for plant in plants {
            let at = rng.below(paragraphs.len() + 1);
            paragraphs.insert(at, (*plant).to_string());
        }
        notes.push(NoteSpec {
            title: (*title).to_string(),
            markdown: format!("# {title}\n\n{}\n", paragraphs.join("\n\n")),
            links: vec![],
        });
    }
    for long in &golden.long_notes {
        let mut markdown = format!("# {}\n\nImported reference document.\n", long.title);
        for section in &long.sections {
            markdown.push_str(&format!("\n## {section}\n\n"));
            let count = 3 + rng.below(4);
            let mut paragraphs: Vec<String> = (0..count)
                .map(|_| {
                    let sentences = 3 + rng.below(4);
                    paragraph(&mut rng, vocabulary, sentences)
                })
                .collect();
            for plant in sections
                .get(&(long.title.as_str(), section.as_str()))
                .into_iter()
                .flatten()
            {
                let at = rng.below(paragraphs.len() + 1);
                paragraphs.insert(at, (*plant).to_string());
            }
            markdown.push_str(&paragraphs.join("\n\n"));
            markdown.push('\n');
        }
        notes.push(NoteSpec {
            title: long.title.clone(),
            markdown,
            links: vec![],
        });
    }
    for topic in &golden.topics {
        let list: Vec<String> = topic.subjects.iter().map(|s| format!("- {s}")).collect();
        notes.push(NoteSpec {
            title: topic.title.clone(),
            markdown: format!(
                "# {}\n\n{}\n\n## Notes\n\n{}\n",
                topic.title,
                topic.summary,
                list.join("\n")
            ),
            links: topic.subjects.clone(),
        });
    }
    let specials = notes.len();
    if total <= specials {
        return Err(format!("use more than {specials} notes").into());
    }
    for index in 0..total - specials {
        let shape = TITLE_SHAPES[rng.below(TITLE_SHAPES.len())];
        let first = capitalized(rng.pick(vocabulary));
        let second = rng.pick(vocabulary).to_string();
        let title = format!("{shape}: {first} {second} {index}");
        // Mostly short notes, some medium and a few large ones, so BM25
        // length normalization and passage splitting are both exercised.
        let count = match rng.below(100) {
            0..=1 => 30 + rng.below(30),
            2..=11 => 6 + rng.below(8),
            _ => 1 + rng.below(4),
        };
        let mut body: Vec<String> = (0..count)
            .map(|_| {
                let sentences = 2 + rng.below(5);
                paragraph(&mut rng, vocabulary, sentences)
            })
            .collect();
        // Structured blocks the chunker must keep whole: lists and tables.
        match rng.below(10) {
            0 => {
                let rows: Vec<String> = (0..3 + rng.below(6))
                    .map(|_| {
                        let item = capitalized(rng.pick(vocabulary));
                        let owner = rng.pick(vocabulary).to_string();
                        let state = rng.pick(vocabulary).to_string();
                        format!("| {item} | {owner} | {state} |")
                    })
                    .collect();
                let at = rng.below(body.len() + 1);
                body.insert(
                    at,
                    format!(
                        "| Item | Owner | Status |\n|---|---|---|\n{}",
                        rows.join("\n")
                    ),
                );
            }
            1 | 2 => {
                let items: Vec<String> = (0..2 + rng.below(5))
                    .map(|_| format!("- {}", sentence(&mut rng, vocabulary)))
                    .collect();
                let at = rng.below(body.len() + 1);
                body.insert(at, items.join("\n"));
            }
            _ => {}
        }
        notes.push(NoteSpec {
            markdown: format!("# {title}\n\n{}\n", body.join("\n\n")),
            title,
            links: vec![],
        });
    }
    for index in (1..notes.len()).rev() {
        let other = rng.below(index + 1);
        notes.swap(index, other);
    }
    Ok(notes)
}

pub fn corpus_sha256(notes: &[NoteSpec]) -> String {
    let mut hash = Sha256::new();
    for note in notes {
        for part in std::iter::once(&note.title)
            .chain(std::iter::once(&note.markdown))
            .chain(&note.links)
        {
            hash.update((part.len() as u64).to_le_bytes());
            hash.update(part.as_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}

pub struct Built {
    pub workspace: Workspace,
    /// Golden title -> note ID, for notes named by the golden set.
    pub ids_by_title: BTreeMap<String, String>,
    pub notes: usize,
    pub links: usize,
    pub markdown_bytes: usize,
    pub create_ms: f64,
    /// Chat conversation IDs that own the chat-only notes.
    pub chats: Vec<String>,
    pub chat_notes: Vec<ChatNote>,
}

/// A note visible only inside `chat`, findable by its unique `marker`. Its
/// text also repeats an exact-case query so a leaking retriever would surface
/// it for ordinary golden queries too.
#[derive(Debug, Clone)]
pub struct ChatNote {
    pub chat: String,
    pub id: String,
    pub marker: String,
}

/// Creates the corpus in a new explicit workspace through checked owner writes.
pub fn build(data: &Path, golden: &Golden, specs: &[NoteSpec]) -> Result<Built> {
    let started = Instant::now();
    let mut workspace = Workspace::open(data)?;
    let named: BTreeSet<&str> = golden
        .cases
        .iter()
        .flat_map(|c| c.relevant.iter().map(String::as_str))
        .chain(
            golden
                .topics
                .iter()
                .flat_map(|t| t.subjects.iter().map(String::as_str)),
        )
        .collect();
    let mut ids_by_title = BTreeMap::new();
    let mut markdown_bytes = 0;
    for (batch, chunk) in specs.chunks(BATCH).enumerate() {
        let key = format!("retrieval-eval-notes-{batch}");
        let op = workspace.store.allocate_operation(&key)?;
        let mut writes = Vec::with_capacity(chunk.len());
        for (index, spec) in chunk.iter().enumerate() {
            let id = op.creation_id(index);
            markdown_bytes += spec.markdown.len();
            if (named.contains(spec.title.as_str()) || !spec.links.is_empty())
                && ids_by_title
                    .insert(spec.title.clone(), id.clone())
                    .is_some()
            {
                return Err(format!("duplicate golden title {}", spec.title).into());
            }
            writes.push(Put {
                id,
                expected_version: None,
                archived: false,
                data: RecordData::Note(Note::working(&spec.title, &spec.markdown)),
            });
        }
        applied(workspace.owner_change(
            &key,
            &ChangeRequest {
                reason: "Synthetic retrieval evaluation corpus".into(),
                writes,
                inputs: vec![],
            },
        )?)?;
    }
    let mut links = Vec::new();
    for spec in specs.iter().filter(|s| !s.links.is_empty()) {
        let from = &ids_by_title[&spec.title];
        for target in &spec.links {
            links.push((from.clone(), ids_by_title[target].clone()));
        }
    }
    for (batch, chunk) in links.chunks(BATCH).enumerate() {
        let key = format!("retrieval-eval-links-{batch}");
        let op = workspace.store.allocate_operation(&key)?;
        let writes = chunk
            .iter()
            .enumerate()
            .map(|(index, (from, to))| Put {
                id: op.creation_id(index),
                expected_version: None,
                archived: false,
                data: RecordData::Link(Link {
                    from: from.clone(),
                    to: to.clone(),
                    relation: "related".into(),
                }),
            })
            .collect();
        applied(workspace.owner_change(
            &key,
            &ChangeRequest {
                reason: "Synthetic topic links".into(),
                writes,
                inputs: vec![],
            },
        )?)?;
    }
    let bait: Vec<&str> = golden
        .cases
        .iter()
        .filter(|c| c.stratum == Stratum::Exact)
        .map(|c| c.query.as_str())
        .collect();
    let mut chats = Vec::new();
    let mut chat_notes = Vec::new();
    for label in CHAT_LABELS {
        let chat = workspace
            .new_thread(&format!("Retrieval evaluation chat {label}"))?
            .id;
        let key = format!("retrieval-eval-chat-{label}");
        let op = workspace.store.allocate_operation(&key)?;
        let mut writes = Vec::new();
        for index in 0..CHAT_NOTES_PER_CHAT {
            let id = op.creation_id(index);
            let marker = format!("zzvis{label}{index}");
            let query = bait[(chat_notes.len()) % bait.len()];
            writes.push(Put {
                id: id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Note(Note {
                    chat: Some(chat.clone()),
                    ..Note::working(
                        format!("Chat {label} working note {index}"),
                        format!("# Chat {label} working note {index}\n\n{query}\n\n{marker}\n"),
                    )
                }),
            });
            chat_notes.push(ChatNote {
                chat: chat.clone(),
                id,
                marker,
            });
        }
        applied(workspace.owner_change(
            &key,
            &ChangeRequest {
                reason: "Synthetic chat-only notes".into(),
                writes,
                inputs: vec![],
            },
        )?)?;
        chats.push(chat);
    }
    Ok(Built {
        workspace,
        ids_by_title,
        notes: specs.len(),
        links: links.len(),
        markdown_bytes,
        create_ms: ms(started),
        chats,
        chat_notes,
    })
}

fn applied(outcome: ApplyOutcome) -> Result<Receipt> {
    match outcome {
        ApplyOutcome::Applied(receipt) => Ok(receipt),
        other => Err(format!("synthetic checked write did not apply: {other:?}").into()),
    }
}

fn ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

/// A ranked note retriever under evaluation: note IDs, best first, no
/// duplicates. It receives the workspace so the same retriever serves every
/// measurement, including the post-save probe. `chat` is the conversation the
/// query runs in (`None` for general search); results must be visible there.
pub trait Retriever {
    fn name(&self) -> String;
    fn search(
        &mut self,
        workspace: &mut Workspace,
        query: &str,
        chat: Option<&str>,
    ) -> Result<Vec<String>>;
}

/// Today's application search: in-memory FTS5 keyword passages (top 30),
/// collapsed to each note's first occurrence. It indexes general notes only
/// and has no chat context, so chat-only notes are never found.
pub struct CurrentAppSearch;

impl Retriever for CurrentAppSearch {
    fn name(&self) -> String {
        "current-app-keyword".into()
    }
    fn search(
        &mut self,
        workspace: &mut Workspace,
        query: &str,
        _chat: Option<&str>,
    ) -> Result<Vec<String>> {
        let mut seen = BTreeSet::new();
        Ok(workspace
            .search(query)?
            .into_iter()
            .filter(|hit| seen.insert(hit.note.clone()))
            .map(|hit| hit.note)
            .collect())
    }
}

/// The derived-index search worker (Task 1): exact and English-stemmed FTS5
/// lanes fused by RRF, collapsed per note and re-checked with core
/// visibility. The first query builds the persisted index.
#[derive(Default)]
pub struct WorkerSearch {
    handle: Option<brn_threads_app::search::SearchHandle>,
}

impl Retriever for WorkerSearch {
    fn name(&self) -> String {
        "worker-keyword".into()
    }
    fn search(
        &mut self,
        workspace: &mut Workspace,
        query: &str,
        chat: Option<&str>,
    ) -> Result<Vec<String>> {
        if self.handle.is_none() {
            self.handle = Some(brn_threads_app::search::SearchHandle::spawn(&workspace.store)?.0);
        }
        let handle = self.handle.as_ref().expect("spawned above");
        Ok(handle
            .search(query, chat, 50)?
            .matches
            .into_iter()
            .map(|m| m.note)
            .collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
    pub id: String,
    pub stratum: Stratum,
    /// 1-based rank of each relevant note; `None` when not returned.
    pub ranks: Vec<Option<usize>>,
    pub returned: usize,
    pub ms: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Metrics {
    pub cases: usize,
    pub recall_at_10: f64,
    pub recall_at_50: f64,
    pub mrr: f64,
    /// Abstention only: share of queries that returned any result (lower is better).
    pub any_result_rate: f64,
}

/// Recall@k averages, per judged case, the share of relevant notes ranked
/// within k; MRR averages 1/rank of the best relevant note. Cases without
/// relevant notes (abstention) only contribute to `any_result_rate`.
pub fn metrics(results: &[&CaseResult]) -> Metrics {
    let mut out = Metrics {
        cases: results.len(),
        ..Metrics::default()
    };
    let judged: Vec<_> = results.iter().filter(|r| !r.ranks.is_empty()).collect();
    if !judged.is_empty() {
        let n = judged.len() as f64;
        let recall = |k: usize| {
            judged
                .iter()
                .map(|r| {
                    r.ranks
                        .iter()
                        .filter(|rank| rank.is_some_and(|x| x <= k))
                        .count() as f64
                        / r.ranks.len() as f64
                })
                .sum::<f64>()
                / n
        };
        out.recall_at_10 = recall(10);
        out.recall_at_50 = recall(50);
        out.mrr = judged
            .iter()
            .map(|r| {
                r.ranks
                    .iter()
                    .flatten()
                    .min()
                    .map_or(0.0, |&rank| 1.0 / rank as f64)
            })
            .sum::<f64>()
            / n;
    }
    let unjudged: Vec<_> = results.iter().filter(|r| r.ranks.is_empty()).collect();
    if !unjudged.is_empty() {
        out.any_result_rate =
            unjudged.iter().filter(|r| r.returned > 0).count() as f64 / unjudged.len() as f64;
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timings {
    pub create_ms: f64,
    /// First query, including any lazy index build.
    pub first_query_ms: f64,
    pub query_p50_ms: f64,
    pub query_p95_ms: f64,
    pub query_max_ms: f64,
    /// First query after saving one changed note; times the query only, which
    /// includes any index sync it triggers, not the save. A single rare term,
    /// so not comparable with the per-case percentiles.
    pub post_save_query_ms: f64,
}

/// Context checks over every evaluated query: golden cases run in general
/// search; each chat-only marker runs in general, its own chat and the other
/// chat. A violation is any returned ID that is not a current, non-archived
/// note visible in the query's context under core rules.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Visibility {
    pub chat_notes: usize,
    pub queries: usize,
    pub violations: usize,
    /// Share of chat-only notes ranked within 10 by their marker in their own chat.
    pub in_chat_recall_at_10: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub format: String,
    pub retriever: String,
    pub notes: usize,
    pub links: usize,
    pub markdown_bytes: usize,
    pub seed: u64,
    pub corpus_sha256: String,
    pub golden_sha256: String,
    pub max_returned: usize,
    pub visibility: Visibility,
    pub timings: Timings,
    pub overall: Metrics,
    pub strata: BTreeMap<String, Metrics>,
    pub cases: Vec<CaseResult>,
}

pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

/// Builds the corpus in `data` (which must not exist yet) and evaluates the
/// current application search.
pub fn run(data: &Path, total: usize, seed: u64) -> Result<Report> {
    run_with(data, total, seed, &mut CurrentAppSearch)
}

/// Builds the corpus in `data` (which must not exist yet) and evaluates `retriever`.
pub fn run_with(
    data: &Path,
    total: usize,
    seed: u64,
    retriever: &mut dyn Retriever,
) -> Result<Report> {
    if data.exists() {
        return Err("evaluation data directory must not exist yet".into());
    }
    let golden = golden()?;
    let specs = corpus(&golden, total, seed)?;
    let corpus_sha256 = corpus_sha256(&specs);
    let mut built = build(data, &golden, &specs)?;
    let ids_by_title = built.ids_by_title.clone();
    let workspace = &mut built.workspace;
    let name = retriever.name();
    let started = Instant::now();
    retriever.search(workspace, &golden.cases[0].query, None)?;
    let first_query_ms = ms(started);
    let rows = workspace.store.records()?;
    let mut violations = 0;
    let mut results = Vec::with_capacity(golden.cases.len());
    for case in &golden.cases {
        let started = Instant::now();
        let ranked = retriever.search(workspace, &case.query, None)?;
        let elapsed = ms(started);
        violations += invisible(&rows, &ranked, None);
        let ranks = case
            .relevant
            .iter()
            .map(|title| {
                let id = &ids_by_title[title];
                ranked.iter().position(|hit| hit == id).map(|p| p + 1)
            })
            .collect();
        results.push(CaseResult {
            id: case.id.clone(),
            stratum: case.stratum,
            ranks,
            returned: ranked.len(),
            ms: elapsed,
        });
    }
    let mut visibility = Visibility {
        chat_notes: built.chat_notes.len(),
        queries: results.len(),
        ..Visibility::default()
    };
    let mut found_in_chat = 0;
    for note in &built.chat_notes {
        let contexts = std::iter::once(None).chain(built.chats.iter().map(|c| Some(c.as_str())));
        for chat in contexts {
            let ranked = retriever.search(workspace, &note.marker, chat)?;
            visibility.queries += 1;
            violations += invisible(&rows, &ranked, chat);
            if chat == Some(note.chat.as_str()) && ranked.iter().take(10).any(|id| id == &note.id) {
                found_in_chat += 1;
            }
        }
    }
    visibility.violations = violations;
    if !built.chat_notes.is_empty() {
        visibility.in_chat_recall_at_10 = found_in_chat as f64 / built.chat_notes.len() as f64;
    }
    let post_save_query_ms = post_save_probe(workspace, retriever, &golden, &ids_by_title)?;
    let mut latencies: Vec<f64> = results.iter().map(|r| r.ms).collect();
    latencies.sort_by(f64::total_cmp);
    let strata = Stratum::ALL
        .iter()
        .map(|&stratum| {
            let rows: Vec<_> = results.iter().filter(|r| r.stratum == stratum).collect();
            (stratum.name().to_string(), metrics(&rows))
        })
        .collect();
    Ok(Report {
        format: FORMAT.into(),
        retriever: name,
        notes: built.notes,
        links: built.links,
        markdown_bytes: built.markdown_bytes,
        seed,
        corpus_sha256,
        golden_sha256: format!("{:x}", Sha256::digest(GOLDEN.as_bytes())),
        max_returned: results.iter().map(|r| r.returned).max().unwrap_or(0),
        visibility,
        timings: Timings {
            create_ms: built.create_ms,
            first_query_ms,
            query_p50_ms: percentile(&latencies, 0.50),
            query_p95_ms: percentile(&latencies, 0.95),
            query_max_ms: latencies.last().copied().unwrap_or(0.0),
            post_save_query_ms,
        },
        overall: metrics(&results.iter().collect::<Vec<_>>()),
        strata,
        cases: results,
    })
}

/// Saves one changed golden subject note through the editor path, then
/// queries its new content through the retriever; the hit proves the
/// retriever observed the change.
fn post_save_probe(
    workspace: &mut Workspace,
    retriever: &mut dyn Retriever,
    golden: &Golden,
    ids_by_title: &BTreeMap<String, String>,
) -> Result<f64> {
    let title = golden
        .cases
        .iter()
        .find(|c| c.stratum == Stratum::Exact)
        .and_then(|c| c.plant.as_ref())
        .map(|p| p.note.clone())
        .ok_or("no exact case")?;
    let id = &ids_by_title[&title];
    let record = workspace.store.record(id)?.ok_or("missing synced note")?;
    let RecordData::Note(note) = record.data else {
        return Err("synced record is not a note".into());
    };
    let session = workspace.store.begin_edit(id, record.version)?;
    let marker = "zzsyncprobemarker";
    workspace
        .store
        .update_buffer(&session.id, 1, &format!("{}\n\n{marker}\n", note.markdown))?;
    workspace.store.save(&session.id, 1)?;
    let started = Instant::now();
    let hits = retriever.search(workspace, marker, None)?;
    let elapsed = ms(started);
    if !hits.contains(id) {
        return Err("post-save probe did not find the saved change".into());
    }
    Ok(elapsed)
}

/// Counts returned IDs that are not current, non-archived notes visible in `chat`.
fn invisible(rows: &[Record], ranked: &[String], chat: Option<&str>) -> usize {
    ranked
        .iter()
        .filter(|id| {
            !rows.iter().find(|r| &r.id == *id).is_some_and(|r| {
                !r.archived
                    && matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none())
                    && record_in_chat(r, rows, chat)
            })
        })
        .count()
}

/// Creates `root` atomically (refusing existing paths) and returns its data path.
pub fn reserve(root: &Path) -> Result<std::path::PathBuf> {
    if !root.is_absolute() || root.file_name().is_none() {
        return Err("choose an absolute explicit evaluation root".into());
    }
    let parent = root.parent().ok_or("missing evaluation root parent")?;
    if fs::canonicalize(parent)? != parent {
        return Err("evaluation parent must exist and use its canonical path".into());
    }
    fs::create_dir(root)?;
    Ok(root.join("data"))
}

pub fn summary(report: &Report) -> String {
    let t = &report.timings;
    let mut lines = vec![
        format!(
            "{} · {} notes · create {:.0} ms · first query {:.0} ms · p50 {:.1} ms · p95 {:.1} ms · post-save query {:.1} ms",
            report.retriever,
            report.notes,
            t.create_ms,
            t.first_query_ms,
            t.query_p50_ms,
            t.query_p95_ms,
            t.post_save_query_ms
        ),
        format!(
            "{:<11} {:>5} {:>9} {:>9} {:>6} {:>8}",
            "stratum", "cases", "recall@10", "recall@50", "MRR", "any-hit"
        ),
    ];
    let overall = "overall".to_string();
    for (name, m) in report.strata.iter().chain([(&overall, &report.overall)]) {
        lines.push(format!(
            "{:<11} {:>5} {:>9.3} {:>9.3} {:>6.3} {:>8.3}",
            name, m.cases, m.recall_at_10, m.recall_at_50, m.mrr, m.any_result_rate
        ));
    }
    let v = &report.visibility;
    lines.push(format!(
        "visibility: {} chat-only notes · {} queries · {} violations · in-chat recall@10 {:.3}",
        v.chat_notes, v.queries, v.violations, v.in_chat_recall_at_10
    ));
    lines.join("\n")
}
