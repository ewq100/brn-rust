//! Compile-only stock host. Do not run during gates 1–2.
use gpui_kit::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, WindowOptions,
    div, px,
};
use zorite_editor::EditorState;

struct Qualification {
    editor: Entity<EditorState>,
}

impl Render for Qualification {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().p_4().text_size(px(16.)).child(self.editor.clone())
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        zorite_editor::bind_keys(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            let editor = cx.new(|cx| {
                EditorState::new(window, cx).with_text("# Z1\n\nStock editor qualification.")
            });
            cx.new(|_| Qualification { editor })
        })
        .expect("open Z1 qualification window");
    });
}
