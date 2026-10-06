//! Semantic Action candidates; workflow owns UUID parsing and deterministic admission.
//!
//! These Serde types are also the single structural source of the
//! `propose_actions` tool schema. Schema bounds are provider hints; the Rust
//! `validate` methods below remain the UTF-8 byte-limit authority.
use crate::{AiError, AiErrorKind, AiResult};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize};
use std::{borrow::Cow, marker::PhantomData};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionCandidate {
    Create {
        data: ActionCandidateData,
    },
    Replace {
        target: CheckedActionRef,
        data: ActionCandidateData,
    },
}

/// Exact full-record reference returned by read_action; never ID-only authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckedActionRef {
    #[schemars(with = "WireId")]
    pub id: String,
    #[schemars(range(min = 1, max = i64::MAX))]
    pub version: u64,
    #[schemars(length(equal = 64), regex(pattern = "^[0-9a-f]{64}$"))]
    pub sha256: String,
}

/// Existing UUID or a 1-based member in this same ordered proposal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionRef {
    Existing {
        #[schemars(with = "WireId")]
        id: String,
    },
    Member {
        #[schemars(range(min = 1, max = 20))]
        index: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionCandidateState {
    Open,
    Waiting,
    Blocked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionCandidatePriority {
    Low,
    Normal,
    High,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionCandidateData {
    #[schemars(length(min = 1, max = 512))]
    pub title: String,
    #[schemars(length(max = 65536))]
    pub description: String,
    pub state: ActionCandidateState,
    #[serde(deserialize_with = "required_nullable")]
    #[schemars(with = "RequiredNullable<String>", length(max = 512))]
    pub owner: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    #[schemars(with = "RequiredNullable<WireId>")]
    pub related_person: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    #[schemars(with = "RequiredNullable<WireId>")]
    pub related_project: Option<String>,
    #[schemars(with = "Vec<WireId>", length(max = 64))]
    pub sources: Vec<String>,
    #[serde(deserialize_with = "required_nullable")]
    #[schemars(with = "RequiredNullable<WireId>")]
    pub thread: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    #[schemars(with = "RequiredNullable<WireDate>")]
    pub due_on: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    #[schemars(with = "RequiredNullable<WireDate>")]
    pub follow_up_on: Option<String>,
    #[schemars(length(max = 64))]
    pub dependencies: Vec<ActionRef>,
    #[serde(deserialize_with = "required_nullable")]
    #[schemars(with = "RequiredNullable<ActionRef>")]
    pub parent: Option<ActionRef>,
    #[serde(deserialize_with = "required_nullable")]
    #[schemars(with = "RequiredNullable<ActionRef>")]
    pub follows_up: Option<ActionRef>,
    #[serde(deserialize_with = "required_nullable")]
    #[schemars(with = "RequiredNullable<ActionCandidatePriority>")]
    pub priority: Option<ActionCandidatePriority>,
}

// Schema-only wire forms. Workflow parses UUIDs/dates; `validate` owns bytes.
#[derive(JsonSchema)]
#[expect(dead_code, reason = "schema-only")]
#[schemars(extend("format" = "uuid"))]
struct WireId(#[schemars(length(min = 1, max = 64))] String);
#[derive(JsonSchema)]
#[expect(dead_code, reason = "schema-only")]
#[schemars(extend("format" = "date"))]
struct WireDate(#[schemars(length(max = 10))] String);

/// Schema for an `Option` read through `required_nullable`: the field stays
/// required while explicit null remains valid. `schemars(required)` alone would
/// drop null, because it uses `T`'s non-optional schema.
struct RequiredNullable<T>(PhantomData<T>);
impl<T: JsonSchema> JsonSchema for RequiredNullable<T> {
    fn inline_schema() -> bool {
        true
    }
    fn schema_name() -> Cow<'static, str> {
        <Option<T>>::schema_name()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        <Option<T>>::json_schema(generator)
    }
}

// A deserialize_with field is required even when its type is Option. Explicit
// null still means None and is serialized back as null, preserving exact intent.
fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}
fn rejected() -> AiError {
    AiError::new(AiErrorKind::ToolRejected)
}
fn bounded_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
}
impl CheckedActionRef {
    pub fn validate(&self) -> AiResult<()> {
        if !bounded_id(&self.id)
            || !(1..=i64::MAX as u64).contains(&self.version)
            || self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(rejected());
        }
        Ok(())
    }
}
impl ActionRef {
    fn validate(&self) -> AiResult<()> {
        if match self {
            Self::Existing { id } => !bounded_id(id),
            Self::Member { index } => !(1..=20).contains(index),
        } {
            return Err(rejected());
        }
        Ok(())
    }
}
impl ActionCandidate {
    pub(crate) fn validate(&self) -> AiResult<()> {
        let data = match self {
            Self::Create { data } => data,
            Self::Replace { target, data } => {
                target.validate()?;
                data
            }
        };
        if !(1..=512).contains(&data.title.len())
            || data.description.len() > 65536
            || data.owner.as_ref().is_some_and(|s| s.len() > 512)
            || [&data.related_person, &data.related_project, &data.thread]
                .into_iter()
                .flatten()
                .any(|id| !bounded_id(id))
            || data.sources.len() > 64
            || data.sources.iter().any(|id| !bounded_id(id))
            || [&data.due_on, &data.follow_up_on]
                .into_iter()
                .flatten()
                .any(|date| date.len() > 10)
            || data.dependencies.len() > 64
        {
            return Err(rejected());
        }
        for reference in data
            .dependencies
            .iter()
            .chain(data.parent.iter())
            .chain(data.follows_up.iter())
        {
            reference.validate()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn whole() -> Value {
        json!({"title":"Exact candidate õ\r\n","description":"\u{feff}🦀\r\n","state":"blocked",
            "owner":null,"related_person":null,"related_project":null,"sources":[],"thread":null,
            "due_on":null,"follow_up_on":null,"dependencies":[],"parent":null,"follows_up":null,"priority":null})
    }
    fn candidate(data: Value) -> ActionCandidate {
        serde_json::from_value(json!({"kind":"create","data":data})).unwrap()
    }

    #[test]
    fn all_fourteen_fields_are_required_nulls_round_trip_and_shapes_are_closed() {
        let original = whole();
        let fields = original.as_object().unwrap();
        assert_eq!(fields.len(), 14);
        for field in fields.keys() {
            let mut incomplete = original.clone();
            incomplete.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<ActionCandidateData>(incomplete).is_err(),
                "{field}"
            );
        }
        let parsed: ActionCandidateData = serde_json::from_value(original.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), original);
        for input in [
            json!({"kind":"create","id":"model-id","data":whole()}),
            json!({"kind":"replace","before":{},"data":whole()}),
            json!({"kind":"create","data":whole(),"approve":true}),
            json!({"kind":"trash","data":whole()}),
        ] {
            assert!(serde_json::from_value::<ActionCandidate>(input).is_err());
        }
        for (field, value) in [
            ("state", json!("completed")),
            ("priority", json!("urgent")),
            ("parent", json!("legacy-uuid")),
            ("sources", Value::Null),
        ] {
            let mut malformed = original.clone();
            malformed[field] = value;
            assert!(
                serde_json::from_value::<ActionCandidateData>(malformed).is_err(),
                "{field}"
            );
        }
        let mut unknown = original;
        unknown["identity"] = json!("model-id");
        assert!(serde_json::from_value::<ActionCandidateData>(unknown).is_err());
        for input in [
            json!({"kind":"member","index":1,"id":"x"}),
            json!({"kind":"existing","id":"x","version":1}),
            json!({"kind":"member","index":-1}),
            json!({"kind":"member","index":1.5}),
            json!({"kind":"member"}),
            json!({"kind":"existing"}),
        ] {
            assert!(serde_json::from_value::<ActionRef>(input).is_err());
        }
        let mut checked = json!({"id":"x","version":1,"sha256":"a".repeat(64)});
        checked["data"] = whole();
        assert!(serde_json::from_value::<CheckedActionRef>(checked).is_err());
    }

    #[test]
    fn candidate_bounds_cover_nested_references_and_leave_domain_semantics_to_workflow() {
        let invalid = [
            ("title", json!("")),
            ("title", json!("õ".repeat(257))),
            ("description", json!("x".repeat(65537))),
            ("owner", json!("õ".repeat(257))),
            ("related_person", json!("")),
            ("related_project", json!("x".repeat(65))),
            ("thread", json!("x".repeat(65))),
            ("sources", json!([""])),
            ("sources", json!(vec!["x"; 65])),
            ("sources", json!(["x".repeat(65)])),
            ("due_on", json!("x".repeat(11))),
            ("follow_up_on", json!("x".repeat(11))),
            (
                "dependencies",
                json!(vec![json!({"kind":"member","index":1}); 65]),
            ),
            ("dependencies", json!([{"kind":"member","index":0}])),
            ("dependencies", json!([{"kind":"member","index":21}])),
            (
                "dependencies",
                json!([{"kind":"existing","id":"x".repeat(65)}]),
            ),
            ("parent", json!({"kind":"existing","id":""})),
            ("parent", json!({"kind":"member","index":0})),
            ("follows_up", json!({"kind":"member","index":21})),
        ];
        for (field, value) in invalid {
            let mut data = whole();
            data[field] = value;
            assert_eq!(
                candidate(data).validate().unwrap_err().kind,
                AiErrorKind::ToolRejected,
                "{field}"
            );
        }
        let mut maximum = whole();
        maximum["title"] = json!("õ".repeat(256));
        maximum["description"] = json!("x".repeat(65536));
        maximum["owner"] = json!("x".repeat(512));
        maximum["sources"] = json!(vec!["x".repeat(64); 64]);
        maximum["dependencies"] = json!(vec![json!({"kind":"existing","id":"x".repeat(64)}); 64]);
        maximum["parent"] = json!({"kind":"member","index":1});
        maximum["follows_up"] = json!({"kind":"member","index":20});
        maximum["due_on"] = json!("not-a-date");
        maximum["related_person"] = json!("not-a-uuid");
        // Protocol allows duplicate/unresolved edges and invalid UUID/date meaning;
        // deterministic domain admission (including actual member length) owns those.
        assert!(candidate(maximum).validate().is_ok());
        for state in ["open", "waiting", "blocked"] {
            for priority in [Value::Null, json!("low"), json!("normal"), json!("high")] {
                let mut data = whole();
                data["state"] = json!(state);
                data["priority"] = priority;
                assert!(candidate(data).validate().is_ok());
            }
        }
    }

    #[test]
    fn checked_reference_requires_signed_revision_and_exact_lowercase_hash() {
        let whole = CheckedActionRef {
            id: "workflow parses UUID".into(),
            version: 1,
            sha256: "0123456789abcdef".repeat(4),
        };
        for version in [1, i64::MAX as u64] {
            let mut reference = whole.clone();
            reference.version = version;
            assert!(reference.validate().is_ok());
        }
        for version in [0, i64::MAX as u64 + 1, u64::MAX] {
            let mut reference = whole.clone();
            reference.version = version;
            assert!(reference.validate().is_err());
        }
        for hash in [
            "a".repeat(63),
            "a".repeat(65),
            "A".repeat(64),
            "g".repeat(64),
            "õ".repeat(32),
        ] {
            let mut reference = whole.clone();
            reference.sha256 = hash;
            assert!(reference.validate().is_err());
        }
        for id in [String::new(), "x".repeat(65)] {
            let mut reference = whole.clone();
            reference.id = id;
            assert!(reference.validate().is_err());
        }
    }
}
