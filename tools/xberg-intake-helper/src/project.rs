use brn_intake::document::{DocumentExtraction, DocumentRequest};
use brn_intake::{ImageAsset, asset_file_name, digest, hex};

/// Relocate image occurrences, not shared definitions or ordinary links. References
/// without titles become the inline shape consumed by existing host adoption.
pub fn relocate(markdown: &str, from: &str, to: &str) -> Result<String, String> {
    use markdown::mdast::Node;
    fn definitions(node: &Node, out: &mut std::collections::BTreeMap<String, (String, bool)>) {
        if let Node::Definition(d) = node {
            out.entry(d.identifier.clone())
                .or_insert((d.url.clone(), d.title.is_some()));
        }
        if let Some(children) = node.children() {
            for child in children {
                definitions(child, out);
            }
        }
    }
    fn visit(
        node: &Node,
        text: &str,
        from: &str,
        to: &str,
        defs: &std::collections::BTreeMap<String, (String, bool)>,
        edits: &mut Vec<(usize, usize, String)>,
    ) {
        match node {
            Node::Image(i) if i.url == from && i.title.is_none() => {
                if let Some(p) = &i.position {
                    let raw = &text[p.start.offset..p.end.offset];
                    let needle = format!("]({from})");
                    if let Some(offset) = raw.rfind(&needle) {
                        edits.push((
                            p.start.offset + offset + 2,
                            p.start.offset + offset + 2 + from.len(),
                            to.into(),
                        ));
                    }
                }
            }
            Node::ImageReference(i)
                if defs
                    .get(&i.identifier)
                    .is_some_and(|(url, title)| url == from && !title) =>
            {
                if let Some(p) = &i.position {
                    let alt = i
                        .alt
                        .replace('\\', "\\\\")
                        .replace('[', "\\[")
                        .replace(']', "\\]");
                    edits.push((p.start.offset, p.end.offset, format!("![{alt}]({to})")));
                }
            }
            _ => {}
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, text, from, to, defs, edits);
            }
        }
    }
    let ast = markdown::to_mdast(markdown, &markdown::ParseOptions::default())
        .map_err(|e| e.to_string())?;
    let mut defs = std::collections::BTreeMap::new();
    definitions(&ast, &mut defs);
    let mut edits = Vec::new();
    visit(&ast, markdown, from, to, &defs, &mut edits);
    let mut result = markdown.to_owned();
    for (start, end, replacement) in edits.into_iter().rev() {
        result.replace_range(start..end, &replacement);
    }
    Ok(result)
}
pub fn project(
    request: &DocumentRequest,
    doc: xberg::ExtractedDocument,
) -> Result<DocumentExtraction, String> {
    request.validate()?;
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
    let mut returned_bytes = 0usize;
    for (position, image) in doc.images.unwrap_or_default().into_iter().enumerate() {
        if image.data.is_empty() {
            gaps.push(format!(
                "image {}: SDK returned no retained image bytes",
                image.image_index
            ));
            continue;
        }
        returned_bytes = returned_bytes
            .checked_add(image.data.len())
            .ok_or("returned image byte overflow")?;
        if returned_bytes > request.limits.max_decoded_bytes
            || returned_bytes > brn_intake::MAX_IMAGE_BYTES
        {
            return Err("returned image byte budget".into());
        }
        let media_type = match image.format.as_ref() {
            "png" => "image/png",
            "jpeg" | "jpg" => "image/jpeg",
            _ => {
                gaps.push(format!(
                    "image {}: retained format {} is unsupported (PNG/JPEG only)",
                    image.image_index, image.format
                ));
                continue;
            }
        };
        // Office SDK width/height may be layout extents rather than raster dimensions.
        let (width, height) =
            brn_intake::validate_document_image(&image.data, media_type, &request.limits)?;
        let hash = digest(&image.data);
        let asset = ImageAsset {
            id: format!("asset-{}", hex(&hash)),
            sha256: hash,
            width,
            height,
            media_type: media_type.into(),
            bytes: image.data.to_vec(),
        };
        // Xberg's maintained Markdown renderer uses the image's vector position
        // when bytes are present, not its package source_path or page-local index.
        let old = format!("image_{position}.{}", image.format);
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
        gaps.push("SDK Markdown contains image references that could not be linked to retained assets (missing bytes or unsupported link syntax); external content was not fetched".into());
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
