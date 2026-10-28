//! Bounded, current Action discovery in the run's existing read scope.
use super::*;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Args {
    state: Option<ActionState>,
    owner: Option<String>,
    due: Option<DateFilter>,
    follow_up: Option<DateFilter>,
    query: Option<String>,
    limit: usize,
    cursor: Option<String>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DateFilter {
    from: Option<String>,
    through: Option<String>,
    missing: bool,
}

fn date(s: &str) -> bool {
    s.len() == 10
        && chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok_and(|d| d.to_string() == s)
}
fn valid(a: &Args) -> bool {
    (1..=100).contains(&a.limit)
        && a.query
            .as_ref()
            .is_none_or(|s| !s.is_empty() && s.len() <= 512)
        && a.owner
            .as_ref()
            .is_none_or(|s| !s.is_empty() && s.len() <= 512)
        && a.cursor
            .as_ref()
            .is_none_or(|s| s.len() <= 96 && !s.is_empty())
        && [&a.due, &a.follow_up].iter().all(|f| {
            f.as_ref().is_none_or(|f| {
                (!f.missing || (f.from.is_none() && f.through.is_none()))
                    && (f.missing || f.from.is_some() || f.through.is_some())
                    && f.from.as_ref().is_none_or(|s| date(s))
                    && f.through.as_ref().is_none_or(|s| date(s))
                    && f.from
                        .as_ref()
                        .zip(f.through.as_ref())
                        .is_none_or(|(a, b)| a <= b)
            })
        })
}
pub(super) fn validate(v: &Value) -> bool {
    [
        "state",
        "owner",
        "due",
        "follow_up",
        "query",
        "limit",
        "cursor",
    ]
    .iter()
    .all(|k| v.get(k).is_some())
        && serde_json::from_value::<Args>(v.clone()).is_ok_and(|a| valid(&a))
}
pub(super) fn schema() -> Value {
    let df = json!({"type":["object","null"],"properties":{"from":{"type":["string","null"],"description":"Inclusive YYYY-MM-DD."},"through":{"type":["string","null"],"description":"Inclusive YYYY-MM-DD."},"missing":{"type":"boolean","description":"Only null dates; cannot be combined with bounds."}},"required":["from","through","missing"],"additionalProperties":false});
    object(
        json!({"state":{"type":["string","null"],"enum":["Suggested","Open","Waiting","Done","Cancelled",null]},"owner":{"type":["string","null"],"description":"Exact owner label; null means no owner filter."},"due":df,"follow_up":df,"query":{"type":["string","null"],"description":"Literal case-insensitive description substring, at most 512 UTF-8 bytes."},"limit":{"type":"integer","minimum":1,"maximum":100},"cursor":{"type":["string","null"],"description":"Previous cursor with identical filters and limit; stale changes require restarting."}}),
        json!([
            "state",
            "owner",
            "due",
            "follow_up",
            "query",
            "limit",
            "cursor"
        ]),
    )
}
fn matches_date(value: &Option<String>, filter: &Option<DateFilter>) -> bool {
    let Some(f) = filter else {
        return true;
    };
    if f.missing {
        return value.is_none();
    }
    value.as_ref().is_some_and(|v| {
        date(v)
            && f.from.as_ref().is_none_or(|x| v >= x)
            && f.through.as_ref().is_none_or(|x| v <= x)
    })
}
pub(super) fn query(scope: &Scope, mut args: Args) -> AiResult<Value> {
    if !valid(&args)
        || scope.scout.is_some()
        || scope
            .writing_snapshot
            .as_ref()
            .is_some_and(|s| s.setup.is_none())
    {
        return Err(rejected());
    }
    let store = scope.store.lock().map_err(storage)?;
    live(scope, &store)?;
    let all = store.records().map_err(storage)?;
    let mut rows = Vec::new();
    for r in &all {
        let RecordData::Action(a) = &r.data else {
            continue;
        };
        if r.archived
            || !record_in_chat(r, &all, Some(&scope.chat))
            || !writing_read_allowed_record(scope, &store, &all, r, r.version)?
        {
            continue;
        }
        if args.state.as_ref().is_some_and(|s| s != &a.state)
            || args
                .owner
                .as_ref()
                .is_some_and(|s| a.owner.as_ref() != Some(s))
            || !matches_date(&a.due, &args.due)
            || !matches_date(&a.follow_up, &args.follow_up)
            || args
                .query
                .as_ref()
                .is_some_and(|q| !a.description.to_lowercase().contains(&q.to_lowercase()))
        {
            continue;
        }
        rows.push(r);
    }
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    let cursor = args.cursor.take();
    let mut hash = Sha256::new();
    hash.update(
        serde_json::to_vec(&(
            scope.chat.as_str(),
            &args,
            rows.iter().map(|r| (&r.id, r.version)).collect::<Vec<_>>(),
        ))
        .map_err(storage)?,
    );
    let digest = format!("{:x}", hash.finalize());
    let start = match cursor {
        None => 0,
        Some(c) => {
            let (h, n) = c.split_once(':').ok_or_else(rejected)?;
            if h != digest {
                return Err(rejected());
            }
            let n: usize = n.parse().map_err(|_| rejected())?;
            if n == 0 || n >= rows.len() {
                return Err(rejected());
            }
            n
        }
    };
    let mut end = (start + args.limit).min(rows.len());
    loop {
        let next = (end < rows.len()).then(|| format!("{digest}:{end}"));
        let out = json!({"actions":rows[start..end],"cursor":next});
        if context_budget::result_fits(&out) {
            return Ok(out);
        }
        if end <= start + 1 {
            return Err(context_budget::result_failure());
        }
        end -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scope_pagination_and_staleness() {
        let (_root, mut request, _) = super::super::tests::fixture();
        let mut store = Store::open(&request.data_dir).unwrap();
        let op = store.allocate_operation("action-query-fixture").unwrap();
        let ids = (0..3).map(|i| op.creation_id(i)).collect::<Vec<_>>();
        let other_chat = op.creation_id(3);
        let mut writes: Vec<Put> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| Put {
                id: id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Action(Action {
                    chat: None,
                    kept_outside_project: false,
                    description: format!("Task {i}"),
                    state: ActionState::Open,
                    internal: false,
                    evidence: None,
                    due: None,
                    owner: Some("Alex".into()),
                    waiting_on: None,
                    follow_up: None,
                }),
            })
            .collect();
        writes.push(Put {
            id: other_chat.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Thread(Thread {
                private_context: false,
                context_epoch: 0,
                title: "Other chat".into(),
                state: ThreadState::Open,
                attention: vec![],
            }),
        });
        let owner = HostAuthority::owner("fixture");
        store
            .prepare_owner(
                &op,
                &ChangeRequest {
                    reason: "fixture".into(),
                    writes,
                    inputs: vec![],
                },
                &owner,
            )
            .unwrap();
        store.apply(&op, &owner).unwrap();
        drop(store);
        let (scope, _) = begin(&request).unwrap();
        let args: Args = serde_json::from_value(json!({"state":"Open","owner":"Alex","due":null,"follow_up":null,"query":"task","limit":1,"cursor":null})).unwrap();
        let first = query(&scope, args.clone()).unwrap();
        assert_eq!(first["actions"].as_array().unwrap().len(), 1);
        let mut next = args.clone();
        next.cursor = first["cursor"].as_str().map(str::to_owned);
        let second = query(&scope, next.clone()).unwrap();
        assert_ne!(first["actions"][0]["id"], second["actions"][0]["id"]);
        request.thread = other_chat;
        let (other_scope, _) = begin(&request).unwrap();
        // The same unscoped Actions are visible in both chats, so rejection
        // checks the cursor's chat binding independently of its result set.
        assert_eq!(
            query(&other_scope, args.clone()).unwrap()["actions"][0]["id"],
            first["actions"][0]["id"]
        );
        assert!(query(&other_scope, next.clone()).is_err());
        next.owner = Some("other".into());
        assert!(query(&scope, next).is_err());
        let exact = read_records(
            &scope,
            ReadArgs {
                ids: vec![first["actions"][0]["id"].as_str().unwrap().into()],
                query: None,
            },
        )
        .unwrap();
        assert_eq!(
            exact["records"][0]["version"],
            first["actions"][0]["version"]
        );
        let repeated = read_records(
            &scope,
            ReadArgs {
                ids: vec![first["actions"][0]["id"].as_str().unwrap().into()],
                query: None,
            },
        )
        .unwrap();
        assert_eq!(exact, repeated);
        assert!(query(&research::test_child_scope(&scope), args.clone()).is_err());
        let mut changed = args.clone();
        changed.state = Some(ActionState::Done);
        assert!(
            query(&scope, changed).unwrap()["actions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let mut cursor = args.clone();
        cursor.cursor = first["cursor"].as_str().map(str::to_owned);
        let mut store = Store::open(&request.data_dir).unwrap();
        let id = first["actions"][0]["id"].as_str().unwrap();
        let mut record = store.record(id).unwrap().unwrap();
        if let RecordData::Action(ref mut action) = record.data {
            action.description.push_str(" updated");
        }
        let update = store.allocate_operation("update-action").unwrap();
        store
            .prepare_owner(
                &update,
                &ChangeRequest {
                    reason: "update".into(),
                    writes: vec![Put {
                        id: record.id,
                        expected_version: Some(record.version),
                        archived: false,
                        data: record.data,
                    }],
                    inputs: vec![],
                },
                &owner,
            )
            .unwrap();
        store.apply(&update, &owner).unwrap();
        drop(store);
        assert!(query(&scope, cursor).is_err());
    }
    #[test]
    fn combined_due_and_follow_up_filters_use_both_dates() {
        let (_root, request, _) = super::super::tests::fixture();
        let mut store = Store::open(&request.data_dir).unwrap();
        let op = store.allocate_operation("date-filters").unwrap();
        let dates = [
            (Some("2026-03-01"), Some("2026-02-01")),
            (Some("2026-03-01"), Some("2026-04-01")),
            (Some("tomorrow"), Some("2026-02-01")),
            (None, Some("2026-02-01")),
            (Some("2026-03-01"), None),
        ];
        let ids = (0..dates.len())
            .map(|i| op.creation_id(i))
            .collect::<Vec<_>>();
        let writes = dates
            .into_iter()
            .enumerate()
            .map(|(i, (due, follow_up))| Put {
                id: ids[i].clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Action(Action {
                    chat: None,
                    kept_outside_project: false,
                    description: format!("dated {i}"),
                    state: ActionState::Open,
                    internal: false,
                    evidence: None,
                    due: due.map(str::to_owned),
                    owner: None,
                    waiting_on: None,
                    follow_up: follow_up.map(str::to_owned),
                }),
            })
            .collect();
        let owner = HostAuthority::owner("fixture");
        store
            .prepare_owner(
                &op,
                &ChangeRequest {
                    reason: "dates".into(),
                    writes,
                    inputs: vec![],
                },
                &owner,
            )
            .unwrap();
        store.apply(&op, &owner).unwrap();
        drop(store);
        let (scope, _) = begin(&request).unwrap();
        let mut args: Args = serde_json::from_value(json!({"state":null,"owner":null,"due":{"from":"2026-03-01","through":"2026-03-01","missing":false},"follow_up":{"from":"2026-01-01","through":"2026-02-28","missing":false},"query":null,"limit":100,"cursor":null})).unwrap();
        let result = query(&scope, args.clone()).unwrap();
        assert_eq!(result["actions"].as_array().unwrap().len(), 1);
        assert_eq!(result["actions"][0]["id"], ids[0]);
        args.due = Some(DateFilter {
            from: None,
            through: None,
            missing: true,
        });
        let missing_due = query(&scope, args).unwrap();
        assert_eq!(missing_due["actions"].as_array().unwrap().len(), 1);
        assert_eq!(missing_due["actions"][0]["id"], ids[3]);
    }
    #[test]
    fn encoded_pages_shrink_and_oversized_single_action_fails() {
        let (_root, request, _) = super::super::tests::fixture();
        let mut store = Store::open(&request.data_dir).unwrap();
        let op = store.allocate_operation("large-actions").unwrap();
        let writes = (0..3)
            .map(|i| Put {
                id: op.creation_id(i),
                expected_version: None,
                archived: false,
                data: RecordData::Action(Action {
                    chat: None,
                    kept_outside_project: false,
                    description: "x".repeat(if i == 2 { 150_000 } else { 80_000 }),
                    state: ActionState::Open,
                    internal: false,
                    evidence: None,
                    due: None,
                    owner: None,
                    waiting_on: None,
                    follow_up: None,
                }),
            })
            .collect();
        let owner = HostAuthority::owner("fixture");
        store
            .prepare_owner(
                &op,
                &ChangeRequest {
                    reason: "large".into(),
                    writes,
                    inputs: vec![],
                },
                &owner,
            )
            .unwrap();
        store.apply(&op, &owner).unwrap();
        drop(store);
        let (scope, _) = begin(&request).unwrap();
        let mut args: Args = serde_json::from_value(json!({"state":null,"owner":null,"due":null,"follow_up":null,"query":null,"limit":100,"cursor":null})).unwrap();
        let first = query(&scope, args.clone()).unwrap();
        assert_eq!(first["actions"].as_array().unwrap().len(), 1);
        args.cursor = first["cursor"].as_str().map(str::to_owned);
        let second = query(&scope, args.clone()).unwrap();
        assert_eq!(second["actions"].as_array().unwrap().len(), 1);
        args.cursor = second["cursor"].as_str().map(str::to_owned);
        assert!(query(&scope, args).is_err());
    }
    #[test]
    fn writing_without_setup_rejects_action_query() {
        let (_root, request, snapshot) = super::super::writing_tests::rewrite_fixture(false, 0);
        let (mut scope, _) = begin(&request).unwrap();
        Arc::get_mut(&mut scope).unwrap().writing_snapshot = Some(snapshot);
        let args: Args = serde_json::from_value(json!({"state":null,"owner":null,"due":null,"follow_up":null,"query":null,"limit":10,"cursor":null})).unwrap();
        assert!(query(&scope, args).is_err());
    }
    #[test]
    fn project_private_and_archived_actions_do_not_leak() {
        use super::super::project_tests::project_command;
        let (_root, request, _) = super::super::tests::fixture();
        let mut store = Store::open(&request.data_dir).unwrap();
        let p1 = project_command(
            &mut store,
            ProjectCommand::Create {
                title: "one".into(),
            },
        )
        .writes[0]
            .after
            .id
            .clone();
        project_command(
            &mut store,
            ProjectCommand::MoveChat {
                chat: request.thread.clone(),
                project: Some(p1),
            },
        );
        let p2 = project_command(
            &mut store,
            ProjectCommand::Create {
                title: "two".into(),
            },
        )
        .writes[0]
            .after
            .id
            .clone();
        let other = project_command(
            &mut store,
            ProjectCommand::CreateChat {
                project: p2,
                title: "private".into(),
            },
        )
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Thread(_)))
        .unwrap()
        .after
        .id;
        let op = store.allocate_operation("scoped-actions").unwrap();
        let ids = (0..3).map(|i| op.creation_id(i)).collect::<Vec<_>>();
        let writes = ids
            .iter()
            .enumerate()
            .map(|(i, id)| Put {
                id: id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Action(Action {
                    chat: Some(if i == 1 {
                        other.clone()
                    } else {
                        request.thread.clone()
                    }),
                    kept_outside_project: false,
                    description: format!("Scoped {i}"),
                    state: ActionState::Open,
                    internal: false,
                    evidence: None,
                    due: None,
                    owner: None,
                    waiting_on: None,
                    follow_up: None,
                }),
            })
            .collect();
        let owner = HostAuthority::owner("fixture");
        store
            .prepare_owner(
                &op,
                &ChangeRequest {
                    reason: "scoped".into(),
                    writes,
                    inputs: vec![],
                },
                &owner,
            )
            .unwrap();
        store.apply(&op, &owner).unwrap();
        let record = store.record(&ids[2]).unwrap().unwrap();
        let archive = store.allocate_operation("archive-action").unwrap();
        store
            .prepare_owner(
                &archive,
                &ChangeRequest {
                    reason: "archive".into(),
                    writes: vec![Put {
                        id: record.id,
                        expected_version: Some(record.version),
                        archived: true,
                        data: record.data,
                    }],
                    inputs: vec![],
                },
                &owner,
            )
            .unwrap();
        store.apply(&archive, &owner).unwrap();
        drop(store);
        let (scope, _) = begin(&request).unwrap();
        let args: Args = serde_json::from_value(json!({"state":null,"owner":null,"due":null,"follow_up":null,"query":null,"limit":1,"cursor":null})).unwrap();
        let result = query(&scope, args).unwrap();
        assert_eq!(result["actions"].as_array().unwrap().len(), 1);
        assert_eq!(result["actions"][0]["id"], ids[0]);
        assert!(result["cursor"].is_null());
        assert!(result.get("total").is_none());
        assert!(!result.to_string().contains(&ids[1]));
        assert!(!result.to_string().contains(&ids[2]));
    }
    #[test]
    fn writing_with_setup_cannot_read_actions() {
        let (_root, request, snapshot, _) =
            super::super::cw2_tests::fixture("requirements", WritingReadRequirements::default());
        let mut store = Store::open(&request.data_dir).unwrap();
        let op = store.allocate_operation("writing-action").unwrap();
        let owner = HostAuthority::owner("fixture");
        store
            .prepare_owner(
                &op,
                &ChangeRequest {
                    reason: "action".into(),
                    writes: vec![Put {
                        id: op.creation_id(0),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Action(Action {
                            chat: None,
                            kept_outside_project: false,
                            description: "Secret action".into(),
                            state: ActionState::Open,
                            internal: false,
                            evidence: None,
                            due: None,
                            owner: None,
                            waiting_on: None,
                            follow_up: None,
                        }),
                    }],
                    inputs: vec![],
                },
                &owner,
            )
            .unwrap();
        store.apply(&op, &owner).unwrap();
        drop(store);
        let (mut scope, _) = begin(&request).unwrap();
        Arc::get_mut(&mut scope).unwrap().writing_snapshot = Some(snapshot);
        let args: Args = serde_json::from_value(json!({"state":null,"owner":null,"due":null,"follow_up":null,"query":null,"limit":10,"cursor":null})).unwrap();
        let result = query(&scope, args).unwrap();
        assert!(result["actions"].as_array().unwrap().is_empty());
        assert!(result["cursor"].is_null());
    }
    #[test]
    fn dates_and_missing_are_explicit() {
        assert!(date("2024-02-29"));
        assert!(date("0000-01-01"));
        assert!(!date("2025-02-29"));
        let range = Some(DateFilter {
            from: Some("2024-01-01".into()),
            through: Some("2024-12-31".into()),
            missing: false,
        });
        assert!(matches_date(&Some("2024-06-01".into()), &range));
        assert!(!matches_date(&Some("tomorrow".into()), &range));
        assert!(!matches_date(&None, &range));
        let missing = Some(DateFilter {
            from: None,
            through: None,
            missing: true,
        });
        assert!(matches_date(&None, &missing));
        assert!(!matches_date(&Some("tomorrow".into()), &missing));
    }
    #[test]
    fn validation_requires_fields_and_bounds() {
        let mut v = json!({"state":null,"owner":null,"due":null,"follow_up":null,"query":null,"limit":1,"cursor":null});
        assert!(validate(&v));
        v["limit"] = json!(101);
        assert!(!validate(&v));
        v["limit"] = json!(1);
        v["due"] = json!({"from":"2024-02-30","through":null,"missing":false});
        assert!(!validate(&v));
    }
}
