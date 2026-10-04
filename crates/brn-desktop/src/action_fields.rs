//! Retained local field bytes and syntactic DTO conversion; workflow validates meaning.
use brn_workflow::actions::{ActionData, ActionPriority, ActionState};
use serde::Serialize;
use uuid::Uuid;

pub const LABELS: [&str; 12] = [
    "Title",
    "Description",
    "Owner (optional)",
    "Related person UUID (optional)",
    "Related project UUID (optional)",
    "Source UUIDs (space separated)",
    "Thread UUID (optional)",
    "Due on (YYYY-MM-DD, optional)",
    "Follow up on (YYYY-MM-DD, optional)",
    "Dependency UUIDs (space separated)",
    "Parent Action UUID (optional)",
    "Follows up Action UUID (optional)",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ActionFields {
    pub values: [String; 12],
    pub state: ActionState,
    pub priority: Option<ActionPriority>,
}

impl From<&ActionData> for ActionFields {
    fn from(data: &ActionData) -> Self {
        let id = |id: Option<Uuid>| id.map(|id| id.to_string()).unwrap_or_default();
        let ids = |ids: &[Uuid]| {
            ids.iter()
                .map(Uuid::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        };
        Self {
            values: [
                data.title.clone(),
                data.description.clone(),
                data.owner.clone().unwrap_or_default(),
                id(data.related_person),
                id(data.related_project),
                ids(&data.sources),
                id(data.thread),
                data.due_on.clone().unwrap_or_default(),
                data.follow_up_on.clone().unwrap_or_default(),
                ids(&data.dependencies),
                id(data.parent),
                id(data.follows_up),
            ],
            state: data.state,
            priority: data.priority,
        }
    }
}

impl ActionFields {
    pub fn data(&self) -> Result<ActionData, String> {
        let optional =
            |index: usize| (!self.values[index].is_empty()).then(|| self.values[index].clone());
        let id = |index: usize| -> Result<Option<Uuid>, String> {
            optional(index)
                .map(|text| {
                    Uuid::parse_str(&text).map_err(|_| {
                        format!(
                            "{} needs a complete UUID; local input retained",
                            LABELS[index]
                        )
                    })
                })
                .transpose()
        };
        let ids = |index: usize| -> Result<Vec<Uuid>, String> {
            self.values[index]
                .split_whitespace()
                .map(|text| {
                    Uuid::parse_str(text).map_err(|_| {
                        format!(
                            "{} needs complete UUIDs; local input retained",
                            LABELS[index]
                        )
                    })
                })
                .collect()
        };
        Ok(ActionData {
            title: self.values[0].clone(),
            description: self.values[1].clone(),
            state: self.state,
            owner: optional(2),
            related_person: id(3)?,
            related_project: id(4)?,
            sources: ids(5)?,
            thread: id(6)?,
            due_on: optional(7),
            follow_up_on: optional(8),
            dependencies: ids(9)?,
            parent: id(10)?,
            follows_up: id(11)?,
            priority: self.priority,
        })
    }
}
