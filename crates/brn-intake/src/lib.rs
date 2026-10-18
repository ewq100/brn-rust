//! Versioned, bounded evidence exchanged with the isolated maintained intake helper.
//! Anchors and node IDs are extraction-local; neither hashes nor filenames are source identity.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod document;

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
