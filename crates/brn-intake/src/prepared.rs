//! Transient, source-bound document structure for the dedicated conversion lane.
use crate::{ImageAsset, IntakeLimits, IntakeUsage};
use serde::{Deserialize, Serialize};

pub const PROFILE: &str = "brn-prepared-v2/luna-high-v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedDocument {
    pub schema: u32,
    pub profile: String,
    pub source_sha256: [u8; 32],
    pub format: String,
    pub limits: IntakeLimits,
    pub consumed: IntakeUsage,
    pub blocks: Vec<PreparedBlock>,
    pub assets: Vec<ImageAsset>,
    pub pages: Vec<PreparedPage>,
    pub gaps: Vec<PreparedGap>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparedBlock {
    pub id: String,
    pub locator: String,
    pub content: BlockContent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlockContent {
    Paragraph {
        text: String,
    },
    Heading {
        text: String,
        level: u8,
    },
    ListItem {
        text: String,
        level: u8,
        label: String,
    },
    Metadata {
        name: String,
        value: String,
    },
    Footnote {
        label: String,
        text: String,
    },
    Table {
        rows: Vec<Vec<PreparedCell>>,
    },
    Figure {
        asset_id: String,
        alt: String,
    },
    PageText {
        page: u32,
        text: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparedCell {
    /// Stable source coordinate, independent of table formatting and visible text.
    pub address: String,
    pub column: u32,
    pub text: String,
    pub formula: Option<String>,
    pub result_status: Option<String>,
    pub comment: Option<String>,
    pub links: Vec<String>,
    pub row_span: u32,
    pub column_span: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedPage {
    pub number: u32,
    pub width: f64,
    pub height: f64,
    pub preview_asset_id: String,
    pub needs_transcription: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparedGap {
    pub locator: String,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConversionCandidate {
    pub schema: u32,
    pub source_sha256: [u8; 32],
    pub markdown: String,
    pub page_reviews: Vec<PageReview>,
    pub crops: Vec<FigureCrop>,
    pub table_reviews: Vec<TableReview>,
}

/// Model-owned decisions; the host keeps structured source formatting and bindings.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConversionPlan {
    pub schema: u32,
    pub source_sha256: [u8; 32],
    pub pages: Vec<PageConversion>,
    pub tables: Vec<TableReview>,
    pub crops: Vec<FigureCrop>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PageConversion {
    pub page: u32,
    pub markdown: String,
    pub preview_sha256: [u8; 32],
    pub uncertainty: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TableReview {
    pub block_id: String,
    pub source_sha256: [u8; 32],
    pub uncertainty: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PageReview {
    pub page: u32,
    pub preview_sha256: [u8; 32],
    /// Human-readable uncertainty retained in coverage, never a completeness claim.
    pub uncertainty: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FigureCrop {
    pub id: String,
    pub page: u32,
    /// Source-page coordinates [left, top, right, bottom].
    pub bounds: [f64; 4],
    pub alt: String,
}

mod formatting;
pub use formatting::{apply_plan, check_candidate, format, readable_markdown, table_hash};

impl PreparedDocument {
    /// Validate transient evidence before it becomes a provider request or retained output.
    pub fn validate(&self) -> Result<(), String> {
        use crate::{MAX_ASSETS, MAX_IMAGE_BYTES, MAX_OCCURRENCES, MAX_TEXT_BYTES};
        use std::collections::BTreeSet;
        if self.schema != 2 || self.profile != PROFILE {
            return Err("unsupported prepared profile".into());
        }
        self.limits.validate()?;
        if !matches!(
            self.format.as_str(),
            "docx" | "pptx" | "eml" | "html" | "xlsx" | "pdf"
        ) || self.blocks.is_empty()
            || self.blocks.len() > MAX_OCCURRENCES
            || self.assets.len() > MAX_ASSETS
            || self.pages.len() > MAX_ASSETS
            || self.gaps.len() > MAX_OCCURRENCES
        {
            return Err("prepared collection budget/format".into());
        }
        if self.consumed.input_bytes == 0
            || self.consumed.input_bytes > self.limits.max_input_bytes
            || self.consumed.expanded_bytes > self.limits.max_expanded_bytes
            || self.consumed.package_parts > self.limits.max_package_parts
            || self.consumed.mime_parts > self.limits.max_mime_parts
            || self.consumed.mime_depth > self.limits.max_mime_depth
            || self.consumed.decoded_bytes > self.limits.max_decoded_bytes
            || self.consumed.image_pixels > self.limits.max_total_image_pixels
            || self.consumed.output_bytes > self.limits.max_output_bytes
        {
            return Err("prepared usage budget".into());
        }
        let mut assets = BTreeSet::new();
        let mut hashes = BTreeSet::new();
        let mut bytes = 0usize;
        let mut pixels = 0u64;
        for asset in &self.assets {
            crate::id(&asset.id)?;
            if !assets.insert(asset.id.as_str())
                || !hashes.insert(asset.sha256)
                || asset.media_type != "image/png"
                || crate::digest(&asset.bytes) != asset.sha256
            {
                return Err("prepared asset identity/type".into());
            }
            let (w, h) = crate::decode_png(
                &asset.bytes,
                MAX_IMAGE_BYTES,
                32768,
                self.limits.max_image_pixels,
            )?;
            if (w, h) != (asset.width, asset.height) {
                return Err("prepared asset dimensions".into());
            }
            bytes = bytes
                .checked_add(asset.bytes.len())
                .ok_or("asset byte overflow")?;
            pixels = pixels
                .checked_add(u64::from(w) * u64::from(h))
                .ok_or("pixel overflow")?;
            if bytes > self.limits.max_decoded_bytes.min(MAX_IMAGE_BYTES)
                || pixels > self.limits.max_total_image_pixels
            {
                return Err("aggregate prepared asset budget".into());
            }
        }
        let mut ids = BTreeSet::new();
        for block in &self.blocks {
            crate::id(&block.id)?;
            crate::bounded(&block.locator, 8192, "block locator")?;
            if !ids.insert(block.id.as_str()) {
                return Err("duplicate prepared block".into());
            }
            match &block.content {
                BlockContent::Heading { level, .. } if !(1..=6).contains(level) => {
                    return Err("heading level".into());
                }
                BlockContent::ListItem { level, .. } if *level > 31 => {
                    return Err("list depth".into());
                }
                BlockContent::Figure { asset_id, .. } if !assets.contains(asset_id.as_str()) => {
                    return Err("missing figure asset".into());
                }
                BlockContent::Table { rows } => {
                    if rows.is_empty() || rows.len() > 2048 {
                        return Err("table row budget".into());
                    }
                    let mut coordinates = BTreeSet::new();
                    for row in rows {
                        if row.len() > 256 {
                            return Err("table width budget".into());
                        }
                        let mut columns = BTreeSet::new();
                        for cell in row {
                            crate::id(&cell.address)?;
                            if cell.column >= 16384
                                || !coordinates.insert(cell.address.as_str())
                                || !columns.insert(cell.column)
                                || cell.row_span == 0
                                || cell.column_span == 0
                                || cell.row_span > 2048
                                || cell.column_span > 16384
                            {
                                return Err("table coordinate/span integrity".into());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        let mut pages = BTreeSet::new();
        for page in &self.pages {
            if page.number == 0
                || !pages.insert(page.number)
                || !page.width.is_finite()
                || !page.height.is_finite()
                || page.width <= 0.
                || page.height <= 0.
                || !assets.contains(page.preview_asset_id.as_str())
            {
                return Err("prepared page geometry/preview".into());
            }
        }
        for block in &self.blocks {
            if let BlockContent::PageText { page, .. } = block.content
                && !pages.contains(&page)
            {
                return Err("page text without checked preview".into());
            }
        }
        let encoded = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        // Includes all metadata, not just visible prose. Image bytes have their own aggregate quota.
        let text_size = serde_json::to_vec(&(&self.blocks, &self.gaps))
            .map_err(|e| e.to_string())?
            .len();
        if encoded.len() > self.limits.max_output_bytes
            || text_size > MAX_TEXT_BYTES
            || encoded.contains(&0)
        {
            return Err("prepared serialized/text budget".into());
        }
        Ok(())
    }
}

/// Complete CRC/decompression/dimension validation before a preview is sent to a provider.
pub fn validate_preview(bytes: &[u8], limits: &IntakeLimits) -> Result<(u32, u32), String> {
    limits.validate()?;
    crate::decode_png(
        bytes,
        crate::MAX_IMAGE_BYTES,
        32768,
        limits.max_image_pixels,
    )
}

#[cfg(test)]
mod tests;
