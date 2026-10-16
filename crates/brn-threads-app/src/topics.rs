//! The root Topics note (D6): an ordinary note whose links list the topic
//! pages, in the order the note mentions them. Only the owner sets it.
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicEntry {
    pub note: String,
    pub version: u64,
    pub title: String,
    /// The topic page's first line of prose.
    pub description: String,
    /// Current general notes the topic page links to.
    pub linked_notes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Topics {
    pub note: String,
    pub version: u64,
    pub title: String,
    pub entries: Vec<TopicEntry>,
}

pub(crate) const STARTER: &str = "Topic pages for this workspace. Each topic page links the subject pages of one area; link every topic page from here.\n";

fn description(markdown: &str) -> String {
    let line = markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .unwrap_or_default();
    let mut text: String = line.chars().take(160).collect();
    if line.chars().count() > 160 {
        text.push('…');
    }
    text
}

impl Workspace {
    /// The configured Topics note and its topic pages, or `None` when no
    /// pointer is set or it no longer names a current general note.
    pub fn topics(&self) -> AppResult<Option<Topics>> {
        let Some(id) = self.settings()?.topics_note else {
            return Ok(None);
        };
        let general = self.store.records_for_chat(None)?;
        let notes: BTreeMap<&str, (&Record, &Note)> = general
            .iter()
            .filter(|r| !r.archived)
            .filter_map(|r| match &r.data {
                RecordData::Note(n) if n.superseded_by.is_none() && n.chat.is_none() => {
                    Some((r.id.as_str(), (r, n)))
                }
                _ => None,
            })
            .collect();
        let Some((record, topics)) = notes.get(id.as_str()) else {
            return Ok(None);
        };
        let links: Vec<&Link> = general
            .iter()
            .filter(|r| !r.archived)
            .filter_map(|r| match &r.data {
                RecordData::Link(link) => Some(link),
                _ => None,
            })
            .collect();
        let linked = |from: &str| -> BTreeSet<&str> {
            links
                .iter()
                .filter(|l| l.from == from && l.to != from && notes.contains_key(l.to.as_str()))
                .map(|l| l.to.as_str())
                .collect()
        };
        let text = topics.markdown.to_lowercase();
        let mut entries: Vec<TopicEntry> = linked(&id)
            .into_iter()
            .map(|to| {
                let (r, n) = notes[to];
                TopicEntry {
                    note: r.id.clone(),
                    version: r.version,
                    title: n.title.clone(),
                    description: description(&n.markdown),
                    linked_notes: linked(to).len(),
                }
            })
            .collect();
        let position =
            |entry: &TopicEntry| text.find(&entry.title.to_lowercase()).unwrap_or(usize::MAX);
        entries.sort_by(|a, b| {
            position(a)
                .cmp(&position(b))
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.note.cmp(&b.note))
        });
        Ok(Some(Topics {
            note: id.clone(),
            version: record.version,
            title: topics.title.clone(),
            entries,
        }))
    }

    /// Points the workspace at `note` as its Topics note (owner only).
    pub fn choose_topics_note(&mut self, note: &str) -> AppResult<ApplyOutcome> {
        let op = self
            .store
            .allocate_operation(&format!("topics-note:{}", uuid::Uuid::new_v4()))?;
        Ok(self.store.choose_topics_note(
            &op,
            note,
            &HostAuthority::owner("owner Topics note choice"),
        )?)
    }

    /// First use: creates an ordinary "Topics" note as an owner action and
    /// points the workspace at it. A deferred pointer leaves the note in place.
    pub fn create_topics_note(&mut self) -> AppResult<(Record, ApplyOutcome)> {
        let note = self.new_note("Topics", STARTER)?;
        let outcome = self.choose_topics_note(&note.id)?;
        Ok((note, outcome))
    }
}
