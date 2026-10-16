use crate::*;
use brn_threads_intake::{ConversionResult, CoverageStatus, GapKind, ImportIntent};
use std::collections::BTreeSet;

/// The owner instruction for useful intake of a source, the same from the
/// desktop and the CLI, so a reused intake chat can reuse it.
pub const SOURCE_INTAKE_INSTRUCTION: &str = "Capture useful supported information from this source, maintaining relevant ordinary notes and source references. Preserve qualifications; do not copy the whole source as a note. Retain meaningful selected figures using the read_source host links and preserve detected omissions; unchecked coverage does not mean proven loss.";

/// Preserve the converter's distinction between a detected loss and unchecked coverage.
pub fn import_report(converted: &ConversionResult) -> ImportReport {
    ImportReport {
        status: match converted.coverage.status {
            CoverageStatus::Complete => ImportStatus::Complete,
            CoverageStatus::Partial => ImportStatus::Partial,
            CoverageStatus::Unchecked => ImportStatus::Unchecked,
        },
        issues: converted
            .coverage
            .gaps
            .iter()
            .map(|gap| ImportIssue {
                kind: match gap.kind {
                    GapKind::Limitation => ImportIssueKind::Limitation,
                    GapKind::Omission => ImportIssueKind::Omission,
                    GapKind::Unverified => ImportIssueKind::Unverified,
                },
                locator: gap.locator.clone(),
                detail: gap.detail.clone(),
            })
            .collect(),
    }
}
impl Workspace {
    /// The chat for useful intake of `locator`: the most recent open chat in
    /// the same scope (`project`, or general when `None`) whose Run already
    /// received this source, so a retry or re-supply lands beside the earlier
    /// outcome. A chat whose Working Run is still live in some process is
    /// skipped, because the next Run there would interrupt it. Otherwise a new
    /// chat titled `title`. `true` means reused.
    pub fn source_intake_chat(
        &mut self,
        project: Option<&str>,
        locator: &str,
        title: &str,
    ) -> AppResult<(Record, bool)> {
        let records = self.records()?;
        if let Some(project) = project
            && !records
                .iter()
                .any(|r| r.id == project && !r.archived && matches!(r.data, RecordData::Project(_)))
        {
            return Err(AppError::Invalid(
                "Choose an active project for this source".into(),
            ));
        }
        // One file is one source however its path is spelled, including a
        // Source saved under an older spelling.
        let sources = matching_sources(&records, &canonical_locator(locator));
        if !sources.is_empty() {
            // Receipt order is durable; a Run's first write is its creation.
            let mut created = BTreeMap::new();
            for (position, receipt) in self.history()?.into_iter().enumerate() {
                for write in receipt.writes {
                    if matches!(write.after.data, RecordData::Run(_)) {
                        created.entry(write.after.id).or_insert(position);
                    }
                }
            }
            // `begin()` interrupts every Working Run in its chat. Skip a chat
            // whose Working Run still holds its lease; an orphaned one (lease
            // free) does not block reuse, as the next Run there recovers it.
            // Unknown liveness counts as live.
            let derived = self.store.derived_directory().ok();
            let busy = |chat: &str| {
                records.iter().any(|r| {
                    matches!(&r.data, RecordData::Run(run) if run.thread == chat && run.state == RunState::Working)
                        && derived
                            .as_deref()
                            .is_none_or(|d| brn_ai::run_lease::run_is_live(d, &r.id))
                })
            };
            let reused = records
                .iter()
                .filter_map(|r| match &r.data {
                    RecordData::Run(run) if run.sources.iter().any(|s| sources.contains(s.as_str())) => {
                        Some((created.get(&r.id).copied().unwrap_or_default(), &run.thread))
                    }
                    _ => None,
                })
                .filter(|(_, chat)| {
                    records.iter().any(|r| {
                        r.id == **chat
                            && !r.archived
                            && matches!(&r.data, RecordData::Thread(t) if t.state == ThreadState::Open)
                    }) && project_for_chat(&records, chat).map(|p| p.id.as_str()) == project
                        && !busy(chat)
                })
                .max_by_key(|(position, _)| *position)
                .and_then(|(_, chat)| records.iter().find(|r| r.id == *chat));
            if let Some(chat) = reused {
                return Ok((chat.clone(), true));
            }
        }
        let chat = if let Some(project) = project {
            self.project_command(
                &format!("source-chat:{}", uuid::Uuid::new_v4()),
                ProjectCommand::CreateChat {
                    project: project.into(),
                    title: title.into(),
                },
            )?
            .writes
            .into_iter()
            .find(|w| w.before.is_none() && matches!(w.after.data, RecordData::Thread(_)))
            .map(|w| w.after)
            .ok_or_else(|| AppError::Invalid("Project chat was not created".into()))?
        } else {
            self.new_thread(title)?
        };
        Ok((chat, false))
    }
    /// Posts [`SOURCE_INTAKE_INSTRUCTION`] in `chat` before a useful intake
    /// Run for `locator`, unless that Run reuses the chat's latest instruction
    /// for this source: then nothing is written and the result is `None`.
    /// New owner text since then, or another source, gets a fresh instruction.
    pub fn post_intake_instruction(
        &mut self,
        chat: &str,
        locator: &str,
    ) -> AppResult<Option<Receipt>> {
        if brn_ai::threads::intake_instruction_to_reuse(
            &self.store,
            chat,
            SOURCE_INTAKE_INSTRUCTION,
            &[locator],
        )?
        .is_some()
        {
            return Ok(None);
        }
        self.post_message(chat, SOURCE_INTAKE_INSTRUCTION).map(Some)
    }
    pub fn import_full(
        &mut self,
        key: &str,
        title: &str,
        converted: &ConversionResult,
    ) -> AppResult<ApplyOutcome> {
        self.import_full_scoped(key, title, converted, None)
    }
    pub fn attach_full(
        &mut self,
        key: &str,
        thread: &str,
        title: &str,
        converted: &ConversionResult,
    ) -> AppResult<ApplyOutcome> {
        self.import_full_scoped(key, title, converted, Some(thread))
    }
    pub fn import_full_scoped(
        &mut self,
        key: &str,
        title: &str,
        converted: &ConversionResult,
        chat: Option<&str>,
    ) -> AppResult<ApplyOutcome> {
        if converted.intent != ImportIntent::FullNote {
            return Err(AppError::Invalid(
                "Useful-information intake needs semantic selection, not a full-copy note".into(),
            ));
        }
        if converted.source_reference.trim().is_empty() {
            return Err(AppError::Invalid(
                "Full import requires an external source reference".into(),
            ));
        }
        for asset in &converted.assets {
            let actual: [u8; 32] = Sha256::digest(&asset.bytes).into();
            if asset.name.is_empty() || actual != asset.sha256 {
                return Err(AppError::Invalid(
                    "Converted asset identity failed verification".into(),
                ));
            }
        }
        let binding = format!(
            "{:x}",
            Sha256::digest(if let Some(chat) = chat {
                serde_json::to_vec(&("chat-attachment", title, converted, chat))?
            } else {
                serde_json::to_vec(&(title, converted))?
            })
        );
        let op = self
            .store
            .allocate_bound_operation(&format!("full-import:{key}"), &binding)?;
        let authority = HostAuthority::owner("full-note import request");
        // A project import stops that project's Working Runs; their free
        // lease files go with them on every applied path below.
        if self.store.receipt(&op)?.is_some() {
            let outcome = self.store.apply(&op, &authority)?;
            return Ok(self.remove_leases_ended_by(outcome));
        }
        if let Some(thread) = chat
            && !matches!(
                self.store.record(thread)?,
                Some(Record {
                    archived: false,
                    data: RecordData::Thread(Thread {
                        state: ThreadState::Open,
                        ..
                    }) | RecordData::Project(_),
                    ..
                })
            )
        {
            return Err(AppError::Invalid(
                "Choose an active chat for this attachment".into(),
            ));
        }
        if self.store.prepared(&op).is_ok() {
            let outcome = self.store.apply(&op, &authority)?;
            return Ok(self.remove_leases_ended_by(outcome));
        }
        let records = self.records()?;
        // The exact real path first, then a Source saved under an older
        // spelling of the same file; the Put below records the real path.
        let locator = canonical_locator(&converted.source_reference);
        let existing = find_source(&records, &locator, |source| source.chat.as_deref() == chat);
        let source_id = existing
            .map(|record| record.id.clone())
            .unwrap_or_else(|| op.creation_id(0));
        let source_version = converted
            .source_sha256
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let relation = format!("{}{source_version}", crate::relations::FULL_IMPORT_PREFIX);
        let previously_imported=records.iter().any(|record|!record.archived && matches!(&record.data,RecordData::Link(link) if link.to==source_id && link.relation==relation && records.iter().any(|note|!note.archived && note.id==link.from && matches!(&note.data,RecordData::Note(n) if n.superseded_by.is_none() && n.import.as_ref().is_some_and(|i|i.full_note && i.source==source_id)))));
        let gaps = converted
            .coverage
            .gaps
            .iter()
            .map(|gap| format!("{}: {}", gap.locator, gap.detail))
            .collect::<Vec<_>>();
        let report = import_report(converted);
        let observed_at = format!(
            "Unix seconds {}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| AppError::Invalid("System clock precedes epoch".into()))?
                .as_secs()
        );
        let mut source = if let Some(Record {
            data: RecordData::Source(source),
            ..
        }) = existing
        {
            source.clone()
        } else {
            Source {
                chat: chat.map(str::to_owned),
                locator: locator.clone(),
                label: title.into(),
                observed_at: observed_at.clone(),
                external_version: None,
                outcome: String::new(),
                gaps: vec![],
                report: None,
            }
        };
        source.locator = locator;
        source.observed_at = observed_at;
        if !previously_imported {
            source.external_version = Some(source_version);
            source.outcome = format!("{:?}", converted.coverage.status);
            source.gaps = gaps.clone();
            source.report = Some(report.clone());
        }
        let mut writes = vec![Put {
            id: source_id.clone(),
            expected_version: existing.map(|record| record.version),
            archived: false,
            data: RecordData::Source(source),
        }];
        if !previously_imported {
            let note_id = op.creation_id(1);
            let mut markdown = converted.markdown.clone();
            for (index, asset) in converted.assets.iter().enumerate() {
                let id = op.creation_id(index + 2);
                markdown = markdown.replace(
                    &format!("]({})", asset.name),
                    &format!("](brn-asset://{id})"),
                );
                writes.push(Put {
                    id,
                    expected_version: None,
                    archived: false,
                    data: RecordData::Asset(Asset {
                        note: note_id.clone(),
                        source: Some(source_id.clone()),
                        media_type: asset.media_type.clone(),
                        bytes: asset.bytes.clone(),
                    }),
                });
            }
            let mut note = Note::working(title, markdown);
            note.protected = true;
            note.chat = chat.map(str::to_owned);
            note.import = Some(ImportCoverage {
                source: source_id.clone(),
                full_note: true,
                gaps,
                report: Some(report),
            });
            writes.push(Put {
                id: note_id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Note(note),
            });
            writes.push(Put {
                id: op.creation_id(converted.assets.len() + 2),
                expected_version: None,
                archived: false,
                data: RecordData::Link(Link {
                    from: note_id.clone(),
                    to: source_id,
                    relation,
                }),
            });
            if let Some(chat) = chat {
                if let Some(mut project) = records
                    .iter()
                    .find(|r| r.id == chat && matches!(r.data, RecordData::Project(_)))
                    .cloned()
                {
                    let RecordData::Project(p) = &mut project.data else {
                        unreachable!()
                    };
                    writes.extend(project_context_fences(&records, &p.chats)?);
                    // The same Runs the fences stop: their chats get the
                    // notice and a Review for each candidate left.
                    let ended: Vec<_> = records
                        .iter()
                        .filter_map(|r| match &r.data {
                            RecordData::Run(run)
                                if run.state == RunState::Working
                                    && p.chats.contains(&run.thread) =>
                            {
                                Some((r.id.clone(), run.clone()))
                            }
                            _ => None,
                        })
                        .collect();
                    brn_threads_core::run_attention::ended_run_attention(
                        &mut writes,
                        &ended,
                        true,
                        |id| Ok(records.iter().find(|r| r.id == id).cloned()),
                        |operations| self.store.active_candidates(operations),
                    )?;
                    p.documents.push(note_id.clone());
                    writes.push(Put {
                        id: project.id,
                        expected_version: Some(project.version),
                        archived: false,
                        data: project.data,
                    });
                }
                writes.push(Put {
                    id: op.creation_id(converted.assets.len() + 3),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Link(Link {
                        from: chat.into(),
                        to: note_id,
                        relation: "attachment".into(),
                    }),
                });
            }
        }
        let request = ChangeRequest {
            reason: if previously_imported {
                format!("Observed previously imported full note: {title}")
            } else {
                format!(
                    "Full-note import: {title} ({:?})",
                    converted.coverage.status
                )
            },
            writes,
            inputs: vec![],
        };
        self.store.prepare_owner(&op, &request, &authority)?;
        let outcome = self.store.apply(&op, &authority)?;
        Ok(self.remove_leases_ended_by(outcome))
    }
    pub fn keep_in_brn(
        &mut self,
        key: &str,
        note_id: &str,
        version: u64,
    ) -> AppResult<ApplyOutcome> {
        let binding = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&("keep-in-brn", note_id, version))?)
        );
        let op = self
            .store
            .allocate_bound_operation(&format!("keep-in-brn:{key}"), &binding)?;
        let owner = HostAuthority::owner("owner retained a chat attachment in BRN");
        if self.store.prepared(&op).is_ok() {
            return Ok(self.store.apply(&op, &owner)?);
        }
        let record = self
            .store
            .note(note_id)?
            .ok_or_else(|| AppError::Invalid("Attachment missing".into()))?;
        if record.archived {
            return Err(AppError::Invalid(
                "Restore this private document before keeping it in BRN".into(),
            ));
        }
        if record.version != version {
            return Ok(ApplyOutcome::Stale {
                records: vec![note_id.into()],
            });
        }
        let RecordData::Note(mut note) = record.data else {
            unreachable!()
        };
        if note.chat.is_none() {
            return Err(AppError::Invalid("This note is already in BRN".into()));
        }
        let original_chat = note.chat.clone();
        note.chat = None;
        let records = self.records()?;
        let retention_context = records
            .iter()
            .find(|r| r.id == note_id)
            .filter(|r| record_in_chat(r, &records, original_chat.as_deref()))
            .map(|_| original_chat.clone())
            .unwrap_or_else(|| {
                records
                    .iter()
                    .find(|r| {
                        !r.archived
                            && matches!(r.data, RecordData::Project(_))
                            && project_document_ids(&records, r).contains(note_id)
                    })
                    .map(|r| r.id.clone())
            });
        let mut writes = Vec::new();
        let mut inputs = Vec::new();
        if let Some(import) = &mut note.import {
            let source = self
                .store
                .record(&import.source)?
                .ok_or_else(|| AppError::Invalid("Attachment source missing".into()))?;
            inputs.push(RevisionInput {
                record: source.id.clone(),
                version: source.version,
                expectation: InputExpectation::Current,
            });
            if let RecordData::Source(mut observation) = source.data {
                let old_source = source.id;
                // The global Source for the same file, under any spelling; a
                // new one records the real path.
                let locator = canonical_locator(&observation.locator);
                let global = find_source(&records, &locator, |s| s.chat.is_none());
                let source_id = global
                    .map(|r| r.id.clone())
                    .unwrap_or_else(|| op.creation_id(0));
                if global.is_none() {
                    observation.chat = None;
                    observation.locator = locator;
                    writes.push(Put {
                        id: source_id.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Source(observation),
                    });
                }
                import.source = source_id.clone();
                for r in &records {
                    let mut data = r.data.clone();
                    match &mut data {
                        RecordData::Asset(a)
                            if a.note == note_id && a.source.as_ref() == Some(&old_source) =>
                        {
                            a.source = Some(source_id.clone())
                        }
                        RecordData::Link(l) if l.from == note_id && l.to == old_source => {
                            l.to = source_id.clone()
                        }
                        _ => continue,
                    }
                    writes.push(Put {
                        id: r.id.clone(),
                        expected_version: Some(r.version),
                        archived: r.archived,
                        data,
                    });
                }
            }
        }
        let mut retained = BTreeMap::new();
        for id in managed_asset_ids(&note.markdown)
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
        {
            let record = records
                .iter()
                .find(|r| r.id == id && !r.archived && matches!(r.data, RecordData::Asset(_)))
                .ok_or_else(|| {
                    AppError::Invalid(
                        "A referenced figure is missing; inspect this attachment before keeping it"
                            .into(),
                    )
                })?;
            if !record_in_chat(record, &records, retention_context.as_deref()) {
                return Err(AppError::Invalid(
                    "A referenced figure belongs to another chat".into(),
                ));
            }
            let RecordData::Asset(asset) = &record.data else {
                unreachable!()
            };
            let owned = asset.note == note_id;
            if record_in_chat(record, &records, None)
                || owned
                    && writes
                        .iter()
                        .any(|w| w.id == record.id && matches!(w.data, RecordData::Asset(_)))
            {
                continue;
            }
            let mut copy = asset.clone();
            copy.note = note_id.into();
            if let Some(source) = &asset.source {
                let source_record = records
                    .iter()
                    .find(|r| r.id == *source)
                    .ok_or_else(|| AppError::Invalid("Figure source missing".into()))?;
                if let RecordData::Source(observation) = &source_record.data
                    && observation.chat.is_some()
                {
                    // Staged global Sources already record the real path.
                    let locator = canonical_locator(&observation.locator);
                    let global = find_source(&records, &locator, |s| s.chat.is_none());
                    let staged=writes.iter().find(|w|matches!(&w.data,RecordData::Source(s) if s.chat.is_none() && s.locator==locator));
                    let source_id = global
                        .map(|r| r.id.clone())
                        .or_else(|| staged.map(|w| w.id.clone()))
                        .unwrap_or_else(|| op.creation_id(writes.len() + 1));
                    if global.is_none() && staged.is_none() {
                        let mut observation = observation.clone();
                        observation.chat = None;
                        observation.locator = locator;
                        writes.push(Put {
                            id: source_id.clone(),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Source(observation),
                        });
                    }
                    copy.source = Some(source_id);
                    inputs.push(RevisionInput {
                        record: source.clone(),
                        version: source_record.version,
                        expectation: InputExpectation::Current,
                    });
                }
            }
            if owned {
                if copy.source != asset.source {
                    writes.push(Put {
                        id: record.id.clone(),
                        expected_version: Some(record.version),
                        archived: false,
                        data: RecordData::Asset(copy),
                    });
                }
                continue;
            }
            let new_id = op.creation_id(writes.len() + 1);
            writes.push(Put {
                id: new_id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Asset(copy),
            });
            retained.insert(id.clone(), format!("brn-asset://{new_id}"));
            inputs.push(RevisionInput {
                record: id,
                version: record.version,
                expectation: InputExpectation::Current,
            });
        }
        inputs.sort_by(|a, b| a.record.cmp(&b.record).then(a.version.cmp(&b.version)));
        inputs.dedup_by(|a, b| {
            a.record == b.record && a.version == b.version && a.expectation == b.expectation
        });
        note.markdown = crate::assets::rewrite(&note.markdown, |id| {
            Some(
                retained
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| format!("brn-asset://{id}")),
            )
        })?;
        writes.push(Put {
            id: note_id.into(),
            expected_version: Some(version),
            archived: false,
            data: RecordData::Note(note),
        });
        self.store.prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Keep this attachment in BRN".into(),
                writes,
                inputs,
            },
            &owner,
        )?;
        Ok(self.store.apply(&op, &owner)?)
    }
}
