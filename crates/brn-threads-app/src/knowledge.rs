//! The owner-triggered knowledge check: deterministic findings over general
//! knowledge (never chat-only material), as listed by the ordinary-wiki
//! guidance. Findings can seed one chat for a bounded `maintain-notes`
//! review. A clean check is not proof that the knowledge is correct.
use crate::*;

/// Findings kept per kind; `counts` reports every one.
pub const FINDINGS_PER_KIND: usize = 25;
/// Outgoing note links that make a page look like a topic page.
const TOPIC_LINKS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FindingKind {
    /// A link whose other end is missing, archived or superseded.
    BrokenLink,
    /// A note derived from an input version that is no longer current.
    StaleInput,
    /// An unapplied candidate whose target changed since it was prepared.
    StaleCandidate,
    /// Imported material with recorded coverage gaps.
    CoverageGap,
    /// A note with no links to or from other notes.
    Orphan,
    DuplicateTitle,
    DuplicateContent,
    /// Notes linked as `contradicts`: an unresolved disagreement.
    Contradiction,
    /// A `supersedes` link whose target is still current.
    Supersession,
    /// Relation names that look like variants of each other.
    RelationVariant,
    /// A page linking many notes that the Topics note does not list.
    UnlistedTopic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub kind: FindingKind,
    /// Records involved, most relevant first.
    pub records: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeCheck {
    pub notes: usize,
    pub links: usize,
    pub counts: BTreeMap<FindingKind, usize>,
    pub findings: Vec<Finding>,
}

fn words(text: &str) -> String {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let w = w.to_lowercase();
            match w.strip_suffix('s') {
                Some(stem) if stem.len() > 2 => stem.to_owned(),
                _ => w,
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn relation_key(relation: &str) -> String {
    relation
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .take(5)
        .collect()
}

struct Collector {
    counts: BTreeMap<FindingKind, usize>,
    findings: Vec<Finding>,
}

impl Collector {
    fn add(&mut self, kind: FindingKind, records: Vec<String>, detail: String) {
        let count = self.counts.entry(kind).or_default();
        *count += 1;
        if *count <= FINDINGS_PER_KIND {
            self.findings.push(Finding {
                kind,
                records,
                detail,
            });
        }
    }
}

impl Workspace {
    /// Runs the deterministic checks over current general knowledge.
    pub fn knowledge_check(&self) -> AppResult<KnowledgeCheck> {
        let all = self.store.records()?;
        let by_id: BTreeMap<&str, &Record> = all.iter().map(|r| (r.id.as_str(), r)).collect();
        let general = self.store.records_for_chat(None)?;
        let visible: BTreeSet<&str> = general.iter().map(|r| r.id.as_str()).collect();
        let notes: BTreeMap<&str, (&Record, &Note)> = general
            .iter()
            .filter_map(|r| match &r.data {
                RecordData::Note(n) => Some((r.id.as_str(), (r, n))),
                _ => None,
            })
            .collect();
        let title = |id: &str| {
            notes
                .get(id)
                .map_or_else(|| id.to_owned(), |(_, n)| format!("“{}”", n.title))
        };
        let mut out = Collector {
            counts: BTreeMap::new(),
            findings: Vec::new(),
        };

        // Links: every non-archived link with a visible general end.
        let mut note_links: Vec<(&str, &str, &str)> = Vec::new();
        let mut relations: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
        for record in all.iter().filter(|r| !r.archived) {
            let RecordData::Link(link) = &record.data else {
                continue;
            };
            let (from, to) = (link.from.as_str(), link.to.as_str());
            // Broken: the other end is missing, or is a general note or
            // source that was archived or superseded. Chat-owned records and
            // other kinds (chats, Actions) are private or not knowledge.
            let broken = |id: &str| match by_id.get(id) {
                None => true,
                Some(r) => match &r.data {
                    RecordData::Note(n) => {
                        n.chat.is_none() && (r.archived || n.superseded_by.is_some())
                    }
                    RecordData::Source(s) => s.chat.is_none() && r.archived,
                    _ => false,
                },
            };
            match (visible.contains(from), visible.contains(to)) {
                (true, true) => {
                    if notes.contains_key(from) && notes.contains_key(to) {
                        note_links.push((from, to, link.relation.as_str()));
                    }
                    if !relations::is_host_owned(&link.relation) {
                        relations
                            .entry(relation_key(&link.relation))
                            .or_default()
                            .insert(link.relation.as_str());
                    }
                }
                // A missing, archived or superseded end is reported; an end
                // that exists only in a chat is private and is not.
                (true, false) | (false, true) => {
                    let (seen, other) = if visible.contains(from) {
                        (from, to)
                    } else {
                        (to, from)
                    };
                    if broken(other) {
                        out.add(
                            FindingKind::BrokenLink,
                            vec![record.id.clone(), seen.to_owned()],
                            format!(
                                "{} has a `{}` link to a missing, archived or superseded record",
                                title(seen),
                                link.relation
                            ),
                        );
                    }
                }
                (false, false) => {}
            }
        }

        // Input versions recorded by the latest change to each note.
        let mut latest: BTreeMap<String, OperationId> = BTreeMap::new();
        for receipt in self.history()? {
            for write in &receipt.writes {
                if notes.contains_key(write.after.id.as_str()) {
                    latest.insert(write.after.id.clone(), receipt.operation.clone());
                }
            }
        }
        for (note, operation) in &latest {
            let Ok(prepared) = self.store.prepared(operation) else {
                continue;
            };
            for input in &prepared.request.inputs {
                if input.expectation != InputExpectation::Current || input.record == *note {
                    continue;
                }
                if let Some(current) = by_id.get(input.record.as_str())
                    && visible.contains(input.record.as_str())
                    && current.version > input.version
                {
                    out.add(
                        FindingKind::StaleInput,
                        vec![note.clone(), input.record.clone()],
                        format!(
                            "{} was derived from version {} of {}, now at version {}",
                            title(note),
                            input.version,
                            title(&input.record),
                            current.version
                        ),
                    );
                }
            }
        }

        for candidate in self.candidates()? {
            for write in &candidate.request.writes {
                if let Some(expected) = write.expected_version
                    && visible.contains(write.id.as_str())
                    && by_id.get(write.id.as_str()).map(|r| r.version) != Some(expected)
                {
                    out.add(
                        FindingKind::StaleCandidate,
                        vec![candidate.operation.as_str().to_owned(), write.id.clone()],
                        format!(
                            "An unapplied proposal expects version {expected} of {}, which has changed",
                            title(&write.id)
                        ),
                    );
                }
            }
        }

        for record in &general {
            let gaps = match &record.data {
                RecordData::Note(n) => n.import.as_ref().map(|i| i.gaps.len()).unwrap_or(0),
                RecordData::Source(s) => s.gaps.len(),
                _ => 0,
            };
            if gaps > 0 {
                out.add(
                    FindingKind::CoverageGap,
                    vec![record.id.clone()],
                    format!("{} records {gaps} coverage gap(s)", title(&record.id)),
                );
            }
        }

        let topics = self.settings()?.topics_note;
        let mut degree: BTreeMap<&str, usize> = BTreeMap::new();
        let mut outgoing: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for (from, to, relation) in &note_links {
            if from != to {
                *degree.entry(from).or_default() += 1;
                *degree.entry(to).or_default() += 1;
                outgoing.entry(from).or_default().insert(to);
            }
            match *relation {
                relations::CONTRADICTS => out.add(
                    FindingKind::Contradiction,
                    vec![from.to_string(), to.to_string()],
                    format!(
                        "{} and {} are linked as contradicting",
                        title(from),
                        title(to)
                    ),
                ),
                relations::SUPERSEDES => out.add(
                    FindingKind::Supersession,
                    vec![to.to_string(), from.to_string()],
                    format!(
                        "{} is superseded by {} but is still current",
                        title(to),
                        title(from)
                    ),
                ),
                _ => {}
            }
        }
        for id in notes.keys() {
            if !degree.contains_key(id) && topics.as_deref() != Some(*id) {
                out.add(
                    FindingKind::Orphan,
                    vec![id.to_string()],
                    format!("{} has no links to or from other notes", title(id)),
                );
            }
        }

        let mut titles: BTreeMap<String, Vec<&str>> = BTreeMap::new();
        let mut contents: BTreeMap<String, Vec<&str>> = BTreeMap::new();
        for (id, (_, note)) in &notes {
            titles.entry(words(&note.title)).or_default().push(id);
            let content = words(&note.markdown);
            if !content.is_empty() {
                contents.entry(content).or_default().push(id);
            }
        }
        for (kind, groups) in [
            (FindingKind::DuplicateTitle, &titles),
            (FindingKind::DuplicateContent, &contents),
        ] {
            for ids in groups.values().filter(|ids| ids.len() > 1) {
                let names: Vec<String> = ids.iter().map(|id| title(id)).collect();
                out.add(
                    kind,
                    ids.iter().map(|id| id.to_string()).collect(),
                    format!(
                        "{} notes share the same {}: {}",
                        ids.len(),
                        if kind == FindingKind::DuplicateTitle {
                            "title"
                        } else {
                            "content"
                        },
                        names.join(", ")
                    ),
                );
            }
        }

        for names in relations.values().filter(|names| names.len() > 1) {
            let names: Vec<&str> = names.iter().copied().collect();
            let preferred = names
                .iter()
                .find(|name| relations::is_recommended(name))
                .map(|name| format!("; the recommended name is `{name}`"))
                .unwrap_or_default();
            out.add(
                FindingKind::RelationVariant,
                vec![],
                format!(
                    "Relation names look like variants: {}{preferred}",
                    names
                        .iter()
                        .map(|n| format!("`{n}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }

        if let Some(topics) = topics.as_deref().filter(|id| notes.contains_key(id)) {
            let listed = outgoing.get(topics).cloned().unwrap_or_default();
            for (id, targets) in &outgoing {
                if *id != topics && targets.len() >= TOPIC_LINKS && !listed.contains(id) {
                    out.add(
                        FindingKind::UnlistedTopic,
                        vec![id.to_string(), topics.to_owned()],
                        format!(
                            "{} links {} notes like a topic page but is not linked from {}",
                            title(id),
                            targets.len(),
                            title(topics)
                        ),
                    );
                }
            }
        }

        Ok(KnowledgeCheck {
            notes: notes.len(),
            links: note_links.len(),
            counts: out.counts,
            findings: out.findings,
        })
    }

    /// Starts one chat holding the findings as an owner message, for a
    /// bounded `maintain-notes` review. Returns the chat ID.
    pub fn seed_knowledge_review(&mut self, check: &KnowledgeCheck) -> AppResult<String> {
        let total: usize = check.counts.values().sum();
        let chat = self
            .new_thread(&format!("Knowledge check · {total} findings"))?
            .id;
        let mut text = format!(
            "Knowledge check over {} current notes and {} note links found {total} findings.\n",
            check.notes, check.links
        );
        for (kind, count) in &check.counts {
            text.push_str(&format!("- {kind:?}: {count}\n"));
        }
        text.push('\n');
        for finding in &check.findings {
            text.push_str(&format!(
                "- {:?}: {} [{}]\n",
                finding.kind,
                finding.detail,
                finding.records.join(", ")
            ));
        }
        text.push_str(
            "\nReview these findings with the maintain-notes playbook within the run budget. \
             Report what was inspected, corrected and left uncertain. A clean check is not proof \
             of correctness.",
        );
        self.post_message(&chat, &text)?;
        Ok(chat)
    }
}
