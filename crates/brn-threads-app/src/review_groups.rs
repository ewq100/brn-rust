use crate::*;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewGroup {
    /// Candidate's complete Note/Action write identity; no text hunks are selected.
    pub id: String,
    pub title: String,
    pub kind: String,
    pub writes: Vec<String>,
    /// New primary records referenced by this group must be included together.
    pub required_groups: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewGroups {
    pub groups: Vec<ReviewGroup>,
    /// These side effects have no structural owner; keep the coherent full result
    /// or request a rewrite instead of guessing which group owns them.
    pub ungrouped_writes: Vec<String>,
}

fn references(data: &RecordData) -> Vec<&str> {
    match data {
        RecordData::Note(note) => note
            .import
            .iter()
            .map(|import| import.source.as_str())
            .chain(note.superseded_by.iter().map(String::as_str))
            .chain(
                note.markdown
                    .split("brn-asset://")
                    .skip(1)
                    .filter_map(|tail| {
                        tail.split(|c: char| {
                            c.is_whitespace() || matches!(c, ')' | '>' | '<' | '\"' | '\'' | '`')
                        })
                        .next()
                    }),
            )
            .collect(),
        RecordData::Asset(asset) => std::iter::once(asset.note.as_str())
            .chain(asset.source.iter().map(String::as_str))
            .collect(),
        RecordData::Link(link) => vec![&link.from, &link.to],
        RecordData::Comment(comment) => vec![&comment.note],
        RecordData::Message(message) => vec![&message.thread],
        RecordData::Thread(thread) => thread
            .attention
            .iter()
            .filter_map(|attention| attention.record.as_deref())
            .collect(),
        _ => vec![],
    }
}

fn groups(request: &ChangeRequest) -> ReviewGroups {
    let roots = request
        .writes
        .iter()
        .filter(|write| matches!(write.data, RecordData::Note(_) | RecordData::Action(_)))
        .map(|write| write.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut groups = vec![];
    for root in &roots {
        let mut included = BTreeSet::from([(*root).to_owned()]);
        // Revision inputs are global to this immutable change, so each selected
        // group keeps their host Source writes and atomic expected-version checks.
        // A structural Note reference alone would drop the guard for an Action.
        included.extend(
            request
                .writes
                .iter()
                .filter(|write| {
                    matches!(write.data, RecordData::Source(_))
                        && request.inputs.iter().any(|input| input.record == write.id)
                })
                .map(|write| write.id.clone()),
        );
        let mut required = BTreeSet::new();
        loop {
            let before = included.len();
            for write in &request.writes {
                match &write.data {
                    RecordData::Asset(asset) if asset.note == *root => {
                        included.insert(write.id.clone());
                    }
                    RecordData::Comment(comment) if comment.note == *root => {
                        included.insert(write.id.clone());
                    }
                    // A shared context thread/source does not make every sibling
                    // link belong to this group; attach links through its primary
                    // record or owned figure/comment only.
                    RecordData::Link(link)
                        if [&link.from, &link.to].iter().any(|endpoint| {
                            endpoint.as_str() == *root
                                || request.writes.iter().any(|member| {
                                    member.id.as_str() == endpoint.as_str()
                                        && included.contains(&member.id)
                                        && matches!(
                                            member.data,
                                            RecordData::Asset(_) | RecordData::Comment(_)
                                        )
                                })
                        }) =>
                    {
                        included.insert(write.id.clone());
                    }
                    RecordData::Message(message) if included.contains(&message.thread) => {
                        included.insert(write.id.clone());
                    }
                    _ => {}
                }
            }
            let members = included.clone();
            for member in request
                .writes
                .iter()
                .filter(|write| members.contains(&write.id))
            {
                for reference in references(&member.data) {
                    if let Some(target) = request.writes.iter().find(|write| write.id == reference)
                    {
                        if roots.contains(reference) {
                            let owns_asset = matches!(&member.data, RecordData::Asset(asset) if asset.note == reference);
                            if reference != *root
                                && (target.expected_version.is_none() || owns_asset)
                            {
                                required.insert(reference.to_owned());
                            }
                        } else {
                            included.insert(reference.to_owned());
                        }
                    }
                }
            }
            if included.len() == before {
                break;
            }
        }
        let write = request
            .writes
            .iter()
            .find(|write| write.id == *root)
            .unwrap();
        let (kind, title) = match &write.data {
            RecordData::Note(note) => ("Note", note.title.clone()),
            RecordData::Action(action) => ("Action", action.description.clone()),
            _ => unreachable!(),
        };
        groups.push(ReviewGroup {
            id: (*root).into(),
            title,
            kind: kind.into(),
            writes: included.into_iter().collect(),
            required_groups: required.into_iter().collect(),
        });
    }
    let ungrouped_writes = request
        .writes
        .iter()
        .filter(|write| !groups.iter().any(|group| group.writes.contains(&write.id)))
        .map(|write| write.id.clone())
        .collect();
    ReviewGroups {
        groups,
        ungrouped_writes,
    }
}

impl Workspace {
    pub fn review_groups(&self, id: &str) -> AppResult<ReviewGroups> {
        let operation = self.store.operation(id)?;
        Ok(groups(&self.store.prepared(&operation)?.request))
    }

    /// Prepare a newly displayed immutable result before any approval. The old
    /// candidate is retired atomically and its Review attention follows replacement.
    /// Bases stay exact: selecting groups does not silently rebase stale writing.
    pub fn review_select(&mut self, id: &str, selected: &[String]) -> AppResult<Prepared> {
        let original = self.store.operation(id)?;
        let mut request = self.store.prepared(&original)?.request;
        let groups = groups(&request);
        let selected = selected.iter().cloned().collect::<BTreeSet<_>>();
        if selected.is_empty()
            || selected
                .iter()
                .any(|id| !groups.groups.iter().any(|group| &group.id == id))
        {
            return Err(AppError::Invalid(
                "Select one or more complete Note/Action groups from this candidate".into(),
            ));
        }
        if selected.len() < groups.groups.len() {
            if !groups.ungrouped_writes.is_empty() {
                return Err(AppError::Invalid("This proposal has ungrouped supporting changes; keep it together or request a rewritten candidate".into()));
            }
            let mut retained = BTreeSet::new();
            for group in groups
                .groups
                .iter()
                .filter(|group| selected.contains(&group.id))
            {
                if group
                    .required_groups
                    .iter()
                    .any(|required| !selected.contains(required))
                {
                    return Err(AppError::Invalid(format!(
                        "{} requires the new related groups: {}. Select them together or request a rewritten candidate",
                        group.title,
                        group.required_groups.join(", ")
                    )));
                }
                retained.extend(group.writes.iter().cloned());
            }
            request.writes.retain(|write| retained.contains(&write.id));
        }
        request.reason = format!("Owner selected complete review groups: {}", request.reason);
        self.replace_review_request(original, request)
    }
}
