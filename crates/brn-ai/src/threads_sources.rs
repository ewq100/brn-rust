//! Host-held source evidence. Only selected figures can enter checked note writes.
use super::*;
use std::collections::BTreeSet;

// No Debug/Serialize: converted bodies and unselected image bytes are transient.
#[derive(Clone)]
pub struct TransientSource {
    pub reference: String,
    pub text: String,
    pub fingerprint: Option<String>,
    pub report: Option<ImportReport>,
    pub assets: Vec<TransientAsset>,
}
#[derive(Clone)]
pub struct TransientAsset {
    pub name: String,
    pub media_type: String,
    pub sha256: [u8; 32],
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}
impl TransientSource {
    pub fn text(reference: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            reference: reference.into(),
            text: text.into(),
            fingerprint: None,
            report: None,
            assets: vec![],
        }
    }
}
fn asset_link(source: usize, asset: usize) -> String {
    format!("brn-transient-asset://{source}/{asset}")
}
fn host_observed_source(source: &Source, supplied: &BTreeMap<String, TransientSource>) -> bool {
    source.report.is_some()
        || supplied
            .get(&source.locator)
            .is_some_and(|source| source.fingerprint.is_some())
}
pub(super) fn source_metadata(
    sources: &BTreeMap<String, TransientSource>,
    store: &Store,
    reference: &str,
    chat: Option<&str>,
    source_ids: &[String],
) -> AiResult<Value> {
    let source_record = store
        .records_for_chat(chat)
        .map_err(storage)?
        .into_iter()
        .find(|record| {
            !record.archived
                && matches!(&record.data, RecordData::Source(source) if source.locator == reference && source_ids.contains(&record.id))
        })
        .map(|record| json!({"id":record.id,"version":record.version}));
    source_value(sources, reference, source_record)
}
/// The `read_source` value for a reference, given its already-found Source record.
pub(super) fn source_value(
    sources: &BTreeMap<String, TransientSource>,
    reference: &str,
    source_record: Option<Value>,
) -> AiResult<Value> {
    let (source_index, (_, source)) = sources
        .iter()
        .enumerate()
        .find(|(_, (key, _))| key.as_str() == reference)
        .ok_or_else(rejected)?;
    let mut text = source.text.clone();
    let assets: Vec<_> = source.assets.iter().enumerate().map(|(index, asset)| {
        let link = asset_link(source_index, index);
        text = text.replace(&format!("]({})", asset.name), &format!("]({link})"));
        json!({"link":link,"name":asset.name,"media_type":asset.media_type,"width":asset.width,"height":asset.height})
    }).collect();
    Ok(
        json!({"reference":reference,"source_record":source_record,"text":text,"fingerprint":source.fingerprint,"coverage":source.report,"figures":assets}),
    )
}
pub(super) fn validate_sources(sources: &BTreeMap<String, TransientSource>) -> AiResult<()> {
    let mut total = 0usize;
    for source in sources.values() {
        if source.reference.trim().is_empty()
            || source.text.len() > 1024 * 1024
            || source.assets.len() > 64
        {
            return Err(rejected());
        }
        let mut names = BTreeSet::new();
        for asset in &source.assets {
            total = total.checked_add(asset.bytes.len()).ok_or_else(rejected)?;
            if asset.bytes.is_empty()
                || total > 64 * 1024 * 1024
                || asset.width == 0
                || asset.height == 0
                || !matches!(
                    asset.media_type.as_str(),
                    "image/png" | "image/jpeg" | "image/gif"
                )
                || !names.insert(&asset.name)
                || asset.name.is_empty()
            {
                return Err(rejected());
            }
            let actual = Asset {
                note: String::new(),
                source: None,
                media_type: asset.media_type.clone(),
                bytes: asset.bytes.clone(),
            }
            .content_hash();
            let expected: String = asset
                .sha256
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            if actual != expected {
                return Err(rejected());
            }
        }
    }
    Ok(())
}

// A later intake may keep earlier knowledge without supplying the earlier raw
// source again. Use the committed note's evidence, never the model's retagging.
fn retained_useful_report(
    records: &[Record],
    note_id: &str,
    previous: &Note,
    proposed: &Note,
    selected: &[(&TransientSource, &Record)],
) -> Option<ImportReport> {
    let coverage = previous
        .import
        .as_ref()
        .filter(|coverage| !coverage.full_note)?;
    let source_locator = |id: &str| {
        records.iter().find_map(|record| {
            (record.id == id)
                .then_some(&record.data)
                .and_then(|data| match data {
                    RecordData::Source(source) => Some(source.locator.as_str()),
                    _ => None,
                })
        })
    };
    let primary = source_locator(&coverage.source)?;
    let retained_text = previous.markdown.split("\n\n").any(|paragraph| {
        let paragraph = paragraph.trim();
        // Short facts can be substantive. Keep qualifications conservatively
        // for any unchanged paragraph with words or quantities, excluding only
        // blank text and markup-only separators. No semantic replacement is
        // inferred from the paragraph's length.
        paragraph.chars().any(char::is_alphanumeric)
            && proposed
                .markdown
                .split("\n\n")
                .any(|current| current.trim() == paragraph)
    });
    let retained = |reference: &str| {
        retained_text
            || records.iter().any(|record| match &record.data {
                RecordData::Source(source) if source.locator == reference => {
                    let link = format!("](brn://{})", record.id);
                    previous.markdown.contains(&link) && proposed.markdown.contains(&link)
                }
                RecordData::Asset(asset) if asset.note == note_id && !record.archived => {
                    let link = format!("](brn-asset://{})", record.id);
                    asset.source.as_deref().and_then(source_locator) == Some(reference)
                        && previous.markdown.contains(&link)
                        && proposed.markdown.contains(&link)
                }
                _ => false,
            })
    };
    let requalified = |reference: &str| {
        selected.iter().any(|(source, _)| {
            source.reference == reference
                && source.report.as_ref().is_some_and(|report| {
                    report.status == ImportStatus::Complete
                        && !report
                            .issues
                            .iter()
                            .any(|issue| issue.kind == ImportIssueKind::Omission)
                })
        })
    };
    let previous_report = coverage.report.clone().unwrap_or_else(|| ImportReport {
        status: ImportStatus::Unchecked,
        issues: coverage
            .gaps
            .iter()
            .map(|gap| ImportIssue {
                kind: ImportIssueKind::Unverified,
                locator: "document".into(),
                detail: format!("Previously recorded coverage requires inspection: {gap}"),
            })
            .collect(),
    });
    let had_issues = !previous_report.issues.is_empty();
    let mut issues = vec![];
    for issue in previous_report.issues {
        // Multi-source reports already scope each issue by the external locator.
        // Recover that scope so qualifying one source cannot erase another.
        let scoped = records
            .iter()
            .filter_map(|record| match &record.data {
                RecordData::Source(source) => Some(source.locator.as_str()),
                _ => None,
            })
            .filter_map(|reference| {
                issue
                    .locator
                    .strip_prefix(&format!("{reference} · "))
                    .map(|locator| (reference, locator))
            })
            .max_by_key(|(reference, _)| reference.len());
        let (reference, locator) = scoped.unwrap_or((primary, issue.locator.as_str()));
        if !retained(reference) || requalified(reference) {
            continue;
        }
        issues.push(ImportIssue {
            kind: issue.kind,
            locator: format!("{reference} · {locator}"),
            detail: if issue.detail.starts_with("Retained prior input: ") {
                issue.detail
            } else {
                format!("Retained prior input: {}", issue.detail)
            },
        });
    }
    if issues.is_empty() && (had_issues || !retained(primary) || requalified(primary)) {
        // Aggregate Partial may have come solely from the removed source's
        // issues. It is not independent evidence that another retained source
        // remains partial after those issues have been qualified or removed.
        return None;
    }
    let status = if issues
        .iter()
        .any(|issue| issue.kind == ImportIssueKind::Omission)
    {
        ImportStatus::Partial
    } else if issues
        .iter()
        .any(|issue| issue.kind == ImportIssueKind::Unverified)
    {
        ImportStatus::Unchecked
    } else {
        previous_report.status
    };
    Some(ImportReport { status, issues })
}

fn combined_status(first: ImportStatus, second: ImportStatus) -> ImportStatus {
    match (first, second) {
        (ImportStatus::Partial, _) | (_, ImportStatus::Partial) => ImportStatus::Partial,
        (ImportStatus::Unchecked, _) | (_, ImportStatus::Unchecked) => ImportStatus::Unchecked,
        _ => ImportStatus::Complete,
    }
}

fn useful_source_report(source: &TransientSource) -> Option<ImportReport> {
    let mut report = source.report.clone()?;
    if report.status == ImportStatus::Unchecked
        && !report
            .issues
            .iter()
            .any(|issue| issue.kind == ImportIssueKind::Unverified)
    {
        // Persist unchecked status explicitly, including a host report without
        // detailed issues. A later aggregate's Partial status must not hide this
        // independent uncertainty when another source is requalified.
        report.issues.push(ImportIssue {
            kind: ImportIssueKind::Unverified,
            locator: "document".into(),
            detail: "Source coverage was not independently checked for this selected input".into(),
        });
    }
    Some(report)
}

pub(super) fn materialize_selected_sources(
    sources: &BTreeMap<String, TransientSource>,
    store: &Store,
    operation: &OperationId,
    request: &mut ChangeRequest,
    chat: Option<&str>,
    source_ids: &[String],
) -> AiResult<()> {
    let mut records = store.records_for_chat(chat).map_err(storage)?;
    // Re-preparation must use the original evidence snapshot, including after
    // a committed host source observation advanced its version. Changed model
    // requests still fail the core's immutable request comparison.
    if let Ok(prepared) = store.prepared(operation) {
        for write in &prepared.request.writes {
            if matches!(write.data, RecordData::Source(_))
                && let Some(version) = write.expected_version
                && let Some(snapshot) = store.revision(&write.id, version).map_err(storage)?
                && let Some(record) = records.iter_mut().find(|record| record.id == write.id)
            {
                *record = snapshot;
            }
        }
    }
    // A model may name a source or improve its label, but cannot claim checked
    // converter coverage or revise the host's observed source identity.
    for write in &request.writes {
        let RecordData::Source(proposed) = &write.data else {
            continue;
        };
        let current = records.iter().find(|record| record.id == write.id);
        let current = current.and_then(|record| match &record.data {
            RecordData::Source(source) => Some(source),
            _ => None,
        });
        if records.iter().any(|record| record.id != write.id && matches!(&record.data, RecordData::Source(source) if host_observed_source(source, sources) && source.locator == proposed.locator)) {
            return Err(rejected());
        }
        if proposed.report != current.and_then(|source| source.report.clone()) {
            return Err(rejected());
        }
        if let Some(current) = current.filter(|source| host_observed_source(source, sources))
            && (write.archived
                || proposed.locator != current.locator
                || proposed.external_version != current.external_version
                || proposed.outcome != current.outcome
                || proposed.gaps != current.gaps)
        {
            return Err(rejected());
        }
    }
    let source_records: Vec<_> =
        sources
            .iter()
            .enumerate()
            .map(|(index, (reference, source))| {
                let record = records
                .iter()
                .find(|record| {
                    !record.archived
                        && matches!(&record.data, RecordData::Source(s) if s.locator == *reference && source_ids.contains(&record.id))
                })
                .ok_or_else(rejected)?;
                let RecordData::Source(current) = &record.data else {
                    return Err(rejected());
                };
                if source.fingerprint.as_ref().is_some_and(|fingerprint| {
                    current.external_version.as_ref() != Some(fingerprint)
                }) || source
                    .report
                    .as_ref()
                    .is_some_and(|report| current.report.as_ref() != Some(report))
                {
                    return Err(rejected());
                }
                Ok((index, source, record))
            })
            .collect::<AiResult<_>>()?;
    let mut used: BTreeSet<_> = request
        .writes
        .iter()
        .map(|write| write.id.clone())
        .collect();
    let linked: Vec<_> = request
        .writes
        .iter()
        .filter_map(|write| match &write.data {
            RecordData::Link(link) => Some((link.from.clone(), link.to.clone())),
            _ => None,
        })
        .collect();
    let mut assets = Vec::new();
    let mut checked_sources = request
        .inputs
        .iter()
        .filter(|input| {
            source_records
                .iter()
                .any(|(_, _, source)| source.id == input.record)
        })
        .map(|input| input.record.clone())
        .collect::<BTreeSet<_>>();
    for write in &mut request.writes {
        let RecordData::Note(note) = &mut write.data else {
            continue;
        };
        // Useful intake cannot invent a full-import classification or checked coverage.
        // Existing protected full imports retain their own original coverage snapshot.
        // Retry against the exact original note, even after this operation has
        // applied. Receipt replay must not accumulate its own coverage as new
        // historical input and change the immutable prepared request.
        let previous_record = if let Some(version) = write.expected_version {
            store.revision(&write.id, version).map_err(storage)?
        } else {
            None
        };
        let existing_note = previous_record
            .as_ref()
            .and_then(|record| match &record.data {
                RecordData::Note(note) => Some(note),
                _ => None,
            });
        let existing_full_import = existing_note
            .and_then(|note| note.import.as_ref().filter(|import| import.full_note))
            .cloned();
        let mut selected_sources = vec![];
        for (source_index, source, record) in &source_records {
            let mut selected = source_records.len() == 1
                || linked
                    .iter()
                    .any(|(from, to)| from == &write.id && to == &record.id)
                || request.inputs.iter().any(|input| input.record == record.id)
                || note
                    .import
                    .as_ref()
                    .is_some_and(|import| import.source == record.id);
            for (asset_index, asset) in source.assets.iter().enumerate() {
                let transient_link = format!("]({})", asset_link(*source_index, asset_index));
                if !note.markdown.contains(&transient_link) {
                    continue;
                }
                selected = true;
                let id = (0..64)
                    .map(|index| operation.creation_id(index))
                    .find(|id| !used.contains(id))
                    .ok_or_else(rejected)?;
                used.insert(id.clone());
                note.markdown = note
                    .markdown
                    .replace(&transient_link, &format!("](brn-asset://{id})"));
                assets.push(Put {
                    id,
                    expected_version: None,
                    archived: false,
                    data: RecordData::Asset(Asset {
                        note: write.id.clone(),
                        source: Some(record.id.clone()),
                        media_type: asset.media_type.clone(),
                        bytes: asset.bytes.clone(),
                    }),
                });
            }
            if selected {
                note.import = Some(existing_full_import.clone().unwrap_or_else(|| {
                    let report = useful_source_report(source);
                    ImportCoverage {
                        source: record.id.clone(),
                        full_note: false,
                        gaps: report
                            .as_ref()
                            .map(|report| report.issues.iter().map(ImportIssue::display).collect())
                            .unwrap_or_default(),
                        report,
                    }
                }));
            }
            if selected && !request.inputs.iter().any(|input| input.record == record.id) {
                request.inputs.push(RevisionInput {
                    expectation: InputExpectation::Current,
                    record: record.id.clone(),
                    version: record.version,
                });
            }
            if selected {
                checked_sources.insert(record.id.clone());
                selected_sources.push((*source, *record));
            }
        }
        if existing_full_import.is_none() && selected_sources.len() > 1 {
            let mut report = ImportReport {
                status: ImportStatus::Complete,
                issues: vec![],
            };
            for (source, _) in &selected_sources {
                let status = source
                    .report
                    .as_ref()
                    .map(|report| report.status)
                    .unwrap_or(ImportStatus::Unchecked);
                report.status = combined_status(report.status, status);
                if let Some(source_report) = useful_source_report(source) {
                    report
                        .issues
                        .extend(source_report.issues.iter().map(|issue| ImportIssue {
                            kind: issue.kind,
                            locator: format!("{} · {}", source.reference, issue.locator),
                            detail: issue.detail.clone(),
                        }));
                } else {
                    report.issues.push(ImportIssue {
                        kind: ImportIssueKind::Unverified,
                        locator: source.reference.clone(),
                        detail: "Coverage was not independently recorded for this selected source"
                            .into(),
                    });
                }
            }
            note.import = Some(ImportCoverage {
                source: selected_sources[0].1.id.clone(),
                full_note: false,
                gaps: report.issues.iter().map(ImportIssue::display).collect(),
                report: Some(report),
            });
        }
        if existing_full_import.is_none()
            && !selected_sources.is_empty()
            && let Some(previous) = existing_note
            && let Some(retained) =
                retained_useful_report(&records, &write.id, previous, note, &selected_sources)
            && let Some(coverage) = &mut note.import
        {
            let report = coverage.report.get_or_insert_with(|| ImportReport {
                status: ImportStatus::Unchecked,
                issues: vec![ImportIssue {
                    kind: ImportIssueKind::Unverified,
                    locator: "document".into(),
                    detail: "Coverage was not independently recorded for this selected source"
                        .into(),
                }],
            });
            report.status = combined_status(report.status, retained.status);
            for issue in retained.issues {
                if !report.issues.contains(&issue) {
                    report.issues.push(issue);
                }
            }
            coverage.gaps = report.issues.iter().map(ImportIssue::display).collect();
        }
        if note.markdown.contains("brn-transient-asset://") {
            return Err(rejected());
        }
    }
    for (_, _, record) in &source_records {
        if checked_sources.contains(&record.id)
            && !request.writes.iter().any(|write| write.id == record.id)
        {
            // This unchanged host observation carries an expected version into
            // the shared SQLite writer transaction, including later owner review.
            // A pre-Apply comparison alone cannot close a cross-process race.
            request.writes.push(Put {
                id: record.id.clone(),
                expected_version: Some(record.version),
                archived: false,
                data: record.data.clone(),
            });
        }
    }
    if request.writes.len() + assets.len() > 64 {
        return Err(rejected());
    }
    request.writes.extend(assets);
    Ok(())
}

/// Early readable Stale result; the expected Source writes also enforce this
/// check atomically in the core. Applied receipts replay before this check.
pub(super) fn changed_prepared_sources(
    store: &Store,
    request: &ChangeRequest,
) -> AiResult<Vec<String>> {
    let mut changed = vec![];
    for write in &request.writes {
        if matches!(write.data, RecordData::Source(_))
            && write.expected_version.is_some()
            && request.inputs.iter().any(|input| input.record == write.id)
            && store
                .record(&write.id)
                .map_err(storage)?
                .as_ref()
                .map(|record| record.version)
                != write.expected_version
        {
            changed.push(write.id.clone());
        }
    }
    Ok(changed)
}
