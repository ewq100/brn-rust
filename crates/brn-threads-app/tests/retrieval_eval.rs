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
        connected: None,
    }
}

#[test]
fn golden_fixture_satisfies_structural_and_lexical_rules() {
    let golden = golden().unwrap();
    assert_eq!(golden.cases.len(), 163);
    assert_eq!(golden.long_notes.len(), 4);
    assert_eq!(golden.deep_notes.len(), 2);
    assert_eq!(golden.topics.len(), 10);
    for stratum in Stratum::ALL {
        let count = golden.cases.iter().filter(|c| c.stratum == stratum).count();
        assert_eq!(count, stratum.expected_cases(), "{}", stratum.name());
    }
}

/// The first two long words of `query`, as a two-word title.
fn content_words(query: &str) -> String {
    query
        .split(' ')
        .filter(|w| w.len() >= 5)
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn validation_refuses_the_new_strata_rule_violations() {
    let edit = |stratum: Stratum, change: &dyn Fn(&mut Case)| {
        let mut golden = golden().unwrap();
        let case = golden
            .cases
            .iter_mut()
            .find(|c| c.stratum == stratum)
            .unwrap();
        change(case);
        validate(&golden).is_err()
    };
    // A description that repeats two words of the note's title.
    let mut golden = golden().unwrap();
    let case = golden
        .cases
        .iter_mut()
        .find(|c| c.stratum == Stratum::Described)
        .unwrap();
    let renamed = content_words(&case.query);
    case.plant.as_mut().unwrap().note = renamed.clone();
    case.relevant = vec![renamed];
    let error = validate(&golden).unwrap_err().to_string();
    assert!(error.contains("words with the title"), "{error}");
    // A family without enough siblings.
    assert!(edit(Stratum::NearDuplicate, &|c| {
        let family = c.family.as_mut().unwrap();
        family.axes = family.axes.iter().map(|a| a[..1].to_vec()).collect();
        family.target = vec![0; family.axes.len()];
    }));
    // A supersession link in the wrong direction.
    assert!(edit(Stratum::Supersession, &|c| {
        let link = &mut c.links[0];
        std::mem::swap(&mut link.from, &mut link.to);
    }));
    // A scattered case with a single relevant note.
    assert!(edit(Stratum::Scattered, &|c| {
        c.relevant.truncate(1);
        c.more.clear();
        c.links.clear();
    }));
    // A sentence-paraphrase that no longer repeats a paraphrase query.
    assert!(edit(Stratum::SentenceParaphrase, &|c| {
        c.query = format!("{} again", c.query);
    }));
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
            if case.stratum == Stratum::DeepSection {
                let at = holder.markdown.find(&plant.text).unwrap();
                assert!(at > 128 * 1024, "{} starts at byte {at}", case.id);
            }
        }
    }
    assert!(corpus(&golden, 10, DEFAULT_SEED).is_err());
}

#[test]
fn new_strata_notes_have_the_structure_their_metrics_need() {
    let golden = golden().unwrap();
    let notes = corpus(&golden, 400, DEFAULT_SEED).unwrap();
    let by_title = |title: &str| notes.iter().find(|n| n.title == title).unwrap();
    let mut titles = std::collections::BTreeSet::new();
    assert!(notes.iter().all(|n| titles.insert(n.title.as_str())));
    for case in golden
        .cases
        .iter()
        .filter(|c| c.stratum == Stratum::NearDuplicate)
    {
        let family = case.family.as_ref().unwrap();
        let target = &case.plant.as_ref().unwrap().note;
        // The target and at least ten siblings share a title stem.
        let stem = family.title.split('{').next().unwrap().trim();
        let siblings = notes
            .iter()
            .filter(|n| n.title.starts_with(stem) && &n.title != target)
            .count();
        assert!(siblings >= 10, "{}: {siblings} siblings", case.id);
    }
    for case in golden
        .cases
        .iter()
        .filter(|c| c.stratum == Stratum::Supersession)
    {
        let new = by_title(&case.relevant[0]);
        assert_eq!(new.supersedes, vec![case.relevant[1].clone()]);
        assert!(new.links.is_empty());
    }
    for case in golden
        .cases
        .iter()
        .filter(|c| c.stratum == Stratum::Scattered)
    {
        let linked: usize = case.relevant.iter().map(|t| by_title(t).links.len()).sum();
        assert_eq!(linked, case.links.len(), "{}", case.id);
    }
    // Enlarged deep notes exceed the 128 KiB indexing window.
    for deep in &golden.deep_notes {
        assert!(by_title(&deep.title).markdown.len() > 128 * 1024);
    }
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
    assert_eq!(
        (m.complete_at_10, m.connected_at_1, m.new_above_old),
        (None, None, None)
    );
    assert_eq!(metrics(&[]), Metrics::default());
    assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0, 5.0], 0.5), 3.0);
}

#[test]
fn scattered_and_supersession_metrics_follow_their_definitions() {
    let scattered = |ranks: Vec<Option<usize>>, connected| CaseResult {
        connected: Some(connected),
        ..result(Stratum::Scattered, ranks, 50)
    };
    let rows = [
        scattered(vec![Some(1), Some(10)], true),
        scattered(vec![Some(2), Some(11)], false),
        scattered(vec![Some(3), None, Some(4)], false),
        scattered(vec![Some(1), Some(2), Some(3)], true),
    ];
    let m = metrics(&rows.iter().collect::<Vec<_>>());
    assert_eq!(m.complete_at_10, Some(0.5));
    assert_eq!(m.connected_at_1, Some(0.5));
    assert_eq!(m.new_above_old, None);

    let rows = [
        result(Stratum::Supersession, vec![Some(1), Some(2)], 5),
        result(Stratum::Supersession, vec![Some(3), Some(2)], 5),
        result(Stratum::Supersession, vec![Some(4), None], 5),
        result(Stratum::Supersession, vec![None, Some(1)], 5),
    ];
    let m = metrics(&rows.iter().collect::<Vec<_>>());
    assert_eq!(m.new_above_old, Some(0.5));
    assert_eq!(m.complete_at_10, None);
}

#[test]
fn small_evaluation_runs_end_to_end_with_sane_baseline() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let report = run(&root.path().join("data"), 400, DEFAULT_SEED).unwrap();
    assert_eq!(report.notes, 400);
    assert_eq!(report.cases.len(), 163);
    assert!(report.links > 0);
    // Chat-only notes never leak, and today's search cannot find them in their own chat.
    assert_eq!(
        report.visibility,
        Visibility {
            chat_notes: 12,
            queries: 163 + 12 * 3,
            violations: 0,
            in_chat_recall_at_10: 0.0,
        }
    );
    // Strata today's keyword search already solves must not regress.
    assert_eq!(report.strata["exact"].recall_at_10, 1.0);
    assert_eq!(report.strata["topic"].recall_at_10, 1.0);
    assert!(report.strata["section"].recall_at_10 >= 0.9);
    assert_eq!(report.strata["abstention"].any_result_rate, 0.0);
    // The new strata are reported; the original-strata overall excludes them.
    assert_eq!(report.strata.len(), Stratum::ALL.len());
    assert_eq!(report.overall.cases, 100);
    assert_eq!(report.overall_all.cases, 163);
    assert!(report.strata["scattered"].complete_at_10.is_some());
    assert!(report.strata["scattered"].connected_at_1.is_some());
    assert!(report.strata["supersession"].new_above_old.is_some());
    for metrics in report
        .strata
        .values()
        .chain([&report.overall, &report.overall_all])
    {
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

/// Places each judged query's relevant notes from rank 3 on, behind two
/// decoys, so the harness's title-to-ID mapping and metrics are checked end to
/// end. Sentence-paraphrase cases share their query with a paraphrase case; the
/// paraphrase note comes first and its standalone twin right after it.
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
    let mut by_query: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for case in &golden.cases {
        let titles = by_query.entry(case.query.clone()).or_default();
        titles.extend(case.relevant.iter().cloned());
    }
    let mut oracle = Oracle(by_query);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let report = run_with(&root.path().join("data"), 400, DEFAULT_SEED, &mut oracle).unwrap();
    assert_eq!(report.retriever, "oracle");
    for (name, m) in &report.strata {
        if name == "abstention" {
            assert_eq!(m.any_result_rate, 0.0);
        } else {
            assert_eq!((m.recall_at_10, m.recall_at_50), (1.0, 1.0), "{name}");
            let first = if name == "sentence-paraphrase" {
                4.0
            } else {
                3.0
            };
            assert!((m.mrr - 1.0 / first).abs() < 1e-9, "{name}");
        }
    }
    assert_eq!(report.strata["scattered"].complete_at_10, Some(1.0));
    assert_eq!(report.strata["supersession"].new_above_old, Some(1.0));
    for case in report.cases.iter().filter(|c| !c.ranks.is_empty()) {
        let first = if case.stratum == Stratum::SentenceParaphrase {
            4
        } else {
            3
        };
        let expected: Vec<_> = (first..first + case.ranks.len()).map(Some).collect();
        assert_eq!(case.ranks, expected, "{}", case.id);
    }
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

#[test]
fn worker_index_keeps_solved_strata_finds_inflections_and_respects_chats() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let report = run_with(
        &root.path().join("data"),
        400,
        DEFAULT_SEED,
        &mut WorkerSearch::default(),
    )
    .unwrap();
    assert_eq!(report.retriever, "worker-keyword");
    assert_eq!(report.visibility.violations, 0);
    assert_eq!(report.visibility.in_chat_recall_at_10, 1.0);
    assert_eq!(report.strata["exact"].recall_at_10, 1.0);
    assert_eq!(report.strata["topic"].recall_at_10, 1.0);
    assert!(report.strata["section"].recall_at_10 >= 0.9);
    assert!(report.strata["inflection"].recall_at_50 >= 0.5);
    assert_eq!(report.strata["abstention"].any_result_rate, 0.0);
}
