//! Fixture v2 seeding: validation, deterministic planning and checked owner
//! writes. The example's `main` and the integration test both include this file.
use brn_threads_app::*;
use brn_threads_intake::{ConversionResult, Coverage, CoverageStatus, ImportIntent};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const BATCH: usize = 100;
const DEEP_START: usize = 128 * 1024;
const MIN_SECTION_BYTES: usize = 200;

pub struct Args {
    pub data_dir: PathBuf,
    pub fixture: PathBuf,
    pub case: String,
    pub filler: usize,
    pub seed: u64,
}

#[derive(Deserialize)]
struct FixtureFile {
    version: u32,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    #[serde(default)]
    seeds: Seeds,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Seeds {
    #[serde(default)]
    threads: Vec<ThreadSeed>,
    #[serde(default)]
    notes: Vec<NoteSeed>,
    #[serde(default)]
    distractors: Vec<Distractor>,
    #[serde(default)]
    links: Vec<LinkSeed>,
    #[serde(default)]
    topics: Option<TopicsSeed>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThreadSeed {
    key: String,
    title: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoteSeed {
    key: String,
    title: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    chat: Option<String>,
    #[serde(default)]
    edits: Vec<String>,
    #[serde(default)]
    import: Option<ImportSeed>,
    #[serde(default)]
    long: Option<LongSeed>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportSeed {
    source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LongSeed {
    sections: usize,
    #[serde(default)]
    target_bytes: Option<usize>,
    plant_section: usize,
    plant_heading: String,
    plant_text: String,
    #[serde(default)]
    seed: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Distractor {
    key: String,
    count: usize,
    title: String,
    text: String,
    #[serde(default)]
    values: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkSeed {
    from: String,
    to: String,
    relation: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TopicsSeed {
    pages: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub case: String,
    pub filler: usize,
    pub seed: u64,
    pub notes: BTreeMap<String, String>,
    pub threads: BTreeMap<String, String>,
    pub versions: BTreeMap<String, u64>,
    pub topics: Option<String>,
    pub ranges: BTreeMap<String, ByteRange>,
    pub filler_ids_sha256: String,
}

struct PlannedNote {
    key: String,
    title: String,
    markdown: String,
    chat: Option<String>,
    edits: Vec<String>,
    import: Option<String>,
    /// Explicit fixture notes report a version; templated distractors do not.
    explicit: bool,
}

struct Plan {
    threads: Vec<(String, String)>,
    notes: Vec<PlannedNote>,
    links: Vec<(String, String, String)>,
    topics: Option<Vec<String>>,
    filler: Vec<(String, String)>,
    ranges: BTreeMap<String, ByteRange>,
}

fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(message.into().into())
}

fn body(title: &str, text: &str) -> String {
    format!("# {title}\n\n{text}\n")
}

/// Splitmix64: small, deterministic and dependency-free.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn pick<'a>(&mut self, words: &[&'a str]) -> &'a str {
        words[(self.next() % words.len() as u64) as usize]
    }
}

const NOUNS: &[&str] = &[
    "ledger",
    "window",
    "schedule",
    "invoice",
    "garden",
    "cabinet",
    "survey",
    "meeting",
    "budget",
    "checklist",
    "contract",
    "workshop",
    "calendar",
    "inventory",
    "archive",
    "notebook",
    "delivery",
    "proposal",
    "report",
    "roster",
    "library",
    "kitchen",
    "printer",
    "newsletter",
    "timetable",
    "voucher",
    "folder",
    "template",
    "agenda",
    "receipt",
];
const ADJECTIVES: &[&str] = &[
    "quarterly",
    "draft",
    "shared",
    "revised",
    "weekly",
    "pending",
    "local",
    "final",
    "early",
    "routine",
    "annual",
    "interim",
    "general",
    "regular",
    "minor",
];
const VERBS: &[&str] = &[
    "updated",
    "reviewed",
    "postponed",
    "approved",
    "drafted",
    "archived",
    "compared",
    "confirmed",
    "reorganised",
    "circulated",
];
const DAYS: &[&str] = &[
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

fn sentence(rng: &mut Rng) -> String {
    match rng.next() % 4 {
        0 => format!(
            "The {} {} was {} after the {} {} on {}.",
            rng.pick(ADJECTIVES),
            rng.pick(NOUNS),
            rng.pick(VERBS),
            rng.pick(ADJECTIVES),
            rng.pick(NOUNS),
            rng.pick(DAYS)
        ),
        1 => format!(
            "We {} the {} {} before the {} meeting.",
            rng.pick(&["checked", "moved", "sorted", "listed"]),
            rng.pick(ADJECTIVES),
            rng.pick(NOUNS),
            rng.pick(NOUNS)
        ),
        2 => format!(
            "Each {} follows the {} {} and stays with the {}.",
            rng.pick(NOUNS),
            rng.pick(ADJECTIVES),
            rng.pick(NOUNS),
            rng.pick(NOUNS)
        ),
        _ => format!(
            "A {} {} is {} every {}.",
            rng.pick(ADJECTIVES),
            rng.pick(NOUNS),
            rng.pick(VERBS),
            rng.pick(DAYS)
        ),
    }
}

fn paragraph(rng: &mut Rng, at_least: usize) -> String {
    let mut text = String::new();
    while text.len() < at_least.max(1) {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(&sentence(rng));
    }
    text
}

/// Replaces each `{name}` with `values[name][index % len]`; every placeholder
/// must have values.
fn fill(template: &str, values: &BTreeMap<String, Vec<String>>, index: usize) -> Result<String> {
    let mut out = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close)
                if !after[..close].is_empty()
                    && after[..close]
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '-') =>
            {
                let name = &after[..close];
                let Some(list) = values.get(name).filter(|list| !list.is_empty()) else {
                    return invalid(format!("placeholder {{{name}}} has no values"));
                };
                out.push_str(&rest[..open]);
                out.push_str(&list[index % list.len()]);
                rest = &after[close + 1..];
            }
            _ => {
                out.push_str(&rest[..=open]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// Deterministic long body with one planted section; returns the markdown and
/// the plant section's byte range (heading through plant text).
fn long_body(title: &str, text: &str, long: &LongSeed) -> Result<(String, ByteRange)> {
    if long.sections == 0 || long.plant_section >= long.sections {
        return invalid("long.plant_section must be below long.sections");
    }
    let mut out = if text.is_empty() {
        format!("# {title}\n\n")
    } else {
        format!("# {title}\n\n{text}\n\n")
    };
    let plant = format!("## {}\n\n{}", long.plant_heading, long.plant_text);
    let natural = long.sections * 400;
    let target = long.target_bytes.unwrap_or(natural);
    let budget = target.saturating_sub(out.len() + plant.len() + 2);
    let before = long.plant_section;
    let after = long.sections - 1 - long.plant_section;
    let mut before_bytes = if long.sections > 1 {
        budget * before / (long.sections - 1)
    } else {
        0
    };
    // Push the plant past 128 KiB whenever the target leaves room for it.
    let wanted = (DEEP_START + 1024).saturating_sub(out.len());
    if before > 0 && before_bytes < wanted && budget >= wanted + after * MIN_SECTION_BYTES {
        before_bytes = wanted;
    }
    let after_bytes = budget.saturating_sub(before_bytes);
    let mut rng = Rng(long.seed ^ 0x6C6F_6E67);
    let mut range = None;
    for section in 0..long.sections {
        if section == long.plant_section {
            let start = out.len();
            out.push_str(&plant);
            range = Some(ByteRange {
                start,
                end: out.len(),
            });
            out.push_str("\n\n");
            continue;
        }
        let share = if section < long.plant_section {
            before_bytes / before
        } else {
            after_bytes / after
        };
        out.push_str(&format!(
            "## Notes on {} {}\n\n",
            rng.pick(NOUNS),
            section + 1
        ));
        out.push_str(&paragraph(&mut rng, share.saturating_sub(40)));
        out.push_str("\n\n");
    }
    out.truncate(out.trim_end().len());
    out.push('\n');
    Ok((out, range.expect("plant section is within range")))
}

fn plan(case: &Case, filler: usize, seed: u64) -> Result<Plan> {
    let seeds = &case.seeds;
    let mut keys = BTreeSet::from(["case".to_owned()]);
    let mut claim = |key: &str| -> Result<()> {
        if key.trim().is_empty() {
            return invalid("seed key must not be empty");
        }
        if !keys.insert(key.to_owned()) {
            return invalid(format!("duplicate seed key {key}"));
        }
        Ok(())
    };
    let mut threads = vec![("case".to_owned(), "Evaluation case".to_owned())];
    for thread in &seeds.threads {
        claim(&thread.key)?;
        threads.push((thread.key.clone(), thread.title.clone()));
    }
    let thread_keys: BTreeSet<&str> = threads.iter().map(|(k, _)| k.as_str()).collect();
    let mut notes = Vec::new();
    let mut ranges = BTreeMap::new();
    let mut sources = BTreeSet::new();
    for seed_note in &seeds.notes {
        claim(&seed_note.key)?;
        if seed_note.title.trim().is_empty() {
            return invalid(format!("note {} has an empty title", seed_note.key));
        }
        if let Some(chat) = &seed_note.chat
            && !thread_keys.contains(chat.as_str())
        {
            return invalid(format!(
                "note {} names unknown chat thread {chat}",
                seed_note.key
            ));
        }
        if let Some(import) = &seed_note.import {
            if !seed_note.edits.is_empty() {
                return invalid(format!("imported note {} cannot have edits", seed_note.key));
            }
            if import.source.trim().is_empty() || !sources.insert(import.source.clone()) {
                return invalid(format!(
                    "note {} needs a unique non-empty import source",
                    seed_note.key
                ));
            }
        }
        let markdown = if let Some(long) = &seed_note.long {
            let (markdown, range) = long_body(&seed_note.title, &seed_note.text, long)
                .map_err(|e| format!("note {}: {e}", seed_note.key))?;
            ranges.insert(seed_note.key.clone(), range);
            markdown
        } else {
            body(&seed_note.title, &seed_note.text)
        };
        notes.push(PlannedNote {
            key: seed_note.key.clone(),
            title: seed_note.title.clone(),
            markdown,
            chat: seed_note.chat.clone(),
            edits: seed_note
                .edits
                .iter()
                .map(|text| body(&seed_note.title, text))
                .collect(),
            import: seed_note.import.as_ref().map(|i| i.source.clone()),
            explicit: true,
        });
    }
    let mut titles: BTreeSet<String> = notes.iter().map(|n| n.title.clone()).collect();
    let mut distractor_titles = BTreeSet::new();
    for distractor in &seeds.distractors {
        if distractor.count == 0 {
            return invalid(format!("distractor {} has count 0", distractor.key));
        }
        for index in 0..distractor.count {
            let key = format!("{}-{index}", distractor.key);
            claim(&key)?;
            let title = fill(&distractor.title, &distractor.values, index)
                .map_err(|e| format!("distractor {key}: {e}"))?;
            let text = fill(&distractor.text, &distractor.values, index)
                .map_err(|e| format!("distractor {key}: {e}"))?;
            if titles.contains(&title) || !distractor_titles.insert(title.clone()) {
                return invalid(format!("duplicate distractor title {title:?} ({key})"));
            }
            notes.push(PlannedNote {
                key,
                markdown: body(&title, &text),
                title,
                chat: None,
                edits: vec![],
                import: None,
                explicit: false,
            });
        }
    }
    titles.extend(distractor_titles);
    for link in &seeds.links {
        for end in [&link.from, &link.to] {
            if !keys.contains(end) {
                return invalid(format!("link names unknown key {end}"));
            }
        }
        if link.relation.trim().is_empty() || link.from == link.to {
            return invalid(format!(
                "link {} -> {} needs a relation and distinct ends",
                link.from, link.to
            ));
        }
    }
    let note_keys: BTreeSet<&str> = notes.iter().map(|n| n.key.as_str()).collect();
    let topics = match &seeds.topics {
        Some(topics) => {
            if topics.pages.is_empty() {
                return invalid("topics.pages must not be empty");
            }
            for page in &topics.pages {
                if !note_keys.contains(page.as_str()) {
                    return invalid(format!("topics.pages names unknown note {page}"));
                }
            }
            if !titles.insert("Topics".into()) {
                return invalid("a fixture note is already titled \"Topics\"");
            }
            Some(topics.pages.clone())
        }
        None => None,
    };
    let mut rng = Rng(seed);
    let mut filler_notes = Vec::with_capacity(filler);
    for index in 0..filler {
        let mut title = format!(
            "{} {} log {:05}",
            rng.pick(ADJECTIVES),
            rng.pick(NOUNS),
            index + 1
        );
        while titles.contains(&title) {
            title.push_str(" bis");
        }
        titles.insert(title.clone());
        let sentences = 3 + rng.next() % 4;
        let text = (0..sentences)
            .map(|_| sentence(&mut rng))
            .collect::<Vec<_>>()
            .join(" ");
        filler_notes.push((title.clone(), body(&title, &text)));
    }
    Ok(Plan {
        threads,
        notes,
        links: seeds
            .links
            .iter()
            .map(|l| (l.from.clone(), l.to.clone(), l.relation.clone()))
            .collect(),
        topics,
        filler: filler_notes,
        ranges,
    })
}

fn load(text: &str, case: &str) -> Result<Case> {
    let file: FixtureFile = serde_json::from_str(text)?;
    if file.version != 2 {
        return invalid(format!("fixture version {} is not 2", file.version));
    }
    file.cases
        .into_iter()
        .find(|c| c.id == case)
        .ok_or_else(|| format!("case {case} not found in fixture").into())
}

fn applied(outcome: ApplyOutcome) -> Result<Receipt> {
    match outcome {
        ApplyOutcome::Applied(receipt) => Ok(receipt),
        other => invalid(format!("synthetic checked write did not apply: {other:?}")),
    }
}

/// Creates `data` in batches of 100 checked owner writes; returns IDs in order.
fn create_all(
    workspace: &mut Workspace,
    prefix: &str,
    reason: &str,
    data: Vec<RecordData>,
) -> Result<Vec<String>> {
    let mut ids = Vec::with_capacity(data.len());
    let mut data = data.into_iter();
    for batch in 0.. {
        let chunk: Vec<RecordData> = data.by_ref().take(BATCH).collect();
        if chunk.is_empty() {
            break;
        }
        let key = format!("wiki-eval-{prefix}-{batch}");
        let op = workspace.store.allocate_operation(&key)?;
        let writes = chunk
            .into_iter()
            .enumerate()
            .map(|(index, data)| {
                let id = op.creation_id(index);
                ids.push(id.clone());
                Put {
                    id,
                    expected_version: None,
                    archived: false,
                    data,
                }
            })
            .collect();
        applied(workspace.owner_change(
            &key,
            &ChangeRequest {
                reason: reason.into(),
                writes,
                inputs: vec![],
            },
        )?)?;
    }
    Ok(ids)
}

fn import_note(
    workspace: &mut Workspace,
    index: usize,
    note: &PlannedNote,
    chat: Option<&str>,
) -> Result<String> {
    let source = note.import.as_deref().expect("planned import");
    let converted = ConversionResult {
        intent: ImportIntent::FullNote,
        markdown: note.markdown.clone(),
        assets: vec![],
        source_sha256: Sha256::digest(source.as_bytes()).into(),
        source_reference: source.into(),
        converter: "synthetic evaluation seed".into(),
        sources: vec![],
        coverage: Coverage {
            status: CoverageStatus::Complete,
            checked_items: vec![],
            gaps: vec![],
        },
    };
    let key = format!("wiki-eval-import-{index}");
    let outcome = match chat {
        Some(chat) => workspace.attach_full(&key, chat, &note.title, &converted)?,
        None => workspace.import_full(&key, &note.title, &converted)?,
    };
    applied(outcome)?
        .writes
        .into_iter()
        .find_map(|w| match w.after.data {
            RecordData::Note(_) => Some(w.after.id),
            _ => None,
        })
        .ok_or_else(|| "import wrote no note".into())
}

#[allow(dead_code)] // used by the integration test
/// Validates `text` for `case` and returns the number of planned notes
/// (explicit and templated), without touching any workspace.
pub fn validate(text: &str, case: &str) -> Result<usize> {
    Ok(plan(&load(text, case)?, 0, 0)?.notes.len())
}

pub fn run(args: &Args) -> Result<Report> {
    let text = fs::read_to_string(&args.fixture)
        .map_err(|e| format!("cannot read fixture {}: {e}", args.fixture.display()))?;
    seed_fixture(&args.data_dir, &text, &args.case, args.filler, args.seed)
}

pub fn seed_fixture(
    data_dir: &Path,
    fixture: &str,
    case_id: &str,
    filler: usize,
    seed: u64,
) -> Result<Report> {
    let case = load(fixture, case_id)?;
    let plan = plan(&case, filler, seed)?;
    if data_dir.exists() && fs::read_dir(data_dir)?.next().is_some() {
        return invalid(format!(
            "data dir {} must be fresh (missing or empty)",
            data_dir.display()
        ));
    }
    let mut workspace = Workspace::open(data_dir)?;
    let mut ids: BTreeMap<String, String> = BTreeMap::new();

    let thread_ids = create_all(
        &mut workspace,
        "threads",
        "Synthetic evaluation threads",
        plan.threads
            .iter()
            .map(|(_, title)| {
                RecordData::Thread(Thread {
                    private_context: false,
                    context_epoch: 0,
                    title: title.clone(),
                    state: ThreadState::Open,
                    attention: vec![],
                })
            })
            .collect(),
    )?;
    let threads: BTreeMap<String, String> = plan
        .threads
        .iter()
        .map(|(key, _)| key.clone())
        .zip(thread_ids)
        .collect();
    ids.extend(threads.clone());

    // Ordinary notes first, in fixture order; protected imports then follow in array order.
    let ordinary: Vec<&PlannedNote> = plan.notes.iter().filter(|n| n.import.is_none()).collect();
    let created = create_all(
        &mut workspace,
        "notes",
        "Synthetic evaluation notes",
        ordinary
            .iter()
            .map(|n| {
                RecordData::Note(Note {
                    chat: n.chat.as_ref().map(|chat| threads[chat].clone()),
                    ..Note::working(&n.title, &n.markdown)
                })
            })
            .collect(),
    )?;
    for (note, id) in ordinary.iter().zip(created) {
        ids.insert(note.key.clone(), id);
    }
    for (index, note) in plan.notes.iter().filter(|n| n.import.is_some()).enumerate() {
        let chat = note.chat.as_ref().map(|chat| threads[chat].as_str());
        let id = import_note(&mut workspace, index, note, chat)?;
        ids.insert(note.key.clone(), id);
    }

    let rounds = plan.notes.iter().map(|n| n.edits.len()).max().unwrap_or(0);
    for round in 0..rounds {
        let mut writes = Vec::new();
        for note in plan.notes.iter().filter(|n| n.edits.len() > round) {
            let id = &ids[&note.key];
            let record = workspace
                .store
                .record(id)?
                .ok_or_else(|| format!("note {} vanished", note.key))?;
            let RecordData::Note(current) = record.data else {
                return invalid(format!("{} is not a note", note.key));
            };
            writes.push(Put {
                id: id.clone(),
                expected_version: Some(record.version),
                archived: false,
                data: RecordData::Note(Note {
                    markdown: note.edits[round].clone(),
                    ..current
                }),
            });
        }
        for (batch, chunk) in writes.chunks(BATCH).enumerate() {
            let key = format!("wiki-eval-edits-{round}-{batch}");
            applied(workspace.owner_change(
                &key,
                &ChangeRequest {
                    reason: "Synthetic owner edit".into(),
                    writes: chunk.to_vec(),
                    inputs: vec![],
                },
            )?)?;
        }
    }

    let mut links: Vec<(String, String, String)> = plan
        .links
        .iter()
        .map(|(from, to, relation)| (ids[from].clone(), ids[to].clone(), relation.clone()))
        .collect();
    let topics_id = match &plan.topics {
        Some(pages) => {
            let mut text = String::from(
                "Topic pages for this workspace. Each topic page links the subject pages of one area.\n\n",
            );
            for page in pages {
                let title = &plan.notes.iter().find(|n| &n.key == page).unwrap().title;
                text.push_str(&format!("- {title}\n"));
            }
            let id = create_all(
                &mut workspace,
                "topics",
                "Synthetic Topics note",
                vec![RecordData::Note(Note::working(
                    "Topics",
                    body("Topics", text.trim_end()),
                ))],
            )?
            .remove(0);
            links.extend(
                pages
                    .iter()
                    .map(|page| (id.clone(), ids[page].clone(), "related".to_owned())),
            );
            Some(id)
        }
        None => None,
    };

    let filler_ids = create_all(
        &mut workspace,
        "filler",
        "Synthetic filler notes",
        plan.filler
            .iter()
            .map(|(title, markdown)| RecordData::Note(Note::working(title, markdown)))
            .collect(),
    )?;
    // Pairs 2i<->2i+1 are linked both ways; an odd last note joins its neighbour.
    for index in (0..filler_ids.len()).step_by(2) {
        let (a, b) = if index + 1 < filler_ids.len() {
            (index, index + 1)
        } else if index > 0 {
            (index, index - 1)
        } else {
            continue;
        };
        links.push((
            filler_ids[a].clone(),
            filler_ids[b].clone(),
            "related".into(),
        ));
        links.push((
            filler_ids[b].clone(),
            filler_ids[a].clone(),
            "related".into(),
        ));
    }
    create_all(
        &mut workspace,
        "links",
        "Synthetic evaluation links",
        links
            .into_iter()
            .map(|(from, to, relation)| RecordData::Link(Link { from, to, relation }))
            .collect(),
    )?;
    if let Some(id) = &topics_id {
        applied(workspace.choose_topics_note(id)?)?;
    }

    let mut versions = BTreeMap::new();
    for note in plan.notes.iter().filter(|n| n.explicit) {
        let record = workspace
            .store
            .record(&ids[&note.key])?
            .ok_or_else(|| format!("note {} vanished", note.key))?;
        versions.insert(note.key.clone(), record.version);
    }
    let mut hash = Sha256::new();
    for id in &filler_ids {
        hash.update((id.len() as u64).to_le_bytes());
        hash.update(id.as_bytes());
    }
    Ok(Report {
        case: case_id.to_owned(),
        filler,
        seed,
        notes: plan
            .notes
            .iter()
            .map(|n| (n.key.clone(), ids[&n.key].clone()))
            .collect(),
        threads,
        versions,
        topics: topics_id,
        ranges: plan.ranges,
        filler_ids_sha256: format!("{:x}", hash.finalize()),
    })
}
