use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, Context, Entity, EntityInputHandler,
    InteractiveElement, IntoElement, ParentElement, Pixels, Render, Styled, Subscription,
    TestAppContext, Window, WindowBounds, WindowOptions,
    base::{
        MarkdownExtensions, MarkdownNode, MarkdownParseContext, MarkdownPlugin, TextView,
        TextViewState, markdown_ast::Node,
    },
    div, point, px, size,
    test::{TestSupportExt, TestWindowExt},
};
use std::ops::Range;
use zorite_editor::{EditorEvent, EditorState, SyntaxStyle};

// BRN's two LiteralHtml plugins, from threads_ui.rs:253–269, reproduced only
// in this test host. No BRN database or provider is initialized.
#[derive(Clone)]
struct LiteralHtml {
    block: bool,
}
impl MarkdownPlugin for LiteralHtml {
    fn name(&self) -> &str {
        "brn-literal-html"
    }
    fn is_block(&self) -> bool {
        self.block
    }
    fn parse(&self, node: &Node, _: &MarkdownParseContext<'_>) -> Option<MarkdownNode> {
        if let Node::Html(html) = node {
            Some(MarkdownNode::new(self.name(), ()).text(html.value.clone()))
        } else {
            None
        }
    }
    fn render(&self, node: &MarkdownNode, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child(node.as_text().to_owned())
    }
}

pub struct Host {
    pub editor: Entity<EditorState>,
    pub reader: Entity<TextViewState>,
    pub changed: usize,
    pub notifications: usize,
    pub capture_read_only: bool,
    pub show_reader: bool,
    pub font_size: f32,
    pub comment_offset: Option<usize>,
    _subscriptions: Vec<Subscription>,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let read_only = self.capture_read_only;
        let mut view = div()
            .relative()
            .w_full()
            .text_size(px(self.font_size))
            .font_family(".SystemUIFont")
            .capture_key_down(move |_, window, cx| {
                if read_only {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            });
        // Deliberately test the strongest simple key/action-capture wrapper.
        // It cannot intercept EntityInputHandler callbacks or child mouse edits.
        macro_rules! capture {
            ($($action:ty),* $(,)?) => { $(
                view = view.capture_action(move |_: &$action, _, cx| {
                    if read_only { cx.stop_propagation(); } else { cx.propagate(); }
                });
            )* };
        }
        capture!(
            zorite_editor::Backspace,
            zorite_editor::Delete,
            zorite_editor::Newline,
            zorite_editor::Paste,
            zorite_editor::Cut,
            zorite_editor::Undo,
            zorite_editor::Redo,
            zorite_editor::Indent,
            zorite_editor::Outdent,
            zorite_editor::Bold,
            zorite_editor::Italic,
            zorite_editor::Underline,
            zorite_editor::Strike,
            zorite_editor::Highlight,
            zorite_editor::Code
        );
        if let Some(offset) = self.comment_offset
            && let Some(caret) = self.editor.read(_cx).bounds_for_offset(offset)
        {
            view = view.child(
                div()
                    .id("margin-comment")
                    .absolute()
                    .left(px(260.))
                    .top(caret.origin.y)
                    .w(px(120.))
                    .h(px(20.))
                    .child("Comment")
                    .test_support(),
            );
        }
        if self.show_reader {
            view.child(
                TextView::new(&self.reader)
                    .w_full()
                    .markdown_extensions(
                        MarkdownExtensions::default()
                            .plugin(LiteralHtml { block: true })
                            .plugin(LiteralHtml { block: false })
                            .parser_revision(1),
                    )
                    .image_source(|_| {
                        std::sync::Arc::new(gpui_kit::Image::from_bytes(
                            gpui_kit::ImageFormat::Png,
                            Vec::new(),
                        ))
                        .into()
                    })
                    .on_link_click(|_, _, _, cx| cx.stop_propagation()),
            )
        } else {
            view.child(self.editor.clone())
        }
    }
}

pub struct Fixture {
    pub window: AnyWindowHandle,
    pub host: Entity<Host>,
    pub editor: Entity<EditorState>,
    pub reader: Entity<TextViewState>,
}
impl Fixture {
    pub fn new(cx: &mut TestAppContext, source: &str) -> Self {
        cx.update(|cx| {
            gpui_kit::init(cx);
            zorite_editor::bind_keys(cx);
        });
        let (window, host) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(0.), px(0.)),
                        size(px(800.), px(1000.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    let editor = cx.new(|cx| {
                        let editor = EditorState::new(window, cx).with_text(source);
                        assert!(
                            editor.bounds_for_offset(0).is_none(),
                            "no geometry before first paint"
                        );
                        editor
                    });
                    let reader = cx.new(|cx| TextViewState::markdown(source, cx));
                    cx.new(|cx| {
                        let changed = cx.subscribe(&editor, |this: &mut Host, _, event, _| {
                            if matches!(event, EditorEvent::Changed) {
                                this.changed += 1;
                            }
                        });
                        let notifications = cx.observe(&editor, |this: &mut Host, _, _| {
                            this.notifications += 1;
                        });
                        Host {
                            editor,
                            reader,
                            changed: 0,
                            notifications: 0,
                            capture_read_only: false,
                            show_reader: false,
                            font_size: 16.,
                            comment_offset: None,
                            _subscriptions: vec![changed, notifications],
                        }
                    })
                },
            )
            .unwrap()
        });
        let (editor, reader) = host.read_with(cx, |h, _| (h.editor.clone(), h.reader.clone()));
        let f = Self {
            window,
            host,
            editor,
            reader,
        };
        cx.update_window(window, |_, window, cx| {
            f.editor.update(cx, |e, cx| e.focus(window, cx))
        })
        .unwrap();
        f.frame(cx);
        f
    }
    pub fn frame(&self, cx: &mut TestAppContext) {
        cx.run_until_parked();
        cx.update_window(self.window, |_, w, cx| w.render_frame(cx))
            .unwrap();
        cx.run_until_parked();
    }
    pub fn text(&self, cx: &TestAppContext) -> String {
        self.editor.read_with(cx, |e, _| e.text().to_owned())
    }
    pub fn changed(&self, cx: &TestAppContext) -> usize {
        self.host.read_with(cx, |h, _| h.changed)
    }
    pub fn press(&self, cx: &mut TestAppContext, key: &str) {
        cx.update_window(self.window, |_, w, cx| w.press(key, cx))
            .unwrap();
        self.frame(cx);
    }
    pub fn cursor(&self, cx: &mut TestAppContext, offset: usize) {
        self.editor.update(cx, |e, cx| e.set_cursor(offset, cx));
        self.frame(cx);
    }
    pub fn selection(&self, cx: &mut TestAppContext) -> Range<usize> {
        cx.update_window(self.window, |_, w, cx| {
            self.editor.update(cx, |e, cx| {
                let utf16 = e.selected_text_range(false, w, cx).unwrap().range;
                utf16_to_byte(e.text(), utf16.start)..utf16_to_byte(e.text(), utf16.end)
            })
        })
        .unwrap()
    }
    pub fn select(&self, cx: &mut TestAppContext, range: Range<usize>) {
        self.cursor(cx, range.start);
        for _ in 0..=self.text(cx).len() {
            if self.editor.read_with(cx, |e, _| e.cursor()) >= range.end {
                break;
            }
            self.press(cx, "shift-right");
        }
        assert_eq!(self.selection(cx), range);
    }
    pub fn bounds(&self, cx: &TestAppContext, offset: usize) -> Bounds<Pixels> {
        self.editor.read_with(cx, |e, _| {
            e.bounds_for_offset(offset).expect("laid-out caret")
        })
    }
    pub fn markdown(&self, cx: &mut TestAppContext) {
        self.editor
            .update(cx, |e, cx| e.set_markdown_style(style(), cx));
        self.frame(cx);
    }
    pub fn show_reader(&self, cx: &mut TestAppContext) {
        self.host.update(cx, |h, cx| {
            h.show_reader = true;
            cx.notify();
        });
        self.frame(cx);
    }
}

pub fn utf16_to_byte(text: &str, units: usize) -> usize {
    let mut count = 0;
    for (byte, ch) in text.char_indices() {
        if count == units {
            return byte;
        }
        count += ch.len_utf16();
        assert!(count <= units, "selection split a UTF-16 surrogate pair");
    }
    assert_eq!(count, units);
    text.len()
}
pub fn style() -> SyntaxStyle {
    let color = gpui_kit::rgb(0x333333).into();
    SyntaxStyle {
        marker: color,
        code: color,
        code_bg: color,
        link: color,
        tag: color,
        quote: color,
        alert_note: color,
        alert_tip: color,
        alert_important: color,
        alert_warning: color,
        alert_caution: color,
        alert_icons: None,
        rule: color,
        mark_bg: color,
        block_label: None,
        block_label_gen: 0,
        block_ref_count: None,
        popover_bg: color,
        popover_border: color,
        popover_fg: color,
        popover_hover: color,
        popover_divider: color,
        popover_danger: color,
        mono: gpui_kit::font("Menlo"),
        property_icon: None,
    }
}
