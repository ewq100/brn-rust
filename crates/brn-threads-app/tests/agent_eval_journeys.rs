//! Scripted policy journeys for the evaluation cases (evaluation design §6.2).
//! Each case is seeded from the v2 fixture with 1,000 linked fillers. A
//! scripted model issues fixed queries taken from the prompt's wording,
//! never from the target's title, and drives the real runtime tools over the
//! real search worker. They fail when retrieval or tools regress and say
//! nothing about model behaviour. No provider is contacted.

#[path = "../examples/wiki_eval_seed/seed.rs"]
#[allow(dead_code)]
mod seed;

use brn_ai::threads::{ThreadRunRequest, TraceLevel, prepare_run, run_prepared_scripted};
use brn_ai::{AiTerminal, Provider, ReasoningEffort, Selection};
use brn_threads_app::search::SearchHandle;
use brn_threads_app::*;
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;

const FIXTURE: &str = include_str!("../../../scripts/fixtures/wiki-eval-cases-v2.json");
const FILLER: usize = 1000;
/// The first page an agent sees with `limit` 10.
const PAGE: usize = 10;
const ROUNDS: u16 = 3;

struct Case {
    _root: tempfile::TempDir,
    data: PathBuf,
    handle: SearchHandle,
    report: seed::Report,
}

impl Case {
    fn id(&self, key: &str) -> &str {
        self.report
            .notes
            .get(key)
            .or_else(|| self.report.threads.get(key))
            .unwrap_or_else(|| panic!("unknown seed key {key}"))
    }
}

fn seeded(case: &str) -> Case {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let report = seed::seed_fixture(&data, FIXTURE, case, FILLER, 1).unwrap();
    let mut workspace = Workspace::open(&data).unwrap();
    let settings = workspace.settings().unwrap();
    workspace
        .configure(WorkspaceSettings {
            provider: "Codex".into(),
            model: "gpt-6.1-sol".into(),
            effort: "medium".into(),
            auth_file: None,
            credentials_dir: None,
            ..settings
        })
        .unwrap();
    workspace
        .choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
        .unwrap();
    let handle = workspace.search_handle().unwrap();
    handle.prepare().wait().unwrap();
    Case {
        _root: root,
        data,
        handle,
        report,
    }
}

fn call(id: &str, name: &str, args: Value) -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::tool_call(id, name, args),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]
}

fn finish() -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::text("Answered from the notes read."),
        MockStreamEvent::final_response_with_total_tokens(1),
    ]
}

fn search(query: &str) -> Value {
    json!({"query":query,"limit":PAGE,"mode":"hybrid","cursor":null})
}

/// What one scripted journey must show.
struct Journey<'a> {
    case: &'a str,
    chat: &'a str,
    query: &'a str,
    /// Found on the first page of `query`.
    found: &'a str,
    /// Reached through `note_connections` from `found`, when different.
    via_connections: Option<&'a str>,
    /// Must never appear in any result.
    private: &'a [&'a str],
    /// Note content the trace must not contain.
    content: &'a [&'a str],
}

/// Runs the journey: search, optionally Connections from the hit, read the
/// target, answer. Returns the case for further checks.
async fn run(j: Journey<'_>) -> Case {
    let case = seeded(j.case);
    let chat = case.id(j.chat).to_owned();
    let found = case.id(j.found).to_owned();
    let page = case.handle.search(j.query, Some(&chat), PAGE).unwrap();
    let hit = page
        .matches
        .iter()
        .find(|m| m.note == found)
        .unwrap_or_else(|| {
            let titles: Vec<&str> = page.matches.iter().map(|m| m.title.as_str()).collect();
            panic!("{}: {} not on the first page: {titles:?}", j.case, j.found)
        });
    let mut turns = vec![call("s", "search_notes", search(j.query))];
    let (target, version, start) = match j.via_connections {
        None => (found.clone(), hit.version, hit.start),
        Some(key) => {
            let target = case.id(key).to_owned();
            let near = case
                .handle
                .connections(&found, Some(&chat), 1, false)
                .unwrap();
            let neighbour = near
                .hops
                .iter()
                .flat_map(|h| &h.shown)
                .find(|n| n.id == target)
                .unwrap_or_else(|| panic!("{}: {key} not within one link", j.case));
            turns.push(call(
                "c",
                "note_connections",
                json!({"id":found,"hops":1,"sources":false}),
            ));
            (target, neighbour.version, 0)
        }
    };
    turns.push(call(
        "r",
        "read_note",
        json!({"id":target,"version":version,"start":start,"max_bytes":2048,"section":false}),
    ));
    turns.push(finish());
    let model = MockCompletionModel::from_stream_turns(turns);
    let prepared = prepare_run(ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: case.data.clone(),
        thread: chat.clone(),
        prompt: j.query.into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: ReasoningEffort::Medium,
        max_tool_rounds: ROUNDS,
        auth_file: None,
        credentials_dir: None,
        sources: vec![],
        search: Some(Arc::new(case.handle.clone())),
        trace: TraceLevel::Full,
    })
    .unwrap();
    let result = run_prepared_scripted(prepared, model).await.unwrap();
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{}: {:?}",
        j.case,
        result.answer
    );
    let trace = result.trace.expect("trace requested");
    assert!(
        trace.rounds <= ROUNDS,
        "{}: {} rounds",
        j.case,
        trace.rounds
    );
    assert!(
        trace.calls.iter().all(|c| c.outcome == "success"),
        "{}: {:?}",
        j.case,
        trace.calls
    );
    let read = trace.calls.last().unwrap();
    assert_eq!(
        (read.tool.as_str(), read.args["id"].as_str()),
        ("read_note", Some(target.as_str()))
    );
    assert!(read.bytes > 0);
    let text = serde_json::to_string(&trace).unwrap();
    let listed: Vec<&str> = trace
        .calls
        .iter()
        .flat_map(|c| c.result["ids"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect();
    assert!(listed.contains(&found.as_str()), "{}: {text}", j.case);
    for key in j.private {
        assert!(!listed.contains(&case.id(key)), "{}: {key} listed", j.case);
    }
    for content in j.content {
        assert!(
            !text.contains(content),
            "{}: trace leaked {content:?}",
            j.case
        );
    }
    case
}

#[tokio::test(flavor = "multi_thread")]
async fn r1_described_subject_is_on_the_first_page() {
    run(Journey {
        case: "R1",
        chat: "case",
        query: "cold-store refit north quay insulation firm",
        found: "target",
        via_connections: None,
        private: &[],
        content: &["Viru Termo", "3 March 2027"],
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn r2_alias_resolves_through_the_linked_roster() {
    run(Journey {
        case: "R2",
        chat: "case",
        query: "KT Friday night shift availability",
        found: "roster",
        via_connections: Some("target"),
        private: &[],
        content: &["Kadri Tamm", "15 January 2027"],
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn r3_current_gate_rule_is_on_the_first_page() {
    run(Journey {
        case: "R3",
        chat: "case",
        query: "hazardous cargo gate 3 driver",
        found: "gate-3-hazardous-cargo-2026",
        via_connections: None,
        private: &[],
        content: &["booking reference"],
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn r4_scattered_evidence_is_one_link_from_the_topic_page() {
    let case = run(Journey {
        case: "R4",
        chat: "case",
        query: "spring concert seats timing",
        found: "topic",
        via_connections: Some("tally"),
        private: &[],
        content: &["212 tickets"],
    })
    .await;
    let chat = case.id("case").to_owned();
    let near = case
        .handle
        .connections(case.id("topic"), Some(&chat), 1, false)
        .unwrap();
    let shown: Vec<&str> = near
        .hops
        .iter()
        .flat_map(|h| &h.shown)
        .map(|n| n.id.as_str())
        .collect();
    for key in ["hall", "programme", "tally"] {
        assert!(shown.contains(&case.id(key)), "{key} not connected");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn r5_current_tariff_version_is_found() {
    let case = run(Journey {
        case: "R5",
        chat: "case",
        query: "berth fee vessel November",
        found: "berth-fees",
        via_connections: None,
        private: &[],
        content: &["2.10 EUR"],
    })
    .await;
    let chat = case.id("case").to_owned();
    let page = case
        .handle
        .search("berth fee vessel November", Some(&chat), PAGE)
        .unwrap();
    let hit = page
        .matches
        .iter()
        .find(|m| m.note == case.id("berth-fees"))
        .unwrap();
    assert_eq!(hit.version, 2, "search offers the current version");
}

#[tokio::test(flavor = "multi_thread")]
async fn r7_private_offer_stays_in_its_chat() {
    let case = run(Journey {
        case: "R7",
        chat: "case",
        query: "coordinator salary floor",
        found: "budget",
        via_connections: None,
        private: &["offer"],
        content: &["BLUEFIN", "4,200"],
    })
    .await;
    let (general, private) = (case.id("case"), case.id("private"));
    for query in ["coordinator offer floor", "BLUEFIN"] {
        let page = case.handle.search(query, Some(general), PAGE).unwrap();
        assert!(page.matches.iter().all(|m| m.note != case.id("offer")));
    }
    assert!(
        case.handle
            .connections(case.id("offer"), Some(general), 1, true)
            .is_err()
    );
    let own = case.handle.search("BLUEFIN", Some(private), PAGE).unwrap();
    assert_eq!(own.matches[0].note, case.id("offer"));
}

#[tokio::test(flavor = "multi_thread")]
async fn r8_deep_section_is_read_at_its_offset() {
    let case = run(Journey {
        case: "R8",
        chat: "case",
        query: "reefer plugs checked alarm",
        found: "handbook",
        via_connections: None,
        private: &[],
        content: &["form R-7", "every 4 hours"],
    })
    .await;
    let range = &case.report.ranges["handbook"];
    assert!(range.start > 128 * 1024);
    let chat = case.id("case").to_owned();
    let hit = case
        .handle
        .search("reefer plugs checked alarm", Some(&chat), PAGE)
        .unwrap()
        .matches
        .remove(0);
    assert!(
        hit.start < range.end && hit.end > range.start,
        "hit {}..{} outside the planted section {}..{}",
        hit.start,
        hit.end,
        range.start,
        range.end
    );
}

/// S2: the seeded knowledge-check review names the Link records to rename
/// and re-point, and `note_connections` exposes Link IDs, so one checked
/// change group built only from that output resolves the mechanical
/// findings without a BrokenLink.
#[tokio::test(flavor = "multi_thread")]
async fn s2_review_names_the_links_one_change_group_resolves() {
    let case = seeded("S2");
    let mut workspace = Workspace::open(&case.data).unwrap();
    let before = workspace.knowledge_check().unwrap();
    let chat = workspace.seed_knowledge_review(&before).unwrap();
    let RecordData::Message(message) = &workspace.messages(&chat).unwrap()[0].data else {
        panic!("review message")
    };
    let finding = |kind| {
        before
            .findings
            .iter()
            .find(|f| f.kind == kind)
            .unwrap_or_else(|| panic!("{kind:?} missing"))
            .clone()
    };
    let link = |id: &str| workspace.store.record(id).unwrap().unwrap();
    let variant = finding(FindingKind::RelationVariant);
    let [rename] = variant.records.as_slice() else {
        panic!("{variant:?}")
    };
    let rename = link(rename);
    let RecordData::Link(renamed) = &rename.data else {
        panic!()
    };
    assert_eq!(renamed.relation, "relates-to");
    let supersession = finding(FindingKind::Supersession);
    let [old, new, inbound] = supersession.records.as_slice() else {
        panic!("{supersession:?}")
    };
    assert_eq!(
        (old.as_str(), new.as_str()),
        (case.id("fees-2025"), case.id("fees-2026"))
    );
    assert!(
        supersession
            .detail
            .contains("Already linking the replacement: “Bicycle coop”")
    );
    let inbound = link(inbound);
    for text in [
        rename.id.as_str(),
        inbound.id.as_str(),
        "RelationVariant: update each listed Link in place",
        "re-point or archive the listed links",
    ] {
        assert!(
            message.text.contains(text),
            "{text:?} not in {}",
            message.text
        );
    }

    let bicycle = case.id("bicycle").to_owned();
    let topics = workspace.settings().unwrap().topics_note.unwrap();
    let fees = link(old);
    let RecordData::Note(mut retired) = fees.data.clone() else {
        panic!()
    };
    retired.superseded_by = Some(new.clone());
    let link_data = |from: &str, to: &str, relation: &str| json!({"Link":{"from":from,"to":to,"relation":relation}});
    let RecordData::Link(pointed) = &inbound.data else {
        panic!()
    };
    let request = json!({
        "reason":"Resolve the mechanical knowledge-check findings",
        "writes":[
            {"id":rename.id,"expected_version":rename.version,"archived":false,
                "data":link_data(&renamed.from, &renamed.to, "related")},
            {"id":fees.id,"expected_version":fees.version,"archived":false,
                "data":{"Note":retired}},
            {"id":inbound.id,"expected_version":inbound.version,"archived":true,
                "data":link_data(&pointed.from, &pointed.to, &pointed.relation)},
            {"id":"new:0","expected_version":null,"archived":false,
                "data":link_data(&topics, &bicycle, "related")},
            {"id":"new:1","expected_version":null,"archived":false,
                "data":link_data(&bicycle, case.id("rota"), "related")}
        ],
        "inputs":[{"record":fees.id,"version":fees.version}]
    });
    let prepared = prepare_run(ThreadRunRequest {
        request_message: None,
        queue_claim: None,
        owner_request: None,
        data_dir: case.data.clone(),
        thread: chat.clone(),
        prompt: "Please work through these knowledge-check findings within your budget.".into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        },
        effort: ReasoningEffort::Medium,
        max_tool_rounds: 8,
        auth_file: None,
        credentials_dir: None,
        sources: vec![],
        search: Some(Arc::new(case.handle.clone())),
        trace: TraceLevel::Full,
    })
    .unwrap();
    let op = Store::open(&case.data)
        .unwrap()
        .allocate_operation(&format!("{}:proposal:review", prepared.run_id()))
        .unwrap()
        .as_str()
        .to_owned();
    let model = MockCompletionModel::from_stream_turns([
        call(
            "c",
            "note_connections",
            json!({"id":bicycle,"hops":1,"sources":false}),
        ),
        call("r", "read_records", json!({"ids":[fees.id],"query":null})),
        call(
            "p",
            "prepare_changes",
            json!({"request_key":"review","request":request}),
        ),
        call("a", "apply_changes", json!({"operation":op})),
        finish(),
    ]);
    let result = run_prepared_scripted(prepared, model.clone())
        .await
        .unwrap();
    assert!(
        matches!(result.answer.terminal, AiTerminal::Completed),
        "{:?}",
        result.answer
    );
    let seen: Vec<String> = model
        .requests()
        .iter()
        .map(|r| format!("{:?}", r.chat_history.last()))
        .collect();
    assert!(
        seen[1].contains(&inbound.id),
        "connections lack Link IDs: {}",
        seen[1]
    );
    assert!(seen[4].contains("Applied"), "{}", seen[4]);

    let after = workspace.knowledge_check().unwrap();
    let count = |kind| after.counts.get(&kind).copied().unwrap_or(0);
    for kind in [
        FindingKind::RelationVariant,
        FindingKind::Supersession,
        FindingKind::UnlistedTopic,
        FindingKind::Orphan,
        FindingKind::BrokenLink,
    ] {
        assert_eq!(count(kind), 0, "{kind:?}: {:?}", after.findings);
    }
    assert_eq!(count(FindingKind::DuplicateTitle), 1);
}
