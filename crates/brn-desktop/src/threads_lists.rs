//! Navigation lists and small lifecycle rules derived from current records.
//! Canonical state and every write remain in the shared service.
use super::layout::TileKind;
use super::state::linked;
use super::theme::{Appearance, ThemeId};
use brn_threads_app::{ActionState, Receipt, Record, RecordData, ThreadState, record_in_chat};
use std::collections::{BTreeMap, BTreeSet};

/// Presentation only: a follow-up date is not a deadline or a state transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FollowUpGroup {
    Suggested,
    Overdue,
    Today,
    Upcoming,
    CheckDate,
    Waiting,
    Open,
    Done,
    Cancelled,
}
impl FollowUpGroup {
    pub(super) const ALL: [Self; 9] = [
        Self::Suggested,
        Self::Overdue,
        Self::Today,
        Self::Upcoming,
        Self::CheckDate,
        Self::Waiting,
        Self::Open,
        Self::Done,
        Self::Cancelled,
    ];

    pub(super) fn presentation(self) -> (&'static str, &'static str, Option<&'static str>) {
        match self {
            Self::Suggested => ("suggested", "Suggested", None),
            Self::Overdue => ("overdue", "Overdue follow-ups", None),
            Self::Today => ("today", "Follow up today", None),
            Self::Upcoming => ("upcoming", "Upcoming follow-ups", None),
            Self::CheckDate => ("check-date", "Check follow-up date", None),
            Self::Waiting => ("waiting", "Waiting", None),
            Self::Open => ("open", "Open", None),
            Self::Done => ("done", "Done", Some("tasks-done")),
            Self::Cancelled => ("cancelled", "Cancelled", Some("tasks-cancelled")),
        }
    }
}

/// Strict date-only parsing matches canonical Action validation. Do not use
/// day_number: it intentionally accepts timestamp prefixes and loose dates.
fn follow_up_date(value: &str) -> Option<chrono::NaiveDate> {
    let date = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
    (value.len() == 10 && date.to_string() == value).then_some(date)
}

/// Re-evaluated when the view renders; no midnight timer or background work.
/// Uses the existing platform offset adapter (UTC if the system cannot say).
pub(super) fn follow_up_today() -> chrono::NaiveDate {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64);
    follow_up_day_at(now, local_offset(now))
}
fn follow_up_day_at(now: i64, offset: i64) -> chrono::NaiveDate {
    chrono::DateTime::from_timestamp(now.saturating_add(offset), 0)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH)
        .date_naive()
}

pub(super) fn follow_up_group(
    action: &brn_threads_app::Action,
    today: chrono::NaiveDate,
) -> FollowUpGroup {
    use FollowUpGroup::*;
    match action.state {
        ActionState::Suggested => return Suggested,
        ActionState::Done => return Done,
        ActionState::Cancelled => return Cancelled,
        ActionState::Open | ActionState::Waiting => {}
    }
    if let Some(value) = &action.follow_up {
        return match follow_up_date(value) {
            None => CheckDate,
            Some(date) if date < today => Overdue,
            Some(date) if date == today => Today,
            Some(_) => Upcoming,
        };
    }
    if action.state == ActionState::Waiting {
        Waiting
    } else {
        Open
    }
}

/// Only partition the already-visible records; never expand a project scope.
/// Earliest follow-up first, stable identity tie-break. `due` is never consulted.
pub(super) fn follow_up_records(
    records: &[Record],
    group: FollowUpGroup,
    today: chrono::NaiveDate,
) -> Vec<&Record> {
    let mut rows: Vec<_> = records.iter().filter(|record| !record.archived
        && matches!(&record.data, RecordData::Action(action) if follow_up_group(action, today) == group)).collect();
    rows.sort_by_key(|record| {
        let RecordData::Action(action) = &record.data else {
            unreachable!()
        };
        (
            action.follow_up.as_deref().and_then(follow_up_date),
            record.id.as_str(),
        )
    });
    rows
}

/// Latest receipt position and commit time for records and the threads their
/// messages and runs belong to, and whether that latest change was committed
/// under the standing delegation (`maintenance:` authority).
#[derive(Default)]
pub(super) struct Activity(BTreeMap<String, (usize, String, bool)>);
impl Activity {
    pub(super) fn new(history: &[Receipt]) -> Self {
        let mut out = BTreeMap::new();
        for (index, receipt) in history.iter().enumerate() {
            let delegated = receipt.authority.starts_with("maintenance:");
            for write in &receipt.writes {
                let entry = (index, receipt.committed_at.clone(), delegated);
                match &write.after.data {
                    RecordData::Message(message) => {
                        out.insert(message.thread.clone(), entry.clone());
                    }
                    RecordData::Run(run) => {
                        out.insert(run.thread.clone(), entry.clone());
                    }
                    _ => {}
                }
                out.insert(write.after.id.clone(), entry);
            }
        }
        Self(out)
    }
    pub(super) fn order(&self, id: &str) -> usize {
        self.0.get(id).map_or(0, |(order, ..)| *order)
    }
    pub(super) fn stamp(&self, id: &str) -> &str {
        self.0.get(id).map_or("", |(_, stamp, _)| stamp.as_str())
    }
    /// The record's latest recorded change came from delegated maintenance.
    /// This names the authority only; it never implies personal review.
    pub(super) fn delegated(&self, id: &str) -> bool {
        self.0.get(id).is_some_and(|(.., delegated)| *delegated)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ThreadRow {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) resolved: bool,
    pub(super) reasons: Vec<String>,
    pub(super) note: Option<String>,
    pub(super) has_action: bool,
    pub(super) order: usize,
    pub(super) day: Option<i64>,
}
impl ThreadRow {
    pub(super) fn needs_you(&self) -> bool {
        !self.reasons.is_empty()
    }
}

/// Days since 1970-01-01 for an ISO date prefix (`YYYY-MM-DD…`).
pub(super) fn day_number(stamp: &str) -> Option<i64> {
    let year: i64 = stamp.get(0..4)?.parse().ok()?;
    let month: i64 = stamp.get(5..7)?.parse().ok()?;
    let day: i64 = stamp.get(8..10)?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// A commit time as people read it, in this Mac's time zone: "Today 22:47",
/// "14 Oct, 22:47", or "14 Oct 2025, 22:47" in another year. Callers keep
/// the raw ISO value (UTC) in the tooltip; an unreadable stamp is returned
/// unchanged.
pub(super) fn human_time(stamp: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64);
    human_time_at(stamp, now, local_offset)
}

/// `human_time` at `now` (UTC seconds) with `offset` giving the zone's
/// seconds east of UTC at an instant. A date without a time of day is shown
/// as written.
pub(super) fn human_time_at(stamp: &str, now: i64, offset: impl Fn(i64) -> i64) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let Some(utc_day) = day_number(stamp) else {
        return stamp.to_owned();
    };
    let clock = stamp.get(10..16).filter(|rest| {
        let bytes = rest.as_bytes();
        matches!(bytes[0], b'T' | b' ')
            && bytes[3] == b':'
            && [1, 2, 4, 5].iter().all(|&i| bytes[i].is_ascii_digit())
    });
    let today = (now + offset(now)).div_euclid(86_400);
    // `clock` has checked its digits.
    let (day, minutes) = match clock {
        Some(clock) => {
            let hours: i64 = clock[1..3].parse().unwrap_or_default();
            let mins: i64 = clock[4..6].parse().unwrap_or_default();
            let seconds = stamp
                .get(16..19)
                .filter(|rest| rest.starts_with(':'))
                .and_then(|rest| rest[1..].parse::<i64>().ok())
                .unwrap_or_default();
            let epoch = utc_day * 86_400 + hours * 3_600 + mins * 60 + seconds;
            let local = epoch + offset(epoch);
            (
                local.div_euclid(86_400),
                Some(local.rem_euclid(86_400) / 60),
            )
        }
        None => (utc_day, None),
    };
    let (year, month, date) = civil_from_days(day);
    let when = if day == today {
        "Today".to_owned()
    } else if year == civil_from_days(today).0 {
        format!("{date} {}", MONTHS[(month - 1) as usize])
    } else {
        format!("{date} {} {year}", MONTHS[(month - 1) as usize])
    };
    match minutes {
        Some(minutes) if day == today => {
            format!("{when} {:02}:{:02}", minutes / 60, minutes % 60)
        }
        Some(minutes) => format!("{when}, {:02}:{:02}", minutes / 60, minutes % 60),
        None => when,
    }
}

/// Seconds east of UTC in this Mac's time zone at `epoch` (UTC seconds),
/// from the C library's `localtime_r`, so each date gets its own daylight
/// saving. UTC (0) when the system can't say.
#[cfg(unix)]
#[allow(
    clippy::useless_conversion,
    clippy::unnecessary_fallible_conversions,
    reason = "`c_long` is 64 bits on 64-bit macOS and Linux, 32 bits on 32-bit targets"
)]
fn local_offset(epoch: i64) -> i64 {
    use std::ffi::{c_char, c_int, c_long};
    // `struct tm` as macOS and glibc lay it out; only `gmtoff` is read.
    #[repr(C)]
    #[allow(dead_code, reason = "fields mirror struct tm; only gmtoff is read")]
    struct Tm {
        sec: c_int,
        min: c_int,
        hour: c_int,
        mday: c_int,
        mon: c_int,
        year: c_int,
        wday: c_int,
        yday: c_int,
        isdst: c_int,
        gmtoff: c_long,
        zone: *const c_char,
    }
    unsafe extern "C" {
        fn localtime_r(time: *const c_long, out: *mut Tm) -> *mut Tm;
    }
    // `time_t` is `long` on macOS and Linux; where `long` is 32 bits, an
    // instant it can't hold reads as UTC.
    let Some(time) = c_long::try_from(epoch).ok() else {
        return 0;
    };
    let mut out = std::mem::MaybeUninit::<Tm>::zeroed();
    // SAFETY: both pointers are valid for the call; `localtime_r` only reads
    // `time` and writes the zeroed `out`, which it fully initializes when it
    // returns non-null.
    let filled = unsafe { localtime_r(&time, out.as_mut_ptr()) };
    if filled.is_null() {
        return 0;
    }
    // SAFETY: `localtime_r` succeeded, so `out` is initialized.
    i64::from(unsafe { out.assume_init() }.gmtoff)
}

/// Without the C library's `localtime_r` (Windows), times read as UTC, the
/// documented fallback.
#[cfg(not(unix))]
fn local_offset(_epoch: i64) -> i64 {
    0
}

/// The civil (year, month, day) of a day number (days since 1970-01-01).
fn civil_from_days(day: i64) -> (i64, i64, i64) {
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let date = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2), month, date)
}

/// The UTC calendar year of a day number.
#[cfg(test)]
fn civil_year(day: i64) -> i64 {
    civil_from_days(day).0
}

/// Bookkeeping that writes only chat controls (queue set-up and recovery on
/// every launch). Changes groups it under Background and Home leaves it out;
/// a change that needs refresh is never background.
pub(super) fn is_background(receipt: &Receipt) -> bool {
    receipt.needs_refresh.is_empty()
        && !receipt.writes.is_empty()
        && (receipt.writes.iter().all(|write| matches!(write.after.data, RecordData::ChatControl(_)))
            || receipt.writes.iter().all(|write| matches!(&write.after.data, RecordData::Message(m) if m.role == brn_threads_app::WRITING_CONTEXT_ROLE)))
}

/// A recorded reason as the owner reads it: chat-control bookkeeping drops
/// its "Chat control: " prefix. The recorded reason itself is unchanged.
pub(super) fn receipt_reason(reason: &str) -> String {
    let Some(rest) = reason.strip_prefix("Chat control: ") else {
        return reason.to_owned();
    };
    let mut chars = rest.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => reason.to_owned(),
    }
}

/// The notes, chats, tasks, projects and sources a change touched, by title:
/// "A", "A, B" or "A, B and N more".
pub(super) fn affected_titles(receipt: &Receipt) -> String {
    let mut titles: Vec<String> = vec![];
    for write in &receipt.writes {
        let title = match &write.after.data {
            RecordData::Note(note) => note.title.as_str(),
            RecordData::Thread(thread) => thread.title.as_str(),
            RecordData::Action(action) => action.description.as_str(),
            RecordData::Project(project) => project.title.as_str(),
            RecordData::Source(source) => source.label.as_str(),
            RecordData::Settings(_) => "Workspace settings",
            _ => continue,
        };
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        if !title.is_empty() && !titles.contains(&title) {
            titles.push(title);
        }
    }
    match titles.as_slice() {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a}, {b}"),
        [a, b, rest @ ..] => format!("{a}, {b} and {} more", rest.len()),
    }
}

/// A task state change offered from its row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TaskStep {
    Accept,
    ConfirmDone,
    Waiting,
    MarkOpen,
    Cancel,
}
impl TaskStep {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Accept => "Accept task",
            Self::ConfirmDone => "Confirm done",
            Self::Waiting => "Waiting",
            Self::MarkOpen => "Mark open",
            Self::Cancel => "Cancel task",
        }
    }
    pub(super) fn state(self) -> ActionState {
        match self {
            Self::Accept | Self::MarkOpen => ActionState::Open,
            Self::ConfirmDone => ActionState::Done,
            Self::Waiting => ActionState::Waiting,
            Self::Cancel => ActionState::Cancelled,
        }
    }
    /// The reason each change records, in the Tasks vocabulary.
    pub(super) fn reason(self) -> &'static str {
        match self {
            Self::Accept | Self::MarkOpen => "Owner opened a task",
            Self::ConfirmDone => "Owner confirmed a task is done",
            Self::Waiting => "Owner marked a task waiting",
            Self::Cancel => "Owner cancelled a task",
        }
    }
}

/// An open task's one visible main action and its row ⋯ items, which hold
/// only changes of state, Cancel task last. Done and Cancelled offer none.
pub(super) fn task_steps(state: &ActionState) -> (Option<TaskStep>, Vec<TaskStep>) {
    use TaskStep::*;
    match state {
        ActionState::Suggested => (Some(Accept), vec![Waiting, ConfirmDone, Cancel]),
        ActionState::Open => (Some(ConfirmDone), vec![Waiting, Cancel]),
        ActionState::Waiting => (Some(ConfirmDone), vec![MarkOpen, Cancel]),
        ActionState::Done | ActionState::Cancelled => (None, vec![]),
    }
}

pub(super) fn today() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| (elapsed.as_secs() / 86_400) as i64)
}

pub(super) fn thread_rows(records: &[Record], activity: &Activity) -> Vec<ThreadRow> {
    let kinds: BTreeMap<&str, &RecordData> = records
        .iter()
        .filter(|record| !record.archived)
        .map(|record| (record.id.as_str(), &record.data))
        .collect();
    let mut rows: Vec<_> = records
        .iter()
        .filter_map(|record| {
            if brn_threads_app::project_for_chat(records, &record.id).is_some_and(|p| p.archived) {
                return None;
            }
            let RecordData::Thread(thread) = &record.data else {
                return None;
            };
            let links = linked(records, &record.id);
            Some(ThreadRow {
                id: record.id.clone(),
                title: thread.title.clone(),
                resolved: record.archived || thread.state == ThreadState::Resolved,
                reasons: thread
                    .attention
                    .iter()
                    .map(|attention| attention.reason.clone())
                    .collect(),
                note: links
                    .iter()
                    .find(|id| matches!(kinds.get(id.as_str()), Some(RecordData::Note(_))))
                    .cloned(),
                has_action: links
                    .iter()
                    .any(|id| matches!(kinds.get(id.as_str()), Some(RecordData::Action(_)))),
                order: activity.order(&record.id),
                day: day_number(activity.stamp(&record.id)),
            })
        })
        .collect();
    rows.sort_by(|a, b| b.order.cmp(&a.order).then_with(|| a.title.cmp(&b.title)));
    rows
}

pub(super) struct Sidebar<'a> {
    pub(super) needs: Vec<&'a ThreadRow>,
    pub(super) open: Vec<&'a ThreadRow>,
    pub(super) total: usize,
    pub(super) resolved: usize,
}
pub(super) const SIDEBAR_OPEN: usize = 10;

/// Needs you shows every attention thread; Open shows the most recent; resolved
/// threads are only counted. The current open thread stays visible.
pub(super) fn sidebar<'a>(rows: &'a [ThreadRow], current: Option<&str>) -> Sidebar<'a> {
    let needs = rows.iter().filter(|row| row.needs_you()).collect();
    let mut open: Vec<_> = rows
        .iter()
        .filter(|row| !row.resolved && !row.needs_you())
        .take(SIDEBAR_OPEN)
        .collect();
    if let Some(row) = rows.iter().find(|row| {
        Some(row.id.as_str()) == current
            && !row.resolved
            && !row.needs_you()
            && !open.iter().any(|shown| shown.id == row.id)
    }) {
        open.push(row);
    }
    Sidebar {
        needs,
        open,
        total: rows.len(),
        resolved: rows.iter().filter(|row| row.resolved).count(),
    }
}

/// Why Recent chats lists no chat, so its empty state stays true.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RecentEmpty {
    /// The workspace has no chats.
    NoChats,
    /// Every remaining chat is listed under Needs you.
    OthersNeedYou,
    /// Every chat is archived.
    AllArchived,
}
impl Sidebar<'_> {
    pub(super) fn recent_empty(&self) -> Option<RecentEmpty> {
        if !self.open.is_empty() {
            None
        } else if self.total == 0 {
            Some(RecentEmpty::NoChats)
        } else if !self.needs.is_empty() {
            Some(RecentEmpty::OthersNeedYou)
        } else {
            Some(RecentEmpty::AllArchived)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ThreadFilter {
    All,
    Open,
    Resolved,
    NeedsYou,
    LinkedNote,
    HasAction,
}
impl ThreadFilter {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Open => "Open",
            Self::Resolved => "Archived",
            Self::NeedsYou => "Needs you",
            Self::LinkedNote => "Linked note",
            Self::HasAction => "Has task",
        }
    }
    fn accepts(self, row: &ThreadRow) -> bool {
        match self {
            Self::All => true,
            Self::Open => !row.resolved,
            Self::Resolved => row.resolved,
            Self::NeedsYou => row.needs_you(),
            Self::LinkedNote => row.note.is_some(),
            Self::HasAction => row.has_action,
        }
    }
}

pub(super) const LIST_LIMIT: usize = 200;

pub(super) struct Grouped<'a> {
    pub(super) groups: Vec<(&'static str, Vec<&'a ThreadRow>)>,
    pub(super) matched: usize,
}

/// Filter-as-you-type over titles and attention reasons, grouped by UTC day.
pub(super) fn all_threads<'a>(
    rows: &'a [ThreadRow],
    filter: ThreadFilter,
    query: &str,
    today: i64,
) -> Grouped<'a> {
    let query = query.trim().to_lowercase();
    let matched: Vec<_> = rows
        .iter()
        .filter(|row| filter.accepts(row))
        .filter(|row| {
            query.is_empty()
                || row.title.to_lowercase().contains(&query)
                || row
                    .reasons
                    .iter()
                    .any(|reason| reason.to_lowercase().contains(&query))
        })
        .collect();
    let mut groups: Vec<(&'static str, Vec<&ThreadRow>)> =
        vec![("Today", vec![]), ("This week", vec![]), ("Older", vec![])];
    for row in matched.iter().take(LIST_LIMIT) {
        let age = row.day.map_or(i64::MAX, |day| today - day);
        let slot = if age <= 0 {
            0
        } else if age < 7 {
            1
        } else {
            2
        };
        groups[slot].1.push(row);
    }
    groups.retain(|(_, rows)| !rows.is_empty());
    Grouped {
        groups,
        matched: matched.len(),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct NoteRow {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) protected: bool,
    pub(super) imported: bool,
    pub(super) order: usize,
    /// Links into this note from other general notes.
    pub(super) inbound: usize,
    /// The first prose line of the saved text, for the Library row.
    pub(super) description: String,
    /// The Source record the note's recorded import coverage names.
    pub(super) source: Option<String>,
    /// A faithful full-note import (a reference), not a written article.
    pub(super) full_import: bool,
}
impl NoteRow {
    /// Full imported references are files; every other current note, also
    /// one written from sources, is an article. Protection does not decide.
    pub(super) fn is_file(&self) -> bool {
        self.full_import
    }
}

/// Longest Library row description, in characters.
pub(super) const DESCRIPTION_LIMIT: usize = 160;

/// A short description from saved Markdown: the first prose line, without
/// headings, fences, tables, images or inline markup, cut at a word. Empty
/// when the note has no prose yet; nothing is invented.
pub(super) fn note_description(markdown: &str) -> String {
    let mut fenced = false;
    let mut out = String::new();
    for line in markdown.lines() {
        let line = line.trim();
        if line.starts_with("```") || line.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced
            || line.is_empty()
            || line.starts_with('#')
            || line.starts_with('|')
            || line.starts_with("![")
            || line.starts_with("---")
            || line.starts_with("<!--")
        {
            if !out.is_empty() {
                break;
            }
            continue;
        }
        let line = line
            .trim_start_matches(['>', ' '])
            .trim_start_matches("- [ ] ")
            .trim_start_matches("- [x] ");
        let item = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
            .or_else(|| line.strip_prefix("+ "));
        let line = item.unwrap_or(line);
        if !out.is_empty() {
            out.push_str(if item.is_some() { "; " } else { " " });
        }
        out.push_str(&plain_inline(line));
        if out.chars().count() >= DESCRIPTION_LIMIT {
            break;
        }
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.chars().count() <= DESCRIPTION_LIMIT {
        return out;
    }
    let cut: String = out.chars().take(DESCRIPTION_LIMIT).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{}…", cut.trim_end_matches([',', ';', ':', '.', ' ']))
}

/// Inline Markdown as plain text: `[label](url)` keeps the label, `[[Note]]`
/// keeps the name, emphasis and code marks drop.
fn plain_inline(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '*' | '`' => {}
            '_' if chars.peek() == Some(&'_') => {
                chars.next();
            }
            '[' => {
                let wiki = chars.peek() == Some(&'[');
                if wiki {
                    chars.next();
                }
                let mut label = String::new();
                for next in chars.by_ref() {
                    if next == ']' {
                        break;
                    }
                    label.push(next);
                }
                if wiki {
                    if chars.peek() == Some(&']') {
                        chars.next();
                    }
                    let name = label.split('|').next_back().unwrap_or(&label);
                    out.push_str(name);
                    continue;
                }
                out.push_str(&label);
                if chars.peek() == Some(&'(') {
                    for next in chars.by_ref() {
                        if next == ')' {
                            break;
                        }
                    }
                }
            }
            ch => out.push(ch),
        }
    }
    out
}
pub(super) fn letter(title: &str) -> char {
    title
        .chars()
        .find(|ch| ch.is_alphanumeric())
        .and_then(|ch| ch.to_uppercase().next())
        .filter(|ch| ch.is_alphabetic())
        .unwrap_or('#')
}

pub(super) fn note_rows(records: &[Record], activity: &Activity) -> Vec<NoteRow> {
    let general: BTreeSet<&str> = records
        .iter()
        .filter(|r| !r.archived)
        .filter(|r| matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none() && n.chat.is_none()))
        .filter(|r| record_in_chat(r, records, None))
        .map(|r| r.id.as_str())
        .collect();
    let mut inbound: BTreeMap<&str, usize> = BTreeMap::new();
    for record in records.iter().filter(|r| !r.archived) {
        if let RecordData::Link(link) = &record.data
            && link.from != link.to
            && general.contains(link.from.as_str())
            && general.contains(link.to.as_str())
        {
            *inbound.entry(link.to.as_str()).or_default() += 1;
        }
    }
    let mut rows: Vec<_> = records
        .iter()
        .filter(|record| !record.archived)
        .filter_map(|record| match &record.data {
            RecordData::Note(note)
                if note.superseded_by.is_none()
                    && note.chat.is_none()
                    && record_in_chat(record, records, None) =>
            {
                Some(NoteRow {
                    id: record.id.clone(),
                    title: note.title.clone(),
                    protected: note.protected,
                    imported: note.import.is_some(),
                    order: activity.order(&record.id),
                    inbound: inbound.get(record.id.as_str()).copied().unwrap_or(0),
                    description: note_description(&note.markdown),
                    source: note.import.as_ref().map(|import| import.source.clone()),
                    full_import: note.import.as_ref().is_some_and(|import| import.full_note),
                })
            }
            _ => None,
        })
        .collect();
    rows.sort_by(|a, b| {
        a.title
            .to_lowercase()
            .cmp(&b.title.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    rows
}

pub(super) fn filter_notes<'a>(
    rows: &'a [NoteRow],
    query: &str,
    limit: usize,
) -> (Vec<&'a NoteRow>, usize) {
    let query = query.trim().to_lowercase();
    let matched: Vec<_> = rows
        .iter()
        .filter(|row| row.title.to_lowercase().contains(&query))
        .collect();
    let total = matched.len();
    (matched.into_iter().take(limit).collect(), total)
}

/// Notes linked to any open thread.
pub(super) fn notes_linked_to_open(records: &[Record], rows: &[ThreadRow]) -> BTreeSet<String> {
    let notes: BTreeSet<&str> = records
        .iter()
        .filter(|record| !record.archived && matches!(record.data, RecordData::Note(_)))
        .map(|record| record.id.as_str())
        .collect();
    rows.iter()
        .filter(|row| !row.resolved)
        .flat_map(|row| linked(records, &row.id))
        .filter(|id| notes.contains(id.as_str()))
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Command {
    NewThread,
    NewNote,
    Rewrite,
    Save,
    /// The open document's Outline (READ-01); no global shortcut.
    Outline,
    /// Appearance: System/Light/Dark primary, plus legacy palettes under clear
    /// labels. The payload is the preference, not the resolved palette.
    Theme(Appearance),
    OpenTile(TileKind),
    Keys,
    /// The explicit web search dialog (only the typed query is sent).
    WebSearch,
}
impl Command {
    /// Destinations are named, never "Open tile: …".
    pub(super) fn label(self) -> String {
        match self {
            Self::NewThread => "New chat".into(),
            Self::NewNote => "New note".into(),
            Self::Rewrite => "Rewrite with AI".into(),
            Self::Save => "Save note".into(),
            Self::Outline => "Show or hide outline".into(),
            Self::Theme(appearance) => match appearance {
                // Primary choices read as plain appearance commands.
                Appearance::System | Appearance::Light | Appearance::Dark => {
                    format!(
                        "Appearance: {} · {}",
                        appearance.label(),
                        appearance.description()
                    )
                }
                // Legacy palettes are clearly labelled as such.
                Appearance::Legacy(theme) => {
                    format!(
                        "Appearance: {} (legacy) · {}",
                        theme.label(),
                        theme.description()
                    )
                }
            },
            Self::OpenTile(TileKind::Note) => "Current note".into(),
            Self::OpenTile(TileKind::Chat) => "Current chat".into(),
            Self::OpenTile(kind) => kind.label().into(),
            Self::Keys => "Keyboard shortcuts".into(),
            Self::WebSearch => "Search the web…".into(),
        }
    }
    /// The bound keystroke, shown in launcher rows with Kit's `Kbd`.
    pub(super) fn keystroke(self) -> Option<&'static str> {
        Some(match self {
            Self::NewThread => "cmd-t",
            Self::NewNote => "cmd-n",
            Self::Rewrite => "cmd-shift-r",
            Self::Save => "cmd-s",
            Self::OpenTile(TileKind::Threads) => "cmd-\\",
            Self::OpenTile(TileKind::Settings) => "cmd-,",
            _ => return None,
        })
    }
    pub(super) fn all() -> Vec<Self> {
        let mut out = vec![
            Self::NewThread,
            Self::NewNote,
            Self::Rewrite,
            Self::Save,
            Self::Outline,
        ];
        // System/Light/Dark first, then the legacy palettes under clear labels.
        out.extend(Appearance::PRIMARY.map(Self::Theme));
        out.extend(ThemeId::LEGACY.map(|theme| Self::Theme(Appearance::Legacy(theme))));
        out.extend(
            [
                TileKind::Notes,
                TileKind::Connections,
                TileKind::AllThreads,
                TileKind::Threads,
                TileKind::Note,
                TileKind::Chat,
                TileKind::Linked,
                TileKind::Projects,
                TileKind::History,
                TileKind::Actions,
                TileKind::Imports,
                TileKind::Settings,
                TileKind::Overview,
            ]
            .map(Self::OpenTile),
        );
        out.push(Self::Keys);
        out.push(Self::WebSearch);
        out
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum LaunchItem {
    Thread {
        id: String,
        title: String,
        resolved: bool,
    },
    /// A current general note: an article, or a full imported reference
    /// (`file`). Only current notes are listed; candidates never are.
    Note {
        id: String,
        title: String,
        file: bool,
    },
    /// A project-only document of the project the launcher opened from.
    /// It is listed only there, so it never reaches another project's or
    /// the general results.
    ProjectDoc {
        id: String,
        project: String,
        title: String,
        project_title: String,
        /// Other active projects also hold this document.
        shared: bool,
    },
    /// A current general note the owner search matched by its text rather
    /// than its title, shown with the matching passage; opens the note.
    Passage {
        id: String,
        title: String,
        file: bool,
        snippet: String,
    },
    /// A saved chat Message the owner search matched; opens its chat.
    Message {
        chat: String,
        message: String,
        chat_title: String,
        role: String,
        snippet: String,
        highlights: Vec<std::ops::Range<usize>>,
        archived: bool,
    },
    Command(Command),
}
impl LaunchItem {
    /// The visible text; the group heading and kind icon name the kind.
    pub(super) fn label(&self) -> String {
        match self {
            Self::Thread { title, .. }
            | Self::Note { title, .. }
            | Self::ProjectDoc { title, .. }
            | Self::Passage { title, .. } => title.clone(),
            Self::Message { snippet, .. } => snippet.clone(),
            Self::Command(command) => command.label(),
        }
    }
    /// Muted text after the label: a Message's chat and who wrote it, or
    /// a project document's project.
    pub(super) fn meta(&self) -> Option<String> {
        match self {
            Self::ProjectDoc { project_title, .. } => Some(project_title.clone()),
            Self::Passage { snippet, .. } => Some(snippet.clone()),
            Self::Message {
                chat_title, role, ..
            } => Some(format!(
                "{chat_title} · {}",
                super::search_view::role_label(role)
            )),
            _ => None,
        }
    }
    /// The announced name keeps the kind, the scope and the archived state.
    pub(super) fn accessible_label(&self) -> String {
        match self {
            Self::Thread {
                title, resolved, ..
            } => format!(
                "Chat: {title}{}",
                if *resolved { " · archived" } else { "" }
            ),
            Self::Note {
                title, file: false, ..
            } => format!("Article: {title}"),
            Self::Note {
                title, file: true, ..
            } => format!("File: {title} · full imported reference"),
            Self::ProjectDoc {
                title,
                project_title,
                shared: false,
                ..
            } => format!("Project document: {title} · only in {project_title}"),
            Self::ProjectDoc {
                title,
                project_title,
                shared: true,
                ..
            } => format!(
                "Project document: {title} · in {project_title} and other projects, not in Library"
            ),
            Self::Passage {
                title,
                file,
                snippet,
                ..
            } => format!(
                "{}: {title} · text matches: {snippet}",
                if *file { "File" } else { "Article" }
            ),
            Self::Message {
                chat_title,
                role,
                snippet,
                archived,
                ..
            } => format!(
                "Chat history in {chat_title}, {}: {snippet}{}",
                super::search_view::role_label(role),
                if *archived { " · archived" } else { "" }
            ),
            Self::Command(command) => command.label(),
        }
    }
    /// Results are ordered articles, files, this project's documents,
    /// chats, commands, then text matches and chat history, so headings
    /// follow order.
    pub(super) fn group(&self) -> &'static str {
        match self {
            Self::Note { file: false, .. } => "Articles",
            Self::Note { file: true, .. } => "Files",
            Self::ProjectDoc { .. } => "This project",
            Self::Thread { .. } => "Chats",
            Self::Command(_) => "Commands",
            Self::Passage { .. } => "In document text",
            Self::Message { .. } => "In chat history",
        }
    }
    /// The stable element key of the group heading.
    pub(super) fn group_key(&self) -> &'static str {
        match self {
            Self::Note { file: false, .. } => "articles",
            Self::Note { file: true, .. } => "files",
            Self::ProjectDoc { .. } => "project",
            Self::Thread { .. } => "chats",
            Self::Command(_) => "commands",
            Self::Passage { .. } => "in-documents",
            Self::Message { .. } => "in-chats",
        }
    }
    /// What the group's results are, beside its heading: current saved
    /// knowledge, or conversation history that is not current knowledge.
    pub(super) fn group_scope(&self) -> Option<&'static str> {
        match self {
            Self::Note { .. } | Self::Passage { .. } => Some("Current knowledge · saved library"),
            Self::ProjectDoc { .. } => Some("Project documents · not in Library"),
            Self::Thread { .. } => Some("Conversations"),
            Self::Message { .. } => Some("History · not current knowledge"),
            Self::Command(_) => None,
        }
    }
}
pub(super) const LAUNCH_LIMIT: usize = 20;

/// Empty queries show recent articles, recent chats, then commands;
/// otherwise every current article and file, thread (including resolved) and
/// command whose label matches (at most `LAUNCH_LIMIT`), then the current
/// notes the owner search matched only by their text (`passages`), then the
/// owner search's matching chat Messages (`messages`, as the Library search lists
/// them under "In chat history"). The Messages arrive after the label
/// matches, so they come last: an answer landing never moves a row the
/// selection or ↵ already points at. `notes` holds current general notes
/// only, so candidates never appear here; `project` holds the project-only
/// documents of the project the launcher opened from, matched by title.
pub(super) fn launcher(
    query: &str,
    threads: &[ThreadRow],
    notes: &[NoteRow],
    project: &[LaunchItem],
    passages: &[brn_threads_app::SearchHit],
    messages: &[brn_threads_app::ChatMessageHit],
) -> Vec<LaunchItem> {
    let query = query.trim().to_lowercase();
    let thread = |row: &ThreadRow| LaunchItem::Thread {
        id: row.id.clone(),
        title: row.title.clone(),
        resolved: row.resolved,
    };
    let note = |row: &NoteRow| LaunchItem::Note {
        id: row.id.clone(),
        title: row.title.clone(),
        file: row.is_file(),
    };
    if query.is_empty() {
        let mut recent: Vec<&NoteRow> = notes.iter().filter(|row| !row.is_file()).collect();
        recent.sort_by_key(|row| std::cmp::Reverse(row.order));
        return recent
            .into_iter()
            .take(4)
            .map(note)
            .chain(
                threads
                    .iter()
                    .filter(|row| !row.resolved)
                    .take(4)
                    .map(thread),
            )
            .chain(Command::all().into_iter().map(LaunchItem::Command))
            .take(LAUNCH_LIMIT)
            .collect();
    }
    let matches = |item: &LaunchItem| match item {
        // The project's name is in every one of its documents' labels.
        LaunchItem::ProjectDoc { title, .. } => title.to_lowercase().contains(&query),
        _ => item.accessible_label().to_lowercase().contains(&query),
    };
    let message = |hit: &brn_threads_app::ChatMessageHit| LaunchItem::Message {
        chat: hit.chat.clone(),
        message: hit.message.clone(),
        chat_title: hit.chat_title.clone(),
        role: hit.role.clone(),
        snippet: hit.snippet.clone(),
        highlights: hit.highlights.clone(),
        archived: hit.archived,
    };
    let mut items: Vec<LaunchItem> = notes
        .iter()
        .filter(|row| !row.is_file())
        .map(note)
        .chain(notes.iter().filter(|row| row.is_file()).map(note))
        .chain(project.iter().cloned())
        .chain(threads.iter().map(thread))
        .chain(Command::all().into_iter().map(LaunchItem::Command))
        .filter(matches)
        .take(LAUNCH_LIMIT)
        .collect();
    // The search already matched these; their text need not contain the
    // query verbatim. Only current general notes not already listed by
    // title, once each; the request asks for at most `LAUNCH_MESSAGES`.
    let mut listed: std::collections::HashSet<String> = items
        .iter()
        .filter_map(|item| match item {
            LaunchItem::Note { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect();
    for hit in passages {
        let Some(row) = notes.iter().find(|row| row.id == hit.note) else {
            continue;
        };
        if !listed.insert(row.id.clone()) {
            continue;
        }
        let snippet = if hit.snippet.is_empty() {
            &hit.quote
        } else {
            &hit.snippet
        };
        items.push(LaunchItem::Passage {
            id: row.id.clone(),
            title: row.title.clone(),
            file: row.is_file(),
            snippet: snippet.split_whitespace().collect::<Vec<_>>().join(" "),
        });
    }
    items.extend(messages.iter().map(message));
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_threads_app::*;

    fn follow_up_action(state: ActionState, date: Option<&str>) -> Action {
        Action {
            kept_outside_project: false,
            chat: None,
            description: "Synthetic follow-up".into(),
            state,
            internal: false,
            evidence: None,
            due: Some("1900-01-01".into()),
            owner: None,
            waiting_on: None,
            follow_up: date.map(str::to_owned),
        }
    }

    #[test]
    fn follow_up_groups_keep_commitments_dates_and_waiting_distinct() {
        use FollowUpGroup::*;
        let today = follow_up_date("2028-03-01").unwrap();
        for (state, date, expected) in [
            (ActionState::Open, Some("2028-02-29"), Overdue),
            (ActionState::Waiting, Some("2028-03-01"), Today),
            (ActionState::Open, Some("2028-03-02"), Upcoming),
            (ActionState::Waiting, None, Waiting),
            (ActionState::Open, None, Open),
            (ActionState::Suggested, Some("2000-01-01"), Suggested),
            (ActionState::Done, Some("2000-01-01"), Done),
            (ActionState::Cancelled, Some("2000-01-01"), Cancelled),
        ] {
            let action = follow_up_action(state, date);
            assert_eq!(follow_up_group(&action, today), expected);
            let mut other_due = action.clone();
            other_due.due = Some("2099-12-31".into());
            assert_eq!(
                follow_up_group(&other_due, today),
                expected,
                "due must not become follow-up"
            );
        }
        for invalid in [
            "",
            "tomorrow",
            "2027-02-29",
            "2028-02-30",
            "2028-3-01",
            "2028-03-01T00:00:00Z",
            " 2028-03-01",
        ] {
            assert_eq!(
                follow_up_group(
                    &follow_up_action(ActionState::Waiting, Some(invalid)),
                    today
                ),
                CheckDate,
                "{invalid}"
            );
        }
    }

    #[test]
    fn follow_up_day_uses_local_midnight_and_offset_at_current_instant() {
        let utc = chrono::NaiveDate::from_ymd_opt(2028, 3, 1)
            .unwrap()
            .and_hms_opt(0, 30, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        assert_eq!(follow_up_day_at(utc, 0).to_string(), "2028-03-01");
        assert_eq!(follow_up_day_at(utc, -3600).to_string(), "2028-02-29");
        assert_eq!(
            follow_up_day_at(utc + 23 * 3600, 3600).to_string(),
            "2028-03-02"
        );
        // Offset comes from the current instant, not a fixed zone/DST assumption.
        assert_eq!(follow_up_day_at(utc - 5400, 3600).to_string(), "2028-03-01");
        assert_eq!(follow_up_day_at(utc - 5401, 3600).to_string(), "2028-02-29");
    }

    #[test]
    fn follow_up_partition_is_read_only_complete_and_stably_ordered() {
        let today = follow_up_date("2028-03-01").unwrap();
        let mut records = vec![
            record(
                "later",
                RecordData::Action(follow_up_action(ActionState::Waiting, Some("2028-02-29"))),
            ),
            record(
                "z",
                RecordData::Action(follow_up_action(ActionState::Open, Some("2028-02-28"))),
            ),
            record(
                "a",
                RecordData::Action(follow_up_action(ActionState::Open, Some("2028-02-28"))),
            ),
            record(
                "undated",
                RecordData::Action(follow_up_action(ActionState::Waiting, None)),
            ),
            record(
                "archived",
                RecordData::Action(follow_up_action(ActionState::Open, Some("2028-02-27"))),
            ),
        ];
        records.last_mut().unwrap().archived = true;
        let before = records.clone();
        let overdue = follow_up_records(&records, FollowUpGroup::Overdue, today);
        assert_eq!(
            overdue.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["a", "z", "later"]
        );
        let all = FollowUpGroup::ALL
            .into_iter()
            .flat_map(|group| follow_up_records(&records, group, today))
            .collect::<Vec<_>>();
        assert_eq!(all.len(), 4);
        assert_eq!(all.iter().map(|r| &r.id).collect::<BTreeSet<_>>().len(), 4);
        assert_eq!(records, before);
        // Passing a scoped record list never pulls in records from elsewhere.
        assert_eq!(
            follow_up_records(&records[3..4], FollowUpGroup::Overdue, today).len(),
            0
        );
    }

    fn record(id: &str, data: RecordData) -> Record {
        Record {
            id: id.into(),
            version: 1,
            archived: false,
            data,
        }
    }
    fn thread(id: &str, title: &str, state: ThreadState, reason: Option<&str>) -> Record {
        record(
            id,
            RecordData::Thread(Thread {
                private_context: false,
                context_epoch: 0,
                title: title.into(),
                state,
                attention: reason
                    .map(|reason| Attention {
                        kind: AttentionKind::Question,
                        reason: reason.into(),
                        record: None,
                    })
                    .into_iter()
                    .collect(),
            }),
        )
    }
    thread_local! {
        static OPS: std::cell::RefCell<(tempfile::TempDir, Workspace)> = {
            let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
            let workspace = Workspace::open(root.path().join("ops")).unwrap();
            std::cell::RefCell::new((root, workspace))
        };
    }
    fn op(name: &str) -> OperationId {
        OPS.with(|ops| ops.borrow_mut().1.store.allocate_operation(name).unwrap())
    }
    fn receipt(
        op_name: &str,
        authority: &str,
        at: &str,
        writes: Vec<(Option<Record>, Record)>,
    ) -> Receipt {
        Receipt {
            operation: op(op_name),
            request_hash: String::new(),
            reason: "fixture".into(),
            writes: writes
                .into_iter()
                .map(|(before, after)| Written { before, after })
                .collect(),
            authority: authority.into(),
            committed_at: at.into(),
            undo_of: None,
            needs_refresh: vec![],
        }
    }
    fn note(id: &str, title: &str, protected: bool) -> Record {
        let mut note = Note::working(title, "text");
        note.protected = protected;
        record(id, RecordData::Note(note))
    }

    #[test]
    fn day_numbers_match_the_civil_calendar() {
        assert_eq!(day_number("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(day_number("2026-10-12T19:40:31.118Z"), Some(20_738));
        assert_eq!(day_number("2024-03-01"), Some(19_783));
        assert_eq!(day_number("garbage"), None);
        for (stamp, year) in [
            ("1970-01-01", 1970),
            ("2024-02-29", 2024),
            ("2024-12-31", 2024),
            ("2025-01-01", 2025),
            ("2026-10-14", 2026),
        ] {
            assert_eq!(civil_year(day_number(stamp).unwrap()), year, "{stamp}");
        }
    }

    #[test]
    fn human_times_read_as_today_or_a_short_local_date() {
        // 14 Oct 2026, 21:00 UTC.
        let now = day_number("2026-10-14").unwrap() * 86_400 + 21 * 3_600;
        let utc = |_: i64| 0;
        for (stamp, shown) in [
            ("2026-10-14T19:47:03.118Z", "Today 19:47"),
            ("2026-10-13T08:05:00Z", "13 Oct, 08:05"),
            ("2026-03-01T00:00:00Z", "1 Mar, 00:00"),
            ("2025-12-31T23:59:59Z", "31 Dec 2025, 23:59"),
            ("2026-10-14", "Today"),
            ("2026-10-02", "2 Oct"),
            ("garbage", "garbage"),
            ("", ""),
        ] {
            assert_eq!(human_time_at(stamp, now, utc), shown, "{stamp}");
        }
        // UTC+3 (EEST): the clock and the day move with the zone, and "now"
        // (00:00 on 15 Oct there) is a new local day.
        let eest = |_: i64| 3 * 3_600;
        for (stamp, shown) in [
            ("2026-10-14T19:47:03.118Z", "14 Oct, 22:47"),
            ("2026-10-14T21:30:00Z", "Today 00:30"),
            ("2025-12-31T22:30:00Z", "1 Jan, 01:30"),
            ("2026-10-15", "Today"),
        ] {
            assert_eq!(human_time_at(stamp, now, eest), shown, "{stamp}");
        }
        // Each instant gets its own offset (daylight saving).
        let summer = |epoch: i64| {
            if epoch >= day_number("2026-10-25").unwrap() * 86_400 {
                2 * 3_600
            } else {
                3 * 3_600
            }
        };
        assert_eq!(
            human_time_at("2026-10-30T10:00:00Z", now, summer),
            "30 Oct, 12:00"
        );
    }

    #[test]
    fn bookkeeping_and_reasons_read_as_the_owner_sees_them() {
        assert_eq!(
            receipt_reason("Chat control: recover paused queue"),
            "Recover paused queue"
        );
        assert_eq!(receipt_reason("Saved the brief"), "Saved the brief");
        let control = record(
            "control",
            RecordData::ChatControl(ChatControl {
                queue: vec![],
                pending: None,
                paused: false,
                quiet: false,
            }),
        );
        let only = |writes: Vec<Record>| {
            receipt(
                &format!("bookkeeping-{}", writes.len()),
                "owner:x",
                "2026-10-14T10:00:00Z",
                writes.into_iter().map(|after| (None, after)).collect(),
            )
        };
        assert!(is_background(&only(vec![control.clone()])));
        let mut refresh = only(vec![control.clone()]);
        refresh.needs_refresh = vec!["n".into()];
        assert!(
            !is_background(&refresh),
            "needs refresh is never background"
        );
        assert!(!is_background(&only(vec![
            control.clone(),
            note("n", "Brief", false)
        ])));
        assert!(!is_background(&only(vec![])));
        assert_eq!(
            affected_titles(&only(vec![
                note("a", "Brief", false),
                control,
                note("b", "Plan", false),
                note("c", "Brief", false),
                note("d", "Notes", false),
                note("e", "More", false),
            ])),
            "Brief, Plan and 2 more"
        );
    }

    #[test]
    fn sidebar_hides_resolved_and_bounds_open_threads() {
        let mut records = vec![
            thread(
                "needs",
                "Delivery date",
                ThreadState::Open,
                Some("Which date?"),
            ),
            thread("done", "Old question", ThreadState::Resolved, None),
        ];
        for i in 0..14 {
            records.push(thread(
                &format!("t{i:02}"),
                &format!("Chat {i}"),
                ThreadState::Open,
                None,
            ));
        }
        let history: Vec<_> = records
            .iter()
            .enumerate()
            .map(|(i, r)| {
                receipt(
                    &format!("op{i}"),
                    "owner:x",
                    "2026-10-12T10:00:00Z",
                    vec![(None, r.clone())],
                )
            })
            .collect();
        let rows = thread_rows(&records, &Activity::new(&history));
        let bar = sidebar(&rows, Some("t00"));
        assert_eq!(
            bar.needs.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["needs"]
        );
        assert_eq!(
            bar.open.len(),
            SIDEBAR_OPEN + 1,
            "current chat stays visible"
        );
        assert_eq!(bar.open[0].id, "t13", "most recent first");
        assert_eq!(bar.open.last().unwrap().id, "t00");
        assert!(bar.open.iter().all(|row| !row.resolved));
        assert_eq!((bar.total, bar.resolved), (16, 1));
        assert!(
            sidebar(&rows, Some("done"))
                .open
                .iter()
                .all(|row| row.id != "done")
        );
        assert_eq!(bar.recent_empty(), None);
    }

    #[test]
    fn recent_chats_says_no_chats_yet_only_without_chats() {
        let empty = |records: &[Record]| {
            let rows = thread_rows(records, &Activity::new(&[]));
            sidebar(&rows, None).recent_empty()
        };
        assert_eq!(empty(&[]), Some(RecentEmpty::NoChats));
        let needs = thread(
            "n",
            "Delivery update",
            ThreadState::Open,
            Some("Run failed"),
        );
        let archived = thread("a", "Old question", ThreadState::Resolved, None);
        assert_eq!(
            empty(std::slice::from_ref(&needs)),
            Some(RecentEmpty::OthersNeedYou)
        );
        assert_eq!(
            empty(&[needs, archived.clone()]),
            Some(RecentEmpty::OthersNeedYou)
        );
        assert_eq!(
            empty(std::slice::from_ref(&archived)),
            Some(RecentEmpty::AllArchived)
        );
        let open = thread("o", "Plan", ThreadState::Open, None);
        assert_eq!(empty(&[archived, open]), None);
    }

    #[test]
    fn all_threads_filters_and_groups_by_day() {
        let records = vec![
            thread("a", "Today open", ThreadState::Open, None),
            thread("b", "Week resolved", ThreadState::Resolved, None),
            thread(
                "c",
                "Old open",
                ThreadState::Open,
                Some("Review the proposal"),
            ),
            note("n", "Plan", false),
            record(
                "l",
                RecordData::Link(Link {
                    from: "n".into(),
                    to: "a".into(),
                    relation: "writing".into(),
                }),
            ),
        ];
        let history = vec![
            receipt(
                "1",
                "owner:x",
                "2026-09-01T10:00:00Z",
                vec![(None, records[2].clone())],
            ),
            receipt(
                "2",
                "owner:x",
                "2026-10-09T10:00:00Z",
                vec![(None, records[1].clone())],
            ),
            receipt(
                "3",
                "owner:x",
                "2026-10-12T10:00:00Z",
                vec![(None, records[0].clone())],
            ),
        ];
        let rows = thread_rows(&records, &Activity::new(&history));
        let today = day_number("2026-10-12").unwrap();
        let all = all_threads(&rows, ThreadFilter::All, "", today);
        assert_eq!(
            all.groups
                .iter()
                .map(|(name, rows)| (*name, rows.len()))
                .collect::<Vec<_>>(),
            [("Today", 1), ("This week", 1), ("Older", 1)]
        );
        let ids = |filter, query: &str| {
            all_threads(&rows, filter, query, today)
                .groups
                .into_iter()
                .flat_map(|(_, rows)| rows.into_iter().map(|row| row.id.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(ThreadFilter::Resolved, ""), ["b"]);
        assert_eq!(ids(ThreadFilter::NeedsYou, ""), ["c"]);
        assert_eq!(ids(ThreadFilter::LinkedNote, ""), ["a"]);
        assert_eq!(
            ids(ThreadFilter::Open, "proposal"),
            ["c"],
            "reasons are searchable"
        );
        assert!(ids(ThreadFilter::HasAction, "").is_empty());
    }

    #[test]
    fn note_rows_and_title_filter_scale_to_five_thousand_notes() {
        let records: Vec<_> = (0..5_000)
            .map(|i| {
                note(
                    &format!("n{i:04}"),
                    &format!("{} note {i}", ["Alpha", "Beta", "Õun", "1st"][i % 4]),
                    i % 50 == 0,
                )
            })
            .collect();
        let history: Vec<_> = records
            .iter()
            .enumerate()
            .map(|(i, r)| {
                receipt(
                    &format!("{i}"),
                    if i % 2 == 0 {
                        "owner:x"
                    } else {
                        "maintenance:y"
                    },
                    "2026-10-12T10:00:00Z",
                    vec![(None, r.clone())],
                )
            })
            .collect();
        let activity = Activity::new(&history);
        let rows = note_rows(&records, &activity);
        assert_eq!(rows.len(), 5_000);
        assert_eq!(rows.iter().filter(|row| row.protected).count(), 100);
        assert!(!activity.delegated("n0000") && activity.delegated("n0001"));
        let (shown, total) = filter_notes(&rows, "NOTE 499", 100);
        assert_eq!(total, 11);
        assert_eq!(shown.len(), 11);
        let (shown, total) = filter_notes(&rows, "beta", 100);
        assert_eq!((shown.len(), total), (100, 1_250));
    }

    #[test]
    fn descriptions_come_from_the_first_prose_line_without_markup() {
        assert_eq!(
            note_description(
                "# Harbor\n\n## Status\nThe **berth** plan uses [[Tides|tide tables]] and [the survey](https://x.test).\n\nMore."
            ),
            "The berth plan uses tide tables and the survey."
        );
        assert_eq!(
            note_description(
                "```\ncode only\n```\n| a | b |\n![figure](x.png)\n- First point\n- Second"
            ),
            "First point; Second"
        );
        assert_eq!(
            note_description("# Only a title\n"),
            "",
            "nothing is invented"
        );
        let long = note_description(&"word ".repeat(80));
        assert!(
            long.ends_with('…') && long.chars().count() <= DESCRIPTION_LIMIT + 1,
            "{long}"
        );
    }

    #[test]
    fn launcher_finds_resolved_threads_notes_and_commands() {
        let records = vec![
            thread("t", "Renewal date", ThreadState::Resolved, None),
            note("n", "Renewal contract", false),
        ];
        let activity = Activity::default();
        let threads = thread_rows(&records, &activity);
        let notes = note_rows(&records, &activity);
        let items = launcher("renewal", &threads, &notes, &[], &[], &[]);
        assert_eq!(items.len(), 2);
        // Current knowledge first, then conversations.
        assert_eq!(items[0].accessible_label(), "Article: Renewal contract");
        assert_eq!(items[0].group(), "Articles");
        assert_eq!(
            items[0].group_scope(),
            Some("Current knowledge · saved library")
        );
        assert_eq!(items[1].label(), "Renewal date");
        assert_eq!(items[1].accessible_label(), "Chat: Renewal date · archived");
        assert_eq!(items[1].group(), "Chats");
        assert_eq!(
            launcher("archived", &threads, &notes, &[], &[], &[]),
            [items[1].clone()],
            "the archived state stays searchable"
        );
        assert!(
            launcher("raba", &threads, &notes, &[], &[], &[]).contains(&LaunchItem::Command(
                Command::Theme(Appearance::Legacy(ThemeId::Raba))
            ))
        );
        // The primary System/Light/Dark commands are reachable by name.
        assert!(
            launcher("system", &threads, &notes, &[], &[], &[])
                .contains(&LaunchItem::Command(Command::Theme(Appearance::System)))
        );
        let empty = launcher("", &threads, &notes, &[], &[], &[]);
        assert!(
            !empty
                .iter()
                .any(|item| matches!(item, LaunchItem::Thread { .. })),
            "resolved stay out of recent"
        );
        assert!(empty.contains(&LaunchItem::Command(Command::NewThread)));
        assert_eq!(Command::NewThread.keystroke(), Some("cmd-t"));
        assert_eq!(Command::OpenTile(TileKind::Overview).keystroke(), None);
        assert_eq!(Command::Keys.keystroke(), None);
    }

    /// F-UI1: the launcher lists the owner search's chat Messages under
    /// "In chats", after every label match, with an archived label.
    #[test]
    fn launcher_lists_matching_chat_messages_after_the_label_matches() {
        let records = vec![note("n", "Berth plan", false)];
        let activity = Activity::default();
        let notes = note_rows(&records, &activity);
        let hit = |message: &str, snippet: &str, archived: bool| ChatMessageHit {
            message: message.into(),
            version: 1,
            chat: "chat".into(),
            chat_title: "Replies".into(),
            role: "owner".into(),
            snippet: snippet.into(),
            highlights: vec![std::ops::Range { start: 0, end: 5 }],
            archived,
        };
        // The service matched both; the second snippet lacks the word.
        let messages = [
            hit("m1", "Berth survey reply draft 1.4 m", false),
            hit("m2", "Survey reply, kept", true),
        ];
        let items = launcher("berth", &[], &notes, &[], &[], &messages);
        let groups: Vec<_> = items.iter().map(LaunchItem::group).collect();
        assert_eq!(groups, ["Articles", "In chat history", "In chat history"]);
        assert_eq!(
            items[1].group_scope(),
            Some("History · not current knowledge")
        );
        assert_eq!(items[1].label(), "Berth survey reply draft 1.4 m");
        assert_eq!(items[1].meta().as_deref(), Some("Replies · You"));
        assert_eq!(
            items[1].accessible_label(),
            "Chat history in Replies, You: Berth survey reply draft 1.4 m"
        );
        assert_eq!(
            items[2].accessible_label(),
            "Chat history in Replies, You: Survey reply, kept · archived"
        );
        assert!(matches!(
            &items[2],
            LaunchItem::Message { chat, message, archived: true, .. } if chat == "chat" && message == "m2"
        ));
        // Without a search answer nothing is invented.
        assert!(
            launcher("berth", &[], &[], &[], &[], &[]).is_empty(),
            "no label matches and no Messages"
        );
        // A word that is a command and also in saved chat text (the theme
        // names are everyday Estonian words): the command keeps its row
        // when the search answers, so ↵ and the selection never move to a
        // Message that arrived later.
        let before = launcher("raba", &[], &notes, &[], &[], &[]);
        let theme = LaunchItem::Command(Command::Theme(Appearance::Legacy(ThemeId::Raba)));
        assert_eq!(before.first(), Some(&theme));
        let raba = [hit("m3", "Raba walk on Saturday", false)];
        let after = launcher("raba", &[], &notes, &[], &[], &raba);
        assert_eq!(after[..before.len()], before[..], "earlier rows stay put");
        assert_eq!(after.first(), Some(&theme));
        let groups: Vec<_> = after.iter().map(LaunchItem::group).collect();
        assert_eq!(groups, ["Commands", "In chat history"]);
        assert!(matches!(
            after.last(),
            Some(LaunchItem::Message { message, .. }) if message == "m3"
        ));
    }

    /// A word only in a note's text finds the note, once, after the label
    /// matches; a note already listed by title and a non-current note are
    /// not repeated or invented.
    #[test]
    fn launcher_lists_notes_matched_only_by_their_text() {
        let records = vec![
            note("alpha", "Atomic Alpha", false),
            note("lamp", "Lamp decision", false),
        ];
        let notes = note_rows(&records, &Activity::default());
        let hit = |id: &str, snippet: &str| -> SearchHit {
            serde_json::from_value(serde_json::json!({
                "note": id, "version": 1, "title": "", "start": 0, "end": 1,
                "quote": "quote", "snippet": snippet,
            }))
            .unwrap()
        };
        let passages = [
            hit("lamp", "Use violet lamps."),
            hit("alpha", "The lamp is\nblue."),
            hit("alpha", "A second lamp passage."),
            hit("candidate", "Never listed."),
        ];
        let before = launcher("lamp", &[], &notes, &[], &[], &[]);
        let items = launcher("lamp", &[], &notes, &[], &passages, &[]);
        assert_eq!(items[..before.len()], before[..], "earlier rows stay put");
        let found: Vec<_> = items[before.len()..].to_vec();
        assert_eq!(
            found,
            [LaunchItem::Passage {
                id: "alpha".into(),
                title: "Atomic Alpha".into(),
                file: false,
                snippet: "The lamp is blue.".into(),
            }]
        );
        assert_eq!(found[0].group(), "In document text");
        assert_eq!(
            found[0].accessible_label(),
            "Article: Atomic Alpha · text matches: The lamp is blue."
        );
    }

    #[test]
    fn launcher_commands_name_every_destination_including_projects() {
        let labels: Vec<String> = Command::all().into_iter().map(Command::label).collect();
        for destination in [
            "Library",
            "Connections",
            "All chats",
            "Recent chats",
            "Current note",
            "Current chat",
            "Linked work",
            "Projects",
            "Recent changes",
            "Tasks",
            "Settings",
            "Inbox",
        ] {
            assert!(
                labels.iter().any(|label| label == destination),
                "{destination}"
            );
        }
        assert!(
            labels.iter().all(|label| !label.starts_with("Open tile:")),
            "{labels:?}"
        );
        assert!(labels.iter().any(|label| label == "Show or hide outline"));
        assert_eq!(Command::Outline.keystroke(), None);
        assert_eq!(
            launcher("projects", &[], &[], &[], &[], &[]),
            [LaunchItem::Command(Command::OpenTile(TileKind::Projects))]
        );
    }

    #[test]
    fn launcher_lists_the_context_project_documents_after_files() {
        let budget = LaunchItem::ProjectDoc {
            id: "doc".into(),
            project: "p".into(),
            title: "Expansion budget".into(),
            project_title: "Harbor expansion".into(),
            shared: false,
        };
        let items = launcher("budget", &[], &[], std::slice::from_ref(&budget), &[], &[]);
        assert_eq!(items, std::slice::from_ref(&budget));
        assert_eq!(items[0].group(), "This project");
        assert_eq!(
            items[0].group_scope(),
            Some("Project documents · not in Library")
        );
        assert_eq!(items[0].meta().as_deref(), Some("Harbor expansion"));
        assert_eq!(
            items[0].accessible_label(),
            "Project document: Expansion budget · only in Harbor expansion"
        );
        let shared = LaunchItem::ProjectDoc {
            id: "doc".into(),
            project: "p".into(),
            title: "Expansion budget".into(),
            project_title: "Harbor expansion".into(),
            shared: true,
        };
        assert_eq!(
            shared.accessible_label(),
            "Project document: Expansion budget · in Harbor expansion and other projects, not in Library",
            "a document linked into another project never claims exclusivity"
        );
        assert!(launcher("tides", &[], &[], std::slice::from_ref(&budget), &[], &[]).is_empty());
        assert!(
            launcher("harbor", &[], &[], &[budget], &[], &[]).is_empty(),
            "matched by its own title, not its project's"
        );
    }

    #[test]
    fn an_archived_article_leaves_the_library_and_search() {
        let mut records = vec![
            note("current", "Berth schedule", false),
            note("old", "Berth schedule 2025", false),
        ];
        let activity = Activity::default();
        let titles = |records: &[Record]| {
            let notes = note_rows(records, &activity);
            launcher("berth", &[], &notes, &[], &[], &[])
                .iter()
                .map(LaunchItem::label)
                .collect::<Vec<_>>()
        };
        assert_eq!(titles(&records), ["Berth schedule", "Berth schedule 2025"]);
        records[1].archived = true;
        assert_eq!(titles(&records), ["Berth schedule"]);
        assert_eq!(
            note_rows(&records, &activity)
                .iter()
                .map(|row| row.id.as_str())
                .collect::<Vec<_>>(),
            ["current"]
        );
    }
}
