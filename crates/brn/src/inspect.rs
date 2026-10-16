//! Bounded inspection pages: compact summaries over the core's SELECT-only
//! pages and bounded note reads. The core owns the SQL and the bounds; this
//! module owns the summaries and the cursor syntax.
use crate::contract::{self, Annotations, CliError, Code};
use crate::{encode, Invocation, Result};
use brn_threads_app::*;
use serde_json::{json, Value};

/// Writes summarised per receipt or candidate.
const MAX_WRITES: usize = 32;
/// Characters kept of a reason, authority or run progress.
const MAX_TEXT: usize = 512;
/// Characters kept of a record title.
const MAX_TITLE: usize = 160;

fn cut(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// `history`, `candidates` and `records` page only when a page option is given;
/// without one they keep today's unbounded output.
pub(crate) fn page_mode(inv: &Invocation) -> bool {
    ["limit", "cursor", "record", "kind"]
        .iter()
        .any(|option| inv.args.contains_key(*option))
}

/// The validated `--limit`, else the command's default.
fn limit(inv: &Invocation) -> usize {
    let default = crate::spec::spec(&inv.command)
        .and_then(|spec| spec.page)
        .and_then(|page| page.default)
        .unwrap_or(1);
    inv.args
        .get("limit")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default) as usize
}

/// A validated numeric `--cursor`, else `default`.
fn number_cursor(inv: &Invocation, default: u64) -> u64 {
    inv.args
        .get("cursor")
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn set_page(
    notes: &mut Annotations,
    limit: usize,
    shown: usize,
    cursor: &Option<String>,
    total: Option<u64>,
) {
    notes.page = Some(json!({"limit": limit, "shown": shown, "cursor": cursor, "total": total}));
}

/// `{id, kind, version, archived, created}` per write, at most 32.
fn write_summaries(writes: &[Written]) -> Result<(Vec<contract::Changed>, usize)> {
    let mut changed = contract::changed(&encode(writes)?);
    let total = changed.len();
    changed.truncate(MAX_WRITES);
    Ok((changed, total))
}

fn receipt_summary(receipt: &Receipt) -> Result<Value> {
    let (writes, writes_total) = write_summaries(&receipt.writes)?;
    Ok(json!({
        "operation": receipt.operation.as_str(),
        "reason": cut(&receipt.reason, MAX_TEXT),
        "authority": cut(&receipt.authority, MAX_TEXT),
        "committed_at": receipt.committed_at,
        "undo_of": receipt.undo_of.as_ref().map(OperationId::as_str),
        "writes": writes,
        "writes_total": writes_total,
    }))
}

/// A prepared request's writes as `{id, kind, version, archived, created}`,
/// where `version` is the expected version (null for a new record).
fn request_writes(request: &ChangeRequest) -> Result<(Vec<Value>, usize)> {
    let mut writes = vec![];
    for put in request.writes.iter().take(MAX_WRITES) {
        let data = encode(&put.data)?;
        writes.push(json!({
            "id": put.id,
            "kind": data.as_object().and_then(|d| d.keys().next().cloned()),
            "version": put.expected_version,
            "archived": put.archived,
            "created": put.expected_version.is_none(),
        }));
    }
    Ok((writes, request.writes.len()))
}

/// `history --limit N [--record ID] [--cursor C]`.
pub(crate) fn history(
    workspace: &Workspace,
    inv: &Invocation,
    notes: &mut Annotations,
) -> Result<Value> {
    let limit = limit(inv);
    let record = inv.args.get("record").map(String::as_str);
    let after = number_cursor(inv, 0) as i64;
    let (rows, more) = workspace.store.receipt_page(after, limit, record)?;
    let total = workspace.store.receipt_count(record)?;
    let cursor = more
        .then(|| rows.last().map(|(rowid, _)| rowid.to_string()))
        .flatten();
    let receipts = rows
        .iter()
        .map(|(_, receipt)| receipt_summary(receipt))
        .collect::<Result<Vec<_>>>()?;
    set_page(notes, limit, receipts.len(), &cursor, Some(total));
    Ok(json!({"receipts": receipts, "shown": receipts.len(), "cursor": cursor, "total": total}))
}

/// `candidates --limit N [--cursor C]`.
pub(crate) fn candidates(
    workspace: &Workspace,
    inv: &Invocation,
    notes: &mut Annotations,
) -> Result<Value> {
    let limit = limit(inv);
    let after = number_cursor(inv, 0) as i64;
    let (rows, more) = workspace.store.candidate_page(after, limit)?;
    let cursor = more
        .then(|| rows.last().map(|row| row.rowid.to_string()))
        .flatten();
    let mut candidates = vec![];
    for row in &rows {
        let request = &row.prepared.request;
        let (writes, writes_total) = request_writes(request)?;
        candidates.push(json!({
            "operation": row.prepared.operation.as_str(),
            "request_hash": row.prepared.request_hash,
            "reason": cut(&request.reason, MAX_TEXT),
            "writes": writes,
            "writes_total": writes_total,
            "inputs_total": request.inputs.len(),
            "undo_of": row.undo_of.as_ref().map(OperationId::as_str),
            "replacement_of": row.replacement_of.as_ref().map(OperationId::as_str),
        }));
    }
    set_page(notes, limit, candidates.len(), &cursor, None);
    Ok(
        json!({"candidates": candidates, "shown": candidates.len(), "cursor": cursor, "total": null}),
    )
}

/// The variant name and title (when the record has one) of a record.
fn kind_and_title(record: &Record) -> Result<(Option<String>, Option<String>)> {
    let data = encode(&record.data)?;
    let kind = data.as_object().and_then(|d| d.keys().next().cloned());
    let title = kind
        .as_ref()
        .and_then(|kind| data[kind]["title"].as_str())
        .map(|title| cut(title, MAX_TITLE));
    Ok((kind, title))
}

/// `records --limit N [--kind KIND] [--cursor ID]`.
pub(crate) fn records(
    workspace: &Workspace,
    inv: &Invocation,
    notes: &mut Annotations,
) -> Result<Value> {
    let limit = limit(inv);
    let after = inv.args.get("cursor").map(String::as_str).unwrap_or("");
    let kind = inv.args.get("kind").map(String::as_str);
    let (rows, more) = workspace.store.record_page(kind, after, limit)?;
    let cursor = more.then(|| rows.last().map(|r| r.id.clone())).flatten();
    let mut records = vec![];
    for record in &rows {
        let (kind, title) = kind_and_title(record)?;
        records.push(json!({
            "id": record.id, "kind": kind, "version": record.version,
            "archived": record.archived, "title": title,
        }));
    }
    set_page(notes, limit, records.len(), &cursor, None);
    Ok(json!({"records": records, "shown": records.len(), "cursor": cursor, "total": null}))
}

/// `revisions --note ID [--limit 1..32] [--cursor VERSION]`.
pub(crate) fn revisions(
    workspace: &Workspace,
    inv: &Invocation,
    notes: &mut Annotations,
) -> Result<Value> {
    let limit = limit(inv);
    let page = workspace
        .store
        .note_revisions(inv.arg("note")?, number_cursor(inv, 0))?;
    let truncated = page.revisions.len() > limit;
    let mut revisions = page.revisions;
    revisions.truncate(limit);
    let cursor = if truncated {
        revisions.last().map(|r| r.version)
    } else {
        page.next_after
    }
    .map(|version| version.to_string());
    set_page(notes, limit, revisions.len(), &cursor, None);
    Ok(json!({"id": page.id, "revisions": revisions, "shown": revisions.len(), "cursor": cursor}))
}

/// `revision --note ID --version V [--limit BYTES] [--cursor BYTE]`.
pub(crate) fn revision(
    workspace: &Workspace,
    inv: &Invocation,
    notes: &mut Annotations,
) -> Result<Value> {
    let limit = limit(inv);
    let start = number_cursor(inv, 0) as usize;
    let slice = workspace.store.read_note_revision(
        inv.arg("note")?,
        inv.number("version")?,
        start,
        limit,
    )?;
    let cursor = slice.next_start.map(|next| next.to_string());
    set_page(
        notes,
        limit,
        slice.end - slice.start,
        &cursor,
        Some(slice.total_bytes as u64),
    );
    let mut value = encode(&slice)?;
    value["cursor"] = json!(cursor);
    Ok(value)
}

/// `read --id ID --limit BYTES [--cursor VERSION.START]`: a bounded slice of
/// one note revision; a later page of a note that changed is Stale.
pub(crate) fn read(
    workspace: &Workspace,
    inv: &Invocation,
    notes: &mut Annotations,
) -> Result<Value> {
    let id = inv.arg("id")?;
    let limit = limit(inv);
    let (version, start) = match inv.args.get("cursor") {
        Some(cursor) => {
            let (version, start) = cursor.split_once('.').unwrap_or_default();
            let parse = |text: &str| {
                text.parse::<u64>()
                    .map_err(|_| CliError::invalid_value("cursor", "--cursor takes VERSION.START"))
            };
            (parse(version)?, parse(start)? as usize)
        }
        None => {
            let record = workspace
                .store
                .record(id)?
                .ok_or_else(|| CliError::new(Code::NotFound, "Record not found"))?;
            if !matches!(record.data, RecordData::Note(_)) {
                return Err(CliError::invalid_request("record is not a note"));
            }
            (record.version, 0)
        }
    };
    let outcome = workspace
        .store
        .read_note_range(id, version, start, limit, false)?;
    let (cursor, shown, total) = match &outcome {
        NoteReadOutcome::Read { note } => (
            note.next_start.map(|next| format!("{version}.{next}")),
            note.end - note.start,
            Some(note.total_bytes as u64),
        ),
        NoteReadOutcome::Stale { .. } => (None, 0, None),
    };
    set_page(notes, limit, shown, &cursor, total);
    let mut value = encode(&outcome)?;
    value["cursor"] = json!(cursor);
    Ok(value)
}

fn run_state(name: &str) -> Option<RunState> {
    Some(match name {
        "working" => RunState::Working,
        "interrupted" => RunState::Interrupted,
        "completed" => RunState::Completed,
        "cancelled" => RunState::Cancelled,
        "failed" => RunState::Failed,
        _ => return None,
    })
}

/// `runs [--thread ID] [--state STATE] [--limit N] [--cursor RUN]`.
pub(crate) fn runs(
    workspace: &Workspace,
    inv: &Invocation,
    notes: &mut Annotations,
) -> Result<Value> {
    let limit = limit(inv);
    let after = inv.args.get("cursor").map(String::as_str).unwrap_or("");
    let state = inv.args.get("state").and_then(|s| run_state(s));
    let thread = inv.args.get("thread").map(String::as_str);
    let (rows, more) = workspace.store.run_page(thread, state, after, limit)?;
    let cursor = more.then(|| rows.last().map(|r| r.id.clone())).flatten();
    let mut runs = vec![];
    for record in &rows {
        let RecordData::Run(run) = &record.data else {
            continue;
        };
        runs.push(json!({
            "id": record.id,
            "version": record.version,
            "archived": record.archived,
            "thread": run.thread,
            "state": format!("{:?}", run.state).to_lowercase(),
            "provider": run.provider,
            "model": run.model,
            "effort": run.effort,
            "budget": run.budget,
            "fence": run.fence,
            "progress": cut(&run.progress, MAX_TEXT),
            "operations_total": run.operations.len(),
            // A Working run guards its chat: chat writes come back Deferred.
            "guards_writes": run.state == RunState::Working && !record.archived,
        }));
    }
    set_page(notes, limit, runs.len(), &cursor, None);
    Ok(json!({"runs": runs, "shown": runs.len(), "cursor": cursor, "total": null}))
}

/// Inputs listed per operation.
const MAX_INPUTS: usize = 64;

/// `operation --id OP | --key K --command NAME`, checked before opening.
pub(crate) fn operation_form(options: &std::collections::BTreeMap<String, String>) -> Result<()> {
    let has = |name: &str| options.contains_key(name);
    if has("id") && has("key") {
        return Err(CliError::conflicting(
            &["id", "key"],
            "Choose either --id or --key with --command",
        ));
    }
    if has("key") && !has("command") {
        return Err(CliError::missing_option("command"));
    }
    if has("command") && !has("key") {
        return Err(CliError::missing_option("key"));
    }
    if !has("id") && !has("key") {
        return Err(CliError::missing_option("id"));
    }
    if let Some(command) = options.get("command") {
        if crate::spec::stored_host_key(command, "k").is_none() {
            return Err(CliError::invalid_value(
                "command",
                "--command must name a command that takes --key",
            ));
        }
    }
    Ok(())
}

/// Where one operation stands. A key lookup is a pure SELECT: it never allocates.
pub(crate) fn operation(workspace: &Workspace, inv: &Invocation) -> Result<Value> {
    let not_found = || CliError::new(Code::NotFound, "No such operation");
    let (id, related) = match (inv.args.get("id"), inv.args.get("key")) {
        (Some(id), _) => (id.clone(), vec![]),
        (None, Some(key)) => {
            let command = inv.arg("command")?;
            let stored = crate::spec::stored_host_key(command, key).ok_or_else(not_found)?;
            let id = workspace
                .store
                .operation_by_host_key(&stored)?
                .ok_or_else(not_found)?;
            // knowledge-check --seed-chat yes records the chat under its own key.
            let related = match crate::spec::spec(command).map(|s| s.name) {
                Some("knowledge-check") => workspace
                    .store
                    .operation_by_host_key(&format!(
                        "new-thread:knowledge-review-chat:{}",
                        crate::spec::cli_key(command, key)
                    ))?
                    .map(|op| vec![op.as_str().to_owned()])
                    .unwrap_or_default(),
                _ => vec![],
            };
            (id.as_str().to_owned(), related)
        }
        (None, None) => return Err(CliError::missing_option("id")),
    };
    let info = workspace
        .store
        .operation_info(&id)
        .map_err(|error| match error {
            brn_threads_app::Error::Missing(_) => not_found(),
            other => other.into(),
        })?;
    let state = match info.state {
        OperationState::Applied => "applied",
        OperationState::Prepared => "prepared",
        OperationState::Retired => "retired",
        OperationState::Allocated => "allocated",
    };
    let request = info.prepared.as_ref().map(|p| &p.request);
    let mut inputs = vec![];
    for input in request.map(|r| r.inputs.as_slice()).unwrap_or_default() {
        if inputs.len() == MAX_INPUTS {
            break;
        }
        inputs.push(json!({
            "record": input.record,
            "version": input.version,
            "expectation": format!("{:?}", input.expectation),
        }));
    }
    let (writes, writes_total) = match (&info.receipt, request) {
        (Some(receipt), _) => {
            let (writes, total) = write_summaries(&receipt.writes)?;
            (encode(writes)?, total)
        }
        (None, Some(request)) => {
            let (writes, total) = request_writes(request)?;
            (json!(writes), total)
        }
        (None, None) => (json!([]), 0),
    };
    let reason = info
        .receipt
        .as_ref()
        .map(|r| r.reason.as_str())
        .or(request.map(|r| r.reason.as_str()))
        .map(|reason| cut(reason, MAX_TEXT));
    let request_hash = info
        .receipt
        .as_ref()
        .map(|r| r.request_hash.clone())
        .or(info.prepared.as_ref().map(|p| p.request_hash.clone()));
    Ok(json!({
        "operation": info.id.as_str(),
        "state": state,
        "undo_of": info.undo_of.as_ref().map(OperationId::as_str),
        "replacement_of": info.replacement_of.as_ref().map(OperationId::as_str),
        "request_hash": request_hash,
        "reason": reason,
        "inputs": inputs,
        "inputs_total": request.map_or(0, |r| r.inputs.len()),
        "writes": writes,
        "writes_total": writes_total,
        "receipt": info.receipt.as_ref().map(|r| json!({
            "committed_at": r.committed_at,
            "authority": cut(&r.authority, MAX_TEXT),
        })),
        "related": related,
    }))
}
