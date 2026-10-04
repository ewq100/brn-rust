use super::*;
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    editor::FileFingerprint,
    knowledge::{IdentityOutcome, NoteIdentityInfo, NoteLinks},
    proposals::{DraftNoteChange, DraftRequest, ProposalSource, SourceVersion},
};
use gpui_kit::{EntityInputHandler, TestSupportExt, VisualTestContext, test::TestWindowExt};
use std::fs;

const SAVED: &str = "\u{feff}# Saved consumer\r\n日本語 λ\r\n";

struct Fixture(tempfile::TempDir);
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        fs::create_dir(owner.path().join("data")).unwrap();
        fs::create_dir(owner.path().join("vault")).unwrap();
        fs::write(owner.path().join("vault/consumer.md"), SAVED).unwrap();
        Self(owner)
    }
    fn config(&self) -> brn_workflow::app::AppConfig {
        brn_workflow::app::AppConfig {
            vault_root: Some(self.0.path().join("vault")),
            credentials_dir: Some(self.0.path().join("credentials")),
            model_dir: None,
        }
    }
    fn unchanged(&self) {
        assert_eq!(
            fs::read(self.0.path().join("vault/consumer.md")).unwrap(),
            SAVED.as_bytes()
        );
        assert_eq!(
            fs::read_dir(self.0.path().join("vault")).unwrap().count(),
            1
        );
    }
}

fn source() -> ProposalSource {
    ProposalSource {
        source: SourceVersion {
            path: "consumer.md".into(),
            fingerprint: FileFingerprint {
                device: 741,
                inode: 852,
                len: SAVED.len() as u64,
                sha256: [0x17; 32],
            },
        },
        text: SAVED.into(),
    }
}
fn captured_form() -> crate::draft::DraftForm {
    let mut form = crate::draft::DraftForm::new(None).unwrap();
    form.edit(
        "Link to saved evidence".into(),
        "consumer.md".into(),
        SAVED.into(),
        crate::draft::DraftKind::Replace,
    );
    form.source = Some(source());
    form
}
struct DraftProbe(Entity<Desktop>);
impl Render for DraftProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |desktop, cx| {
            div()
                .id("draft-test-pane")
                .size_full()
                .flex()
                .flex_col()
                .child(desktop.render_draft(cx))
                .test_support()
        })
    }
}
fn open_form(
    cx: &mut gpui_kit::TestAppContext,
    fixture: &Fixture,
    form: crate::draft::DraftForm,
) -> (gpui_kit::WindowHandle<Root>, Entity<Desktop>) {
    cx.update(gpui_kit::component::init);
    let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
    let saved = capture.clone();
    let data = fixture.0.path().join("data");
    let config = fixture.config();
    let window = cx.open_window(
        size(px(layout::WINDOW_MIN), px(layout::WINDOW_MIN)),
        move |window, cx| {
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
                ai.draft = Some(form);
                desktop.open_doc = Some(DocRef::Draft);
                desktop.centre_tab = CentreTab::Document;
                desktop.sync_draft_widgets(window, cx);
                desktop
            });
            *saved.borrow_mut() = Some(desktop.clone());
            let probe = cx.new(|_| DraftProbe(desktop));
            Root::new(probe, window, cx)
        },
    );
    let desktop = capture.borrow().clone().unwrap();
    (window, desktop)
}
fn scroll_to(visual: &mut VisualTestContext, id: &'static str) {
    use gpui_kit::{InputEvent as _, MouseMoveEvent, ScrollDelta, ScrollWheelEvent};
    visual.update(|window, cx| {
        window.render_frame(cx);
        let pane = window.find("draft-test-pane");
        let target = window.find(id);
        let position = pane.bounds().origin + point(px(3.), px(3.));
        let dy = pane.bounds().origin.y + px(20.) - target.bounds().origin.y;
        window.dispatch_event(
            MouseMoveEvent {
                position,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        window.render_frame(cx);
        window.dispatch_event(
            ScrollWheelEvent {
                position,
                delta: ScrollDelta::Pixels(point(px(0.), dy)),
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        let target = window.find(id);
        assert!(
            target.visible() && target.bounds().center().y < window.viewport_size().height,
            "{id} unavailable at 480x480: {:?}",
            target.bounds()
        );
    });
}

#[gpui_kit::test]
fn captured_replace_exposes_link_inspection_and_preparation_in_scrolled_form(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let (window, desktop) = open_form(cx, &fixture, captured_form());
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    scroll_to(&mut visual, "inspect-initial-link-target");
    scroll_to(&mut visual, "prepare-initial-note-link");
    visual.update(|_, cx| {
        let form = desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .draft
            .as_ref()
            .unwrap();
        assert!(form.prepared_request().is_none());
        assert!(form.submitted.is_none());
        assert_eq!(form.text, SAVED);
    });
    fixture.unchanged();
}

fn prepared_form() -> (crate::draft::DraftForm, DraftRequest) {
    let consumer = source();
    let request = DraftRequest {
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Prepared 日本語 λ".into(),
        changes: vec![DraftNoteChange::Replace {
            path: consumer.source.path.clone(),
            expected: consumer.source.fingerprint.clone(),
            text: format!(
                "{SAVED}\r\n[Label λ](brn://note/22222222-2222-4222-8222-222222222222)\r\n"
            ),
        }],
        sources: vec![
            consumer.source.clone(),
            SourceVersion {
                path: "archive/資料 λ.md".into(),
                fingerprint: FileFingerprint {
                    device: 963,
                    inode: 1074,
                    len: 2185,
                    sha256: [0x29; 32],
                },
            },
        ],
    };
    let mut form = crate::draft::DraftForm::from_prepared(request.clone()).unwrap();
    form.source = Some(consumer);
    (form, request)
}

#[gpui_kit::test]
fn prepared_widgets_keep_bindings_fixed_and_copy_full_proofs_and_edited_body(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let (form, request) = prepared_form();
    let (window, desktop) = open_form(cx, &fixture, form);
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    let expected = format!(
        "Source 1\nPath: \"consumer.md\"\nDevice: 741\nInode: 852\nBytes: {}\nSHA-256: {}\n\nSource 2\nPath: \"archive/資料 λ.md\"\nDevice: 963\nInode: 1074\nBytes: 2185\nSHA-256: {}",
        SAVED.len(),
        "17".repeat(32),
        "29".repeat(32)
    );
    visual.update(|window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("prepare-initial-note-link").is_none());
        assert_eq!(
            desktop.read(cx).draft_link_proofs.read(cx).value().as_ref(),
            expected
        );
    });
    for id in [
        "initial-kind-Create",
        "initial-kind-Trash",
        "load-initial-proposal-source",
        "use-initial-captured-text",
    ] {
        scroll_to(&mut visual, id);
        visual.update(|window, cx| window.click(id, cx));
        visual.run_until_parked();
        visual.update(|_, cx| {
            let form = desktop
                .read(cx)
                .ai
                .as_ref()
                .unwrap()
                .draft
                .as_ref()
                .unwrap();
            assert_eq!(form.request().unwrap(), request);
            assert!(form.source_operation.is_none());
        });
    }
    // Real native editing retains full title/body and refuses destination changes.
    let title = "\u{feff}Later title 日本語\r\nλ\r";
    let text = "\u{feff}# Later full proposed body\r\nΕλληνικά 🦀\r\n";
    visual.update(|window, cx| {
        let fields = desktop.read(cx);
        let (title_field, path_field, body) = (
            fields.draft_title.clone(),
            fields.draft_path.clone(),
            fields.draft_editor.clone(),
        );
        title_field.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), title, window, cx)
        });
        path_field.update(cx, |input, cx| {
            input.replace_text_in_range(Some(0..0), "rebound/", window, cx)
        });
        body.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), text, window, cx)
        });
    });
    visual.run_until_parked();
    visual.update(|_, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.capture_draft_widgets(cx);
            assert_eq!(desktop.draft_path.read(cx).value().as_ref(), "consumer.md");
            let form = desktop.ai.as_ref().unwrap().draft.as_ref().unwrap();
            assert_eq!(form.source.as_ref().unwrap().text.as_bytes(), SAVED.as_bytes());
            let actual = form.request().unwrap();
            assert_eq!(actual.id, request.id);
            assert_eq!(actual.sources, request.sources);
            assert_eq!(actual.title, title);
            assert!(matches!(&actual.changes[0], DraftNoteChange::Replace {path, expected, text: actual_text}
                if path == "consumer.md" && expected == &request.sources[0].fingerprint && actual_text.as_bytes() == text.as_bytes()));
            assert!(form.submitted.is_none());
        });
    });
    scroll_to(&mut visual, "copy-prepared-source-proofs");
    visual.update(|window, cx| {
        let proof = desktop.read(cx).draft_link_proofs.clone();
        proof.update(cx, |editor, cx| {
            editor.focus(window, cx);
            editor.replace_text_in_range(Some(0..0), "blocked", window, cx);
            assert_eq!(editor.value().as_ref(), expected);
        });
        window.click("copy-prepared-source-proofs", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(expected.as_str())
        );
    });
    scroll_to(&mut visual, "copy-initial-full-body");
    visual.update(|window, cx| {
        window.click("copy-initial-full-body", cx);
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some(text)
        );
    });
    scroll_to(&mut visual, "separate-initial-proposal");
    visual.update(|window, cx| window.click("separate-initial-proposal", cx));
    visual.run_until_parked();
    visual.update(|_, cx| {
        let desktop = desktop.read(cx);
        let form = desktop.ai.as_ref().unwrap().draft.as_ref().unwrap();
        assert_ne!(form.id, request.id);
        assert_eq!(form.request().unwrap().sources, request.sources);
        assert_eq!(form.title, title);
        assert_eq!(form.text.as_bytes(), text.as_bytes());
        assert_eq!(
            desktop.draft_link_proofs.read(cx).value().as_ref(),
            expected
        );
        assert!(form.submitted.is_none());
    });
    fixture.unchanged();
}

#[gpui_kit::test]
fn native_link_inputs_and_later_typing_survive_stale_inspection_acknowledgement(
    cx: &mut gpui_kit::TestAppContext,
) {
    let fixture = Fixture::new();
    let (window, desktop) = open_form(cx, &fixture, captured_form());
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.run_until_parked();
    let target = "archive/資料 λ.md";
    let label = "日本語 [literal] *λ* 🧭";
    visual.update(|window, cx| {
        let fields = desktop.read(cx);
        let (path, label_field) = (
            fields.draft_link_target.clone(),
            fields.draft_link_label.clone(),
        );
        path.update(cx, |input, cx| {
            input.replace_text_in_range(Some(0..0), target, window, cx)
        });
        label_field.update(cx, |input, cx| {
            input.replace_text_in_range(Some(0..0), label, window, cx)
        });
    });
    visual.run_until_parked();
    let operation = visual.update(|_, cx| {
        desktop.update(cx, |desktop, cx| {
            desktop.capture_draft_widgets(cx);
            let ai = desktop.ai.as_mut().unwrap();
            assert_eq!(ai.link_preparation.target_path, target);
            assert_eq!(ai.link_preparation.label, label);
            let command = ai.inspect_link_target().unwrap();
            assert!(matches!(&command.1, AppCommand::NoteLinks(path) if path == target));
            command.0
        })
    });
    scroll_to(&mut visual, "create-initial-review-draft");
    visual.update(|window, cx| window.click("create-initial-review-draft", cx));
    visual.run_until_parked();
    visual.update(|_, cx| {
        let ai = desktop.read(cx).ai.as_ref().unwrap();
        assert_eq!(ai.link_preparation.operation, Some(operation));
        assert!(ai.pending.contains_key(&operation));
        assert!(ai.draft.as_ref().unwrap().submitted.is_none());
    });
    let later_label = format!("{label}\r\nRetained invalid second line");
    let later_body = format!("{SAVED}Later authored body õ\r\n");
    visual.update(|window, cx| {
        let fields = desktop.read(cx);
        let (label_field, body) = (fields.draft_link_label.clone(), fields.draft_editor.clone());
        label_field.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), &later_label, window, cx)
        });
        body.update(cx, |input, cx| {
            let end = input.value().encode_utf16().count();
            input.replace_text_in_range(Some(0..end), &later_body, window, cx)
        });
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        desktop.update(cx, |desktop, cx| {
            let ai = desktop.ai.as_mut().unwrap();
            assert!(
                ai.apply(
                    operation,
                    AppEvent::NoteLinks(Box::new(NoteLinks {
                        source: NoteIdentityInfo {
                            path: target.into(),
                            note_id: Some(Uuid::new_v4()),
                            sha256: [0x29; 32]
                        },
                        source_outcome: Some(IdentityOutcome::Unique),
                        links: vec![],
                        issues: vec![],
                    }))
                )
                .is_empty()
            );
            assert!(ai.link_preparation.target.is_none());
            assert_eq!(ai.link_preparation.label.as_bytes(), later_label.as_bytes());
            assert!(ai.draft.as_ref().unwrap().prepared_request().is_none());
            assert!(ai.draft.as_ref().unwrap().submitted.is_none());
            desktop.sync_draft_widgets(window, cx);
            assert_eq!(desktop.draft_link_target.read(cx).value().as_ref(), target);
            assert_eq!(
                desktop.draft_link_label.read(cx).value().as_bytes(),
                later_label.as_bytes()
            );
            assert_eq!(
                desktop.draft_editor.read(cx).value().as_bytes(),
                later_body.as_bytes()
            );
        })
    });
    fixture.unchanged();
}
