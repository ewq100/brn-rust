//! Local bounded preparation and deterministic candidate acceptance. No provider access.
use crate::*;
use brn_intake::prepared::{
    BlockContent, ConversionCandidate, PROFILE, PreparedBlock, PreparedDocument, PreparedGap,
    PreparedPage,
};
use quick_xml::{Reader, events::Event};
use std::{ffi::OsString, io::Cursor, path::Path};

impl Converter {
    pub fn prepare(
        &self,
        request: ConversionRequest<'_>,
        cancel: &AtomicBool,
    ) -> Result<PreparedDocument, ConversionError> {
        self.limits
            .validate()
            .map_err(|_| ConversionError::Budget)?;
        validate_request(&request, cancel, self.limits.max_input_bytes)?;
        let deadline = Instant::now() + Duration::from_millis(self.limits.wall_time_ms);
        let temp = transient::TransientDir::create(&self.temp_root)?;
        let result = (|| {
            let doc = if request.kind == DocumentKind::Pdf {
                self.prepare_pdf(&request, cancel, deadline, &temp)?
            } else {
                let kind = match request.kind {
                    DocumentKind::Docx => "docx",
                    DocumentKind::Eml => "eml",
                    DocumentKind::Html => "html",
                    DocumentKind::Xlsx => "xlsx",
                    DocumentKind::Pptx => "pptx",
                    _ => return Err(ConversionError::Invalid),
                };
                let input = serde_json::to_vec(&HelperRequest {
                    limits: Some(self.limits.clone()),
                    kind: kind.into(),
                    bytes: request.bytes.to_vec(),
                })
                .map_err(|_| ConversionError::Protocol)?;
                let output = process::run(
                    &self.helper_path,
                    &[OsString::from("--prepare")],
                    Some(input),
                    &temp,
                    process::ProcessBudget {
                        output_bytes: self.limits.max_output_bytes,
                        storage_bytes: self.limits.max_output_bytes + self.limits.max_input_bytes,
                        deadline,
                    },
                    cancel,
                    false,
                )?;
                let doc: PreparedDocument =
                    serde_json::from_slice(&output).map_err(|_| ConversionError::Protocol)?;
                if doc.format != kind {
                    return Err(ConversionError::Protocol);
                }
                doc
            };
            doc.validate().map_err(|_| ConversionError::Protocol)?;
            if doc.source_sha256 != brn_intake::digest(request.bytes)
                || doc.limits != self.limits
                || doc.consumed.input_bytes != request.bytes.len()
            {
                return Err(ConversionError::Protocol);
            }
            if cancel.load(Ordering::Acquire) {
                return Err(ConversionError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(ConversionError::Timeout);
            }
            Ok(doc)
        })();
        temp.cleanup()?;
        result
    }

    /// Accept only the candidate bound to this exact original and host preparation profile.
    /// Page previews are removed; linked crops/figures become canonical retained assets.
    pub fn finish_candidate(
        &self,
        request: ConversionRequest<'_>,
        doc: PreparedDocument,
        candidate: ConversionCandidate,
        cancel: &AtomicBool,
    ) -> Result<ConversionResult, ConversionError> {
        self.limits
            .validate()
            .map_err(|_| ConversionError::Budget)?;
        let deadline = Instant::now() + Duration::from_millis(self.limits.wall_time_ms);
        validate_request(&request, cancel, self.limits.max_input_bytes)?;
        if doc.source_sha256 != brn_intake::digest(request.bytes)
            || doc.limits != self.limits
            || doc.consumed.input_bytes != request.bytes.len()
        {
            return Err(ConversionError::Protocol);
        }
        let mut markdown = brn_intake::prepared::readable_markdown(&doc, &candidate)
            .map_err(|_| ConversionError::Protocol)?;
        let previews: std::collections::BTreeSet<_> = doc
            .pages
            .iter()
            .map(|p| p.preview_asset_id.as_str())
            .collect();
        let mut assets = Vec::new();
        for asset in &doc.assets {
            let name = brn_intake::asset_file_name(asset).map_err(|_| ConversionError::Protocol)?;
            if !previews.contains(asset.id.as_str())
                && candidate.markdown.contains(&format!("]({name})"))
            {
                assets.push(RetainedAsset {
                    name,
                    media_type: asset.media_type.clone(),
                    sha256: asset.sha256,
                    width: asset.width,
                    height: asset.height,
                    bytes: asset.bytes.clone(),
                });
            }
        }
        let mut total_pixels: u64 = assets
            .iter()
            .map(|a| u64::from(a.width) * u64::from(a.height))
            .sum();
        let mut total_bytes: usize = assets.iter().map(|a| a.bytes.len()).sum();
        for crop in &candidate.crops {
            if cancel.load(Ordering::Acquire) {
                return Err(ConversionError::Cancelled);
            }
            let page = doc
                .pages
                .iter()
                .find(|p| p.number == crop.page)
                .ok_or(ConversionError::Protocol)?;
            let preview = doc
                .assets
                .iter()
                .find(|a| a.id == page.preview_asset_id)
                .ok_or(ConversionError::Protocol)?;
            if Instant::now() >= deadline {
                return Err(ConversionError::Timeout);
            }
            let (_, _, width, height) = crop_geometry(preview, page, crop.bounds)?;
            let area = u64::from(width) * u64::from(height);
            if assets.len() >= brn_intake::MAX_ASSETS
                || area > self.limits.max_image_pixels
                || total_pixels
                    .checked_add(area)
                    .is_none_or(|p| p > self.limits.max_total_image_pixels)
            {
                return Err(ConversionError::Budget);
            }
            let remaining = self
                .limits
                .max_decoded_bytes
                .min(brn_intake::MAX_IMAGE_BYTES)
                .checked_sub(total_bytes)
                .ok_or(ConversionError::Budget)?;
            let asset = crop_png(preview, page, crop.bounds, remaining)?;
            if !assets.iter().any(|a| a.sha256 == asset.sha256) {
                total_pixels += area;
                total_bytes += asset.bytes.len();
            }
            markdown = markdown.replace(
                &format!("](crop:{})", crop.id),
                &format!("]({})", asset.name),
            );
            if !assets
                .iter()
                .any(|a: &RetainedAsset| a.sha256 == asset.sha256)
            {
                assets.push(asset);
            }
        }
        // Evidence comments bind the checked candidate only. Located coverage remains
        // separate from readable note prose; storage rewrites image destinations.
        let mut gaps: Vec<_> = doc
            .gaps
            .iter()
            .map(|g| CoverageGap {
                kind: GapKind::Limitation,
                locator: g.locator.clone(),
                detail: g.detail.clone(),
            })
            .collect();
        for review in &candidate.table_reviews {
            if !review.uncertainty.trim().is_empty() {
                let locator = doc
                    .blocks
                    .iter()
                    .find(|b| b.id == review.block_id)
                    .map(|b| b.locator.clone())
                    .ok_or(ConversionError::Protocol)?;
                gaps.push(CoverageGap {
                    kind: GapKind::Omission,
                    locator,
                    detail: review.uncertainty.clone(),
                });
            }
        }
        for review in &candidate.page_reviews {
            if !review.uncertainty.trim().is_empty() {
                gaps.push(CoverageGap {
                    kind: GapKind::Omission,
                    locator: format!("page/{}", review.page),
                    detail: review.uncertainty.clone(),
                });
            }
        }
        for page in &doc.pages {
            if page.needs_transcription {
                gaps.push(CoverageGap{kind:GapKind::Limitation,locator:format!("page/{}",page.number),detail:"Scanned-page transcription was visually reviewed by Luna; accuracy is not independently verified.".into()});
            }
        }
        let mut sources: Vec<_> = doc
            .blocks
            .iter()
            .map(|b| SourceCoverage {
                locator: b.locator.clone(),
                text: block_text(&b.content),
            })
            .collect();
        for crop in &candidate.crops {
            sources.push(SourceCoverage {
                locator: format!("page/{}/crop/{}", crop.page, crop.id),
                text: format!("{}; source page bounds {:?}", crop.alt, crop.bounds),
            });
        }
        let mut result = ConversionResult {
            intent: request.intent,
            markdown,
            assets,
            source_sha256: doc.source_sha256,
            source_reference: request.source_reference.into(),
            converter: PROFILE.into(),
            sources,
            coverage: Coverage {
                status: CoverageStatus::Unchecked,
                checked_items: Vec::new(),
                gaps,
            },
        };
        result.check_inventory(request.inventory)?;
        let pixels: u64 = result
            .assets
            .iter()
            .map(|a| u64::from(a.width) * u64::from(a.height))
            .sum();
        let bytes: usize = result.assets.iter().map(|a| a.bytes.len()).sum();
        if result.assets.len() > brn_intake::MAX_ASSETS
            || pixels > self.limits.max_total_image_pixels
            || bytes > brn_intake::MAX_IMAGE_BYTES
            || serde_json::to_vec(&result)
                .map_err(|_| ConversionError::Protocol)?
                .len()
                > self.limits.max_output_bytes
        {
            return Err(ConversionError::Budget);
        }
        if cancel.load(Ordering::Acquire) {
            return Err(ConversionError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(ConversionError::Timeout);
        }
        Ok(result)
    }

    fn prepare_pdf(
        &self,
        request: &ConversionRequest<'_>,
        cancel: &AtomicBool,
        deadline: Instant,
        temp: &transient::TransientDir,
    ) -> Result<PreparedDocument, ConversionError> {
        let tools = self
            .pdf_tools
            .as_ref()
            .ok_or(ConversionError::Unavailable)?;
        let renderer = pdf_renderer(&tools.pdftotext)?;
        let input = temp.path.join("input.pdf");
        fs::write(&input, request.bytes).map_err(|_| ConversionError::Io)?;
        let budget = || process::ProcessBudget {
            output_bytes: brn_intake::MAX_TEXT_BYTES.min(self.limits.max_output_bytes),
            storage_bytes: self.limits.max_output_bytes + self.limits.max_input_bytes,
            deadline,
        };
        let geometry = process::run(
            &tools.pdftotext,
            &[
                "-bbox-layout".into(),
                "-enc".into(),
                "UTF-8".into(),
                input.as_os_str().to_owned(),
                "-".into(),
            ],
            None,
            temp,
            budget(),
            cancel,
            true,
        )?;
        let pages = parse_pages(&geometry)?;
        if pages.is_empty() || pages.len() > brn_intake::MAX_ASSETS {
            return Err(ConversionError::Budget);
        }
        let mut doc=PreparedDocument{schema:2,profile:PROFILE.into(),source_sha256:brn_intake::digest(request.bytes),format:"pdf".into(),limits:self.limits.clone(),consumed:brn_intake::IntakeUsage{input_bytes:request.bytes.len(),..Default::default()},blocks:Vec::new(),assets:Vec::new(),pages:Vec::new(),gaps:vec![PreparedGap{locator:"document".into(),detail:"PDF layout and figures are visually reviewed by Luna against page previews. Complex reading order, table associations and scan accuracy need independent source verification.".into()}]};
        // One renderer process amortizes font/runtime startup across all bounded pages.
        let prefix = temp.path.join("preview");
        process::run_renderer(
            &renderer,
            &[
                "-f".into(),
                "1".into(),
                "-l".into(),
                pages.len().to_string().into(),
                "-scale-to".into(),
                "1600".into(),
                "-png".into(),
                input.as_os_str().to_owned(),
                prefix.as_os_str().to_owned(),
            ],
            temp,
            budget(),
            cancel,
        )?;
        temp.check_budget(self.limits.max_output_bytes + self.limits.max_input_bytes)?;
        let page_digits = pages.len().to_string().len();
        let mut total_bytes = 0usize;
        let mut total_pixels = 0u64;
        for (index, (width, height, text)) in pages.into_iter().enumerate() {
            if cancel.load(Ordering::Acquire) {
                return Err(ConversionError::Cancelled);
            }
            let number = index as u32 + 1;
            let path = temp
                .path
                .join(format!("preview-{number:0page_digits$}.png"));
            let meta = fs::symlink_metadata(&path).map_err(|_| ConversionError::Protocol)?;
            if !meta.is_file() || meta.len() as usize > brn_intake::MAX_IMAGE_BYTES {
                return Err(ConversionError::Budget);
            }
            let bytes = fs::read(path).map_err(|_| ConversionError::Io)?;
            let (w, h) = brn_intake::prepared::validate_preview(&bytes, &self.limits)
                .map_err(|_| ConversionError::Invalid)?;
            total_bytes += bytes.len();
            total_pixels += u64::from(w) * u64::from(h);
            if total_bytes > self.limits.max_decoded_bytes
                || total_pixels > self.limits.max_total_image_pixels
            {
                return Err(ConversionError::Budget);
            }
            let hash = brn_intake::digest(&bytes);
            let id = if let Some(asset) = doc.assets.iter().find(|a| a.sha256 == hash) {
                asset.id.clone()
            } else {
                let id = format!("preview-{number}");
                doc.assets.push(brn_intake::ImageAsset {
                    id: id.clone(),
                    sha256: hash,
                    width: w,
                    height: h,
                    media_type: "image/png".into(),
                    bytes,
                });
                id
            };
            doc.pages.push(PreparedPage {
                number,
                width,
                height,
                preview_asset_id: id,
                needs_transcription: text.trim().is_empty(),
            });
            doc.blocks.push(PreparedBlock {
                id: format!("page-{number}"),
                locator: format!("page/{number}"),
                content: BlockContent::PageText { page: number, text },
            });
        }
        doc.consumed.decoded_bytes = total_bytes;
        doc.consumed.image_pixels = total_pixels;
        doc.consumed.output_bytes = serde_json::to_vec(&doc)
            .map_err(|_| ConversionError::Protocol)?
            .len();
        Ok(doc)
    }
}

fn validate_request(
    request: &ConversionRequest<'_>,
    cancel: &AtomicBool,
    max: usize,
) -> Result<(), ConversionError> {
    if cancel.load(Ordering::Acquire) {
        return Err(ConversionError::Cancelled);
    }
    if request.bytes.is_empty() || request.bytes.len() > max {
        return Err(ConversionError::Budget);
    }
    if request.source_reference.is_empty()
        || request.source_reference.len() > 8192
        || request.source_reference.contains('\0')
    {
        return Err(ConversionError::Invalid);
    }
    Ok(())
}
fn pdf_renderer(text: &Path) -> Result<PathBuf, ConversionError> {
    if let Some(path) = std::env::var_os("BRN_PDFTOPPM").map(PathBuf::from) {
        return if pdf_tools::executable(&path) {
            Ok(path)
        } else {
            Err(ConversionError::Unavailable)
        };
    }
    let mut candidates = vec![text.with_file_name("pdftoppm")];
    if let Ok(resolved) = text.canonicalize() {
        candidates.push(resolved.with_file_name("pdftoppm"));
    }
    if let Some(path) = std::env::var_os("PATH") {
        candidates.extend(std::env::split_paths(&path).map(|p| p.join("pdftoppm")));
    }
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin/pdftoppm"),
        PathBuf::from("/usr/local/bin/pdftoppm"),
    ]);
    candidates
        .into_iter()
        .find(|p| pdf_tools::executable(p))
        .ok_or(ConversionError::Unavailable)
}
fn block_text(content: &BlockContent) -> String {
    match content {
        BlockContent::Paragraph { text }
        | BlockContent::Heading { text, .. }
        | BlockContent::ListItem { text, .. }
        | BlockContent::Footnote { text, .. }
        | BlockContent::PageText { text, .. } => text.clone(),
        BlockContent::Metadata { name, value } => format!("{name}: {value}"),
        BlockContent::Figure { alt, .. } => alt.clone(),
        BlockContent::Table { rows } => rows
            .iter()
            .flatten()
            .map(|c| format!("{}: {}", c.address, c.text))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}
fn parse_pages(xml: &[u8]) -> Result<Vec<(f64, f64, String)>, ConversionError> {
    if xml.len() > brn_intake::MAX_TEXT_BYTES {
        return Err(ConversionError::Budget);
    }
    struct Line {
        position: Option<(f64, f64)>,
        text: String,
    }
    struct Page {
        width: f64,
        height: f64,
        text: String,
        lines: Vec<Line>,
        unbound_words: bool,
    }
    fn page_size(e: &quick_xml::events::BytesStart<'_>) -> Result<(f64, f64), ConversionError> {
        let (mut width, mut height) = (None, None);
        for attribute in e.attributes() {
            let attribute = attribute.map_err(|_| ConversionError::Protocol)?;
            let slot = match attribute.key.as_ref() {
                b"width" => &mut width,
                b"height" => &mut height,
                _ => continue,
            };
            let value = std::str::from_utf8(attribute.value.as_ref())
                .map_err(|_| ConversionError::Protocol)?
                .parse::<f64>()
                .map_err(|_| ConversionError::Protocol)?;
            if !value.is_finite() || value <= 0. || slot.replace(value).is_some() {
                return Err(ConversionError::Protocol);
            }
        }
        Ok((
            width.ok_or(ConversionError::Protocol)?,
            height.ok_or(ConversionError::Protocol)?,
        ))
    }
    fn position(
        e: &quick_xml::events::BytesStart<'_>,
        page: &Page,
    ) -> Result<Option<(f64, f64)>, ConversionError> {
        let mut coordinates = [None; 4];
        for attribute in e.attributes() {
            let attribute = attribute.map_err(|_| ConversionError::Protocol)?;
            let (index, bound) = match attribute.key.as_ref() {
                b"xMin" => (0, page.width),
                b"yMin" => (1, page.height),
                b"xMax" => (2, page.width),
                b"yMax" => (3, page.height),
                _ => continue,
            };
            let value = std::str::from_utf8(attribute.value.as_ref())
                .map_err(|_| ConversionError::Protocol)?
                .parse::<f64>()
                .map_err(|_| ConversionError::Protocol)?;
            if !value.is_finite()
                || value < 0.
                || value > bound
                || coordinates[index].replace(value).is_some()
            {
                return Err(ConversionError::Protocol);
            }
        }
        if let (Some(left), Some(right)) = (coordinates[0], coordinates[2])
            && left > right
        {
            return Err(ConversionError::Protocol);
        }
        if let (Some(top), Some(bottom)) = (coordinates[1], coordinates[3])
            && top > bottom
        {
            return Err(ConversionError::Protocol);
        }
        // Partial geometry is insufficient to reorder a line. Preserve the original
        // page order whenever any line lacks a complete source rectangle.
        Ok(match coordinates {
            [Some(left), Some(top), Some(_), Some(_)] => Some((left, top)),
            _ => None,
        })
    }
    let mut reader = Reader::from_reader(xml);
    let mut pages = Vec::new();
    let mut current: Option<Page> = None;
    let mut line: Option<Line> = None;
    let mut in_word = false;
    let mut events = 0;
    let mut depth = 0usize;
    loop {
        events += 1;
        if events > 200_000 || depth > 128 {
            return Err(ConversionError::Budget);
        }
        match reader.read_event().map_err(|_| ConversionError::Protocol)? {
            Event::Start(e) => {
                depth += 1;
                match e.local_name().as_ref() {
                    b"page" => {
                        if current.is_some() {
                            return Err(ConversionError::Protocol);
                        }
                        let (width, height) = page_size(&e)?;
                        current = Some(Page {
                            width,
                            height,
                            text: String::new(),
                            lines: Vec::new(),
                            unbound_words: false,
                        });
                    }
                    b"line" => {
                        if line.is_some() || in_word {
                            return Err(ConversionError::Protocol);
                        }
                        let page = current.as_ref().ok_or(ConversionError::Protocol)?;
                        line = Some(Line {
                            position: position(&e, page)?,
                            text: String::new(),
                        });
                    }
                    b"word" => {
                        if in_word {
                            return Err(ConversionError::Protocol);
                        }
                        let page = current.as_ref().ok_or(ConversionError::Protocol)?;
                        position(&e, page)?;
                        if line.is_none() {
                            current
                                .as_mut()
                                .ok_or(ConversionError::Protocol)?
                                .unbound_words = true;
                        }
                        in_word = true;
                    }
                    _ => {}
                }
            }
            Event::Text(t) if in_word => {
                let decoded = t
                    .xml_content(quick_xml::XmlVersion::Implicit1_0)
                    .map_err(|_| ConversionError::Protocol)?;
                let decoded =
                    quick_xml::escape::unescape(&decoded).map_err(|_| ConversionError::Protocol)?;
                current
                    .as_mut()
                    .ok_or(ConversionError::Protocol)?
                    .text
                    .push_str(&decoded);
                if let Some(line) = &mut line {
                    line.text.push_str(&decoded);
                }
            }
            Event::GeneralRef(r) if in_word => {
                let reference = r.decode().map_err(|_| ConversionError::Protocol)?;
                let decoded = quick_xml::escape::unescape(&format!("&{reference};"))
                    .map_err(|_| ConversionError::Protocol)?
                    .into_owned();
                current
                    .as_mut()
                    .ok_or(ConversionError::Protocol)?
                    .text
                    .push_str(&decoded);
                if let Some(line) = &mut line {
                    line.text.push_str(&decoded);
                }
            }
            Event::End(e) => {
                depth = depth.checked_sub(1).ok_or(ConversionError::Protocol)?;
                match e.local_name().as_ref() {
                    b"word" => {
                        if !in_word {
                            return Err(ConversionError::Protocol);
                        }
                        in_word = false;
                        current
                            .as_mut()
                            .ok_or(ConversionError::Protocol)?
                            .text
                            .push(' ');
                        if let Some(line) = &mut line {
                            line.text.push(' ');
                        }
                    }
                    b"line" => {
                        if in_word {
                            return Err(ConversionError::Protocol);
                        }
                        let completed = line.take().ok_or(ConversionError::Protocol)?;
                        let page = current.as_mut().ok_or(ConversionError::Protocol)?;
                        page.text.push('\n');
                        page.lines.push(completed);
                    }
                    b"page" => {
                        if in_word || line.is_some() {
                            return Err(ConversionError::Protocol);
                        }
                        let mut page = current.take().ok_or(ConversionError::Protocol)?;
                        if !page.unbound_words
                            && !page.lines.is_empty()
                            && page.lines.iter().all(|line| line.position.is_some())
                        {
                            page.lines.sort_by(|a, b| {
                                let (ax, ay) = a.position.expect("checked line geometry");
                                let (bx, by) = b.position.expect("checked line geometry");
                                ay.total_cmp(&by).then(ax.total_cmp(&bx))
                            });
                            page.text = page
                                .lines
                                .into_iter()
                                .map(|line| line.text + "\n")
                                .collect();
                        }
                        pages.push((page.width, page.height, page.text));
                    }
                    _ => {}
                }
            }
            Event::Empty(e) => match e.local_name().as_ref() {
                b"page" => {
                    if current.is_some() {
                        return Err(ConversionError::Protocol);
                    }
                    let (width, height) = page_size(&e)?;
                    pages.push((width, height, String::new()));
                }
                b"line" | b"word" => {
                    position(&e, current.as_ref().ok_or(ConversionError::Protocol)?)?;
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    if current.is_some() || line.is_some() || in_word || depth != 0 {
        return Err(ConversionError::Protocol);
    }
    Ok(pages)
}

fn crop_geometry(
    preview: &brn_intake::ImageAsset,
    page: &PreparedPage,
    bounds: [f64; 4],
) -> Result<(u32, u32, u32, u32), ConversionError> {
    let [l, t, r, b] = bounds;
    if bounds.iter().any(|v| !v.is_finite())
        || l < 0.
        || t < 0.
        || r <= l
        || b <= t
        || r > page.width
        || b > page.height
    {
        return Err(ConversionError::Invalid);
    }
    let left = (l / page.width * f64::from(preview.width)).floor() as u32;
    let top = (t / page.height * f64::from(preview.height)).floor() as u32;
    let right = (r / page.width * f64::from(preview.width))
        .ceil()
        .min(f64::from(preview.width)) as u32;
    let bottom = (b / page.height * f64::from(preview.height))
        .ceil()
        .min(f64::from(preview.height)) as u32;
    let width = right
        .checked_sub(left)
        .filter(|v| *v > 0)
        .ok_or(ConversionError::Invalid)?;
    let height = bottom
        .checked_sub(top)
        .filter(|v| *v > 0)
        .ok_or(ConversionError::Invalid)?;
    Ok((left, top, width, height))
}
struct BoundedPng {
    bytes: Vec<u8>,
    limit: usize,
}
impl std::io::Write for BoundedPng {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|size| size > self.limit)
        {
            return Err(std::io::Error::other("crop encoded byte budget"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn crop_png(
    preview: &brn_intake::ImageAsset,
    page: &PreparedPage,
    bounds: [f64; 4],
    max_bytes: usize,
) -> Result<RetainedAsset, ConversionError> {
    let (left, top, width, height) = crop_geometry(preview, page, bounds)?;
    let bottom = top + height;
    let mut decoder = png::Decoder::new(Cursor::new(&preview.bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    decoder.set_limits(png::Limits {
        bytes: (brn_intake::MAX_IMAGE_PIXELS * 8) as usize,
    });
    let mut reader = decoder.read_info().map_err(|_| ConversionError::Invalid)?;
    let mut decoded = vec![0; reader.output_buffer_size().ok_or(ConversionError::Budget)?];
    let info = reader
        .next_frame(&mut decoded)
        .map_err(|_| ConversionError::Invalid)?;
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return Err(ConversionError::Invalid),
    };
    let mut cropped = Vec::with_capacity(width as usize * height as usize * channels);
    for y in top..bottom {
        let start = (y as usize * info.width as usize + left as usize) * channels;
        let end = start + width as usize * channels;
        cropped.extend_from_slice(decoded.get(start..end).ok_or(ConversionError::Invalid)?);
    }
    let mut output = BoundedPng {
        bytes: Vec::new(),
        limit: max_bytes,
    };
    {
        let mut encoder = png::Encoder::new(&mut output, width, height);
        encoder.set_color(info.color_type);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|_| ConversionError::Invalid)?;
        writer
            .write_image_data(&cropped)
            .map_err(|_| ConversionError::Budget)?;
        writer.finish().map_err(|_| ConversionError::Budget)?;
    }
    let bytes = output.bytes;
    let sha256 = brn_intake::digest(&bytes);
    Ok(RetainedAsset {
        name: format!("{}.png", brn_intake::hex(&sha256)),
        media_type: "image/png".into(),
        sha256,
        width,
        height,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bbox_lines_use_source_top_to_bottom_then_left_to_right_order() {
        let pages=parse_pages(br#"<doc><page width="600" height="800"><flow><block><line xMin="10" yMin="700" xMax="200" yMax="715"><word>Footer</word></line><line xMin="250" yMin="100" xMax="300" yMax="120"><word>Gate</word><word>M-0042</word></line></block></flow><flow><block><line xMin="450" yMin="100" xMax="510" yMax="120"><word>Outlet</word></line><line xMin="50" yMin="100" xMax="100" yMax="120"><word>Inlet</word></line><line xMin="20" yMin="150" xMax="200" yMax="170"><word>Caption</word></line></block></flow></page></doc>"#).unwrap();
        assert_eq!(
            pages[0].2,
            "Inlet \nGate M-0042 \nOutlet \nCaption \nFooter \n"
        );
    }
    #[test]
    fn bbox_missing_or_partial_line_geometry_preserves_extraction_order() {
        for missing in ["", " xMin=\"50\" yMin=\"100\""] {
            let source = format!(
                "<doc><page width=\"600\" height=\"800\"><line xMin=\"10\" yMin=\"700\" xMax=\"200\" yMax=\"715\"><word>First</word></line><line{missing}><word>Second</word></line></page></doc>"
            );
            assert_eq!(
                parse_pages(source.as_bytes()).unwrap()[0].2,
                "First \nSecond \n"
            );
        }
        let source=br#"<page width="600" height="800"><word>Unbound</word><line xMin="10" yMin="100" xMax="200" yMax="120"><word>Bound</word></line></page>"#;
        assert_eq!(parse_pages(source).unwrap()[0].2, "Unbound Bound \n");
    }
    #[test]
    fn bbox_rejects_nonfinite_out_of_page_and_reversed_rectangles() {
        for geometry in [
            "xMin=\"NaN\" yMin=\"1\" xMax=\"2\" yMax=\"3\"",
            "xMin=\"1\" yMin=\"1\" xMax=\"inf\" yMax=\"3\"",
            "xMin=\"-1\"",
            "yMin=\"801\"",
            "xMin=\"2\" xMax=\"1\"",
            "yMin=\"3\" yMax=\"2\"",
        ] {
            let source = format!(
                "<page width=\"600\" height=\"800\"><line {geometry}><word>Text</word></line></page>"
            );
            assert_eq!(
                parse_pages(source.as_bytes()).unwrap_err(),
                ConversionError::Protocol
            );
        }
        assert!(parse_pages(br#"<page width="600" height="800"><line><word xMin="601">Text</word></line></page>"#).is_err());
        assert!(
            parse_pages(br#"<page width="600" height="800"><line yMin="NaN"/></page>"#).is_err()
        );
    }
    #[test]
    fn bbox_same_position_is_stable_and_empty_scan_page_is_preserved() {
        let pages=parse_pages(br#"<doc><page width="600" height="800"><line xMin="10" yMin="100" xMax="50" yMax="110"><word>A&amp;B</word></line><line xMin="10" yMin="100" xMax="50" yMax="110"><word>00123</word></line></page><page width="600" height="800"/></doc>"#).unwrap();
        assert_eq!(pages[0].2, "A&B \n00123 \n");
        assert_eq!(pages[1], (600., 800., String::new()));
    }
    #[test]
    fn bbox_pages_preserve_entities_scan_and_dimensions() {
        let pages=parse_pages(br#"<?xml version="1.0"?><html><body><doc><page width="600" height="800"><flow><block><line><word>A&amp;B</word><word>00123</word></line></block></flow></page><page width="400" height="500"></page></doc></body></html>"#).unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0], (600., 800., "A&B 00123 \n".into()));
        assert!(pages[1].2.is_empty());
        assert!(parse_pages(br#"<page width="NaN" height="800"></page>"#).is_err());
        assert!(parse_pages(br#"<page width="1" height="2">"#).is_err());
    }
    fn preview() -> brn_intake::ImageAsset {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 4, 4);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            let pixels: Vec<_> = (0..48).collect();
            writer.write_image_data(&pixels).unwrap();
        }
        brn_intake::ImageAsset {
            id: "preview".into(),
            sha256: brn_intake::digest(&bytes),
            width: 4,
            height: 4,
            media_type: "image/png".into(),
            bytes,
        }
    }
    #[test]
    fn source_coordinates_crop_actual_pixel_content() {
        let preview = preview();
        let page = PreparedPage {
            number: 1,
            width: 400.,
            height: 400.,
            preview_asset_id: "preview".into(),
            needs_transcription: false,
        };
        let crop = crop_png(
            &preview,
            &page,
            [100., 100., 300., 300.],
            brn_intake::MAX_IMAGE_BYTES,
        )
        .unwrap();
        assert_eq!((crop.width, crop.height), (2, 2));
        assert_eq!(brn_intake::digest(&crop.bytes), crop.sha256);
        let mut reader = png::Decoder::new(Cursor::new(crop.bytes))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut pixels).unwrap();
        assert_eq!(pixels, vec![15, 16, 17, 18, 19, 20, 27, 28, 29, 30, 31, 32]);
        assert!(crop_png(&preview, &page, [100., 100., 300., 300.], 8).is_err());
    }
    #[test]
    fn scan_preview_receipts_and_crop_binding_are_checked() {
        let asset = preview();
        let mut doc = PreparedDocument {
            schema: 2,
            profile: PROFILE.into(),
            source_sha256: brn_intake::digest(b"scan"),
            format: "pdf".into(),
            limits: IntakeLimits::default(),
            consumed: brn_intake::IntakeUsage {
                input_bytes: 4,
                ..Default::default()
            },
            blocks: vec![PreparedBlock {
                id: "page1".into(),
                locator: "page/1".into(),
                content: BlockContent::PageText {
                    page: 1,
                    text: String::new(),
                },
            }],
            assets: vec![asset.clone()],
            pages: vec![PreparedPage {
                number: 1,
                width: 400.,
                height: 400.,
                preview_asset_id: asset.id.clone(),
                needs_transcription: true,
            }],
            gaps: vec![],
        };
        let mut candidate = ConversionCandidate {
            schema: 2,
            source_sha256: doc.source_sha256,
            markdown: "Scanned text 00123\n\n![Chart](crop:figure) <!-- source:page1 -->".into(),
            page_reviews: vec![brn_intake::prepared::PageReview {
                page: 1,
                preview_sha256: asset.sha256,
                uncertainty: String::new(),
            }],
            table_reviews: vec![],
            crops: vec![brn_intake::prepared::FigureCrop {
                id: "figure".into(),
                page: 1,
                bounds: [10., 10., 390., 390.],
                alt: "Chart".into(),
            }],
        };
        brn_intake::prepared::check_candidate(&doc, &candidate).unwrap();
        candidate.page_reviews[0].preview_sha256 = [0; 32];
        assert!(brn_intake::prepared::check_candidate(&doc, &candidate).is_err());
        candidate.page_reviews[0].preview_sha256 = asset.sha256;
        candidate.crops[0].bounds[2] = 401.;
        assert!(brn_intake::prepared::check_candidate(&doc, &candidate).is_err());
        candidate.crops.clear();
        assert!(brn_intake::prepared::check_candidate(&doc, &candidate).is_err());
        // A literal no-observation value must not become a Setext underline while
        // deriving expected text. Zero and changed marker/value associations fail.
        doc.blocks[0].content = BlockContent::PageText {
            page: 1,
            text: "Marker Depth\nM-0042 0\nM-0110\n--".into(),
        };
        doc.pages[0].needs_transcription = false;
        candidate.markdown = "| Marker | Depth |\n| --- | --- |\n| M-0042 | 0 |\n| M-0110 | -- |\n\n<!-- source:page1 -->".into();
        brn_intake::prepared::check_candidate(&doc, &candidate).unwrap();
        assert!(
            !brn_intake::prepared::readable_markdown(&doc, &candidate)
                .unwrap()
                .contains("<!--")
        );
        candidate.markdown = candidate.markdown.replace("M-0110 | --", "M-0110 | 0");
        assert!(brn_intake::prepared::check_candidate(&doc, &candidate).is_err());
    }
}
