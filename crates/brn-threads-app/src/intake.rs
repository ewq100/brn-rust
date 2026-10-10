use crate::*;
use brn_threads_intake::{ConversionResult, ImportIntent};
impl Workspace {
    pub fn import_full(
        &mut self,
        key: &str,
        title: &str,
        converted: &ConversionResult,
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
            Sha256::digest(serde_json::to_vec(&(title, converted))?)
        );
        let op = self
            .store
            .allocate_bound_operation(&format!("full-import:{key}"), &binding)?;
        let authority = HostAuthority::owner("full-note import request");
        if self.store.prepared(&op).is_ok() {
            return Ok(self.store.apply(&op, &authority)?);
        }
        let records = self.records()?;
        let existing=records.iter().find(|record|matches!(&record.data,RecordData::Source(source) if source.locator==converted.source_reference));
        let source_id = existing
            .map(|record| record.id.clone())
            .unwrap_or_else(|| op.creation_id(0));
        let source_version = converted
            .source_sha256
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let relation = format!("full-import:{source_version}");
        let previously_imported=records.iter().any(|record|!record.archived && matches!(&record.data,RecordData::Link(link) if link.to==source_id && link.relation==relation && records.iter().any(|note|!note.archived && note.id==link.from && matches!(&note.data,RecordData::Note(n) if n.superseded_by.is_none() && n.import.as_ref().is_some_and(|i|i.full_note && i.source==source_id)))));
        let gaps = converted
            .coverage
            .gaps
            .iter()
            .map(|gap| format!("{}: {}", gap.locator, gap.detail))
            .collect::<Vec<_>>();
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
                locator: converted.source_reference.clone(),
                label: title.into(),
                observed_at: observed_at.clone(),
                external_version: None,
                outcome: String::new(),
                gaps: vec![],
            }
        };
        source.observed_at = observed_at;
        if !previously_imported {
            source.external_version = Some(source_version);
            source.outcome = format!("{:?}", converted.coverage.status);
            source.gaps = gaps.clone();
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
            note.import = Some(ImportCoverage {
                source: source_id.clone(),
                full_note: true,
                gaps,
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
                    from: note_id,
                    to: source_id,
                    relation,
                }),
            });
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
        Ok(self.store.apply(&op, &authority)?)
    }
}
