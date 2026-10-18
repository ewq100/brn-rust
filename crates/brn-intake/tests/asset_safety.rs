use brn_intake::{document::*, *};

#[test]
fn maintained_png_decoder_requires_complete_terminal_chunks_and_checksums() {
    let image = include_bytes!("fixtures/water-use.png");
    let dimensions = validate_png_image(image).unwrap();
    assert!(dimensions.0 > 0 && dimensions.1 > 0);
    assert!(validate_png_image(&image[..image.len() - 1]).is_err());
    assert!(validate_png_image(&image[..image.len() / 2]).is_err());
    let mut damaged = image.to_vec();
    damaged[40] ^= 1;
    assert!(validate_png_image(&damaged).is_err());
    let mut trailing = image.to_vec();
    trailing.push(0);
    assert!(validate_png_image(&trailing).is_err());
}

#[test]
fn current_asset_codec_hash_dimensions_and_pixel_limits_are_checked() {
    let bytes = include_bytes!("fixtures/water-use.png").to_vec();
    let (width, height) = validate_png_image(&bytes).unwrap();
    let hash = digest(&bytes);
    let asset = ImageAsset {
        id: format!("asset-{}", hex(&hash)),
        sha256: hash,
        width,
        height,
        media_type: "image/png".into(),
        bytes,
    };
    let request = DocumentRequest {
        schema: 1,
        backend: "xberg".into(),
        kind: "text".into(),
        limits: Default::default(),
        bytes: b"fixed synthetic input".to_vec(),
    };
    let mut result = DocumentExtraction {
        schema: 1,
        backend: request.backend.clone(),
        engine_identity: expected_engine_identity("xberg").unwrap(),
        source_sha256: digest(&request.bytes),
        markdown: format!("![figure]({})", asset_file_name(&asset).unwrap()),
        assets: vec![asset],
        gaps: vec![],
    };
    result.validate_for(&request).unwrap();
    let encoded = serde_json::to_vec(&result).unwrap();
    let decoded: DocumentExtraction = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, result);
    let mut bounded = request.clone();
    bounded.limits.max_image_pixels = 1;
    assert!(result.validate_for(&bounded).is_err());
    result.assets[0].width += 1;
    assert!(result.validate_for(&request).is_err());
    result.assets[0].width -= 1;
    result.assets[0].bytes.push(0);
    assert!(result.validate_for(&request).is_err());
}
