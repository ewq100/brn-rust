//! Actual asset proof widgets and exact captured approval; no GUI/provider calls.
use super::super::*;
use crate::review::{ProposalReview, asset_tests::fixture};
use brn_workflow::proposals::NoteChange;
use gpui_kit::{
    EntityInputHandler, TestSupportExt, VisualTestContext, component::WindowExt,
    test::TestWindowExt,
};
use sha2::{Digest, Sha256};

struct ReviewProbe(Entity<Desktop>);
impl Render for ReviewProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(desktop.render_proposal_review(cx))
                .test_support()
        })
    }
}

fn window(
    cx: &mut gpui_kit::TestAppContext,
    record: brn_workflow::proposals::ProposalRecord,
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
    let handle = cx.open_window(size(px(1100.), px(1000.)), move |window, cx| {
        let desktop = cx.new(|cx| {
            let mut desktop = Desktop::new(
                data,
                config,
                (LayoutState::default(), Loaded::Missing),
                window,
                cx,
            );
            desktop.app_worker.take().unwrap().shutdown().unwrap();
            let ai = desktop.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            ai.review = Some(ProposalReview::new(record));
            desktop.review_member = 1;
            desktop.sync_review_widgets(window, cx);
            desktop
        });
        *capture.borrow_mut() = Some(desktop.clone());
        let probe = cx.new(|_| ReviewProbe(desktop));
        Root::new(probe, window, cx)
    });
    let desktop = saved.borrow().clone().unwrap();
    (fixture, handle, desktop)
}

fn hex(hash: &[u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn assert_proof(window: &Window, scope: &str, change: &NoteChange) {
    assert!(window.try_find(format!("{scope}-proof")).is_some());
    let kind = match change {
        NoteChange::CreateAsset { .. } => "Create asset",
        NoteChange::ReplaceAsset { .. } => "Replace asset",
        NoteChange::TrashAsset { .. } => "Move asset to Trash",
        _ => panic!("typed asset fixture"),
    };
    assert_eq!(window.find(format!("{scope}-kind")).label(), Some(kind));
    assert_eq!(
        window.find(format!("{scope}-destination")).label(),
        Some(format!("Destination: {}", change.path()).as_str())
    );
    assert_eq!(
        window.find(format!("{scope}-parent")).label(),
        Some(
            format!(
                "Captured parent · device {} · inode {}",
                change.parent().device,
                change.parent().inode
            )
            .as_str()
        )
    );
    let before = change.before().map_or_else(
        || "Before: no asset at the captured destination".to_owned(),
        |proof| {
            format!(
                "Captured before · device {} · inode {} · {} bytes · SHA-256 {}",
                proof.device,
                proof.inode,
                proof.len,
                hex(&proof.sha256)
            )
        },
    );
    assert_eq!(
        window.find(format!("{scope}-before")).label(),
        Some(before.as_str())
    );
    let candidate = change.candidate_bytes().map_or_else(
        || "Proposed: move the exact captured asset to recoverable Trash".to_owned(),
        |bytes| {
            format!(
                "Exact proposed asset · {} bytes · SHA-256 {}",
                bytes.len(),
                hex(&Sha256::digest(bytes).into())
            )
        },
    );
    assert_eq!(
        window.find(format!("{scope}-candidate")).label(),
        Some(candidate.as_str())
    );
}

#[gpui_kit::test]
fn native_assets_show_exact_proofs_without_editor_or_selection_controls(
    cx: &mut gpui_kit::TestAppContext,
) {
    let record = fixture();
    let (_fixture, handle, desktop) = window(cx, record.clone());
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    for index in 1..4 {
        visual.update(|window, cx| {
            desktop.update(cx, |desktop, cx| {
                desktop.review_member = index;
                desktop.sync_review_widgets(window, cx);
                cx.notify();
            })
        });
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
            assert_proof(
                window,
                &format!("review-asset-{index}"),
                &record.draft.changes[index],
            );
            assert!(window.try_find("review-text-editor").is_none());
            assert!(window.try_find("review-comment-selection").is_none());
            assert!(
                window
                    .try_find(format!("reattach-comment-{}", record.comments[0].id))
                    .is_none()
            );
            assert!(desktop.read(cx).review_editor.read(cx).value().is_empty());
        });
    }
    visual.update(|window, cx| {
        let input = desktop.read(cx).review_title.clone();
        input.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), "Owner asset title", window, cx);
        });
    });
    visual.run_until_parked();
    visual.update(|_, cx| {
        let desktop = desktop.read(cx);
        let review = desktop.ai.as_ref().unwrap().review.as_ref().unwrap();
        assert_eq!(review.title(), "Owner asset title");
        assert_eq!(review.texts().len(), 4);
        assert!(review.texts()[1..].iter().all(Option::is_none));
        assert_eq!(review.record.draft.changes, record.draft.changes);
    });
}

#[gpui_kit::test]
fn exact_asset_approval_keeps_captured_candidate_and_full_before_proof(
    cx: &mut gpui_kit::TestAppContext,
) {
    let mut record = fixture();
    record.draft.changes = vec![record.draft.changes[2].clone()];
    let (_fixture, handle, desktop) = window(cx, record.clone());
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.open_approval_dialog(false, window, cx)
        })
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        assert!(window.has_active_dialog(cx));
        desktop.update(cx, |desktop, cx| {
            let mut later = record.clone();
            later.version += 1;
            if let NoteChange::ReplaceAsset { bytes, .. } = &mut later.draft.changes[0] {
                bytes.push(10);
            }
            desktop.ai.as_mut().unwrap().review = Some(ProposalReview::new(later));
            cx.notify();
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert_proof(window, "captured-asset-0", &record.draft.changes[0]);
        assert!(window.try_find("review-text-editor").is_none());
        window.close_dialog(cx);
    });
}
