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
