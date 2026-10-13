#[path = "../examples/retrieval_eval/support.rs"]
#[allow(dead_code)]
mod support;

use brn_threads_app::{RecordData, Workspace, record_in_chat};
use std::collections::BTreeMap;
use support::*;

fn result(stratum: Stratum, ranks: Vec<Option<usize>>, returned: usize) -> CaseResult {
    CaseResult {
        id: "case".into(),
        stratum,
        ranks,
        returned,
        ms: 0.0,
    }
}

#[test]
fn golden_fixture_satisfies_structural_and_lexical_rules() {
    let golden = golden().unwrap();
    assert_eq!(golden.cases.len(), 100);
    assert_eq!(golden.long_notes.len(), 4);
    assert_eq!(golden.topics.len(), 10);
}

#[test]
fn validation_refuses_literal_matches_and_dangling_relevance() {
    let mut golden = golden().unwrap();
    let case = golden
        .cases
        .iter_mut()
        .find(|c| c.stratum == Stratum::Paraphrase)
        .unwrap();
    let planted = case.plant.as_ref().unwrap().text.clone();
    let literal = planted
        .split(|c: char| !c.is_alphabetic())
        .find(|w| w.len() >= 7)
        .unwrap()
        .to_string();
    case.query = format!("{} {literal}", case.query);
    assert!(validate(&golden).is_err());

    let mut golden = support::golden().unwrap();
    golden.cases[0].relevant = vec!["No such note".into()];
    assert!(validate(&golden).is_err());
}

#[test]
fn corpus_is_deterministic_and_seed_sensitive() {
    let golden = golden().unwrap();
    let a = corpus(&golden, 300, DEFAULT_SEED).unwrap();
    let b = corpus(&golden, 300, DEFAULT_SEED).unwrap();
    let c = corpus(&golden, 300, DEFAULT_SEED + 1).unwrap();
    assert_eq!(a.len(), 300);
    assert!(a.iter().any(|n| n.markdown.contains("\n|---|---|---|\n")));
    assert!(
        a.iter()
            .any(|n| n.markdown.contains("\n- ") && n.links.is_empty())
    );
    assert_eq!(corpus_sha256(&a), corpus_sha256(&b));
    assert_ne!(corpus_sha256(&a), corpus_sha256(&c));
    for case in golden.cases.iter().filter(|c| c.plant.is_some()) {
        let plant = case.plant.as_ref().unwrap();
        let holder = a.iter().find(|n| n.title == plant.note).unwrap();
        assert!(holder.markdown.contains(&plant.text), "{}", case.id);
        if let Some(section) = &plant.section {
            let heading = format!("\n## {section}\n");
            let start = holder.markdown.find(&heading).unwrap();
            let end = holder.markdown[start + heading.len()..]
                .find("\n## ")
                .map_or(holder.markdown.len(), |i| start + heading.len() + i);
            assert!(
                holder.markdown[start..end].contains(&plant.text),
                "{}",
                case.id
            );
        }
    }
    assert!(corpus(&golden, 10, DEFAULT_SEED).is_err());
}

#[test]
fn metrics_follow_their_definitions() {
    let rows = [
        result(Stratum::Exact, vec![Some(1)], 5),
        result(Stratum::Exact, vec![Some(20)], 30),
        result(Stratum::Exact, vec![None], 30),
        result(Stratum::Abstention, vec![], 0),
        result(Stratum::Abstention, vec![], 3),
    ];
    let m = metrics(&rows.iter().collect::<Vec<_>>());
    assert_eq!(m.cases, 5);
    assert!((m.recall_at_10 - 1.0 / 3.0).abs() < 1e-9);
    assert!((m.recall_at_50 - 2.0 / 3.0).abs() < 1e-9);
    assert!((m.mrr - (1.0 + 1.0 / 20.0) / 3.0).abs() < 1e-9);
    assert!((m.any_result_rate - 0.5).abs() < 1e-9);
    assert_eq!(metrics(&[]), Metrics::default());
    assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0, 5.0], 0.5), 3.0);
}

#[test]
fn small_evaluation_runs_end_to_end_with_sane_baseline() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let report = run(&root.path().join("data"), 400, DEFAULT_SEED).unwrap();
    assert_eq!(report.notes, 400);
    assert_eq!(report.cases.len(), 100);
    assert!(report.links > 0);
    // Chat-only notes never leak, and today's search cannot find them in their own chat.
    assert_eq!(
        report.visibility,
        Visibility {
            chat_notes: 12,
            queries: 100 + 12 * 3,
            violations: 0,
            in_chat_recall_at_10: 0.0,
        }
    );
    // Strata today's keyword search already solves must not regress.
    assert_eq!(report.strata["exact"].recall_at_10, 1.0);
    assert_eq!(report.strata["topic"].recall_at_10, 1.0);
    assert!(report.strata["section"].recall_at_10 >= 0.9);
    for metrics in report.strata.values().chain([&report.overall]) {
        for value in [
            metrics.recall_at_10,
            metrics.recall_at_50,
            metrics.mrr,
            metrics.any_result_rate,
        ] {
            assert!((0.0..=1.0).contains(&value));
        }
    }
    assert!(run(&root.path().join("data"), 400, DEFAULT_SEED).is_err());
}

#[test]
fn identical_seeds_give_identical_ranks_despite_random_record_ids() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let a = run(&root.path().join("a"), 400, DEFAULT_SEED).unwrap();
    let b = run(&root.path().join("b"), 400, DEFAULT_SEED).unwrap();
    assert_eq!(a.corpus_sha256, b.corpus_sha256);
    let ranks = |r: &Report| r.cases.iter().map(|c| c.ranks.clone()).collect::<Vec<_>>();
    assert_eq!(ranks(&a), ranks(&b));
}

/// Places each judged case's relevant note third, behind two decoys, so the
/// harness's title-to-ID mapping and metrics are checked end to end.
struct Oracle(BTreeMap<String, Vec<String>>);

impl Retriever for Oracle {
    fn name(&self) -> String {
        "oracle".into()
    }
    fn search(
        &mut self,
        workspace: &mut Workspace,
        query: &str,
        chat: Option<&str>,
    ) -> support::Result<Vec<String>> {
        let Some(titles) = self.0.get(query) else {
            return CurrentAppSearch.search(workspace, query, chat);
        };
        if titles.is_empty() {
            return Ok(vec![]);
        }
        let notes = workspace.notes()?;
        let id = |title: &str| {
            notes
                .iter()
                .find(|r| matches!(&r.data, RecordData::Note(n) if n.title == title))
                .map(|r| r.id.clone())
                .unwrap()
        };
        let mut decoys = notes
            .iter()
            .filter(|r| {
                !titles
                    .iter()
                    .any(|t| matches!(&r.data, RecordData::Note(n) if &n.title == t))
            })
            .map(|r| r.id.clone());
        let mut ranked = vec![decoys.next().unwrap(), decoys.next().unwrap()];
        ranked.extend(titles.iter().map(|t| id(t)));
        Ok(ranked)
    }
}

#[test]
fn metrics_reflect_a_known_ranking_through_the_retriever_seam() {
    let golden = golden().unwrap();
    let mut oracle = Oracle(
        golden
            .cases
            .iter()
            .map(|c| (c.query.clone(), c.relevant.clone()))
            .collect(),
    );
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let report = run_with(&root.path().join("data"), 300, DEFAULT_SEED, &mut oracle).unwrap();
    assert_eq!(report.retriever, "oracle");
    for (name, m) in &report.strata {
        if name == "abstention" {
            assert_eq!(m.any_result_rate, 0.0);
        } else {
            assert_eq!((m.recall_at_10, m.recall_at_50), (1.0, 1.0), "{name}");
            assert!((m.mrr - 1.0 / 3.0).abs() < 1e-9, "{name}");
        }
    }
    assert!(
        report
            .cases
            .iter()
            .filter(|c| !c.ranks.is_empty())
            .all(|c| c.ranks == vec![Some(3)])
    );
}

/// Literal scan over every current note; optionally ignores chat context so
/// the visibility check is shown to catch leaks.
struct Scan {
    respect_context: bool,
}

impl Retriever for Scan {
    fn name(&self) -> String {
        format!("scan-respect-context-{}", self.respect_context)
    }
    fn search(
        &mut self,
        workspace: &mut Workspace,
        query: &str,
        chat: Option<&str>,
    ) -> support::Result<Vec<String>> {
        let rows = workspace.records()?;
        Ok(rows
            .iter()
            .filter(|r| {
                !r.archived
                    && matches!(&r.data, RecordData::Note(n)
                        if n.superseded_by.is_none() && n.markdown.contains(query))
                    && (!self.respect_context || record_in_chat(r, &rows, chat))
            })
            .map(|r| r.id.clone())
            .collect())
    }
}

#[test]
fn visibility_check_counts_leaks_and_credits_in_chat_finds() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let scoped = run_with(
        &root.path().join("scoped"),
        300,
        DEFAULT_SEED,
        &mut Scan {
            respect_context: true,
        },
    )
    .unwrap();
    assert_eq!(scoped.visibility.violations, 0);
    assert_eq!(scoped.visibility.in_chat_recall_at_10, 1.0);
    let leaky = run_with(
        &root.path().join("leaky"),
        300,
        DEFAULT_SEED,
        &mut Scan {
            respect_context: false,
        },
    )
    .unwrap();
    assert_eq!(leaky.visibility.in_chat_recall_at_10, 1.0);
    // Each marker leaks into general and the other chat, and the bait text
    // leaks chat-only notes into golden exact queries.
    assert!(leaky.visibility.violations >= 12 * 2 + 12);
}
