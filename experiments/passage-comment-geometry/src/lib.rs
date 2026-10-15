//! One active passage comment on BRN's current Read viewer. Synthetic host only.
use brn_threads_core::{Comment, exact_writing_range};
use gpui_kit::{
    App, AppContext, Bounds, Context, Entity, Image, ImageFormat, InteractiveElement, IntoElement,
    ParentElement, Pixels, Point, Render, SharedString, StatefulInteractiveElement, Styled,
    Subscription, TestSupportExt, TextAlign, Window,
    base::{
        MarkdownExtensions, MarkdownNode, MarkdownParseContext, MarkdownPlugin, RangeHighlight,
        TextView, TextViewState, TextViewStyle, markdown_ast::Node,
    },
    canvas,
    component::input::{Editor, EditorState, InputEvent},
    div, fill, point, px, rgb, size,
};
use std::{cell::RefCell, ops::Range, rc::Rc, sync::Arc};

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

/// Preserve BRN's shared policy: literal HTML, stored-map-only assets, explicit
/// lowercase http(s) link opening. The experiment passes an empty synthetic map.
pub fn readable(view: &Entity<TextViewState>) -> TextView {
    TextView::new(view)
        .w_full()
        .h_full()
        .scrollable(true)
        .font_family("Georgia")
        .text_size(px(16.))
        .style(TextViewStyle::default().with_paragraph_gap(gpui_kit::rems(0.95)))
        .markdown_extensions(
            MarkdownExtensions::default()
                .plugin(LiteralHtml { block: true })
                .plugin(LiteralHtml { block: false })
                .parser_revision(1),
        )
        .image_source(|_| Arc::new(Image::from_bytes(ImageFormat::Png, Vec::new())).into())
        .on_link_click(|url, _, _, cx| {
            if url.starts_with("https://") || url.starts_with("http://") {
                cx.open_url(url);
            }
            cx.stop_propagation();
        })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unavailable {
    NoComment,
    DirtyDocument,
    WrongNoteOrVersion,
    Unmapped,
    AwaitingLayout,
}
#[derive(Clone, Debug, Default)]
pub struct Measurement {
    /// Passage-start glyph box in window coordinates, never full range geometry.
    pub passage: Option<Bounds<Pixels>>,
    pub margin: Option<Bounds<Pixels>>,
    /// Offset from the margin's origin; can be negative in x because passage is to its left.
    pub passage_in_margin: Option<Point<Pixels>>,
    pub card: Option<Bounds<Pixels>>,
    pub serial: usize,
}

pub struct Workshop {
    pub editor: Entity<EditorState>,
    pub reader: Entity<TextViewState>,
    pub note_id: String,
    pub version: u64,
    pub saved: String,
    pub comments: Vec<Comment>,
    pub active: Option<usize>,
    pub unavailable: Option<Unavailable>,
    pub generation: u64,
    pub show_write: bool,
    pub font_size: f32,
    pub measurement: Rc<RefCell<Measurement>>,
    // Consume each paint's callback once; never reuse it if the viewer is hidden
    // or skips painting next frame. This is distinct from the public last result.
    pending_anchor: Rc<RefCell<Option<Bounds<Pixels>>>>,
    seen_source: String,
    _change: Subscription,
    _observe: Subscription,
}
impl Workshop {
    pub fn new(
        source: &str,
        comments: Vec<Comment>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| EditorState::new(w, cx).default_value(source));
        let reader = cx.new(|cx| {
            TextViewState::markdown(source, cx)
                .selectable(true)
                .scrollable(true)
        });
        let change = cx.subscribe(&editor, |this: &mut Self, editor, event, cx| {
            if matches!(event, InputEvent::Change) {
                this.generation += 1;
                this.sync_source(editor.read(cx).value().to_string(), cx);
            }
        });
        // Toolkit programmatic edits may notify without a Change event. They
        // must invalidate geometry too; notifications never advance generation.
        let observe = cx.observe(&editor, |this: &mut Self, editor, cx| {
            this.sync_source(editor.read(cx).value().to_string(), cx);
        });
        Self {
            editor,
            reader,
            note_id: "synthetic".into(),
            version: 1,
            saved: source.into(),
            comments,
            active: None,
            unavailable: Some(Unavailable::NoComment),
            generation: 0,
            show_write: false,
            font_size: 16.,
            measurement: Rc::new(RefCell::new(Measurement::default())),
            pending_anchor: Rc::new(RefCell::new(None)),
            seen_source: source.into(),
            _change: change,
            _observe: observe,
        }
    }

    fn sync_source(&mut self, source: String, cx: &mut Context<Self>) {
        if source == self.seen_source {
            return;
        }
        self.seen_source = source.clone();
        *self.pending_anchor.borrow_mut() = None;
        self.unavailable = Some(Unavailable::DirtyDocument);
        self.measurement.borrow_mut().passage = None;
        self.measurement.borrow_mut().card = None;
        self.reader.update(cx, |reader, cx| {
            reader.clear_range_start(cx);
            reader.clear_range_highlights(cx);
            reader.set_text(&source, cx);
        });
        cx.notify();
    }

    pub fn select_comment(
        &mut self,
        index: usize,
        reveal: bool,
        cx: &mut Context<Self>,
    ) -> Result<(), Unavailable> {
        *self.pending_anchor.borrow_mut() = None;
        self.measurement.borrow_mut().passage = None;
        self.measurement.borrow_mut().card = None;
        self.reader.update(cx, |r, cx| {
            r.clear_range_start(cx);
            r.clear_range_highlights(cx);
        });
        self.active = Some(index);
        let result = self.valid_range(index, cx).and_then(|range| {
            let text = self.reader.read(cx).rendered_text();
            if text.source() != self.saved {
                return Err(Unavailable::AwaitingLayout);
            }
            text.range_for_source(range).ok_or(Unavailable::Unmapped)
        });
        let rendered_range = match result {
            Ok(range) => range,
            Err(error) => {
                self.unavailable = Some(error.clone());
                cx.notify();
                return Err(error);
            }
        };
        let result = self.reader.update(cx, |r, cx| {
            r.track_range_start(rendered_range.clone(), cx)?;
            r.set_range_highlights(
                [RangeHighlight::new(rendered_range.clone(), rgb(0xffe3a0))],
                cx,
            )?;
            if reveal {
                r.reveal_range(rendered_range, cx)?;
            }
            Ok::<_, gpui_kit::base::RangeHighlightError>(())
        });
        if result.is_err() {
            self.unavailable = Some(Unavailable::Unmapped);
            return Err(Unavailable::Unmapped);
        }
        self.unavailable = Some(Unavailable::AwaitingLayout);
        cx.notify();
        Ok(())
    }

    pub fn valid_range(&self, index: usize, cx: &App) -> Result<Range<usize>, Unavailable> {
        if self.editor.read(cx).value().as_ref() != self.saved {
            return Err(Unavailable::DirtyDocument);
        }
        let comment = self.comments.get(index).ok_or(Unavailable::NoComment)?;
        if comment.note != self.note_id {
            return Err(Unavailable::WrongNoteOrVersion);
        }
        exact_writing_range(comment, self.version, &self.saved)
            .map(|(a, b)| a..b)
            .ok_or(Unavailable::WrongNoteOrVersion)
    }
}
impl Render for Workshop {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        {
            let mut frame = self.measurement.borrow_mut();
            *self.pending_anchor.borrow_mut() = None;
            let serial = frame.serial + 1;
            *frame = Measurement {
                serial,
                ..Default::default()
            };
        }
        let mut buttons = div().flex().gap_2().h(px(40.));
        for index in 0..self.comments.len() {
            buttons = buttons.child(
                div()
                    .id(format!("comment-{index}"))
                    .px_3()
                    .py_1()
                    .child(format!("Reveal comment {}", index + 1))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let _ = this.select_comment(index, true, cx);
                    }))
                    .test_support(),
            );
        }
        buttons = buttons.child(
            div()
                .id("write-toggle")
                .px_3()
                .py_1()
                .child("Read / Write")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.show_write = !this.show_write;
                    cx.notify();
                }))
                .test_support(),
        );
        let pending = self.pending_anchor.clone();
        let allowed = self
            .active
            .is_some_and(|index| self.valid_range(index, cx).is_ok());
        let viewer = readable(&self.reader)
            .text_size(px(self.font_size))
            .on_range_start_layout(move |geometry, _, _| {
                *pending.borrow_mut() = if allowed { geometry } else { None };
            });
        let text = if self.show_write {
            Editor::new(&self.editor)
                .h_full()
                .font_family("Georgia")
                .text_size(px(16.))
                .into_any_element()
        } else {
            viewer.into_any_element()
        };
        let frame = self.measurement.clone();
        let pending = self.pending_anchor.clone();
        let label: SharedString = self
            .active
            .and_then(|i| self.comments.get(i))
            .map(|c| c.body.clone())
            .unwrap_or_else(|| "Select a comment".into())
            .into();
        let margin = canvas(
            |_, _, _| (),
            move |bounds, _, w, cx| {
                let mut frame = frame.borrow_mut();
                frame.passage = pending.borrow_mut().take();
                frame.margin = Some(bounds);
                frame.card = None;
                frame.passage_in_margin = None;
                if let Some(passage) = frame.passage {
                    let local = passage.origin - bounds.origin;
                    frame.passage_in_margin = Some(local);
                    let origin = bounds.origin + point(px(0.), local.y);
                    let card = Bounds::new(origin, size(bounds.size.width, px(32.)));
                    frame.card = Some(card);
                    w.paint_quad(fill(card, rgb(0xffe3a0)));
                    let run = w.text_style().to_run(label.len());
                    let line = w
                        .text_system()
                        .shape_line(label.clone(), px(13.), &[run], None);
                    line.paint(
                        origin + point(px(6.), px(4.)),
                        px(22.),
                        TextAlign::Left,
                        None,
                        w,
                        cx,
                    )
                    .unwrap();
                } else {
                    let message: SharedString = "Passage position unavailable".into();
                    let run = w.text_style().to_run(message.len());
                    let line = w.text_system().shape_line(message, px(12.), &[run], None);
                    line.paint(bounds.origin, px(20.), TextAlign::Left, None, w, cx)
                        .unwrap();
                }
            },
        )
        .w(px(220.))
        .h_full();
        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .child(buttons)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .id("document")
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .child(text)
                            .test_support(),
                    )
                    .child(
                        div()
                            .id("margin")
                            .w(px(220.))
                            .h_full()
                            .overflow_hidden()
                            .child(margin)
                            .test_support(),
                    ),
            )
    }
}

pub fn synthetic_comment(source: &str, range: Range<usize>, body: &str) -> Comment {
    Comment {
        note: "synthetic".into(),
        base_version: 1,
        quote: source[range.clone()].into(),
        range: Some((range.start, range.end)),
        mapped_version: None,
        mapped_range: None,
        body: body.into(),
        unresolved: false,
    }
}
pub fn demo_source() -> (String, Vec<Comment>) {
    let paragraph = "A **täpne** paragraph with õun, a\u{0308} and 👩🏽‍💻.";
    let source = format!(
        "# Existing BRN viewer\n\n{paragraph}\n\n{}\n\n{paragraph}",
        "Synthetic filler paragraph to exercise wrapping and scrolling.\n\n".repeat(24)
    );
    let a = source.find("täpne").unwrap();
    let b = source.rfind("täpne").unwrap();
    let comments = vec![
        synthetic_comment(&source, a..a + "täpne".len(), "First occurrence"),
        synthetic_comment(&source, b..b + "täpne".len(), "Second occurrence"),
    ];
    (source, comments)
}
