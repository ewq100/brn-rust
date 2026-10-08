//! Versioned, bounded evidence exchanged with the isolated maintained intake helper.
//! Anchors and node IDs are extraction-local; neither hashes nor filenames are source identity.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_REQUEST_BYTES: usize = MAX_INPUT_BYTES * 4 + 1024;
pub const MAX_SOURCES: usize = 2048;
pub const MAX_ASSETS: usize = 32;
pub const MAX_OCCURRENCES: usize = 2048;
pub const MAX_TEXT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_RETAINED_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_IMAGE_PIXELS: u64 = 32 * 1024 * 1024;
pub const MAX_TOTAL_IMAGE_PIXELS: u64 = 64 * 1024 * 1024;
pub const CONVERTER: &str = "brn-intake-v1/betteroffice-0.3.0/mail-parser-0.11.8";

/// Per-intake quotas may tighten the qualified hard profile, never enlarge it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct IntakeLimits {
    pub max_input_bytes: usize,
    pub max_expanded_bytes: usize,
    pub max_package_parts: usize,
    pub max_mime_parts: usize,
    pub max_mime_depth: usize,
    pub max_decoded_bytes: usize,
    pub max_image_pixels: u64,
    pub max_total_image_pixels: u64,
    pub max_output_bytes: usize,
    pub wall_time_ms: u64,
}
impl Default for IntakeLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: MAX_INPUT_BYTES,
            max_expanded_bytes: 16 * 1024 * 1024,
            max_package_parts: 512,
            max_mime_parts: 512,
            max_mime_depth: 32,
            max_decoded_bytes: 16 * 1024 * 1024,
            max_image_pixels: MAX_IMAGE_PIXELS,
            max_total_image_pixels: MAX_TOTAL_IMAGE_PIXELS,
            max_output_bytes: MAX_OUTPUT_BYTES,
            wall_time_ms: 5000,
        }
    }
}
impl IntakeLimits {
    pub fn validate(&self) -> Result<(), String> {
        let hard = Self {
            wall_time_ms: 30_000,
            ..Self::default()
        };
        for (value, max) in [
            (self.max_input_bytes, hard.max_input_bytes),
            (self.max_expanded_bytes, hard.max_expanded_bytes),
            (self.max_package_parts, hard.max_package_parts),
            (self.max_mime_parts, hard.max_mime_parts),
            (self.max_mime_depth, hard.max_mime_depth),
            (self.max_decoded_bytes, hard.max_decoded_bytes),
            (self.max_output_bytes, hard.max_output_bytes),
        ] {
            if value == 0 || value > max {
                return Err("intake quota exceeds qualified hard bounds or is zero".into());
            }
        }
        if self.max_image_pixels == 0
            || self.max_image_pixels > hard.max_image_pixels
            || self.max_total_image_pixels == 0
            || self.max_total_image_pixels > hard.max_total_image_pixels
            || self.wall_time_ms == 0
            || self.wall_time_ms > hard.wall_time_ms
        {
            return Err("intake image/time quota exceeds qualified hard bounds or is zero".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IntakeUsage {
    pub input_bytes: usize,
    pub expanded_bytes: usize,
    pub package_parts: usize,
    pub mime_parts: usize,
    pub mime_depth: usize,
    pub decoded_bytes: usize,
    pub image_pixels: u64,
    pub output_bytes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HelperRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<IntakeLimits>,
    pub kind: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Extraction {
    #[serde(default)]
    pub limits: IntakeLimits,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumed: Option<IntakeUsage>,
    pub schema: u32,
    pub converter: String,
    pub original_sha256: [u8; 32],
    pub markdown: String,
    pub sources: Vec<SourceNode>,
    pub assets: Vec<ImageAsset>,
    pub occurrences: Vec<ImageOccurrence>,
    pub gaps: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceNode {
    pub id: String,
    pub parent: Option<String>,
    pub name: String,
    pub media_type: String,
    pub locator: String,
    pub status: String,
    #[serde(with = "binary")]
    pub bytes: Vec<u8>,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImageAsset {
    pub id: String,
    pub sha256: [u8; 32],
    pub width: u32,
    pub height: u32,
    pub media_type: String,
    #[serde(with = "binary")]
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImageOccurrence {
    pub id: String,
    pub source_id: String,
    pub asset_id: String,
    pub locator: String,
    pub alt: Option<String>,
    /// UTF-8 byte range in `Extraction.markdown`, including the checked asset link.
    pub start: usize,
    pub end: usize,
}

pub fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub fn hex(hash: &[u8; 32]) -> String {
    hash.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn asset_file_name(asset: &ImageAsset) -> Result<String, String> {
    let ext = match asset.media_type.as_str() {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => return Err("unsupported asset media type".into()),
    };
    Ok(format!("{}.{}", hex(&asset.sha256), ext))
}

/// A Source owns its created asset files even when another Source has identical bytes.
pub fn asset_file_name_for_source(asset: &ImageAsset, namespace: &str) -> Result<String, String> {
    Ok(format!(
        "{}-{}",
        source_namespace(namespace)?,
        asset_file_name(asset)?
    ))
}

fn source_namespace(value: &str) -> Result<String, String> {
    let hyphenated = value.len() == 36;
    if value.len() != 32 && !hyphenated {
        return Err("Source asset namespace must be a nonnil UUID".into());
    }
    let mut normalized = String::with_capacity(32);
    for (i, b) in value.bytes().enumerate() {
        if hyphenated && matches!(i, 8 | 13 | 18 | 23) {
            if b != b'-' {
                return Err("Source asset namespace must be a nonnil UUID".into());
            }
        } else if b.is_ascii_hexdigit() {
            normalized.push(char::from(b.to_ascii_lowercase()));
        } else {
            return Err("Source asset namespace must be a nonnil UUID".into());
        }
    }
    if normalized.bytes().all(|b| b == b'0') {
        return Err("Source asset namespace must be a nonnil UUID".into());
    }
    Ok(normalized)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceTextEdit {
    start: usize,
    end: usize,
    replacement: String,
}

fn apply_source_edits(text: &str, edits: &[SourceTextEdit]) -> String {
    let mut rendered = String::new();
    let mut cursor = 0;
    for edit in edits {
        rendered.push_str(&text[cursor..edit.start]);
        rendered.push_str(&edit.replacement);
        cursor = edit.end;
    }
    rendered.push_str(&text[cursor..]);
    rendered
}

fn materialized_offset(offset: usize, edits: &[SourceTextEdit]) -> usize {
    let mut mapped = offset;
    for edit in edits.iter().take_while(|edit| edit.end <= offset) {
        mapped = mapped - (edit.end - edit.start) + edit.replacement.len();
    }
    mapped
}

fn bounded(value: &str, max: usize, what: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > max || value.contains('\0') {
        Err(format!("invalid {what}"))
    } else {
        Ok(())
    }
}
fn id(value: &str) -> Result<(), String> {
    bounded(value, 256, "id")?;
    if value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"-_: .".contains(&b))
        && !value.contains(' ')
    {
        Ok(())
    } else {
        Err("unsafe id".into())
    }
}
impl Extraction {
    fn source_asset_edits(&self, namespace: &str) -> Result<Vec<SourceTextEdit>, String> {
        self.validate()?;
        source_namespace(namespace)?;
        let assets: BTreeMap<_, _> = self.assets.iter().map(|a| (a.id.as_str(), a)).collect();
        let mut edits = Vec::with_capacity(self.occurrences.len());
        for occurrence in &self.occurrences {
            let asset = assets[occurrence.asset_id.as_str()];
            let base = asset_file_name(asset)?;
            let fragment = &self.markdown[occurrence.start..occurrence.end];
            let bare = format!("]({base})");
            let legacy = format!("](assets/{base})");
            let matches: Vec<_> = fragment
                .match_indices(&bare)
                .map(|(start, value)| (start, value.len()))
                .chain(
                    fragment
                        .match_indices(&legacy)
                        .map(|(start, value)| (start, value.len())),
                )
                .collect();
            if matches.len() != 1 {
                return Err("image occurrence has no unique qualified asset destination".into());
            }
            let (start, length) = matches[0];
            // Only the destination changes; alt text and surrounding source text remain exact.
            edits.push(SourceTextEdit {
                start: occurrence.start + start + 2,
                end: occurrence.start + start + length - 1,
                replacement: asset_file_name_for_source(asset, namespace)?,
            });
        }
        edits.sort_unstable_by_key(|edit| edit.start);
        Ok(edits)
    }

    fn source_local_edits(
        &self,
        source: &SourceNode,
        edits: &[SourceTextEdit],
    ) -> Result<Vec<SourceTextEdit>, String> {
        if source.text.is_empty() || edits.is_empty() {
            return Ok(Vec::new());
        }
        let mut mapped: Option<(String, Vec<SourceTextEdit>)> = None;
        for (count, (start, _)) in self.markdown.match_indices(&source.text).enumerate() {
            if count >= MAX_OCCURRENCES {
                return Err("source text correspondence budget".into());
            }
            let end = start + source.text.len();
            let mut local = Vec::new();
            let first = edits.partition_point(|edit| edit.end <= start);
            for edit in edits[first..].iter().take_while(|edit| edit.start < end) {
                if edit.start < start || edit.end > end {
                    return Err("source text cuts through an image destination".into());
                }
                local.push(SourceTextEdit {
                    start: edit.start - start,
                    end: edit.end - start,
                    replacement: edit.replacement.clone(),
                });
            }
            let candidate = (apply_source_edits(&source.text, &local), local);
            if mapped
                .as_ref()
                .is_some_and(|previous| previous != &candidate)
            {
                return Err("ambiguous source text correspondence during materialization".into());
            }
            mapped = Some(candidate);
        }
        mapped
            .map(|(_, edits)| edits)
            .ok_or_else(|| "source text has no exact correspondence".into())
    }

    /// Derive note-specific asset destinations without changing the measured immutable snapshot.
    /// Call this on the original extraction with the new Source note's UUID.
    pub fn materialize_for_source(&self, namespace: &str) -> Result<Self, String> {
        let edits = self.source_asset_edits(namespace)?;
        let mut materialized = self.clone();
        materialized.markdown = apply_source_edits(&self.markdown, &edits);
        for source in &mut materialized.sources {
            let local = self.source_local_edits(source, &edits)?;
            source.text = apply_source_edits(&source.text, &local);
        }
        for occurrence in &mut materialized.occurrences {
            occurrence.start = materialized_offset(occurrence.start, &edits);
            occurrence.end = materialized_offset(occurrence.end, &edits);
        }
        // Usage describes the helper's original output, not this derived representation.
        materialized.consumed = None;
        materialized.validate()?;
        Ok(materialized)
    }

    /// Map a quote in a materialized source node back to the immutable node's UTF-8 range.
    /// Changed image destinations cannot be cited as unchanged source evidence.
    pub fn original_source_range(
        &self,
        namespace: &str,
        source_id: &str,
        start: usize,
        end: usize,
    ) -> Result<(usize, usize), String> {
        let edits = self.source_asset_edits(namespace)?;
        let source = self
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .ok_or("unknown source node")?;
        let local = self.source_local_edits(source, &edits)?;
        let rendered = apply_source_edits(&source.text, &local);
        let quote = rendered
            .get(start..end)
            .filter(|quote| !quote.is_empty())
            .ok_or("invalid materialized source quote range")?;
        let mut original_start = start;
        let mut original_end = end;
        let mut delta = 0isize;
        for edit in &local {
            let changed_start = edit
                .start
                .checked_add_signed(delta)
                .ok_or("source range overflow")?;
            let changed_end = changed_start + edit.replacement.len();
            if start < changed_end && end > changed_start {
                return Err("source quote crosses a changed image destination".into());
            }
            if changed_end <= start {
                original_start = original_start + (edit.end - edit.start) - edit.replacement.len();
            }
            if changed_end <= end {
                original_end = original_end + (edit.end - edit.start) - edit.replacement.len();
            }
            delta += edit.replacement.len() as isize - (edit.end - edit.start) as isize;
        }
        if source.text.get(original_start..original_end) != Some(quote) {
            return Err("source quote differs from immutable evidence".into());
        }
        Ok((original_start, original_end))
    }

    /// Validate untrusted helper output before writing any blob or using any evidence.
    /// This validates envelope integrity; full image decoding belongs to the isolated helper.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 {
            return Err("unsupported extraction schema".into());
        }
        self.limits.validate()?;
        bounded(&self.converter, 1024, "converter")?;
        if self.markdown.len() > MAX_TEXT_BYTES
            || self.sources.is_empty()
            || self.sources.len() > MAX_SOURCES
            || self.assets.len() > MAX_ASSETS
            || self.occurrences.len() > MAX_OCCURRENCES
            || self.gaps.len() > 2048
        {
            return Err("extraction collection/text budget".into());
        }
        let mut source_ids = BTreeSet::new();
        let mut source_locators = BTreeSet::new();
        let mut retained = 0usize;
        let mut text = 0usize;
        for (i, source) in self.sources.iter().enumerate() {
            id(&source.id)?;
            bounded(&source.name, 4096, "source name")?;
            bounded(&source.media_type, 256, "source media type")?;
            bounded(&source.locator, 8192, "source locator")?;
            if !matches!(
                source.status.as_str(),
                "complete" | "partial" | "unprocessed" | "retained-inline" | "container"
            ) {
                return Err("invalid source status".into());
            }
            if i == 0 {
                if source.parent.is_some()
                    || source.bytes.len() > self.limits.max_input_bytes
                    || digest(&source.bytes) != self.original_sha256
                {
                    return Err("root original mismatch".into());
                }
            } else if !source
                .parent
                .as_ref()
                .is_some_and(|parent| source_ids.contains(parent))
            {
                return Err("missing, cyclic or unordered source parent".into());
            }
            if !source_ids.insert(source.id.clone())
                || !source_locators.insert((source.parent.clone(), source.locator.clone()))
            {
                return Err("duplicate source identity/locator".into());
            }
            retained = retained
                .checked_add(source.bytes.len())
                .ok_or("retained size overflow")?;
            text = text
                .checked_add(source.text.len())
                .ok_or("text size overflow")?;
            if retained > MAX_RETAINED_BYTES || text > MAX_TEXT_BYTES {
                return Err("source payload budget".into());
            }
            if !source.text.is_empty() && !self.markdown.contains(&source.text) {
                return Err("source extracted text has no exact global correspondence".into());
            }
            if source.status == "unprocessed" && !source.text.is_empty() {
                return Err("unprocessed source has factual text".into());
            }
        }
        let mut assets = BTreeMap::new();
        let mut image_bytes = 0usize;
        let mut pixels = 0u64;
        for asset in &self.assets {
            if asset.bytes.is_empty()
                || digest(&asset.bytes) != asset.sha256
                || asset.id != format!("asset-{}", hex(&asset.sha256))
            {
                return Err("asset identity/hash mismatch".into());
            }
            asset_file_name(asset)?;
            let area = u64::from(asset.width) * u64::from(asset.height);
            pixels += area;
            image_bytes = image_bytes
                .checked_add(asset.bytes.len())
                .ok_or("image size overflow")?;
            if area == 0
                || area > self.limits.max_image_pixels
                || pixels > self.limits.max_total_image_pixels
                || image_bytes > MAX_IMAGE_BYTES
            {
                return Err("image budget".into());
            }
            if assets.insert(asset.id.as_str(), asset).is_some() {
                return Err("duplicate asset".into());
            }
        }
        let mut occurrence_ids = BTreeSet::new();
        let mut occurrence_locators = BTreeSet::new();
        let mut used_assets = BTreeSet::new();
        let mut ranges = Vec::new();
        for occurrence in &self.occurrences {
            id(&occurrence.id)?;
            bounded(&occurrence.locator, 8192, "occurrence locator")?;
            if occurrence
                .alt
                .as_ref()
                .is_some_and(|s| s.len() > 8192 || s.contains('\0'))
            {
                return Err("invalid image alt".into());
            }
            let asset = assets
                .get(occurrence.asset_id.as_str())
                .ok_or("missing occurrence asset")?;
            if !source_ids.contains(&occurrence.source_id)
                || !occurrence_ids.insert(&occurrence.id)
                || !occurrence_locators.insert((&occurrence.source_id, &occurrence.locator))
            {
                return Err("invalid occurrence association".into());
            }
            let fragment = self
                .markdown
                .get(occurrence.start..occurrence.end)
                .filter(|s| !s.is_empty())
                .ok_or("invalid UTF-8 occurrence range")?;
            let file_name = asset_file_name(asset)?;
            if !fragment.contains(&file_name) {
                return Err("occurrence range does not bind its asset".into());
            }
            used_assets.insert(occurrence.asset_id.as_str());
            ranges.push((occurrence.start, occurrence.end));
        }
        ranges.sort_unstable();
        if ranges.windows(2).any(|r| r[0].1 > r[1].0) || used_assets.len() != assets.len() {
            return Err("overlapping occurrences or unused asset".into());
        }
        for gap in &self.gaps {
            bounded(gap, 16384, "gap")?;
        }
        let mut counter = SizeCounter(0);
        serde_json::to_writer(&mut counter, self).map_err(|e| e.to_string())?;
        if counter.0 > self.limits.max_output_bytes {
            return Err("configured output byte quota".into());
        }
        if let Some(usage) = &self.consumed {
            let decoded = self
                .sources
                .iter()
                .skip(1)
                .map(|s| s.bytes.len())
                .sum::<usize>();
            if usage.input_bytes != self.sources[0].bytes.len()
                || usage.decoded_bytes != decoded
                || decoded > self.limits.max_decoded_bytes
                || usage.expanded_bytes > self.limits.max_expanded_bytes
                || usage.package_parts > self.limits.max_package_parts
                || usage.mime_parts > self.limits.max_mime_parts
                || usage.mime_depth > self.limits.max_mime_depth
                || usage.image_pixels != pixels
                || pixels > self.limits.max_total_image_pixels
                || self.assets.iter().any(|a| {
                    u64::from(a.width) * u64::from(a.height) > self.limits.max_image_pixels
                })
                || usage.output_bytes != counter.0
            {
                return Err("consumed intake scope differs from payload or quota".into());
            }
        }
        Ok(())
    }
}
struct SizeCounter(usize);
impl std::io::Write for SizeCounter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("output size overflow"))?;
        if self.0 > MAX_OUTPUT_BYTES {
            return Err(std::io::Error::other("output protocol budget"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
mod binary {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(value))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        if encoded.len() > super::MAX_RETAINED_BYTES * 4 / 3 + 4 {
            return Err(serde::de::Error::custom("base64 payload budget"));
        }
        STANDARD.decode(encoded).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "helper")]
pub mod helper;

/// Bounded complete PNG validation for retained historical image records.
/// This is format-independent decoding, with the historical record budget.
pub fn validate_png_image(bytes: &[u8]) -> Result<(u32, u32), String> {
    decode_png(bytes, 1024 * 1024, 4096, 4_194_304)
}

fn decode_png(
    bytes: &[u8],
    max_encoded: usize,
    max_dimension: u32,
    max_pixels: u64,
) -> Result<(u32, u32), String> {
    if bytes.is_empty() || bytes.len() > max_encoded {
        return Err("PNG encoded byte budget".into());
    }
    let mut cursor = std::io::Cursor::new(bytes);
    let mut options = png::DecodeOptions::default();
    options.set_ignore_checksums(false);
    options.set_skip_ancillary_crc_failures(false);
    let mut decoder = png::Decoder::new_with_options(&mut cursor, options);
    decoder.set_limits(png::Limits {
        bytes: (max_pixels * 8) as usize,
    });
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("PNG header: {e}"))?;
    let (width, height) = (reader.info().width, reader.info().height);
    if width == 0
        || height == 0
        || width > max_dimension
        || height > max_dimension
        || u64::from(width) * u64::from(height) > max_pixels
    {
        return Err("PNG dimension/pixel budget".into());
    }
    if reader.info().animation_control.is_some() {
        return Err("animated PNG unprocessed; complete animation decoding not qualified".into());
    }
    let size = reader
        .output_buffer_size()
        .ok_or("PNG output buffer overflow")?;
    if size > (max_pixels * 8) as usize {
        return Err("PNG decoded byte budget".into());
    }
    let mut output = vec![0; size];
    reader
        .next_frame(&mut output)
        .map_err(|e| format!("PNG complete decode: {e}"))?;
    reader
        .finish()
        .map_err(|e| format!("PNG terminal chunks: {e}"))?;
    drop(reader);
    if cursor.position() != bytes.len() as u64 {
        return Err("PNG trailing data".into());
    }
    Ok((width, height))
}
