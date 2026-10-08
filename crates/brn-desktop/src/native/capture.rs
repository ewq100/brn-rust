//! Design-handbook captures of the real `Desktop` view tree.
//!
//! This renders the production views over a seeded synthetic workspace with the
//! pinned toolkit's headless Metal renderer and real macOS text shaping. It is
//! visual evidence of implemented appearance only: it is not native window,
//! keyboard, accessibility or owner qualification. Compiled only with the
//! development `native-capture` feature and run on the main thread when
//! `BRN_DESIGN_CAPTURE=1`; `scripts/design-capture.sh` seeds the workspace and sets:
//!
//! - `BRN_CAPTURE_DATA` / `BRN_CAPTURE_VAULT`: a seeded disposable workspace;
//! - `BRN_CAPTURE_OUT`: an existing directory that receives PNG files;
//! - `BRN_CAPTURE_PREFIX`: optional file-name prefix (for before/after sets).
use super::*;
use crate::layout::{Appearance, Loaded};
use gpui_kit::{AnyWindowHandle, HeadlessAppContext, test::TestWindowExt};
use std::{cell::RefCell, path::Path, rc::Rc, sync::Arc};

// Field order matters: entity handles must drop before the app context.
struct Capture {
    desktop: Entity<Desktop>,
    window: AnyWindowHandle,
    out: PathBuf,
    prefix: String,
    cx: HeadlessAppContext,
}

impl Capture {
    fn open(data: &Path, vault: &Path, appearance: Appearance, width: f32, height: f32) -> Self {
        let platform = gpui_kit::platform::current_platform(true);
        let mut cx = HeadlessAppContext::with_platform(
            platform.text_system(),
            Arc::new(gpui_kit::assets::Assets),
            gpui_kit::platform::current_headless_renderer,
        );
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_reduce_motion(true);
        });
        let saved = Rc::new(RefCell::new(None));
        let captured = saved.clone();
        let data = data.to_path_buf();
        let vault = vault.to_path_buf();
        // Matches a first run of the app: minimal, vault rail closed.
        let layout = LayoutState {
            appearance,
            vault_collapsed: true,
            ..LayoutState::default()
        };
        let handle = cx
            .open_window(size(px(width), px(height)), move |window, cx| {
                let desktop = cx.new(|cx| {
                    Desktop::new(
                        data,
                        brn_workflow::app::AppConfig {
                            vault_root: Some(vault),
                            credentials_dir: None,
                            model_dir: None,
                        },
                        (layout, Loaded::Missing),
                        window,
                        cx,
                    )
                });
                *captured.borrow_mut() = Some(desktop.clone());
                cx.new(|cx| desktop_root(desktop, window, cx))
            })
            .expect("headless capture window");
        let desktop = saved.borrow().clone().unwrap();
        let out = PathBuf::from(std::env::var("BRN_CAPTURE_OUT").expect("BRN_CAPTURE_OUT"));
        let prefix = std::env::var("BRN_CAPTURE_PREFIX").unwrap_or_default();
        let mut capture = Self {
            cx,
            window: handle.into(),
            desktop,
            out,
            prefix,
        };
        capture.settle(|desktop| {
            let ai = desktop.ai.as_ref().unwrap();
            ai.ready && ai.vault_bound && !ai.notes.is_empty()
        });
        capture
    }

    /// Lets the real AppWorker thread answer while advancing the poll timer.
    fn settle(&mut self, done: impl Fn(&Desktop) -> bool) {
        for _ in 0..300 {
            self.cx.advance_clock(Duration::from_millis(60));
            self.cx.run_until_parked();
            let desktop = self.desktop.clone();
            if self.cx.update(|cx| done(desktop.read(cx))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        for _ in 0..12 {
            self.cx.advance_clock(Duration::from_millis(60));
            self.cx.run_until_parked();
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn act(&mut self, f: impl FnOnce(&mut Desktop, &mut Window, &mut Context<Desktop>)) {
        let desktop = self.desktop.clone();
        self.cx
            .update_window(self.window, |_, window, cx| {
                desktop.update(cx, |this, cx| f(this, window, cx))
            })
            .unwrap();
    }

    fn click(&mut self, id: &'static str) {
        self.cx
            .update_window(self.window, |_, window, cx| {
                window.render_frame(cx);
                window.click(id, cx);
                window.render_frame(cx);
                window.dispatch_event(
                    gpui_kit::PlatformInput::MouseMove(gpui_kit::MouseMoveEvent {
                        position: point(px(600.), px(6.)),
                        pressed_button: None,
                        modifiers: Default::default(),
                    }),
                    cx,
                );
            })
            .unwrap();
    }

    fn shot(&mut self, name: &str) {
        self.cx
            .update_window(self.window, |_, window, cx| window.render_frame(cx))
            .unwrap();
        let image = self.cx.capture_screenshot(self.window).expect("capture");
        let path = self.out.join(format!("{}{name}.png", self.prefix));
        image.save(&path).expect("save capture");
        eprintln!("captured {}", path.display());
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

fn seed_conversation(data: &Path) {
    let (mut store, _) = brn_store::work::WorkStore::open(data).expect("work store");
    let first = Uuid::from_u128(0x5e2a_c4a7_0000_4000_8000_0000_0000_0001);
    let conversation = store
        .begin_turn_with_effort(
            first,
            None,
            "Are there any open actions for Serna?",
            "chatgpt",
            "gpt-6.1-sol",
            Some("medium"),
        )
        .expect("begin turn")
        .conversation_id;
    store
        .finish_turn(
            first,
            brn_workflow::WorkTurnStatus::Completed,
            "Three Serna actions are unfinished:\n\n\
             1. Reply to Anna's three Serna questions — Open, due 6 Oct (overdue).\n\
             2. Get final paint specification from Anna — Waiting since she promised it for 4 Oct; follow-up is due today.\n\
             3. Place Serna paint order with Värvikoda — Blocked until the specification arrives; due 10 Oct.\n\n\
             The scaffolding confirmation with Märt is already completed.\n\n\
             Sources: projects/serna.md, people/anna-tamm.md, meetings/2026-10-01-serna-site-meeting.md",
            None,
        )
        .expect("finish turn");
    let second = Uuid::from_u128(0x5e2a_c4a7_0000_4000_8000_0000_0000_0002);
    store
        .begin_turn_with_effort(
            second,
            Some(conversation),
            "What colour is the house going to be?",
            "chatgpt",
            "gpt-6.1-sol",
            Some("medium"),
        )
        .expect("begin second turn");
    store
        .finish_turn(
            second,
            brn_workflow::WorkTurnStatus::Completed,
            "Current approved knowledge says dark red (RAL 3011), decided at the 12 September site meeting \
             [decisions/serna-exterior-colour.md].\n\n\
             This may be outdated: the 6 October weekly sync says the client now prefers blue, and the 1 October \
             site meeting discussed green versus red again without a new decision. I have not changed the \
             approved colour. Treat it as unresolved until you confirm.",
            None,
        )
        .expect("finish second turn");
}

/// Runs the capture tour; the caller must be on the process main thread.
pub fn run() {
    let (Some(data), Some(vault)) = (env_path("BRN_CAPTURE_DATA"), env_path("BRN_CAPTURE_VAULT"))
    else {
        panic!("set BRN_CAPTURE_DATA and BRN_CAPTURE_VAULT");
    };
    seed_conversation(&data);
    // Seed-only mode prepares a demo workspace for hands-on review.
    if std::env::var_os("BRN_CAPTURE_SEED_ONLY").is_some() {
        return;
    }

    let mut c = Capture::open(&data, &vault, Appearance::Dark, 1280., 820.);
    c.shot("01-home-empty-chat-dark");

    let conversation = c.cx.update(|cx| {
        c.desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .conversations
            .first()
            .map(|conversation| conversation.id)
    });
    if let Some(id) = conversation {
        c.act(move |this, _, cx| this.simple_history(Some(id), cx));
        c.settle(|d| !d.ai.as_ref().unwrap().turns.is_empty());
        c.shot("02-chat-conversation-dark");
    }

    c.click("open-dashboard");
    c.settle(|d| d.open_doc == Some(DocRef::Dashboard));
    c.shot("03-dashboard-dark");

    c.click("open-inbox");
    c.settle(|d| d.open_doc == Some(DocRef::Inbox));
    c.shot("04-inbox-dark");

    c.click("open-findings");
    c.settle(|d| d.open_doc == Some(DocRef::Findings));
    c.shot("05-needs-review-dark");

    c.click("open-activity");
    c.settle(|d| d.open_doc == Some(DocRef::Activity));
    c.shot("06-activity-dark");

    let proposal = c.cx.update(|cx| {
        c.desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .proposals
            .iter()
            .find(|proposal| proposal.draft.title.contains("buy paint"))
            .map(|proposal| proposal.draft.id)
    });
    if let Some(id) = proposal {
        c.act(move |this, _, cx| this.simple_leave(simple::EditorTransition::Review(id), cx));
        c.settle(|d| d.open_doc == Some(DocRef::Proposal(id)));
        c.shot("07-proposal-review-dark");
    }

    c.act(|this, _, cx| this.simple_note("projects/serna.md".into(), cx));
    c.settle(|d| d.open_doc == Some(DocRef::SavedNote) && d.ai.as_ref().unwrap().editor.is_some());
    c.shot("08-note-editor-dark");

    c.act(|this, _, cx| {
        this.simple_scoped_note(
            "sources/2026-09-12-site-meeting-notes.md".into(),
            brn_workflow::library::KnowledgeScope::Source,
            cx,
        )
    });
    c.settle(|d| {
        d.open_doc == Some(DocRef::Evidence)
            && d.ai
                .as_ref()
                .unwrap()
                .evidence
                .as_ref()
                .is_some_and(|e| e.note.is_some())
    });
    c.shot("09-source-evidence-dark");

    c.act(|this, _, cx| this.close_document(cx));
    c.settle(|d| d.open_doc.is_none());
    c.act(|this, window, cx| this.open_settings(window, cx));
    c.settle(|_| false);
    c.shot("10-settings-dark");
    drop(c);

    let mut light = Capture::open(&data, &vault, Appearance::Light, 1280., 820.);
    light.click("open-dashboard");
    light.settle(|d| d.open_doc == Some(DocRef::Dashboard));
    light.shot("11-dashboard-light");
    let proposal = light.cx.update(|cx| {
        light
            .desktop
            .read(cx)
            .ai
            .as_ref()
            .unwrap()
            .proposals
            .iter()
            .find(|proposal| proposal.draft.title.contains("buy paint"))
            .map(|proposal| proposal.draft.id)
    });
    if let Some(id) = proposal {
        light.act(move |this, _, cx| this.simple_leave(simple::EditorTransition::Review(id), cx));
        light.settle(|d| d.open_doc == Some(DocRef::Proposal(id)));
        light.shot("13-proposal-review-light");
    }
    drop(light);

    let mut narrow = Capture::open(&data, &vault, Appearance::Dark, 720., 760.);
    narrow.click("open-inbox");
    narrow.settle(|d| d.open_doc == Some(DocRef::Inbox));
    narrow.shot("12-narrow-inbox-dark");
}
