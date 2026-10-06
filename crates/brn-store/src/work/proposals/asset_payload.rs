//! Canonical padded base64 only for newly typed opaque asset payload fields.
use super::MAX_ASSET_BYTES;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{
    Deserializer, Serializer,
    de::{Error, Visitor},
    ser::Error as _,
};
use std::fmt;

const MAX_ENCODED_BYTES: usize = MAX_ASSET_BYTES.div_ceil(3) * 4;

pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    if bytes.len() > MAX_ASSET_BYTES {
        return Err(S::Error::custom("asset payload exceeds 16 MiB"));
    }
    serializer.serialize_str(&STANDARD.encode(bytes))
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    struct Payload;
    impl Visitor<'_> for Payload {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("canonical padded base64 asset payload at most 16 MiB")
        }
        fn visit_str<E: Error>(self, text: &str) -> Result<Self::Value, E> {
            if text.len() > MAX_ENCODED_BYTES || !text.len().is_multiple_of(4) {
                return Err(E::custom(
                    "asset encoded payload exceeds bounds or lacks padding",
                ));
            }
            let padding = text
                .as_bytes()
                .iter()
                .rev()
                .take_while(|byte| **byte == b'=')
                .count();
            if padding > 2 {
                return Err(E::custom("invalid asset base64 padding"));
            }
            let len = (text.len() / 4 * 3)
                .checked_sub(padding)
                .ok_or_else(|| E::custom("invalid asset base64 length"))?;
            if len > MAX_ASSET_BYTES {
                return Err(E::custom("asset decoded payload exceeds 16 MiB"));
            }
            let mut bytes = vec![0; len];
            let written = STANDARD.decode_slice(text, &mut bytes).map_err(E::custom)?;
            if written != len || STANDARD.encode(&bytes) != text {
                return Err(E::custom("asset payload is not canonical base64"));
            }
            Ok(bytes)
        }
    }
    deserializer.deserialize_str(Payload)
}
