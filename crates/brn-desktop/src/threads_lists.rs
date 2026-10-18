//! Navigation lists and small lifecycle rules derived from current records.
//! Canonical state and every write remain in the shared service.
use super::layout::{Preset, TileKind};
use super::state::linked;
use super::theme::ThemeId;
use brn_threads_app::{ActionState, Receipt, Record, RecordData, ThreadState, record_in_chat};
use std::collections::{BTreeMap, BTreeSet};

/// Latest receipt position and commit time for records and the threads their
/// messages and runs belong to.
#[derive(Default)]
pub(super) struct Activity(BTreeMap<String, (usize, String)>);
impl Activity {
    pub(super) fn new(history: &[Receipt]) -> Self {
        let mut out = BTreeMap::new();
        for (index, receipt) in history.iter().enumerate() {
            for write in &receipt.writes {
                let entry = (index, receipt.committed_at.clone());
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
        self.0.get(id).map_or(0, |(order, _)| *order)
    }
    pub(super) fn stamp(&self, id: &str) -> &str {
        self.0.get(id).map_or("", |(_, stamp)| stamp.as_str())
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
    pub(super) const ALL: [Self; 6] = [
        Self::All,
        Self::Open,
        Self::Resolved,
        Self::NeedsYou,
        Self::LinkedNote,
        Self::HasAction,
    ];
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Open => "open",
            Self::Resolved => "archived",
            Self::NeedsYou => "needs-you",
            Self::LinkedNote => "linked-note",
            Self::HasAction => "has-action",
        }
    }
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

pub(super) const SECTION_LIMIT: usize = 12;
pub(super) const LETTER_LIMIT: usize = 150;

pub(super) struct NotesView<'a> {
    pub(super) recent: Vec<&'a NoteRow>,
    pub(super) linked: (Vec<&'a NoteRow>, usize),
    pub(super) kept: (Vec<&'a NoteRow>, usize),
    /// Notes with inbound links, most linked first.
    pub(super) most_linked: (Vec<&'a NoteRow>, usize),
    pub(super) letters: Vec<(char, usize)>,
    pub(super) letter: (Vec<&'a NoteRow>, usize),
}

/// Notes tile sections: every list is bounded and reports how many it hides.
pub(super) fn notes_view<'a>(
    rows: &'a [NoteRow],
    linked_to_open: &BTreeSet<String>,
    selected: char,
) -> NotesView<'a> {
    let mut recent: Vec<_> = rows.iter().collect();
    recent.sort_by_key(|row| std::cmp::Reverse(row.order));
    recent.truncate(SECTION_LIMIT);
    let bounded = |items: Vec<&'a NoteRow>, limit: usize| {
        let total = items.len();
        (items.into_iter().take(limit).collect::<Vec<_>>(), total)
    };
    let mut letters: BTreeMap<char, usize> = BTreeMap::new();
    for row in rows {
        *letters.entry(letter(&row.title)).or_default() += 1;
    }
    NotesView {
        recent,
        linked: bounded(
            rows.iter()
                .filter(|row| linked_to_open.contains(&row.id))
                .collect(),
            SECTION_LIMIT,
        ),
        kept: bounded(
            rows.iter()
                .filter(|row| row.protected || row.imported)
                .collect(),
            SECTION_LIMIT,
        ),
        most_linked: bounded(
            {
                let mut linked: Vec<_> = rows.iter().filter(|row| row.inbound > 0).collect();
                linked.sort_by(|a, b| {
                    b.inbound
                        .cmp(&a.inbound)
                        .then_with(|| a.title.cmp(&b.title))
                });
                linked
            },
            SECTION_LIMIT,
        ),
        letters: letters.into_iter().collect(),
        letter: bounded(
            rows.iter()
                .filter(|row| letter(&row.title) == selected)
                .collect(),
            LETTER_LIMIT,
        ),
    }
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
    Layout(Preset),
    Theme(ThemeId),
    OpenTile(TileKind),
    Keys,
}
impl Command {
    /// Destinations are named, never "Open tile: …".
    pub(super) fn label(self) -> String {
        match self {
            Self::NewThread => "New chat".into(),
            Self::NewNote => "New note".into(),
            Self::Rewrite => "Rewrite with AI".into(),
            Self::Save => "Save note".into(),
            Self::Layout(preset) => format!("Layout: {}", preset.label()),
            Self::Theme(theme) => format!("Theme: {} · {}", theme.label(), theme.description()),
            Self::OpenTile(TileKind::Note) => "Current note".into(),
            Self::OpenTile(TileKind::Chat) => "Current chat".into(),
            Self::OpenTile(kind) => kind.label().into(),
            Self::Keys => "Keys and mouse".into(),
        }
    }
    /// The bound keystroke, shown in launcher rows with Kit's `Kbd`.
    pub(super) fn keystroke(self) -> Option<&'static str> {
        Some(match self {
            Self::NewThread => "cmd-t",
            Self::NewNote => "cmd-n",
            Self::Rewrite => "cmd-shift-r",
            Self::Save => "cmd-s",
            Self::Layout(Preset::Write) => "alt-1",
            Self::Layout(Preset::Chat) => "alt-2",
            Self::Layout(Preset::Solo) => "alt-3",
            Self::OpenTile(TileKind::Threads) => "cmd-\\",
            Self::OpenTile(TileKind::Settings) => "cmd-,",
            Self::OpenTile(TileKind::Overview) => "cmd-0",
            _ => return None,
        })
    }
    pub(super) fn all() -> Vec<Self> {
        let mut out = vec![Self::NewThread, Self::NewNote, Self::Rewrite, Self::Save];
        out.extend(Preset::ALL.map(Self::Layout));
        out.extend(ThemeId::ALL.map(Self::Theme));
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
                TileKind::Settings,
                TileKind::Overview,
            ]
            .map(Self::OpenTile),
        );
        out.push(Self::Keys);
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
    Note {
        id: String,
        title: String,
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
            Self::Thread { title, .. } | Self::Note { title, .. } => title.clone(),
            Self::Message { snippet, .. } => snippet.clone(),
            Self::Command(command) => command.label(),
        }
    }
    /// Muted text after the label: a Message's chat and who wrote it.
    pub(super) fn meta(&self) -> Option<String> {
        match self {
            Self::Message {
                chat_title, role, ..
            } => Some(format!(
                "{chat_title} · {}",
                super::search_view::role_label(role)
            )),
            _ => None,
        }
    }
    /// The announced name keeps the kind and the archived state.
    pub(super) fn accessible_label(&self) -> String {
        match self {
            Self::Thread {
                title, resolved, ..
            } => format!(
                "Chat: {title}{}",
                if *resolved { " · archived" } else { "" }
            ),
            Self::Note { title, .. } => format!("Note: {title}"),
            Self::Message {
                chat_title,
                role,
                snippet,
                archived,
                ..
            } => format!(
                "Message in {chat_title}, {}: {snippet}{}",
                super::search_view::role_label(role),
                if *archived { " · archived" } else { "" }
            ),
            Self::Command(command) => command.label(),
        }
    }
    /// Results are ordered chats, notes, commands, then chat messages, so
    /// headings follow order.
    pub(super) fn group(&self) -> &'static str {
        match self {
            Self::Thread { .. } => "Chats",
            Self::Note { .. } => "Notes",
            Self::Message { .. } => "In chats",
            Self::Command(_) => "Commands",
        }
    }
}
pub(super) const LAUNCH_LIMIT: usize = 20;

/// Empty queries show recent work then commands; otherwise every thread
/// (including resolved), note and command whose label matches (at most
/// `LAUNCH_LIMIT`), then the owner search's matching chat Messages
/// (`messages`, as the Notes search lists them under "In chats"). The
/// Messages arrive after the label matches, so they come last: an answer
/// landing never moves a row the selection or ↵ already points at.
pub(super) fn launcher(
    query: &str,
    threads: &[ThreadRow],
    notes: &[NoteRow],
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
    };
    if query.is_empty() {
        let mut recent: Vec<&NoteRow> = notes.iter().collect();
        recent.sort_by_key(|row| std::cmp::Reverse(row.order));
        return threads
            .iter()
            .filter(|row| !row.resolved)
            .take(4)
            .map(thread)
            .chain(recent.into_iter().take(4).map(note))
            .chain(Command::all().into_iter().map(LaunchItem::Command))
            .take(LAUNCH_LIMIT)
            .collect();
    }
    let matches = |item: &LaunchItem| item.accessible_label().to_lowercase().contains(&query);
    let message = |hit: &brn_threads_app::ChatMessageHit| LaunchItem::Message {
        chat: hit.chat.clone(),
        message: hit.message.clone(),
        chat_title: hit.chat_title.clone(),
        role: hit.role.clone(),
        snippet: hit.snippet.clone(),
        highlights: hit.highlights.clone(),
        archived: hit.archived,
    };
    threads
        .iter()
        .map(thread)
        .chain(notes.iter().map(note))
        .chain(Command::all().into_iter().map(LaunchItem::Command))
        .filter(matches)
        .take(LAUNCH_LIMIT)
        // The search already matched these; their text need not contain
        // the query verbatim. The request asks for at most
        // `LAUNCH_MESSAGES`.
        .chain(messages.iter().map(message))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_threads_app::*;

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
    fn notes_view_stays_bounded_at_five_thousand_notes() {
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
                    "owner:x",
                    "2026-10-12T10:00:00Z",
                    vec![(None, r.clone())],
                )
            })
            .collect();
        let rows = note_rows(&records, &Activity::new(&history));
        let linked: BTreeSet<String> = (0..30).map(|i| format!("n{i:04}")).collect();
        let view = notes_view(&rows, &linked, 'Õ');
        assert_eq!(view.recent.len(), SECTION_LIMIT);
        assert_eq!(view.recent[0].id, "n4999");
        assert_eq!((view.linked.0.len(), view.linked.1), (SECTION_LIMIT, 30));
        assert_eq!((view.kept.0.len(), view.kept.1), (SECTION_LIMIT, 100));
        assert_eq!(
            view.letters,
            [('#', 1_250), ('A', 1_250), ('B', 1_250), ('Õ', 1_250)]
        );
        assert_eq!((view.letter.0.len(), view.letter.1), (LETTER_LIMIT, 1_250));
        let (shown, total) = filter_notes(&rows, "NOTE 499", 100);
        assert_eq!(total, 11);
        assert_eq!(shown.len(), 11);
        let (shown, total) = filter_notes(&rows, "beta", 100);
        assert_eq!((shown.len(), total), (100, 1_250));
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
        let items = launcher("renewal", &threads, &notes, &[]);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label(), "Renewal date");
        assert_eq!(items[0].accessible_label(), "Chat: Renewal date · archived");
        assert_eq!(items[0].group(), "Chats");
        assert_eq!(items[1].accessible_label(), "Note: Renewal contract");
        assert_eq!(items[1].group(), "Notes");
        assert_eq!(
            launcher("archived", &threads, &notes, &[]),
            [items[0].clone()],
            "the archived state stays searchable"
        );
        assert!(
            launcher("raba", &threads, &notes, &[])
                .contains(&LaunchItem::Command(Command::Theme(ThemeId::Raba)))
        );
        let empty = launcher("", &threads, &notes, &[]);
        assert!(
            !empty
                .iter()
                .any(|item| matches!(item, LaunchItem::Thread { .. })),
            "resolved stay out of recent"
        );
        assert!(empty.contains(&LaunchItem::Command(Command::NewThread)));
        assert_eq!(Command::Layout(Preset::Chat).keystroke(), Some("alt-2"));
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
        let items = launcher("berth", &[], &notes, &messages);
        let groups: Vec<_> = items.iter().map(LaunchItem::group).collect();
        assert_eq!(groups, ["Notes", "In chats", "In chats"]);
        assert_eq!(items[1].label(), "Berth survey reply draft 1.4 m");
        assert_eq!(items[1].meta().as_deref(), Some("Replies · You"));
        assert_eq!(
            items[1].accessible_label(),
            "Message in Replies, You: Berth survey reply draft 1.4 m"
        );
        assert_eq!(
            items[2].accessible_label(),
            "Message in Replies, You: Survey reply, kept · archived"
        );
        assert!(matches!(
            &items[2],
            LaunchItem::Message { chat, message, archived: true, .. } if chat == "chat" && message == "m2"
        ));
        // Without a search answer nothing is invented.
        assert!(
            launcher("berth", &[], &[], &[]).is_empty(),
            "no label matches and no Messages"
        );
        // A word that is a command and also in saved chat text (the theme
        // names are everyday Estonian words): the command keeps its row
        // when the search answers, so ↵ and the selection never move to a
        // Message that arrived later.
        let before = launcher("raba", &[], &notes, &[]);
        let theme = LaunchItem::Command(Command::Theme(ThemeId::Raba));
        assert_eq!(before.first(), Some(&theme));
        let raba = [hit("m3", "Raba walk on Saturday", false)];
        let after = launcher("raba", &[], &notes, &raba);
        assert_eq!(after[..before.len()], before[..], "earlier rows stay put");
        assert_eq!(after.first(), Some(&theme));
        let groups: Vec<_> = after.iter().map(LaunchItem::group).collect();
        assert_eq!(groups, ["Commands", "In chats"]);
        assert!(matches!(
            after.last(),
            Some(LaunchItem::Message { message, .. }) if message == "m3"
        ));
    }

    #[test]
    fn launcher_commands_name_every_destination_including_projects() {
        let labels: Vec<String> = Command::all().into_iter().map(Command::label).collect();
        for destination in [
            "Notes",
            "Connections",
            "All chats",
            "Chats",
            "Current note",
            "Current chat",
            "Linked work",
            "Projects",
            "Changes",
            "Tasks",
            "Settings",
            "Home",
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
        assert_eq!(
            launcher("projects", &[], &[], &[]),
            [LaunchItem::Command(Command::OpenTile(TileKind::Projects))]
        );
    }
}
