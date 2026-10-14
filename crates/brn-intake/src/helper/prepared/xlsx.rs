//! Sparse XLSX evidence, using Calamine for values/formulas and OPC for associations.
use crate::prepared::{BlockContent, PreparedCell, PreparedDocument};
use calamine::{Reader, Xlsx};
use quick_xml::{Reader as XmlReader, events::Event};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
};

const MAX_CELLS: usize = 250_000;
const MAX_XML_NODES: usize = 500_000;

fn push(
    doc: &mut PreparedDocument,
    locator: impl AsRef<str>,
    content: BlockContent,
) -> Result<(), String> {
    super::push(doc, locator.as_ref(), content)
}
fn gap(doc: &mut PreparedDocument, locator: impl AsRef<str>, detail: String) -> Result<(), String> {
    super::gap(doc, locator.as_ref(), &detail)
}
fn add_image(
    doc: &mut PreparedDocument,
    bytes: &[u8],
    locator: impl AsRef<str>,
    alt: &str,
) -> Result<(), String> {
    super::add_image(doc, bytes, locator.as_ref(), alt)
}

#[derive(Default)]
struct Node {
    name: String,
    attrs: BTreeMap<String, String>,
    text: String,
    children: Vec<Node>,
}
impl Node {
    fn attr(&self, name: &str) -> &str {
        self.attrs.get(name).map(String::as_str).unwrap_or("")
    }
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|n| n.name == name)
    }
    fn text_content(&self) -> String {
        let mut result = self.text.clone();
        for child in &self.children {
            result.push_str(&child.text_content());
        }
        result
    }
    fn descendants<'a>(&'a self, name: &str, out: &mut Vec<&'a Node>) {
        if self.name == name {
            out.push(self);
        }
        for child in &self.children {
            child.descendants(name, out);
        }
    }
    fn named(&self, name: &str) -> Vec<&Node> {
        let mut out = Vec::new();
        self.descendants(name, &mut out);
        out
    }
}

fn xml(bytes: &[u8]) -> Result<Node, String> {
    let mut reader = XmlReader::from_reader(bytes);
    let mut stack = vec![Node::default()];
    let mut nodes = 0usize;
    loop {
        let event = reader.read_event().map_err(|e| format!("XLSX XML: {e}"))?;
        let empty = matches!(&event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                nodes += 1;
                if nodes > MAX_XML_NODES || stack.len() > 64 {
                    return Err("XLSX XML structure budget".into());
                }
                let mut node = Node {
                    name: String::from_utf8_lossy(e.local_name().as_ref()).into_owned(),
                    ..Node::default()
                };
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| format!("XLSX attribute: {e}"))?;
                    let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                    let value = attr
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|e| format!("XLSX attribute: {e}"))?
                        .into_owned();
                    if node.attrs.insert(key, value).is_some() {
                        return Err("duplicate XLSX attribute".into());
                    }
                }
                if empty {
                    stack
                        .last_mut()
                        .ok_or("XLSX XML stack")?
                        .children
                        .push(node);
                } else {
                    stack.push(node);
                }
            }
            Event::End(_) => {
                if stack.len() <= 1 {
                    return Err("XLSX XML end without start".into());
                }
                let node = stack.pop().ok_or("XLSX XML stack")?;
                stack
                    .last_mut()
                    .ok_or("XLSX XML stack")?
                    .children
                    .push(node);
            }
            Event::Text(e) => stack
                .last_mut()
                .ok_or("XLSX XML stack")?
                .text
                .push_str(&e.xml10_content().map_err(|e| e.to_string())?),
            Event::CData(e) => stack
                .last_mut()
                .ok_or("XLSX XML stack")?
                .text
                .push_str(&e.xml10_content().map_err(|e| e.to_string())?),
            Event::GeneralRef(e) => {
                let reference = format!("&{};", e.decode().map_err(|e| e.to_string())?);
                let decoded = quick_xml::escape::unescape(&reference)
                    .map_err(|e| format!("XLSX entity: {e}"))?;
                stack
                    .last_mut()
                    .ok_or("XLSX XML stack")?
                    .text
                    .push_str(&decoded);
            }
            Event::DocType(_) => return Err("XLSX DTD is unsupported".into()),
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 || stack[0].children.len() != 1 {
        return Err("XLSX XML must have one complete root".into());
    }
    stack
        .pop()
        .ok_or("XLSX XML root")?
        .children
        .pop()
        .ok_or_else(|| "XLSX XML root".into())
}

#[derive(Clone)]
struct Relation {
    target: String,
    kind: String,
    external: bool,
}
fn resolve(base: &str, target: &str) -> Result<String, String> {
    if target.is_empty() || target.contains(['\\', '\0', '?', '#', ':']) {
        return Err("invalid XLSX part target".into());
    }
    let mut segments: Vec<&str> = if target.starts_with('/') {
        Vec::new()
    } else {
        base.rsplit_once('/')
            .map(|(dir, _)| dir.split('/').collect())
            .unwrap_or_default()
    };
    for segment in target.trim_start_matches('/').split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop().ok_or("XLSX part target escapes package")?;
            }
            _ => segments.push(segment),
        }
    }
    Ok(segments.join("/"))
}
fn relationships(
    parts: &BTreeMap<String, Vec<u8>>,
    part: &str,
) -> Result<BTreeMap<String, Relation>, String> {
    let path = match part.rsplit_once('/') {
        Some((dir, name)) => format!("{dir}/_rels/{name}.rels"),
        None if part.is_empty() => "_rels/.rels".into(),
        None => format!("_rels/{part}.rels"),
    };
    let Some(bytes) = parts.get(&path) else {
        return Ok(BTreeMap::new());
    };
    let root = xml(bytes)?;
    if root.name != "Relationships" {
        return Err("XLSX relationships root".into());
    }
    let mut result = BTreeMap::new();
    for node in root.children.iter().filter(|n| n.name == "Relationship") {
        let external = node.attr("TargetMode") == "External";
        let relation = Relation {
            target: if external {
                node.attr("Target").into()
            } else {
                resolve(part, node.attr("Target"))?
            },
            kind: node.attr("Type").into(),
            external,
        };
        if node.attr("Id").is_empty() || result.insert(node.attr("Id").into(), relation).is_some() {
            return Err("duplicate or empty XLSX relationship ID".into());
        }
    }
    Ok(result)
}
fn part_xml(parts: &BTreeMap<String, Vec<u8>>, path: &str) -> Result<Node, String> {
    xml(parts
        .get(path)
        .ok_or_else(|| format!("missing XLSX part: {path}"))?)
}

fn coordinate(address: &str) -> Result<(u32, u32), String> {
    let mut column = 0u32;
    let mut index = 0usize;
    for byte in address.bytes() {
        if !byte.is_ascii_uppercase() {
            break;
        }
        column = column
            .checked_mul(26)
            .and_then(|n| n.checked_add(u32::from(byte - b'A') + 1))
            .ok_or("XLSX column overflow")?;
        index += 1;
    }
    let row: u32 = address
        .get(index..)
        .ok_or("XLSX address")?
        .parse()
        .map_err(|_| "XLSX address must be A1 coordinates")?;
    if column == 0 || column > 16_384 || row == 0 || row > 1_048_576 {
        return Err("XLSX address outside worksheet".into());
    }
    Ok((row - 1, column - 1))
}
type CellRange = ((u32, u32), (u32, u32));
fn area(reference: &str) -> Result<CellRange, String> {
    let (start, end) = reference.split_once(':').unwrap_or((reference, reference));
    let start = coordinate(start)?;
    let end = coordinate(end)?;
    if end.0 < start.0 || end.1 < start.1 {
        return Err("reversed XLSX range".into());
    }
    Ok((start, end))
}
fn address((row, column): (u32, u32)) -> String {
    let mut n = column + 1;
    let mut chars = Vec::new();
    while n != 0 {
        chars.push((b'A' + ((n - 1) % 26) as u8) as char);
        n = (n - 1) / 26;
    }
    format!("{}{}", chars.iter().rev().collect::<String>(), row + 1)
}
fn blank(pos: (u32, u32)) -> PreparedCell {
    PreparedCell {
        address: address(pos),
        column: pos.1,
        text: String::new(),
        formula: None,
        result_status: None,
        comment: None,
        links: Vec::new(),
        row_span: 1,
        column_span: 1,
    }
}
fn metadata(
    doc: &mut PreparedDocument,
    locator: String,
    name: impl Into<String>,
    value: impl Into<String>,
) -> Result<(), String> {
    push(
        doc,
        locator,
        BlockContent::Metadata {
            name: name.into(),
            value: value.into(),
        },
    )
}

fn styles(
    parts: &BTreeMap<String, Vec<u8>>,
    workbook: &str,
    rels: &BTreeMap<String, Relation>,
) -> Result<Vec<String>, String> {
    let fallback = resolve(workbook, "styles.xml")?;
    let path = rels
        .values()
        .find(|r| r.kind.ends_with("/styles") && !r.external)
        .map(|r| r.target.as_str())
        .unwrap_or(&fallback);
    if !parts.contains_key(path) {
        return Ok(vec!["General".into()]);
    }
    let root = part_xml(parts, path)?;
    let mut formats = BTreeMap::new();
    for n in root.named("numFmt") {
        formats.insert(
            n.attr("numFmtId").to_string(),
            n.attr("formatCode").to_string(),
        );
    }
    let mut result = Vec::new();
    if let Some(xfs) = root.child("cellXfs") {
        for xf in xfs.children.iter().filter(|n| n.name == "xf") {
            let id = xf.attr("numFmtId");
            let code = formats.get(id).cloned().unwrap_or_else(|| match id {
                "" | "0" => "General".into(),
                "1" => "0".into(),
                "2" => "0.00".into(),
                "3" => "#,##0".into(),
                "4" => "#,##0.00".into(),
                "9" => "0%".into(),
                "10" => "0.00%".into(),
                "49" => "@".into(),
                _ => format!("builtin:{id}"),
            });
            result.push(code);
        }
    }
    if result.is_empty() {
        result.push("General".into());
    }
    Ok(result)
}

pub(super) fn read(bytes: &[u8], doc: &mut PreparedDocument) -> Result<(), String> {
    // The prepared entry point already admits the package and owns usage accounting.
    // Reopening through the same maintained bounded OPC reader cannot expand beyond it.
    let parts: BTreeMap<String, Vec<u8>> =
        ooxml_opc::unzip_parts_with_limits(bytes, doc.limits.max_expanded_bytes as u64)
            .map_err(|e| format!("XLSX package: {e}"))?
            .into_iter()
            .collect();
    let roots = relationships(&parts, "")?;
    let offices: Vec<_> = roots
        .values()
        .filter(|r| r.kind.ends_with("/officeDocument") && !r.external)
        .collect();
    if offices.len() != 1 {
        return Err("XLSX needs one internal workbook relationship".into());
    }
    let workbook_path = &offices[0].target;
    let workbook_xml = part_xml(&parts, workbook_path)?;
    if workbook_xml.name != "workbook" {
        return Err("XLSX workbook root".into());
    }
    let workbook_rels = relationships(&parts, workbook_path)?;
    let formats = styles(&parts, workbook_path, &workbook_rels)?;
    // Calamine reserves from untrusted uniqueCount; prove this bound before its constructor.
    let shared_path = resolve(workbook_path, "sharedStrings.xml")?;
    if let Some(bytes) = parts.get(&shared_path) {
        let shared = xml(bytes)?;
        if let Some(n) = shared.attrs.get("uniqueCount")
            && n.parse::<usize>().map_err(|_| "XLSX shared-string count")? > MAX_CELLS
        {
            return Err("XLSX shared-string budget".into());
        }
        if shared.named("si").len() > MAX_CELLS {
            return Err("XLSX shared-string budget".into());
        }
    }
    let mut sheets = Vec::new();
    let mut names = BTreeSet::new();
    let mut count = 0usize;
    for sheet in workbook_xml
        .child("sheets")
        .ok_or("missing XLSX sheets")?
        .children
        .iter()
        .filter(|n| n.name == "sheet")
    {
        if !names.insert(sheet.attr("name")) {
            return Err("duplicate XLSX sheet name".into());
        }
        let relation = workbook_rels
            .get(sheet.attr("r:id"))
            .ok_or("missing XLSX sheet relationship")?;
        if relation.external {
            return Err("external XLSX sheet".into());
        }
        let root = part_xml(&parts, &relation.target)?;
        if relation.kind.ends_with("/worksheet") {
            for cell in root.named("c") {
                coordinate(cell.attr("r"))?;
                count += 1;
                if count > MAX_CELLS {
                    return Err("XLSX aggregate cell budget".into());
                }
                if let Some(formula) = cell.child("f") {
                    if let Some(index) = formula.attrs.get("si")
                        && index
                            .parse::<usize>()
                            .map_err(|_| "XLSX shared formula index")?
                            >= MAX_CELLS
                    {
                        return Err("XLSX shared formula budget".into());
                    }
                    if let Some(reference) = formula.attrs.get("ref") {
                        area(reference)?;
                    }
                }
            }
        }
        sheets.push((sheet, relation, root));
    }
    let mut workbook =
        Xlsx::new(Cursor::new(bytes)).map_err(|e| format!("Calamine workbook: {e}"))?;
    for n in workbook_xml.named("definedName") {
        metadata(
            doc,
            format!("xlsx/workbook/defined-name/{}", n.attr("name")),
            n.attr("name"),
            n.text_content(),
        )?;
    }
    for (index, (sheet, relation, root)) in sheets.iter().enumerate() {
        let locator = format!("xlsx/sheet-{}", index + 1);
        push(
            doc,
            locator.clone(),
            BlockContent::Heading {
                text: sheet.attr("name").into(),
                level: 1,
            },
        )?;
        metadata(
            doc,
            format!("{locator}/state"),
            "Worksheet state",
            if sheet.attr("state").is_empty() {
                "visible"
            } else {
                sheet.attr("state")
            },
        )?;
        if !relation.kind.ends_with("/worksheet") {
            gap(
                doc,
                locator,
                "Non-worksheet sheet requires visual review".into(),
            )?;
            continue;
        }
        let rels = relationships(&parts, &relation.target)?;
        let source_cells = root.named("c");
        let mut sources = BTreeMap::new();
        let mut cells = BTreeMap::new();
        for source in source_cells {
            let pos = coordinate(source.attr("r"))?;
            if sources.insert(pos, source).is_some() {
                return Err("duplicate XLSX cell coordinate".into());
            }
            cells.insert(pos, blank(pos));
        }
        let mut reader = workbook
            .worksheet_cells_reader(sheet.attr("name"))
            .map_err(|e| format!("Calamine worksheet: {e}"))?;
        while let Some(record) = reader
            .next_cell_with_formula()
            .map_err(|e| format!("Calamine cell: {e}"))?
        {
            let source = sources
                .get(&record.pos)
                .ok_or("Calamine cell has no source coordinate")?;
            let cell = cells.get_mut(&record.pos).ok_or("XLSX cell association")?;
            cell.text = calamine::Data::from(record.value).to_string();
            // XLSX numbers are supplied as decimal text. Retain that exact source
            // representation rather than round a long identifier through f64.
            if (source.attr("t").is_empty() || source.attr("t") == "n")
                && let Some(value) = source.child("v").filter(|v| !v.text.is_empty())
            {
                cell.text = value.text.clone();
            }
            if let Some(formula) = source.child("f") {
                cell.formula = record.formula.or_else(|| Some(formula.text_content()));
                if cell.formula.as_deref() == Some("") {
                    gap(
                        doc,
                        format!("{locator}/cell/{}", cell.address),
                        "Shared formula expression is unavailable at this occurrence".into(),
                    )?;
                }
                let cached = source
                    .child("v")
                    .is_some_and(|v| !v.text.is_empty() || source.attr("t") == "str");
                cell.result_status = Some(if cached { "cached" } else { "unknown" }.into());
                if !cached {
                    cell.text.clear();
                }
            }
            let style = if source.attr("s").is_empty() {
                0
            } else {
                source
                    .attr("s")
                    .parse::<usize>()
                    .map_err(|_| "XLSX cell style")?
            };
            let format = formats.get(style).ok_or("XLSX unknown cell style")?;
            if format != "General" && format != "@" {
                metadata(
                    doc,
                    format!("{locator}/cell/{}/number-format", cell.address),
                    "Number format",
                    format.clone(),
                )?;
                let raw = source.child("v").map(|v| v.text.as_str()).unwrap_or("");
                if source.attr("t").is_empty() || source.attr("t") == "n" {
                    if let Some(value) = zero_padded(raw, format) {
                        cell.text = value;
                    } else if !raw.is_empty() {
                        gap(
                            doc,
                            format!("{locator}/cell/{}", cell.address),
                            format!(
                                "Value retained with source number format {format}; full Excel display formatting is not reproduced"
                            ),
                        )?;
                    }
                }
            }
        }
        drop(reader);
        let original_cells = sources.len();
        let cell_limit = MAX_CELLS - count + original_cells;
        enrich_cells(doc, &parts, root, &rels, &locator, &mut cells, cell_limit)?;
        count = count
            .checked_add(cells.len().saturating_sub(original_cells))
            .ok_or("XLSX cell budget overflow")?;
        if count > MAX_CELLS {
            return Err("XLSX aggregate cell budget".into());
        }
        let mut rows: BTreeMap<u32, Vec<PreparedCell>> = BTreeMap::new();
        for ((row, _), cell) in cells {
            rows.entry(row).or_default().push(cell);
        }
        if !rows.is_empty() {
            push(
                doc,
                format!("{locator}/cells"),
                BlockContent::Table {
                    rows: rows.into_values().collect(),
                },
            )?;
        }
        for header in root.named("headerFooter") {
            for item in &header.children {
                let text = item.text_content();
                if text.trim().is_empty() {
                    continue;
                }
                metadata(
                    doc,
                    format!("{locator}/header-footer/{}", item.name),
                    item.name.clone(),
                    text,
                )?;
            }
        }
        for (drawing_index, drawing) in root.named("drawing").iter().enumerate() {
            let relation = rels
                .get(drawing.attr("r:id"))
                .ok_or("missing XLSX drawing relationship")?;
            if relation.external {
                gap(
                    doc,
                    format!("{locator}/drawing-{drawing_index}"),
                    "External drawing was not fetched".into(),
                )?;
                continue;
            }
            pictures(
                doc,
                &parts,
                &relation.target,
                &format!("{locator}/drawing-{}", drawing_index + 1),
            )?;
        }
        for feature in [
            "legacyDrawing",
            "legacyDrawingHF",
            "oleObjects",
            "controls",
            "extLst",
        ] {
            if !root.named(feature).is_empty() {
                gap(
                    doc,
                    format!("{locator}/{feature}"),
                    format!(
                        "{feature} includes content or visual associations not fully reproduced"
                    ),
                )?;
            }
        }
    }
    Ok(())
}

fn zero_padded(raw: &str, format: &str) -> Option<String> {
    if format.is_empty() || format.len() > 256 || !format.bytes().all(|b| b == b'0') {
        return None;
    }
    let number: f64 = raw.parse().ok()?;
    if !number.is_finite() || number.fract() != 0.0 || number.abs() > 9_007_199_254_740_991.0 {
        return None;
    }
    let digits = format!("{:.0}", number.abs());
    Some(format!(
        "{}{:0>width$}",
        if number < 0.0 { "-" } else { "" },
        digits,
        width = format.len()
    ))
}

fn enrich_cells(
    doc: &mut PreparedDocument,
    parts: &BTreeMap<String, Vec<u8>>,
    root: &Node,
    rels: &BTreeMap<String, Relation>,
    locator: &str,
    cells: &mut BTreeMap<(u32, u32), PreparedCell>,
    cell_limit: usize,
) -> Result<(), String> {
    let mut link_count = 0usize;
    for relation in rels
        .values()
        .filter(|r| r.kind.ends_with("/comments") && !r.external)
    {
        let comments = part_xml(parts, &relation.target)?;
        let authors: Vec<_> = comments
            .named("author")
            .into_iter()
            .map(Node::text_content)
            .collect();
        for comment in comments.named("comment") {
            let pos = coordinate(comment.attr("ref"))?;
            let text = comment
                .child("text")
                .map(Node::text_content)
                .unwrap_or_default();
            let cell = cells.entry(pos).or_insert_with(|| blank(pos));
            if cell.comment.replace(text).is_some() {
                return Err("duplicate XLSX cell comment".into());
            }
            if let Ok(index) = comment.attr("authorId").parse::<usize>()
                && let Some(author) = authors.get(index)
            {
                metadata(
                    doc,
                    format!("{locator}/cell/{}/comment-author", cell.address),
                    "Comment author",
                    author.clone(),
                )?;
            }
            if cells.len() > cell_limit {
                return Err("XLSX aggregate cell association budget".into());
            }
        }
    }
    for link in root.named("hyperlink") {
        let (start, end) = area(link.attr("ref"))?;
        let count = u64::from(end.0 - start.0 + 1) * u64::from(end.1 - start.1 + 1);
        if count > MAX_CELLS as u64 {
            return Err("XLSX hyperlink cell budget".into());
        }
        link_count = link_count
            .checked_add(count as usize)
            .ok_or("XLSX hyperlink budget overflow")?;
        if link_count > MAX_CELLS {
            return Err("XLSX hyperlink association budget".into());
        }
        let target = if !link.attr("r:id").is_empty() {
            let relation = rels
                .get(link.attr("r:id"))
                .ok_or("missing XLSX hyperlink relationship")?;
            if !relation.kind.ends_with("/hyperlink") {
                return Err("XLSX hyperlink relationship type".into());
            }
            relation.target.clone()
        } else if !link.attr("location").is_empty() {
            format!("#{}", link.attr("location"))
        } else {
            return Err("XLSX hyperlink has no target".into());
        };
        for row in start.0..=end.0 {
            for column in start.1..=end.1 {
                let pos = (row, column);
                let cell = cells.entry(pos).or_insert_with(|| blank(pos));
                cell.links.push(target.clone());
                if !link.attr("display").is_empty() {
                    metadata(
                        doc,
                        format!("{locator}/cell/{}/link-display", cell.address),
                        "Hyperlink display",
                        link.attr("display"),
                    )?;
                }
                if !link.attr("tooltip").is_empty() {
                    metadata(
                        doc,
                        format!("{locator}/cell/{}/link-tooltip", cell.address),
                        "Hyperlink tooltip",
                        link.attr("tooltip"),
                    )?;
                }
                if cells.len() > cell_limit {
                    return Err("XLSX aggregate cell association budget".into());
                }
            }
        }
    }
    for merge in root.named("mergeCell") {
        let (start, end) = area(merge.attr("ref"))?;
        let cell = cells.entry(start).or_insert_with(|| blank(start));
        cell.row_span = end.0 - start.0 + 1;
        cell.column_span = end.1 - start.1 + 1;
        metadata(
            doc,
            format!("{locator}/merge/{}", cell.address),
            "Merged cells",
            merge.attr("ref"),
        )?;
        if cells.len() > cell_limit {
            return Err("XLSX aggregate cell association budget".into());
        }
    }
    for relation in rels
        .values()
        .filter(|r| r.kind.ends_with("/threadedComment") || r.kind.ends_with("/threadedComments"))
    {
        gap(
            doc,
            format!("{locator}/threaded-comments"),
            "Threaded comment participant associations are not fully reproduced".into(),
        )?;
        if !relation.external {
            let comments = part_xml(parts, &relation.target)?;
            for comment in comments.named("threadedComment") {
                let pos = coordinate(comment.attr("ref"))?;
                let cell = cells.entry(pos).or_insert_with(|| blank(pos));
                let text = comment
                    .child("text")
                    .map(Node::text_content)
                    .unwrap_or_default();
                let existing = cell.comment.get_or_insert_with(String::new);
                if !existing.is_empty() {
                    existing.push('\n');
                }
                existing.push_str(&text);
                if cells.len() > cell_limit {
                    return Err("XLSX aggregate cell association budget".into());
                }
            }
        }
    }
    Ok(())
}

fn pictures(
    doc: &mut PreparedDocument,
    parts: &BTreeMap<String, Vec<u8>>,
    path: &str,
    locator: &str,
) -> Result<(), String> {
    let root = part_xml(parts, path)?;
    let rels = relationships(parts, path)?;
    for (index, anchor) in root.children.iter().enumerate() {
        let anchor_locator = format!("{locator}/anchor-{}", index + 1);
        if let Some(from) = anchor.child("from") {
            metadata(
                doc,
                format!("{anchor_locator}/position"),
                "Drawing anchor",
                format!(
                    "row={}; column={}; row-offset={}; column-offset={}",
                    from.child("row").map(|n| n.text.as_str()).unwrap_or(""),
                    from.child("col").map(|n| n.text.as_str()).unwrap_or(""),
                    from.child("rowOff").map(|n| n.text.as_str()).unwrap_or(""),
                    from.child("colOff").map(|n| n.text.as_str()).unwrap_or("")
                ),
            )?;
        }
        for (picture_index, pic) in anchor.named("pic").iter().enumerate() {
            let picture_locator = format!("{anchor_locator}/picture-{}", picture_index + 1);
            let description = pic
                .named("cNvPr")
                .first()
                .map(|n| {
                    if n.attr("descr").is_empty() {
                        n.attr("name")
                    } else {
                        n.attr("descr")
                    }
                })
                .unwrap_or("Worksheet figure");
            let blips = pic.named("blip");
            if blips.len() != 1 {
                return Err("XLSX picture needs one image association".into());
            }
            let blip = blips[0];
            if !blip.attr("r:embed").is_empty() && !blip.attr("r:link").is_empty() {
                return Err("XLSX picture has ambiguous image associations".into());
            }
            let id = if blip.attr("r:embed").is_empty() {
                blip.attr("r:link")
            } else {
                blip.attr("r:embed")
            };
            let relation = rels.get(id).ok_or("missing XLSX image relationship")?;
            if !relation.kind.ends_with("/image") {
                return Err("XLSX picture relationship type".into());
            }
            if relation.external {
                gap(
                    doc,
                    picture_locator,
                    "External image was not fetched".into(),
                )?;
            } else {
                add_image(
                    doc,
                    parts
                        .get(&relation.target)
                        .ok_or("missing XLSX image part")?,
                    picture_locator,
                    description,
                )?;
            }
        }
        for chart in anchor.named("chart") {
            let relation = rels
                .get(chart.attr("r:id"))
                .ok_or("missing XLSX chart relationship")?;
            gap(
                doc,
                format!("{anchor_locator}/chart"),
                "Chart visual appearance requires review; cached chart text retained separately"
                    .into(),
            )?;
            if !relation.external {
                let chart = part_xml(parts, &relation.target)?;
                for (i, value) in chart.named("v").iter().enumerate() {
                    metadata(
                        doc,
                        format!("{anchor_locator}/chart/cache-{}", i + 1),
                        "Chart cached value",
                        value.text_content(),
                    )?;
                }
                for (i, text) in chart.named("t").iter().enumerate() {
                    metadata(
                        doc,
                        format!("{anchor_locator}/chart/text-{}", i + 1),
                        "Chart text",
                        text.text_content(),
                    )?;
                }
            }
        }
        if !anchor.named("sp").is_empty() || !anchor.named("grpSp").is_empty() {
            gap(
                doc,
                anchor_locator,
                "Drawing shapes or grouped drawing associations are not fully reproduced".into(),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IntakeLimits, IntakeUsage, prepared::PROFILE};
    use std::io::Write;

    fn fixture(sheet: &str, extra: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let root = br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="office" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;
        let workbook = br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Evidence" sheetId="1" r:id="sheet"/></sheets></workbook>"#;
        let rels = br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="sheet" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="styles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#;
        let styles = br#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="1"><numFmt numFmtId="164" formatCode="00000"/></numFmts><cellXfs count="2"><xf numFmtId="0"/><xf numFmtId="164"/></cellXfs></styleSheet>"#;
        for (name, bytes) in [
            ("_rels/.rels", root.as_slice()),
            ("xl/workbook.xml", workbook.as_slice()),
            ("xl/_rels/workbook.xml.rels", rels.as_slice()),
            ("xl/styles.xml", styles.as_slice()),
            ("xl/worksheets/sheet1.xml", sheet.as_bytes()),
        ]
        .into_iter()
        .chain(extra.iter().copied())
        {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }
    fn document() -> PreparedDocument {
        PreparedDocument {
            schema: 2,
            profile: PROFILE.into(),
            source_sha256: [0; 32],
            format: "xlsx".into(),
            limits: IntakeLimits::default(),
            consumed: IntakeUsage::default(),
            blocks: Vec::new(),
            assets: Vec::new(),
            pages: Vec::new(),
            gaps: Vec::new(),
        }
    }
    fn read_checked(bytes: &[u8]) -> PreparedDocument {
        let doc = crate::helper::prepared::prepare(crate::HelperRequest {
            kind: "xlsx".into(),
            bytes: bytes.to_vec(),
            limits: None,
        })
        .unwrap();
        let markdown = crate::prepared::format(&doc).unwrap();
        crate::prepared::check_candidate(
            &doc,
            &crate::prepared::ConversionCandidate {
                schema: 2,
                source_sha256: doc.source_sha256,
                markdown,
                page_reviews: vec![],
                crops: vec![],
                table_reviews: doc
                    .blocks
                    .iter()
                    .filter_map(|b| {
                        if let BlockContent::Table { rows } = &b.content {
                            Some(crate::prepared::TableReview {
                                block_id: b.id.clone(),
                                source_sha256: crate::prepared::table_hash(rows).unwrap(),
                                uncertainty: String::new(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect(),
            },
        )
        .unwrap();
        doc
    }
    fn cells(doc: &PreparedDocument) -> Vec<&PreparedCell> {
        doc.blocks
            .iter()
            .filter_map(|block| match &block.content {
                BlockContent::Table { rows } => Some(rows),
                _ => None,
            })
            .flat_map(|rows| rows.iter().flat_map(|row| row.iter()))
            .collect()
    }
    #[test]
    fn sparse_coordinates_formulas_zero_padding_blank_comments_and_links() {
        let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><dimension ref="A1:F9"/><sheetData><row r="1"><c r="A1" s="1"><v>42</v></c><c r="C1"><f>SUM(A1,2)</f><v>44</v></c><c r="F1"><f>NOW()</f></c></row></sheetData><mergeCells><mergeCell ref="A1:B2"/></mergeCells><hyperlinks><hyperlink ref="E9" r:id="link" tooltip="Reference"/></hyperlinks><headerFooter><oddHeader>Source &amp; ID</oddHeader></headerFooter></worksheet>"#;
        let rels = br#"<Relationships><Relationship Id="comments" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="../comments1.xml"/><Relationship Id="link" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.test/source" TargetMode="External"/></Relationships>"#;
        let comments = br#"<comments><authors><author>Ada</author></authors><commentList><comment ref="D7" authorId="0"><text><r><t>Blank cell note</t></r></text></comment></commentList></comments>"#;
        let bytes = fixture(
            sheet,
            &[
                ("xl/worksheets/_rels/sheet1.xml.rels", rels),
                ("xl/comments1.xml", comments),
            ],
        );
        let doc = read_checked(&bytes);
        let cells = cells(&doc);
        let by_address = |a| *cells.iter().find(|c| c.address == a).unwrap();
        assert_eq!(by_address("A1").text, "00042");
        assert_eq!(
            (by_address("A1").row_span, by_address("A1").column_span),
            (2, 2)
        );
        assert_eq!(by_address("C1").formula.as_deref(), Some("SUM(A1,2)"));
        assert_eq!(by_address("C1").result_status.as_deref(), Some("cached"));
        assert_eq!(by_address("F1").result_status.as_deref(), Some("unknown"));
        assert!(by_address("F1").text.is_empty());
        assert_eq!(by_address("D7").comment.as_deref(), Some("Blank cell note"));
        assert_eq!(by_address("D7").column, 3);
        assert_eq!(by_address("E9").links, ["https://example.test/source"]);
        assert!(doc.blocks.iter().any(|b| matches!(&b.content, BlockContent::Metadata { name, value } if name == "oddHeader" && value == "Source & ID")));
        assert!(!doc.blocks.iter().any(
            |b| matches!(&b.content,BlockContent::Metadata {name,..} if name=="Formula source")
        ));
    }
    #[test]
    fn rejects_unbounded_shared_string_reservation_before_calamine() {
        let bytes = fixture(
            "<worksheet><sheetData/></worksheet>",
            &[(
                "xl/sharedStrings.xml",
                br#"<sst uniqueCount="18446744073709551615"/>"#,
            )],
        );
        assert!(
            read(&bytes, &mut document())
                .unwrap_err()
                .contains("shared-string budget")
        );
    }
    #[test]
    fn rejects_huge_shared_formula_index_before_calamine() {
        let bytes = fixture(
            r#"<worksheet><sheetData><row><c r="A1"><f t="shared" si="18446744073709551615" ref="A1:B1">A1+1</f><v>2</v></c></row></sheetData></worksheet>"#,
            &[],
        );
        assert!(
            read(&bytes, &mut document())
                .unwrap_err()
                .contains("shared formula budget")
        );
    }
    #[test]
    fn sparse_far_apart_cells_do_not_allocate_a_dense_rectangle() {
        let bytes = fixture(
            r#"<worksheet><dimension ref="A1:XFD1048576"/><sheetData><row r="1"><c r="A1"><v>1</v></c></row><row r="1048576"><c r="XFD1048576"><v>2</v></c></row></sheetData></worksheet>"#,
            &[],
        );
        let doc = read_checked(&bytes);
        assert_eq!(cells(&doc).len(), 2);
        assert_eq!(cells(&doc)[1].address, "XFD1048576");
    }
    #[test]
    fn prevents_escaping_relationships_and_duplicate_coordinates() {
        assert!(resolve("xl/workbook.xml", "../../outside.xml").is_err());
        let bytes = fixture(
            r#"<worksheet><sheetData><row><c r="A1"><v>1</v></c><c r="A1"><v>2</v></c></row></sheetData></worksheet>"#,
            &[],
        );
        assert!(
            read(&bytes, &mut document())
                .unwrap_err()
                .contains("duplicate XLSX cell coordinate")
        );
    }
    #[test]
    fn zero_masks_are_conservative_and_never_evaluate_formulas() {
        assert_eq!(zero_padded("-42", "00000"), Some("-00042".into()));
        assert_eq!(zero_padded("4.2", "00000"), None);
        assert_eq!(zero_padded("42", "00000;[Red]00000"), None);
    }
    #[test]
    fn empty_header_footer_fields_are_not_readable_content() {
        let bytes = fixture(
            r#"<worksheet><sheetData><row><c r="A1"><v>1</v></c></row></sheetData><headerFooter><oddHeader>Scope retained</oddHeader><evenHeader/><firstFooter>  </firstFooter></headerFooter></worksheet>"#,
            &[],
        );
        let doc = read_checked(&bytes);
        assert!(doc.blocks.iter().any(|b|matches!(&b.content,BlockContent::Metadata {name,value} if name=="oddHeader"&&value=="Scope retained")));
        assert!(!doc.blocks.iter().any(|b|matches!(&b.content,BlockContent::Metadata {name,..} if name=="evenHeader"||name=="firstFooter")));
    }
    #[test]
    fn preserves_exact_numeric_identifiers_beyond_float_precision() {
        let bytes = fixture(
            r#"<worksheet><sheetData><row><c r="A1"><v>9007199254740993</v></c><c r="B1"><f>A1+1</f><v>9007199254740994</v></c></row></sheetData></worksheet>"#,
            &[],
        );
        let doc = read_checked(&bytes);
        let cells = cells(&doc);
        assert_eq!(cells[0].text, "9007199254740993");
        assert_eq!(cells[1].text, "9007199254740994");
        assert_eq!(cells[1].result_status.as_deref(), Some("cached"));
    }
    #[test]
    fn repeated_image_bytes_keep_distinct_anchored_occurrences() {
        let sheet = r#"<worksheet xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheetData/><drawing r:id="drawing"/></worksheet>"#;
        let sheet_rels = br#"<Relationships><Relationship Id="drawing" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#;
        let drawing = br#"<wsDr xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><twoCellAnchor><from><col>0</col><row>1</row></from><pic><nvPicPr><cNvPr name="First" descr="First occurrence"/></nvPicPr><blipFill><blip r:embed="image"/></blipFill></pic></twoCellAnchor><twoCellAnchor><from><col>3</col><row>7</row></from><pic><nvPicPr><cNvPr name="Second" descr="Second occurrence"/></nvPicPr><blipFill><blip r:embed="image"/></blipFill></pic></twoCellAnchor></wsDr>"#;
        let drawing_rels = br#"<Relationships><Relationship Id="image" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/></Relationships>"#;
        let mut image = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut image, 1, 1);
            encoder.set_color(png::ColorType::Rgb);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[255, 0, 0]).unwrap();
        }
        let bytes = fixture(
            sheet,
            &[
                ("xl/worksheets/_rels/sheet1.xml.rels", sheet_rels),
                ("xl/drawings/drawing1.xml", drawing),
                ("xl/drawings/_rels/drawing1.xml.rels", drawing_rels),
                ("xl/media/image1.png", &image),
            ],
        );
        let doc = read_checked(&bytes);
        assert_eq!(doc.assets.len(), 1);
        let figures: Vec<_> = doc
            .blocks
            .iter()
            .filter(|b| matches!(b.content, BlockContent::Figure { .. }))
            .collect();
        assert_eq!(figures.len(), 2);
        assert_ne!(figures[0].locator, figures[1].locator);
        assert!(
            matches!(&figures[0].content, BlockContent::Figure { alt, .. } if alt == "First occurrence")
        );
        assert!(
            matches!(&figures[1].content, BlockContent::Figure { alt, .. } if alt == "Second occurrence")
        );
    }
    #[test]
    fn shared_formula_followers_retain_their_own_cache_and_expanded_expression() {
        let bytes = fixture(
            r#"<worksheet><sheetData><row><c r="A1"><f t="shared" si="0" ref="A1:B1">A2+1</f><v>2</v></c><c r="B1"><f t="shared" si="0"/><v>9</v></c></row></sheetData></worksheet>"#,
            &[],
        );
        let doc = read_checked(&bytes);
        let cells = cells(&doc);
        assert_eq!(cells[0].formula.as_deref(), Some("A2+1"));
        assert_eq!(cells[1].formula.as_deref(), Some("B2+1"));
        assert_eq!(cells[0].text, "2");
        assert_eq!(cells[1].text, "9");
    }
}
