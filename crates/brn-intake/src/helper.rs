//! Maintained parsing adapter. This module is linked only into the restricted helper.
use crate::*;
use docx_edit::structured::{
    Block, BlockKind, CachedResult, ExportOptions, Inline, InlineKind, MarkdownOptions,
    RevisionView, StorySelection,
};
use html5ever::tokenizer::{BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer};
use mail_parser::{Message, MessageParser, MimeHeaders, PartType};
use std::{collections::BTreeMap, io::Cursor};

const MAX_PACKAGE_BYTES: usize = 16 * 1024 * 1024;

mod pptx;

#[derive(Default)]
struct Budget {
    expanded_bytes: usize,
    package_parts: usize,
    mime_parts: usize,
    mime_depth: usize,
    limits: IntakeLimits,
}

pub fn extract(request: HelperRequest) -> Result<Extraction, String> {
    let limits = request.limits.clone().unwrap_or_default();
    limits.validate()?;
    extract_with_budget(
        request,
        &mut Budget {
            limits,
            ..Budget::default()
        },
        0,
    )
}
fn extract_with_budget(
    request: HelperRequest,
    budget: &mut Budget,
    source_base: usize,
) -> Result<Extraction, String> {
    if request.bytes.is_empty() || request.bytes.len() > budget.limits.max_input_bytes {
        return Err("input budget or empty input".into());
    }
    let media = match request.kind.as_str() {
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "eml" => "message/rfc822",
        _ => return Err("unsupported intake kind".into()),
    };
    let mut extraction = Extraction {
        limits: budget.limits.clone(),
        consumed: None,
        schema: 1,
        converter: CONVERTER.into(),
        original_sha256: digest(&request.bytes),
        markdown: String::new(),
        sources: vec![SourceNode {
            id: format!("source-{source_base}"),
            parent: None,
            name: format!("original.{}", request.kind),
            media_type: media.into(),
            locator: "original".into(),
            status: "partial".into(),
            bytes: request.bytes.clone(),
            text: String::new(),
        }],
        assets: Vec::new(),
        occurrences: Vec::new(),
        gaps: Vec::new(),
    };
    if request.kind == "docx" {
        docx(&request.bytes, 0, &mut extraction, budget)?;
    } else if request.kind == "pptx" {
        pptx::extract(&request.bytes, 0, &mut extraction, budget)?;
    } else {
        let message = MessageParser::default()
            .parse(&request.bytes)
            .ok_or("unreadable MIME message")?;
        email(&message, 0, 0, 0, &mut extraction, budget)?;
    }
    let decoded_bytes = extraction
        .sources
        .iter()
        .skip(1)
        .map(|s| s.bytes.len())
        .sum();
    extraction.consumed = Some(IntakeUsage {
        input_bytes: request.bytes.len(),
        expanded_bytes: budget.expanded_bytes,
        package_parts: budget.package_parts,
        mime_parts: budget.mime_parts,
        mime_depth: budget.mime_depth,
        decoded_bytes,
        image_pixels: extraction
            .assets
            .iter()
            .map(|a| u64::from(a.width) * u64::from(a.height))
            .sum(),
        output_bytes: 0,
    });
    if decoded_bytes > extraction.limits.max_decoded_bytes {
        return Err("decoded MIME payload quota".into());
    }
    for _ in 0..4 {
        let size = serde_json::to_vec(&extraction)
            .map_err(|e| e.to_string())?
            .len();
        let usage = extraction.consumed.as_mut().expect("set usage");
        if usage.output_bytes == size {
            break;
        }
        usage.output_bytes = size;
    }
    extraction.validate()?;
    Ok(extraction)
}

fn gap(extraction: &mut Extraction, message: impl Into<String>) -> Result<(), String> {
    if extraction.gaps.len() >= 2048 {
        return Err("gap count budget".into());
    }
    let message = message.into();
    if message.len() > 16384 {
        return Err("gap size budget".into());
    }
    extraction.gaps.push(message);
    Ok(())
}
fn append(extraction: &mut Extraction, text: &str) -> Result<usize, String> {
    if extraction.markdown.len().saturating_add(text.len()) > MAX_TEXT_BYTES {
        return Err("aggregate extracted text budget".into());
    }
    let start = extraction.markdown.len();
    extraction.markdown.push_str(text);
    Ok(start)
}
fn node(
    extraction: &mut Extraction,
    parent: usize,
    name: String,
    media_type: String,
    locator: String,
    status: &str,
    bytes: &[u8],
) -> Result<usize, String> {
    if bytes.len().saturating_add(
        extraction
            .sources
            .iter()
            .skip(1)
            .map(|s| s.bytes.len())
            .sum::<usize>(),
    ) > extraction.limits.max_decoded_bytes
    {
        return Err("decoded MIME payload quota".into());
    }
    if extraction.sources.len() >= MAX_SOURCES
        || bytes.len().saturating_add(
            extraction
                .sources
                .iter()
                .map(|s| s.bytes.len())
                .sum::<usize>(),
        ) > MAX_RETAINED_BYTES
    {
        return Err("retained MIME source budget".into());
    }
    let index = extraction.sources.len();
    let base = extraction.sources[0]
        .id
        .strip_prefix("source-")
        .and_then(|base| base.parse::<usize>().ok())
        .ok_or("invalid generated source namespace")?;
    let ordinal = base.checked_add(index).ok_or("source ordinal overflow")?;
    extraction.sources.push(SourceNode {
        id: format!("source-{ordinal}"),
        parent: Some(extraction.sources[parent].id.clone()),
        name,
        media_type,
        locator,
        status: status.into(),
        bytes: bytes.to_vec(),
        text: String::new(),
    });
    Ok(index)
}
fn fence(label: &str, value: &str) -> String {
    let longest = value.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let ticks = "`".repeat(longest.max(2) + 1);
    format!("\n{label}\n\n{ticks}\n{value}\n{ticks}\n")
}

fn image_asset(bytes: &[u8], limits: &IntakeLimits) -> Result<ImageAsset, String> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err("image byte budget".into());
    }
    let format = image::guess_format(bytes).map_err(|e| format!("image signature: {e}"))?;
    let media_type = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        _ => {
            return Err(
                "image format unprocessed: only complete PNG/JPEG decoding qualified".into(),
            );
        }
    };
    if format == image::ImageFormat::Png {
        let (width, height) =
            crate::decode_png(bytes, MAX_IMAGE_BYTES, 16384, limits.max_image_pixels)?;
        let sha256 = digest(bytes);
        return Ok(ImageAsset {
            id: format!("asset-{}", hex(&sha256)),
            sha256,
            width,
            height,
            media_type: media_type.into(),
            bytes: bytes.to_vec(),
        });
    }
    let dimensions = image::ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|e| format!("image dimensions: {e}"))?;
    if u64::from(dimensions.0) * u64::from(dimensions.1) > limits.max_image_pixels {
        return Err("image pixel budget".into());
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(MAX_IMAGE_PIXELS * 8);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|e| format!("incomplete or damaged image: {e}"))?;
    let sha256 = digest(bytes);
    Ok(ImageAsset {
        id: format!("asset-{}", hex(&sha256)),
        sha256,
        width: decoded.width(),
        height: decoded.height(),
        media_type: media_type.into(),
        bytes: bytes.to_vec(),
    })
}
fn add_asset(extraction: &mut Extraction, asset: ImageAsset) -> Result<String, String> {
    if !extraction
        .assets
        .iter()
        .any(|existing| existing.id == asset.id)
    {
        if extraction.assets.len() >= MAX_ASSETS
            || extraction
                .assets
                .iter()
                .map(|a| a.bytes.len())
                .sum::<usize>()
                .saturating_add(asset.bytes.len())
                > MAX_IMAGE_BYTES
            || extraction
                .assets
                .iter()
                .map(|a| u64::from(a.width) * u64::from(a.height))
                .sum::<u64>()
                + u64::from(asset.width) * u64::from(asset.height)
                > extraction.limits.max_total_image_pixels
        {
            return Err("aggregate image budget".into());
        }
        extraction.assets.push(asset.clone());
    }
    Ok(asset.id)
}
fn occurrence(
    extraction: &mut Extraction,
    source: usize,
    asset_id: String,
    locator: String,
    alt: Option<String>,
    start: usize,
    end: usize,
) -> Result<(), String> {
    if extraction.occurrences.len() >= MAX_OCCURRENCES {
        return Err("image occurrence budget".into());
    }
    extraction.occurrences.push(ImageOccurrence {
        id: format!("occurrence-{}", extraction.occurrences.len()),
        source_id: extraction.sources[source].id.clone(),
        asset_id,
        locator,
        alt,
        start,
        end,
    });
    Ok(())
}

struct Picture {
    anchor: docx_edit::structured::Anchor,
    token: String,
    locator: String,
    part: Option<String>,
    external: bool,
    alt: Option<String>,
}
// Only the upstream typed export is traversed. Word semantics and package relationships are
// already resolved by BetterOffice; this walk gives each image a unique rendering placeholder.
fn picture_inlines(
    inlines: &mut [Inline],
    pictures: &mut Vec<Picture>,
    source: &str,
) -> Result<(), String> {
    for inline in inlines {
        match &mut inline.content {
            InlineKind::Image {
                alt_text,
                part,
                relationship_id,
                external_target,
            } => {
                if pictures.len() >= MAX_OCCURRENCES {
                    return Err("DOCX image occurrence budget".into());
                }
                let anchor = serde_json::to_string(&inline.anchor).map_err(|e| e.to_string())?;
                let locator = format!(
                    "docx/export/{}/anchor={anchor}/relationship={}",
                    inline.id,
                    relationship_id.as_deref().unwrap_or("unresolved")
                );
                let token = format!(
                    "brnimage{}",
                    hex(&digest(
                        format!("{source}:{}:{locator}", pictures.len()).as_bytes()
                    ))
                );
                pictures.push(Picture {
                    anchor: inline.anchor.clone(),
                    token: token.clone(),
                    locator,
                    part: part.clone(),
                    external: external_target.is_some(),
                    alt: alt_text.clone(),
                });
                *alt_text = Some(token);
            }
            InlineKind::ContentControl { inlines, .. }
            | InlineKind::Field {
                cached_result: CachedResult::Inline { inlines },
                ..
            } => picture_inlines(inlines, pictures, source)?,
            InlineKind::Field {
                cached_result: CachedResult::Blocks { blocks },
                ..
            } => picture_blocks(blocks, pictures, source)?,
            _ => (),
        }
    }
    Ok(())
}
fn picture_blocks(
    blocks: &mut [Block],
    pictures: &mut Vec<Picture>,
    source: &str,
) -> Result<(), String> {
    for block in blocks {
        match &mut block.content {
            BlockKind::Paragraph { paragraph }
            | BlockKind::Heading { paragraph, .. }
            | BlockKind::ListItem { paragraph, .. } => {
                picture_inlines(&mut paragraph.inlines, pictures, source)?
            }
            BlockKind::Table { table } => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        picture_blocks(&mut cell.blocks, pictures, source)?;
                    }
                }
            }
            BlockKind::ContentControl { blocks, .. } => picture_blocks(blocks, pictures, source)?,
            _ => (),
        }
    }
    Ok(())
}
fn admit_package(bytes: &[u8], budget: &mut Budget) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("Office ZIP admission: {e}"))?;
    if archive.len().saturating_add(budget.package_parts) > budget.limits.max_package_parts {
        return Err("Office package member budget".into());
    }
    let parts: BTreeMap<String, Vec<u8>> = ooxml_opc::unzip_parts_with_limits(
        bytes,
        budget
            .limits
            .max_expanded_bytes
            .saturating_sub(budget.expanded_bytes) as u64,
    )
    .map_err(|e| format!("Office package admission: {e}"))?
    .into_iter()
    .collect();
    budget.package_parts += archive.len();
    budget.expanded_bytes += parts.values().map(Vec::len).sum::<usize>();
    Ok(parts)
}

fn docx(
    bytes: &[u8],
    source: usize,
    extraction: &mut Extraction,
    budget: &mut Budget,
) -> Result<(), String> {
    let parts = admit_package(bytes, budget)?;
    let limits = docx_parse::ParseLimits {
        max_xml_bytes: MAX_PACKAGE_BYTES,
        max_xml_events: 200_000,
        max_xml_depth: 128,
        ..Default::default()
    };
    let (wire, parsed_parts) = docx_parse::parse_docx_s9_wire_parts_with_limits(
        bytes,
        docx_parse::S9ParseOptions::default(),
        &limits,
    )
    .map_err(|e| format!("DOCX parse: {e}"))?;
    let options = ExportOptions {
        stories: Some(vec![
            StorySelection::Body,
            StorySelection::Headers,
            StorySelection::Footers,
            StorySelection::Footnotes,
            StorySelection::Endnotes,
            StorySelection::Comments,
        ]),
        max_blocks: Some(10_000),
        max_bytes: Some(MAX_TEXT_BYTES as u32),
        ..ExportOptions::new(RevisionView::Accepted)
    };
    let mut structured =
        docx_edit::structured::export_package_structured(wire, &parsed_parts, &options)
            .map_err(|e| e.to_string())?;
    let mut diagnostics = structured.diagnostics.clone();
    let mut pictures = Vec::new();
    for story in &mut structured.stories {
        picture_blocks(
            &mut story.blocks,
            &mut pictures,
            &extraction.sources[source].id,
        )?;
    }
    let rendered = docx_edit::structured::render_docx_markdown(
        &structured,
        &MarkdownOptions {
            max_bytes: Some(MAX_TEXT_BYTES as u32),
        },
    )
    .map_err(|e| e.to_string())?;
    for diagnostic in &rendered.diagnostics {
        if !diagnostics.contains(diagnostic) {
            diagnostics.push(diagnostic.clone());
        }
    }
    if rendered.truncated || structured.truncated {
        gap(
            extraction,
            format!(
                "{} DOCX export truncated at configured block/byte budget",
                extraction.sources[source].id
            ),
        )?;
    }
    gap(
        extraction,
        format!(
            "{}: Reflowed accepted-revision DOCX extraction; original layout, charts, SmartArt, shapes and unrepresented containers are not qualified. Separate authored picture titles unavailable; inspect retained original.",
            extraction.sources[source].id
        ),
    )?;
    for part in parts
        .keys()
        .filter(|p| p.contains("/charts/") || p.contains("/diagrams/"))
    {
        gap(
            extraction,
            format!(
                "{}: visual part {part} retained only in original; factual visual content unavailable",
                extraction.sources[source].id
            ),
        )?;
    }
    let mut replacements = Vec::new();
    for picture in pictures {
        let candidates = [
            format!("![{}]()", picture.token),
            format!("<img alt=\"{}\">", picture.token),
        ];
        let matches: Vec<_> = candidates
            .iter()
            .flat_map(|pattern| {
                rendered
                    .markdown
                    .match_indices(pattern)
                    .map(|(offset, text)| (offset, text.len()))
            })
            .collect();
        if matches.len() != 1 {
            gap(
                extraction,
                format!(
                    "{} {}: missing/ambiguous Markdown image join",
                    extraction.sources[source].id, picture.locator
                ),
            )?;
            for (start, len) in matches {
                replacements.push((
                    start,
                    len,
                    "[image unavailable; ambiguous upstream join]".into(),
                    None,
                ));
            }
            continue;
        }
        let (start, len) = matches[0];
        let asset = if picture.external {
            Err("external image target unavailable; no network fetch".into())
        } else {
            picture
                .part
                .as_ref()
                .and_then(|p| parts.get(p))
                .ok_or_else(|| "unresolved or missing package image part".to_string())
                .and_then(|bytes| image_asset(bytes, &budget.limits))
        };
        match asset {
            Ok(asset) => {
                let replacement = format!("![image]({})", asset_file_name(&asset)?);
                replacements.push((start, len, replacement, Some((asset, picture))));
            }
            Err(reason) => {
                gap(
                    extraction,
                    format!(
                        "{} {}: {reason}",
                        extraction.sources[source].id, picture.locator
                    ),
                )?;
                replacements.push((
                    start,
                    len,
                    "[image unavailable; inspect retained original]".into(),
                    None,
                ));
            }
        }
    }
    replacements.sort_by_key(|r| r.0);
    let mut markdown = String::new();
    let mut cursor = 0;
    let mut bound = Vec::new();
    for (start, len, replacement, image) in replacements {
        if start < cursor {
            return Err("overlapping upstream image joins".into());
        }
        markdown.push_str(&rendered.markdown[cursor..start]);
        let local_start = markdown.len();
        markdown.push_str(&replacement);
        if let Some((asset, picture)) = image {
            bound.push((local_start, markdown.len(), asset, picture));
        }
        cursor = start + len;
    }
    markdown.push_str(&rendered.markdown[cursor..]);
    let base = append(extraction, &markdown)?;
    extraction.sources[source].text = markdown;
    let mut resolved_images = Vec::new();
    for (start, end, asset, picture) in bound {
        resolved_images.push(picture.anchor.clone());
        let asset_id = add_asset(extraction, asset)?;
        let locator = format!(
            "{}/part={}",
            picture.locator,
            picture.part.as_deref().unwrap_or("unresolved")
        );
        occurrence(
            extraction,
            source,
            asset_id,
            locator,
            picture.alt,
            base + start,
            base + end,
        )?;
    }
    for diagnostic in diagnostics {
        if diagnostic.code == docx_edit::structured::DiagnosticCode::ImageDataOmitted
            && diagnostic
                .anchor
                .as_ref()
                .is_some_and(|anchor| resolved_images.contains(anchor))
        {
            continue;
        }
        let location = diagnostic
            .anchor
            .as_ref()
            .map(anchor_label)
            .unwrap_or_else(|| "document".into());
        let code = serde_json::to_value(diagnostic.code)
            .map_err(|e| e.to_string())?
            .as_str()
            .unwrap_or("diagnostic")
            .to_owned();
        gap(
            extraction,
            format!(
                "{} · {location} [{code}]: {}",
                extraction.sources[source].id, diagnostic.message
            ),
        )?;
    }
    Ok(())
}

fn anchor_label(anchor: &docx_edit::structured::Anchor) -> String {
    use docx_edit::structured::Anchor;
    match anchor {
        Anchor::Paragraph { story, para_id } => format!("{story}, paragraph {para_id}"),
        Anchor::Range(range) => format!(
            "{}, {}:{}–{}:{}",
            range.story,
            range.start.para_id,
            range.start.offset,
            range.end.para_id,
            range.end.offset
        ),
        Anchor::Table { story, table_index } => format!("{story}, table {table_index}"),
        Anchor::Control { story, control_id } => format!("{story}, control {control_id}"),
        Anchor::SourcePart { part, path, .. } => format!(
            "{part}, exported location {}",
            path.iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(".")
        ),
        Anchor::Unlocated { story, .. } => format!("{story}, exact location unavailable"),
    }
}

#[derive(Default)]
struct ImageReferences {
    references: Vec<(String, Option<String>)>,
    remote: bool,
    excessive: bool,
}
impl TokenSink for ImageReferences {
    type Handle = ();
    fn process_token(&mut self, token: Token, _: u64) -> TokenSinkResult<()> {
        if let Token::TagToken(tag) = token
            && tag.kind == TagKind::StartTag
            && tag.name.as_ref() == "img"
        {
            let attr = |name: &str| {
                tag.attrs
                    .iter()
                    .find(|a| a.name.local.as_ref() == name)
                    .map(|a| a.value.to_string())
            };
            if let Some(src) = attr("src") {
                if self.references.len() >= MAX_OCCURRENCES {
                    self.excessive = true;
                } else if let Some(cid) = src
                    .get(..4)
                    .filter(|s| s.eq_ignore_ascii_case("cid:"))
                    .and_then(|_| src.get(4..))
                {
                    self.references.push((cid.to_owned(), attr("alt")));
                } else {
                    self.remote = true;
                }
            }
        }
        TokenSinkResult::Continue
    }
}
fn html_images(html: &str) -> Result<ImageReferences, String> {
    let mut tokenizer = Tokenizer::new(ImageReferences::default(), Default::default());
    let mut input = BufferQueue::default();
    input.push_back(html.into());
    let _ = tokenizer.feed(&mut input);
    tokenizer.end();
    if tokenizer.sink.excessive {
        return Err("HTML image reference budget".into());
    }
    Ok(tokenizer.sink)
}
fn media(part: &mail_parser::MessagePart<'_>) -> String {
    part.content_type()
        .map(|c| format!("{}/{}", c.ctype(), c.subtype().unwrap_or("octet-stream")))
        .unwrap_or_else(|| {
            match &part.body {
                PartType::Text(_) => "text/plain",
                PartType::Html(_) => "text/html",
                PartType::Multipart(_) => "multipart/mixed",
                PartType::Message(_) => "message/rfc822",
                _ => "application/octet-stream",
            }
            .into()
        })
}
fn email(
    message: &Message<'_>,
    source: usize,
    depth: usize,
    base_depth: usize,
    extraction: &mut Extraction,
    budget: &mut Budget,
) -> Result<(), String> {
    if depth > 16
        || message.parts.is_empty()
        || message.parts.len().saturating_add(budget.mime_parts) > budget.limits.max_mime_parts
    {
        return Err("MIME depth/part budget".into());
    }
    budget.mime_parts += message.parts.len();
    let mut metadata =
        String::from("\nEmail metadata (decoded; exact raw headers retained in original)\n");
    let addresses = |value: Option<&mail_parser::Address<'_>>| {
        value
            .map(|value| {
                value
                    .iter()
                    .map(
                        |address| match (address.name.as_deref(), address.address.as_deref()) {
                            (Some(name), Some(address)) => format!("{name} <{address}>"),
                            (None, Some(address)) => address.to_owned(),
                            (Some(name), None) => format!("{name} <address unknown>"),
                            _ => "unknown".into(),
                        },
                    )
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".into())
    };
    let ids = |name| {
        // The convenience getters select only the last physical header, and
        // as_text() selects only the last TextList item. Preserve both orders.
        let decoded = message
            .header_values(name)
            .filter_map(|value| value.as_text_list())
            .flatten()
            .map(|id| id.as_ref())
            .collect::<Vec<_>>()
            .join(" ");
        if decoded.is_empty() {
            "unknown".into()
        } else {
            decoded
        }
    };
    for (label, value) in [
        ("From", addresses(message.from())),
        ("To", addresses(message.to())),
        ("Cc", addresses(message.cc())),
        ("Subject", message.subject().unwrap_or("unknown").to_owned()),
        (
            "Date",
            message
                .date()
                .map(|date| date.to_rfc822())
                .unwrap_or_else(|| "unknown".into()),
        ),
        (
            "Message-ID",
            message.message_id().unwrap_or("unknown").to_owned(),
        ),
        ("In-Reply-To", ids(mail_parser::HeaderName::InReplyTo)),
        ("References", ids(mail_parser::HeaderName::References)),
    ] {
        metadata.push_str(&fence(label, &value));
    }
    metadata.push_str(&fence(
        "Original Date/timezone header",
        message.header_raw("Date").unwrap_or("unknown"),
    ));
    append(extraction, &metadata)?;
    extraction.sources[source].text = metadata;
    gap(
        extraction,
        format!(
            "{}: decoded email headers are source claims; sender authenticity and thread relationships have not been independently verified",
            extraction.sources[source].id
        ),
    )?;
    let mut contexts = MimeContext {
        message,
        source,
        depth,
        base_depth,
        targets: BTreeMap::new(),
        references: Vec::new(),
    };
    contexts.visit(0, source, None, 0, extraction, budget)?;
    for (owner, scope, references) in contexts.references {
        if references.remote {
            gap(
                extraction,
                format!(
                    "{}: remote/non-CID HTML images unavailable; no network fetch",
                    extraction.sources[owner].id
                ),
            )?;
        }
        for (ordinal, (cid, alt)) in references.references.into_iter().enumerate() {
            let targets = scope
                .as_ref()
                .and_then(|scope| contexts.targets.get(&(scope.clone(), cid.clone())));
            let Some(target) = targets
                .filter(|targets| targets.len() == 1)
                .and_then(|targets| targets.first())
                .copied()
            else {
                gap(
                    extraction,
                    format!(
                        "{} HTML image {ordinal}: missing/ambiguous CID {cid} in actual related container",
                        extraction.sources[owner].id
                    ),
                )?;
                continue;
            };
            match image_asset(&extraction.sources[target].bytes, &budget.limits) {
                Ok(asset) => {
                    let link = format!("\n![inline image]({})\n", asset_file_name(&asset)?);
                    let start = append(extraction, &link)?;
                    let asset_id = add_asset(extraction, asset)?;
                    let locator = format!(
                        "{}/html-image/{ordinal}/cid={cid}/target={}",
                        extraction.sources[owner].locator, extraction.sources[target].locator
                    );
                    occurrence(
                        extraction,
                        owner,
                        asset_id,
                        locator,
                        alt,
                        start,
                        start + link.len(),
                    )?;
                }
                Err(error) => gap(
                    extraction,
                    format!("{} CID {cid}: {error}", extraction.sources[owner].id),
                )?,
            }
        }
    }
    Ok(())
}
/// Map the local root onto the already-retained attachment and retain every
/// descendant. Work on a bounded candidate so even a late aggregate refusal is atomic.
fn merge_attachment(
    extraction: &mut Extraction,
    attachment: usize,
    local: Extraction,
) -> Result<(), String> {
    local.validate()?;
    if local.sources[0].bytes != extraction.sources[attachment].bytes {
        return Err("attachment extraction differs from retained original".into());
    }
    let mut candidate = extraction.clone();
    let base = append(&mut candidate, &local.markdown)?;
    candidate.sources[attachment].text = local.sources[0].text.clone();
    candidate.sources[attachment].status = local.sources[0].status.clone();
    // The MIME declaration may be generic. Successful typed parsing establishes
    // the retained package kind; exact declared headers stay in the original EML.
    candidate.sources[attachment].media_type = local.sources[0].media_type.clone();
    let mut sources = BTreeMap::from([(local.sources[0].id.clone(), attachment)]);
    for source in local.sources.iter().skip(1) {
        let parent = source
            .parent
            .as_ref()
            .and_then(|id| sources.get(id))
            .copied()
            .ok_or("attachment source parent unavailable")?;
        let index = node(
            &mut candidate,
            parent,
            source.name.clone(),
            source.media_type.clone(),
            source.locator.clone(),
            &source.status,
            &source.bytes,
        )?;
        candidate.sources[index].text = source.text.clone();
        sources.insert(source.id.clone(), index);
    }
    for asset in local.assets {
        add_asset(&mut candidate, asset)?;
    }
    for image in local.occurrences {
        let source = *sources
            .get(&image.source_id)
            .ok_or("attachment image source unavailable")?;
        occurrence(
            &mut candidate,
            source,
            image.asset_id,
            image.locator,
            image.alt,
            base + image.start,
            base + image.end,
        )?;
    }
    for message in local.gaps {
        let remapped = sources
            .iter()
            .find_map(|(old, index)| {
                message
                    .strip_prefix(old)
                    .filter(|rest| rest.starts_with([' ', ':', '·']))
                    .map(|rest| format!("{}{rest}", candidate.sources[*index].id))
            })
            .unwrap_or_else(|| format!("{}: {message}", candidate.sources[attachment].id));
        gap(&mut candidate, remapped)?;
    }
    candidate.validate()?;
    *extraction = candidate;
    Ok(())
}

struct MimeContext<'m, 'b> {
    message: &'m Message<'b>,
    source: usize,
    depth: usize,
    base_depth: usize,
    targets: BTreeMap<(String, String), Vec<usize>>,
    references: Vec<(usize, Option<String>, ImageReferences)>,
}
impl MimeContext<'_, '_> {
    fn visit(
        &mut self,
        part_id: u32,
        parent: usize,
        scope: Option<String>,
        nesting: usize,
        extraction: &mut Extraction,
        budget: &mut Budget,
    ) -> Result<(), String> {
        let actual_depth = self.base_depth + nesting;
        if actual_depth > budget.limits.max_mime_depth {
            return Err("MIME nesting quota".into());
        }
        budget.mime_depth = budget.mime_depth.max(actual_depth);
        let part = self
            .message
            .parts
            .get(part_id as usize)
            .ok_or("invalid MIME part reference")?;
        let media_type = media(part);
        // A single-part attachment is still a child of the email evidence.
        // The email root retains raw EML/headers, not decoded document bytes.
        let body_root = part_id == 0
            && part.attachment_name().is_none()
            && !matches!(
                part.body,
                PartType::Binary(_) | PartType::InlineBinary(_) | PartType::Message(_)
            );
        let index = if body_root {
            self.source
        } else {
            node(
                extraction,
                parent,
                part.attachment_name().unwrap_or("unnamed MIME part").into(),
                media_type.clone(),
                format!("mime/{}/part/{part_id}", self.source),
                if matches!(part.body, PartType::Multipart(_)) {
                    "container"
                } else {
                    "partial"
                },
                part.contents(),
            )?
        };
        if part.is_encoding_problem {
            gap(
                extraction,
                format!(
                    "{}: MIME transfer/charset decoding problem; inspect original",
                    extraction.sources[index].id
                ),
            )?;
        }
        let scope = if media_type.eq_ignore_ascii_case("multipart/related") {
            Some(extraction.sources[index].id.clone())
        } else {
            scope
        };
        if let (Some(scope), Some(cid)) = (&scope, part.content_id()) {
            self.targets
                .entry((scope.clone(), cid.trim_matches(['<', '>']).to_owned()))
                .or_default()
                .push(index);
        }
        match &part.body {
            PartType::Multipart(children) => {
                for child in children {
                    self.visit(
                        *child,
                        index,
                        scope.clone(),
                        nesting + 1,
                        extraction,
                        budget,
                    )?;
                }
            }
            PartType::Message(message) => email(
                message,
                index,
                self.depth + 1,
                self.base_depth + nesting + 1,
                extraction,
                budget,
            )?,
            PartType::Text(text) | PartType::Html(text) if part.attachment_name().is_none() => {
                let label = if matches!(part.body, PartType::Html(_)) {
                    "Actual HTML alternative (inert quoted source)"
                } else {
                    "Actual plain-text body"
                };
                let output = fence(label, text);
                append(extraction, &output)?;
                extraction.sources[index].text.push_str(&output);
                if matches!(part.body, PartType::Html(_)) {
                    self.references.push((index, scope, html_images(text)?));
                }
            }
            _ => {
                let office_kind = if media_type.eq_ignore_ascii_case(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                ) || part
                    .attachment_name()
                    .is_some_and(|name| name.to_ascii_lowercase().ends_with(".docx"))
                {
                    Some("docx")
                } else if media_type.eq_ignore_ascii_case(
                    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                ) || part
                    .attachment_name()
                    .is_some_and(|name| name.to_ascii_lowercase().ends_with(".pptx"))
                {
                    Some("pptx")
                } else {
                    None
                };
                if let Some(kind) = office_kind {
                    append(
                        extraction,
                        &fence(
                            &format!("{} attachment", kind.to_ascii_uppercase()),
                            part.attachment_name().unwrap_or("unnamed attachment"),
                        ),
                    )?;
                    // Conversion and merge are isolated: a refusal cannot leak child
                    // sources, assets or ranges into the retained parent envelope.
                    let local_request = HelperRequest {
                        limits: Some(budget.limits.clone()),
                        kind: kind.into(),
                        bytes: part.contents().to_vec(),
                    };
                    let converted = extract_with_budget(local_request, budget, index)
                        .and_then(|local| merge_attachment(extraction, index, local));
                    if let Err(error) = converted {
                        extraction.sources[index].status = "unprocessed".into();
                        gap(
                            extraction,
                            format!(
                                "{}: {} unprocessed: {error}",
                                extraction.sources[index].id,
                                kind.to_ascii_uppercase()
                            ),
                        )?;
                    }
                } else {
                    extraction.sources[index].status = if part.content_id().is_some() {
                        "retained-inline"
                    } else {
                        "unprocessed"
                    }
                    .into();
                    gap(
                        extraction,
                        format!(
                            "{}: {} retained byte-for-byte, unprocessed/unsupported attachment or inline resource",
                            extraction.sources[index].id, media_type
                        ),
                    )?;
                }
            }
        }
        Ok(())
    }
}
