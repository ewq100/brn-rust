use super::*;
use markdown::{ParseOptions, mdast::Node};
use std::collections::{BTreeMap, BTreeSet};
use unicode_normalization::UnicodeNormalization;

fn escape(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\\' | '`' | '*' | '_' | '[' | ']' | '#' | '|' | '.' | ')' | '!' | '-' | '+' | '='
            | '~' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    // Only source footnote references are recognized; everything else stays literal.
    let mut result = out;
    let mut cursor = 0;
    while let Some(start) = result[cursor..].find("\\[^").map(|n| n + cursor) {
        let Some(end) = result[start..].find("\\]").map(|n| n + start) else {
            break;
        };
        let label = result[start + 3..end].replace("\\-", "-").replace("\\_", "_");
        if !label.is_empty()
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        {
            result.replace_range(start..end + 2, &format!("[^{label}]"));
            cursor = start + label.len() + 3;
        } else {
            cursor = end + 2;
        }
        if cursor >= result.len() {
            break;
        }
    }
    result
}

fn display_columns(rows: &[Vec<PreparedCell>]) -> Result<Vec<u32>, String> {
    let used: BTreeSet<u32> = rows.iter().flatten().map(|c| c.column).collect();
    if used.len() > 256 {
        return Err("table width budget".into());
    }
    let extent = used.last().copied().unwrap_or(0) + 1;
    Ok(if extent <= 256 {
        (0..extent).collect()
    } else {
        used.into_iter().collect()
    })
}
fn cell_content(cell: &PreparedCell) -> String {
    let mut text = escape(&cell.text).replace('\n', "<br>").replace('\r', "");
    for (label, value) in [
        ("Formula", cell.formula.as_ref()),
        ("Result status", cell.result_status.as_ref()),
        ("Comment", cell.comment.as_ref()),
    ] {
        if let Some(value) = value {
            text.push_str(&format!(
                "<br>{label}: {}",
                escape(value).replace('\n', "<br>")
            ));
        }
    }
    for link in &cell.links {
        text.push_str(&format!("<br>Link: {}", escape(link)));
    }
    if cell.row_span > 1 || cell.column_span > 1 {
        text.push_str(&format!(
            "<br>Span: {} rows × {} columns",
            cell.row_span, cell.column_span
        ));
    }
    text
}

/// Deterministic readable starting point. Binding comments are internal evidence, not visible prose.
pub fn format(doc: &PreparedDocument) -> Result<String, String> {
    doc.validate()?;
    let mut out = String::new();
    for block in &doc.blocks {
        let marker = format!("<!-- source:{} -->", block.id);
        if !matches!(block.content, BlockContent::ListItem { .. })
            && !out.is_empty()
            && !out.ends_with("\n\n")
        {
            out.push('\n');
        }
        match &block.content {
            BlockContent::Paragraph { text } | BlockContent::PageText { text, .. } => {
                out.push_str(&format!("{} {marker}\n\n", escape(text)))
            }
            BlockContent::Heading { text, level } => out.push_str(&format!(
                "{} {} {marker}\n\n",
                "#".repeat(usize::from(*level)),
                escape(text)
            )),
            BlockContent::ListItem { text, level, label } => out.push_str(&format!(
                "{}- {}{} {marker}\n",
                "  ".repeat(usize::from(*level)),
                if label.is_empty() {
                    String::new()
                } else {
                    format!("{} ", escape(label))
                },
                escape(text)
            )),
            BlockContent::Metadata { name, value } => out.push_str(&format!(
                "**{}:** {} {marker}\n\n",
                escape(name),
                escape(value)
            )),
            BlockContent::Footnote { label, text } => {
                out.push_str(&format!("[^{}]: {} {marker}\n\n", label, escape(text)))
            }
            BlockContent::Figure { asset_id, alt } => {
                let asset = doc
                    .assets
                    .iter()
                    .find(|a| &a.id == asset_id)
                    .ok_or("missing asset")?;
                out.push_str(&format!(
                    "![{}]({}) {marker}\n\n",
                    escape(alt),
                    crate::asset_file_name(asset)?
                ));
            }
            BlockContent::Table { rows } => {
                if !out.ends_with("\n\n") {
                    out.push('\n');
                }
                let columns = display_columns(rows)?;
                out.push('|');
                for c in &columns {
                    out.push_str(&format!(" Column {} |", c + 1));
                }
                out.push('\n');
                out.push('|');
                for _ in &columns {
                    out.push_str(" --- |");
                }
                out.push('\n');
                for row in rows {
                    out.push('|');
                    for column in &columns {
                        if let Some(cell) = row.iter().find(|c| c.column == *column) {
                            out.push_str(&format!(
                                " {} <!-- cell:{}:{} --> |",
                                cell_content(cell),
                                block.id,
                                cell.address
                            ));
                        } else {
                            out.push_str(" | ");
                        }
                    }
                    out.push('\n');
                }
                out.push_str(&format!("\n{marker}\n\n"));
            }
        }
    }
    if out.len() > crate::MAX_TEXT_BYTES {
        return Err("draft text budget".into());
    }
    Ok(out)
}

fn visible(node: &Node, out: &mut String) {
    match node {
        Node::Text(t) => {
            out.push_str(&t.value);
        }
        Node::InlineCode(t) => {
            out.push_str(&t.value);
        }
        Node::Code(t) => {
            out.push_str(&t.value);
        }
        Node::Image(_) => {}
        Node::Html(h) if h.value == "<br>" => out.push(' '),
        Node::FootnoteReference(r) => {
            out.push_str(&format!("[^{}]", r.identifier));
        }
        _ => {}
    }
    if let Some(children) = node.children() {
        for child in children {
            visible(child, out);
        }
    }
    if matches!(
        node,
        Node::Paragraph(_) | Node::Heading(_) | Node::TableCell(_) | Node::Code(_) | Node::Break(_)
    ) {
        out.push(' ');
    }
}
fn normalized(text: &str) -> String {
    text.nfkc()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn text_in(markdown: &str) -> Result<String, String> {
    let node = markdown::to_mdast(markdown, &ParseOptions::gfm()).map_err(|e| e.to_string())?;
    let mut result = String::new();
    visible(&node, &mut result);
    Ok(normalized(&result))
}
fn visible_range(node: &Node, start: usize, end: usize, out: &mut String) -> Result<(), String> {
    if let Some(p) = node.position()
        && (p.end.offset <= start || p.start.offset >= end)
    {
        return Ok(());
    }
    if node.children().is_none() {
        if !matches!(node, Node::Html(_) | Node::Image(_))
            && node
                .position()
                .is_some_and(|p| p.start.offset < start || p.end.offset > end)
        {
            return Err("source marker divides a visible text node".into());
        }
        visible(node, out);
    } else if let Some(children) = node.children() {
        for child in children {
            visible_range(child, start, end, out)?;
        }
        if matches!(
            node,
            Node::Paragraph(_)
                | Node::Heading(_)
                | Node::TableCell(_)
                | Node::Code(_)
                | Node::Break(_)
        ) {
            out.push(' ');
        }
    }
    Ok(())
}
fn exact_range(
    ast: &Node,
    start: usize,
    end: usize,
    expected_markdown: &str,
) -> Result<(), String> {
    let mut actual = String::new();
    visible_range(ast, start, end, &mut actual)?;
    if normalized(&actual) != text_in(expected_markdown)? {
        return Err("altered source-bound content".into());
    }
    Ok(())
}
fn enclosing<'a>(node: &'a Node, offset: usize, path: &mut Vec<&'a Node>) {
    if let Some(p) = node.position()
        && (p.start.offset > offset || p.end.offset <= offset)
    {
        return;
    }
    path.push(node);
    if let Some(children) = node.children() {
        for child in children {
            if child
                .position()
                .is_some_and(|p| p.start.offset <= offset && p.end.offset > offset)
            {
                enclosing(child, offset, path);
                break;
            }
        }
    }
}
fn tables<'a>(node: &'a Node, out: &mut Vec<&'a markdown::mdast::Table>) {
    if let Node::Table(table) = node {
        out.push(table);
    }
    if let Some(children) = node.children() {
        for child in children {
            tables(child, out);
        }
    }
}
fn checked_html(node: &Node, markers: &BTreeSet<String>) -> Result<(), String> {
    if let Node::Html(h) = node
        && h.value != "<br>"
        && !markers.contains(h.value.trim())
    {
        return Err("unbound raw HTML in candidate".into());
    }
    if let Some(children) = node.children() {
        for child in children {
            checked_html(child, markers)?;
        }
    }
    Ok(())
}
fn canonical_images(
    node: &Node,
    markdown: &str,
    candidate: &ConversionCandidate,
) -> Result<(), String> {
    if let Node::Image(image) = node {
        let p = image.position.as_ref().ok_or("image position missing")?;
        if !markdown[p.start.offset..p.end.offset].ends_with(&format!("]({})", image.url)) {
            return Err("image destination must use canonical inline syntax".into());
        }
        if let Some(id) = image.url.strip_prefix("crop:")
            && candidate
                .crops
                .iter()
                .find(|c| c.id == id)
                .is_none_or(|c| c.alt != image.alt)
        {
            return Err("crop alt binding mismatch".into());
        }
    }
    if let Some(children) = node.children() {
        for child in children {
            canonical_images(child, markdown, candidate)?;
        }
    }
    Ok(())
}
fn images(node: &Node, out: &mut Vec<String>) {
    if let Node::Image(i) = node {
        out.push(i.url.clone());
    }
    if matches!(node, Node::ImageReference(_)) {
        out.push("unsupported-reference-image".into());
    }
    if let Some(children) = node.children() {
        for child in children {
            images(child, out);
        }
    }
}

/// Check semantic structure bindings, visible text, source identity and image/page receipts.
/// These checks detect corruption; they do not prove the model's interpretation of a scan.
pub fn check_candidate(
    doc: &PreparedDocument,
    candidate: &ConversionCandidate,
) -> Result<(), String> {
    doc.validate()?;
    if candidate.schema != 2
        || candidate.source_sha256 != doc.source_sha256
        || candidate.markdown.len() > crate::MAX_TEXT_BYTES
        || candidate.markdown.contains('\0')
    {
        return Err("candidate profile/hash/text budget".into());
    }
    let ast =
        markdown::to_mdast(&candidate.markdown, &ParseOptions::gfm()).map_err(|e| e.to_string())?;
    let mut markers = BTreeSet::new();
    for block in &doc.blocks {
        markers.insert(format!("<!-- source:{} -->", block.id));
        if let BlockContent::Table { rows } = &block.content {
            for cell in rows.iter().flatten() {
                markers.insert(format!("<!-- cell:{}:{} -->", block.id, cell.address));
            }
        }
    }
    checked_html(&ast, &markers)?;
    canonical_images(&ast, &candidate.markdown, candidate)?;
    let mut previous = 0;
    for block in &doc.blocks {
        let marker = format!("<!-- source:{} -->", block.id);
        let mut matches = candidate.markdown.match_indices(&marker);
        let (position, _) = matches.next().ok_or("missing source binding")?;
        if matches.next().is_some() || position < previous {
            return Err("duplicate/reordered source binding".into());
        }
        let segment = &candidate.markdown[previous..position];
        let mut path = Vec::new();
        enclosing(&ast, position, &mut path);
        if !path
            .iter()
            .any(|n| matches!(n,Node::Html(h) if h.value.contains(&marker)))
        {
            return Err("source binding hidden in literal code".into());
        }
        match &block.content {
            BlockContent::Paragraph { text } => {
                exact_range(&ast, previous, position, &escape(text))?
            }
            BlockContent::PageText { text, .. } if !text.trim().is_empty() => {
                let mut actual = String::new();
                visible_range(&ast, previous, position, &mut actual)?;
                // Extracted PDF text is plain source data. Parsing it as Markdown can
                // consume literal punctuation (for example a table value `--`).
                if normalized(&actual) != normalized(text) {
                    return Err("altered source-bound content".into());
                }
            }
            BlockContent::PageText { .. } => {}
            BlockContent::Heading { text, level } => {
                exact_range(&ast, previous, position, &escape(text))?;
                if !path
                    .iter()
                    .any(|n| matches!(n,Node::Heading(h) if h.depth==*level))
                {
                    return Err("source heading hierarchy changed".into());
                }
            }
            BlockContent::ListItem { text, level, label } => {
                exact_range(
                    &ast,
                    previous,
                    position,
                    &escape(&format!("{label} {text}")),
                )?;
                if path.iter().filter(|n| matches!(n, Node::List(_))).count()
                    != usize::from(*level) + 1
                    || !path.iter().any(|n| matches!(n, Node::ListItem(_)))
                {
                    return Err("source list nesting changed".into());
                }
            }
            BlockContent::Metadata { name, value } => {
                exact_range(
                    &ast,
                    previous,
                    position,
                    &format!("**{}:** {}", escape(name), escape(value)),
                )?;
            }
            BlockContent::Footnote { label, text } => {
                exact_range(&ast, previous, position, &escape(text))?;
                if !path
                    .iter()
                    .any(|n| matches!(n,Node::FootnoteDefinition(f) if &f.identifier==label))
                {
                    return Err("footnote definition binding changed".into());
                }
            }
            BlockContent::Figure { asset_id, .. } => {
                let asset = doc
                    .assets
                    .iter()
                    .find(|a| &a.id == asset_id)
                    .ok_or("figure missing")?;
                let parsed =
                    markdown::to_mdast(segment, &ParseOptions::gfm()).map_err(|e| e.to_string())?;
                let mut urls = Vec::new();
                images(&parsed, &mut urls);
                if !urls.contains(&crate::asset_file_name(asset)?) {
                    return Err("figure occurrence omitted or moved".into());
                }
            }
            BlockContent::Table { rows } => {
                let parsed =
                    markdown::to_mdast(segment, &ParseOptions::gfm()).map_err(|e| e.to_string())?;
                let mut found = Vec::new();
                tables(&parsed, &mut found);
                if found.len() != 1 || found[0].children.len() != rows.len() + 1 {
                    return Err("source table row binding changed".into());
                }
                let columns = display_columns(rows)?;
                for (row_index, row) in rows.iter().enumerate() {
                    let Node::TableRow(actual) = &found[0].children[row_index + 1] else {
                        return Err("invalid table row".into());
                    };
                    for cell in row {
                        let actual = actual
                            .children
                            .get(
                                columns
                                    .iter()
                                    .position(|c| *c == cell.column)
                                    .ok_or("unbound source column")?,
                            )
                            .ok_or("source table column moved")?;
                        let p = actual.position().ok_or("missing table cell range")?;
                        let content = &segment[p.start.offset..p.end.offset];
                        let binding = format!("<!-- cell:{}:{} -->", block.id, cell.address);
                        if content.matches(&binding).count() != 1
                            || candidate.markdown.matches(&binding).count() != 1
                        {
                            return Err("table cell binding moved/duplicated".into());
                        }
                        let mut actual_text = String::new();
                        visible(actual, &mut actual_text);
                        if normalized(&actual_text) != text_in(&cell_content(cell))? {
                            return Err("altered table cell roles/values".into());
                        }
                    }
                }
            }
        }
        previous = position + marker.len();
    }
    if !candidate.markdown[previous..].trim().is_empty() {
        return Err("unbound content after final source block".into());
    }
    let mut table_reviews = BTreeMap::new();
    for review in &candidate.table_reviews {
        if review.uncertainty.len() > 8192
            || table_reviews
                .insert(review.block_id.as_str(), review)
                .is_some()
        {
            return Err("table review duplicate/budget".into());
        }
    }
    let mut table_count = 0;
    for block in &doc.blocks {
        if let BlockContent::Table { rows } = &block.content {
            table_count += 1;
            if table_reviews.get(block.id.as_str()).is_none_or(|r| {
                table_hash(rows).is_err() || table_hash(rows).is_ok_and(|h| r.source_sha256 != h)
            }) {
                return Err("source table review binding mismatch".into());
            }
        }
    }
    if table_reviews.len() != table_count {
        return Err("table review missing/extra".into());
    }
    let mut page_reviews = BTreeMap::new();
    for review in &candidate.page_reviews {
        if review.uncertainty.len() > 8192 || page_reviews.insert(review.page, review).is_some() {
            return Err("page review budget/duplicate".into());
        }
    }
    if page_reviews.len() != doc.pages.len() {
        return Err("page inspection receipts missing/extra".into());
    }
    for page in &doc.pages {
        let preview = doc
            .assets
            .iter()
            .find(|a| a.id == page.preview_asset_id)
            .ok_or("preview missing")?;
        if page_reviews
            .get(&page.number)
            .is_none_or(|r| r.preview_sha256 != preview.sha256)
        {
            return Err("page inspection preview mismatch".into());
        }
        if page.needs_transcription {
            let block = doc
                .blocks
                .iter()
                .find(|b| matches!(b.content,BlockContent::PageText{page:p,..} if p==page.number))
                .ok_or("scan page binding missing")?;
            let marker = format!("<!-- source:{} -->", block.id);
            let end = candidate
                .markdown
                .find(&marker)
                .ok_or("scan binding missing")?;
            let begin = candidate.markdown[..end]
                .rfind("<!-- source:")
                .and_then(|n| candidate.markdown[n..end].find("-->").map(|m| n + m + 3))
                .unwrap_or(0);
            if text_in(&candidate.markdown[begin..end])?.is_empty()
                && page_reviews[&page.number].uncertainty.trim().is_empty()
            {
                return Err("scan page neither transcribed nor marked uncertain".into());
            }
        }
    }
    let mut crop_ids = BTreeSet::new();
    if candidate.crops.len() > crate::MAX_ASSETS {
        return Err("crop count budget".into());
    }
    for crop in &candidate.crops {
        crate::id(&crop.id)?;
        let page = doc
            .pages
            .iter()
            .find(|p| p.number == crop.page)
            .ok_or("crop page missing")?;
        let [l, t, r, b] = crop.bounds;
        if !crop_ids.insert(crop.id.clone())
            || (r - l >= page.width * 0.98 && b - t >= page.height * 0.98)
            || crop.alt.len() > 8192
            || crop.bounds.iter().any(|n| !n.is_finite())
            || l < 0.
            || t < 0.
            || r <= l
            || b <= t
            || r > page.width
            || b > page.height
        {
            return Err("invalid crop geometry/identity".into());
        }
    }
    let mut urls = Vec::new();
    images(&ast, &mut urls);
    if urls.len()
        != doc
            .blocks
            .iter()
            .filter(|b| matches!(b.content, BlockContent::Figure { .. }))
            .count()
            + candidate.crops.len()
    {
        return Err("unbound or repeated image occurrence".into());
    }
    let preview_ids: BTreeSet<_> = doc
        .pages
        .iter()
        .map(|p| p.preview_asset_id.as_str())
        .collect();
    let allowed: BTreeSet<_> = doc
        .assets
        .iter()
        .filter(|a| !preview_ids.contains(a.id.as_str()))
        .map(crate::asset_file_name)
        .collect::<Result<_, _>>()?;
    for url in &urls {
        if let Some(id) = url.strip_prefix("crop:") {
            if !crop_ids.contains(id) {
                return Err("unbound crop link".into());
            }
        } else if !allowed.contains(url) {
            return Err("unbound image or retained full-page preview".into());
        }
    }
    for id in crop_ids {
        if urls
            .iter()
            .filter(|u| u.as_str() == format!("crop:{id}"))
            .count()
            != 1
        {
            return Err("crop must have exactly one figure occurrence".into());
        }
        let crop = candidate
            .crops
            .iter()
            .find(|c| c.id == id)
            .ok_or("crop missing")?;
        let block = doc
            .blocks
            .iter()
            .find(|b| matches!(b.content,BlockContent::PageText{page,..} if page==crop.page))
            .ok_or("crop lacks source page text binding")?;
        let marker = format!("<!-- source:{} -->", block.id);
        let end = candidate
            .markdown
            .find(&marker)
            .ok_or("crop source marker missing")?;
        let begin = doc
            .blocks
            .iter()
            .take_while(|b| b.id != block.id)
            .last()
            .map(|b| format!("<!-- source:{} -->", b.id))
            .and_then(|m| candidate.markdown.find(&m).map(|p| p + m.len()))
            .unwrap_or(0);
        let page_ast = markdown::to_mdast(&candidate.markdown[begin..end], &ParseOptions::gfm())
            .map_err(|e| e.to_string())?;
        let mut page_urls = Vec::new();
        images(&page_ast, &mut page_urls);
        if !page_urls.contains(&format!("crop:{id}")) {
            return Err("crop moved from bound page".into());
        }
    }
    Ok(())
}

/// Check all evidence bindings, then remove only parsed internal source/cell comments.
/// Located source coverage remains separate; formatting evidence never becomes note prose.
pub fn readable_markdown(
    doc: &PreparedDocument,
    candidate: &ConversionCandidate,
) -> Result<String, String> {
    check_candidate(doc, candidate)?;
    let ast =
        markdown::to_mdast(&candidate.markdown, &ParseOptions::gfm()).map_err(|e| e.to_string())?;
    fn ranges(node: &Node, out: &mut Vec<(usize, usize)>) {
        if let Node::Html(h) = node
            && h.value != "<br>"
            && let Some(p) = &h.position
        {
            out.push((p.start.offset, p.end.offset));
        }
        if let Some(children) = node.children() {
            for child in children {
                ranges(child, out);
            }
        }
    }
    let mut comments = Vec::new();
    ranges(&ast, &mut comments);
    let mut out = String::with_capacity(candidate.markdown.len());
    let mut previous = 0;
    for (start, end) in comments {
        out.push_str(&candidate.markdown[previous..start]);
        previous = end;
    }
    out.push_str(&candidate.markdown[previous..]);
    Ok(out)
}

/// Hash source cell roles/coordinates, not rendered Markdown or a model's claimed interpretation.
pub fn table_hash(rows: &[Vec<PreparedCell>]) -> Result<[u8; 32], String> {
    Ok(crate::digest(
        &serde_json::to_vec(rows).map_err(|e| e.to_string())?,
    ))
}

/// Apply bounded model decisions through the prewritten formatter. The model cannot rewrite
/// structured source values, marker grammar, numeric list labels or footnote bindings.
pub fn apply_plan(
    doc: &PreparedDocument,
    plan: &ConversionPlan,
) -> Result<ConversionCandidate, String> {
    doc.validate()?;
    if plan.schema != 2
        || plan.source_sha256 != doc.source_sha256
        || plan.pages.len() != doc.pages.len()
        || plan.tables.len() > crate::MAX_SOURCES
        || plan.crops.len() > crate::MAX_ASSETS
    {
        return Err("conversion plan identity/count budget".into());
    }
    let pages: BTreeMap<_, _> = plan.pages.iter().map(|p| (p.page, p)).collect();
    if pages.len() != plan.pages.len() {
        return Err("duplicate conversion page".into());
    }
    for page in &doc.pages {
        let review = pages.get(&page.number).ok_or("page conversion missing")?;
        let asset = doc
            .assets
            .iter()
            .find(|a| a.id == page.preview_asset_id)
            .ok_or("page preview missing")?;
        if review.preview_sha256 != asset.sha256
            || review.uncertainty.len() > 8192
            || review.markdown.len() > crate::MAX_TEXT_BYTES
            || review.markdown.contains("<!--")
        {
            return Err("page conversion identity/content budget".into());
        }
    }
    let draft = format(doc)?;
    let mut markdown = String::new();
    let mut previous = 0;
    for block in &doc.blocks {
        let marker = std::format!("<!-- source:{} -->", block.id);
        let end = draft[previous..]
            .find(&marker)
            .map(|p| p + previous)
            .ok_or("formatted source marker missing")?;
        if let BlockContent::PageText { page, .. } = block.content {
            let conversion = pages.get(&page).ok_or("page conversion missing")?;
            if !markdown.is_empty() && !markdown.ends_with("\n\n") {
                markdown.push_str("\n\n");
            }
            markdown.push_str(conversion.markdown.trim());
            markdown.push(' ');
        } else {
            markdown.push_str(&draft[previous..end]);
        }
        markdown.push_str(&marker);
        previous = end + marker.len();
    }
    markdown.push_str(&draft[previous..]);
    let candidate = ConversionCandidate {
        schema: 2,
        source_sha256: doc.source_sha256,
        markdown,
        page_reviews: plan
            .pages
            .iter()
            .map(|p| PageReview {
                page: p.page,
                preview_sha256: p.preview_sha256,
                uncertainty: p.uncertainty.clone(),
            })
            .collect(),
        crops: plan.crops.clone(),
        table_reviews: plan.tables.clone(),
    };
    check_candidate(doc, &candidate)?;
    Ok(candidate)
}
