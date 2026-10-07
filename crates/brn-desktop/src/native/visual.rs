//! Render only complete checked workflow PNG payloads, never paths or URLs.
use super::*;
use brn_workflow::{inbox_processing::InboxSourceVisual, proposals::SourceVersion};
use gpui_kit::{AnyElement, Image, ImageFormat, ObjectFit, StyledImage, TestSupportExt, img};
use std::sync::Arc;

pub(super) fn png_image(bytes: &[u8]) -> Arc<Image> {
    Arc::new(Image::from_bytes(ImageFormat::Png, bytes.to_vec()))
}
pub(super) fn png_panel(id: &'static str, bytes: &[u8], proof: &InboxSourceVisual) -> AnyElement {
    let image = png_image(bytes);
    div()
        .id(id)
        .test_support()
        .aria_label(format!("Actual checked PNG image {}", image.id()))
        .flex()
        .flex_col()
        .gap_1()
        .child(
            img(image)
                .object_fit(ObjectFit::Contain)
                .w(px(320.))
                .h(px(220.)),
        )
        .child(format!(
            "PNG · {} × {} · {} bytes · SHA-256 {}",
            proof.width,
            proof.height,
            proof.byte_len,
            hex(&proof.sha256)
        ))
        .child(
            div()
                .id(format!("{id}-alt"))
                .test_support()
                .aria_label(format!(
                    "Original alt text: {}",
                    proof.alt_text.as_deref().unwrap_or("not supplied")
                ))
                .child(format!(
                    "Original alt text: {}",
                    proof.alt_text.as_deref().unwrap_or("not supplied")
                )),
        )
        .child(
            div()
                .id(format!("{id}-title"))
                .test_support()
                .aria_label(format!(
                    "Original title: {}",
                    proof.title.as_deref().unwrap_or("not supplied")
                ))
                .child(format!(
                    "Original title: {}",
                    proof.title.as_deref().unwrap_or("not supplied")
                )),
        )
        .child(format!(
            "Occurrence 1 · {} · relationship {} · converted bytes {}..{}",
            proof.part_name, proof.relationship_id, proof.image_start, proof.image_end
        ))
        .into_any_element()
}
pub(super) fn hex(hash: &[u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}
pub(super) fn file_proof(label: &str, source: &SourceVersion) -> String {
    let proof = &source.fingerprint;
    format!(
        "{label}: {} · device {} · inode {} · {} bytes · SHA-256 {}",
        source.path,
        proof.device,
        proof.inode,
        proof.len,
        hex(&proof.sha256)
    )
}

pub(super) fn intake_image_panel(
    index: usize,
    asset: &brn_intake::ImageAsset,
    occurrence: &brn_intake::ImageOccurrence,
) -> AnyElement {
    let format = match asset.media_type.as_str() {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        _ => {
            return div()
                .child("Image inspection unavailable for this media type.")
                .into_any_element();
        }
    };
    let image = Arc::new(Image::from_bytes(format, asset.bytes.clone()));
    div()
        .id(format!("intake-image-{index}"))
        .test_support()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            img(image)
                .object_fit(ObjectFit::Contain)
                .w(px(480.))
                .h(px(320.)),
        )
        .child(format!(
            "Occurrence {} · source {} · {} · {} × {} · {} bytes · SHA-256 {}",
            occurrence.id,
            occurrence.source_id,
            asset.media_type,
            asset.width,
            asset.height,
            asset.bytes.len(),
            hex(&asset.sha256)
        ))
        .child(format!(
            "Exact locator: {} · extracted bytes {}..{}",
            occurrence.locator, occurrence.start, occurrence.end
        ))
        .child(format!(
            "Upstream alt text: {} · separate authored picture title unavailable",
            occurrence.alt.as_deref().unwrap_or("not supplied")
        ))
        .into_any_element()
}
