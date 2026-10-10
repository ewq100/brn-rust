//! Bounded transient conversion. Returned knowledge has no original-file payload.
//! Semantic selection for useful intake and protected-note creation belong to the host.
mod process;
mod transient;

use brn_intake::{Extraction, HelperRequest, IntakeLimits};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImportIntent {
    UsefulInformation,
    FullNote,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentKind {
    Docx,
    Pdf,
    Markdown,
    Text,
    Eml,
}
#[derive(Clone, Debug)]
pub struct PdfTools {
    pub pdftotext: PathBuf,
    pub pdfimages: PathBuf,
}
#[derive(Clone, Debug)]
pub struct Converter {
    pub helper_path: PathBuf,
    pub pdf_tools: Option<PdfTools>,
    /// Explicit private application scratch directory, outside the knowledge store.
    pub temp_root: PathBuf,
    pub limits: IntakeLimits,
}
pub struct ConversionRequest<'a> {
    pub kind: DocumentKind,
    pub intent: ImportIntent,
    pub bytes: &'a [u8],
    pub source_reference: &'a str,
    pub inventory: Option<&'a SourceInventory>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetainedAsset {
    pub name: String,
    pub media_type: String,
    pub sha256: [u8; 32],
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceCoverage {
    pub locator: String,
    pub text: String,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoverageStatus {
    Complete,
    Partial,
    Unchecked,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GapKind {
    Limitation,
    Omission,
    Unverified,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageGap {
    pub kind: GapKind,
    pub locator: String,
    pub detail: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Coverage {
    /// Complete is relative to the supplied independent inventory, never every PDF/DOCX.
    pub status: CoverageStatus,
    pub checked_items: Vec<String>,
    pub gaps: Vec<CoverageGap>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConversionResult {
    pub intent: ImportIntent,
    pub markdown: String,
    pub assets: Vec<RetainedAsset>,
    pub source_sha256: [u8; 32],
    pub source_reference: String,
    pub converter: String,
    pub sources: Vec<SourceCoverage>,
    pub coverage: Coverage,
}
/// Hand-checked source objects, supplied by the host; not synthesized from converter output.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceInventory {
    pub items: Vec<InventoryItem>,
    pub expected_assets: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryItem {
    pub label: String,
    pub locator: String,
    pub needle: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversionError {
    Cancelled,
    Timeout,
    Budget,
    Unavailable,
    Invalid,
    Protocol,
    Io,
}
impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ConversionError {}

impl Converter {
    pub fn convert(
        &self,
        request: ConversionRequest<'_>,
        cancel: &AtomicBool,
    ) -> Result<ConversionResult, ConversionError> {
        self.limits
            .validate()
            .map_err(|_| ConversionError::Budget)?;
        if cancel.load(Ordering::Acquire) {
            return Err(ConversionError::Cancelled);
        }
        if request.bytes.is_empty() || request.bytes.len() > self.limits.max_input_bytes {
            return Err(ConversionError::Budget);
        }
        if request.source_reference.is_empty()
            || request.source_reference.len() > 8192
            || request.source_reference.contains('\0')
        {
            return Err(ConversionError::Invalid);
        }
        let deadline = Instant::now() + Duration::from_millis(self.limits.wall_time_ms);
        let temp = transient::TransientDir::create(&self.temp_root)?;
        let converted = (|| {
            let mut result = match request.kind {
                DocumentKind::Docx | DocumentKind::Eml => {
                    self.document(&request, cancel, deadline, &temp)?
                }
                DocumentKind::Markdown | DocumentKind::Text => self.plain(&request)?,
                DocumentKind::Pdf => self.pdf(&request, cancel, deadline, &temp)?,
            };
            result.check_inventory(request.inventory)?;
            if serde_json::to_vec(&result)
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
        })();
        // Explicit cleanup is checked even on failure/cancellation; Drop covers unwind.
        temp.cleanup()?;
        converted
    }
    /// Reap only this adapter's abandoned run directories, after a full day.
    pub fn cleanup_expired(&self) -> Result<usize, ConversionError> {
        transient::cleanup_expired(&self.temp_root)
    }

    fn document(
        &self,
        request: &ConversionRequest<'_>,
        cancel: &AtomicBool,
        deadline: Instant,
        temp: &transient::TransientDir,
    ) -> Result<ConversionResult, ConversionError> {
        let input = serde_json::to_vec(&HelperRequest {
            kind: if request.kind == DocumentKind::Eml {
                "eml"
            } else {
                "docx"
            }
            .into(),
            bytes: request.bytes.to_vec(),
            limits: Some(self.limits.clone()),
        })
        .map_err(|_| ConversionError::Protocol)?;
        let output = process::run(
            &self.helper_path,
            &[],
            Some(input),
            temp,
            process::ProcessBudget {
                output_bytes: self.limits.max_output_bytes,
                storage_bytes: self.limits.max_output_bytes + self.limits.max_input_bytes,
                deadline,
            },
            cancel,
            false,
        )?;
        let extraction: Extraction =
            serde_json::from_slice(&output).map_err(|_| ConversionError::Protocol)?;
        extraction
            .validate()
            .map_err(|_| ConversionError::Protocol)?;
        if extraction.original_sha256 != brn_intake::digest(request.bytes)
            || extraction.converter != brn_intake::CONVERTER
            || extraction.limits != self.limits
            || extraction.consumed.is_none()
        {
            return Err(ConversionError::Protocol);
        }
        let gaps = extraction
            .gaps
            .iter()
            .map(|detail| CoverageGap {
                // This known general disclaimer does not imply a substantive omission.
                kind: if detail.contains("Reflowed accepted-revision DOCX extraction;") {
                    GapKind::Limitation
                } else {
                    GapKind::Omission
                },
                locator: "document".into(),
                detail: detail
                    .replace("inspect retained original", "recheck the external source")
                    .replace(
                        "retained only in original",
                        "available only in the external source",
                    ),
            })
            .collect();
        Ok(ConversionResult {
            intent: request.intent,
            markdown: extraction.markdown,
            assets: extraction
                .assets
                .into_iter()
                .map(|asset| {
                    Ok(RetainedAsset {
                        name: brn_intake::asset_file_name(&asset)
                            .map_err(|_| ConversionError::Protocol)?,
                        media_type: asset.media_type,
                        sha256: asset.sha256,
                        width: asset.width,
                        height: asset.height,
                        bytes: asset.bytes,
                    })
                })
                .collect::<Result<_, ConversionError>>()?,
            source_sha256: extraction.original_sha256,
            source_reference: request.source_reference.into(),
            converter: extraction.converter,
            sources: extraction
                .sources
                .into_iter()
                .map(|source| SourceCoverage {
                    locator: source.locator,
                    text: source.text,
                })
                .collect(),
            coverage: Coverage {
                status: CoverageStatus::Unchecked,
                checked_items: vec![],
                gaps,
            },
        })
    }
    fn plain(&self, request: &ConversionRequest<'_>) -> Result<ConversionResult, ConversionError> {
        let text = std::str::from_utf8(request.bytes).map_err(|_| ConversionError::Invalid)?;
        if text.contains('\0') || text.len() > brn_intake::MAX_TEXT_BYTES {
            return Err(ConversionError::Budget);
        }
        let mut gaps = Vec::new();
        if request.kind == DocumentKind::Markdown && text.contains("![") {
            gaps.push(CoverageGap{kind:GapKind::Omission,locator:"figures".into(),detail:"Referenced Markdown images remain external; provide a document with embedded figures for a self-contained full note.".into()});
        }
        Ok(ConversionResult {
            intent: request.intent,
            markdown: text.into(),
            assets: vec![],
            source_sha256: brn_intake::digest(request.bytes),
            source_reference: request.source_reference.into(),
            converter: "UTF-8 verbatim".into(),
            sources: vec![SourceCoverage {
                locator: request.source_reference.into(),
                text: text.into(),
            }],
            coverage: Coverage {
                status: CoverageStatus::Unchecked,
                checked_items: vec![],
                gaps,
            },
        })
    }
    fn pdf(
        &self,
        request: &ConversionRequest<'_>,
        cancel: &AtomicBool,
        deadline: Instant,
        temp: &transient::TransientDir,
    ) -> Result<ConversionResult, ConversionError> {
        let tools = self
            .pdf_tools
            .as_ref()
            .ok_or(ConversionError::Unavailable)?;
        let input = temp.path.join("input.pdf");
        fs::write(&input, request.bytes).map_err(|_| ConversionError::Io)?;
        let text = process::run(
            &tools.pdftotext,
            &[
                "-layout".into(),
                "-enc".into(),
                "UTF-8".into(),
                input.as_os_str().to_owned(),
                "-".into(),
            ],
            None,
            temp,
            process::ProcessBudget {
                output_bytes: brn_intake::MAX_TEXT_BYTES.min(self.limits.max_output_bytes),
                storage_bytes: self.limits.max_output_bytes + self.limits.max_input_bytes,
                deadline,
            },
            cancel,
            true,
        )?;
        let text = String::from_utf8(text).map_err(|_| ConversionError::Invalid)?;
        let list = process::run(
            &tools.pdfimages,
            &["-list".into(), input.as_os_str().to_owned()],
            None,
            temp,
            process::ProcessBudget {
                output_bytes: 256 * 1024,
                storage_bytes: self.limits.max_output_bytes + self.limits.max_input_bytes,
                deadline,
            },
            cancel,
            true,
        )?;
        let list = String::from_utf8(list).map_err(|_| ConversionError::Protocol)?;
        let mut image_pages = BTreeMap::new();
        let mut pixels = 0u64;
        for line in list.lines() {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields
                .first()
                .and_then(|s| s.parse::<usize>().ok())
                .is_none()
            {
                continue;
            }
            if fields.len() < 5 {
                return Err(ConversionError::Protocol);
            }
            let page: usize = fields[0].parse().map_err(|_| ConversionError::Protocol)?;
            let number: usize = fields[1].parse().map_err(|_| ConversionError::Protocol)?;
            let width: u64 = fields[3].parse().map_err(|_| ConversionError::Protocol)?;
            let height: u64 = fields[4].parse().map_err(|_| ConversionError::Protocol)?;
            let area = width.checked_mul(height).ok_or(ConversionError::Budget)?;
            pixels = pixels.checked_add(area).ok_or(ConversionError::Budget)?;
            if area == 0
                || area > self.limits.max_image_pixels
                || pixels > self.limits.max_total_image_pixels
                || number >= brn_intake::MAX_ASSETS
            {
                return Err(ConversionError::Budget);
            }
            if fields[2] != "image" {
                return Err(ConversionError::Invalid);
            } // masks require a qualified composition path
            if image_pages.insert(number, page).is_some() {
                return Err(ConversionError::Protocol);
            }
        }
        let prefix = temp.path.join("figure");
        process::run(
            &tools.pdfimages,
            &[
                "-png".into(),
                input.as_os_str().to_owned(),
                prefix.as_os_str().to_owned(),
            ],
            None,
            temp,
            process::ProcessBudget {
                output_bytes: 256 * 1024,
                storage_bytes: self.limits.max_output_bytes + self.limits.max_input_bytes,
                deadline,
            },
            cancel,
            true,
        )?;
        temp.check_budget(self.limits.max_output_bytes + self.limits.max_input_bytes)?;
        let mut assets = Vec::new();
        let mut page_assets: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        let mut total_image_bytes = 0usize;
        for (number, page) in image_pages {
            let path = temp.path.join(format!("figure-{number:03}.png"));
            let metadata = fs::symlink_metadata(&path).map_err(|_| ConversionError::Protocol)?;
            if !metadata.is_file() || metadata.len() as usize > brn_intake::MAX_IMAGE_BYTES {
                return Err(ConversionError::Budget);
            }
            let bytes = fs::read(path).map_err(|_| ConversionError::Io)?;
            total_image_bytes += bytes.len();
            if total_image_bytes > brn_intake::MAX_IMAGE_BYTES {
                return Err(ConversionError::Budget);
            }
            let info = png::Decoder::new(std::io::Cursor::new(&bytes))
                .read_info()
                .map_err(|_| ConversionError::Invalid)?;
            let (width, height) = (info.info().width, info.info().height);
            let area = u64::from(width) * u64::from(height);
            if area == 0 || area > self.limits.max_image_pixels {
                return Err(ConversionError::Budget);
            }
            let sha256 = brn_intake::digest(&bytes);
            let name = format!("{}.png", brn_intake::hex(&sha256));
            page_assets.entry(page).or_default().push(name.clone());
            if !assets.iter().any(|a: &RetainedAsset| a.name == name) {
                assets.push(RetainedAsset {
                    name,
                    media_type: "image/png".into(),
                    sha256,
                    width,
                    height,
                    bytes,
                });
            }
        }
        let mut markdown = String::new();
        let mut sources = Vec::new();
        for (index, page) in text.split('\u{c}').enumerate() {
            if page.trim().is_empty() {
                continue;
            }
            let number = index + 1;
            // Longer fences avoid treating source backticks as note markup.
            let fence = "`".repeat(
                page.split(|c| c != '`')
                    .map(str::len)
                    .max()
                    .unwrap_or(0)
                    .max(2)
                    + 1,
            );
            markdown.push_str(&format!(
                "## Page {number}\n\n{fence}text\n{}\n{fence}\n\n",
                page.trim_end()
            ));
            sources.push(SourceCoverage {
                locator: format!("page:{number}"),
                text: page.trim_end().into(),
            });
            if let Some(names) = page_assets.remove(&number) {
                for name in names {
                    markdown.push_str(&format!("![Extracted figure, page {number}]({name})\n\n"));
                }
            }
        }
        let mut gaps = vec![CoverageGap { kind: GapKind::Limitation, locator: "document".into(), detail: "Poppler text-layer layout with page-linked raster figures. Original typography, vector artwork, scans/OCR, complex reading order and semantic table reconstruction are not qualified; inspect the external source or supply an independent inventory.".into() }];
        if sources.is_empty() {
            gaps.push(CoverageGap { kind: GapKind::Omission, locator: "document".into(), detail: "No readable text layer; scanned/unsupported PDF requires another conversion path.".into() });
        }
        if !page_assets.is_empty() {
            gaps.push(CoverageGap {
                kind: GapKind::Omission,
                locator: "figures".into(),
                detail: "Figures on pages without readable text could not be located in the note."
                    .into(),
            });
        }
        if markdown.len() > brn_intake::MAX_TEXT_BYTES
            || serde_json::to_vec(&assets)
                .map_err(|_| ConversionError::Protocol)?
                .len()
                + markdown.len()
                > self.limits.max_output_bytes
        {
            return Err(ConversionError::Budget);
        }
        Ok(ConversionResult {
            intent: request.intent,
            markdown,
            assets,
            source_sha256: brn_intake::digest(request.bytes),
            source_reference: request.source_reference.into(),
            converter: "Poppler/pdftotext-layout+pdfimages-png".into(),
            sources,
            coverage: Coverage {
                status: CoverageStatus::Unchecked,
                checked_items: vec![],
                gaps,
            },
        })
    }
}
impl ConversionResult {
    /// Recheck a retained result, e.g. after asset preparation. Missing figures become Partial.
    pub fn check_inventory(
        &mut self,
        inventory: Option<&SourceInventory>,
    ) -> Result<(), ConversionError> {
        self.coverage.checked_items.clear();
        self.coverage.gaps.retain(|gap| {
            gap.kind == GapKind::Limitation
                || !gap.detail.starts_with("Inventory:") && gap.kind != GapKind::Unverified
        });
        if let Some(inventory) = inventory {
            if inventory.items.is_empty()
                || inventory.items.len() > 2048
                || inventory.expected_assets > brn_intake::MAX_ASSETS
            {
                return Err(ConversionError::Invalid);
            }
            let normalized = inventory_text(&self.markdown);
            for item in &inventory.items {
                if item.needle.is_empty()
                    || item.needle.len() > 8192
                    || item.label.is_empty()
                    || item.label.len() > 1024
                    || item.locator.len() > 8192
                {
                    return Err(ConversionError::Invalid);
                }
                let needle = inventory_text(&item.needle);
                if needle.is_empty() {
                    return Err(ConversionError::Invalid);
                }
                if normalized.contains(&needle) {
                    self.coverage.checked_items.push(item.label.clone());
                } else {
                    self.coverage.gaps.push(CoverageGap {
                        kind: GapKind::Omission,
                        locator: item.locator.clone(),
                        detail: format!("Inventory: missing {}", item.label),
                    });
                }
            }
            if self.assets.len() != inventory.expected_assets
                || self
                    .assets
                    .iter()
                    .any(|asset| !self.markdown.contains(&format!("]({})", asset.name)))
            {
                self.coverage.gaps.push(CoverageGap {
                    kind: GapKind::Omission,
                    locator: "figures".into(),
                    detail: "Inventory: necessary figure count or note destination differs".into(),
                });
            }
        } else {
            self.coverage.gaps.push(CoverageGap { kind: GapKind::Unverified, locator: "document".into(), detail: "Source coverage has not been checked against an independent inventory; full import completeness is unverified.".into() });
        }
        self.coverage.status = if self
            .coverage
            .gaps
            .iter()
            .any(|gap| gap.kind == GapKind::Omission)
        {
            CoverageStatus::Partial
        } else if inventory.is_none() {
            if self.intent == ImportIntent::FullNote {
                CoverageStatus::Partial
            } else {
                CoverageStatus::Unchecked
            }
        } else {
            CoverageStatus::Complete
        };
        Ok(())
    }
}

fn inventory_text(text: &str) -> String {
    let mut visible = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        visible.push_str(&rest[..start]);
        if let Some(end) = rest[start..].find("-->") {
            rest = &rest[start + end + 3..];
        } else {
            rest = &rest[start..];
            break;
        }
    }
    visible.push_str(rest);
    visible
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '\\' | '*' | '|' | '`'))
        .collect()
}
