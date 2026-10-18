use brn_intake::document::{DocumentExtraction, DocumentRequest};
use brn_intake::{ImageAsset, asset_file_name, digest, hex};

/// Checked destination-only relocation. No prose, URLs or arbitrary fields are normalized.
pub fn relocate(markdown: &str, from: &str, to: &str) -> Result<String, String> {
    fn visit(node: &markdown::mdast::Node, from: &str, ranges: &mut Vec<(usize, usize)>) {
        if let markdown::mdast::Node::Image(image) = node
            && image.url == from
            && let Some(p) = &image.position
        {
            ranges.push((p.start.offset, p.end.offset));
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, from, ranges);
            }
        }
    }
    let ast = markdown::to_mdast(markdown, &markdown::ParseOptions::default())
        .map_err(|e| e.to_string())?;
    let mut ranges = Vec::new();
    visit(&ast, from, &mut ranges);
    let mut result = markdown.to_owned();
    for (start, end) in ranges.into_iter().rev() {
        let original = &markdown[start..end];
        // Require the exact inline destination; reference-style images stay explicit gaps.
        let needle = format!("]({from})");
        let Some(offset) = original.rfind(&needle) else {
            return Err("unsupported returned asset link syntax".into());
        };
        result.replace_range(start + offset + 2..start + offset + 2 + from.len(), to);
    }
    Ok(result)
}
pub fn project(
    request: &DocumentRequest,
    doc: xberg::ExtractedDocument,
) -> Result<DocumentExtraction, String> {
    if doc.content.len() > brn_intake::MAX_TEXT_BYTES
        || doc.processing_warnings.len() > brn_intake::MAX_OCCURRENCES
        || doc
            .images
            .as_ref()
            .is_some_and(|images| images.len() > brn_intake::MAX_OCCURRENCES)
    {
        return Err("SDK projection budget".into());
    }
    let mut markdown = doc.content;
    let mut gaps: Vec<String> = doc
        .processing_warnings
        .into_iter()
        .map(|w| format!("{}: {}", w.source, w.message))
        .collect();
    let mut assets: Vec<ImageAsset> = Vec::new();
    for image in doc.images.unwrap_or_default() {
        if image.data.is_empty() {
            gaps.push(format!(
                "image {}: SDK returned no retained image bytes",
                image.image_index
            ));
            continue;
        }
        // Common transient asset contract accepts checked PNG only. Other returned formats
        // remain honest gaps; do not fetch or reconstruct missing source data.
        if image.format.as_ref() != "png" {
            gaps.push(format!(
                "image {}: retained format {} is not PNG",
                image.image_index, image.format
            ));
            continue;
        }
        let hash = digest(&image.data);
        let asset = ImageAsset {
            id: format!("asset-{}", hex(&hash)),
            sha256: hash,
            width: image.width.ok_or("returned PNG lacks width")?,
            height: image.height.ok_or("returned PNG lacks height")?,
            media_type: "image/png".into(),
            bytes: image.data.to_vec(),
        };
        let old = image
            .source_path
            .unwrap_or_else(|| format!("image_{}.png", image.image_index));
        let name = asset_file_name(&asset)?;
        markdown = relocate(&markdown, &old, &name)?;
        if !assets.iter().any(|a| a.id == asset.id) {
            assets.push(asset);
        }
    }
    // A normal SDK reference is not evidence that its image bytes were returned.
    fn unretained_image(node: &markdown::mdast::Node, names: &[String]) -> bool {
        if let markdown::mdast::Node::Image(image) = node
            && !names.contains(&image.url)
        {
            return true;
        }
        if matches!(node, markdown::mdast::Node::ImageReference(_)) {
            return true;
        }
        node.children()
            .is_some_and(|children| children.iter().any(|child| unretained_image(child, names)))
    }
    let names = assets
        .iter()
        .map(asset_file_name)
        .collect::<Result<Vec<_>, _>>()?;
    let ast = markdown::to_mdast(&markdown, &markdown::ParseOptions::default())
        .map_err(|e| e.to_string())?;
    if unretained_image(&ast, &names) {
        gaps.push("SDK Markdown contains image references without retained bytes; external content was not fetched".into());
    }
    if request.backend == "xberg_brn" && matches!(request.kind.as_str(), "text" | "markdown") {
        markdown = String::from_utf8(request.bytes.clone()).map_err(|_| "text is not UTF-8")?;
    }
    let result = DocumentExtraction {
        schema: 1,
        backend: request.backend.clone(),
        engine_identity: env!("BRN_ENGINE_ID").into(),
        source_sha256: digest(&request.bytes),
        markdown,
        assets,
        gaps,
    };
    result.validate_for(request)?;
    Ok(result)
}
