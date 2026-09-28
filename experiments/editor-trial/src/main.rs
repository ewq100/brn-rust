use brn_editor_trial::{Anchor, Document, PendingSelection, load_document_from_args};
use gpui_kit::{
    AppContext, Context, Entity, ScrollAnchor, ScrollHandle, Subscription, Window, WindowOptions,
    component::{
        Root,
        button::Button,
        input::{Editor, EditorState, Input, InputEvent, InputState},
        scroll::ScrollableElement,
    },
    div,
    prelude::*,
    px,
};

struct Trial {
    title: String,
    document: Document,
    editor: Entity<EditorState>,
    comment_input: Entity<InputState>,
    page_scroll: ScrollHandle,
    editor_anchor: ScrollAnchor,
    pending: Option<PendingSelection>,
    message: String,
    diff: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl Trial {
    fn new(title: String, text: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("markdown")
                .default_value(text.clone())
        });
        let comment_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Write a comment on the captured passage")
        });
        let subscription = cx.subscribe(&editor, |this, editor, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.document
                    .replace_text(editor.read(cx).value().to_string());
                this.diff = None;
                cx.notify();
            }
        });
        let page_scroll = ScrollHandle::new();
        let editor_anchor = ScrollAnchor::for_handle(page_scroll.clone());
        Self {
            title,
            document: Document::new(text),
            editor,
            comment_input,
            page_scroll,
            editor_anchor,
            pending: None,
            message: "Select text in the editor, then capture it before writing a comment.".into(),
            diff: None,
            _subscriptions: vec![subscription],
        }
    }

    fn capture(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor.read(cx);
        let text = editor.value().to_string();
        let range = editor.selected_range();
        self.document.replace_text(text);
        match self.document.capture_selection(range) {
            Ok(pending) => {
                self.message = format!("Captured: {}", preview(pending.quote(), 100));
                self.pending = Some(pending);
            }
            Err(error) => {
                self.message = error;
                self.pending = None;
            }
        }
        cx.notify();
    }

    fn add_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pending) = self.pending.take() else {
            self.message = "Capture a passage before adding a comment.".into();
            cx.notify();
            return;
        };
        let current_text = self.editor.read(cx).value().to_string();
        self.document.replace_text(current_text);
        let body = self.comment_input.read(cx).value().to_string();
        match self.document.add_comment(pending.clone(), body) {
            Ok(id) => {
                self.comment_input
                    .update(cx, |input, cx| input.set_value("", window, cx));
                self.message = format!("Comment {id} added.");
            }
            Err(error) => {
                self.message = error;
                self.pending = Some(pending);
            }
        }
        cx.notify();
    }

    fn focus_comment(&mut self, id: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(comment) = self
            .document
            .comments()
            .iter()
            .find(|comment| comment.id == id)
        else {
            return;
        };
        if let Anchor::Resolved(range) = &comment.anchor {
            let range = range.clone();
            self.editor.update(cx, |editor, cx| {
                editor.set_selected_range(range, cx);
                editor.focus(window, cx);
            });
            self.editor_anchor.scroll_to(window, cx);
            self.message = format!("Selected comment {id}'s current passage.");
        } else {
            self.message = "This comment no longer has a safe current passage.".into();
        }
        cx.notify();
    }
}

fn preview(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let short: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{short}…")
    } else {
        short
    }
}

impl Render for Trial {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = if self.document.is_modified() {
            "Modified"
        } else {
            "At opened revision"
        };
        let mut comments = div().flex().flex_col().gap_2();
        if self.document.comments().is_empty() {
            comments = comments.child("No comments yet.");
        }
        for comment in self.document.comments() {
            let id = comment.id;
            let status = match &comment.anchor {
                Anchor::Resolved(range) => {
                    format!("Anchored at bytes {}–{}", range.start, range.end)
                }
                Anchor::Unresolved(reason) => format!("Unresolved: {reason}"),
            };
            comments =
                comments.child(
                    div()
                        .p_2()
                        .border_1()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(format!("#{id} · {status}"))
                        .child(format!(
                            "Original quote ({} bytes):",
                            comment.original_quote.len()
                        ))
                        .child(
                            div()
                                .h(px(128.))
                                .border_1()
                                .overflow_scrollbar()
                                .id(format!("quote-scroll-{id}"))
                                .child(div().font_family("Menlo").flex().flex_col().children(
                                    comment.original_quote.split('\n').map(|line| {
                                        div().whitespace_nowrap().child(line.to_owned())
                                    }),
                                )),
                        )
                        .child(format!("Comment: {}", comment.body))
                        .child(
                            Button::new(format!("focus-{id}"))
                                .label("Show passage")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.focus_comment(id, window, cx)
                                })),
                        ),
                );
        }
        let pending = self
            .pending
            .as_ref()
            .map(|p| preview(p.quote(), 120))
            .unwrap_or_else(|| "None".into());
        let diff = self
            .diff
            .as_deref()
            .unwrap_or("Press Refresh diff to compare with the opened document.");
        div().id("trial-page").size_full().flex().flex_col().gap_3().p_3()
            .overflow_y_scroll().track_scroll(&self.page_scroll)
            .vertical_scrollbar(&self.page_scroll)
            .child(format!("{} · {} · {} bytes · {} comments", self.title, state, self.document.text().len(), self.document.comments().len()))
            .child("Edits and comments live only in memory. Closing the window discards them; the source file is never overwritten.")
            .child(div().flex().gap_2()
                .child(Button::new("capture").label("Capture selection")
                    .on_click(cx.listener(|this, _, window, cx| this.capture(window, cx))))
                .child(Button::new("refresh-diff").label("Refresh diff")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.document.replace_text(this.editor.read(cx).value().to_string());
                        this.diff = Some(this.document.unified_diff());
                        cx.notify();
                    }))))
            .child(self.message.clone())
            .child(div().id("editor-anchor").anchor_scroll(Some(self.editor_anchor.clone()))
                .child(Editor::new(&self.editor).h(px(410.)).aria_label("Markdown document editor")))
            .child(format!("Captured passage: {pending}"))
            .child(div().flex().flex_col().gap_2()
                .child(Input::new(&self.comment_input).w_full().id("comment-body"))
                .child(Button::new("add-comment").label("Add comment")
                    .on_click(cx.listener(|this, _, window, cx| this.add_comment(window, cx)))))
            .child("Comments")
            .child(comments)
            .child("Revision diff (opened → current)")
            .child(div().p_2().border_1().font_family("Menlo").flex().flex_col().children(diff.lines().map(|line| div().child(line.to_owned()))))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() == 1 && args[0] == "--help" {
        println!(
            "Usage: brn-editor-trial [UTF8_MARKDOWN_PATH]\nOpens a local in-memory editor trial; source files are never saved."
        );
        return;
    }
    let (title, text) = match load_document_from_args(args.iter().map(String::as_str)) {
        Ok(document) => document,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.spawn(async move |cx| {
                cx.open_window(WindowOptions::default(), |window, cx| {
                    let trial = cx.new(|cx| Trial::new(title, text, window, cx));
                    cx.new(|cx| Root::new(trial, window, cx))
                })
                .expect("failed to open editor window");
            })
            .detach();
        });
}
