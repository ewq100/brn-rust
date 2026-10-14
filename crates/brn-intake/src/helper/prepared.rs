//! Bounded typed projections used by the source-bound conversion lane.
//! Raw packages and MIME trees remain transient inside the restricted helper.
use super::{Budget, admit_package, image_asset};
use crate::prepared::*;
use crate::{
    HelperRequest, IntakeLimits, IntakeUsage, MAX_ASSETS, MAX_IMAGE_BYTES, MAX_SOURCES,
    MAX_TEXT_BYTES, digest,
};
use docx_edit::structured::{
    Block, BlockKind, CachedResult, ExportOptions, Inline, InlineKind, RevisionView, StoryKind,
    StorySelection,
};
use std::collections::BTreeMap;

mod html;
mod pptx;
mod xlsx;

pub fn prepare(request: HelperRequest) -> Result<PreparedDocument, String> {
    let limits = request.limits.clone().unwrap_or_default();
    limits.validate()?;
    if request.bytes.is_empty() || request.bytes.len() > limits.max_input_bytes {
        return Err("prepared input budget".into());
    }
    let mut doc = PreparedDocument {
        schema: 2,
        profile: PROFILE.into(),
        source_sha256: digest(&request.bytes),
        format: if request.kind == "htm" {
            "html".into()
        } else {
            request.kind.clone()
        },
        limits: limits.clone(),
        consumed: IntakeUsage {
            input_bytes: request.bytes.len(),
            ..Default::default()
        },
        blocks: vec![],
        assets: vec![],
        pages: vec![],
        gaps: vec![],
    };
    match request.kind.as_str() {
        "docx" | "pptx" | "eml" => {
            // Reuse qualified admission, MIME container joins, attachments and image validation.
            let extraction = super::extract(request.clone())?;
            doc.consumed = extraction.consumed.clone().ok_or("missing usage")?;
            let mut image_ids = BTreeMap::new();
            for asset in &extraction.assets {
                let normalized = normalized_image(&asset.bytes, &doc.limits)?;
                image_ids.insert(asset.id.clone(), normalized.id.clone());
                if !doc.assets.iter().any(|a| a.id == normalized.id) {
                    doc.assets.push(normalized);
                }
            }
            for detail in &extraction.gaps {
                gap(&mut doc, "document", detail)?;
            }
            if request.kind == "docx" {
                docx(&request.bytes, &mut doc, "docx", true)?;
            } else if request.kind == "pptx" {
                pptx::read(&request.bytes, &mut doc, &extraction, &image_ids)?;
            } else {
                if request.kind == "eml" {
                    metadata(&request.bytes, &mut doc, "email")?;
                }
                for source in &extraction.sources {
                    if request.kind == "eml" && source.parent.is_none() {
                        continue;
                    }
                    if source.status == "unprocessed" {
                        gap(
                            &mut doc,
                            &source.locator,
                            format!(
                                "Unprocessed attachment {} ({}) remains external",
                                source.name, source.media_type
                            ),
                        )?;
                        continue;
                    }
                    let is_docx = source.media_type
                        == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                        && source.parent.is_some();
                    if source.media_type == "text/html" {
                        html::read(&source.bytes, &mut doc, &source.locator, false)?;
                    } else if source.media_type == "message/rfc822" {
                        metadata(&source.bytes, &mut doc, &source.locator)?;
                    } else if is_docx {
                        docx(&source.bytes, &mut doc, &source.locator, true)?;
                    } else if source.media_type == "text/plain" {
                        push(
                            &mut doc,
                            &source.locator,
                            BlockContent::Paragraph {
                                text: String::from_utf8(source.bytes.clone())
                                    .map_err(|_| "MIME decoded text is not UTF-8")?,
                            },
                        )?;
                    } else if !source.text.is_empty() {
                        push(
                            &mut doc,
                            &source.locator,
                            BlockContent::Paragraph {
                                text: source.text.clone(),
                            },
                        )?;
                    }
                    // Typed DOCX figures already retain their paragraph/cell positions.
                    // MIME figures retain the qualified nearest-container source binding.
                    for occurrence in extraction
                        .occurrences
                        .iter()
                        .filter(|o| o.source_id == source.id && !is_docx)
                    {
                        push(
                            &mut doc,
                            &occurrence.locator,
                            BlockContent::Figure {
                                asset_id: image_ids
                                    .get(&occurrence.asset_id)
                                    .ok_or("missing normalized image mapping")?
                                    .clone(),
                                alt: occurrence
                                    .alt
                                    .clone()
                                    .unwrap_or_else(|| "Source figure".into()),
                            },
                        )?;
                    }
                }
                gap(
                    &mut doc,
                    "figures",
                    "Legacy projected figures retain exact source occurrences; visual placement relative to paragraphs requires source review",
                )?;
            }
        }
        "xlsx" => {
            let mut budget = Budget {
                limits,
                ..Default::default()
            };
            admit_package(&request.bytes, &mut budget)?;
            doc.consumed.expanded_bytes = budget.expanded_bytes;
            doc.consumed.package_parts = budget.package_parts;
            xlsx::read(&request.bytes, &mut doc)?;
        }
        "html" | "htm" => html::read(&request.bytes, &mut doc, "html", true)?,
        _ => return Err("unsupported prepared kind".into()),
    }
    if doc.blocks.is_empty() {
        push(
            &mut doc,
            "document",
            BlockContent::Metadata {
                name: "Source content".into(),
                value: "No substantive readable objects exposed".into(),
            },
        )?;
        gap(
            &mut doc,
            "document",
            "No substantive readable objects exposed; original source inspection required",
        )?;
    }
    doc.consumed.image_pixels = doc
        .assets
        .iter()
        .map(|a| u64::from(a.width) * u64::from(a.height))
        .sum();
    for _ in 0..5 {
        let size = serde_json::to_vec(&doc).map_err(|e| e.to_string())?.len();
        if size > doc.limits.max_output_bytes {
            return Err("prepared output quota".into());
        }
        if size == doc.consumed.output_bytes {
            break;
        }
        doc.consumed.output_bytes = size;
    }
    doc.validate()?;
    Ok(doc)
}

fn push(
    doc: &mut PreparedDocument,
    locator: impl AsRef<str>,
    content: BlockContent,
) -> Result<(), String> {
    let locator = locator.as_ref();
    if doc.blocks.len() >= MAX_SOURCES || locator.is_empty() || locator.len() > 8192 {
        return Err("prepared block/locator budget".into());
    }
    let total = serde_json::to_vec(&content)
        .map_err(|e| e.to_string())?
        .len();
    if total > MAX_TEXT_BYTES {
        return Err("prepared block text budget".into());
    }
    doc.blocks.push(PreparedBlock {
        id: format!("b{:04}", doc.blocks.len() + 1),
        locator: locator.into(),
        content,
    });
    Ok(())
}
fn gap(
    doc: &mut PreparedDocument,
    locator: impl AsRef<str>,
    detail: impl AsRef<str>,
) -> Result<(), String> {
    let locator = locator.as_ref();
    let detail = detail.as_ref();
    if doc.gaps.len() >= 2048 || detail.is_empty() || detail.len() > 16384 || locator.len() > 8192 {
        return Err("prepared gap budget".into());
    }
    doc.gaps.push(PreparedGap {
        locator: locator.into(),
        detail: detail.into(),
    });
    Ok(())
}
fn add_image(
    doc: &mut PreparedDocument,
    bytes: &[u8],
    locator: &str,
    alt: &str,
) -> Result<(), String> {
    match normalized_image(bytes, &doc.limits) {
        Ok(asset) => {
            let id = asset.id.clone();
            if !doc.assets.iter().any(|a| a.id == id) {
                let image_bytes = doc
                    .assets
                    .iter()
                    .map(|a| a.bytes.len())
                    .sum::<usize>()
                    .saturating_add(asset.bytes.len());
                let pixels = doc
                    .assets
                    .iter()
                    .map(|a| u64::from(a.width) * u64::from(a.height))
                    .sum::<u64>()
                    + u64::from(asset.width) * u64::from(asset.height);
                if doc.assets.len() >= MAX_ASSETS
                    || image_bytes > MAX_IMAGE_BYTES
                    || pixels > doc.limits.max_total_image_pixels
                {
                    return Err("prepared aggregate image budget".into());
                }
                doc.assets.push(asset);
            }
            push(
                doc,
                locator,
                BlockContent::Figure {
                    asset_id: id,
                    alt: alt.into(),
                },
            )
        }
        Err(reason) => gap(doc, locator, format!("Image unavailable: {reason}")),
    }
}

fn normalized_image(bytes: &[u8], limits: &IntakeLimits) -> Result<crate::ImageAsset, String> {
    let asset = image_asset(bytes, limits)?;
    if asset.media_type == "image/png" {
        return Ok(asset);
    }
    let decoded = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
        .map_err(|e| e.to_string())?;
    let mut output = std::io::Cursor::new(Vec::new());
    decoded
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    image_asset(output.get_ref(), limits)
}

fn metadata(bytes: &[u8], doc: &mut PreparedDocument, prefix: &str) -> Result<(), String> {
    use mail_parser::{HeaderValue, MessageParser};
    let message = MessageParser::default()
        .parse(bytes)
        .ok_or("unreadable message metadata")?;
    let mut counts = BTreeMap::new();
    for header in message.headers() {
        let name = header.name.as_str();
        let lower = name.to_ascii_lowercase();
        if !matches!(
            lower.as_str(),
            "from"
                | "sender"
                | "to"
                | "cc"
                | "bcc"
                | "date"
                | "subject"
                | "reply-to"
                | "message-id"
                | "in-reply-to"
                | "references"
                | "resent-from"
                | "resent-to"
                | "resent-cc"
                | "resent-bcc"
                | "resent-date"
                | "resent-message-id"
        ) && !lower.starts_with("list-")
            && !lower.starts_with("x-")
        {
            continue;
        }
        let count = counts.entry(lower).or_insert(0);
        *count += 1;
        let value = match &header.value {
            HeaderValue::Text(value) => value.to_string(),
            HeaderValue::TextList(values) => values
                .iter()
                .map(|v| v.as_ref())
                .collect::<Vec<_>>()
                .join(" "),
            HeaderValue::DateTime(value) => value.to_rfc822(),
            HeaderValue::Address(values) => values
                .iter()
                .map(|a| match (&a.name, &a.address) {
                    (Some(n), Some(a)) => format!("{n} <{a}>"),
                    (None, Some(a)) => a.to_string(),
                    (Some(n), None) => n.to_string(),
                    _ => "unknown".into(),
                })
                .collect::<Vec<_>>()
                .join(", "),
            _ => std::str::from_utf8(
                bytes
                    .get(header.offset_start as usize..header.offset_end as usize)
                    .ok_or("header offset")?,
            )
            .map_err(|_| "non UTF-8 header")?
            .trim()
            .to_string(),
        };
        push(
            doc,
            format!("{prefix}/header/{name}/{count}"),
            BlockContent::Metadata {
                name: name.into(),
                value,
            },
        )?;
    }
    Ok(())
}

fn docx(
    bytes: &[u8],
    doc: &mut PreparedDocument,
    prefix: &str,
    images: bool,
) -> Result<(), String> {
    let limits = docx_parse::ParseLimits {
        max_xml_bytes: super::MAX_PACKAGE_BYTES,
        max_xml_events: 200_000,
        max_xml_depth: 128,
        ..Default::default()
    };
    let (wire, parts) = docx_parse::parse_docx_s9_wire_parts_with_limits(
        bytes,
        docx_parse::S9ParseOptions::default(),
        &limits,
    )
    .map_err(|e| e.to_string())?;
    let options = ExportOptions {
        stories: Some(vec![
            StorySelection::Body,
            StorySelection::Headers,
            StorySelection::Footers,
            StorySelection::Footnotes,
            StorySelection::Endnotes,
            StorySelection::Comments,
        ]),
        max_blocks: Some(MAX_SOURCES as u32),
        max_bytes: Some(MAX_TEXT_BYTES as u32),
        ..ExportOptions::new(RevisionView::Accepted)
    };
    let structured = docx_edit::structured::export_package_structured(wire, &parts, &options)
        .map_err(|e| e.to_string())?;
    let parts: BTreeMap<_, _> = parts.into_iter().collect();
    let scope = crate::hex(&digest(prefix.as_bytes()));
    if structured.truncated {
        return Err("typed DOCX export truncated at prepared budget".into());
    }
    for story in &structured.stories {
        let locator = format!("{prefix}/story/{}", story.story);
        reference_gaps(&story.blocks, doc, &locator)?;
        if matches!(story.kind, StoryKind::Footnote | StoryKind::Endnote) {
            let kind = if story.kind == StoryKind::Footnote {
                "fn"
            } else {
                "en"
            };
            let label = note_label(&scope, kind, story.note_id.as_deref().unwrap_or("unknown"));
            let text = blocks_text(&story.blocks, &scope);
            push(doc, &locator, BlockContent::Footnote { label, text })?;
            if images {
                block_figures(&story.blocks, doc, &parts, &locator)?;
            }
            if story
                .blocks
                .iter()
                .any(|b| matches!(b.content, BlockKind::Table { .. }))
            {
                gap(
                    doc,
                    &locator,
                    "Footnote table prose retained; table layout needs source review",
                )?;
            }
            continue;
        }
        if story.kind != StoryKind::Body {
            push(
                doc,
                &locator,
                BlockContent::Metadata {
                    name: "Document section".into(),
                    value: match story.kind {
                        StoryKind::Header => "Header",
                        StoryKind::Footer => "Footer",
                        StoryKind::Comment => "Comments",
                        _ => "Document",
                    }
                    .into(),
                },
            )?;
        }
        if let Some(comment) = &story.comment {
            push(
                doc,
                format!("{locator}/annotation-context"),
                BlockContent::Metadata {
                    name: "Source comment context".into(),
                    value: format!(
                        "ID {}; author {:?}; date {:?}; source ranges {:?}; parent {:?}; resolved {}",
                        comment.id,
                        comment.author,
                        comment.date,
                        comment.anchors,
                        comment.parent_id,
                        comment.resolved
                    ),
                },
            )?;
        }
        project_blocks(&story.blocks, doc, &parts, &locator, images, &scope)?;
    }
    for diagnostic in structured.diagnostics {
        gap(
            doc,
            prefix,
            format!("{:?}: {}", diagnostic.code, diagnostic.message),
        )?;
    }
    Ok(())
}

fn reference_gaps(
    blocks: &[Block],
    doc: &mut PreparedDocument,
    prefix: &str,
) -> Result<(), String> {
    for block in blocks {
        let locator = format!("{prefix}/block/{}", block.id);
        match &block.content {
            BlockKind::Paragraph { paragraph }
            | BlockKind::Heading { paragraph, .. }
            | BlockKind::ListItem { paragraph, .. } => {
                inline_reference_gaps(&paragraph.inlines, doc, &locator)?
            }
            BlockKind::ContentControl { blocks, .. } => reference_gaps(blocks, doc, &locator)?,
            BlockKind::Table { table } => {
                for (r, row) in table.rows.iter().enumerate() {
                    for cell in &row.cells {
                        reference_gaps(
                            &cell.blocks,
                            doc,
                            &format!("{locator}/R{}C{}", r + 1, cell.column + 1),
                        )?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
fn inline_reference_gaps(
    inlines: &[Inline],
    doc: &mut PreparedDocument,
    prefix: &str,
) -> Result<(), String> {
    for (index, inline) in inlines.iter().enumerate() {
        let locator = format!("{prefix}/inline/{index}");
        match &inline.content {
            InlineKind::Field {
                field_type,
                instruction,
                cached_result,
                ..
            } => {
                if field_type.eq_ignore_ascii_case("NOTEREF")
                    || instruction
                        .split_whitespace()
                        .next()
                        .is_some_and(|name| name.eq_ignore_ascii_case("NOTEREF"))
                {
                    gap(
                        doc,
                        &locator,
                        format!(
                            "Word NOTEREF cached value retained; bookmark target and reused-note association are unresolved: {}",
                            instruction.trim()
                        ),
                    )?;
                }
                match cached_result {
                    CachedResult::Inline { inlines } => {
                        inline_reference_gaps(inlines, doc, &locator)?
                    }
                    CachedResult::Blocks { blocks } => reference_gaps(blocks, doc, &locator)?,
                    _ => {}
                }
            }
            InlineKind::ContentControl { inlines, .. } => {
                inline_reference_gaps(inlines, doc, &locator)?
            }
            _ => {}
        }
    }
    Ok(())
}

fn block_figures(
    blocks: &[Block],
    doc: &mut PreparedDocument,
    parts: &BTreeMap<String, Vec<u8>>,
    locator: &str,
) -> Result<(), String> {
    for (i, block) in blocks.iter().enumerate() {
        let location = format!("{locator}/block/{i}");
        match &block.content {
            BlockKind::Paragraph { paragraph }
            | BlockKind::Heading { paragraph, .. }
            | BlockKind::ListItem { paragraph, .. } => {
                inline_figures(&paragraph.inlines, doc, parts, &location)?
            }
            BlockKind::ContentControl { blocks, .. } => {
                block_figures(blocks, doc, parts, &location)?
            }
            BlockKind::Table { table } => {
                for (r, row) in table.rows.iter().enumerate() {
                    for (c, cell) in row.cells.iter().enumerate() {
                        block_figures(
                            &cell.blocks,
                            doc,
                            parts,
                            &format!("{location}/row/{r}/cell/{c}"),
                        )?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn note_label(scope: &str, kind: &str, id: &str) -> String {
    format!(
        "{kind}-{}-{}",
        &scope[..16],
        &crate::hex(&digest(id.as_bytes()))[..16]
    )
}
fn inline_text(inlines: &[Inline], scope: &str) -> String {
    let mut text = String::new();
    for inline in inlines {
        match &inline.content {
            InlineKind::Text { text: value } => text.push_str(value),
            InlineKind::Tab => text.push('\t'),
            InlineKind::Break { .. } => text.push('\n'),
            InlineKind::NoteReference {
                note_kind, note_id, ..
            } => text.push_str(&format!(
                "[^{}]",
                note_label(
                    scope,
                    match note_kind {
                        docx_edit::structured::NoteKind::Footnote => "fn",
                        docx_edit::structured::NoteKind::Endnote => "en",
                    },
                    note_id
                )
            )),
            InlineKind::Field { cached_result, .. } => match cached_result {
                CachedResult::Inline { inlines } => text.push_str(&inline_text(inlines, scope)),
                CachedResult::Blocks { blocks } => text.push_str(&blocks_text(blocks, scope)),
                CachedResult::Missing => text.push_str("[field result unavailable]"),
            },
            InlineKind::ContentControl { inlines, .. } => {
                text.push_str(&inline_text(inlines, scope))
            }
            _ => {}
        }
        if let Some(link) = &inline.link {
            text.push_str(&format!(" ({})", link.href));
        }
    }
    text
}

fn blocks_text(blocks: &[Block], scope: &str) -> String {
    blocks
        .iter()
        .map(|block| match &block.content {
            BlockKind::Paragraph { paragraph }
            | BlockKind::Heading { paragraph, .. }
            | BlockKind::ListItem { paragraph, .. } => inline_text(&paragraph.inlines, scope),
            BlockKind::Table { table } => table
                .rows
                .iter()
                .map(|row| {
                    row.cells
                        .iter()
                        .map(|cell| blocks_text(&cell.blocks, scope))
                        .collect::<Vec<_>>()
                        .join(" | ")
                })
                .collect::<Vec<_>>()
                .join("\n"),
            BlockKind::ContentControl { blocks, .. } => blocks_text(blocks, scope),
            _ => String::new(),
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}
fn inline_figures(
    inlines: &[Inline],
    doc: &mut PreparedDocument,
    parts: &BTreeMap<String, Vec<u8>>,
    locator: &str,
) -> Result<(), String> {
    for (i, inline) in inlines.iter().enumerate() {
        let location = format!("{locator}/inline/{i}");
        match &inline.content {
            InlineKind::Image {
                part,
                external_target,
                alt_text,
                ..
            } => {
                if external_target.is_some() {
                    gap(
                        doc,
                        &location,
                        "External image unavailable; no resource fetch",
                    )?;
                } else if let Some(bytes) = part.as_ref().and_then(|p| parts.get(p)) {
                    add_image(
                        doc,
                        bytes,
                        &location,
                        alt_text.as_deref().unwrap_or("Source figure"),
                    )?;
                } else {
                    gap(doc, &location, "Image package part unresolved")?;
                }
            }
            InlineKind::ContentControl { inlines, .. } => {
                inline_figures(inlines, doc, parts, &location)?
            }
            InlineKind::Field {
                cached_result: CachedResult::Inline { inlines },
                ..
            } => inline_figures(inlines, doc, parts, &location)?,
            InlineKind::Unsupported { element, .. } => gap(
                doc,
                &location,
                format!("Unsupported substantive inline {element}"),
            )?,
            _ => {}
        }
    }
    Ok(())
}
fn project_blocks(
    blocks: &[Block],
    doc: &mut PreparedDocument,
    parts: &BTreeMap<String, Vec<u8>>,
    prefix: &str,
    images: bool,
    scope: &str,
) -> Result<(), String> {
    for block in blocks {
        let locator = format!("{prefix}/block/{}", block.id);
        match &block.content {
            BlockKind::Paragraph { paragraph } => {
                if images {
                    paragraph_parts(&paragraph.inlines, doc, parts, &locator, scope)?;
                } else {
                    let text = inline_text(&paragraph.inlines, scope);
                    if !text.is_empty() {
                        push(doc, &locator, BlockContent::Paragraph { text })?;
                    }
                }
            }
            BlockKind::Heading { paragraph, heading } => {
                push(
                    doc,
                    &locator,
                    BlockContent::Heading {
                        text: inline_text(&paragraph.inlines, scope),
                        level: (heading.outline_level + 1).min(6),
                    },
                )?;
                gap_if_heading(doc, &locator, heading.outline_level)?;
                if images {
                    if has_images(&paragraph.inlines) {
                        gap(
                            doc,
                            &locator,
                            "Heading contains inline figures; figure location retained after heading text, inline layout requires source review",
                        )?;
                    }
                    inline_figures(&paragraph.inlines, doc, parts, &locator)?;
                }
            }
            BlockKind::ListItem {
                paragraph,
                list,
                heading,
            } => {
                let label = list.marker.clone().unwrap_or_default();
                if list.marker.is_none() {
                    gap(doc, &locator, "Source list label unresolved")?;
                }
                if let Some(heading) = heading {
                    push(
                        doc,
                        &locator,
                        BlockContent::Heading {
                            text: format!("{} {}", label, inline_text(&paragraph.inlines, scope))
                                .trim()
                                .into(),
                            level: (heading.outline_level + 1).min(6),
                        },
                    )?;
                    gap_if_heading(doc, &locator, heading.outline_level)?;
                } else {
                    push(
                        doc,
                        &locator,
                        BlockContent::ListItem {
                            text: inline_text(&paragraph.inlines, scope),
                            level: list.level,
                            label,
                        },
                    )?;
                }
                if images {
                    if has_images(&paragraph.inlines) {
                        gap(
                            doc,
                            &locator,
                            "List item contains inline figures; figure location retained after list text, inline layout requires source review",
                        )?;
                    }
                    inline_figures(&paragraph.inlines, doc, parts, &locator)?;
                }
            }
            BlockKind::Table { table } => {
                let rows = table
                    .rows
                    .iter()
                    .enumerate()
                    .map(|(r, row)| {
                        row.cells
                            .iter()
                            .filter(|cell| cell.row_span != 0)
                            .map(|cell| PreparedCell {
                                address: format!("R{}C{}", r + 1, cell.column + 1),
                                column: cell.column,
                                text: blocks_text(&cell.blocks, scope),
                                formula: None,
                                result_status: None,
                                comment: None,
                                links: vec![],
                                row_span: cell.row_span,
                                column_span: cell.grid_span,
                            })
                            .collect()
                    })
                    .collect();
                push(doc, &locator, BlockContent::Table { rows })?;
                for (r, row) in table.rows.iter().enumerate() {
                    for cell in &row.cells {
                        let cell_locator = format!("{locator}/R{}C{}", r + 1, cell.column + 1);
                        if cell.grid_span != 1 || cell.row_span != 1 {
                            gap(
                                doc,
                                &cell_locator,
                                "Merged cell coordinates retained; visual associations need source review",
                            )?;
                        }
                        if cell
                            .blocks
                            .iter()
                            .any(|b| matches!(b.content, BlockKind::Table { .. }))
                        {
                            gap(
                                doc,
                                &cell_locator,
                                "Nested table text retained in source cell; nested grid requires source review",
                            )?;
                        }
                        if images {
                            for block in &cell.blocks {
                                match &block.content {
                                    BlockKind::Paragraph { paragraph }
                                    | BlockKind::Heading { paragraph, .. }
                                    | BlockKind::ListItem { paragraph, .. } => inline_figures(
                                        &paragraph.inlines,
                                        doc,
                                        parts,
                                        &cell_locator,
                                    )?,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            BlockKind::ContentControl { blocks, .. } => {
                project_blocks(blocks, doc, parts, &locator, images, scope)?
            }
            BlockKind::Unsupported { element } => gap(
                doc,
                &locator,
                format!("Unsupported substantive block {element}"),
            )?,
            _ => {}
        }
    }
    Ok(())
}

fn has_images(inlines: &[Inline]) -> bool {
    inlines.iter().any(|inline| match &inline.content {
        InlineKind::Image { .. } => true,
        InlineKind::ContentControl { inlines, .. }
        | InlineKind::Field {
            cached_result: CachedResult::Inline { inlines },
            ..
        } => has_images(inlines),
        _ => false,
    })
}
fn paragraph_parts(
    inlines: &[Inline],
    doc: &mut PreparedDocument,
    parts: &BTreeMap<String, Vec<u8>>,
    locator: &str,
    scope: &str,
) -> Result<(), String> {
    let mut pending = String::new();
    let mut segment = 0;
    for (index, inline) in inlines.iter().enumerate() {
        let nested = match &inline.content {
            InlineKind::ContentControl { inlines, .. }
            | InlineKind::Field {
                cached_result: CachedResult::Inline { inlines },
                ..
            } if has_images(inlines) => Some(inlines.as_slice()),
            _ => None,
        };
        if matches!(inline.content, InlineKind::Image { .. }) || nested.is_some() {
            if !pending.is_empty() {
                push(
                    doc,
                    format!("{locator}/text/{segment}"),
                    BlockContent::Paragraph {
                        text: std::mem::take(&mut pending),
                    },
                )?;
                segment += 1;
            }
            let location = format!("{locator}/inline/{index}");
            if let Some(nested) = nested {
                paragraph_parts(nested, doc, parts, &location, scope)?;
            } else {
                inline_figures(std::slice::from_ref(inline), doc, parts, &location)?;
            }
            if let Some(link) = &inline.link {
                push(
                    doc,
                    format!("{location}/link"),
                    BlockContent::Metadata {
                        name: "Source link".into(),
                        value: link.href.clone(),
                    },
                )?;
            }
        } else {
            pending.push_str(&inline_text(std::slice::from_ref(inline), scope));
            if matches!(inline.content, InlineKind::Unsupported { .. }) {
                inline_figures(
                    std::slice::from_ref(inline),
                    doc,
                    parts,
                    &format!("{locator}/inline/{index}"),
                )?;
            }
        }
    }
    if !pending.is_empty() {
        push(
            doc,
            format!("{locator}/text/{segment}"),
            BlockContent::Paragraph { text: pending },
        )?;
    }
    Ok(())
}
fn gap_if_heading(doc: &mut PreparedDocument, locator: &str, outline: u8) -> Result<(), String> {
    if outline >= 6 {
        push(
            doc,
            format!("{locator}/source-depth"),
            BlockContent::Metadata {
                name: "Source heading level".into(),
                value: (outline + 1).to_string(),
            },
        )?;
        gap(
            doc,
            locator,
            "Source heading depth exceeds Markdown level six; presentation flattened to six with exact source level retained",
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read(kind: &str, bytes: &[u8]) -> PreparedDocument {
        let doc = prepare(HelperRequest {
            kind: kind.into(),
            bytes: bytes.into(),
            limits: None,
        })
        .unwrap();
        let markdown = crate::prepared::format(&doc).unwrap();
        crate::prepared::check_candidate(
            &doc,
            &ConversionCandidate {
                schema: 2,
                source_sha256: doc.source_sha256,
                markdown,
                page_reviews: vec![],
                crops: vec![],
                table_reviews: doc
                    .blocks
                    .iter()
                    .filter_map(|b| {
                        if let BlockContent::Table { rows } = &b.content {
                            Some(TableReview {
                                block_id: b.id.clone(),
                                source_sha256: crate::prepared::table_hash(rows).unwrap(),
                                uncertainty: String::new(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect(),
            },
        )
        .unwrap();
        doc
    }
    #[test]
    fn html_preserves_ordinal_nesting_and_sparse_spanned_cells() {
        let doc=read("html",br#"<h1>Guide</h1><script>drop 99</script><ol start="4"><li>Fourth<ol type="a"><li>Substep</li></ol></li><li value="8">Eighth</li></ol><table><caption>Limits</caption><tr><th>A</th><th colspan="2">B</th></tr><tr><td></td><td>12</td><td><a href="https://example.org">Proof</a></td></tr></table>"#);
        assert!(doc.blocks.iter().any(
            |b| matches!(&b.content,BlockContent::ListItem {label,level:0,..} if label=="4.")
        ));
        assert!(doc.blocks.iter().any(
            |b| matches!(&b.content,BlockContent::ListItem {label,level:1,..} if label=="a.")
        ));
        assert!(
            doc.blocks
                .iter()
                .any(|b| matches!(&b.content,BlockContent::ListItem {label,..} if label=="8."))
        );
        let table = doc
            .blocks
            .iter()
            .find_map(|b| match &b.content {
                BlockContent::Table { rows } => Some(rows),
                _ => None,
            })
            .unwrap();
        assert_eq!(table[1][0].text, "");
        assert_eq!(table[1][1].column, 1);
        assert_eq!(table[1][1].text, "12");
        assert_eq!(table[1][2].links, ["https://example.org"]);
        assert!(
            !serde_json::to_string(&doc.blocks)
                .unwrap()
                .contains("drop 99")
        );
        assert!(!doc.gaps.is_empty());
    }
    #[test]
    fn html_repeated_figures_remain_two_occurrences_of_one_asset() {
        use base64::Engine;
        let png = include_bytes!("../../tests/fixtures/water-use.png");
        let data = base64::engine::general_purpose::STANDARD.encode(png);
        let source = format!(
            "<p>First<img src=\"data:image/png;base64,{data}\"></p><p>Second<img src=\"data:image/png;base64,{data}\"></p><img src=\"https://example.org/missing.png\">"
        );
        let doc = read("html", source.as_bytes());
        assert_eq!(doc.assets.len(), 1);
        assert_eq!(
            doc.blocks
                .iter()
                .filter(|b| matches!(b.content, BlockContent::Figure { .. }))
                .count(),
            2
        );
        assert!(
            doc.gaps
                .iter()
                .any(|g| g.detail.contains("no filesystem/network"))
        );
    }
    #[test]
    fn email_keeps_duplicate_named_headers_and_unique_alternative_wording() {
        let doc=read("eml",b"From: Person <person@example.org>\r\nX-Decision: first\r\nX-Decision: second\r\nList-ID: example.test\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=alt\r\n\r\n--alt\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nPlain exception remains.\r\n--alt\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>HTML details remain.</p>\r\n--alt--\r\n");
        let metadata = doc
            .blocks
            .iter()
            .filter_map(|b| match &b.content {
                BlockContent::Metadata { name, value }
                    if name.eq_ignore_ascii_case("X-Decision") =>
                {
                    Some(value.as_str())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(metadata, ["first", "second"]);
        let blocks = serde_json::to_string(&doc.blocks).unwrap();
        assert!(blocks.contains("Plain exception remains."));
        assert!(blocks.contains("HTML details remain."));
    }
    #[test]
    fn maintained_docx_and_pptx_expose_bound_objects() {
        let docx = read("docx", include_bytes!("../../tests/fixtures/harbor.docx"));
        assert!(
            docx.blocks
                .iter()
                .any(|b| matches!(b.content, BlockContent::Table { .. }))
        );
        assert!(!docx.assets.is_empty());
        let pptx = read("pptx", include_bytes!("../../tests/fixtures/quay.pptx"));
        assert!(
            pptx.blocks
                .iter()
                .any(|b| matches!(b.content, BlockContent::Table { .. }))
        );
        assert!(!pptx.gaps.is_empty());
    }
    #[test]
    fn document_sections_are_human_labels_and_noteref_association_is_qualified() {
        let doc = read(
            "docx",
            include_bytes!("../../../brn-threads-intake/tests/fixtures/luna/footnotes.docx"),
        );
        assert!(doc.blocks.iter().any(|b|matches!(&b.content,BlockContent::Metadata {name,value} if name=="Document section"&&value=="Header")));
        assert!(doc.blocks.iter().any(|b|matches!(&b.content,BlockContent::Metadata {name,value} if name=="Document section"&&value=="Footer")));
        assert!(!doc.blocks.iter().any(
            |b| matches!(&b.content,BlockContent::Metadata {value,..} if value.contains("hf:rId"))
        ));
        assert!(doc.gaps.iter().any(|g| g.locator.contains("/inline/")
            && g.detail.contains("NOTEREF")
            && g.detail.contains("FreshMassRule")
            && g.detail.contains("unresolved")));
        assert_eq!(
            doc.blocks
                .iter()
                .filter(|b| matches!(b.content, BlockContent::Footnote { .. }))
                .count(),
            3
        );
    }
    #[test]
    fn inline_document_figure_stays_between_its_authored_text() {
        use docx_edit::structured::Anchor;
        let mut doc = read("html", b"<p>Fixture</p>");
        doc.blocks.clear();
        let inline = |id: &str, content| Inline {
            id: id.into(),
            anchor: Anchor::Paragraph {
                story: "body".into(),
                para_id: "p0".into(),
            },
            marks: None,
            link: None,
            revisions: vec![],
            content,
        };
        let inlines = [
            inline(
                "before",
                InlineKind::Text {
                    text: "Before figure".into(),
                },
            ),
            inline(
                "figure",
                InlineKind::Image {
                    alt_text: Some("Flow".into()),
                    relationship_id: Some("image".into()),
                    part: Some("media/figure.png".into()),
                    external_target: None,
                },
            ),
            inline(
                "after",
                InlineKind::Text {
                    text: "After figure".into(),
                },
            ),
        ];
        let parts = BTreeMap::from([(
            "media/figure.png".into(),
            include_bytes!("../../tests/fixtures/water-use.png").to_vec(),
        )]);
        paragraph_parts(
            &inlines,
            &mut doc,
            &parts,
            "document/p0",
            "0123456789abcdef",
        )
        .unwrap();
        assert!(
            matches!(&doc.blocks[0].content,BlockContent::Paragraph {text} if text=="Before figure")
        );
        assert!(matches!(doc.blocks[1].content, BlockContent::Figure { .. }));
        assert!(
            matches!(&doc.blocks[2].content,BlockContent::Paragraph {text} if text=="After figure")
        );
        doc.consumed.image_pixels = doc
            .assets
            .iter()
            .map(|a| u64::from(a.width) * u64::from(a.height))
            .sum();
        let markdown = crate::prepared::format(&doc).unwrap();
        crate::prepared::check_candidate(
            &doc,
            &ConversionCandidate {
                schema: 2,
                source_sha256: doc.source_sha256,
                markdown,
                page_reviews: vec![],
                crops: vec![],
                table_reviews: doc
                    .blocks
                    .iter()
                    .filter_map(|b| {
                        if let BlockContent::Table { rows } = &b.content {
                            Some(TableReview {
                                block_id: b.id.clone(),
                                source_sha256: crate::prepared::table_hash(rows).unwrap(),
                                uncertainty: String::new(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect(),
            },
        )
        .unwrap();
    }
    #[test]
    fn attachment_footnotes_do_not_collide_and_bind_their_own_assertions() {
        use base64::Engine;
        use std::io::{Cursor, Write};
        fn document(note: &str) -> Vec<u8> {
            let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
            let files=[
                ("[Content_Types].xml",r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#.to_string()),
                ("_rels/.rels",r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="root" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#.to_string()),
                ("word/_rels/document.xml.rels",r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="notes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes" Target="footnotes.xml"/></Relationships>"#.to_string()),
                ("word/document.xml",r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Claim</w:t><w:footnoteReference w:id="1"/></w:r></w:p></w:body></w:document>"#.to_string()),
                ("word/footnotes.xml",format!(r#"<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:footnote w:id="1"><w:p><w:r><w:t>{note}</w:t></w:r></w:p></w:footnote></w:footnotes>"#)),
            ];
            for (name, text) in files {
                zip.start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(text.as_bytes()).unwrap();
            }
            zip.finish().unwrap().into_inner()
        }
        let mut email="From: person@example.org\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=attachments\r\n\r\n".to_string();
        for (i, note) in ["First attachment note", "Second attachment note"]
            .iter()
            .enumerate()
        {
            let encoded = base64::engine::general_purpose::STANDARD.encode(document(note));
            email.push_str(&format!("--attachments\r\nContent-Type: application/vnd.openxmlformats-officedocument.wordprocessingml.document\r\nContent-Disposition: attachment; filename={i}.docx\r\nContent-Transfer-Encoding: base64\r\n\r\n{encoded}\r\n"));
        }
        email.push_str("--attachments--\r\n");
        let doc = read("eml", email.as_bytes());
        let notes = doc
            .blocks
            .iter()
            .filter_map(|b| match &b.content {
                BlockContent::Footnote { label, text } => Some((label, text)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(notes.len(), 2);
        assert_ne!(notes[0].0, notes[1].0);
        for (label, _) in notes {
            assert!(doc.blocks.iter().any(|b|matches!(&b.content,BlockContent::Paragraph {text} if text.contains(&format!("[^{label}]")))));
        }
    }
}
