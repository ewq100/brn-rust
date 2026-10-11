use crate::*;

impl Workspace {
    /// Current records flagged by this historical update/Undo receipt, without
    /// claiming semantic dependence or that a later revision still needs editing.
    pub fn refresh_records(&self, receipt: &str) -> AppResult<Vec<Record>> {
        let operation = self.store.operation(receipt)?;
        let receipt = self
            .store
            .receipt(&operation)?
            .ok_or_else(|| AppError::Invalid("Change has no receipt".into()))?;
        receipt
            .needs_refresh
            .iter()
            .filter_map(|id| self.store.record(id).transpose())
            .map(|record| record.map_err(AppError::from))
            .collect()
    }

    /// One owner-requested ordinary follow-up per receipt. Repeated clicks and
    /// restarts return the same thread; no dependent content changes automatically.
    pub fn refresh_followup(&mut self, receipt: &str) -> AppResult<Record> {
        let original = self.store.operation(receipt)?;
        let receipt = self
            .store
            .receipt(&original)?
            .ok_or_else(|| AppError::Invalid("Change has no receipt".into()))?;
        if receipt.needs_refresh.is_empty() {
            return Err(AppError::Invalid(
                "This change did not flag directly derived records".into(),
            ));
        }
        let key = format!("owner-refresh-followup:{}", original.as_str());
        let operation = self.store.allocate_operation(&key)?;
        let thread = operation.creation_id(0);
        let mut writes = vec![
            Put {
                id: thread.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Thread(Thread {
                    title: "Refresh related work".into(),
                    state: ThreadState::Open,
                    attention: vec![],
                }),
            },
            Put {
                id: operation.creation_id(1),
                expected_version: None,
                archived: false,
                data: RecordData::Message(Message {
                    thread: thread.clone(),
                    role: "owner".into(),
                    text: format!(
                        "Review the linked directly derived records after this change: {}. Read current inputs and current records; determine whether each needs refreshing. Preserve protected writing and human completion rules. This follow-up does not itself change the linked work.",
                        receipt.reason
                    ),
                }),
            },
        ];
        writes.extend(
            receipt
                .needs_refresh
                .iter()
                .enumerate()
                .map(|(index, id)| Put {
                    id: operation.creation_id(index + 2),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Link(Link {
                        from: thread.clone(),
                        to: id.clone(),
                        relation: "refresh".into(),
                    }),
                }),
        );
        writes.extend(receipt.writes.iter().enumerate().map(|(index, write)| Put {
            id: operation.creation_id(index + 2 + receipt.needs_refresh.len()),
            expected_version: None,
            archived: false,
            data: RecordData::Link(Link {
                from: thread.clone(),
                to: write.after.id.clone(),
                relation: "changed-input".into(),
            }),
        }));
        self.owner_change(
            &key,
            &ChangeRequest {
                reason: "Owner opened one follow-up for directly derived work".into(),
                writes,
                inputs: vec![],
            },
        )?;
        self.store
            .record(&thread)?
            .ok_or_else(|| AppError::Invalid("Refresh follow-up was not applied".into()))
    }
}
