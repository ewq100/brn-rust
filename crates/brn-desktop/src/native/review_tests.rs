//! Real native text-buffer to shared review persistence; no graphical or provider calls.
#![cfg(target_os = "macos")]

use crate::{ai::AiState, review::selection_target};
use brn_workflow::{
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    editor::EditorView,
    proposals::{
        CommentTarget, DraftNoteChange, DraftRequest, NoteChange, ReviewComment, SourceVersion,
    },
};
use gpui_kit::component::input::{Rope, RopeExt};
use std::{
    collections::VecDeque,
    fs,
    time::{Duration, Instant},
};
use uuid::Uuid;

const ORIGINALS: [(&str, &str); 3] = [
    (
        "replace.md",
        "\u{feff}---\r\ntitle: café 🧭\r\n---\r\n正文 λ\r\n",
    ),
    ("trash.md", "\u{feff}Retained 日本語 🦀\r\n"),
    ("source.md", "\u{feff}Source Ελληνικά λ\r\n"),
];

struct Fixture(tempfile::TempDir);

impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        fs::create_dir(owner.path().join("data")).unwrap();
        fs::create_dir(owner.path().join("vault")).unwrap();
        for (path, text) in ORIGINALS {
            fs::write(owner.path().join("vault").join(path), text).unwrap();
        }
        Self(owner)
    }

    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(
            self.0.path().join("data"),
            AppConfig {
                vault_root: Some(self.0.path().join("vault")),
                credentials_dir: Some(self.0.path().join("credentials")),
                model_dir: None,
            },
        )
        .unwrap();
        loop {
            match worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1
            {
                AppEvent::Ready { .. } => return worker,
                AppEvent::Failed(error) => panic!("startup: {:?}: {}", error.kind, error.message),
                _ => {}
            }
        }
    }

    fn assert_vault_unchanged(&self) {
        let vault = self.0.path().join("vault");
        for (path, text) in ORIGINALS {
            assert_eq!(fs::read(vault.join(path)).unwrap(), text.as_bytes());
        }
        assert!(!vault.join("new.md").exists());
        assert_eq!(fs::read_dir(vault).unwrap().count(), ORIGINALS.len());
    }
}

fn reply(worker: &AppWorker, command: (Uuid, AppCommand)) -> (Uuid, AppEvent) {
    let (id, command) = command;
    worker.submit(id, command).unwrap();
    loop {
        let event = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if event.0 == id {
            if let AppEvent::Failed(error) = &event.1 {
                panic!("review command: {:?}: {}", error.kind, error.message);
            }
            return event;
        }
    }
}

fn editor(worker: &AppWorker, path: &str) -> EditorView {
    let (_, event) = reply(
        worker,
        (Uuid::new_v4(), AppCommand::OpenEditor(path.into())),
    );
    let AppEvent::Editor(view) = event else {
        panic!("editor response");
    };
    view
}

fn ready() -> AiState {
    let mut state = AiState::default();
    state.ready = true;
    state.vault_bound = true;
    state
}

fn acknowledge(worker: &AppWorker, state: &mut AiState, event: (Uuid, AppEvent)) {
    let mut events = VecDeque::from([event]);
    while let Some((id, event)) = events.pop_front() {
        for command in state.apply(id, event) {
            assert!(
                matches!(command.1, AppCommand::Proposals(_)),
                "only local review-list refresh is expected"
            );
            events.push_back(reply(worker, command));
        }
    }
}

#[test]
fn gpui_rope_full_review_and_exact_comment_reattachment_survive_worker_restart() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let replace = editor(&worker, "replace.md").observed.unwrap();
    let trash = editor(&worker, "trash.md").observed.unwrap();
    let source = editor(&worker, "source.md").observed.unwrap();
    let proposal = Uuid::new_v4();
    let (_, created) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::CreateProposal(DraftRequest {
                inbox_visual: None,
                inbox_knowledge: None,
                inbox_source: None,
                action_changes: Vec::new(),
                id: proposal,
                group_id: None,
                session_id: None,
                title: "Full review 🧭".into(),
                changes: vec![
                    DraftNoteChange::Create {
                        path: "new.md".into(),
                        text: "\u{feff}# 草案\r\n同じ λ / 同じ λ\r\n".into(),
                    },
                    DraftNoteChange::Replace {
                        path: "replace.md".into(),
                        expected: replace.clone(),
                        text: "\u{feff}# 替换\r\n保持 café 🧭\r\n".into(),
                    },
                    DraftNoteChange::Trash {
                        path: "trash.md".into(),
                        expected: trash.clone(),
                    },
                ],
                sources: vec![SourceVersion {
                    path: "source.md".into(),
                    fingerprint: source.clone(),
                }],
            }),
        ),
    );
    let AppEvent::Proposal(created) = created else {
        panic!("created review");
    };
    fixture.assert_vault_unchanged();

    let mut state = ready();
    let open = state.open_review(proposal).unwrap();
    acknowledge(&worker, &mut state, reply(&worker, open));
    let mut create_rope = Rope::from(state.review.as_ref().unwrap().text(0).unwrap());
    let start = create_rope.to_string().find("草案").unwrap();
    create_rope.replace(start..start + "草案".len(), "文書 🦀");
    let mut replace_rope = Rope::from(state.review.as_ref().unwrap().text(1).unwrap());
    let start = replace_rope.to_string().find("保持").unwrap();
    replace_rope.replace(start..start + "保持".len(), "明確");
    let mut title_rope = Rope::from(state.review.as_ref().unwrap().title());
    let end = title_rope.to_string().len();
    title_rope.replace(end..end, "\r\n全体 λ");
    let full_texts = vec![
        Some(create_rope.to_string()),
        Some(replace_rope.to_string()),
        None,
    ];
    let review = state.review.as_mut().unwrap();
    review
        .edit_title(title_rope.to_string(), Instant::now())
        .unwrap();
    review
        .edit_text(0, full_texts[0].clone().unwrap(), Instant::now())
        .unwrap();
    review
        .edit_text(1, full_texts[1].clone().unwrap(), Instant::now())
        .unwrap();
    assert!(!state.review_can_leave() && !state.review_can_mutate());
    let edit = state.recover_review().unwrap();
    let AppCommand::EditProposal(submitted) = &edit.1 else {
        panic!("full proposal edit");
    };
    assert_eq!(submitted.expected, created.stamp());
    assert_eq!(submitted.title, title_rope.to_string());
    assert_eq!(submitted.texts, full_texts);
    acknowledge(&worker, &mut state, reply(&worker, edit));
    assert!(state.review_can_leave() && state.review_can_mutate());
    let review = state.review.as_ref().unwrap();
    assert_eq!(review.texts(), full_texts);
    assert_eq!(review.title(), title_rope.to_string());
    assert_eq!(review.record.draft.sources, created.draft.sources);
    assert!(
        matches!(&review.record.draft.changes[1], NoteChange::Replace { before_text, before, .. } if before_text == ORIGINALS[0].1 && before == &replace)
    );
    assert_eq!(review.record.draft.changes[2], created.draft.changes[2]);
    fixture.assert_vault_unchanged();

    // Select the second identical quote using the native buffer's UTF-8 byte offsets.
    let text = create_rope.to_string();
    let quote = "同じ λ";
    let start = text.rfind(quote).unwrap();
    assert!(start > text[..start].chars().count());
    let target = selection_target(review, 0, start..start + quote.len()).unwrap();
    let CommentTarget::Text(old_anchor) = target.clone() else {
        panic!("exact text anchor");
    };
    assert_eq!(old_anchor.quote, quote);
    let comment = ReviewComment {
        id: Uuid::new_v4(),
        text: "\u{feff}Temporary 日本語 λ\r\n".into(),
        target,
    };
    let command = state.review_comment(comment.clone(), false).unwrap();
    acknowledge(&worker, &mut state, reply(&worker, command));
    assert_eq!(
        state.review.as_ref().unwrap().record.comments,
        vec![comment.clone()]
    );

    create_rope.replace(old_anchor.start..old_anchor.end, "改訂 λ");
    create_rope.replace("\u{feff}".len().."\u{feff}".len(), "追記 🧭\r\n");
    let changed = create_rope.to_string();
    assert!(
        changed.contains(quote),
        "another old quote remains; it must never be guessed as the target"
    );
    state
        .review
        .as_mut()
        .unwrap()
        .edit_text(0, changed.clone(), Instant::now())
        .unwrap();
    let edit = state.recover_review().unwrap();
    acknowledge(&worker, &mut state, reply(&worker, edit));
    let unresolved = &state.review.as_ref().unwrap().record.comments[0];
    assert_eq!(unresolved.id, comment.id);
    assert_eq!(unresolved.text, comment.text);
    assert_eq!(
        unresolved.target,
        CommentTarget::Unresolved(old_anchor.clone())
    );
    fixture.assert_vault_unchanged();

    let new_quote = "改訂 λ";
    let start = changed.find(new_quote).unwrap();
    assert_ne!(start, old_anchor.start);
    let target = selection_target(
        state.review.as_ref().unwrap(),
        0,
        start..start + new_quote.len(),
    )
    .unwrap();
    let reattached = ReviewComment { target, ..comment };
    let command = state.review_comment(reattached.clone(), true).unwrap();
    acknowledge(&worker, &mut state, reply(&worker, command));
    let complete = state.review.as_ref().unwrap().record.clone();
    assert_eq!(complete.comments, vec![reattached]);
    assert!(
        matches!(&complete.comments[0].target, CommentTarget::Text(anchor) if anchor.start == start && anchor.end == start + new_quote.len() && anchor.quote == new_quote)
    );
    assert_eq!(complete.draft.changes[0].text(), Some(changed.as_str()));
    assert_eq!(complete.draft.changes[1].text(), full_texts[1].as_deref());
    assert_eq!(complete.draft.changes[2], created.draft.changes[2]);
    fixture.assert_vault_unchanged();
    worker.shutdown().unwrap();

    let mut worker = fixture.worker();
    let mut reopened = ready();
    let open = reopened.open_review(proposal).unwrap();
    acknowledge(&worker, &mut reopened, reply(&worker, open));
    assert_eq!(reopened.review.as_ref().unwrap().record, complete);
    assert!(reopened.review_can_leave() && reopened.review_can_mutate());
    assert_eq!(editor(&worker, "replace.md").observed, Some(replace));
    assert_eq!(editor(&worker, "trash.md").observed, Some(trash));
    assert_eq!(editor(&worker, "source.md").observed, Some(source));
    fixture.assert_vault_unchanged();
    worker.shutdown().unwrap();
}

#[cfg(feature = "native-test-support")]
#[gpui_kit::test]
fn actual_title_widget_preserves_multiline_bytes_through_loading_typing_and_worker_restart(
    cx: &mut gpui_kit::TestAppContext,
) {
    use gpui_kit::{AppContext, EntityInputHandler};

    const LOADED: &str = "\u{feff}初期 日本語 🧭\r\n第二 λ\n第三\r";
    const APPENDED: &str = "追加 café 🦀\r\n";
    cx.update(gpui_kit::component::init);
    let window = cx.add_window(|_, _| gpui_kit::Empty);
    let title = cx
        .update_window(window.into(), |_, window, cx| {
            // Positive control reproduces the former widget's destructive normalization.
            let single = cx.new(|cx| {
                gpui_kit::component::input::InputState::new(window, cx).default_value(LOADED)
            });
            assert_ne!(single.read(cx).value().as_ref(), LOADED);
            assert_eq!(
                single.read(cx).value().as_ref(),
                LOADED.replace(['\n', '\r'], "")
            );
            let title =
                cx.new(|cx| super::super::review_title_state(window, cx).default_value(LOADED));
            assert_eq!(title.read(cx).value().as_ref(), LOADED);
            title.update(cx, |title, cx| {
                title.set_value("temporary", window, cx);
                title.set_value(LOADED, window, cx);
                assert_eq!(title.value().as_ref(), LOADED);
                let end = LOADED.encode_utf16().count();
                // Use the same native input-handler edit path as ordinary typing.
                title.replace_text_in_range(Some(end..end), APPENDED, window, cx);
                assert_eq!(title.value().as_ref(), format!("{LOADED}{APPENDED}"));
            });
            title
        })
        .unwrap();
    let edited = cx.read(|cx| title.read(cx).value().to_string());

    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let proposal = Uuid::new_v4();
    let (_, event) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::CreateProposal(DraftRequest {
                inbox_visual: None,
                inbox_knowledge: None,
                inbox_source: None,
                action_changes: Vec::new(),
                id: proposal,
                group_id: None,
                session_id: None,
                title: LOADED.into(),
                changes: vec![DraftNoteChange::Create {
                    path: "new.md".into(),
                    text: "\u{feff}本文 λ\r\n".into(),
                }],
                sources: vec![],
            }),
        ),
    );
    let AppEvent::Proposal(created) = event else {
        panic!("created review");
    };
    assert_eq!(created.draft.title.as_bytes(), LOADED.as_bytes());
    let mut state = ready();
    let open = state.open_review(proposal).unwrap();
    acknowledge(&worker, &mut state, reply(&worker, open));
    assert_eq!(state.review.as_ref().unwrap().title(), LOADED);
    state
        .review
        .as_mut()
        .unwrap()
        .edit_title(edited.clone(), Instant::now())
        .unwrap();
    let edit = state.recover_review().unwrap();
    let AppCommand::EditProposal(submitted) = &edit.1 else {
        panic!("full title edit");
    };
    assert_eq!(submitted.expected, created.stamp());
    assert_eq!(
        submitted.title.as_bytes(),
        format!("{LOADED}{APPENDED}").as_bytes()
    );
    assert_eq!(submitted.texts, vec![Some("\u{feff}本文 λ\r\n".into())]);
    acknowledge(&worker, &mut state, reply(&worker, edit));
    let complete = state.review.as_ref().unwrap().record.clone();
    assert_eq!(complete.draft.title.as_bytes(), edited.as_bytes());
    assert!(state.review_can_leave());
    fixture.assert_vault_unchanged();
    worker.shutdown().unwrap();

    let mut worker = fixture.worker();
    let mut state = ready();
    let open = state.open_review(proposal).unwrap();
    acknowledge(&worker, &mut state, reply(&worker, open));
    assert_eq!(state.review.as_ref().unwrap().record, complete);
    cx.update_window(window.into(), |_, window, cx| {
        title.update(cx, |title, cx| {
            title.set_value(
                state.review.as_ref().unwrap().title().to_owned(),
                window,
                cx,
            )
        });
        assert_eq!(
            title.read(cx).value().as_ref().as_bytes(),
            edited.as_bytes()
        );
    })
    .unwrap();
    fixture.assert_vault_unchanged();
    worker.shutdown().unwrap();
}
