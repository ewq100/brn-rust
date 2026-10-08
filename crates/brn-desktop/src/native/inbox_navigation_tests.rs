use super::*;
use brn_workflow::{
    inbox::InboxKind,
    inbox_processing::{InboxConversionFormat, InboxSourceBinding},
    proposals::{DraftNoteChange, DraftRequest},
};
use gpui_kit::{EntityInputHandler, VisualTestContext, test::TestWindowExt};
use sha2::Digest;

fn source_request() -> DraftRequest {
    // Checked symbolic presentation input, never submitted as fresh authority.
    let text = "\u{feff}# Exact imported body õ\r\n";
    let binding: InboxSourceBinding = serde_json::from_value(serde_json::json!({
        "batch_id":Uuid::new_v4(),"index":0,"note_id":Uuid::new_v4(),
        "original":{"capture":{"id":Uuid::new_v4(),"kind":InboxKind::Markdown,
            "title":"Source õ", "original_name":"copy.md", "copy":{
                "directory":"/synthetic/inbox", "directory_device":1,"directory_inode":2,
                "file_device":1,"file_inode":3,"byte_len":text.len(),
                "sha256": <[u8;32]>::from(sha2::Sha256::digest(text.as_bytes()))}},"received_at_ms":1},
        "format":InboxConversionFormat::VerbatimMarkdownV1,"byte_len":text.len(),
        "sha256":<[u8;32]>::from(sha2::Sha256::digest(text.as_bytes())),
    })).unwrap();
    let request = DraftRequest {
        intake: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Source õ".into(),
        changes: vec![DraftNoteChange::Create {
            path: "sources/copy.md".into(),
            text: binding.markdown(text).unwrap(),
        }],
        sources: vec![],
        action_changes: vec![],
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: Some(Box::new(binding)),
    };
    request.validate().unwrap();
    request
}

#[gpui_kit::test]
fn prepared_source_opens_explicitly_and_unsubmitted_input_blocks_inbox_navigation(
    cx: &mut gpui_kit::TestAppContext,
) {
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    let vault = owner.path().join("vault");
    std::fs::create_dir(&data).unwrap();
    std::fs::create_dir(&vault).unwrap();
    let exact = "\u{feff}Untouched 日本語\r\n";
    std::fs::write(vault.join("current.md"), exact).unwrap();
    let credentials = owner.path().join("credentials");
    let request = source_request();
    let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
    let saved = capture.clone();
    cx.update(gpui_kit::component::init);
    let window = cx.open_window(
        size(px(layout::WINDOW_MIN), px(layout::WINDOW_MIN)),
        move |window, cx| {
            let desktop = cx.new(|cx| {
                let mut d = Desktop::new(
                    data,
                    brn_workflow::app::AppConfig {
                        vault_root: Some(vault),
                        credentials_dir: Some(credentials),
                        model_dir: None,
                    },
                    (LayoutState::default(), Loaded::Missing),
                    window,
                    cx,
                );
                d.app_worker.take().unwrap().shutdown().unwrap();
                let ai = d.ai.as_mut().unwrap();
                ai.ready = true;
                ai.vault_bound = true;
                ai.inbox_queue.visible = true;
                ai.inbox_queue.prepared = Some(request.clone());
                ai.pending.clear();
                assert!(ai.draft.is_none());
                d.open_doc = Some(DocRef::Inbox);
                d
            });
            *saved.borrow_mut() = Some(desktop.clone());
            Root::new(desktop, window, cx)
        },
    );
    let desktop = capture.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |d, cx| {
            let prepared = d.ai.as_ref().unwrap().inbox_queue.prepared.clone().unwrap();
            d.simple_leave(simple::EditorTransition::InboxSourceDraft, cx);
            assert_eq!(d.open_doc, Some(DocRef::Draft));
            assert!(!d.ai.as_ref().unwrap().inbox_queue.visible);
            assert_eq!(
                d.ai.as_ref()
                    .unwrap()
                    .draft
                    .as_ref()
                    .unwrap()
                    .request()
                    .unwrap(),
                prepared
            );
            d.sync_draft_widgets(window, cx);
            let title = "Retained later title 日本語";
            d.draft_title
                .update(cx, |widget, cx| widget.set_value(title, window, cx));
            d.simple_leave(simple::EditorTransition::Inbox, cx);
            assert_eq!(d.open_doc, Some(DocRef::Draft));
            assert!(d.simple_transition.is_none());
            let form = d.ai.as_ref().unwrap().draft.as_ref().unwrap();
            assert_eq!(form.title, title);
            assert_eq!(form.request().unwrap().inbox_source, prepared.inbox_source);
            assert!(d.ai.as_mut().unwrap().discard_draft());
            d.simple_leave(simple::EditorTransition::Inbox, cx);
            assert_eq!(d.open_doc, Some(DocRef::Inbox));
            assert!(d.ai.as_ref().unwrap().inbox_queue.visible);
        })
    });
    visual.update(|window, cx| desktop.update(cx, |d, cx| {
        let prepared = source_request();
        let DraftNoteChange::Create { path, text } = &prepared.changes[0] else { panic!("source Create") };
        let record = serde_json::from_value(serde_json::json!({
            "draft": {"id":prepared.id,"group_id":null,"session_id":null,
                "vault":{"id":Uuid::new_v4(),"root":"/synthetic/vault","identity":{"device":1,"inode":4}},
                "title":prepared.title,"changes":[{"kind":"create","path":path,"parent":{"device":1,"inode":5},"text":text}],
                "sources":[],"inbox_source":prepared.inbox_source},
            "version":1,"state":"draft","comments":[],"created_at_ms":1,"updated_at_ms":1,
        })).unwrap();
        d.ai.as_mut().unwrap().pending.clear();
        d.ai.as_mut().unwrap().review = Some(crate::review::ProposalReview::new(record));
        d.open_doc = Some(DocRef::Proposal(prepared.id));
        d.sync_review_widgets(window,cx);
        cx.notify();
    }));
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let editor = desktop.read(cx).review_editor.clone();
        editor.update(cx, |editor, cx| {
            let exact = editor.value().to_string();
            editor.replace_text_in_range(Some(0..0), "source rewrite", window, cx);
            assert_eq!(
                editor.value().as_bytes(),
                exact.as_bytes(),
                "Source review cannot edit converted bytes"
            );
        });
    });
    assert_eq!(
        std::fs::read(owner.path().join("vault/current.md")).unwrap(),
        exact.as_bytes()
    );
    assert!(!owner.path().join("vault/sources/copy.md").exists());
}
