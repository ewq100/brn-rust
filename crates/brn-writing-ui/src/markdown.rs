//! One stored-asset-only, literal-HTML, http(s)-link Markdown policy.
use crate::native::{PROSE_TEXT, UI_FONT};
use gpui_kit::base::{
    MarkdownExtensions, MarkdownNode, MarkdownParseContext, MarkdownPlugin, TextView,
    TextViewState, markdown_ast::Node,
};
use gpui_kit::{App, Entity, Image, ImageFormat, Window, div, prelude::*};
use std::{collections::BTreeMap, sync::Arc};
// Inline HTML is shown literally. Managed figures use only stored image bytes;
// source URLs never authorize network or filesystem reads during rendering.
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
/// An image the caller's stored assets cannot supply renders as a bounded,
/// labelled placeholder with its alternative text, instead of nothing
/// (spec §10.1). Images that resolve keep the renderer's own image.
#[derive(Clone)]
struct MissingImage {
    stored: Arc<std::collections::BTreeSet<String>>,
    /// A paragraph holding only the image becomes a full-width block, so
    /// its alternative text wraps; an image inside prose stays inline.
    block: bool,
}
impl MarkdownPlugin for MissingImage {
    fn name(&self) -> &str {
        if self.block {
            "brn-missing-image-block"
        } else {
            "brn-missing-image"
        }
    }
    fn is_block(&self) -> bool {
        self.block
    }
    fn parse(&self, node: &Node, _: &MarkdownParseContext<'_>) -> Option<MarkdownNode> {
        let node = match (self.block, node) {
            (true, Node::Paragraph(paragraph)) => match paragraph.children.as_slice() {
                [only @ Node::Image(_)] => only,
                _ => return None,
            },
            (true, _) => return None,
            (false, node) => node,
        };
        let Node::Image(image) = node else {
            return None;
        };
        if self.stored.contains(&image.url) {
            return None;
        }
        let alt = image.alt.trim();
        let text = if alt.is_empty() {
            "Image unavailable".to_owned()
        } else {
            format!("Image unavailable · {alt}")
        };
        Some(
            MarkdownNode::new(self.name(), ())
                .text(text.clone())
                .accessibility_label(text),
        )
    }
    fn render(&self, node: &MarkdownNode, _: &mut Window, _: &mut App) -> impl IntoElement {
        let p = crate::theme::pal();
        div()
            .w_full()
            .my_1()
            .px_3()
            .py_2()
            .rounded(gpui_kit::px(crate::theme::metrics::RADIUS_CONTROL))
            .border_1()
            .border_dashed()
            .border_color(gpui_kit::rgb(p.border))
            .bg(gpui_kit::rgb(p.alt))
            .text_color(gpui_kit::rgb(p.muted))
            .text_size(gpui_kit::px(crate::theme::metrics::TEXT_LABEL + 1.))
            .whitespace_normal()
            .child(node.as_text().to_owned())
    }
}
/// Only absolute lowercase http(s) URLs may open externally; every other
/// scheme stays inert. Unusual casing such as `HTTPS://` fails closed rather
/// than opening an unexpected URL.
pub fn external_link(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}
/// Images resolve only from the caller-visible stored asset map; any other
/// reference renders an empty placeholder instead of authorizing a read.
pub fn stored_image(assets: &BTreeMap<String, Arc<Image>>, uri: &str) -> Arc<Image> {
    assets
        .get(uri)
        .cloned()
        .unwrap_or_else(|| Arc::new(Image::from_bytes(ImageFormat::Png, Vec::new())))
}
/// The one Markdown policy shared by every rendered note and message: HTML is
/// shown literally, images resolve only through `stored_image` for the
/// caller's own asset map, and `external_link` gates which links may open.
/// Callers keep their own fonts, scrolling and caller-scoped asset map.
pub fn safe_markdown(view: TextView, assets: Arc<BTreeMap<String, Arc<Image>>>) -> TextView {
    safe_markdown_with(view, assets, MarkdownExtensions::default())
}
/// `safe_markdown` whose parse also reports its headings to `collector`
/// (the reading view's outline). The observer runs first and declines every
/// node, so the rendered document is the same.
pub fn observed_markdown(
    view: TextView,
    assets: Arc<BTreeMap<String, Arc<Image>>>,
    collector: &crate::outline::HeadingCollector,
) -> TextView {
    safe_markdown_with(
        view,
        assets,
        collector.extensions(MarkdownExtensions::default()),
    )
}
fn safe_markdown_with(
    view: TextView,
    assets: Arc<BTreeMap<String, Arc<Image>>>,
    extensions: MarkdownExtensions,
) -> TextView {
    // The placeholder depends on which images are stored, so the parser
    // revision follows the asset keys.
    let stored: std::collections::BTreeSet<String> = assets.keys().cloned().collect();
    let revision = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::hash::DefaultHasher::new();
        stored.hash(&mut hasher);
        hasher.finish() | 1
    };
    let stored = Arc::new(stored);
    view.markdown_extensions(
        extensions
            .plugin(LiteralHtml { block: true })
            .plugin(LiteralHtml { block: false })
            .plugin(MissingImage {
                stored: stored.clone(),
                block: true,
            })
            .plugin(MissingImage {
                stored,
                block: false,
            })
            .parser_revision(revision),
    )
    .image_source(move |uri| stored_image(&assets, uri.as_ref()).into())
    .on_link_click(|url, _, _, cx| {
        if external_link(url) {
            cx.open_url(url);
        }
        cx.stop_propagation();
    })
}
pub fn markdown(
    view: &Entity<TextViewState>,
    assets: Arc<BTreeMap<String, Arc<Image>>>,
) -> TextView {
    safe_markdown(
        TextView::new(view)
            .font_family(UI_FONT)
            .text_size(PROSE_TEXT)
            .w_full()
            .h_full()
            .scrollable(true),
        assets,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_links_allow_only_absolute_lowercase_http_s() {
        assert!(external_link("https://example.com/a"));
        assert!(external_link("http://example.com"));
        for inert in [
            "javascript:alert(1)",
            "data:text/html,<b>",
            "file:///etc/passwd",
            "brn-asset://record",
            "HTTPS://example.com",
            "https:example.com",
            "//example.com",
            "example.com",
            " https://example.com",
            "",
        ] {
            assert!(!external_link(inert), "{inert:?} must stay inert");
        }
    }
    #[test]
    fn images_resolve_only_from_the_caller_s_own_stored_assets() {
        let key = "brn-asset://stored";
        let image = Arc::new(Image::from_bytes(ImageFormat::Png, vec![1, 2, 3]));
        let assets = BTreeMap::from([(key.to_owned(), image.clone())]);
        assert!(Arc::ptr_eq(&stored_image(&assets, key), &image));
        for uri in [
            "brn-asset://missing",
            "https://example.com/figure.png",
            "file:///tmp/figure.png",
            "",
        ] {
            let fallback = stored_image(&assets, uri);
            assert_eq!(fallback.format, ImageFormat::Png);
            assert!(fallback.bytes.is_empty(), "{uri:?} must not resolve");
        }
    }
}
