use super::*;

#[test]
fn durable_run_identity_preserves_legacy_aliases_unknowns_and_exact_model_effort() {
    let mut settings = WorkspaceSettings {
        document_conversion: Default::default(),
        mcp: None,
        skills: vec![],
        provider: "ChatGPT".into(),
        model: "model".into(),
        effort: "medium".into(),
        auth_file: None,
        credentials_dir: None,
        maintenance: true,
        review_first: false,
        delegation_chosen: true,
        backup_directory: None,
        theme: None,
        topics_note: None,
        helper: None,
    };
    let mut run = Run {
        loaded_skill_details: vec![],
        instruction: None,
        queue_claim: None,
        thread: "thread".into(),
        provider: "Codex".into(),
        model: "model".into(),
        guide_identity: "guide".into(),
        budget: 2,
        effort: "medium".into(),
        loaded_skills: vec![],
        fence: 1,
        state: RunState::Working,
        progress: String::new(),
        operations: vec![],
        sources: vec![],
        web_sources: vec![],
    };
    for (runtime, saved, matches) in [
        ("Codex", "CHATGPT", true),
        ("copilot", "CoPiLoT", true),
        ("ChatGPT", "Copilot", false),
        ("unknown", "UNKNOWN", true),
        ("unknown", "other", false),
        (" codex", "chatgpt", false),
        (" codex", " CODEX", true),
    ] {
        run.provider = runtime.into();
        settings.provider = saved.into();
        assert_eq!(
            run_matches_settings(&run, &settings),
            matches,
            "{runtime}/{saved}"
        );
    }
    run.provider = "Codex".into();
    settings.provider = "ChatGPT".into();
    settings.model = "MODEL".into();
    assert!(!run_matches_settings(&run, &settings));
    settings.model = run.model.clone();
    settings.effort = "MEDIUM".into();
    assert!(!run_matches_settings(&run, &settings));
}
