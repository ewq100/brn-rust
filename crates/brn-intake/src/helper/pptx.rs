//! Projection of the maintained PresentationML model and its source inventory.
//! No XML interpretation, relationship repair or slide rendering occurs here.
use super::*;
use pptx_parse::{GraphicFrameData, PptxPackage, ShapeNode, SourceShape};
use std::collections::BTreeSet;

const SLIDE: &str = "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";
const NOTES: &str = "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml";
const SHAPE: &str = "application/x-brn-pptx-extracted-shape";

pub(super) fn extract(
    bytes: &[u8],
    source: usize,
    extraction: &mut Extraction,
    budget: &mut Budget,
) -> Result<(), String> {
    drop(admit_package(bytes, budget)?);
    // These are aggregate model/XML budgets, in addition to OPC admission and
    // the owned process deadline. Avoid the much larger upstream defaults.
    let limits = pptx_parse::ParseLimits {
        max_xml_bytes: MAX_PACKAGE_BYTES,
        max_xml_events: 200_000,
        max_xml_text_bytes: MAX_TEXT_BYTES,
        max_xml_depth: 128,
        max_attributes_per_element: 128,
        max_attribute_bytes: 64 * 1024,
        max_relationships: 10_000,
        max_shapes: MAX_SOURCES,
        max_paragraphs: 10_000,
        max_runs: 50_000,
        max_comments: MAX_SOURCES,
    };
    let package = pptx_parse::parse_pptx_with_limits(bytes, &limits)
        .map_err(|e| format!("PPTX parse: {e}"))?;
    for entries in package.relationships.values() {
        let mut ids = BTreeSet::new();
        if entries.iter().any(|entry| !ids.insert(&entry.id)) {
            return Err("PPTX ambiguous duplicate package relationship ID".into());
        }
    }
    if package.slides.len() != package.presentation.slides.len() {
        return Err("PPTX presentation/slide inventory mismatch".into());
    }
    check_consumed_singletons(&package)?;
    located_gap(
        extraction,
        source,
        "presentation",
        "Partial reflowed PPTX evidence: authored slide text, table cells, presenter-note body and embedded PNG/JPEG pictures. Layout, spatial meaning, cropping, rotation, animation, charts and SmartArt are not rendered. Inspect retained original; no cleanup completeness claim.",
    )?;
    for (ordinal, (slide, reference)) in package
        .slides
        .iter()
        .zip(&package.presentation.slides)
        .enumerate()
    {
        if slide.part_path != reference.part_path {
            return Err("PPTX presentation order/part correspondence mismatch".into());
        }
        let hidden = slide.hidden == Some(true);
        let label = format!(
            "Slide {}{}",
            ordinal + 1,
            if hidden { " (hidden)" } else { "" }
        );
        let slide_source = node(
            extraction,
            source,
            label.clone(),
            SLIDE.into(),
            format!("pptx/slide/{}/part={}", ordinal + 1, slide.part_path),
            "partial",
            package
                .part_bytes(&slide.part_path)
                .ok_or("PPTX slide part unavailable")?,
        )?;
        let inventory = package
            .slide_source(&slide.part_path)
            .ok_or("PPTX slide source inventory unavailable")?;
        let mut ids = BTreeSet::new();
        check_shapes(&slide.shapes, &inventory.shapes, &mut ids)?;
        omitted(extraction, slide_source, &inventory.omitted)?;
        let mut projection = Projection {
            package: &package,
            extraction,
            part: &slide.part_path,
        };
        projection.shapes(&slide.shapes, &inventory.shapes, slide_source, hidden)?;
        if slide.background_picture.is_some() || slide.background_reference.is_some() {
            located_gap(
                projection.extraction,
                slide_source,
                &slide.part_path,
                "Slide background picture/style is retained in original; its visual meaning is not projected",
            )?;
        }
        if let Some(layout) = &slide.layout_part_path {
            located_gap(
                projection.extraction,
                slide_source,
                layout,
                "Inherited layout text, pictures and visual content are retained in original, not projected as authored slide evidence",
            )?;
            if let Some(master) = package
                .layouts
                .iter()
                .find(|item| &item.part_path == layout)
                .and_then(|item| item.master_part_path.as_ref())
            {
                located_gap(
                    projection.extraction,
                    slide_source,
                    master,
                    "Inherited master content is retained in original, not projected as authored slide evidence",
                )?;
            }
        }
        if let Some(notes) = package.notes_source(&slide.part_path) {
            let notes_source = node(
                projection.extraction,
                slide_source,
                format!("{label} · Presenter notes"),
                NOTES.into(),
                format!("pptx/notes/part={}", notes.part_path),
                "partial",
                package
                    .part_bytes(&notes.part_path)
                    .ok_or("PPTX notes part unavailable")?,
            )?;
            wording(
                projection.extraction,
                notes_source,
                "Presenter-note body",
                &slide.notes,
            )?;
            if notes.other_text_shapes != 0 {
                located_gap(
                    projection.extraction,
                    notes_source,
                    &notes.part_path,
                    format!(
                        "{} other notes text shapes are not represented by the presenter-note body; inspect original",
                        notes.other_text_shapes
                    ),
                )?;
            }
            located_gap(
                projection.extraction,
                notes_source,
                &notes.part_path,
                "Notes-page drawings, decorations and non-body content are not qualified by the presenter-note body projection",
            )?;
        } else if !slide.notes.is_empty() {
            return Err("PPTX notes text has no source inventory".into());
        } else if package
            .relationships
            .get(&slide.part_path)
            .is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| entry.relationship_type.ends_with("/notesSlide"))
            })
        {
            located_gap(
                projection.extraction,
                slide_source,
                &slide.part_path,
                "Referenced notes page is missing or unavailable; presenter-note content not extracted",
            )?;
        }
        if package
            .comments
            .iter()
            .any(|comment| comment.slide_part_path == slide.part_path)
        {
            located_gap(
                projection.extraction,
                slide_source,
                &slide.part_path,
                "Slide comments are retained in original, not projected as authored slide wording",
            )?;
        }
    }
    Ok(())
}

/// The maintained parser selects the first edge for these specific joins.
/// Check the same selectors before projecting anything, rather than silently
/// discarding a second notes page/root/layout or reporting guessed inheritance.
/// Slide lists, master lists and image families are deliberately not singletons.
fn check_consumed_singletons(package: &PptxPackage) -> Result<(), String> {
    singleton_relationship(package, "", "officeDocument", |edge| {
        edge.has_type("/officeDocument")
    })?;
    for slide in &package.slides {
        singleton_relationship(package, &slide.part_path, "notesSlide", |edge| {
            edge.is_type(pptx_parse::relationship_types::NOTES_SLIDE)
        })?;
        singleton_relationship(package, &slide.part_path, "slideLayout", |edge| {
            edge.has_type(pptx_parse::relationship_types::SLIDE_LAYOUT)
        })?;
        if let Some(layout) = slide.layout_part_path.as_ref().and_then(|path| {
            package
                .layouts
                .iter()
                .find(|layout| &layout.part_path == path)
        }) {
            singleton_relationship(package, &layout.part_path, "slideMaster", |edge| {
                edge.has_type(pptx_parse::relationship_types::SLIDE_MASTER)
            })?;
        }
    }
    Ok(())
}

fn singleton_relationship(
    package: &PptxPackage,
    part: &str,
    kind: &str,
    selector: impl Fn(&pptx_parse::Relationship) -> bool,
) -> Result<(), String> {
    let count = package
        .relationships
        .get(part)
        .into_iter()
        .flatten()
        .filter(|edge| selector(edge))
        .take(2)
        .count();
    if count > 1 {
        return Err(format!(
            "PPTX ambiguous {kind} relationships in {}",
            if part.is_empty() {
                "package root"
            } else {
                part
            }
        ));
    }
    Ok(())
}

fn check_shapes(
    shapes: &[ShapeNode],
    sources: &[SourceShape],
    ids: &mut BTreeSet<u32>,
) -> Result<(), String> {
    if shapes.len() != sources.len() {
        return Err("PPTX modeled/source shape count mismatch".into());
    }
    for (shape, source) in shapes.iter().zip(sources) {
        let element = source.element.rsplit(':').next().unwrap_or("");
        let matches = match shape {
            ShapeNode::Shape(_) => matches!(element, "sp" | "cxnSp"),
            ShapeNode::Picture(_) => element == "pic",
            ShapeNode::GraphicFrame(_) => element == "graphicFrame",
            ShapeNode::Group(_) => element == "grpSp",
        };
        if !matches || source.id != shape.id() || !ids.insert(source.id) || source.path.is_empty() {
            return Err("PPTX ambiguous modeled/source shape correspondence".into());
        }
        if let ShapeNode::Group(group) = shape {
            check_shapes(&group.children, &source.children, ids)?;
        } else if !source.children.is_empty() {
            return Err("PPTX unexpected source shape children".into());
        }
    }
    Ok(())
}

fn path_label(path: &[u32]) -> String {
    path.iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(".")
}
fn located_gap(
    extraction: &mut Extraction,
    source: usize,
    locator: &str,
    message: impl AsRef<str>,
) -> Result<(), String> {
    gap(
        extraction,
        format!(
            "{} · {locator}: {}",
            extraction.sources[source].id,
            message.as_ref()
        ),
    )
}
fn omitted(
    extraction: &mut Extraction,
    source: usize,
    items: &[pptx_parse::OmittedElement],
) -> Result<(), String> {
    for item in items {
        located_gap(
            extraction,
            source,
            &format!("element-path={}", path_label(&item.path)),
            format!(
                "Unrepresented {}{}; relationships {:?}; retained in original, factual content unavailable",
                item.element,
                if item.hidden { " (hidden)" } else { "" },
                item.relationship_ids
            ),
        )?;
    }
    Ok(())
}
/// Distinct location wrappers keep duplicate authored wording attributable even
/// when two slides or two attachments contain the same text and image bytes.
fn wording(
    extraction: &mut Extraction,
    source: usize,
    label: &str,
    value: &str,
) -> Result<(), String> {
    if value.is_empty() {
        return Ok(());
    }
    let location = format!(
        "{} · {}",
        extraction.sources[source].id, extraction.sources[source].locator
    );
    let rendered = format!(
        "\n{location}\n{}{}",
        fence("Source location", &extraction.sources[source].name),
        fence(label, value)
    );
    append(extraction, &rendered)?;
    extraction.sources[source].text = rendered;
    Ok(())
}
fn text(body: &pptx_parse::TextBody) -> String {
    body.paragraphs
        .iter()
        .map(|paragraph| {
            paragraph
                .runs
                .iter()
                .map(|run| {
                    if run.line_break {
                        "\n".to_owned()
                    } else {
                        run.text.clone()
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

struct Projection<'a, 'b> {
    package: &'a PptxPackage,
    extraction: &'b mut Extraction,
    part: &'a str,
}
impl Projection<'_, '_> {
    fn shapes(
        &mut self,
        shapes: &[ShapeNode],
        sources: &[SourceShape],
        parent: usize,
        hidden_parent: bool,
    ) -> Result<(), String> {
        for (shape, inventory) in shapes.iter().zip(sources) {
            let base = match shape {
                ShapeNode::Shape(shape) => &shape.base,
                ShapeNode::Picture(picture) => &picture.base,
                ShapeNode::GraphicFrame(frame) => &frame.base,
                ShapeNode::Group(group) => &group.base,
            };
            let hidden = hidden_parent || base.hidden;
            let locator = format!(
                "pptx/part={}/shape={}/element-path={}",
                self.part,
                base.id,
                path_label(&inventory.path)
            );
            let index = node(
                self.extraction,
                parent,
                format!("{}{}", base.name, if hidden { " (hidden)" } else { "" }),
                SHAPE.into(),
                locator.clone(),
                "partial",
                &[],
            )?;
            omitted(self.extraction, index, &inventory.omitted)?;
            for inline in &inventory.omitted_inlines {
                located_gap(
                    self.extraction,
                    index,
                    &format!("element-path={}", path_label(&inline.path)),
                    format!(
                        "Unrepresented inline {} at paragraph {}, table cell {:?}; retained in original",
                        inline.element,
                        inline.paragraph + 1,
                        inline.cell
                    ),
                )?;
            }
            if inventory.media.is_some() {
                located_gap(
                    self.extraction,
                    index,
                    &locator,
                    format!(
                        "Unsupported {:?} media frame retained in original; poster is not the media content",
                        inventory.media
                    ),
                )?;
                continue;
            }
            match shape {
                ShapeNode::Shape(shape) => {
                    if let Some(body) = &shape.text {
                        wording(self.extraction, index, "Authored slide text", &text(body))?;
                    }
                    located_gap(
                        self.extraction,
                        index,
                        &locator,
                        "Shape geometry, connectors, style and spatial relationships are not rendered; extracted wording alone does not preserve their visual meaning",
                    )?;
                    if shape.picture_fill.is_some() {
                        located_gap(
                            self.extraction,
                            index,
                            &locator,
                            "Shape picture fill retained in original, not projected as an ordinary embedded picture",
                        )?;
                    }
                }
                ShapeNode::Group(group) => {
                    self.shapes(&group.children, &inventory.children, index, hidden)?
                }
                ShapeNode::Picture(picture) => self.picture(picture, inventory, index, &locator)?,
                ShapeNode::GraphicFrame(frame) => match &frame.data {
                    GraphicFrameData::Table(table) => {
                        for (row, cells) in table.rows.iter().enumerate() {
                            for (column, cell) in cells.cells.iter().enumerate() {
                                let location =
                                    format!("{locator}/table/row={}/cell={}", row + 1, column + 1);
                                let cell_source = node(
                                    self.extraction,
                                    index,
                                    format!(
                                        "Table row {}, column {}{}",
                                        row + 1,
                                        column + 1,
                                        if hidden { " (hidden)" } else { "" }
                                    ),
                                    SHAPE.into(),
                                    location.clone(),
                                    "partial",
                                    &[],
                                )?;
                                wording(
                                    self.extraction,
                                    cell_source,
                                    "Authored table-cell text",
                                    &text(&cell.text),
                                )?;
                                if cell.merged || cell.grid_span != 1 || cell.row_span != 1 {
                                    located_gap(
                                        self.extraction,
                                        cell_source,
                                        &location,
                                        format!(
                                            "Table span/merge layout not rendered (column span {}, row span {}, continuation {}); wording belongs to this source cell",
                                            cell.grid_span, cell.row_span, cell.merged
                                        ),
                                    )?;
                                }
                            }
                        }
                    }
                    GraphicFrameData::Chart {
                        relationship_id,
                        part_path,
                    } => located_gap(
                        self.extraction,
                        index,
                        &locator,
                        format!(
                            "Native chart relationship={relationship_id}, part={}: visual and factual chart content unavailable in this profile; inspect original",
                            part_path.as_deref().unwrap_or("unresolved")
                        ),
                    )?,
                    GraphicFrameData::Diagram {
                        relationship_ids,
                        drawing_part_path,
                    } => located_gap(
                        self.extraction,
                        index,
                        &locator,
                        format!(
                            "SmartArt relationships={relationship_ids:?}, drawing={}: visual and factual diagram content unavailable in this profile; inspect original",
                            drawing_part_path.as_deref().unwrap_or("unresolved")
                        ),
                    )?,
                    GraphicFrameData::Unknown { uri, .. } => located_gap(
                        self.extraction,
                        index,
                        &locator,
                        format!(
                            "Unsupported drawing graphic URI={uri:?}; retained in original, factual content unavailable"
                        ),
                    )?,
                },
            }
        }
        Ok(())
    }
    fn picture(
        &mut self,
        picture: &pptx_parse::Picture,
        inventory: &SourceShape,
        source: usize,
        locator: &str,
    ) -> Result<(), String> {
        let asset = self.picture_asset(picture, inventory);
        match asset {
            Ok(asset) => {
                let alt = [inventory.title.as_deref(), inventory.description.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join("\n");
                let metadata = format!(
                    "\n{} · {locator}\n{}{}",
                    self.extraction.sources[source].id,
                    fence("Source location", &self.extraction.sources[source].name),
                    fence("Authored picture metadata (title / description)", &alt)
                );
                let link = format!("\n![image]({})\n", asset_file_name(&asset)?);
                append(self.extraction, &metadata)?;
                let start = append(self.extraction, &link)?;
                self.extraction.sources[source].text = format!("{metadata}{link}");
                let asset_id = add_asset(self.extraction, asset)?;
                occurrence(
                    self.extraction,
                    source,
                    asset_id,
                    format!(
                        "{locator}/relationship={}/part={}",
                        picture.relationship_id.as_deref().unwrap_or("unresolved"),
                        picture.media_part_path.as_deref().unwrap_or("unresolved")
                    ),
                    if alt.is_empty() { None } else { Some(alt) },
                    start,
                    start + link.len(),
                )?;
            }
            Err(reason) if reason.starts_with("PPTX ambiguous") => return Err(reason),
            Err(reason) => located_gap(self.extraction, source, locator, reason)?,
        }
        Ok(())
    }
    fn picture_asset(
        &self,
        picture: &pptx_parse::Picture,
        inventory: &SourceShape,
    ) -> Result<ImageAsset, String> {
        let rid = picture
            .relationship_id
            .as_deref()
            .ok_or("Missing embedded picture relationship")?;
        if !inventory.relationship_ids.iter().any(|id| id == rid) {
            return Err("PPTX ambiguous picture/source relationship correspondence".into());
        }
        let entries = self
            .package
            .relationships
            .get(self.part)
            .ok_or("Missing slide relationships")?;
        let mut matches = entries.iter().filter(|entry| entry.id == rid);
        let relationship = matches
            .next()
            .ok_or("Missing picture relationship target")?;
        if matches.next().is_some() {
            return Err("PPTX ambiguous picture relationship ID".into());
        }
        if relationship.target_mode == pptx_parse::TargetMode::External {
            return Err("External picture unavailable; no network fetch".into());
        }
        if !matches!(
            relationship.relationship_type.as_str(),
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                | "http://purl.oclc.org/ooxml/officeDocument/relationships/image"
        ) {
            return Err("Picture relationship is not an image; retained in original".into());
        }
        let target = relationship
            .resolved_target
            .as_deref()
            .ok_or("Unresolved picture part")?;
        if picture.media_part_path.as_deref() != Some(target) {
            return Err("PPTX ambiguous resolved picture part correspondence".into());
        }
        let bytes = self
            .package
            .part_bytes(target)
            .ok_or("Missing embedded picture part")?;
        image_asset(bytes, &self.extraction.limits)
    }
}
