//! Inert HTML tree projection. Neither scripts nor resource loads are executed.
use super::{add_image, gap, push};
use crate::prepared::*;
use base64::Engine;
use html5ever::{parse_document, tendril::TendrilSink};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::collections::BTreeMap;

fn tag(node: &Handle) -> &str {
    match &node.data {
        NodeData::Element { name, .. } => name.local.as_ref(),
        _ => "",
    }
}
fn attr(node: &Handle, key: &str) -> Option<String> {
    match &node.data {
        NodeData::Element { attrs, .. } => attrs
            .borrow()
            .iter()
            .find(|a| a.name.local.as_ref() == key)
            .map(|a| a.value.to_string()),
        _ => None,
    }
}
fn inline(node: &Handle) -> String {
    match &node.data {
        NodeData::Text { contents } => contents.borrow().to_string(),
        NodeData::Element { .. }
            if matches!(
                tag(node),
                "script" | "style" | "noscript" | "ul" | "ol" | "table" | "img" | "figure"
            ) =>
        {
            String::new()
        }
        NodeData::Element { .. } if tag(node) == "br" => "\n".into(),
        _ => {
            let mut value = node
                .children
                .borrow()
                .iter()
                .map(inline)
                .collect::<String>();
            if tag(node) == "a"
                && let Some(target) = attr(node, "href")
            {
                value.push_str(&format!(" ({target})"));
            }
            if matches!(tag(node), "p" | "div") {
                value.push('\n');
            }
            value
        }
    }
}
fn admitted(node: &Handle, depth: usize, count: &mut usize) -> Result<(), String> {
    *count += 1;
    if depth > 128 || *count > 200_000 {
        return Err("HTML depth/node budget".into());
    }
    for child in node.children.borrow().iter() {
        admitted(child, depth + 1, count)?;
    }
    Ok(())
}
pub(super) fn read(
    bytes: &[u8],
    doc: &mut PreparedDocument,
    prefix: &str,
    images: bool,
) -> Result<(), String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "HTML UTF-8 decoding unavailable")?;
    if text.len() > crate::MAX_TEXT_BYTES || text.contains('\0') {
        return Err("HTML text budget".into());
    }
    let dom = parse_document(RcDom::default(), Default::default()).one(text);
    admitted(&dom.document, 0, &mut 0)?;
    walk(&dom.document, doc, prefix, 0, images)
}
fn embedded(
    node: &Handle,
    doc: &mut PreparedDocument,
    locator: &str,
    level: u8,
    images: bool,
) -> Result<(), String> {
    for (i, child) in node.children.borrow().iter().enumerate() {
        let location = format!("{locator}/node/{i}/{}", tag(child));
        if matches!(tag(child), "ul" | "ol" | "table" | "img" | "figure") {
            walk(child, doc, &location, level, images)?;
        } else {
            embedded(child, doc, &location, level, images)?;
        }
    }
    Ok(())
}
fn label(counter: i64, style: &str) -> Option<String> {
    match style {
        "1" => Some(format!("{counter}.")),
        "a" | "A" if counter > 0 && counter <= 100_000 => {
            let mut value = counter;
            let mut chars = Vec::new();
            while value > 0 {
                value -= 1;
                chars.push(b'a' + (value % 26) as u8);
                value /= 26;
            }
            chars.reverse();
            let text = String::from_utf8(chars).ok()?;
            Some(format!(
                "{}.",
                if style == "A" {
                    text.to_ascii_uppercase()
                } else {
                    text
                }
            ))
        }
        _ => None,
    }
}
fn integer(
    node: &Handle,
    name: &str,
    default: i64,
    doc: &mut PreparedDocument,
    locator: &str,
) -> Result<i64, String> {
    match attr(node, name) {
        Some(value) => match value.parse::<i64>() {
            Ok(value) if value.abs_diff(0) <= 1_000_000 => Ok(value),
            _ => {
                gap(
                    doc,
                    locator,
                    format!("Invalid {name} list value; displayed ordinal unqualified"),
                )?;
                Ok(default)
            }
        },
        None => Ok(default),
    }
}
fn walk(
    node: &Handle,
    doc: &mut PreparedDocument,
    locator: &str,
    level: u8,
    images: bool,
) -> Result<(), String> {
    let name = tag(node);
    if matches!(name, "script" | "style" | "noscript" | "template") {
        return Ok(());
    }
    if matches!(name, "ul" | "ol") {
        if level > 32 {
            return Err("HTML list nesting budget".into());
        }
        let children = node.children.borrow();
        let reversed = attr(node, "reversed").is_some();
        let default = if reversed {
            children.iter().filter(|c| tag(c) == "li").count() as i64
        } else {
            1
        };
        let mut ordinal = integer(node, "start", default, doc, locator)?;
        let style = attr(node, "type").unwrap_or_else(|| "1".into());
        for (i, child) in children.iter().enumerate() {
            let location = format!("{locator}/item/{i}");
            if tag(child) != "li" {
                walk(child, doc, &location, level, images)?;
                continue;
            }
            ordinal = integer(child, "value", ordinal, doc, &location)?;
            let marker = if name == "ul" {
                String::new()
            } else {
                match label(ordinal, &style) {
                    Some(v) => v,
                    None => {
                        gap(
                            doc,
                            &location,
                            "List glyph format unresolved; ordinal is retained as decimal",
                        )?;
                        format!("{ordinal}.")
                    }
                }
            };
            push(
                doc,
                &location,
                BlockContent::ListItem {
                    text: inline(child).trim().into(),
                    level,
                    label: marker,
                },
            )?;
            embedded(child, doc, &location, level + 1, images)?;
            ordinal = if reversed {
                ordinal.saturating_sub(1)
            } else {
                ordinal.saturating_add(1)
            };
        }
        return Ok(());
    }
    if name == "table" {
        return table(node, doc, locator, level, images);
    }
    if name == "img" {
        if !images {
            return Ok(());
        }
        let source = attr(node, "src").unwrap_or_default();
        let alt = attr(node, "alt").unwrap_or_else(|| "Source figure".into());
        if let Some((header, encoded)) = source.split_once(',')
            && ["data:image/png;base64", "data:image/jpeg;base64"]
                .iter()
                .any(|h| header.eq_ignore_ascii_case(h))
        {
            if encoded.len() > crate::MAX_IMAGE_BYTES * 4 / 3 + 4 {
                return Err("HTML image encoded budget".into());
            }
            match base64::engine::general_purpose::STANDARD.decode(encoded) {
                Ok(raw) => add_image(doc, &raw, locator, &alt)?,
                Err(_) => gap(doc, locator, "Invalid embedded image base64")?,
            }
        } else {
            gap(
                doc,
                locator,
                format!(
                    "Image resource unavailable within supplied scope (no filesystem/network fetch): {source}"
                ),
            )?;
        }
        return Ok(());
    }
    if matches!(
        name,
        "p" | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "pre"
            | "figcaption"
            | "caption"
            | "blockquote"
    ) {
        let text = inline(node).trim().to_string();
        if !text.is_empty() {
            let content = if name.len() == 2 && name.starts_with('h') {
                BlockContent::Heading {
                    text,
                    level: name.as_bytes()[1] - b'0',
                }
            } else {
                BlockContent::Paragraph { text }
            };
            push(doc, locator, content)?;
        }
        embedded(node, doc, locator, level, images)?;
        return Ok(());
    }
    if let NodeData::Text { contents } = &node.data {
        let text = contents.borrow().trim().to_string();
        if !text.is_empty() {
            push(doc, locator, BlockContent::Paragraph { text })?;
        }
        return Ok(());
    }
    for (i, child) in node.children.borrow().iter().enumerate() {
        walk(
            child,
            doc,
            &format!("{locator}/node/{i}/{}", tag(child)),
            level,
            images,
        )?;
    }
    Ok(())
}
fn rows(node: &Handle, out: &mut Vec<Handle>) {
    for child in node.children.borrow().iter() {
        if tag(child) == "tr" {
            out.push(child.clone());
        } else if matches!(tag(child), "thead" | "tbody" | "tfoot") {
            rows(child, out);
        }
    }
}
fn links(node: &Handle, out: &mut Vec<String>) {
    if tag(node) == "a"
        && let Some(link) = attr(node, "href")
    {
        out.push(link);
    }
    for child in node.children.borrow().iter() {
        links(child, out);
    }
}
fn span(node: &Handle, key: &str) -> Result<u32, String> {
    match attr(node, key) {
        None => Ok(1),
        Some(v) => v
            .parse::<u32>()
            .ok()
            .filter(|v| *v > 0 && *v <= 10_000)
            .ok_or_else(|| "HTML span budget/invalid span".into()),
    }
}
fn table(
    node: &Handle,
    doc: &mut PreparedDocument,
    locator: &str,
    level: u8,
    images: bool,
) -> Result<(), String> {
    let mut source_rows = Vec::new();
    rows(node, &mut source_rows);
    let mut projected = Vec::new();
    let mut occupied = BTreeMap::<u32, u32>::new();
    for (r, row) in source_rows.iter().enumerate() {
        let mut cells = Vec::new();
        let mut column = 0;
        for cell in row
            .children
            .borrow()
            .iter()
            .filter(|c| matches!(tag(c), "td" | "th"))
        {
            while occupied.get(&column).is_some_and(|until| *until > r as u32) {
                column += 1;
            }
            let column_span = span(cell, "colspan")?;
            let row_span = span(cell, "rowspan")?;
            if column.saturating_add(column_span) > 10_000 {
                return Err("HTML table column budget".into());
            }
            let address = format!("R{}C{}", r + 1, column + 1);
            let mut targets = Vec::new();
            links(cell, &mut targets);
            cells.push(PreparedCell {
                address: address.clone(),
                column,
                text: inline(cell).trim().into(),
                formula: None,
                result_status: None,
                comment: None,
                links: targets,
                row_span,
                column_span,
            });
            if row_span != 1 || column_span != 1 {
                gap(
                    doc,
                    format!("{locator}/{address}"),
                    "Source table spans retained at exact grid coordinates; flattened layout requires source review",
                )?;
            }
            for covered in column..column + column_span {
                occupied.insert(covered, r as u32 + row_span);
            }
            column += column_span;
        }
        projected.push(cells);
    }
    push(doc, locator, BlockContent::Table { rows: projected })?;
    for (i, child) in node.children.borrow().iter().enumerate() {
        if tag(child) == "caption" {
            walk(child, doc, &format!("{locator}/caption/{i}"), level, images)?;
        }
    }
    for (r, row) in source_rows.iter().enumerate() {
        for (c, cell) in row.children.borrow().iter().enumerate() {
            if matches!(tag(cell), "td" | "th") {
                embedded(
                    cell,
                    doc,
                    &format!("{locator}/row/{r}/cell/{c}"),
                    level,
                    images,
                )?;
            }
        }
    }
    Ok(())
}
