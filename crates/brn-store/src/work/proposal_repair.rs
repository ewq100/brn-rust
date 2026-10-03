//! Bounded explicit repair admission/history. No filesystem effects.
use super::{
    WorkStore, now_ms,
    proposal_apply::{
        self, ApplyJournal, ApplyMemberPhase, ApplyMemberProof, ApplyOutcome, RepairAttempt,
        RepairBinding, RepairPreview, RepairReceipt, RepairRequest,
    },
    proposals::{self, NoteChange},
};
use crate::{Error, Result, hash, invalid};
use rusqlite::Connection;
use std::collections::HashSet;
use uuid::Uuid;

const MAX_ATTEMPTS: usize = 64;

fn phases(
    journal: &ApplyJournal,
    observations: &[ApplyMemberProof],
) -> Result<Vec<ApplyMemberPhase>> {
    let prepared = journal
        .prepared
        .as_ref()
        .ok_or_else(|| Error::StateChanged("repair requires complete prepared proofs".into()))?;
    if observations.len() != journal.members.len() {
        return Err(invalid("repair requires a complete observation vector"));
    }
    journal
        .approved
        .draft
        .changes
        .iter()
        .zip(prepared)
        .zip(observations)
        .map(|((change, prepared), proof)| {
            let (before, applied) = match change {
                NoteChange::Create { .. } => (
                    proof.destination.is_none() && proof.staging.as_ref() == Some(prepared),
                    proof.destination.as_ref() == Some(prepared) && proof.staging.is_none(),
                ),
                NoteChange::Replace { before, .. } => (
                    proof.destination.as_ref() == Some(before)
                        && proof.staging.as_ref() == Some(prepared),
                    proof.destination.as_ref() == Some(prepared)
                        && proof.staging.as_ref() == Some(before),
                ),
                NoteChange::Trash { before, .. } => (
                    proof.destination.as_ref() == Some(before) && proof.staging.is_none(),
                    proof.destination.is_none() && proof.staging.as_ref() == Some(before),
                ),
            };
            if before {
                Ok(ApplyMemberPhase::Before)
            } else if applied {
                Ok(ApplyMemberPhase::Applied)
            } else {
                Err(Error::StateChanged(
                    "repair encountered an unknown member phase".into(),
                ))
            }
        })
        .collect()
}

fn stamp(
    journal: &ApplyJournal,
    observations: &[ApplyMemberProof],
    prior_ids: &[Uuid],
) -> Result<[u8; 32]> {
    Ok(hash(&proposal_apply::encode(&(
        &journal.approved.draft,
        &journal.request,
        journal.creation_sha256,
        &journal.members,
        &journal.prepared,
        &journal.undo,
        prior_ids,
        observations,
    ))?))
}

fn validate_request(request: &RepairRequest) -> Result<()> {
    proposals::nonnil(request.id)?;
    proposals::nonnil(request.operation_id)?;
    if request.id == request.operation_id {
        return Err(invalid("repair requires a distinct attempt UUID"));
    }
    Ok(())
}

pub(super) fn validate(journal: &ApplyJournal) -> Result<()> {
    let Some(binding) = &journal.repair else {
        return Ok(());
    };
    if journal.no_effects || binding.attempts.is_empty() || binding.attempts.len() > MAX_ATTEMPTS {
        return Err(invalid(
            "repair history requires 1–64 attempts without a no-effect certificate",
        ));
    }
    let mut ids = Vec::with_capacity(binding.attempts.len());
    let mut unique = HashSet::new();
    let mut time = journal.started_at_ms;
    for (index, attempt) in binding.attempts.iter().enumerate() {
        validate_request(&attempt.request)?;
        if attempt.request.operation_id != journal.request.operation_id
            || !unique.insert(attempt.request.id)
            || attempt.started_at_ms < time
            || index + 1 < binding.attempts.len()
                && attempt.outcome != Some(ApplyOutcome::Uncertain)
        {
            return Err(invalid(
                "repair history has incompatible source, identity, time or past outcome",
            ));
        }
        time = attempt.started_at_ms;
        ids.push(attempt.request.id);
    }
    phases(journal, &binding.observations)
        .map_err(|_| invalid("repair capture contains an unknown or incomplete phase"))?;
    let latest = binding.attempts.last().expect("nonempty repair history");
    ids.pop();
    if latest.request.expected != stamp(journal, &binding.observations, &ids)? {
        return Err(invalid(
            "repair capture differs from its exact review stamp",
        ));
    }
    let terminal = journal
        .receipt
        .as_ref()
        .filter(|receipt| receipt.outcome != ApplyOutcome::Uncertain);
    match (terminal, latest.outcome) {
        (Some(receipt), Some(outcome)) if receipt.outcome == outcome => {}
        (None, None | Some(ApplyOutcome::Uncertain)) => {}
        _ => {
            return Err(invalid(
                "latest repair outcome differs from the whole-operation receipt",
            ));
        }
    }
    Ok(())
}

fn covers(newer: Option<&RepairBinding>, older: Option<&RepairBinding>) -> bool {
    let Some(older) = older else {
        return true;
    };
    let Some(newer) = newer else {
        return false;
    };
    if older.attempts.len() > newer.attempts.len()
        || older.attempts.len() == newer.attempts.len() && older.observations != newer.observations
    {
        return false;
    }
    older
        .attempts
        .iter()
        .zip(&newer.attempts)
        .all(|(old, new)| {
            old.request == new.request
                && old.started_at_ms == new.started_at_ms
                && match old.outcome {
                    None => true,
                    Some(ApplyOutcome::Uncertain) => new.outcome.is_some(),
                    Some(outcome) => new.outcome == Some(outcome),
                }
        })
}

pub(super) fn merge(
    existing: &ApplyJournal,
    incoming: &ApplyJournal,
) -> Result<Option<RepairBinding>> {
    let chosen = if proposal_apply::settled(existing) {
        covers(existing.repair.as_ref(), incoming.repair.as_ref()).then_some(&existing.repair)
    } else if proposal_apply::settled(incoming) {
        covers(incoming.repair.as_ref(), existing.repair.as_ref()).then_some(&incoming.repair)
    } else if covers(existing.repair.as_ref(), incoming.repair.as_ref()) {
        Some(&existing.repair)
    } else if covers(incoming.repair.as_ref(), existing.repair.as_ref()) {
        Some(&incoming.repair)
    } else {
        None
    };
    chosen
        .cloned()
        .ok_or_else(|| Error::OperationConflict("recovery has incompatible repair history".into()))
}

impl ApplyJournal {
    /// Pure exact current-phase capture; no eligibility is inferred from text.
    pub fn repair_preview(&self, observations: &[ApplyMemberProof]) -> Result<RepairPreview> {
        self.validate()?;
        if proposal_apply::settled(self) {
            return Err(Error::StateChanged(
                "repair requires an unresolved operation".into(),
            ));
        }
        let phases = phases(self, observations)?;
        let ids: Vec<_> = self
            .repair
            .as_ref()
            .into_iter()
            .flat_map(|binding| &binding.attempts)
            .map(|attempt| attempt.request.id)
            .collect();
        Ok(RepairPreview {
            operation_id: self.request.operation_id,
            expected: stamp(self, observations, &ids)?,
            approved: self.approved.draft.clone(),
            phases,
        })
    }

    /// Exact immutable bindings plus a compatible forward repair prefix.
    pub fn repair_history_covers(&self, older: &ApplyJournal) -> bool {
        if self.validate().is_err()
            || older.validate().is_err()
            || !proposal_apply::same_approval(self, older)
            || older
                .prepared
                .as_ref()
                .is_some_and(|proofs| self.prepared.as_ref() != Some(proofs))
        {
            return false;
        }
        if let Some(old) = &older.receipt {
            let Some(new) = &self.receipt else {
                return false;
            };
            if old.outcome != ApplyOutcome::Uncertain {
                if new != old
                    || self.observations != older.observations
                    || self.no_effects != older.no_effects
                {
                    return false;
                }
            } else if new.outcome == ApplyOutcome::Uncertain {
                if new != old || self.observations != older.observations {
                    return false;
                }
            } else if new.stamp.version <= old.stamp.version {
                return false;
            }
        }
        covers(self.repair.as_ref(), older.repair.as_ref())
    }
}

fn journal_ids(conn: &Connection) -> Result<Vec<Uuid>> {
    let mut statement = conn.prepare("SELECT operation_id FROM proposal_applies ORDER BY rowid")?;
    statement
        .query_map([], |row| row.get::<_, String>(0))?
        .map(|id| crate::parse_id(id?))
        .collect()
}

fn find(conn: &Connection, id: Uuid) -> Result<Option<(ApplyJournal, usize)>> {
    proposals::nonnil(id)?;
    let mut found = None;
    // Keep scalar identities only while checking each bounded journal.
    for operation in journal_ids(conn)? {
        let journal = proposal_apply::read_journal(conn, operation)?
            .ok_or_else(|| invalid("listed repair journal disappeared"))?;
        if let Some(index) = journal.repair.as_ref().and_then(|binding| {
            binding
                .attempts
                .iter()
                .position(|attempt| attempt.request.id == id)
        }) && found.replace((operation, index)).is_some()
        {
            return Err(invalid("repair UUID is duplicated across operations"));
        }
    }
    found
        .map(|(operation, index)| {
            proposal_apply::read_journal(conn, operation)?
                .map(|journal| (journal, index))
                .ok_or_else(|| invalid("repair journal disappeared"))
        })
        .transpose()
}

fn receipt(attempt: &RepairAttempt) -> RepairReceipt {
    RepairReceipt {
        id: attempt.request.id,
        operation_id: attempt.request.operation_id,
        direction: attempt.request.direction,
        outcome: attempt.outcome,
    }
}

pub(super) fn check_collisions(conn: &Connection, journal: &ApplyJournal) -> Result<()> {
    for attempt in journal
        .repair
        .as_ref()
        .into_iter()
        .flat_map(|binding| &binding.attempts)
    {
        if let Some((other, _)) = find(conn, attempt.request.id)?
            && other.request.operation_id != journal.request.operation_id
        {
            return Err(Error::OperationConflict(
                "repair UUID belongs to another operation".into(),
            ));
        }
    }
    Ok(())
}

impl WorkStore {
    pub fn proposal_repair(&self, id: Uuid) -> Result<Option<RepairReceipt>> {
        Ok(find(&self.conn, id)?.map(|(journal, index)| {
            receipt(&journal.repair.expect("located repair").attempts[index])
        }))
    }

    pub fn begin_proposal_repair(
        &mut self,
        request: &RepairRequest,
        observations: &[ApplyMemberProof],
    ) -> Result<ApplyJournal> {
        validate_request(request)?;
        let tx = self.conn.transaction()?;
        if let Some((journal, index)) = find(&tx, request.id)? {
            if journal.repair.as_ref().expect("located repair").attempts[index].request != *request
            {
                return Err(Error::OperationConflict(
                    "repair UUID has another request".into(),
                ));
            }
            return Ok(journal);
        }
        let mut journal = proposal_apply::read_journal(&tx, request.operation_id)?
            .ok_or_else(|| Error::NotFound("repair source is absent".into()))?;
        let preview = journal.repair_preview(observations)?;
        if preview.expected != request.expected {
            return Err(Error::StateChanged(
                "repair capture changed before admission".into(),
            ));
        }
        proposal_apply::current_unresolved(&tx, &journal)?;
        let binding = journal.repair.get_or_insert_with(|| RepairBinding {
            attempts: vec![],
            observations: vec![],
        });
        if binding.attempts.len() >= MAX_ATTEMPTS {
            return Err(invalid("repair history already contains 64 attempts"));
        }
        let mut time = now_ms().max(journal.started_at_ms);
        if let Some(last) = binding.attempts.last_mut() {
            time = time.max(last.started_at_ms);
            if last.outcome.is_none() {
                last.outcome = Some(ApplyOutcome::Uncertain);
            }
        }
        binding.attempts.push(RepairAttempt {
            request: request.clone(),
            started_at_ms: time,
            outcome: None,
        });
        binding.observations = observations.to_vec();
        proposal_apply::write_journal(&tx, &journal)?;
        tx.commit()?;
        Ok(journal)
    }

    pub fn interrupt_proposal_repair(&mut self, id: Uuid) -> Result<RepairReceipt> {
        let tx = self.conn.transaction()?;
        let (mut journal, index) =
            find(&tx, id)?.ok_or_else(|| Error::NotFound("repair attempt is absent".into()))?;
        let binding = journal.repair.as_ref().expect("located repair");
        let attempt = &binding.attempts[index];
        if index + 1 != binding.attempts.len() || attempt.outcome.is_some() {
            return Ok(receipt(attempt));
        }
        proposal_apply::current_unresolved(&tx, &journal)?;
        let attempt = &mut journal.repair.as_mut().expect("located repair").attempts[index];
        attempt.outcome = Some(ApplyOutcome::Uncertain);
        let receipt = receipt(attempt);
        proposal_apply::write_journal(&tx, &journal)?;
        tx.commit()?;
        Ok(receipt)
    }
}
