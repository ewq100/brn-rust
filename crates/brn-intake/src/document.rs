//! Transient document protocol. Engine identity is a build input, never source authority.
use crate::{
    ImageAsset, IntakeLimits, MAX_ASSETS, MAX_IMAGE_BYTES, MAX_OCCURRENCES, MAX_OUTPUT_BYTES,
    MAX_TEXT_BYTES, digest, hex,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const ARCHIVE_SHA256: &str = "f9859ca06d9ee171c65c91cfbb0a25287a45622e7628f1b45ba5353b473b6f0d";
pub const ACCEPTED_PATCHES: [(&str, &str); 4] = [
    (
        "0001-docx-table-grid-spans.patch",
        "92178fcfb5887328dec2f11b1c63d28687b628f4764e4cc41cae30a81598a6d7",
    ),
    (
        "0003-docx-namespaces.patch",
        "e64020a6df795f4cd1b32ad5eba124197e644fec08c88033cc904313442b851f",
    ),
    (
        "0004-pptx-order-hidden.patch",
        "aaf5c6f67c22ff492fae7b80aec446ce083b620837864d6eb80e782d64983a1d",
    ),
    (
        "0005-docx-comment-references.patch",
        "b574b0374514860dccf0d6b503bd8408ce16a259f2ce68f3dd6cef8aa18a69b1",
    ),
];

pub fn expected_engine_identity(backend: &str) -> Result<String, String> {
    match backend {
        "xberg" => Ok(format!("xberg/1.3.10/stock/{ARCHIVE_SHA256}")),
        "xberg_brn" => {
            let mut input = format!("{ARCHIVE_SHA256}\nutf8-identity-v1\n");
            for (name, hash) in ACCEPTED_PATCHES {
                input.push_str(&format!("{name} {hash}\n"));
            }
            Ok(format!(
                "xberg/1.3.10/brn/{}",
                hex(&digest(input.as_bytes()))
            ))
        }
        _ => Err("unknown document backend".into()),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DocumentRequest {
    pub schema: u32,
    pub backend: String,
    pub kind: String,
    pub limits: IntakeLimits,
    pub bytes: Vec<u8>,
}
impl DocumentRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 {
            return Err("unsupported document request schema".into());
        }
        expected_engine_identity(&self.backend)?;
        self.mime_type()?;
        self.limits.validate()?;
        if self.bytes.is_empty() || self.bytes.len() > self.limits.max_input_bytes {
            return Err("document input budget".into());
        }
        // Existing identity projection admission, not a parser or a fallback.
        if self.backend == "xberg_brn" && matches!(self.kind.as_str(), "text" | "markdown") {
            let text = std::str::from_utf8(&self.bytes).map_err(|_| "text is not UTF-8")?;
            if text.contains('\0') || text.len() > MAX_TEXT_BYTES {
                return Err("text identity admission".into());
            }
        }
        Ok(())
    }
    pub fn mime_type(&self) -> Result<&'static str, String> {
        match self.kind.as_str() {
            "pdf" => Ok("application/pdf"),
            "docx" => Ok("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
            "pptx" => {
                Ok("application/vnd.openxmlformats-officedocument.presentationml.presentation")
            }
            "xlsx" => Ok("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
            "eml" => Ok("message/rfc822"),
            "html" => Ok("text/html"),
            "text" => Ok("text/plain"),
            "markdown" => Ok("text/markdown"),
            _ => Err("unsupported document kind".into()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DocumentExtraction {
    pub schema: u32,
    pub backend: String,
    pub engine_identity: String,
    pub source_sha256: [u8; 32],
    pub markdown: String,
    pub assets: Vec<ImageAsset>,
    pub gaps: Vec<String>,
}
impl DocumentExtraction {
    pub fn validate_for(&self, request: &DocumentRequest) -> Result<(), String> {
        request.validate()?;
        if self.schema != 1
            || self.backend != request.backend
            || self.engine_identity != expected_engine_identity(&request.backend)?
            || self.source_sha256 != digest(&request.bytes)
        {
            return Err("document response identity mismatch".into());
        }
        if self.markdown.is_empty()
            || self.markdown.len() > MAX_TEXT_BYTES
            || self.markdown.contains('\0')
        {
            return Err("document text budget or empty result".into());
        }
        if self.assets.len() > MAX_ASSETS || self.gaps.len() > MAX_OCCURRENCES {
            return Err("document asset/gap count budget".into());
        }
        let mut ids = BTreeSet::new();
        let mut total_bytes = 0usize;
        let mut total_pixels = 0u64;
        for asset in &self.assets {
            total_bytes = total_bytes
                .checked_add(asset.bytes.len())
                .ok_or("asset size overflow")?;
            if total_bytes > MAX_IMAGE_BYTES
                || total_bytes > request.limits.max_decoded_bytes
                || asset.media_type != "image/png"
                || asset.sha256 != digest(&asset.bytes)
                || asset.id != format!("asset-{}", hex(&asset.sha256))
                || !ids.insert(&asset.id)
            {
                return Err("invalid document asset identity or budget".into());
            }
            let (w, h) = crate::decode_png(
                &asset.bytes,
                MAX_IMAGE_BYTES,
                u32::MAX,
                request.limits.max_image_pixels,
            )?;
            total_pixels = total_pixels
                .checked_add(u64::from(w) * u64::from(h))
                .ok_or("pixel overflow")?;
            if (w, h) != (asset.width, asset.height)
                || total_pixels > request.limits.max_total_image_pixels
            {
                return Err("document image dimensions/budget".into());
            }
        }
        let mut gap_bytes = 0usize;
        for gap in &self.gaps {
            if gap.is_empty() || gap.contains('\0') || gap.len() > 4096 {
                return Err("invalid document gap".into());
            }
            gap_bytes += gap.len();
        }
        if gap_bytes > MAX_TEXT_BYTES {
            return Err("document gap budget".into());
        }
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if bytes.len() > request.limits.max_output_bytes || bytes.len() > MAX_OUTPUT_BYTES {
            return Err("document output budget".into());
        }
        Ok(())
    }
}
