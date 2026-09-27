use brn_core::{Shell, WorkConfig};
use gpui_kit::{
    AppContext, Context, Entity, Subscription, Task, Window, WindowOptions,
    component::{
        Root,
        button::Button,
        input::{Editor, EditorState, InputEvent},
    },
    div,
    prelude::*,
    px,
};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Workspace,
    Activity,
    Settings,
}
struct Desktop {
    shell: Shell,
    editor: Entity<EditorState>,
    page: Page,
    path: PathBuf,
    message: String,
    _subscriptions: Vec<Subscription>,
    _poll_task: Task<()>,
}
impl Desktop {
    fn new(path: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sample = "Sample workspace text. Edit this text, then run a sample task. Results are kept in memory.";
        let editor = cx.new(|cx| EditorState::new(window, cx).default_value(sample));
        let subscription = cx.subscribe(&editor, |this, editor, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                let value = editor.read(cx).value().to_string();
                match this.shell.edit(value) {
                    Ok(()) => {
                        this.message =
                            format!("Working copy generation {}", this.shell.generation().0)
                    }
                    Err(_) => {
                        this.message =
                            "Sample input exceeds 64 KiB. Shorten it to run tasks; prior results were invalidated.".into()
                    }
                }
                cx.notify();
            }
        });
        let quit_subscription = cx.on_app_quit(|this, _| {
            this.shell.close();
            async {}
        });
        let poll_task = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(40))
                    .await;
                if this
                    .update_in(cx, |this, _, cx| {
                        if this.shell.active().is_some() {
                            this.shell.poll();
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            shell: Shell::new(sample),
            editor,
            page: Page::Workspace,
            path,
            message: "Ready for sample work.".into(),
            _subscriptions: vec![subscription, quit_subscription],
            _poll_task: poll_task,
        }
    }
    fn start(&mut self, cx: &mut Context<Self>) {
        let visible_input = self.editor.read(cx).value().to_string();
        let needs_sync = (self.shell.input_is_valid() && visible_input != self.shell.input())
            || (!self.shell.input_is_valid() && visible_input.len() <= brn_core::MAX_INPUT_BYTES);
        if needs_sync && self.shell.edit(visible_input).is_err() {
            self.message = "Sample input is limited to 64 KiB. Shorten it before starting.".into();
            cx.notify();
            return;
        }
        match self.shell.start(WorkConfig::default()) {
            Ok(id) => self.message = format!("Sample task {} started.", id.0),
            Err(error) => self.message = format!("Could not start sample task: {error:?}"),
        }
        cx.notify();
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.message = if self.shell.cancel() {
            "Cancellation requested."
        } else {
            "No running task to cancel."
        }
        .into();
        cx.notify();
    }
}
impl Render for Desktop {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = self.page;
        let mut body = div()
            .id("shell-body")
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .overflow_y_scroll();
        match page {
            Page::Workspace => {
                body = body.child("Workspace · sample work")
                    .child("This shell does not yet open documents or run AI. Edit the sample and start a deterministic task.")
                    .child(Editor::new(&self.editor).h(px(240.)).aria_label("Sample task input"))
                    .child(div().flex().gap_2()
                        .child(Button::new("start").label("Start sample task").on_click(cx.listener(|this, _, _, cx| this.start(cx))))
                        .child(Button::new("cancel").label("Cancel task").on_click(cx.listener(|this, _, _, cx| this.cancel(cx)))))
                    .child(self.message.clone());
                if let Some(result) = self.shell.result() {
                    body = body.child(result.to_string());
                }
            }
            Page::Activity => {
                body = body
                    .child("Activity · current session")
                    .child(format!(
                        "Working copy generation: {}",
                        self.shell.generation().0
                    ))
                    .child(format!(
                        "Active task: {}",
                        self.shell
                            .active()
                            .map_or("none".into(), |id| id.0.to_string())
                    ))
                    .child(format!(
                        "Progress: {}",
                        self.shell
                            .progress()
                            .map_or("none".into(), |(done, total)| format!("{done}/{total}"))
                    ))
                    .child(format!("Result: {}", self.shell.result().unwrap_or("none")))
                    .child(format!("Cancelled: {}", self.shell.was_cancelled()));
            }
            Page::Settings => {
                body = body.child("Settings · local shell")
                    .child(format!("Selected data directory: {}", self.path.display()))
                    .child("This version writes no documents or database. The directory is checked at launch.");
            }
        }
        div()
            .id("brn-shell")
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .child("BRN desktop shell · sample mode")
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("workspace")
                            .label("Workspace")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::Workspace;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("activity")
                            .label("Activity")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::Activity;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("settings")
                            .label("Settings")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::Settings;
                                cx.notify();
                            })),
                    ),
            )
            .child(body)
    }
}
pub fn run(path: PathBuf) {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.spawn(async move |cx| {
                cx.open_window(WindowOptions::default(), |window, cx| {
                    let desktop = cx.new(|cx| Desktop::new(path, window, cx));
                    cx.new(|cx| Root::new(desktop, window, cx))
                })
                .expect("failed to open BRN desktop window");
            })
            .detach();
        });
}
