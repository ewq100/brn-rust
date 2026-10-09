//! Real root/modal clicks; synthetic authority only, without a graphical session.
use super::*;
use crate::{
    approval::ordering_tests::{member, source},
    review::ProposalReview,
};
use gpui_kit::{VisualTestContext, test::TestWindowExt};

fn fixture(
    cx: &mut gpui_kit::TestAppContext,
    records: Vec<ProposalRecord>,
) -> (
    tempfile::TempDir,
    gpui_kit::WindowHandle<Root>,
    Entity<Desktop>,
) {
    let fixture = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = fixture.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let config = brn_workflow::app::AppConfig {
        vault_root: None,
        credentials_dir: Some(fixture.path().join("credentials")),
        model_dir: None,
    };
    cx.update(|cx| {
        gpui_kit::component::init(cx);
        cx.set_reduce_motion(true);
    });
    let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = saved.clone();
    // A tall headless viewport makes every full snapshot and control reachable.
    // Interactive sizing/usability remains a separate acceptance check.
    let window = cx.open_window(size(px(1100.), px(7000.)), move |window, cx| {
        let desktop = cx.new(|cx| {
            let mut desktop = Desktop::new(
                data,
                config,
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            );
            desktop.app_worker.take().unwrap().shutdown().unwrap();
            desktop.open_doc = Some(DocRef::Proposal(records[0].draft.id));
            desktop.centre_tab = CentreTab::Document;
            let ai = desktop.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            ai.pending.clear();
            ai.review = Some(ProposalReview::new(records[0].clone()));
            ai.proposals = records;
            desktop.sync_review_widgets(window, cx);
            desktop
        });
        *capture.borrow_mut() = Some(desktop.clone());
        desktop_root(desktop, window, cx)
    });
    let desktop = saved.borrow().clone().unwrap();
    (fixture, window, desktop)
}

fn order(window: &Window, record: &ProposalRecord, expected: usize) {
    let label = window
        .find(format!("approval-order-{}", record.draft.id))
        .label()
        .unwrap()
        .to_owned();
    assert!(
        label.starts_with(&format!(
            "Approval order {expected} · {}",
            record.draft.title
        )),
        "{label}"
    );
}

fn operation(window: &Window, id: Uuid) -> Uuid {
    Uuid::parse_str(
        window
            .find(format!("approval-operation-{id}"))
            .label()
            .unwrap()
            .strip_prefix("Approval operation ")
            .unwrap(),
    )
    .unwrap()
}

#[gpui_kit::test]
fn captured_group_widget_moves_keep_selection_ids_and_submit_latest_exact_sequence_once(
    cx: &mut gpui_kit::TestAppContext,
) {
    let group = Uuid::new_v4();
    let records = [member(group, "A"), member(group, "B"), member(group, "C")];
    let (owner, handle, desktop) = fixture(cx, records.to_vec());
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.open_approval_dialog(true, window, cx)
        })
    });
    visual.run_until_parked();
    let operations = visual.update(|window, cx| {
        window.render_frame(cx);
        let operations = records
            .iter()
            .map(|record| operation(window, record.draft.id))
            .collect::<Vec<_>>();
        window.click(format!("approve-selected-{}", records[1].draft.id), cx);
        operations
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window
                .find(format!("approve-selected-{}", records[1].draft.id))
                .checked(),
            Some(false),
            "standalone deselection must rerender without an order move"
        );
        window.click(format!("approve-selected-{}", records[1].draft.id), cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window
                .find(format!("approve-selected-{}", records[1].draft.id))
                .checked(),
            Some(true),
            "standalone reselection must rerender"
        );
        window.click(format!("approve-selected-{}", records[1].draft.id), cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window
                .find(format!("approve-selected-{}", records[1].draft.id))
                .checked(),
            Some(false)
        );
        window.click(format!("approval-move-earlier-{}", records[2].draft.id), cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        order(window, &records[0], 1);
        order(window, &records[2], 2);
        order(window, &records[1], 3);
        window.click(format!("approval-move-earlier-{}", records[2].draft.id), cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        order(window, &records[2], 1);
        order(window, &records[0], 2);
        order(window, &records[1], 3);
        window.click(format!("approval-move-later-{}", records[2].draft.id), cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        order(window, &records[0], 1);
        order(window, &records[2], 2);
        order(window, &records[1], 3);
        window.click(format!("approval-move-earlier-{}", records[2].draft.id), cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        order(window, &records[2], 1);
        order(window, &records[0], 2);
        order(window, &records[1], 3);
        assert_eq!(
            window
                .find(format!("approve-selected-{}", records[1].draft.id))
                .checked(),
            Some(false)
        );
        assert_eq!(
            window
                .find(format!("approve-selected-{}", records[2].draft.id))
                .checked(),
            Some(true)
        );
        for (record, id) in records.iter().zip(&operations) {
            assert_eq!(operation(window, record.draft.id), *id);
        }
        // A newly discovered group member never enters the already-open capture.
        let late = member(group, "Late");
        desktop.update(cx, |desktop, _| {
            desktop.ai.as_mut().unwrap().proposals.push(late.clone());
            // Restore a real local submission lane only for this click. Startup
            // events cannot replace the deliberately frozen presentation fixture.
            desktop.app_worker = Some(
                brn_workflow::app_worker::AppWorker::start(
                    owner.path().join("data"),
                    brn_workflow::app::AppConfig {
                        vault_root: None,
                        credentials_dir: Some(owner.path().join("credentials")),
                        model_dir: None,
                    },
                )
                .unwrap(),
            );
        });
        window.click("confirm-exact-proposal-approval", cx);
        let ai = desktop.read(cx).ai.as_ref().unwrap();
        let captures = ai
            .pending
            .values()
            .filter_map(|pending| match pending {
                crate::ai::Pending::Approval { capture, .. } => Some(capture),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            captures.len(),
            1,
            "one exact application request was admitted"
        );
        assert_eq!(
            captures[0].records(),
            &[records[2].clone(), records[0].clone()]
        );
        assert_eq!(captures[0].requests(), ai.approval_requests);
        assert_eq!(
            ai.approval_requests
                .iter()
                .map(|request| request.operation_id)
                .collect::<Vec<_>>(),
            vec![operations[2], operations[0]]
        );
        assert!(
            !ai.approval_requests
                .iter()
                .any(|request| request.expected.id == late.draft.id)
        );
        assert!(
            !window.has_active_dialog(cx),
            "admitted confirmation closes once"
        );
        // The synthetic records deliberately have no stored effects to apply.
        desktop.update(cx, |desktop, _| {
            desktop.app_worker.take().unwrap().shutdown().unwrap()
        });
    });
}

#[gpui_kit::test]
fn captured_group_widget_pins_sources_and_retains_order_when_blocked_or_current_changes(
    cx: &mut gpui_kit::TestAppContext,
) {
    let group = Uuid::new_v4();
    let a = member(group, "A");
    let b = member(group, "B");
    let source = source(group, "Source");
    let blocking = Uuid::new_v4();
    let (_fixture, handle, desktop) = fixture(cx, vec![a.clone(), b.clone(), source.clone()]);
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.open_approval_dialog(true, window, cx);
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        order(window, &source, 1);
        order(window, &a, 2);
        order(window, &b, 3);
        assert!(
            window
                .find(format!("approval-order-{}", source.draft.id))
                .label()
                .unwrap()
                .contains("Source runs first · pinned")
        );
        window.click(format!("approval-move-later-{}", source.draft.id), cx);
        window.click(format!("approval-move-earlier-{}", a.draft.id), cx);
        window.click(format!("approval-move-earlier-{}", b.draft.id), cx);
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        order(window, &source, 1);
        order(window, &b, 2);
        order(window, &a, 3);
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            ai.pending.insert(
                blocking,
                crate::ai::Pending::AppliedReview {
                    id: a.draft.id,
                    generation: ai.review_generation,
                },
            );
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        window.click(format!("approval-move-later-{}", b.draft.id), cx);
        window.click("confirm-exact-proposal-approval", cx);
        assert!(window.has_active_dialog(cx));
        assert!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .approval_requests
                .is_empty()
        );
        order(window, &b, 2);
        desktop.update(cx, |desktop, cx| {
            desktop.ai.as_mut().unwrap().pending.remove(&blocking);
            let mut changed = a.clone();
            changed.version += 1;
            changed.draft.title = "Changed acknowledged review".into();
            desktop.ai.as_mut().unwrap().review = Some(ProposalReview::new(changed));
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        window.click("confirm-exact-proposal-approval", cx);
        assert!(window.has_active_dialog(cx));
        let ai = desktop.read(cx).ai.as_ref().unwrap();
        assert!(ai.approval_requests.is_empty());
        assert!(ai.notice.contains("captured review changed"));
        order(window, &source, 1);
        order(window, &b, 2);
        order(window, &a, 3);
        window.close_dialog(cx);
        desktop.update(cx, |desktop, cx| {
            desktop.ai.as_mut().unwrap().review = Some(ProposalReview::new(a.clone()));
            desktop.open_approval_dialog(true, window, cx);
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        order(window, &source, 1);
        order(window, &a, 2);
        order(window, &b, 3);
        assert_eq!(
            desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .review
                .as_ref()
                .unwrap()
                .record,
            a
        );
        window.close_dialog(cx);
    });
}
