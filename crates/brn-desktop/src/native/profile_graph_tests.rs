//! Pure page projection and real headless graph controls; no GUI qualification claim.
use super::profile_context_tests::{scroll_to, window};
use super::profile_graph::{Binding, Projection, Rect};
use super::*;
use crate::ai::profile_context_state_tests::{
    PROFILE, PROOF, context, context_request, saved_profile,
};
use brn_workflow::{
    app_worker::AppEvent,
    knowledge::{EdgeOrigin, ProfileContext, ProfileLens},
    library::KnowledgeScope,
};
use gpui_kit::{InputEvent as _, VisualTestContext, test::TestWindowExt};

fn page(edges: usize) -> ProfileContext {
    context(context_request(ProfileLens::Person, 0, 0), 0, edges)
}
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}
#[test]
fn projection_retains_isolated_center_deduplicates_endpoints_and_preserves_every_origin_direction()
{
    let empty = page(0);
    let projection = Projection::new(&empty);
    assert_eq!(projection.nodes.len(), 1);
    assert!(projection.edges.is_empty());
    assert_eq!(projection.nodes[0].endpoint.path, empty.profile.source.path);
    assert_eq!(
        projection.nodes[0].endpoint.sha256,
        empty.profile.source.fingerprint.sha256
    );
    assert_eq!(projection.nodes[0].scope, KnowledgeScope::Current);
    let mut mixed = page(2);
    let incoming = mixed.relationships[0].clone();
    let mut outgoing = incoming.clone();
    std::mem::swap(&mut outgoing.edge.source, &mut outgoing.edge.target);
    std::mem::swap(&mut outgoing.source_scope, &mut outgoing.target_scope);
    outgoing.edge.evidence.truncate(1);
    outgoing.edge.evidence[0].start_byte = 0;
    outgoing.edge.evidence[0].end_byte = PROFILE.len();
    outgoing.edge.evidence[0].quote = PROFILE.into();
    let mut inferred = outgoing.clone();
    inferred.edge.origin = EdgeOrigin::InferredProvenance;
    for proof in &mut inferred.edge.evidence {
        proof.endpoint = brn_workflow::knowledge::EvidenceEndpoint::Target;
    }
    mixed.relationships.extend([outgoing, inferred]);
    mixed.relationship_total = mixed.relationships.len();
    mixed.validate_for(&mixed.request).unwrap();
    let projection = Projection::new(&mixed);
    assert_eq!(projection.nodes.len(), 3);
    assert_eq!(projection.edges, [(1, 0), (2, 0), (0, 1), (0, 1)]);
    assert_eq!(projection.nodes[1].scope, KnowledgeScope::Source);
    assert_eq!(projection.nodes[2].scope, KnowledgeScope::History);
    for (edge, &(source, target)) in mixed.relationships.iter().zip(&projection.edges) {
        assert_eq!(projection.nodes[source].endpoint, edge.edge.source);
        assert_eq!(projection.nodes[target].endpoint, edge.edge.target);
    }
}
#[test]
fn maximum_page_geometry_is_deterministic_positive_disjoint_and_within_scrollable_extent() {
    for count in 0..=25 {
        let context = page(count);
        let a = Projection::new(&context);
        let b = Projection::new(&context);
        assert_eq!(a.bounds, b.bounds);
        assert_eq!(a.nodes.len(), count + 1);
        assert_eq!(a.edges.len(), count);
        for (index, rect) in a.bounds.iter().enumerate() {
            assert!(
                [rect.x, rect.y, rect.width, rect.height, a.width, a.height]
                    .iter()
                    .all(|v| v.is_finite())
            );
            assert!(rect.width > 0. && rect.height > 0. && rect.x >= 0. && rect.y >= 0.);
            assert!(rect.x + rect.width <= a.width && rect.y + rect.height <= a.height);
            for other in &a.bounds[index + 1..] {
                assert!(!overlaps(*rect, *other));
            }
        }
    }
}

#[gpui_kit::test]
fn graph_zero_counts_coverage_loading_errors_and_history_navigation_are_explicit(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        let mut empty = page(0);
        empty.complete = false;
        install(&desktop, empty, window, cx);
        window.click("profile-view-graph", cx);
        assert!(
            window
                .find("profile-context-coverage")
                .label()
                .unwrap()
                .contains("incomplete")
        );
        desktop.update(cx, |desktop, _| {
            desktop
                .ai
                .as_mut()
                .unwrap()
                .inspect_profile_context(ProfileLens::Person, 0, 0)
                .unwrap();
        });
        window.render_frame(cx);
        assert!(
            window
                .find("profile-context-loading")
                .label()
                .unwrap()
                .contains("Previous observation retained")
        );
    });
    scroll_to(&mut visual, "profile-graph-scroll");
    visual.update(|window, cx| {
        window.click("profile-graph-node-0", cx);
        assert!(desktop.read(cx).profile_context.graph_selection.is_none());
        desktop.update(cx, |desktop, _| {
            desktop.ai.as_mut().unwrap().profile_context.intent = None;
            desktop.ai.as_mut().unwrap().profile_context.error =
                Some("Synthetic scan failed".into());
        });
        window.render_frame(cx);
        assert!(
            window
                .find("profile-context-error")
                .label()
                .unwrap()
                .contains("Synthetic scan failed")
        );
    });
    scroll_to(&mut visual, "profile-graph-summary");
    visual.update(|window, cx| {
        assert!(
            window
                .find("profile-graph-summary")
                .label()
                .unwrap()
                .contains("0 displayed edges · 0 total direct relationships · 1 distinct")
        );
        install(&desktop, page(2), window, cx);
    });
    scroll_to(&mut visual, "profile-graph-scroll");
    visual.update(|window, cx| {
        window.click("profile-graph-node-2", cx);
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
            Some("incoming-001.md")
        );
        assert!(!desktop.read(cx).profile_context.open);
    });
}

#[gpui_kit::test]
fn opposite_direction_and_multiple_origin_buttons_keep_distinct_exact_quotes(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    let mut value = page(1);
    let mut outgoing = value.relationships[0].clone();
    std::mem::swap(&mut outgoing.edge.source, &mut outgoing.edge.target);
    std::mem::swap(&mut outgoing.source_scope, &mut outgoing.target_scope);
    let mut inferred = outgoing.clone();
    inferred.edge.origin = EdgeOrigin::InferredProvenance;
    for proof in &mut inferred.edge.evidence {
        proof.endpoint = brn_workflow::knowledge::EvidenceEndpoint::Target;
    }
    value.relationships.extend([outgoing, inferred]);
    value.relationship_total = value.relationships.len();
    for (index, edge) in value.relationships.iter_mut().enumerate() {
        edge.edge.evidence.truncate(1);
        let proof = &mut edge.edge.evidence[0];
        proof.quote = if edge.edge.source.note_id == value.request.note_id
            && proof.endpoint == brn_workflow::knowledge::EvidenceEndpoint::Source
        {
            PROFILE.into()
        } else {
            format!(
                "\u{feff}Exact {} → {} {:?} proof {index} 日本語\r\n",
                edge.edge.source.path, edge.edge.target.path, edge.edge.origin
            )
        };
        proof.start_byte = 0;
        proof.end_byte = proof.quote.len();
    }
    let expected = value
        .relationships
        .iter()
        .map(|edge| edge.edge.evidence[0].quote.clone())
        .collect::<Vec<_>>();
    visual.update(|window, cx| {
        install(&desktop, value, window, cx);
        window.click("profile-view-graph", cx);
    });
    for (index, expected) in expected.into_iter().enumerate() {
        scroll_to(&mut visual, &format!("profile-graph-edge-{index}"));
        visual.update(|window, cx| {
            window.click(format!("profile-graph-edge-{index}"), cx);
            assert_eq!(
                desktop
                    .read(cx)
                    .profile_context
                    .quote
                    .read(cx)
                    .value()
                    .as_ref(),
                expected
            );
        });
        scroll_to(&mut visual, "copy-profile-proof");
        visual.update(|window, cx| {
            window.click("copy-profile-proof", cx);
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some(expected.as_str())
            );
        });
    }
}
fn install(desktop: &Entity<Desktop>, value: ProfileContext, window: &mut Window, cx: &mut App) {
    value.validate_for(&value.request).unwrap();
    desktop.update(cx, |desktop, cx| {
        desktop.ai.as_mut().unwrap().profile_context.context = Some(value);
        desktop.profile_context.open = true;
        desktop.sync_profile_context_widgets(window, cx);
    });
}

#[gpui_kit::test]
fn last_graph_node_and_edge_are_reachable_after_scrolling_a_small_viewport(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.simulate_resize(size(px(480.), px(800.)));
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.click("profile-view-graph", cx);
    });
    scroll_to(&mut visual, "profile-graph-scroll");
    visual.update(|window, cx| {
        window.scroll(
            "profile-graph-scroll",
            gpui_kit::ScrollDelta::Pixels(point(px(-10000.), px(0.))),
            cx,
        );
        window.scroll(
            "profile-graph-scroll",
            gpui_kit::ScrollDelta::Pixels(point(px(0.), px(-10000.))),
            cx,
        );
    });
    scroll_to(&mut visual, "profile-graph-node-25");
    visual.update(|window, cx| {
        assert!(
            window.find("profile-graph-node-25").visible(),
            "node={:?} graph={:?} offset={:?}",
            window.find("profile-graph-node-25"),
            window.find("profile-graph-scroll"),
            desktop.read(cx).profile_context.graph_scroll.offset()
        );
    });
    scroll_to(&mut visual, "profile-graph-edge-24");
    visual.update(|window, cx| {
        assert!(window.find("profile-graph-edge-24").visible());
        window.click("profile-graph-edge-24", cx);
        assert_eq!(desktop.read(cx).profile_context.relationship, 24);
    });
}

#[gpui_kit::test]
fn graph_edge_buttons_select_complete_exact_proof_and_full_page_controls_remain_reachable(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.click("profile-view-graph", cx);
    });
    scroll_to(&mut visual, "profile-graph-summary");
    visual.update(|window, cx| {
        assert!(
            window
                .find("profile-graph-summary")
                .label()
                .unwrap()
                .contains("25 displayed edges · 28 total")
        );
        for index in 0..25 {
            window.find(format!("profile-graph-edge-{index}"));
        }
        for index in 0..26 {
            window.find(format!("profile-graph-node-{index}"));
        }
        let nodes = (0..26)
            .map(|index| window.find(format!("profile-graph-node-{index}")).bounds())
            .collect::<Vec<_>>();
        for (index, node) in nodes.iter().enumerate() {
            assert!(node.size.width > px(0.) && node.size.height > px(0.));
            for other in &nodes[index + 1..] {
                assert!(!node.intersects(other));
            }
        }
        window.scroll(
            "profile-graph-scroll",
            gpui_kit::ScrollDelta::Pixels(point(px(-400.), px(-2400.))),
            cx,
        );
        window.render_frame(cx);
        assert!(window.find("profile-graph-node-25").visible());
    });
    scroll_to(&mut visual, "profile-graph-edge-24");
    visual.update(|window, cx| {
        window.click("profile-graph-edge-24", cx);
        assert_eq!(desktop.read(cx).profile_context.relationship, 24);
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
    });
    scroll_to(&mut visual, "copy-profile-proof");
    visual.update(|window, cx| {
        window.click("copy-profile-proof", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(PROOF)
        );
    });
    // Same existing proof controls preserve the second whole proof, rather than truncating it.
    scroll_to(&mut visual, "profile-next-proof");
    visual.update(|window, cx| {
        window.click("profile-next-proof", cx);
    });
    scroll_to(&mut visual, "copy-profile-proof");
    visual.update(|window, cx| {
        window.click("copy-profile-proof", cx);
        let expected = &desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .profile_context
            .context
            .as_ref()
            .unwrap()
            .relationships[24]
            .edge
            .evidence[1]
            .quote;
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(expected.as_str())
        );
        window.click("profile-relationships-next", cx);
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            let id = ai.profile_context.intent.unwrap();
            let crate::ai::Pending::ProfileContext(capture) = ai.pending.get(&id).unwrap() else {
                panic!("context request")
            };
            ai.apply(
                id,
                AppEvent::ProfileContext(Box::new(context(capture.request.clone(), 27, 28))),
            );
            desktop.sync_profile_context_widgets(window, cx);
            assert_eq!(desktop.profile_context.relationship, 0);
            assert!(desktop.profile_context.graph_selection.is_none());
        });
    });
    scroll_to(&mut visual, "profile-graph-summary");
    visual.update(|window, _| {
        assert!(
            window
                .find("profile-graph-summary")
                .label()
                .unwrap()
                .contains("offset 25 · 3 displayed edges · 28 total")
        );
    });
}

#[gpui_kit::test]
fn graph_node_buttons_use_unsaved_comment_and_initial_draft_guards(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        install(&desktop, page(1), window, cx);
        window.click("profile-view-graph", cx);
    });
    scroll_to(&mut visual, "profile-graph-scroll");
    visual.update(|window, cx| {
        window.click("profile-graph-node-0", cx);
        assert!(desktop.read(cx).profile_context.graph_selection.as_ref().unwrap().contains("UUID"));
        assert_eq!(desktop.read(cx).open_doc, Some(DocRef::SavedNote));
        desktop.update(cx, |desktop, _| {
            desktop.ai.as_mut().unwrap().editor.as_mut().unwrap().edit("Unsaved graph buffer λ".into(), Instant::now()).unwrap();
        });
        window.click("profile-graph-node-1", cx);
        assert!(matches!(desktop.read(cx).simple_transition.as_ref(), Some(simple::EditorTransition::Evidence { path, scope: KnowledgeScope::Source }) if path == "incoming-000.md"));
        assert_eq!(desktop.read(cx).open_doc, Some(DocRef::SavedNote));
        assert_eq!(desktop.read(cx).ai.as_ref().unwrap().editor.as_ref().unwrap().text, "Unsaved graph buffer λ");
        desktop.update(cx, |desktop, cx| {
            desktop.simple_transition = None;
            saved_profile(desktop.ai.as_mut().unwrap());
            desktop.review_comment_draft = Some((Uuid::new_v4(), None));
            desktop.review_comment.update(cx, |editor, cx| editor.set_value("Retained graph comment λ", window, cx));
        });
        install(&desktop, page(1), window, cx);
    });
    scroll_to(&mut visual, "profile-graph-scroll");
    visual.update(|window, cx| {
        window.click("profile-graph-node-1", cx);
        assert_eq!(desktop.read(cx).open_doc, Some(DocRef::SavedNote));
        assert!(desktop.read(cx).simple_transition.is_some());
        assert_eq!(
            desktop.read(cx).review_comment.read(cx).value().as_ref(),
            "Retained graph comment λ"
        );
        desktop.update(cx, |desktop, cx| {
            desktop.simple_transition = None;
            desktop.review_comment_draft = None;
            desktop
                .review_comment
                .update(cx, |editor, cx| editor.set_value("", window, cx));
            let mut draft = crate::draft::DraftForm::new(None).unwrap();
            draft.edit(
                "Retained graph draft".into(),
                "future.md".into(),
                "Full graph draft λ".into(),
                crate::draft::DraftKind::Create,
            );
            desktop.ai.as_mut().unwrap().draft = Some(draft);
        });
        window.click("profile-graph-node-1", cx);
        assert!(desktop.read(cx).simple_transition.is_none());
        assert_eq!(desktop.read(cx).open_doc, Some(DocRef::SavedNote));
        assert_eq!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .draft
                .as_ref()
                .unwrap()
                .text,
            "Full graph draft λ"
        );
    });
}

// Dispatch against the previously painted controls without constructing a fresh frame.
// This exercises the actual old Button closure after the underlying observation changes.
fn click_painted(window: &mut Window, position: gpui_kit::Point<gpui_kit::Pixels>, cx: &mut App) {
    window.dispatch_event(
        gpui_kit::MouseMoveEvent {
            position,
            ..Default::default()
        }
        .to_platform_input(),
        cx,
    );
    window.dispatch_event(
        gpui_kit::MouseDownEvent {
            button: gpui_kit::MouseButton::Left,
            position,
            click_count: 1,
            modifiers: Default::default(),
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
    window.dispatch_event(
        gpui_kit::MouseUpEvent {
            button: gpui_kit::MouseButton::Left,
            position,
            click_count: 1,
            modifiers: Default::default(),
        }
        .to_platform_input(),
        cx,
    );
}
#[gpui_kit::test]
fn painted_graph_buttons_cannot_act_on_replaced_page_or_refreshed_context(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        install(&desktop, page(2), window, cx);
        window.click("profile-view-graph", cx);
    });
    scroll_to(&mut visual, "profile-graph-edge-1");
    visual.update(|window, cx| {
        let bounds = window.find("profile-graph-edge-1").bounds();
        let position = bounds.origin + point(bounds.size.width / 2., bounds.size.height / 2.);
        desktop.update(cx, |desktop, _| {
            let ai = desktop.ai.as_mut().unwrap();
            ai.profile_context.context =
                Some(context(context_request(ProfileLens::Person, 0, 25), 0, 28));
        });
        click_painted(window, position, cx);
        assert_eq!(desktop.read(cx).profile_context.relationship, 0);
        assert!(desktop.read(cx).profile_context.graph_selection.is_none());
        install(&desktop, page(1), window, cx);
    });
    scroll_to(&mut visual, "profile-graph-scroll");
    visual.update(|window, cx| {
        let bounds = window.find("profile-graph-node-1").bounds();
        let position = bounds.origin + point(bounds.size.width / 2., bounds.size.height / 2.);
        desktop.update(cx, |desktop, _| {
            desktop
                .ai
                .as_mut()
                .unwrap()
                .inspect_profile_context(ProfileLens::Person, 0, 0)
                .unwrap();
        });
        click_painted(window, position, cx);
        assert!(desktop.read(cx).simple_transition.is_none());
        assert_eq!(desktop.read(cx).open_doc, Some(DocRef::SavedNote));
        assert!(desktop.read(cx).profile_context.graph_selection.is_none());
    });
}

#[gpui_kit::test]
fn graph_captured_callbacks_reject_reused_indexes_refresh_close_scope_and_profile_replacement(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (_fixture, handle, desktop) = window(cx, false);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        install(&desktop, page(2), window, cx);
        desktop.update(cx, |desktop, cx| {
            let original = desktop
                .ai
                .as_ref()
                .unwrap()
                .profile_context
                .context
                .as_ref()
                .unwrap()
                .clone();
            let binding = Binding::capture(desktop, &original);
            let node = Projection::new(&original).nodes[1].clone();
            let edge = original.relationships[0].clone();
            let mut replaced = original.clone();
            replaced.relationships[0] = replaced.relationships[1].clone();
            desktop.ai.as_mut().unwrap().profile_context.context = Some(replaced);
            desktop.select_graph_edge(&binding, 0, &edge, window, cx);
            desktop.select_graph_node(&binding, &node, cx);
            assert!(desktop.profile_context.graph_selection.is_none());
            assert!(desktop.simple_transition.is_none());
            desktop.ai.as_mut().unwrap().profile_context.context = Some(original.clone());
            let (id, _) = desktop
                .ai
                .as_mut()
                .unwrap()
                .inspect_profile_context(ProfileLens::Person, 0, 0)
                .unwrap();
            let loading_binding = Binding::capture(desktop, &original);
            desktop
                .ai
                .as_mut()
                .unwrap()
                .apply(id, AppEvent::ProfileContext(Box::new(original.clone())));
            desktop.select_graph_edge(&loading_binding, 1, &original.relationships[1], window, cx);
            desktop.select_graph_node(&loading_binding, &node, cx);
            assert!(desktop.profile_context.graph_selection.is_none());
            assert_eq!(desktop.profile_context.relationship, 0);
            assert!(desktop.simple_transition.is_none());
            for change in 0..5 {
                desktop.ai.as_mut().unwrap().profile_context.context = Some(original.clone());
                desktop.profile_context.open = true;
                let binding = Binding::capture(desktop, &original);
                match change {
                    0 => {
                        desktop
                            .ai
                            .as_mut()
                            .unwrap()
                            .inspect_profile_context(ProfileLens::Person, 0, 0)
                            .unwrap();
                    }
                    1 => {
                        desktop.clear_profile_panel();
                    }
                    2 => {
                        desktop
                            .ai
                            .as_mut()
                            .unwrap()
                            .select_scope(KnowledgeScope::Source);
                    }
                    3 => {
                        desktop
                            .ai
                            .as_mut()
                            .unwrap()
                            .profile_context
                            .context
                            .as_mut()
                            .unwrap()
                            .request
                            .relationship_offset = 25;
                    }
                    _ => {
                        desktop
                            .ai
                            .as_mut()
                            .unwrap()
                            .profile_context
                            .context
                            .as_mut()
                            .unwrap()
                            .profile
                            .text
                            .push_str("new exact profile");
                    }
                }
                desktop.select_graph_edge(&binding, 0, &edge, window, cx);
                desktop.select_graph_node(&binding, &node, cx);
                assert!(desktop.profile_context.graph_selection.is_none());
                assert!(desktop.simple_transition.is_none());
                desktop.ai.as_mut().unwrap().knowledge_scope = KnowledgeScope::Current;
            }
        });
    });
}
