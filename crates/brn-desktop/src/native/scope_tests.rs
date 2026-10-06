use super::*;
use brn_workflow::{
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    library::KnowledgeScope,
    proposals::{DraftNoteChange, DraftRequest},
};
use std::{fs, time::Instant};

const SOURCE: &str = "\u{feff}---\r\nbrn_kind: source\r\n---\r\nOriginal 日本語 🧭\nlast λ\r";

struct Fixture(tempfile::TempDir);
impl Fixture {
    fn new() -> Self {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        fs::create_dir(base.path().join("data")).unwrap();
        fs::create_dir(base.path().join("vault")).unwrap();
        fs::write(base.path().join("vault/source.md"), SOURCE).unwrap();
        fs::write(base.path().join("vault/current.md"), "Current\r\n").unwrap();
        Self(base)
    }
    fn config(&self) -> AppConfig {
        AppConfig {
            vault_root: Some(self.0.path().join("vault")),
            credentials_dir: Some(self.0.path().join("credentials")),
            model_dir: None,
        }
    }
    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(self.0.path().join("data"), self.config()).unwrap();
        loop {
            match worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1
            {
                AppEvent::Ready { .. } => return worker,
                AppEvent::Failed(error) => panic!("startup: {}", error.message),
                _ => {}
            }
        }
    }
    fn unchanged(&self) {
        assert_eq!(
            fs::read(self.0.path().join("vault/source.md")).unwrap(),
            SOURCE.as_bytes()
        );
        assert_eq!(
            fs::read(self.0.path().join("vault/current.md")).unwrap(),
            b"Current\r\n"
        );
    }
}
fn reply(worker: &AppWorker, command: (Uuid, AppCommand)) -> (Uuid, AppEvent) {
    let (id, command) = command;
    worker.submit(id, command).unwrap();
    loop {
        let (actual, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if actual == id {
            if let AppEvent::Failed(error) = &event {
                panic!("command: {}", error.message);
            }
            return (id, event);
        }
    }
}

struct EvidenceProbe(Entity<EditorState>);
impl Render for EvidenceProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(simple::evidence_widget(&self.0))
    }
}

#[gpui_kit::test]
fn actual_evidence_widget_preserves_worker_bytes_and_refuses_typing_but_allows_copy(
    cx: &mut gpui_kit::TestAppContext,
) {
    use gpui_kit::{EntityInputHandler, VisualTestContext};
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let (_, event) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::ScopedNote {
                scope: KnowledgeScope::Source,
                path: "source.md".into(),
            },
        ),
    );
    let AppEvent::Note(note) = event else {
        panic!("exact evidence");
    };
    assert_eq!(note.text.as_bytes(), SOURCE.as_bytes());
    cx.update(gpui_kit::component::init);
    let captured = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let editor = cx.new(|cx| EditorState::new(window, cx).default_value(note.text));
        *capture.borrow_mut() = Some(editor.clone());
        let probe = cx.new(|_| EvidenceProbe(editor));
        Root::new(probe, window, cx)
    });
    let editor = captured.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.draw(cx).clear(cx);
        editor.update(cx, |editor, cx| {
            editor.focus(window, cx);
            assert_eq!(editor.value().as_ref(), SOURCE);
            let end = SOURCE.encode_utf16().count();
            editor.replace_text_in_range(Some(end..end), "blocked mutation", window, cx);
            assert_eq!(editor.value().as_ref(), SOURCE);
        });
        window.dispatch_action(Box::new(gpui_kit::component::input::SelectAll), cx);
        window.dispatch_action(Box::new(gpui_kit::component::input::Copy), cx);
    });
    visual.run_until_parked();
    assert_eq!(
        visual
            .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
            .as_deref(),
        Some(SOURCE)
    );
    let (_, event) = reply(&worker, (Uuid::new_v4(), AppCommand::Editors));
    assert!(matches!(event, AppEvent::Editors(editors) if editors.is_empty()));
    fixture.unchanged();
    worker.shutdown().unwrap();
}

#[gpui_kit::test]
fn native_evidence_navigation_keeps_editor_review_comment_and_initial_input_guards(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let (_, view) = reply(
        &worker,
        (Uuid::new_v4(), AppCommand::OpenEditor("current.md".into())),
    );
    let AppEvent::Editor(view) = view else {
        panic!("editor");
    };
    let (_, record) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::CreateProposal(DraftRequest {
                inbox_visual: None,
                inbox_knowledge: None,
                inbox_source: None,
                action_changes: Vec::new(),
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Full review".into(),
                changes: vec![DraftNoteChange::Create {
                    path: "proposed.md".into(),
                    text: "Proposed\r\n".into(),
                }],
                sources: vec![],
            }),
        ),
    );
    let AppEvent::Proposal(record) = record else {
        panic!("proposal");
    };
    worker.shutdown().unwrap();
    cx.update(gpui_kit::component::init);
    let window = cx.add_window(|_, _| gpui_kit::Empty);
    let desktop = cx
        .update_window(window.into(), |_, window, cx| {
            cx.new(|cx| {
                Desktop::new(
                    fixture.0.path().join("data"),
                    fixture.config(),
                    (LayoutState::default(), Loaded::Missing),
                    window,
                    cx,
                )
            })
        })
        .unwrap();
    cx.update_window(window.into(), |_, window, cx| {
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            ai.ready = true;
            ai.vault_bound = true;
            let (id, _) = ai.open_editor("current.md".into());
            ai.apply(id, AppEvent::Editor(view));
            ai.editor
                .as_mut()
                .unwrap()
                .edit("Unacknowledged λ\r\n".into(), Instant::now())
                .unwrap();
            desktop.simple_note_path = Some("current.md".into());
            desktop.open_doc = Some(DocRef::SavedNote);
            desktop.simple_scoped_note("source.md".into(), KnowledgeScope::Source, cx);
            assert!(matches!(
                desktop.simple_transition,
                Some(simple::EditorTransition::Evidence { .. })
            ));
            assert_eq!(desktop.open_doc, Some(DocRef::SavedNote));
            assert!(desktop.ai.as_ref().unwrap().evidence.is_none());
            let recovery = desktop.ai.as_mut().unwrap().recover_editor().unwrap();
            let (id, event) = reply(desktop.app_worker.as_ref().unwrap(), recovery);
            desktop.ai.as_mut().unwrap().apply(id, event);
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
            assert!(desktop.ai.as_ref().unwrap().editor.is_none());

            desktop.simple_transition = None;
            desktop.ai.as_mut().unwrap().evidence = None;
            let mut review = crate::review::ProposalReview::new(record);
            review
                .edit_text(0, "Local review λ".into(), Instant::now())
                .unwrap();
            desktop.ai.as_mut().unwrap().review = Some(review);
            desktop.simple_scoped_note("source.md".into(), KnowledgeScope::All, cx);
            assert!(matches!(
                desktop.simple_transition,
                Some(simple::EditorTransition::Evidence {
                    scope: KnowledgeScope::All,
                    ..
                })
            ));
            assert!(desktop.ai.as_ref().unwrap().evidence.is_none());

            desktop.simple_transition = None;
            desktop.ai.as_mut().unwrap().review = None;
            desktop.review_comment_draft = Some((Uuid::new_v4(), None));
            desktop.review_comment.update(cx, |editor, cx| {
                editor.set_value("Retained comment λ", window, cx)
            });
            desktop.simple_scoped_note("source.md".into(), KnowledgeScope::Source, cx);
            assert!(desktop.ai.as_ref().unwrap().evidence.is_none());
            assert!(desktop.simple_transition.is_some());

            desktop.simple_transition = None;
            desktop.review_comment_draft = None;
            let mut draft = crate::draft::DraftForm::new(None).unwrap();
            draft.edit(
                "Retained initial title".into(),
                "future.md".into(),
                "Full input λ".into(),
                crate::draft::DraftKind::Create,
            );
            desktop.ai.as_mut().unwrap().draft = Some(draft);
            desktop.simple_scoped_note("source.md".into(), KnowledgeScope::Source, cx);
            assert!(desktop.simple_transition.is_none());
            assert!(desktop.ai.as_ref().unwrap().evidence.is_none());
            assert_eq!(
                desktop.ai.as_ref().unwrap().draft.as_ref().unwrap().text,
                "Full input λ"
            );

            desktop.ai.as_mut().unwrap().draft = None;
            desktop.ai.as_mut().unwrap().knowledge_scope = KnowledgeScope::Source;
            desktop.simple_note("current.md".into(), cx);
            assert_eq!(desktop.open_doc, Some(DocRef::SavedNote));
            assert!(
                desktop
                    .ai
                    .as_ref()
                    .unwrap()
                    .pending
                    .values()
                    .any(|pending| matches!(pending, crate::ai::Pending::Editor { .. }))
            );
            desktop.app_worker.as_mut().unwrap().shutdown().unwrap();
        });
    })
    .unwrap();
    fixture.unchanged();
}
