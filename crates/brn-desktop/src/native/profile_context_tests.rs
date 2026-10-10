//! Real headless controls and exact read-only widgets; no interactive desktop use.
use super::*;
use crate::ai::profile_context_state_tests::{
    PROFILE, PROOF, context, context_request, context_state, saved_profile,
};
use brn_workflow::{
    app::AppConfig, app_worker::AppEvent, knowledge::ProfileLens, library::KnowledgeScope,
};
use gpui_kit::{
    EntityInputHandler, InputEvent as _, ScrollDelta, TestSupportExt, VisualTestContext,
    test::TestWindowExt,
};

struct ContextProbe(Entity<Desktop>, bool);
impl Render for ContextProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            if self.1 {
                desktop.render_simple_document(cx)
            } else {
                div()
                    .id("profile-context-test-pane")
                    .size_full()
                    .flex()
                    .flex_col()
                    .children(desktop.render_profile_context(cx))
                    .test_support()
                    .into_any_element()
            }
        })
    }
}
pub(super) fn window(
    cx: &mut gpui_kit::TestAppContext,
    full: bool,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<Root>,
    Entity<Desktop>,
) {
    window_mode(cx, full, false)
}
pub(super) fn shipping_window(
    cx: &mut gpui_kit::TestAppContext,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<Root>,
    Entity<Desktop>,
) {
    window_mode(cx, true, true)
}
fn window_mode(
    cx: &mut gpui_kit::TestAppContext,
    full: bool,
    shipping: bool,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<Root>,
    Entity<Desktop>,
) {
    cx.update(gpui_kit::component::init);
    let fixture = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = fixture.path().join("data");
    let vault = fixture.path().join("vault");
    std::fs::create_dir(&data).unwrap();
    std::fs::create_dir(&vault).unwrap();
    std::fs::write(vault.join("profile.md"), PROFILE).unwrap();
    std::fs::write(
        vault.join("source.md"),
        "---\nbrn_kind: source\n---\nSynthetic Source\n",
    )
    .unwrap();
    let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
    let saved = capture.clone();
    let handle = cx.open_window(size(px(1100.), px(800.)), move |window, cx| {
        let desktop = cx.new(|cx| {
            let mut desktop = Desktop::new(
                data,
                AppConfig {
                    vault_root: Some(vault),
                    credentials_dir: None,
                    model_dir: None,
                },
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            );
            let mut ai = context_state();
            ai.profile_context.context =
                Some(context(context_request(ProfileLens::Person, 0, 0), 27, 28));
            desktop.ai = Some(ai);
            desktop.profile_context.open = true;
            desktop.open_doc = Some(DocRef::SavedNote);
            desktop.simple_note_path = Some("profile.md".into());
            desktop.centre_tab = CentreTab::Document;
            desktop
                .note_editor
                .update(cx, |editor, cx| editor.set_value(PROFILE, window, cx));
            desktop.sync_profile_context_widgets(window, cx);
            desktop
        });
        *saved.borrow_mut() = Some(desktop.clone());
        if shipping {
            desktop_root(desktop, window, cx)
        } else {
            let probe = cx.new(|_| ContextProbe(desktop, full));
            Root::new(probe, window, cx)
        }
    });
    let desktop = capture.borrow().clone().unwrap();
    (fixture, handle, desktop)
}
pub(super) fn scroll_to(visual: &mut VisualTestContext, target: &str) {
    visual.update(|window, cx| {
        window.render_frame(cx);
        let panel = window.find("profile-context-scroll");
        let target = window.find(target.to_owned());
        let position = panel.bounds().origin + point(px(3.), px(3.));
        window.dispatch_event(
            gpui_kit::MouseMoveEvent {
                position,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        window.render_frame(cx);
        window.dispatch_event(
            gpui_kit::ScrollWheelEvent {
                position,
                delta: ScrollDelta::Pixels(point(
                    px(0.),
                    panel.bounds().origin.y + px(15.) - target.bounds().origin.y,
                )),
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
    });
    visual.run_until_parked();
    visual.update(|window, cx| window.render_frame(cx));
}
#[gpui_kit::test]
fn exact_profile_proofs_and_complete_records_are_read_only_copyable_and_reachable(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    let entities = visual.update(|_, cx| {
        let pane = &desktop.read(cx).profile_context;
        (
            pane.profile.entity_id(),
            pane.quote.entity_id(),
            pane.details.entity_id(),
        )
    });
    scroll_to(&mut visual, "copy-profile-markdown");
    visual.update(|window, cx| {
        window.click("copy-profile-markdown", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(PROFILE)
        );
        let editor = desktop.read(cx).profile_context.profile.clone();
        editor.update(cx, |editor, cx| {
            editor.focus(window, cx);
            editor.replace_text_in_range(Some(0..0), "blocked edit", window, cx);
            assert_eq!(editor.value().as_ref(), PROFILE);
        });
    });
    scroll_to(&mut visual, "profile-next-proof");
    visual.update(|window, cx| {
        assert_eq!(
            desktop
                .read(cx)
                .profile_context
                .quote
                .read(cx)
                .value()
                .as_ref(),
            PROOF
        );
        window.click("profile-next-proof", cx);
        window.render_frame(cx);
        assert_eq!(
            desktop.read(cx).profile_context.quote.entity_id(),
            entities.1
        );
        assert_eq!(
            desktop
                .read(cx)
                .profile_context
                .quote
                .read(cx)
                .value()
                .as_ref(),
            "Second complete proof\r\nλ"
        );
    });
    scroll_to(&mut visual, "copy-profile-proof");
    visual.update(|window, cx| {
        window.click("copy-profile-proof", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("Second complete proof\r\nλ")
        );
        let quote = desktop.read(cx).profile_context.quote.clone();
        quote.update(cx, |editor, cx| {
            editor.focus(window, cx);
            editor.replace_text_in_range(Some(0..0), "blocked", window, cx);
            assert_eq!(editor.value().as_ref(), "Second complete proof\r\nλ");
        });
    });
    scroll_to(&mut visual, "copy-profile-details");
    visual.update(|window, cx| {
        let expected = desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .profile_context
            .context
            .clone()
            .unwrap();
        let details = desktop.read(cx).profile_context.details.clone();
        let actual = serde_json::from_str::<brn_workflow::knowledge::ProfileContext>(
            details.read(cx).value().as_ref(),
        )
        .unwrap();
        assert_eq!(actual, expected);
        window.click("copy-profile-details", cx);
        let copied = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap();
        assert_eq!(
            serde_json::from_str::<brn_workflow::knowledge::ProfileContext>(&copied).unwrap(),
            expected
        );
        details.update(cx, |editor, cx| {
            editor.focus(window, cx);
            editor.replace_text_in_range(Some(0..0), "blocked", window, cx);
            assert_eq!(editor.value().as_ref(), copied);
        });
        desktop.update(cx, |desktop, cx| {
            desktop.clear_profile_panel();
            desktop.sync_profile_context_widgets(window, cx);
            assert_eq!(desktop.profile_context.profile.entity_id(), entities.0);
            assert_eq!(desktop.profile_context.quote.entity_id(), entities.1);
            assert_eq!(desktop.profile_context.details.entity_id(), entities.2);
            assert_eq!(
                desktop.profile_context.profile.read(cx).value().as_ref(),
                ""
            );
            assert_eq!(desktop.profile_context.quote.read(cx).value().as_ref(), "");
            assert_eq!(
                desktop.profile_context.details.read(cx).value().as_ref(),
                ""
            );
        });
    });
}
#[gpui_kit::test]
fn real_page_controls_capture_independent_offsets_and_reject_malformed_ack(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        window.click("profile-actions-next", cx);
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            let id = ai.profile_context.intent.unwrap();
            let crate::ai::Pending::ProfileContext(capture) = ai.pending.get(&id).unwrap() else {
                panic!("context request")
            };
            assert_eq!(
                (
                    capture.request.action_offset,
                    capture.request.relationship_offset
                ),
                (25, 0)
            );
            let expected = context(capture.request.clone(), 27, 28);
            let retained = ai.profile_context.context.clone();
            let mut wrong = expected.clone();
            wrong.profile.text.push_str("mismatched");
            ai.apply(id, AppEvent::ProfileContext(Box::new(wrong)));
            assert_eq!(ai.profile_context.context, retained);
            assert_eq!(ai.profile_context.intent, Some(id));
            ai.apply(id, AppEvent::ProfileContext(Box::new(expected)));
            desktop.sync_profile_context_widgets(window, cx);
        });
        window.render_frame(cx);
        window.click("profile-relationships-next", cx);
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            let id = ai.profile_context.intent.unwrap();
            let crate::ai::Pending::ProfileContext(capture) = ai.pending.get(&id).unwrap() else {
                panic!("context request")
            };
            assert_eq!(
                (
                    capture.request.action_offset,
                    capture.request.relationship_offset
                ),
                (25, 25)
            );
            let expected = context(capture.request.clone(), 27, 28);
            ai.apply(id, AppEvent::ProfileContext(Box::new(expected)));
            desktop.sync_profile_context_widgets(window, cx);
            assert_eq!(
                desktop
                    .ai
                    .as_ref()
                    .unwrap()
                    .profile_context
                    .context
                    .as_ref()
                    .unwrap()
                    .actions
                    .len(),
                2
            );
            assert_eq!(
                desktop
                    .ai
                    .as_ref()
                    .unwrap()
                    .profile_context
                    .context
                    .as_ref()
                    .unwrap()
                    .relationships
                    .len(),
                3
            );
        });
        window.render_frame(cx);
        window.click("profile-actions-previous", cx);
        let ai = desktop.read(cx).ai.as_ref().unwrap();
        let crate::ai::Pending::ProfileContext(capture) =
            ai.pending.get(&ai.profile_context.intent.unwrap()).unwrap()
        else {
            panic!("context request")
        };
        assert_eq!(
            (
                capture.request.action_offset,
                capture.request.relationship_offset
            ),
            (0, 25)
        );
    });
}
#[gpui_kit::test]
fn explicit_native_lens_controls_require_clean_saved_current_and_supersede_prior_intent(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, true);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.find("person-context").visible());
        assert!(window.find("project-context").visible());
        window.click("person-context", cx);
        let first = desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .profile_context
            .intent
            .unwrap();
        window.click("project-context", cx);
        let second = desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .profile_context
            .intent
            .unwrap();
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            let second = ai.profile_context.intent.unwrap();
            assert_ne!(first, second);
            let crate::ai::Pending::ProfileContext(capture) = ai.pending.get(&second).unwrap()
            else {
                panic!("context request")
            };
            assert_eq!(capture.request.lens, ProfileLens::Project);
            ai.apply(
                first,
                AppEvent::ProfileContext(Box::new(context(
                    context_request(ProfileLens::Person, 0, 0),
                    27,
                    28,
                ))),
            );
            assert_eq!(ai.profile_context.intent, Some(second));
            ai.editor
                .as_mut()
                .unwrap()
                .edit(
                    "Retained unsaved owner text λ".into(),
                    std::time::Instant::now(),
                )
                .unwrap();
            cx.notify();
        });
        window.render_frame(cx);
        window.click("person-context", cx);
        assert_eq!(
            desktop.read(cx).ai.as_ref().unwrap().profile_context.intent,
            Some(second)
        );
        assert_eq!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .text,
            "Retained unsaved owner text λ"
        );
    });
}
#[gpui_kit::test]
fn real_support_navigation_preserves_unsaved_buffer_and_comment_guards(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    let reference = format!(
        "profile-reference-{}",
        crate::ai::profile_context_state_tests::source_id()
    );
    scroll_to(&mut visual, &reference);
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.ai.as_mut().unwrap().editor.as_mut().unwrap().edit("Unacknowledged λ".into(), std::time::Instant::now()).unwrap();
            cx.notify();
        });
        window.render_frame(cx);
        window.click(reference.clone(), cx);
        assert_eq!(desktop.read(cx).open_doc, Some(DocRef::SavedNote));
        assert!(matches!(desktop.read(cx).simple_transition.as_ref(), Some(simple::EditorTransition::Evidence { scope: KnowledgeScope::Source, path }) if path == "source.md"));
        assert_eq!(desktop.read(cx).ai.as_ref().unwrap().editor.as_ref().unwrap().text, "Unacknowledged λ");
        assert!(desktop.read(cx).profile_context.open);
        desktop.update(cx, |desktop, cx| {
            desktop.simple_transition = None;
            saved_profile(desktop.ai.as_mut().unwrap());
            desktop.ai.as_mut().unwrap().profile_context.context = Some(context(context_request(ProfileLens::Person, 0, 0), 27, 28));
            desktop.sync_profile_context_widgets(window, cx);
            desktop.review_comment_draft = Some((Uuid::new_v4(), None));
            desktop.review_comment.update(cx, |editor, cx| editor.set_value("Retained comment λ", window, cx));
        });
    });
    scroll_to(&mut visual, &reference);
    visual.update(|window, cx| {
        window.render_frame(cx);
        window.click(reference.clone(), cx);
        assert_eq!(desktop.read(cx).open_doc, Some(DocRef::SavedNote));
        assert!(desktop.read(cx).simple_transition.is_some());
        assert_eq!(
            desktop.read(cx).review_comment.read(cx).value().as_ref(),
            "Retained comment λ"
        );
        desktop.update(cx, |desktop, cx| {
            desktop.review_comment_draft = None;
            desktop
                .review_comment
                .update(cx, |editor, cx| editor.set_value("", window, cx));
            desktop.simple_progress_transition(cx);
            assert_eq!(desktop.open_doc, Some(DocRef::Evidence));
            assert_eq!(
                desktop
                    .ai
                    .as_ref()
                    .unwrap()
                    .evidence
                    .as_ref()
                    .unwrap()
                    .scope,
                KnowledgeScope::Source
            );
            assert!(!desktop.profile_context.open);
            assert!(
                desktop
                    .ai
                    .as_ref()
                    .unwrap()
                    .profile_context
                    .context
                    .is_none()
            );
        });
    });
}

#[gpui_kit::test]
fn direct_source_history_and_outgoing_inferred_proof_use_exact_widgets(
    cx: &mut gpui_kit::TestAppContext,
) {
    use brn_workflow::knowledge::{
        EdgeEndpoint, EdgeEvidence, EdgeOrigin, EvidenceEndpoint, NoteEdge, ProfileRelationship,
    };
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            let mut value = context(context_request(ProfileLens::Person, 0, 0), 2, 2);
            value.relationship_total = 3;
            value.relationships.push(ProfileRelationship {
                edge: NoteEdge {
                    source: EdgeEndpoint {
                        path: value.request.profile.path.clone(),
                        note_id: value.request.note_id,
                        sha256: value.request.profile.fingerprint.sha256,
                    },
                    target: EdgeEndpoint {
                        path: "archive/history.md".into(),
                        note_id: Uuid::from_u128(9000),
                        sha256: [9; 32],
                    },
                    origin: EdgeOrigin::InferredProvenance,
                    evidence: vec![EdgeEvidence {
                        endpoint: EvidenceEndpoint::Target,
                        start_byte: 0,
                        end_byte: PROOF.len(),
                        quote: PROOF.into(),
                    }],
                },
                source_scope: KnowledgeScope::Current,
                target_scope: KnowledgeScope::History,
            });
            value.validate_for(&value.request).unwrap();
            desktop.ai.as_mut().unwrap().profile_context.context = Some(value);
            desktop.sync_profile_context_widgets(window, cx);
        });
    });
    scroll_to(&mut visual, "profile-next-relationship");
    for _ in 0..2 {
        visual.update(|window, cx| {
            window.click("profile-next-relationship", cx);
            window.render_frame(cx);
        });
        visual.run_until_parked();
    }
    scroll_to(&mut visual, "copy-profile-proof");
    visual.update(|window, cx| {
        assert_eq!(
            desktop
                .read(cx)
                .profile_context
                .quote
                .read(cx)
                .value()
                .as_ref(),
            PROOF
        );
        window.click("copy-profile-proof", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(PROOF)
        );
    });
    scroll_to(&mut visual, "profile-open-Target");
    visual.update(|window, cx| {
        window.click("profile-open-Target", cx);
        assert_eq!(desktop.read(cx).open_doc, Some(DocRef::Evidence));
        assert_eq!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .evidence
                .as_ref()
                .unwrap()
                .scope,
            KnowledgeScope::History
        );
        assert_eq!(
            desktop.read(cx).simple_note_path.as_deref(),
            Some("archive/history.md")
        );
    });
}
