use brn_threads_app::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, time::Instant};

const FORMAT: &str = "BRN explicit synthetic qualification corpus 1";
const IMAGE: &[u8] = include_bytes!("../../../brn-intake/tests/fixtures/quay-map.png");
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Corpus {
    pub format: String,
    pub source: String,
    pub notes: usize,
    pub short_notes: usize,
    pub medium_notes: usize,
    pub long_notes: usize,
    pub current_markdown_bytes: usize,
    pub managed_images: usize,
    pub image_bytes: usize,
    pub sources: usize,
    pub revised_notes: usize,
    pub additional_note_revisions: usize,
    pub threads: usize,
    pub messages: usize,
    pub actions: usize,
    pub records: usize,
    pub receipts: usize,
    pub database_bytes: u64,
    pub snapshot_bytes: u64,
    pub create_ms: f64,
    pub semantic_content_sha256: String,
    pub selected_note_index: usize,
    pub selected_note: String,
    pub selected_thread: String,
}

// Semantic content and counts are deterministic. Supported store interfaces
// allocate fresh UUIDs and receipt timestamps, so physical DB hashes vary.
pub fn note_body(index: usize) -> String {
    let size = match index % 20 {
        0 => 128 * 1024,
        1..=4 => 16 * 1024,
        _ => 2 * 1024,
    };
    let mut body = format!(
        "# Project {index}: Quay operations\n\n## Current evidence\nSynthetic briefing for the October review. Leena owns the berth inspection; Mari prepares the supplier reply. Information below is qualification data.\n\n| Work | Owner | Status |\n|---|---|---|\n| Inspect pier | Leena | Open |\n| Draft response | Mari | Waiting |\n\n## Process and source excerpts\n"
    );
    let mut section = 0;
    while body.len() < size {
        body.push_str(&format!(
            "\n### Inspection section {section}\n1. Compare arrival plan {index}-{section} with the berth diagram.\n2. Record the supplier's requested date before confirming a commitment.\n3. Preserve the source wording and note any conflict with the accepted plan.\n\n> Synthetic email excerpt: capacity is provisional until the crane inspection is complete.\n\nThe working response should state the observed capacity, identify the unresolved inspection, and ask the coordinator for the remaining evidence. Project Õhtu keeps its decision history available for the next review.\n"
        ));
        section += 1;
    }
    body.push_str(&format!(
        "\n## Final checklist\nTail evidence: quay_tail_{index}.\n"
    ));
    body
}

fn absolute_root(root: &Path) -> Result<()> {
    if !root.is_absolute() || root.file_name().is_none() {
        return Err("Choose an absolute explicit synthetic fixture root".into());
    }
    let parent = root.parent().ok_or("Missing fixture parent")?;
    if fs::canonicalize(parent)? != parent {
        return Err("Fixture parent must exist and use its canonical path (no symlinks)".into());
    }
    Ok(())
}

fn applied(outcome: ApplyOutcome) -> Result<Receipt> {
    if let ApplyOutcome::Applied(receipt) = outcome {
        Ok(receipt)
    } else {
        Err(format!("Synthetic checked write did not apply: {outcome:?}").into())
    }
}

fn commit(app: &mut Workspace, key: &str, writes: Vec<Put>) -> Result<Receipt> {
    applied(app.owner_change(
        key,
        &ChangeRequest {
            reason: "Explicit synthetic qualification fixture".into(),
            writes,
            inputs: vec![],
        },
    )?)
}

fn new_put(id: String, data: RecordData) -> Put {
    Put {
        id,
        expected_version: None,
        archived: false,
        data,
    }
}

fn directory_bytes(path: &Path) -> Result<u64> {
    let mut bytes = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err("Unexpected symlink in synthetic fixture".into());
        }
        bytes += if kind.is_dir() {
            directory_bytes(&entry.path())?
        } else {
            entry.metadata()?.len()
        };
    }
    Ok(bytes)
}

pub fn create(root: &Path, notes: usize, source: &str) -> Result<Corpus> {
    if !(20..=1000).contains(&notes) || !notes.is_multiple_of(20) {
        return Err("Use 20..=1000 notes, in multiples of 20; default is 500".into());
    }
    absolute_root(root)?;
    // Atomic reservation refuses any existing path, including an empty directory,
    // before opening a store or inspecting/writing anything below that path.
    fs::create_dir(root)?;
    let started = Instant::now();
    let mut app = Workspace::open(root.join("data"))?;
    app.choose_delegation(DelegationChoice::MaintainOrdinaryNotes)?;
    let mut semantic_hash = Sha256::new();
    let mut ids = vec![];
    let mut current_bytes = 0;
    let mut revised = 0;
    let mut images = 0;
    for index in 0..notes {
        let key = format!("qualification-note-{index}");
        let op = app.store.allocate_operation(&key)?;
        let id = op.creation_id(0);
        let mut body = note_body(index);
        semantic_hash.update(body.as_bytes());
        let mut note = Note::working(format!("Quay project {index} · Õhtu"), "");
        let mut writes = vec![];
        if index % 5 == 0 {
            let source_id = op.creation_id(1);
            let asset_id = op.creation_id(2);
            body.push_str(&format!(
                "\n![Quay berth diagram](brn-asset://{asset_id})\n"
            ));
            note.import = Some(ImportCoverage {
                source: source_id.clone(),
                full_note: false,
                gaps: vec![],
                report: None,
            });
            writes.push(new_put(
                source_id.clone(),
                RecordData::Source(Source {
                    chat: None,
                    locator: format!("synthetic://qualification/email/{index}"),
                    label: format!("Synthetic coordinator email {index}"),
                    observed_at: "2026-10-11T09:00:00Z".into(),
                    external_version: Some("fixture-v1".into()),
                    outcome: "Useful extracted information".into(),
                    gaps: vec![],
                    report: None,
                }),
            ));
            writes.push(new_put(
                asset_id,
                RecordData::Asset(Asset {
                    note: id.clone(),
                    source: Some(source_id.clone()),
                    media_type: "image/png".into(),
                    bytes: IMAGE.to_vec(),
                }),
            ));
            writes.push(new_put(
                op.creation_id(3),
                RecordData::Link(Link {
                    from: id.clone(),
                    to: source_id,
                    relation: "source".into(),
                }),
            ));
            images += 1;
        }
        note.markdown.clone_from(&body);
        note.protected = index % 20 == 0;
        writes.insert(0, new_put(id.clone(), RecordData::Note(note)));
        commit(&mut app, &key, writes)?;
        if index % 4 == 0 {
            for revision in 2..=4 {
                let record = app.store.record(&id)?.ok_or("Missing synthetic note")?;
                let RecordData::Note(mut note) = record.data else {
                    return Err("Wrong synthetic note kind".into());
                };
                note.markdown.push_str(&format!("\nRevision {revision}: coordinator confirmed inspection evidence {index}-{revision}; external delivery remains waiting.\n"));
                body.clone_from(&note.markdown);
                commit(
                    &mut app,
                    &format!("qualification-revision-{index}-{revision}"),
                    vec![Put {
                        id: id.clone(),
                        expected_version: Some(record.version),
                        archived: false,
                        data: RecordData::Note(note),
                    }],
                )?;
            }
            revised += 1;
        }
        current_bytes += body.len();
        ids.push(id);
    }
    semantic_hash.update(IMAGE);
    let threads = notes / 10;
    let mut selected_thread = String::new();
    for index in 0..threads {
        let key = format!("qualification-thread-{index}");
        let op = app.store.allocate_operation(&key)?;
        let thread = op.creation_id(0);
        commit(
            &mut app,
            &key,
            vec![
                new_put(
                    thread.clone(),
                    RecordData::Thread(Thread {
                        private_context: false,
                        context_epoch: 0,
                        title: format!("Quay reply discussion {index}"),
                        state: ThreadState::Open,
                        attention: if index % 3 == 0 {
                            vec![Attention {
                                kind: AttentionKind::Question,
                                reason:
                                    "Confirm inspection evidence before agreeing a delivery date"
                                        .into(),
                                record: Some(ids[index * 10].clone()),
                            }]
                        } else {
                            vec![]
                        },
                    }),
                ),
                new_put(
                    op.creation_id(1),
                    RecordData::Link(Link {
                        from: thread.clone(),
                        to: ids[index * 10].clone(),
                        relation: "context".into(),
                    }),
                ),
                new_put(
                    op.creation_id(2),
                    RecordData::Action(Action {
                        kept_outside_project: false,
                        chat: None,
                        description: format!("Confirm quay inspection {index}"),
                        state: ActionState::Waiting,
                        internal: false,
                        evidence: Some(ids[index * 10].clone()),
                        due: Some("2026-10-22".into()),
                    }),
                ),
            ],
        )?;
        for message in 0..12 {
            let key = format!("qualification-message-{index}-{message}");
            let op = app.store.allocate_operation(&key)?;
            let text = format!(
                "Synthetic conversation {index}, turn {message}. Compare the supplier request with the current quay note. The reply should preserve the agreed scope, mention the inspection evidence, and ask for confirmation of the remaining crane check.\n\nDraft: Thank you for the update. We can confirm capacity after inspection; the delivery Action remains Waiting.\n"
            );
            semantic_hash.update(text.as_bytes());
            commit(
                &mut app,
                &key,
                vec![new_put(
                    op.creation_id(0),
                    RecordData::Message(Message {
                        thread: thread.clone(),
                        role: if message % 2 == 0 {
                            "owner"
                        } else {
                            "assistant"
                        }
                        .into(),
                        text,
                    }),
                )],
            )?;
        }
        if index == threads / 2 {
            selected_thread = thread;
        }
    }
    let records = app.records()?.len();
    let receipts = app.history()?.len();
    app.export_now(&root.join("snapshot"))?;
    drop(app); // Last SQLite connection closes/checkpoints before size reporting.
    let corpus = Corpus {
        format: FORMAT.into(),
        source: source.into(),
        notes,
        short_notes: notes * 15 / 20,
        medium_notes: notes * 4 / 20,
        long_notes: notes / 20,
        current_markdown_bytes: current_bytes,
        managed_images: images,
        image_bytes: images * IMAGE.len(),
        sources: images,
        revised_notes: revised,
        additional_note_revisions: revised * 3,
        threads,
        messages: threads * 12,
        actions: threads,
        records,
        receipts,
        database_bytes: fs::metadata(root.join("data").join(DATABASE_FILENAME))?.len(),
        snapshot_bytes: directory_bytes(&root.join("snapshot"))?,
        create_ms: started.elapsed().as_secs_f64() * 1000.0,
        semantic_content_sha256: format!("{:x}", semantic_hash.finalize()),
        selected_note_index: notes / 2 / 20 * 20,
        selected_note: ids[notes / 2 / 20 * 20].clone(),
        selected_thread,
    };
    fs::write(
        root.join("corpus.json"),
        serde_json::to_vec_pretty(&corpus)?,
    )?;
    Ok(corpus)
}

fn timed<T>(samples: usize, mut operation: impl FnMut() -> Result<T>) -> Result<(T, Value)> {
    let mut values = vec![];
    let mut last = None;
    for _ in 0..samples {
        let started = Instant::now();
        last = Some(operation()?);
        values.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let mut ordered = values.clone();
    ordered.sort_by(f64::total_cmp);
    let report = json!({ "samples_ms": values, "median_ms": ordered[ordered.len()/2], "max_ms": ordered[ordered.len()-1] });
    Ok((last.ok_or("No measurement samples")?, report))
}

pub fn measure(root: &Path) -> Result<Value> {
    absolute_root(root)?;
    if fs::symlink_metadata(root)?.file_type().is_symlink()
        || fs::symlink_metadata(root.join("corpus.json"))?
            .file_type()
            .is_symlink()
        || fs::metadata(root.join("corpus.json"))?.len() > 16 * 1024
    {
        return Err("Expected a bounded synthetic corpus manifest".into());
    }
    let corpus: Corpus = serde_json::from_slice(&fs::read(root.join("corpus.json"))?)?;
    if corpus.format != FORMAT || !(20..=1000).contains(&corpus.notes) {
        return Err(
            "Only explicitly generated synthetic qualification corpora can be measured".into(),
        );
    }
    let (mut app, open) = timed(1, || Ok(Workspace::open(root.join("data"))?))?;
    let (cold_hits, cold) = timed(1, || {
        Ok(app.search(&format!("quay_tail_{}", corpus.selected_note_index))?)
    })?;
    if !cold_hits.iter().any(|hit| hit.note == corpus.selected_note) {
        return Err("Selected long-note tail was not found".into());
    }
    let (_, warm) = timed(8, || {
        Ok(app.search(&format!("quay_tail_{}", corpus.selected_note_index))?)
    })?;
    let (_, selected_note) = timed(8, || {
        Ok(app
            .store
            .note(&corpus.selected_note)?
            .ok_or("Selected note missing")?)
    })?;
    let (messages, selected_thread) = timed(8, || Ok(app.messages(&corpus.selected_thread)?))?;
    if messages.len() != 12 {
        return Err("Synthetic conversation count changed".into());
    }
    let (_, navigation) = timed(5, || {
        // Calls used to populate views; no rendering/event loop or typing claim.
        Ok((
            app.notes()?.len(),
            app.threads()?.len(),
            app.needs_you()?.len(),
            app.messages(&corpus.selected_thread)?.len(),
        ))
    })?;
    let (history, receipt_read) = timed(1, || Ok(app.history()?))?;
    if history.len() != corpus.receipts {
        return Err("Synthetic receipt count changed".into());
    }
    Ok(json!({
        "corpus": corpus,
        "build": { "profile": if cfg!(debug_assertions) { "dev/test with debug assertions; command specifies exact profile" } else { "release" }, "native_retrieval_feature": cfg!(feature = "native-retrieval"), "architecture": std::env::consts::ARCH, "os": std::env::consts::OS },
        "headless_ms": { "workspace_open": open, "search_cold_index": cold, "search_warm_index": warm, "selected_note_point_read": selected_note, "selected_thread_messages": selected_thread, "navigation_equivalent_reads": navigation, "all_receipts_read": receipt_read },
        "limitations": ["Cold search rebuilds the in-memory index; OS disk caches are not flushed.", "Navigation-equivalent timings measure serialized data reads, excluding rendering and event-loop latency.", "Native startup, typing, and navigation require a separate unlocked graphical app session.", "Collect measurement-process peak RSS with /usr/bin/time -l (macOS) or -v (Linux)."]
    }))
}
