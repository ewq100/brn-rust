//! Readable presentation of an already-qualified immutable extraction.
//!
//! GPUI Base 0.6.6 normally resolves Markdown/HTML image URLs and opens links.
//! Its supported MarkdownPlugin hooks run before those default conversions. Both
//! inline and block hooks below make Image/ImageReference/Html inert, while the
//! link handler replaces every default URL-opening action. Sources are never
//! rewritten. Only protocol-owned image occurrences receive cached Arc<Image>s;
//! document URLs, local paths and data URLs never reach GPUI's image loader.
use brn_intake::{Extraction, SourceNode};
use gpui_kit::{
    AnyElement, App, Image, IntoElement, ObjectFit, ParentElement, Styled, StyledImage,
    TestSupportExt, Window,
    base::{
        MarkdownExtensions, MarkdownNode, MarkdownParseContext, MarkdownPlugin, TextView,
        TextViewStyle, Theme, markdown_ast::Node,
    },
    div, img,
    prelude::*,
    px, relative,
};
use std::{collections::BTreeMap, sync::Arc};

const DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const POLICY_NODE: &str = "brn-intake-inert-resource";

/// Presentation policy over the toolkit's typed AST, not a second Markdown parser.
#[derive(Clone, Copy)]
struct InertResources {
    block: bool,
}
impl MarkdownPlugin for InertResources {
    fn name(&self) -> &str {
        POLICY_NODE
    }
    fn is_block(&self) -> bool {
        self.block
    }
    fn parse(&self, node: &Node, _: &MarkdownParseContext<'_>) -> Option<MarkdownNode> {
        let text = match node {
            Node::Image(image) => format!("[Image link: {}]", image.alt),
            Node::ImageReference(image) => format!("[Image link: {}]", image.alt),
            Node::Html(html) => html.value.clone(),
            // The maintained email adapter quotes decoded metadata/body in code
            // fences. Render the toolkit's already-decoded literal value in the
            // normal proportional font; never interpret its contents as markup.
            Node::Code(code) if self.block => code.value.clone(),
            _ => return None,
        };
        Some(MarkdownNode::new(POLICY_NODE, ()).text(text))
    }
    fn render(&self, node: &MarkdownNode, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().w_full().child(node.as_text().to_owned())
    }
}

pub(super) fn safe_markdown(id: String, text: String, cx: &App) -> TextView {
    TextView::markdown(id, text)
        .style(TextViewStyle::from_theme(&Theme::global(cx)))
        .markdown_extensions(
            MarkdownExtensions::default()
                .plugin(InertResources { block: true })
                .plugin(InertResources { block: false })
                .parser_revision(1),
        )
        .on_link_click(|_, _, _, cx| cx.stop_propagation())
}

fn is_body(source: &SourceNode) -> bool {
    matches!(source.media_type.as_str(), "text/plain" | "text/html")
        && source.name == "unnamed MIME part"
}
fn source_name(source: &SourceNode) -> String {
    if source.parent.is_none() && source.media_type == "message/rfc822" {
        format!("Email · {}", source.name)
    } else if is_body(source) {
        if source.media_type == "text/html" {
            "Email HTML alternative".into()
        } else {
            "Email body".into()
        }
    } else {
        source.name.clone()
    }
}
fn visible_source(source: &SourceNode, extraction: &Extraction) -> bool {
    source.status != "container"
        && !source.media_type.starts_with("multipart/")
        && (source.status != "retained-inline"
            || !source.text.is_empty()
            || extraction
                .occurrences
                .iter()
                .any(|occurrence| occurrence.source_id == source.id))
}
fn gap_text(extraction: &Extraction, gap: &str) -> String {
    for source in &extraction.sources {
        if let Some(rest) = gap.strip_prefix(&source.id)
            && rest.starts_with([' ', ':', '·'])
        {
            // The full locator/code remains available in detailed inspection.
            // Keep the maintained human diagnostic message in this reading view.
            let message = rest.split_once("]: ").map_or(rest, |(_, message)| message);
            return format!(
                "{}: {}",
                source_name(source),
                message.trim_start_matches([' ', ':', '·'])
            );
        }
    }
    gap.to_owned()
}

pub(super) fn render(
    extraction: &Extraction,
    images: &BTreeMap<(String, [u8; 32]), Arc<Image>>,
    expanded_source: Option<&str>,
    cx: &App,
) -> AnyElement {
    let mut reader = div().id("inbox-readable-extraction").test_support()
        .flex().flex_col().gap_4().w_full()
        .child("Extracted wording and retained pictures. Links are inactive; original documents remain available for inspection.");
    for (source_index, source) in extraction
        .sources
        .iter()
        .enumerate()
        .filter(|(_, source)| visible_source(source, extraction))
    {
        let name = source_name(source);
        let mut section = div()
            .id(format!("inbox-reader-source-{source_index}"))
            .test_support()
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(name.clone()),
            );
        if source.status == "unprocessed" {
            section = section.child("Retained attachment · content not read. Inspect the original before relying on it.");
        } else if source.media_type == DOCX {
            section = section.child("Document text and pictures · page layout, charts, shapes and other visual content may require original inspection.");
        } else if source.status == "partial" {
            section = section.child(
                "Extracted content · review the gaps below before relying on complete coverage.",
            );
        }
        let expanded = expanded_source.is_none_or(|selected| selected == source.id)
            || source.parent.is_none()
            || is_body(source);
        if expanded && !source.text.is_empty() {
            section = section.child(safe_markdown(
                format!("inbox-reader-text-{source_index}"),
                source.text.clone(),
                cx,
            ));
        }
        if expanded {
            for (occurrence_index, occurrence) in extraction
                .occurrences
                .iter()
                .enumerate()
                .filter(|(_, occurrence)| occurrence.source_id == source.id)
            {
                let Some(asset) = extraction
                    .assets
                    .iter()
                    .find(|asset| asset.id == occurrence.asset_id)
                else {
                    continue;
                };
                let caption = occurrence
                    .alt
                    .as_deref()
                    .filter(|alt| !alt.is_empty())
                    .map_or_else(
                        || format!("Retained picture from {name}"),
                        |alt| format!("{name} · {alt}"),
                    );
                let mut picture = div()
                    .id(format!("inbox-reader-picture-{occurrence_index}"))
                    .test_support()
                    .aria_label(caption.clone())
                    .flex()
                    .flex_col()
                    .gap_1()
                    .w_full();
                if let Some(image) = images.get(&(asset.media_type.clone(), asset.sha256)) {
                    picture = picture.child(
                        img(image.clone())
                            .object_fit(ObjectFit::Contain)
                            .max_w(relative(1.))
                            .w(px(480.))
                            .h(px(320.)),
                    );
                } else {
                    picture = picture.child("This retained picture is unavailable in the reading view. Inspect its exact original.");
                }
                section = section.child(picture.child(caption));
            }
        }
        reader = reader.child(section);
    }
    if !extraction.gaps.is_empty() {
        let mut gaps = div()
            .id("inbox-reader-gaps")
            .test_support()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child("What needs original inspection"),
            );
        for (index, gap) in extraction.gaps.iter().enumerate() {
            gaps = gaps.child(
                div()
                    .id(format!("inbox-reader-gap-{index}"))
                    .test_support()
                    .child(gap_text(extraction, gap)),
            );
        }
        reader = reader.child(gaps);
    }
    reader.into_any_element()
}

#[cfg(all(test, target_os = "macos", feature = "native-test-support"))]
#[path = "inbox_reader_tests.rs"]
mod tests;
