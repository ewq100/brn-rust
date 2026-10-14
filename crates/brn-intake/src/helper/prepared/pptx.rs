//! Typed slide/table projection after the legacy reader qualifies source joins.
use super::{gap, push};
use crate::{Extraction, prepared::*};
use pptx_parse::{GraphicFrameData, ShapeNode, TextBody};
use std::collections::BTreeMap;

pub(super) fn read(
    bytes: &[u8],
    doc: &mut PreparedDocument,
    extraction: &Extraction,
    images: &BTreeMap<String, String>,
) -> Result<(), String> {
    let limits = pptx_parse::ParseLimits {
        max_xml_bytes: super::super::MAX_PACKAGE_BYTES,
        max_xml_events: 200_000,
        max_xml_text_bytes: crate::MAX_TEXT_BYTES,
        max_xml_depth: 128,
        max_attributes_per_element: 128,
        max_attribute_bytes: 64 * 1024,
        max_relationships: 10_000,
        max_shapes: crate::MAX_SOURCES,
        max_paragraphs: 10_000,
        max_runs: 50_000,
        max_comments: crate::MAX_SOURCES,
    };
    let package = pptx_parse::parse_pptx_with_limits(bytes, &limits).map_err(|e| e.to_string())?;
    for (i, slide) in package.slides.iter().enumerate() {
        let locator = format!("pptx/part={}", slide.part_path);
        push(
            doc,
            &locator,
            BlockContent::Metadata {
                name: "Source slide".into(),
                value: format!(
                    "Slide {}{}",
                    i + 1,
                    if slide.hidden == Some(true) {
                        " (hidden)"
                    } else {
                        ""
                    }
                ),
            },
        )?;
        shapes(&slide.shapes, doc, &locator, extraction, images)?;
        if !slide.notes.is_empty() {
            push(
                doc,
                format!("{locator}/notes"),
                BlockContent::Metadata {
                    name: "Speaker notes".into(),
                    value: slide.notes.clone(),
                },
            )?;
        }
    }
    for comment in &package.comments {
        push(
            doc,
            format!(
                "pptx/part={}/comment={}",
                comment.slide_part_path, comment.id
            ),
            BlockContent::Metadata {
                name: "Source comment".into(),
                value: format!(
                    "Author {}; source position ({},{}); {}",
                    comment.author_id, comment.x_emu, comment.y_emu, comment.text
                ),
            },
        )?;
    }
    Ok(())
}
fn text(body: &TextBody) -> String {
    body.paragraphs
        .iter()
        .map(|p| {
            p.runs
                .iter()
                .map(|r| {
                    if r.line_break {
                        "\n".to_string()
                    } else {
                        r.text.clone()
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn shapes(
    items: &[ShapeNode],
    doc: &mut PreparedDocument,
    prefix: &str,
    extraction: &Extraction,
    images: &BTreeMap<String, String>,
) -> Result<(), String> {
    for item in items {
        match item {
            ShapeNode::Group(group) => shapes(&group.children, doc, prefix, extraction, images)?,
            ShapeNode::Shape(shape) => {
                if let Some(body) = &shape.text {
                    let locator = format!("{prefix}/shape={}", shape.base.id);
                    if shape.base.hidden {
                        push(
                            doc,
                            format!("{locator}/visibility"),
                            BlockContent::Metadata {
                                name: "Source shape visibility".into(),
                                value: "Hidden".into(),
                            },
                        )?;
                    }
                    for (i, paragraph) in body.paragraphs.iter().enumerate() {
                        let value = paragraph
                            .runs
                            .iter()
                            .map(|r| {
                                if r.line_break {
                                    "\n".to_string()
                                } else {
                                    r.text.clone()
                                }
                            })
                            .collect::<String>();
                        let location = format!("{locator}/paragraph/{i}");
                        if !value.is_empty() {
                            match &paragraph.properties.bullet {
                                Some(pptx_parse::Bullet::Character { value: label }) => push(
                                    doc,
                                    &location,
                                    BlockContent::ListItem {
                                        text: value,
                                        level: u8::try_from(paragraph.properties.level)
                                            .map_err(|_| "PPTX list depth")?,
                                        label: label.clone(),
                                    },
                                )?,
                                Some(pptx_parse::Bullet::AutoNumber {
                                    scheme,
                                    start_at,
                                    restart,
                                }) => {
                                    push(doc, &location, BlockContent::Paragraph { text: value })?;
                                    push(
                                        doc,
                                        format!("{location}/list-declaration"),
                                        BlockContent::Metadata {
                                            name: "Source numbered list declaration".into(),
                                            value: format!(
                                                "level {}; scheme {scheme}; start {start_at}; restart {restart}",
                                                paragraph.properties.level
                                            ),
                                        },
                                    )?;
                                    gap(
                                        doc,
                                        &location,
                                        "Numbered list declaration retained; inherited/source-displayed ordinal labels require source review",
                                    )?;
                                }
                                _ => push(doc, &location, BlockContent::Paragraph { text: value })?,
                            }
                        }
                    }
                }
            }
            ShapeNode::GraphicFrame(frame) => {
                let locator = format!("{prefix}/shape={}", frame.base.id);
                if let GraphicFrameData::Table(table) = &frame.data {
                    let rows = table
                        .rows
                        .iter()
                        .enumerate()
                        .map(|(r, row)| {
                            row.cells
                                .iter()
                                .enumerate()
                                .map(|(c, cell)| PreparedCell {
                                    address: format!("R{}C{}", r + 1, c + 1),
                                    column: c as u32,
                                    text: text(&cell.text),
                                    formula: None,
                                    result_status: None,
                                    comment: None,
                                    links: vec![],
                                    row_span: cell.row_span.max(1),
                                    column_span: cell.grid_span.max(1),
                                })
                                .collect()
                        })
                        .collect();
                    push(doc, &locator, BlockContent::Table { rows })?;
                }
            }
            ShapeNode::Picture(picture) => {
                let prefix = format!("{prefix}/shape={}/", picture.base.id);
                let occurrences = extraction
                    .occurrences
                    .iter()
                    .filter(|o| o.locator.starts_with(&prefix))
                    .collect::<Vec<_>>();
                match occurrences.as_slice() {
                    [occurrence] => push(
                        doc,
                        &occurrence.locator,
                        BlockContent::Figure {
                            asset_id: images
                                .get(&occurrence.asset_id)
                                .ok_or("normalized PPTX asset missing")?
                                .clone(),
                            alt: occurrence
                                .alt
                                .clone()
                                .unwrap_or_else(|| "Source figure".into()),
                        },
                    )?,
                    [] => gap(
                        doc,
                        &prefix,
                        "Picture unavailable in qualified source evidence",
                    )?,
                    _ => return Err("ambiguous prepared picture source occurrence".into()),
                }
            }
        }
    }
    Ok(())
}
