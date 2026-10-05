//! Original supersession replay preserves reviewed work before fresh file checks.
use super::link_tests::target;
use super::*;

#[test]
fn inbox_supersession_creation_replay_preserves_history_targets_and_newer_review_without_files() {
    let f = Fixture::new();
    let input = Arc::new(Mutex::new(None::<KnowledgeProposalArgs>));
    let script = input.clone();
    let (created, ready) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let resume = Arc::new(Mutex::new(resume));
    let hook: ProposalAnswerHook = Arc::new(move |_, _, _, proposals, _, _| {
        let candidate = script.lock().unwrap().clone().unwrap();
        let created = created.clone();
        let resume = resume.clone();
        Box::pin(async move {
            let out = tokio::task::spawn_blocking(move || {
                created
                    .send(proposals.propose_knowledge(candidate.clone()).unwrap())
                    .unwrap();
                resume
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
                let mut out = vec![tool_reply(proposals.propose_knowledge(candidate.clone()))];
                for mode in 0..4 {
                    let mut changed = candidate.clone();
                    match mode {
                        0 => changed.supersedes = Some("different.md".into()),
                        1 => changed.supersedes = None,
                        2 => changed.source_paths.swap(0, 1),
                        _ => changed.text.push_str("Changed candidate input"),
                    }
                    out.push(tool_reply(proposals.propose_knowledge(changed)));
                }
                out
            })
            .await
            .unwrap();
            AiAnswer {
                text: serde_json::to_string(&out).unwrap(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = f.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let source = capture_source(&worker, "Blue õ 🦀\r\n");
    let previous = target(&worker, "previous.md", false);
    let extra_targets = [
        target(&worker, "project.md", false),
        target(&worker, "person.md", false),
    ];
    let mut candidate = knowledge(&source);
    candidate.supersedes = Some(previous.1.source.path.clone());
    candidate.source_paths = extra_targets
        .iter()
        .map(|(_, proof)| proof.source.path.clone())
        .collect();
    for (id, _) in &extra_targets {
        candidate
            .text
            .push_str(&format!("\r\n[Context](brn://note/{id})\r\n"));
    }
    *input.lock().unwrap() = Some(candidate.clone());
    let request = semantic(&source);
    worker
        .submit(
            request.id,
            AppCommand::AnalyzeInboxActions(Box::new(request.clone())),
        )
        .unwrap();
    let receipt = ready.recv_timeout(Duration::from_secs(10)).unwrap();
    let proposal_id = Uuid::parse_str(receipt["stamp"]["id"].as_str().unwrap()).unwrap();
    let AppEvent::Proposal(record) = reply(&worker, AppCommand::Proposal(proposal_id)) else {
        panic!("retained supersession review")
    };
    let original_sources = std::iter::once(source.source.source.clone())
        .chain(std::iter::once(previous.1.source.clone()))
        .chain(extra_targets.iter().map(|(_, proof)| proof.source.clone()))
        .collect::<Vec<_>>();
    assert_eq!(record.draft.sources, original_sources);
    assert_eq!(receipt["sources"], json!(original_sources));
    let history = record.draft.changes[1].clone();
    assert_eq!(
        history.text(),
        Some(
            brn_store::note_metadata::to_history(&previous.1.text)
                .unwrap()
                .as_str()
        )
    );
    let changed_current = record.draft.changes[0]
        .text()
        .unwrap()
        .replace("The team chose", "Owner reviewed: The team chose");
    assert_ne!(changed_current, record.draft.changes[0].text().unwrap());
    let edit = ProposalEdit {
        expected: record.stamp(),
        title: "Newer owner supersession review".into(),
        texts: vec![
            Some(changed_current.clone()),
            history.text().map(str::to_owned),
        ],
        action_data: vec![],
    };
    let AppEvent::Proposal(edited) = reply(&worker, AppCommand::EditProposal(edit)) else {
        panic!("owner current-body edit")
    };
    for proof in &original_sources {
        std::fs::remove_file(f.base.path().join("vault").join(&proof.path)).unwrap();
    }
    release.send(()).unwrap();
    let turn = finish(&worker, &request).unwrap();
    assert_eq!(turn.status, WorkTurnStatus::Completed);
    let out = results(&turn);
    assert_eq!(out.len(), 5);
    assert_eq!(out[0]["ok"]["stamp"], json!(edited.stamp()));
    assert_eq!(out[0]["ok"]["sources"], json!(original_sources));
    assert!(out[1..].iter().all(|value| value.get("error").is_some()));
    let retained = analysis(&worker, request.id).proposals.remove(0);
    assert_eq!(retained.stamp(), edited.stamp());
    assert_eq!(retained.draft.sources, original_sources);
    assert_eq!(retained.draft.changes[1], history);
    assert_eq!(
        retained.draft.changes[0].text(),
        Some(changed_current.as_str())
    );
    worker.shutdown().unwrap();
    let mut worker = f.start(Hooks::default());
    assert_eq!(analysis(&worker, request.id).proposals[0], retained);
    assert_eq!(json!(analyze(&worker, &request).unwrap()), json!(turn));
    let AppEvent::Turns(turns) = reply(&worker, AppCommand::Turns(turn.conversation_id)) else {
        panic!("retained turn inventory")
    };
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].id, turn.id);
    retained_original(&worker, &source);
    no_credentials(&f);
    worker.shutdown().unwrap();
}
