use brn_passage_comment_geometry::{Workshop, demo_source};
use gpui_kit::{AppContext, WindowOptions};
fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        let (source, comments) = demo_source();
        gpui_kit::open_window(WindowOptions::default(), cx, |w, cx| {
            cx.new(|cx| Workshop::new(&source, comments, w, cx))
        })
        .expect("open synthetic passage geometry host");
    });
}
