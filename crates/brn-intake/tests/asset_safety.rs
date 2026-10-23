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

#[test]
fn native_jpeg_bytes_are_raster_decoded_with_finite_limits() {
    let bytes = include_bytes!("fixtures/quay-map.jpg");
    let limits = IntakeLimits::default();
    let dimensions = validate_document_image(bytes, "image/jpeg", &limits).unwrap();
    assert!(dimensions.0 > 0 && dimensions.1 > 0);
    for fraction in [2, 3] {
        let truncated = &bytes[..bytes.len() / fraction];
        assert!(validate_document_image(truncated, "image/jpeg", &limits).is_err());
    }
    let mut trailing = bytes.to_vec();
    trailing.extend_from_slice(b"untrusted\xff\xd9");
    assert!(validate_document_image(&trailing, "image/jpeg", &limits).is_err());
    assert!(validate_document_image(bytes, "image/png", &limits).is_err());
    assert!(validate_document_image(bytes, "image/webp", &limits).is_err());
    assert!(
        validate_document_image(
            bytes,
            "image/jpeg",
            &IntakeLimits {
                max_image_pixels: 1,
                ..limits.clone()
            }
        )
        .is_err()
    );
    assert!(
        validate_document_image(
            bytes,
            "image/jpeg",
            &IntakeLimits {
                max_decoded_bytes: bytes.len() - 1,
                ..limits
            }
        )
        .is_err()
    );
}

#[test]
fn jpeg_envelope_preserves_hash_and_checks_dimensions_and_total_pixels() {
    let bytes = include_bytes!("fixtures/quay-map.jpg").to_vec();
    let request = DocumentRequest {
        schema: 1,
        backend: "xberg".into(),
        kind: "docx".into(),
        limits: IntakeLimits::default(),
        bytes: b"synthetic source bytes".to_vec(),
    };
    let (width, height) = validate_document_image(&bytes, "image/jpeg", &request.limits).unwrap();
    let hash = digest(&bytes);
    let mut result = DocumentExtraction {
        schema: 1,
        backend: "xberg".into(),
        engine_identity: expected_engine_identity("xberg").unwrap(),
        source_sha256: digest(&request.bytes),
        markdown: "figure".into(),
        assets: vec![ImageAsset {
            id: format!("asset-{}", hex(&hash)),
            sha256: hash,
            width,
            height,
            media_type: "image/jpeg".into(),
            bytes: bytes.clone(),
        }],
        gaps: vec![],
    };
    result.validate_for(&request).unwrap();
    assert_eq!(result.assets[0].bytes, bytes);
    let mut limited = request.clone();
    limited.limits.max_total_image_pixels = u64::from(width) * u64::from(height) - 1;
    assert!(result.validate_for(&limited).is_err());
    result.assets[0].height += 1;
    assert!(result.validate_for(&request).is_err());
    result.assets[0].height -= 1;
    result.source_sha256[0] ^= 1;
    assert!(result.validate_for(&request).is_err());
}

#[test]
fn jpeg_recovery_does_not_establish_entropy_integrity() {
    // Both zune-jpeg 0.5.15 strict mode and jpeg-decoder 0.3.2 recover this
    // forged terminal marker. Keep the limitation executable, not a false
    // complete-integrity claim or a custom entropy parser.
    let bytes = include_bytes!("fixtures/quay-map.jpg");
    let mut partial = bytes[..bytes.len() / 2].to_vec();
    partial.extend_from_slice(&[0xff, 0xd9]);
    assert!(validate_document_image(&partial, "image/jpeg", &IntakeLimits::default()).is_ok());
}

#[test]
fn hostile_jpeg_dimensions_are_refused_before_raster_allocation() {
    let mut bytes = include_bytes!("fixtures/quay-map.jpg").to_vec();
    let sof = bytes.windows(2).position(|x| x == [0xff, 0xc0]).unwrap();
    bytes[sof + 5..sof + 9].fill(0xff); // 65,535 × 65,535 pixels; tiny encoded file.
    let error =
        validate_document_image(&bytes, "image/jpeg", &IntakeLimits::default()).unwrap_err();
    assert!(error.contains("dimension/pixel budget"), "{error}");
}

#[test]
fn public_image_validation_rejects_unqualified_limits() {
    for pixels in [0, u64::MAX] {
        let limits = IntakeLimits {
            max_image_pixels: pixels,
            ..IntakeLimits::default()
        };
        for (mime, bytes) in [
            (
                "image/png",
                include_bytes!("fixtures/water-use.png").as_slice(),
            ),
            (
                "image/jpeg",
                include_bytes!("fixtures/quay-map.jpg").as_slice(),
            ),
        ] {
            assert!(validate_document_image(bytes, mime, &limits).is_err());
        }
    }
}
