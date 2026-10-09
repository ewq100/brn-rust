use super::*;
use brn_workflow::{
    actions::{ActionData, ActionOrigin, ActionRecord, ActionState},
    editor::FileFingerprint,
    knowledge::{
        EdgeEndpoint, EdgeEvidence, EdgeOrigin, EvidenceEndpoint, IdentityOutcome,
        IdentityResolution, NoteEdge, NoteIdentityInfo, ProfileContext, ProfileContextRequest,
        ProfileLens, ProfileNote, ProfileReference, ProfileRelationship,
    },
    proposals::{ProposalSource, ProposalStamp, SourceVersion},
};

pub(crate) const PROFILE: &str = "\u{feff}---\r\nbrn_id: 11111111-1111-4111-8111-111111111111\r\n---\r\nFull saved profile 日本語\r\n";
pub(crate) const PROOF: &str = "\u{feff}Complete Source proof 日本語\r\nsecond λ\r";
pub(crate) fn profile_id() -> Uuid {
    Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap()
}
pub(crate) fn source_id() -> Uuid {
    Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap()
}
pub(crate) fn profile_source() -> SourceVersion {
    SourceVersion {
        path: "profile.md".into(),
        fingerprint: FileFingerprint {
            device: 1,
            inode: 2,
            len: PROFILE.len() as u64,
            sha256: [
                35, 180, 253, 91, 191, 151, 61, 161, 35, 243, 144, 178, 32, 73, 91, 231, 195, 205,
                114, 188, 76, 195, 60, 191, 237, 240, 223, 69, 165, 197, 44, 255,
            ],
        },
    }
}
pub(crate) fn saved_profile(state: &mut AiState) {
    let source = profile_source();
    let view = EditorView {
        record: EditorRecord {
            path: source.path.clone(),
            stamp: EditStamp {
                baseline: Uuid::new_v4(),
                generation: 0,
            },
            baseline: source.fingerprint.clone(),
            baseline_text: PROFILE.into(),
            text: PROFILE.into(),
        },
        saved: Some(PROFILE.into()),
        observed: Some(source.fingerprint),
        conflict: false,
        pending: vec![],
    };
    let (id, _) = state.open_editor(source.path);
    state.apply(id, AppEvent::Editor(view));
}
pub(crate) fn context_state() -> AiState {
    let mut state = AiState {
        ready: true,
        vault_bound: true,
        ..Default::default()
    };
    saved_profile(&mut state);
    state
}
pub(crate) fn context_request(
    lens: ProfileLens,
    action_offset: usize,
    relationship_offset: usize,
) -> ProfileContextRequest {
    ProfileContextRequest {
        profile: profile_source(),
        note_id: profile_id(),
        lens,
        action_offset,
        relationship_offset,
        limit: 25,
    }
}
fn action(index: usize, lens: ProfileLens) -> ActionRecord {
    let data = ActionData {
        title: format!("Matching retained Action {index}"),
        description: format!("Whole exact Action description {index}\r\n日本語"),
        state: ActionState::Open,
        owner: Some("Synthetic owner".into()),
        related_person: (lens == ProfileLens::Person).then(profile_id),
        related_project: (lens == ProfileLens::Project).then(profile_id),
        sources: vec![source_id()],
        thread: Some(source_id()),
        due_on: None,
        follow_up_on: None,
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: None,
    };
    let mut record = ActionRecord {
        origin: ActionOrigin {
            id: Uuid::from_u128(1000 + index as u128),
            proposal: ProposalStamp {
                id: Uuid::from_u128(999),
                version: 1,
            },
            data: data.clone(),
            created_at_ms: 100 - index as u64,
        },
        version: 1,
        data,
        updated_at_ms: 100 - index as u64,
        waiting_since_ms: None,
        completed_at_ms: None,
    };
    if index == 0 {
        record.version = 2;
        record.data.state = ActionState::Completed;
        record.completed_at_ms = Some(record.updated_at_ms);
    }
    record.validate().unwrap();
    record
}
pub(crate) fn context(
    request: ProfileContextRequest,
    action_total: usize,
    relationship_total: usize,
) -> ProfileContext {
    let profile = EdgeEndpoint {
        path: request.profile.path.clone(),
        note_id: request.note_id,
        sha256: request.profile.fingerprint.sha256,
    };
    let source = NoteIdentityInfo {
        path: "source.md".into(),
        note_id: Some(source_id()),
        sha256: [5; 32],
    };
    let actions = (request.action_offset..action_total.min(request.action_offset + request.limit))
        .map(|i| action(i, request.lens))
        .collect::<Vec<_>>();
    let references = if actions.is_empty() {
        vec![]
    } else {
        vec![ProfileReference {
            resolution: IdentityResolution {
                note_id: source_id(),
                outcome: IdentityOutcome::Unique,
                matches: vec![source.clone()],
                issues: vec![],
            },
            matches: vec![ProfileNote {
                note: source,
                scope: Some(KnowledgeScope::Source),
            }],
        }]
    };
    let relationships = (request.relationship_offset
        ..relationship_total.min(request.relationship_offset + request.limit))
        .map(|index| ProfileRelationship {
            edge: NoteEdge {
                source: EdgeEndpoint {
                    path: format!("incoming-{index:03}.md"),
                    note_id: Uuid::from_u128(5000 + index as u128),
                    sha256: [4; 32],
                },
                target: profile.clone(),
                origin: EdgeOrigin::ExplicitLink,
                evidence: vec![
                    EdgeEvidence {
                        endpoint: EvidenceEndpoint::Source,
                        start_byte: 0,
                        end_byte: PROOF.len(),
                        quote: PROOF.into(),
                    },
                    EdgeEvidence {
                        endpoint: EvidenceEndpoint::Source,
                        start_byte: 200,
                        end_byte: 200 + "Second complete proof\r\nλ".len(),
                        quote: "Second complete proof\r\nλ".into(),
                    },
                ],
            },
            source_scope: if index % 2 == 0 {
                KnowledgeScope::Source
            } else {
                KnowledgeScope::History
            },
            target_scope: KnowledgeScope::Current,
        })
        .collect();
    let context = ProfileContext {
        profile: ProposalSource {
            source: request.profile.clone(),
            text: PROFILE.into(),
        },
        request,
        action_total,
        actions,
        relationship_total,
        relationships,
        references,
        issues: vec![],
        duplicates: vec![],
        complete: true,
    };
    context.validate_for(&context.request).unwrap();
    context
}
fn settle(
    state: &mut AiState,
    lens: ProfileLens,
    action_total: usize,
    relationship_total: usize,
) -> ProfileContext {
    let (id, command) = state.inspect_profile_context(lens, 0, 0).unwrap();
    let AppCommand::ProfileContext(request) = command else {
        panic!("wrong command")
    };
    let context = context(request, action_total, relationship_total);
    state.apply(id, AppEvent::ProfileContext(Box::new(context.clone())));
    context
}
#[test]
fn explicit_lenses_capture_full_saved_identity_and_keep_completed_and_whole_records() {
    let mut state = context_state();
    let person = settle(&mut state, ProfileLens::Person, 2, 2);
    assert_eq!(state.profile_context.context.as_ref(), Some(&person));
    assert_eq!(person.profile.text.as_bytes(), PROFILE.as_bytes());
    assert_eq!(person.actions[0].data.state, ActionState::Completed);
    assert_eq!(person.actions[0].data.related_person, Some(profile_id()));
    assert_eq!(person.actions[0].data.related_project, None);
    assert_eq!(
        person.relationships[1].source_scope,
        KnowledgeScope::History
    );
    assert_eq!(person.references[0].resolution.note_id, source_id());
    let project = settle(&mut state, ProfileLens::Project, 2, 2);
    assert_eq!(project.actions[0].data.related_project, Some(profile_id()));
    assert_eq!(project.actions[0].data.related_person, None);
    assert!(!state.profile_context_loading());
}
#[test]
fn clean_acknowledged_current_managed_saved_profile_is_required() {
    let mut state = context_state();
    assert!(state.profile_context_available());
    state
        .editor
        .as_mut()
        .unwrap()
        .edit("Unacknowledged draft 日本語".into(), Instant::now())
        .unwrap();
    assert!(
        state
            .inspect_profile_context(ProfileLens::Person, 0, 0)
            .is_none()
    );
    saved_profile(&mut state);
    state.editor.as_mut().unwrap().view.conflict = true;
    assert!(!state.profile_context_available());
    saved_profile(&mut state);
    state
        .editor
        .as_mut()
        .unwrap()
        .view
        .pending
        .push(Uuid::new_v4());
    assert!(!state.profile_context_available());
    saved_profile(&mut state);
    state.select_scope(KnowledgeScope::Source);
    assert!(
        state
            .inspect_profile_context(ProfileLens::Person, 0, 0)
            .is_none()
    );
    state.knowledge_scope = KnowledgeScope::Current;
    for text in [
        "Unmanaged saved note",
        "---\nbrn_id: 11111111-1111-4111-8111-111111111111\nbrn_kind: source\n---\nSource",
        "---\nbrn_id: 11111111-1111-4111-8111-111111111111\nbrn_state: history\n---\nHistory",
        "---\nbrn_id: 11111111-1111-4111-8111-111111111111\nbrn_kind: unknown\n---\nInvalid",
    ] {
        let mut view = super::tests::editor_view("profile.md", text);
        view.observed.as_mut().unwrap().len = text.len() as u64;
        state.editor = Some(SimpleEditor::new(view));
        assert!(!state.profile_context_available(), "{text}");
    }
    saved_profile(&mut state);
    state.editor.as_mut().unwrap().view.record.path = "archive/profile.md".into();
    assert!(!state.profile_context_available());
}
#[test]
fn independent_pages_bind_both_offsets_and_preserve_previous_view_until_valid_reply() {
    let mut state = context_state();
    let original = settle(&mut state, ProfileLens::Person, 27, 28);
    assert_eq!(state.profile_action_offset(true), Some(25));
    assert_eq!(state.profile_relationship_offset(true), Some(25));
    assert_eq!(state.profile_action_offset(false), None);
    let (id, command) = state
        .inspect_profile_context(ProfileLens::Person, 25, 0)
        .unwrap();
    assert!(state.profile_context_loading());
    assert_eq!(state.profile_context.context.as_ref(), Some(&original));
    let AppCommand::ProfileContext(request) = command else {
        panic!("wrong command")
    };
    assert_eq!(
        (request.action_offset, request.relationship_offset),
        (25, 0)
    );
    state.apply(
        id,
        AppEvent::ProfileContext(Box::new(context(request, 27, 28))),
    );
    assert_eq!(
        state
            .profile_context
            .context
            .as_ref()
            .unwrap()
            .actions
            .len(),
        2
    );
    assert_eq!(state.profile_action_offset(true), None);
    assert_eq!(state.profile_action_offset(false), Some(0));
    let (id, command) = state
        .inspect_profile_context(ProfileLens::Person, 25, 25)
        .unwrap();
    let AppCommand::ProfileContext(request) = command else {
        panic!("wrong command")
    };
    state.apply(
        id,
        AppEvent::ProfileContext(Box::new(context(request, 27, 28))),
    );
    assert_eq!(
        state
            .profile_context
            .context
            .as_ref()
            .unwrap()
            .relationships
            .len(),
        3
    );
    assert_eq!(state.profile_relationship_offset(false), Some(0));
    assert_eq!(state.profile_relationship_offset(true), None);
    assert!(
        state
            .inspect_profile_context(ProfileLens::Person, usize::MAX, 0)
            .is_none()
    );
}
#[test]
fn malformed_wrong_and_late_replies_neither_replace_view_nor_settle_another_intent() {
    let mut state = context_state();
    let original = settle(&mut state, ProfileLens::Person, 2, 2);
    let (old, _) = state
        .inspect_profile_context(ProfileLens::Person, 0, 0)
        .unwrap();
    let (new, _) = state
        .inspect_profile_context(ProfileLens::Project, 0, 0)
        .unwrap();
    state.apply(old, AppEvent::ProfileContext(Box::new(original.clone())));
    assert_eq!(state.profile_context.intent, Some(new));
    assert!(state.pending.contains_key(&new));
    let valid = context(context_request(ProfileLens::Project, 0, 0), 2, 2);
    let mut malformed = valid.clone();
    malformed.request.relationship_offset = 25;
    state.apply(new, AppEvent::ProfileContext(Box::new(malformed)));
    state.apply(
        new,
        AppEvent::Editor(super::tests::editor_view("wrong.md", "wrong")),
    );
    assert_eq!(state.profile_context.context.as_ref(), Some(&original));
    assert!(state.pending.contains_key(&new));
    let (other, _) = state.command(Pending::Editors, AppCommand::Editors);
    state.apply(other, AppEvent::ProfileContext(Box::new(valid.clone())));
    assert!(state.pending.contains_key(&other));
    assert!(state.pending.contains_key(&new));
    for case in 0..7 {
        let mut malformed = valid.clone();
        match case {
            0 => malformed.profile.text.push_str("changed"),
            1 => malformed.actions[0].data.related_project = None,
            2 => malformed.relationships[0].source_scope = KnowledgeScope::All,
            3 => malformed.relationships[0].edge.evidence[0].end_byte += 1,
            4 => malformed.references.clear(),
            5 => malformed.references[0].resolution.outcome = IdentityOutcome::Absent,
            _ => malformed.action_total += 1,
        }
        assert!(malformed.validate_for(&valid.request).is_err());
        state.apply(new, AppEvent::ProfileContext(Box::new(malformed)));
        assert_eq!(state.profile_context.context.as_ref(), Some(&original));
        assert!(state.pending.contains_key(&new));
    }
    state.apply(new, AppEvent::ProfileContext(Box::new(valid.clone())));
    assert_eq!(state.profile_context.context.as_ref(), Some(&valid));
    assert!(!state.pending.contains_key(&new));
}
#[test]
fn selection_scope_refresh_save_rebind_and_application_invalidate_late_context() {
    for lifecycle in 0..7 {
        let mut state = context_state();
        let original = settle(&mut state, ProfileLens::Person, 0, 0);
        let (late, _) = state
            .inspect_profile_context(ProfileLens::Person, 0, 0)
            .unwrap();
        match lifecycle {
            0 => {
                state.open_editor("different.md".into());
            }
            1 => {
                state.select_scope(KnowledgeScope::History);
            }
            2 => {
                state.command(Pending::Refresh, AppCommand::Refresh);
            }
            3 => {
                state.save_editor(None).unwrap();
            }
            4 => {
                state.command(
                    Pending::Bind,
                    AppCommand::BindVault(std::path::PathBuf::from("/synthetic/other")),
                );
            }
            5 => {
                state.refresh_after_application(&[], state.review_generation);
            }
            _ => {
                state.refresh_editor();
            }
        }
        state.apply(late, AppEvent::ProfileContext(Box::new(original)));
        assert!(state.profile_context.context.is_none());
        assert!(!state.profile_context_loading());
    }
}
#[test]
fn saved_hash_uuid_or_document_generation_change_refuses_reply() {
    for change in 0..4 {
        let mut state = context_state();
        let (id, _) = state
            .inspect_profile_context(ProfileLens::Person, 0, 0)
            .unwrap();
        let context = context(context_request(ProfileLens::Person, 0, 0), 0, 0);
        match change {
            0 => {
                state
                    .editor
                    .as_mut()
                    .unwrap()
                    .view
                    .observed
                    .as_mut()
                    .unwrap()
                    .sha256[0] ^= 1
            }
            1 => state.editor.as_mut().unwrap().view.record.path = "other.md".into(),
            2 => state.note_generation += 1,
            _ => {
                state
                    .editor
                    .as_mut()
                    .unwrap()
                    .edit("New unsaved owner text".into(), Instant::now())
                    .unwrap();
            }
        }
        state.apply(id, AppEvent::ProfileContext(Box::new(context)));
        assert!(state.profile_context.context.is_none());
    }
}

#[test]
fn current_read_failure_retains_exact_view_and_old_failure_cannot_replace_new_error() {
    let mut state = context_state();
    let retained = settle(&mut state, ProfileLens::Person, 2, 2);
    let (old, _) = state
        .inspect_profile_context(ProfileLens::Person, 0, 0)
        .unwrap();
    let (current, _) = state
        .inspect_profile_context(ProfileLens::Project, 0, 0)
        .unwrap();
    state.apply(
        old,
        AppEvent::Failed(brn_workflow::WorkflowError {
            kind: brn_workflow::ErrorKind::ContextStale,
            message: "old failure".into(),
        }),
    );
    assert!(state.profile_context.error.is_none());
    assert_eq!(state.profile_context.intent, Some(current));
    state.apply(
        current,
        AppEvent::Failed(brn_workflow::WorkflowError {
            kind: brn_workflow::ErrorKind::ContextStale,
            message: "Saved profile changed; refresh it".into(),
        }),
    );
    assert_eq!(state.profile_context.context.as_ref(), Some(&retained));
    assert_eq!(
        state.profile_context.error.as_deref(),
        Some("Saved profile changed; refresh it")
    );
    assert!(!state.pending.contains_key(&current));
}
