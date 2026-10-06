//! Conservative WordprocessingML text rendering; never flattens unknown content.
use super::{Failure, Result, check_cancel};
use roxmltree::{Document, Node};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::AtomicBool,
};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const WS: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const RS: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships";
const LIMIT: usize = crate::work::MAX_NOTE_BYTES;

fn word<'i>(node: Node<'_, 'i>) -> Result<&'i str> {
    if !matches!(node.tag_name().namespace(), Some(W | WS)) {
        return Err(Failure::Unsupported);
    }
    Ok(node.tag_name().name())
}
fn attr<'a>(node: Node<'a, '_>, key: &str) -> Option<&'a str> {
    node.attribute((W, key))
        .or_else(|| node.attribute((WS, key)))
}
fn children<'a, 'i>(node: Node<'a, 'i>) -> Result<Vec<Node<'a, 'i>>> {
    if node
        .children()
        .any(|n| n.is_text() && !n.text().unwrap_or_default().trim().is_empty())
    {
        return Err(Failure::Invalid);
    }
    Ok(node.children().filter(Node::is_element).collect())
}
fn child<'a, 'i>(node: Node<'a, 'i>, name: &str) -> Result<Option<Node<'a, 'i>>> {
    let mut found = None;
    for n in children(node)? {
        if word(n)? == name {
            if found.replace(n).is_some() {
                return Err(Failure::Invalid);
            }
        }
    }
    Ok(found)
}
fn val<'a>(node: Node<'a, '_>) -> Result<&'a str> {
    if !children(node)?.is_empty() {
        return Err(Failure::Unsupported);
    }
    attr(node, "val").ok_or(Failure::Invalid)
}
fn number(node: Node<'_, '_>) -> Result<u32> {
    val(node)?.parse().map_err(|_| Failure::Invalid)
}
fn boolean(node: Node<'_, '_>) -> Result<bool> {
    if !children(node)?.is_empty() {
        return Err(Failure::Unsupported);
    }
    match attr(node, "val") {
        None | Some("1" | "true" | "on") => Ok(true),
        Some("0" | "false" | "off") => Ok(false),
        _ => Err(Failure::Invalid),
    }
}
fn push(out: &mut String, text: &str) -> Result<()> {
    if out.len().checked_add(text.len()).is_none_or(|n| n > LIMIT) {
        return Err(Failure::Limit);
    }
    out.push_str(text);
    Ok(())
}
fn escaped(text: &str, cancel: &AtomicBool) -> Result<String> {
    let mut out = String::new();
    for c in text.chars() {
        check_cancel(cancel)?;
        match c {
            '\n' => push(&mut out, "<br>")?,
            '&' => push(&mut out, "&amp;")?,
            '<' => push(&mut out, "&lt;")?,
            '>' => push(&mut out, "&gt;")?,
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '.'
            | '!' | '|' | '~' => {
                push(&mut out, "\\")?;
                push(&mut out, c.encode_utf8(&mut [0; 4]))?;
            }
            _ => push(&mut out, c.encode_utf8(&mut [0; 4]))?,
        }
    }
    Ok(out)
}

#[derive(Clone, Copy, Default)]
struct Emphasis {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    vertical: i8,
}
#[derive(Clone, Copy, Default)]
struct Paragraph {
    outline: Option<u32>,
    list: Option<(u32, u32)>,
}

fn neutral_paint(node: Node<'_, '_>) -> Result<()> {
    if !children(node)?.is_empty() {
        return Err(Failure::Unsupported);
    }
    let name = word(node)?;
    let allowed: &[&str] = if name == "shd" {
        &["val", "color", "fill"]
    } else if name == "u" {
        &["val", "color"]
    } else {
        &["val"]
    };
    for attribute in node.attributes() {
        if !matches!(attribute.namespace(), Some(W | WS)) || !allowed.contains(&attribute.name()) {
            return Err(Failure::Unsupported);
        }
    }
    match name {
        "color" if val(node)? == "auto" => {}
        "highlight" if val(node)? == "none" => {}
        "u" if matches!(attr(node, "val").unwrap_or("single"), "single" | "none")
            && attr(node, "color").is_none_or(|value| value == "auto") => {}
        "shd"
            if matches!(val(node)?, "nil" | "clear")
                && ["color", "fill"]
                    .into_iter()
                    .all(|key| attr(node, key).is_none_or(|value| value == "auto")) => {}
        _ => return Err(Failure::Unsupported),
    }
    Ok(())
}

fn tooltip_title(out: &mut String, tooltip: &str, cancel: &AtomicBool) -> Result<()> {
    if tooltip.is_empty() {
        return Ok(());
    }
    push(out, " \"")?;
    for character in tooltip.chars() {
        check_cancel(cancel)?;
        match character {
            '&' => push(out, "&amp;")?,
            '<' => push(out, "&lt;")?,
            '>' => push(out, "&gt;")?,
            '"' => push(out, "&quot;")?,
            '\\' => push(out, "&#92;")?,
            '\n' => push(out, "&#10;")?,
            '\r' => push(out, "&#13;")?,
            '\t' => push(out, "&#9;")?,
            _ => push(out, character.encode_utf8(&mut [0; 4]))?,
        }
    }
    push(out, "\"")
}

fn metadata(node: Node<'_, '_>) -> Result<()> {
    // Layout may be discarded only when its paint carries no additional emphasis.
    if word(node)? == "shd" {
        neutral_paint(node)?;
    }
    for attribute in node.attributes() {
        if attribute.name().starts_with("theme")
            || (matches!(attribute.name(), "color" | "fill") && attribute.value() != "auto")
        {
            return Err(Failure::Unsupported);
        }
    }
    // Only known layout/property containers and leaves may be discarded.
    for n in children(node)? {
        if !matches!(
            word(n)?,
            "keepNext"
                | "keepLines"
                | "pageBreakBefore"
                | "widowControl"
                | "spacing"
                | "ind"
                | "jc"
                | "tabs"
                | "tab"
                | "pBdr"
                | "top"
                | "left"
                | "bottom"
                | "right"
                | "between"
                | "bar"
                | "start"
                | "end"
                | "insideH"
                | "insideV"
                | "shd"
                | "contextualSpacing"
                | "snapToGrid"
                | "col"
                | "tcW"
                | "tcBorders"
                | "tcMar"
                | "vAlign"
                | "noWrap"
                | "hideMark"
                | "tblW"
                | "tblInd"
                | "tblBorders"
                | "tblLayout"
                | "tblCellMar"
                | "tblLook"
                | "trHeight"
                | "cantSplit"
                | "gridCol"
        ) {
            return Err(Failure::Unsupported);
        }
        metadata(n)?;
    }
    Ok(())
}

fn run_properties(node: Node<'_, '_>, mut emphasis: Emphasis, toggle: bool) -> Result<Emphasis> {
    let mut seen = BTreeSet::new();
    for n in children(node)? {
        let name = word(n)?;
        if !seen.insert(name) {
            return Err(Failure::Invalid);
        }
        match name {
            "b" => {
                let v = boolean(n)?;
                emphasis.bold = if toggle { emphasis.bold ^ v } else { v };
            }
            "i" => {
                let v = boolean(n)?;
                emphasis.italic = if toggle { emphasis.italic ^ v } else { v };
            }
            "u" => {
                neutral_paint(n)?;
                emphasis.underline = match attr(n, "val").unwrap_or("single") {
                    "none" => false,
                    "single" => true,
                    _ => return Err(Failure::Unsupported),
                }
            }
            "strike" => {
                let v = boolean(n)?;
                emphasis.strike = if toggle { emphasis.strike ^ v } else { v };
            }
            "vertAlign" => {
                emphasis.vertical = match val(n)? {
                    "baseline" => 0,
                    "superscript" => 1,
                    "subscript" => -1,
                    _ => return Err(Failure::Unsupported),
                }
            }
            "vanish" | "webHidden" | "dstrike" | "bCs" | "iCs" | "caps" | "smallCaps" | "rtl" => {
                if boolean(n)? {
                    return Err(Failure::Unsupported);
                }
            }
            "color" | "highlight" | "shd" => neutral_paint(n)?,
            "rStyle" | "rFonts" | "sz" | "szCs" | "lang" | "spacing" | "kern" | "position"
            | "noProof" | "snapToGrid" => {
                // Position modifies baseline semantics; only neutral positioning is accepted.
                if name == "position" && val(n)? != "0" {
                    return Err(Failure::Unsupported);
                }
            }
            _ => return Err(Failure::Unsupported),
        }
        if !children(n)?.is_empty() {
            return Err(Failure::Unsupported);
        }
    }
    Ok(emphasis)
}
fn layout_run_properties(node: Node<'_, '_>) -> Result<()> {
    run_properties(node, Emphasis::default(), false)?;
    if children(node)?.iter().any(|n| {
        matches!(
            word(*n).ok(),
            Some("b" | "i" | "u" | "strike" | "vertAlign" | "rStyle")
        )
    }) {
        return Err(Failure::Unsupported);
    }
    Ok(())
}
fn table_properties(node: Node<'_, '_>, allow_style: bool) -> Result<()> {
    let mut seen = BTreeSet::new();
    for property in children(node)? {
        let name = word(property)?;
        if !seen.insert(name) {
            return Err(Failure::Invalid);
        }
        if name == "tblStyle" && allow_style {
            val(property)?;
            continue;
        }
        if !matches!(
            name,
            "tblW"
                | "tblInd"
                | "tblBorders"
                | "tblLayout"
                | "tblCellMar"
                | "tblLook"
                | "jc"
                | "tblCaption"
                | "tblDescription"
        ) {
            return Err(Failure::Unsupported);
        }
        metadata(property)?;
        if matches!(name, "tblCaption" | "tblDescription") && !val(property)?.is_empty() {
            return Err(Failure::Unsupported);
        }
    }
    Ok(())
}
fn cell_properties(node: Node<'_, '_>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for property in children(node)? {
        let name = word(property)?;
        if !seen.insert(name) {
            return Err(Failure::Invalid);
        }
        if !matches!(
            name,
            "tcW" | "tcBorders" | "shd" | "tcMar" | "vAlign" | "noWrap" | "hideMark"
        ) {
            return Err(Failure::Unsupported);
        }
        metadata(property)?;
    }
    Ok(())
}

fn paragraph_properties(node: Node<'_, '_>, mut p: Paragraph) -> Result<Paragraph> {
    let mut seen = BTreeSet::new();
    for n in children(node)? {
        let name = word(n)?;
        if !seen.insert(name) {
            return Err(Failure::Invalid);
        }
        match name {
            "outlineLvl" => {
                let level = number(n)?;
                if level > 9 {
                    return Err(Failure::Invalid);
                }
                if (6..9).contains(&level) {
                    return Err(Failure::Unsupported);
                }
                p.outline = (level != 9).then_some(level);
            }
            "numPr" => {
                let id = child(n, "numId")?
                    .map(number)
                    .transpose()?
                    .or(p.list.map(|v| v.0))
                    .ok_or(Failure::Invalid)?;
                let level = child(n, "ilvl")?
                    .map(number)
                    .transpose()?
                    .unwrap_or(p.list.map_or(0, |v| v.1));
                if level > 8 {
                    return Err(Failure::Unsupported);
                }
                for part in children(n)? {
                    if !matches!(word(part)?, "numId" | "ilvl") {
                        return Err(Failure::Unsupported);
                    }
                }
                p.list = (id != 0).then_some((id, level));
            }
            "pStyle" => {
                if !children(n)?.is_empty() {
                    return Err(Failure::Unsupported);
                }
            }
            "rPr" => {
                run_properties(n, Emphasis::default(), false)?;
            }
            "keepNext" | "keepLines" | "pageBreakBefore" | "widowControl" | "spacing" | "ind"
            | "jc" | "tabs" | "pBdr" | "shd" | "contextualSpacing" | "snapToGrid" => metadata(n)?,
            _ => return Err(Failure::Unsupported),
        }
    }
    Ok(p)
}

struct Styles<'a, 'i> {
    document: Option<&'a Document<'i>>,
    by_id: BTreeMap<&'a str, Node<'a, 'i>>,
    paragraph_default: Option<&'a str>,
    character_default: Option<&'a str>,
    table_default: Option<&'a str>,
    cancel: &'a AtomicBool,
}
impl<'a, 'i> Styles<'a, 'i> {
    fn new(document: Option<&'a Document<'i>>, cancel: &'a AtomicBool) -> Result<Self> {
        let mut styles = Self {
            document,
            by_id: BTreeMap::new(),
            paragraph_default: None,
            character_default: None,
            table_default: None,
            cancel,
        };
        if let Some(doc) = document {
            if word(doc.root_element())? != "styles" {
                return Err(Failure::Invalid);
            }
            for n in children(doc.root_element())? {
                check_cancel(cancel)?;
                match word(n)? {
                    "style" => {
                        let id = attr(n, "styleId").ok_or(Failure::Invalid)?;
                        if styles.by_id.insert(id, n).is_some() {
                            return Err(Failure::Invalid);
                        }
                        if matches!(attr(n, "default"), Some("1" | "true" | "on")) {
                            let default = match attr(n, "type") {
                                Some("paragraph") => &mut styles.paragraph_default,
                                Some("character") => &mut styles.character_default,
                                Some("table") => &mut styles.table_default,
                                _ => continue,
                            };
                            if default.replace(id).is_some() {
                                return Err(Failure::Invalid);
                            }
                        }
                    }
                    "docDefaults" => {
                        for part in children(n)? {
                            let expected = match word(part)? {
                                "rPrDefault" => "rPr",
                                "pPrDefault" => "pPr",
                                _ => return Err(Failure::Unsupported),
                            };
                            child(part, expected)?;
                            if children(part)?
                                .iter()
                                .any(|c| word(*c).ok() != Some(expected))
                            {
                                return Err(Failure::Unsupported);
                            }
                        }
                    }
                    "latentStyles" => {}
                    _ => return Err(Failure::Unsupported),
                }
            }
        }
        Ok(styles)
    }
    fn defaults(&self) -> Result<Emphasis> {
        let Some(doc) = self.document else {
            return Ok(Emphasis::default());
        };
        if word(doc.root_element())? != "styles" {
            return Err(Failure::Invalid);
        }
        let Some(defaults) = child(doc.root_element(), "docDefaults")? else {
            return Ok(Emphasis::default());
        };
        let Some(r) = child(defaults, "rPrDefault")? else {
            return Ok(Emphasis::default());
        };
        match child(r, "rPr")? {
            Some(n) => run_properties(n, Emphasis::default(), false),
            None => Ok(Emphasis::default()),
        }
    }
    fn chain(&self, id: &str, expected: &str) -> Result<Vec<Node<'a, 'i>>> {
        let mut chain = Vec::new();
        let mut seen = BTreeSet::new();
        let mut next = id.to_owned();
        loop {
            check_cancel(self.cancel)?;
            if !seen.insert(next.clone()) {
                return Err(Failure::Invalid);
            }
            if chain.len() >= 64 {
                return Err(Failure::Limit);
            }
            let style = self.by_id.get(next.as_str()).ok_or(Failure::Invalid)?;
            if attr(*style, "type") != Some(expected) {
                return Err(Failure::Unsupported);
            }
            for n in children(*style)? {
                if !matches!(
                    word(n)?,
                    "name"
                        | "basedOn"
                        | "next"
                        | "link"
                        | "uiPriority"
                        | "qFormat"
                        | "semiHidden"
                        | "unhideWhenUsed"
                        | "locked"
                        | "personal"
                        | "personalCompose"
                        | "personalReply"
                        | "rsid"
                        | "pPr"
                        | "rPr"
                        | "tblPr"
                        | "trPr"
                        | "tcPr"
                ) {
                    return Err(Failure::Unsupported);
                }
                if matches!(word(n)?, "tblPr" | "trPr" | "tcPr") && expected != "table" {
                    return Err(Failure::Unsupported);
                }
                if !matches!(word(n)?, "pPr" | "rPr" | "tblPr" | "trPr" | "tcPr")
                    && !children(n)?.is_empty()
                {
                    return Err(Failure::Unsupported);
                }
            }
            chain.push(*style);
            match child(*style, "basedOn")? {
                Some(base) => next = val(base)?.to_owned(),
                None => break,
            }
        }
        chain.reverse();
        Ok(chain)
    }
    fn paragraph(&self, properties: Option<Node<'_, '_>>) -> Result<(Paragraph, Emphasis)> {
        let mut p = Paragraph::default();
        let mut e = self.defaults()?;
        if let Some(doc) = self.document {
            if let Some(defaults) = child(doc.root_element(), "docDefaults")? {
                if let Some(default) = child(defaults, "pPrDefault")? {
                    if let Some(n) = child(default, "pPr")? {
                        p = paragraph_properties(n, p)?;
                    }
                }
            }
        }
        let explicit = properties
            .map(|n| child(n, "pStyle"))
            .transpose()?
            .flatten()
            .map(val)
            .transpose()?;
        let default = self.paragraph_default;
        if let Some(id) = explicit.or(default) {
            for style in self.chain(id, "paragraph")? {
                if let Some(n) = child(style, "pPr")? {
                    p = paragraph_properties(n, p)?;
                }
                if let Some(n) = child(style, "rPr")? {
                    e = run_properties(n, e, true)?;
                }
            }
        }
        if let Some(n) = properties {
            p = paragraph_properties(n, p)?;
        }
        Ok((p, e))
    }
    fn table(&self, id: Option<&str>) -> Result<()> {
        let Some(id) = id.or(self.table_default) else {
            return Ok(());
        };
        for style in self.chain(id, "table")? {
            for n in children(style)? {
                match word(n)? {
                    "rPr" => layout_run_properties(n)?,
                    "pPr" => {
                        if child(n, "pStyle")?.is_some()
                            || child(n, "outlineLvl")?.is_some()
                            || child(n, "numPr")?.is_some()
                        {
                            return Err(Failure::Unsupported);
                        }
                        paragraph_properties(n, Paragraph::default())?;
                        if let Some(r) = child(n, "rPr")? {
                            layout_run_properties(r)?;
                        }
                    }
                    "tblPr" => table_properties(n, false)?,
                    "trPr" => {
                        for property in children(n)? {
                            if !matches!(word(property)?, "cantSplit" | "trHeight" | "jc") {
                                return Err(Failure::Unsupported);
                            }
                            metadata(property)?;
                        }
                    }
                    "tcPr" => cell_properties(n)?,
                    _ => {} // chain() has already checked the remaining style metadata.
                }
            }
        }
        Ok(())
    }
    fn run(&self, node: Option<Node<'_, '_>>, mut e: Emphasis) -> Result<Emphasis> {
        let explicit = node
            .map(|n| child(n, "rStyle"))
            .transpose()?
            .flatten()
            .map(val)
            .transpose()?;
        if let Some(id) = explicit.or(self.character_default) {
            for s in self.chain(id, "character")? {
                if child(s, "pPr")?.is_some() {
                    return Err(Failure::Unsupported);
                }
                if let Some(r) = child(s, "rPr")? {
                    e = run_properties(r, e, true)?;
                }
            }
        }
        if let Some(n) = node {
            e = run_properties(n, e, false)?;
        }
        Ok(e)
    }
}

#[derive(Clone)]
struct Level {
    start: u32,
    decimal: bool,
    delimiter: char,
    restart: u32,
}
struct Numbering<'a, 'i> {
    nums: BTreeMap<u32, Node<'a, 'i>>,
    abstracts: BTreeMap<u32, Node<'a, 'i>>,
    counters: BTreeMap<u32, [Option<u32>; 9]>,
    cancel: &'a AtomicBool,
}
impl<'a, 'i> Numbering<'a, 'i> {
    fn new(document: Option<&'a Document<'i>>, cancel: &'a AtomicBool) -> Result<Self> {
        let mut numbering = Self {
            nums: BTreeMap::new(),
            abstracts: BTreeMap::new(),
            counters: BTreeMap::new(),
            cancel,
        };
        if let Some(doc) = document {
            if word(doc.root_element())? != "numbering" {
                return Err(Failure::Invalid);
            }
            for n in children(doc.root_element())? {
                check_cancel(cancel)?;
                let (map, key) = match word(n)? {
                    "num" => (&mut numbering.nums, "numId"),
                    "abstractNum" => (&mut numbering.abstracts, "abstractNumId"),
                    "numIdMacAtCleanup" => continue,
                    _ => return Err(Failure::Unsupported),
                };
                let id = attr(n, key)
                    .ok_or(Failure::Invalid)?
                    .parse::<u32>()
                    .map_err(|_| Failure::Invalid)?;
                if map.insert(id, n).is_some() {
                    return Err(Failure::Invalid);
                }
            }
        }
        Ok(numbering)
    }
    fn level(&self, id: u32, index: u32) -> Result<Level> {
        check_cancel(self.cancel)?;
        let num = *self.nums.get(&id).ok_or(Failure::Invalid)?;
        let abstract_id = child(num, "abstractNumId")?
            .ok_or(Failure::Invalid)
            .and_then(number)?;
        let abstract_num = *self.abstracts.get(&abstract_id).ok_or(Failure::Invalid)?;
        let mut selected = None;
        for n in children(abstract_num)? {
            match word(n)? {
                "lvl" if attr(n, "ilvl").and_then(|x| x.parse::<u32>().ok()) == Some(index) => {
                    if selected.replace(n).is_some() {
                        return Err(Failure::Invalid);
                    }
                }
                "lvl" | "nsid" | "multiLevelType" | "tmpl" | "name" => {}
                _ => return Err(Failure::Unsupported),
            }
        }
        let mut start_override = None;
        let mut overridden = BTreeSet::new();
        let mut level_override = false;
        for n in children(num)? {
            match word(n)? {
                "abstractNumId" => {}
                "lvlOverride" => {
                    let level = attr(n, "ilvl")
                        .ok_or(Failure::Invalid)?
                        .parse::<u32>()
                        .map_err(|_| Failure::Invalid)?;
                    if !overridden.insert(level) {
                        return Err(Failure::Invalid);
                    }
                    if level == index {
                        for part in children(n)? {
                            match word(part)? {
                                "startOverride" => {
                                    if start_override.replace(number(part)?).is_some() {
                                        return Err(Failure::Invalid);
                                    }
                                }
                                "lvl" => {
                                    if level_override {
                                        return Err(Failure::Invalid);
                                    }
                                    level_override = true;
                                    if attr(part, "ilvl").and_then(|v| v.parse::<u32>().ok())
                                        != Some(index)
                                    {
                                        return Err(Failure::Invalid);
                                    }
                                    selected = Some(part);
                                }
                                _ => return Err(Failure::Unsupported),
                            }
                        }
                    }
                }
                _ => return Err(Failure::Unsupported),
            }
        }
        let node = selected.ok_or(Failure::Invalid)?;
        let mut start = 1;
        let mut decimal = None;
        let mut pattern = None;
        let mut restart = index;
        let mut seen = BTreeSet::new();
        for n in children(node)? {
            let name = word(n)?;
            if !seen.insert(name) {
                return Err(Failure::Invalid);
            }
            match name {
                "start" => start = number(n)?,
                "numFmt" => {
                    decimal = Some(match val(n)? {
                        "decimal" => true,
                        "bullet" => false,
                        _ => return Err(Failure::Unsupported),
                    })
                }
                "lvlText" => pattern = Some(val(n)?.to_owned()),
                "lvlRestart" => {
                    restart = number(n)?;
                    if restart > index {
                        return Err(Failure::Invalid);
                    }
                }
                "suff" => {
                    if !matches!(val(n)?, "tab" | "space") {
                        return Err(Failure::Unsupported);
                    }
                }
                "lvlJc" | "pPr" => metadata(n)?,
                "rPr" => {
                    let e = run_properties(n, Emphasis::default(), false)?;
                    if e.bold || e.italic || e.underline || e.strike || e.vertical != 0 {
                        return Err(Failure::Unsupported);
                    }
                }
                _ => return Err(Failure::Unsupported),
            }
        }
        start = start_override.unwrap_or(start);
        let decimal = decimal.ok_or(Failure::Invalid)?;
        let pattern = pattern.ok_or(Failure::Invalid)?;
        let delimiter = if decimal && pattern == format!("%{})", index + 1) {
            ')'
        } else {
            '.'
        };
        if decimal {
            if pattern != format!("%{}{delimiter}", index + 1) {
                return Err(Failure::Unsupported);
            }
        } else if !matches!(
            pattern.as_str(),
            "•" | "◦" | "▪" | "●" | "○" | "■" | "\u{f0b7}" | "\u{f0a7}"
        ) {
            return Err(Failure::Unsupported);
        }
        Ok(Level {
            start,
            decimal,
            delimiter,
            restart,
        })
    }
    fn prefix(&mut self, id: u32, index: u32) -> Result<String> {
        let level = self.level(id, index)?;
        let mut restart_levels = Vec::new();
        for i in index + 1..9 {
            if self
                .counters
                .get(&id)
                .is_none_or(|c| c[i as usize].is_none())
            {
                continue;
            }
            match self.level(id, i) {
                Ok(l) if l.restart != 0 && index + 1 == l.restart => restart_levels.push(i),
                Ok(_) | Err(Failure::Invalid) => {}
                Err(e) => return Err(e),
            }
        }
        let counters = self.counters.entry(id).or_insert([None; 9]);
        for i in restart_levels {
            counters[i as usize] = None;
        }
        let slot = &mut counters[index as usize];
        let current = match *slot {
            None => level.start,
            Some(n) => n.checked_add(1).ok_or(Failure::Limit)?,
        };
        if current > 999_999_999 {
            return Err(Failure::Unsupported);
        }
        *slot = Some(current);
        let active = *counters;
        let mut indentation = 0usize;
        for ancestor in 0..index {
            let parent = self.level(id, ancestor)?;
            let count = active[ancestor as usize].ok_or(Failure::Unsupported)?;
            let width = if parent.decimal {
                count.to_string().len() + 2
            } else {
                2
            };
            indentation = indentation
                .checked_add(width.max(4))
                .ok_or(Failure::Limit)?;
        }
        let indent = " ".repeat(indentation);
        // Supported glyphs are ordinary unordered-list markers, not literal body wording.
        Ok(if level.decimal {
            format!("{indent}{current}{} ", level.delimiter)
        } else {
            format!("{indent}- ")
        })
    }
}

struct Renderer<'a, 'i> {
    styles: Styles<'a, 'i>,
    numbering: Numbering<'a, 'i>,
    links: &'a BTreeMap<String, String>,
    cancel: &'a AtomicBool,
    last_list: Option<(u32, u32)>,
}
impl Renderer<'_, '_> {
    fn run(&self, node: Node<'_, '_>, inherited: Emphasis) -> Result<String> {
        let e = self.styles.run(child(node, "rPr")?, inherited)?;
        let mut text = String::new();
        for n in children(node)? {
            check_cancel(self.cancel)?;
            if !matches!(word(n)?, "rPr" | "t") && !children(n)?.is_empty() {
                return Err(Failure::Unsupported);
            }
            match word(n)? {
                "rPr" => {}
                "t" => {
                    if n.children().any(|c| c.is_element()) {
                        return Err(Failure::Invalid);
                    }
                    for wording in n.children().filter(Node::is_text) {
                        push(
                            &mut text,
                            &escaped(wording.text().unwrap_or_default(), self.cancel)?,
                        )?;
                    }
                }
                "tab" => push(&mut text, "\t")?,
                "br" => {
                    if !matches!(attr(n, "type"), None | Some("textWrapping"))
                        || attr(n, "clear").is_some_and(|v| v != "none")
                    {
                        return Err(Failure::Unsupported);
                    }
                    push(&mut text, "<br>")?;
                }
                "cr" => push(&mut text, "<br>")?,
                "noBreakHyphen" => push(&mut text, "‑")?,
                "softHyphen" => push(&mut text, "\u{ad}")?,
                "lastRenderedPageBreak" => {}
                _ => return Err(Failure::Unsupported),
            }
        }
        if text.is_empty() {
            return Ok(text);
        }
        let mut out = String::new();
        for (on, tag) in [
            (e.bold, "<strong>"),
            (e.italic, "<em>"),
            (e.underline, "<u>"),
            (e.strike, "<del>"),
            (e.vertical == 1, "<sup>"),
            (e.vertical == -1, "<sub>"),
        ] {
            if on {
                push(&mut out, tag)?;
            }
        }
        push(&mut out, &text)?;
        for (on, tag) in [
            (e.vertical == -1, "</sub>"),
            (e.vertical == 1, "</sup>"),
            (e.strike, "</del>"),
            (e.underline, "</u>"),
            (e.italic, "</em>"),
            (e.bold, "</strong>"),
        ] {
            if on {
                push(&mut out, tag)?;
            }
        }
        Ok(out)
    }
    fn inline(&self, node: Node<'_, '_>, e: Emphasis, hyperlink: bool) -> Result<String> {
        let mut out = String::new();
        for n in children(node)? {
            check_cancel(self.cancel)?;
            match word(n)? {
                "pPr" if !hyperlink => {}
                "r" => push(&mut out, &self.run(n, e)?)?,
                "hyperlink" if !hyperlink => {
                    if attr(n, "anchor").is_some() || attr(n, "docLocation").is_some() {
                        return Err(Failure::Unsupported);
                    }
                    let id = n
                        .attribute((R, "id"))
                        .or_else(|| n.attribute((RS, "id")))
                        .ok_or(Failure::Invalid)?;
                    let destination = self.links.get(id).ok_or(Failure::Invalid)?;
                    let label = self.inline(n, e, true)?;
                    push(&mut out, "[")?;
                    push(&mut out, &label)?;
                    push(&mut out, "](<")?;
                    for byte in destination.bytes() {
                        if byte == b'&' {
                            push(&mut out, "&amp;")?;
                        } else if byte <= 0x20
                            || byte >= 0x7f
                            || matches!(byte, b'<' | b'>' | b'"' | b'\\')
                        {
                            push(&mut out, &format!("%{byte:02X}"))?;
                        } else {
                            push(&mut out, (byte as char).encode_utf8(&mut [0; 4]))?;
                        }
                    }
                    push(&mut out, ">")?;
                    if let Some(tooltip) = attr(n, "tooltip") {
                        tooltip_title(&mut out, tooltip, self.cancel)?;
                    }
                    push(&mut out, ")")?;
                }
                "bookmarkStart" | "bookmarkEnd" | "proofErr" => {
                    if !children(n)?.is_empty() {
                        return Err(Failure::Unsupported);
                    }
                }
                _ => return Err(Failure::Unsupported),
            }
        }
        Ok(out)
    }
    fn paragraph(&mut self, node: Node<'_, '_>, table: bool) -> Result<String> {
        let (p, e) = self.styles.paragraph(child(node, "pPr")?)?;
        if table && (p.outline.is_some() || p.list.is_some()) {
            return Err(Failure::Unsupported);
        }
        if p.outline.is_some() && p.list.is_some() {
            return Err(Failure::Unsupported);
        }
        let mut out = String::new();
        if let Some(level) = p.outline {
            push(&mut out, &"#".repeat(level as usize + 1))?;
            push(&mut out, " ")?;
        }
        if let Some((id, level)) = p.list {
            if self
                .last_list
                .is_some_and(|(previous_id, _)| previous_id != id)
            {
                push(&mut out, "<!-- separate numbering instance -->\n\n")?;
            }
            if level > 0
                && self.last_list.is_none_or(|(previous_id, previous_level)| {
                    previous_id != id || level > previous_level + 1
                })
            {
                return Err(Failure::Unsupported);
            }
            push(&mut out, &self.numbering.prefix(id, level)?)?;
        }
        self.last_list = p.list;
        push(&mut out, &self.inline(node, e, false)?)?;
        Ok(out)
    }
    fn table(&mut self, table: Node<'_, '_>) -> Result<String> {
        self.last_list = None;
        let properties = child(table, "tblPr")?;
        let style = properties
            .map(|n| child(n, "tblStyle"))
            .transpose()?
            .flatten()
            .map(val)
            .transpose()?;
        self.styles.table(style)?;
        let mut accumulated = 0usize;
        let mut rows = Vec::new();
        let mut header = false;
        let mut columns = None;
        let mut seen_props = false;
        let mut grid = None;
        for n in children(table)? {
            check_cancel(self.cancel)?;
            match word(n)? {
                "tblPr" => {
                    if seen_props || !rows.is_empty() {
                        return Err(Failure::Invalid);
                    }
                    seen_props = true;
                    table_properties(n, true)?;
                }
                "tblGrid" => {
                    if grid.is_some() || !rows.is_empty() {
                        return Err(Failure::Invalid);
                    }
                    let parts = children(n)?;
                    if parts.iter().any(|c| word(*c).ok() != Some("gridCol")) {
                        return Err(Failure::Unsupported);
                    }
                    for column in &parts {
                        metadata(*column)?;
                    }
                    grid = Some(parts.len());
                }
                "tr" => {
                    child(n, "trPr")?;
                    let mut row = Vec::new();
                    let mut row_header = false;
                    for cell in children(n)? {
                        match word(cell)? {
                            "trPr" => {
                                if !row.is_empty() {
                                    return Err(Failure::Invalid);
                                }
                                for property in children(cell)? {
                                    match word(property)? {
                                        "tblHeader" => row_header = boolean(property)?,
                                        "cantSplit" | "trHeight" | "jc" => metadata(property)?,
                                        _ => return Err(Failure::Unsupported),
                                    }
                                }
                            }
                            "tc" => {
                                child(cell, "tcPr")?;
                                let mut value = String::new();
                                let mut count = 0;
                                for part in children(cell)? {
                                    match word(part)? {
                                        "tcPr" => {
                                            if count > 0 {
                                                return Err(Failure::Invalid);
                                            }
                                            cell_properties(part)?;
                                        }
                                        "p" => {
                                            if count > 0 {
                                                push(&mut value, "<br><br>")?;
                                            }
                                            push(&mut value, &self.paragraph(part, true)?)?;
                                            count += 1;
                                        }
                                        _ => return Err(Failure::Unsupported),
                                    }
                                }
                                if count == 0 {
                                    return Err(Failure::Invalid);
                                }
                                accumulated = accumulated
                                    .checked_add(value.len() + 3)
                                    .ok_or(Failure::Limit)?;
                                if accumulated > LIMIT {
                                    return Err(Failure::Limit);
                                }
                                row.push(value);
                            }
                            _ => return Err(Failure::Unsupported),
                        }
                    }
                    if row.is_empty() || columns.is_some_and(|c| c != row.len()) {
                        return Err(Failure::Invalid);
                    }
                    if row_header {
                        if !rows.is_empty() {
                            return Err(Failure::Unsupported);
                        }
                        header = true;
                    }
                    columns = Some(row.len());
                    rows.push(row);
                }
                _ => return Err(Failure::Unsupported),
            }
        }
        let columns = columns.ok_or(Failure::Invalid)?;
        if grid.is_some_and(|n| n != columns) {
            return Err(Failure::Invalid);
        }
        let mut out = String::new();
        let emit = |out: &mut String, row: &[String]| -> Result<()> {
            push(out, "|")?;
            for cell in row {
                push(out, " ")?;
                push(out, cell)?;
                push(out, " |")?;
            }
            push(out, "\n")
        };
        let empty_header = vec![String::new(); columns];
        let first = if header { &rows[0] } else { &empty_header };
        emit(&mut out, first)?;
        emit(&mut out, &vec!["---".into(); columns])?;
        for row in rows.iter().skip(usize::from(header)) {
            emit(&mut out, row)?;
        }
        out.pop();
        Ok(out)
    }
}

pub(super) fn render(
    document: &Document<'_>,
    styles: Option<&Document<'_>>,
    numbering: Option<&Document<'_>>,
    links: &BTreeMap<String, String>,
    cancel: &AtomicBool,
) -> Result<String> {
    check_cancel(cancel)?;
    let root = document.root_element();
    if word(root)? != "document" {
        return Err(Failure::Invalid);
    }
    let mut body = None;
    for n in children(root)? {
        if word(n)? != "body" {
            return Err(Failure::Unsupported);
        }
        if body.replace(n).is_some() {
            return Err(Failure::Invalid);
        }
    }
    let body = body.ok_or(Failure::Invalid)?;
    let mut renderer = Renderer {
        styles: Styles::new(styles, cancel)?,
        numbering: Numbering::new(numbering, cancel)?,
        links,
        cancel,
        last_list: None,
    };
    let mut out = String::new();
    let mut blocks = 0;
    let mut section = false;
    for n in children(body)? {
        check_cancel(cancel)?;
        if section {
            return Err(Failure::Invalid);
        }
        let block = match word(n)? {
            "p" => renderer.paragraph(n, false)?,
            "tbl" => renderer.table(n)?,
            "sectPr" => {
                section = true;
                for property in children(n)? {
                    if !matches!(
                        word(property)?,
                        "pgSz"
                            | "pgMar"
                            | "cols"
                            | "docGrid"
                            | "type"
                            | "titlePg"
                            | "textDirection"
                            | "vAlign"
                            | "bidi"
                            | "rtlGutter"
                            | "pgBorders"
                            | "paperSrc"
                    ) {
                        return Err(Failure::Unsupported);
                    }
                    metadata(property)?;
                    if matches!(word(property)?, "textDirection" | "bidi") {
                        return Err(Failure::Unsupported);
                    }
                }
                continue;
            }
            _ => return Err(Failure::Unsupported),
        };
        if blocks > 0 {
            push(&mut out, "\n\n")?;
        }
        push(&mut out, &block)?;
        blocks += 1;
    }
    if blocks > 0 {
        push(&mut out, "\n")?;
    }
    check_cancel(cancel)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    fn convert(body: &str, styles: Option<&str>, numbering: Option<&str>) -> Result<String> {
        let xml = format!(
            r#"<w:document xmlns:w="{W}" xmlns:r="{R}"><w:body>{body}</w:body></w:document>"#
        );
        let document = Document::parse(&xml).unwrap();
        let styles = styles.map(|s| Document::parse(s).unwrap());
        let numbering = numbering.map(|s| Document::parse(s).unwrap());
        let links = BTreeMap::from([(
            "link".into(),
            "https://example.invalid/a b?q=õ&x=<value>".into(),
        )]);
        render(
            &document,
            styles.as_ref(),
            numbering.as_ref(),
            &links,
            &AtomicBool::new(false),
        )
    }
    fn p(text: &str) -> String {
        format!("<w:p><w:r><w:t>{text}</w:t></w:r></w:p>")
    }
    fn numbered(id: u32, level: u32, text: &str) -> String {
        format!(
            "<w:p><w:pPr><w:numPr><w:ilvl w:val=\"{level}\"/><w:numId w:val=\"{id}\"/></w:numPr></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"
        )
    }
    fn numbering() -> String {
        format!(
            r#"<w:numbering xmlns:w="{W}"><w:abstractNum w:abstractNumId="0"><w:multiLevelType w:val="multilevel"/><w:lvl w:ilvl="0"><w:start w:val="2"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/></w:lvl><w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%2."/></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/><w:lvlOverride w:ilvl="0"><w:startOverride w:val="4"/></w:lvlOverride></w:num><w:num w:numId="2"><w:abstractNumId w:val="0"/></w:num></w:numbering>"#
        )
    }

    #[test]
    fn golden_unicode_heading_emphasis_external_link_and_table_preserve_order() {
        let styles = format!(
            r#"<w:styles xmlns:w="{W}"><w:style w:type="paragraph" w:styleId="base"><w:pPr><w:outlineLvl w:val="1"/></w:pPr><w:rPr><w:b/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="heading"><w:basedOn w:val="base"/></w:style></w:styles>"#
        );
        let body = concat!(
            "<w:p><w:pPr><w:pStyle w:val=\"heading\"/></w:pPr><w:r><w:t>Heading õ 日本語</w:t></w:r></w:p>",
            "<w:p><w:r><w:rPr><w:b/><w:i/><w:u w:val=\"single\"/><w:strike/><w:vertAlign w:val=\"subscript\"/></w:rPr><w:t> [exact] </w:t></w:r><w:r><w:rPr><w:vertAlign w:val=\"superscript\"/></w:rPr><w:t>2</w:t></w:r><w:hyperlink r:id=\"link\"><w:r><w:t>Link</w:t></w:r></w:hyperlink></w:p>",
            "<w:tbl><w:tblGrid><w:gridCol/><w:gridCol/></w:tblGrid><w:tr><w:trPr><w:tblHeader/></w:trPr><w:tc><w:p><w:r><w:t>H1</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>H2</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc><w:p><w:r><w:t>a|b</w:t></w:r></w:p><w:p><w:r><w:t>second</w:t></w:r></w:p></w:tc><w:tc><w:p/></w:tc></w:tr></w:tbl>",
            "<w:p/>",
            "<w:p><w:r><w:t>Final</w:t><w:tab/><w:t>word</w:t><w:br/><w:t>&lt;untrusted&gt;</w:t></w:r></w:p>"
        );
        assert_eq!(
            convert(body, Some(&styles), None).unwrap(),
            concat!(
                "## <strong>Heading õ 日本語</strong>\n\n",
                "<strong><em><u><del><sub> \\[exact\\] </sub></del></u></em></strong><sup>2</sup>[Link](<https://example.invalid/a%20b?q=%C3%B5&amp;x=%3Cvalue%3E>)\n\n",
                "| H1 | H2 |\n| --- | --- |\n| a\\|b<br><br>second |  |\n\n\n\n",
                "Final\tword<br>&lt;untrusted&gt;\n"
            )
        );
    }
    #[test]
    fn plain_empty_and_strict_documents_have_no_invented_wording() {
        assert_eq!(convert("", None, None).unwrap(), "");
        assert_eq!(convert("<w:p/>", None, None).unwrap(), "\n");
        assert_eq!(convert("<w:p><w:r><w:t>First<!-- metadata -->last<![CDATA[ & literal ]]></w:t></w:r></w:p>", None, None).unwrap(), "Firstlast &amp; literal \n");
        assert_eq!(
            convert(&(p("First õ 日本語") + &p("Second preserved")), None, None).unwrap(),
            "First õ 日本語\n\nSecond preserved\n"
        );
        let xml = format!(
            r#"<w:document xmlns:w="{WS}"><w:body>{}</w:body></w:document>"#,
            p("Strict")
        );
        assert_eq!(
            render(
                &Document::parse(&xml).unwrap(),
                None,
                None,
                &BTreeMap::new(),
                &AtomicBool::new(false)
            )
            .unwrap(),
            "Strict\n"
        );
        assert_eq!(
            convert(&p("# [x] *bold* 1. &amp; &lt;tag&gt;"), None, None).unwrap(),
            "\\# \\[x\\] \\*bold\\* 1\\. &amp; &lt;tag&gt;\n"
        );
    }
    #[test]
    fn headerless_table_does_not_promote_original_first_row() {
        let body =
            "<w:tbl><w:tr><w:tc><w:p><w:r><w:t>ordinary</w:t></w:r></w:p></w:tc></w:tr></w:tbl>";
        assert_eq!(
            convert(body, None, None).unwrap(),
            "|  |\n| --- |\n| ordinary |\n"
        );
    }
    #[test]
    fn numbering_keeps_starts_independent_counters_nesting_and_restart() {
        let body = numbered(1, 0, "A")
            + &numbered(1, 1, "nested")
            + &numbered(1, 1, "next")
            + &numbered(1, 0, "B")
            + &numbered(1, 1, "restarted")
            + &numbered(2, 0, "Independent");
        assert_eq!(
            convert(&body, None, Some(&numbering())).unwrap(),
            "4. A\n\n    1. nested\n\n    2. next\n\n5. B\n\n    1. restarted\n\n<!-- separate numbering instance -->\n\n2. Independent\n"
        );
        let bullet = numbering().replace(
            "<w:numFmt w:val=\"decimal\"/><w:lvlText w:val=\"%2.\"/>",
            "<w:numFmt w:val=\"bullet\"/><w:lvlText w:val=\"•\"/>",
        );
        assert_eq!(
            convert(
                &(numbered(1, 0, "A") + &numbered(1, 1, "Bullet")),
                None,
                Some(&bullet)
            )
            .unwrap(),
            "4. A\n\n    - Bullet\n"
        );
        assert!(matches!(
            convert(&numbered(1, 1, "orphan"), None, Some(&numbering())),
            Err(Failure::Unsupported)
        ));
        assert!(matches!(
            convert(
                &numbered(1, 0, "roman"),
                None,
                Some(&numbering().replace("decimal", "upperRoman"))
            ),
            Err(Failure::Unsupported)
        ));
        assert!(matches!(
            convert(
                &numbered(1, 0, "compound"),
                None,
                Some(&numbering().replace("%1.", "%1.%2."))
            ),
            Err(Failure::Unsupported)
        ));
    }
    #[test]
    fn ordinary_parenthesis_decimal_markers_are_preserved() {
        let numbering = numbering().replace("%1.", "%1)").replace("%2.", "%2)");
        assert_eq!(
            convert(
                &(numbered(1, 0, "A") + &numbered(1, 1, "nested") + &numbered(1, 0, "B")),
                None,
                Some(&numbering)
            )
            .unwrap(),
            "4) A\n\n    1) nested\n\n5) B\n"
        );
    }
    #[test]
    fn common_table_grid_and_default_layout_style_preserve_simple_table() {
        let styles = format!(
            r#"<w:styles xmlns:w="{W}"><w:style w:type="table" w:default="1" w:styleId="TableNormal"><w:name w:val="Normal Table"/><w:tblPr><w:tblInd w:w="0" w:type="dxa"/><w:tblCellMar><w:top w:w="0"/><w:left w:w="108"/><w:bottom w:w="0"/><w:right w:w="108"/></w:tblCellMar></w:tblPr></w:style><w:style w:type="table" w:styleId="TableGrid"><w:name w:val="Table Grid"/><w:basedOn w:val="TableNormal"/><w:pPr><w:spacing w:after="0"/></w:pPr><w:tblPr><w:tblBorders><w:top w:val="single"/><w:left w:val="single"/><w:bottom w:val="single"/><w:right w:val="single"/><w:insideH w:val="single"/><w:insideV w:val="single"/></w:tblBorders></w:tblPr></w:style></w:styles>"#
        );
        let row = "<w:tr><w:tc><w:p><w:r><w:t>ordinary</w:t></w:r></w:p></w:tc></w:tr>";
        let expected = "|  |\n| --- |\n| ordinary |\n";
        assert_eq!(convert(&format!("<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/><w:tblW w:w=\"0\" w:type=\"auto\"/></w:tblPr>{row}</w:tbl>"), Some(&styles), None).unwrap(), expected);
        assert_eq!(
            convert(&format!("<w:tbl>{row}</w:tbl>"), Some(&styles), None).unwrap(),
            expected
        );
        let conditional = styles.replace("<w:name w:val=\"Table Grid\"/>", "<w:name w:val=\"Table Grid\"/><w:tblStylePr w:type=\"firstRow\"><w:rPr><w:b/></w:rPr></w:tblStylePr>");
        let body =
            format!("<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/></w:tblPr>{row}</w:tbl>");
        assert!(matches!(
            convert(&body, Some(&conditional), None),
            Err(Failure::Unsupported)
        ));
        let emphasized = styles.replace(
            "<w:name w:val=\"Table Grid\"/>",
            "<w:name w:val=\"Table Grid\"/><w:rPr><w:b/></w:rPr>",
        );
        assert!(matches!(
            convert(&body, Some(&emphasized), None),
            Err(Failure::Unsupported)
        ));
    }
    #[test]
    fn inherited_defaults_toggles_and_direct_off_are_distinct() {
        let styles = format!(
            r#"<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault><w:rPr><w:u w:val="single"/></w:rPr></w:rPrDefault></w:docDefaults><w:style w:type="paragraph" w:styleId="normal" w:default="1"><w:rPr><w:i/></w:rPr></w:style><w:style w:type="character" w:styleId="bold"><w:rPr><w:b/></w:rPr></w:style><w:style w:type="character" w:styleId="toggle"><w:basedOn w:val="bold"/><w:rPr><w:b/></w:rPr></w:style></w:styles>"#
        );
        let body = "<w:p><w:r><w:rPr><w:rStyle w:val=\"toggle\"/></w:rPr><w:t>Toggled</w:t></w:r><w:r><w:rPr><w:i w:val=\"0\"/><w:u w:val=\"none\"/><w:b/></w:rPr><w:t>Direct</w:t></w:r></w:p>";
        assert_eq!(
            convert(body, Some(&styles), None).unwrap(),
            "<em><u>Toggled</u></em><strong>Direct</strong>\n"
        );
        let cycle = format!(
            r#"<w:styles xmlns:w="{W}"><w:style w:type="paragraph" w:styleId="cycle"><w:basedOn w:val="cycle"/></w:style></w:styles>"#
        );
        assert!(matches!(
            convert(
                "<w:p><w:pPr><w:pStyle w:val=\"cycle\"/></w:pPr></w:p>",
                Some(&cycle),
                None
            ),
            Err(Failure::Invalid)
        ));
        let unsupported = styles.replace("<w:i/>", "<w:vanish/>");
        assert!(matches!(
            convert(&p("hidden"), Some(&unsupported), None),
            Err(Failure::Unsupported)
        ));
    }
    #[test]
    fn meaningful_unknown_content_is_never_flattened_or_dropped() {
        for tag in [
            "drawing",
            "pict",
            "object",
            "fldChar",
            "instrText",
            "footnoteReference",
            "endnoteReference",
            "commentReference",
            "sym",
        ] {
            let body = format!("<w:p><w:r><w:t>preserved</w:t><w:{tag}/></w:r></w:p>");
            assert!(
                matches!(convert(&body, None, None), Err(Failure::Unsupported)),
                "{tag}"
            );
        }
        for tag in [
            "ins",
            "del",
            "sdt",
            "customXml",
            "fldSimple",
            "moveFrom",
            "moveTo",
        ] {
            let body = format!("<w:p><w:{tag}><w:r><w:t>hidden</w:t></w:r></w:{tag}></w:p>");
            assert!(
                matches!(convert(&body, None, None), Err(Failure::Unsupported)),
                "{tag}"
            );
        }
        assert!(matches!(
            convert(
                "<w:p><w:pPr><w:spacing><w:drawing/></w:spacing></w:pPr></w:p>",
                None,
                None
            ),
            Err(Failure::Unsupported)
        ));
        assert!(matches!(
            convert(
                "<w:p><w:r><w:rPr><w:vanish/></w:rPr><w:t>Hidden</w:t></w:r></w:p>",
                None,
                None
            ),
            Err(Failure::Unsupported)
        ));
        assert!(matches!(
            convert("<w:sectPr><w:headerReference/></w:sectPr>", None, None),
            Err(Failure::Unsupported)
        ));
        assert!(matches!(
            convert(
                "<w:p><w:hyperlink r:id=\"missing\"><w:r><w:t>label</w:t></w:r></w:hyperlink></w:p>",
                None,
                None
            ),
            Err(Failure::Invalid)
        ));
    }
    #[test]
    fn malformed_merged_nested_and_multiple_header_tables_refuse() {
        for property in ["gridSpan", "vMerge", "hMerge"] {
            let body = format!(
                "<w:tbl><w:tr><w:tc><w:tcPr><w:{property} w:val=\"2\"/></w:tcPr><w:p/></w:tc></w:tr></w:tbl>"
            );
            assert!(matches!(
                convert(&body, None, None),
                Err(Failure::Unsupported)
            ));
        }
        assert!(matches!(
            convert(
                "<w:tbl><w:tr><w:tc><w:tbl/></w:tc></w:tr></w:tbl>",
                None,
                None
            ),
            Err(Failure::Unsupported)
        ));
        assert!(matches!(
            convert(
                "<w:tbl><w:tr><w:tc><w:p/></w:tc></w:tr><w:tr><w:tc><w:p/></w:tc><w:tc><w:p/></w:tc></w:tr></w:tbl>",
                None,
                None
            ),
            Err(Failure::Invalid)
        ));
        assert!(matches!(
            convert(
                "<w:tbl><w:tr><w:trPr><w:tblHeader/></w:trPr><w:tc><w:p/></w:tc></w:tr><w:tr><w:trPr><w:tblHeader/></w:trPr><w:tc><w:p/></w:tc></w:tr></w:tbl>",
                None,
                None
            ),
            Err(Failure::Unsupported)
        ));
        assert!(matches!(
            convert("<w:p><w:pPr/><w:pPr/></w:p>", None, None),
            Err(Failure::Invalid)
        ));
        assert!(matches!(
            convert("<w:p>unwrapped wording</w:p>", None, None),
            Err(Failure::Invalid)
        ));
    }

    #[test]
    fn paint_is_neutral_or_refused_directly_and_through_used_styles() {
        let neutral = r#"<w:color w:val="auto"/><w:highlight w:val="none"/><w:shd w:val="clear" w:color="auto" w:fill="auto"/>"#;
        assert_eq!(
            convert(
                &format!("<w:p><w:r><w:rPr>{neutral}</w:rPr><w:t>Neutral</w:t></w:r></w:p>"),
                None,
                None
            )
            .unwrap(),
            "Neutral\n"
        );
        for paint in [
            r#"<w:color w:val="FF0000"/>"#,
            r#"<w:color w:val="auto" w:themeColor="accent1"/>"#,
            r#"<w:highlight w:val="yellow"/>"#,
            r#"<w:shd w:val="clear" w:fill="FFFF00"/>"#,
            r#"<w:shd w:val="solid" w:color="auto" w:fill="auto"/>"#,
            r#"<w:shd w:val="nil" w:themeFill="accent1"/>"#,
            r#"<w:highlight w:val="none"><w:drawing/></w:highlight>"#,
        ] {
            let direct = format!("<w:p><w:r><w:rPr>{paint}</w:rPr><w:t>Meaning</w:t></w:r></w:p>");
            assert!(
                matches!(convert(&direct, None, None), Err(Failure::Unsupported)),
                "direct {paint}"
            );
            let inherited = format!(
                r#"<w:styles xmlns:w="{W}"><w:style w:type="paragraph" w:styleId="base"><w:rPr>{paint}</w:rPr></w:style><w:style w:type="paragraph" w:styleId="derived" w:default="1"><w:basedOn w:val="base"/></w:style></w:styles>"#
            );
            assert!(
                matches!(
                    convert(&p("Inherited"), Some(&inherited), None),
                    Err(Failure::Unsupported)
                ),
                "inherited {paint}"
            );
            let character = format!(
                r#"<w:styles xmlns:w="{W}"><w:style w:type="character" w:styleId="base"><w:rPr>{paint}</w:rPr></w:style><w:style w:type="character" w:styleId="derived"><w:basedOn w:val="base"/></w:style></w:styles>"#
            );
            let styled_run = "<w:p><w:r><w:rPr><w:rStyle w:val=\"derived\"/></w:rPr><w:t>Styled</w:t></w:r></w:p>";
            assert!(
                matches!(
                    convert(styled_run, Some(&character), None),
                    Err(Failure::Unsupported)
                ),
                "character {paint}"
            );
            let defaults = format!(
                r#"<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault><w:rPr>{paint}</w:rPr></w:rPrDefault></w:docDefaults></w:styles>"#
            );
            assert!(
                matches!(
                    convert(&p("Default"), Some(&defaults), None),
                    Err(Failure::Unsupported)
                ),
                "default {paint}"
            );
            let numbered_styles = numbering().replace(
                "<w:lvlText w:val=\"%1.\"/>",
                &format!("<w:lvlText w:val=\"%1.\"/><w:rPr>{paint}</w:rPr>"),
            );
            assert!(
                matches!(
                    convert(&numbered(1, 0, "Numbered"), None, Some(&numbered_styles)),
                    Err(Failure::Unsupported)
                ),
                "numbering {paint}"
            );
        }
        let neutral_defaults = format!(
            r#"<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault><w:rPr>{neutral}</w:rPr></w:rPrDefault></w:docDefaults></w:styles>"#
        );
        assert_eq!(
            convert(&p("Default"), Some(&neutral_defaults), None).unwrap(),
            "Default\n"
        );
    }
    #[test]
    fn cell_paragraph_and_table_style_shading_cannot_silently_disappear() {
        for shading in [
            r#"<w:shd w:val="clear" w:fill="FFFF00"/>"#,
            r#"<w:shd w:val="nil" w:themeFill="accent2"/>"#,
            r#"<w:shd w:val="clear"><w:object/></w:shd>"#,
        ] {
            let cell = format!(
                "<w:tbl><w:tr><w:tc><w:tcPr>{shading}</w:tcPr>{}</w:tc></w:tr></w:tbl>",
                p("Cell")
            );
            assert!(matches!(
                convert(&cell, None, None),
                Err(Failure::Unsupported)
            ));
            let paragraph =
                format!("<w:p><w:pPr>{shading}</w:pPr><w:r><w:t>Paragraph</w:t></w:r></w:p>");
            assert!(matches!(
                convert(&paragraph, None, None),
                Err(Failure::Unsupported)
            ));
            let styles = format!(
                r#"<w:styles xmlns:w="{W}"><w:style w:type="table" w:styleId="TableNormal" w:default="1"><w:tcPr>{shading}</w:tcPr></w:style><w:style w:type="table" w:styleId="TableGrid"><w:basedOn w:val="TableNormal"/></w:style></w:styles>"#
            );
            let table = format!(
                "<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/></w:tblPr><w:tr><w:tc>{}</w:tc></w:tr></w:tbl>",
                p("Styled")
            );
            assert!(matches!(
                convert(&table, Some(&styles), None),
                Err(Failure::Unsupported)
            ));
            let table_default =
                format!("<w:tbl><w:tr><w:tc>{}</w:tc></w:tr></w:tbl>", p("Default"));
            assert!(matches!(
                convert(&table_default, Some(&styles), None),
                Err(Failure::Unsupported)
            ));
        }
        for neutral in [
            r#"<w:shd w:val="nil"/>"#,
            r#"<w:shd w:val="clear" w:color="auto" w:fill="auto"/>"#,
        ] {
            let cell = format!(
                "<w:tbl><w:tr><w:tc><w:tcPr>{neutral}</w:tcPr>{}</w:tc></w:tr></w:tbl>",
                p("Cell")
            );
            assert_eq!(
                convert(&cell, None, None).unwrap(),
                "|  |\n| --- |\n| Cell |\n"
            );
        }
        assert!(matches!(
            convert(
                "<w:p><w:pPr><w:pBdr><w:bottom w:color=\"FF0000\"/></w:pBdr></w:pPr></w:p>",
                None,
                None
            ),
            Err(Failure::Unsupported)
        ));
    }
    #[test]
    fn hyperlink_tooltip_is_preserved_without_title_or_markup_injection() {
        let body = r#"<w:p><w:hyperlink r:id="link" w:tooltip="Exact &quot;title&quot; \ &amp; &lt;tag&gt;&#10;next 日本語"><w:r><w:t>Link</w:t></w:r></w:hyperlink></w:p>"#;
        assert_eq!(
            convert(body, None, None).unwrap(),
            "[Link](<https://example.invalid/a%20b?q=%C3%B5&amp;x=%3Cvalue%3E> \"Exact &quot;title&quot; &#92; &amp; &lt;tag&gt;&#10;next 日本語\")\n"
        );
        let empty = r#"<w:p><w:hyperlink r:id="link" w:tooltip=""><w:r><w:t>Link</w:t></w:r></w:hyperlink></w:p>"#;
        assert_eq!(
            convert(empty, None, None).unwrap(),
            "[Link](<https://example.invalid/a%20b?q=%C3%B5&amp;x=%3Cvalue%3E>)\n"
        );
    }

    #[test]
    fn underline_paint_is_checked_directly_in_styles_and_defaults() {
        for paint in [
            r#"<w:u w:val="single" w:color="FF0000"/>"#,
            r#"<w:u w:val="single" w:themeColor="accent1"/>"#,
            r#"<w:u w:val="single" w:themeTint="80"/>"#,
            r#"<w:u w:val="single" w:themeShade="80"/>"#,
        ] {
            let direct = format!("<w:p><w:r><w:rPr>{paint}</w:rPr><w:t>Meaning</w:t></w:r></w:p>");
            assert!(matches!(
                convert(&direct, None, None),
                Err(Failure::Unsupported)
            ));
            let inherited = format!(
                r#"<w:styles xmlns:w="{W}"><w:style w:type="paragraph" w:styleId="base"><w:rPr>{paint}</w:rPr></w:style><w:style w:type="paragraph" w:styleId="derived" w:default="1"><w:basedOn w:val="base"/></w:style></w:styles>"#
            );
            assert!(matches!(
                convert(&p("Inherited"), Some(&inherited), None),
                Err(Failure::Unsupported)
            ));
            let defaults = format!(
                r#"<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault><w:rPr>{paint}</w:rPr></w:rPrDefault></w:docDefaults></w:styles>"#
            );
            assert!(matches!(
                convert(&p("Default"), Some(&defaults), None),
                Err(Failure::Unsupported)
            ));
        }
        for neutral in [r#"<w:u/>"#, r#"<w:u w:val="single" w:color="auto"/>"#] {
            let body = format!("<w:p><w:r><w:rPr>{neutral}</w:rPr><w:t>Neutral</w:t></w:r></w:p>");
            assert_eq!(convert(&body, None, None).unwrap(), "<u>Neutral</u>\n");
            let defaults = format!(
                r#"<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault><w:rPr>{neutral}</w:rPr></w:rPrDefault></w:docDefaults></w:styles>"#
            );
            assert_eq!(
                convert(&p("Default"), Some(&defaults), None).unwrap(),
                "<u>Default</u>\n"
            );
        }
    }
    #[test]
    fn wide_parent_and_deeper_decimal_markers_keep_nested_markdown_lists() {
        let numbering = format!(
            r#"<w:numbering xmlns:w="{W}"><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:start w:val="100"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/></w:lvl><w:lvl w:ilvl="1"><w:start w:val="1000"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%2)"/></w:lvl><w:lvl w:ilvl="2"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="•"/></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>"#
        );
        let body = numbered(1, 0, "Wide parent")
            + &numbered(1, 1, "Wide child")
            + &numbered(1, 2, "Grandchild")
            + &numbered(1, 1, "Next child")
            + &numbered(1, 2, "Next grandchild");
        assert_eq!(
            convert(&body, None, Some(&numbering)).unwrap(),
            "100. Wide parent\n\n     1000) Wide child\n\n           - Grandchild\n\n     1001) Next child\n\n           - Next grandchild\n"
        );
    }
    #[test]
    fn decimal_counter_boundary_changes_child_indent_without_changing_bullets() {
        let numbering = numbering().replace(
            r#"<w:startOverride w:val="4"/>"#,
            r#"<w:startOverride w:val="99"/>"#,
        );
        let body = numbered(1, 0, "Before boundary")
            + &numbered(1, 1, "Child")
            + &numbered(1, 0, "After boundary")
            + &numbered(1, 1, "Child");
        assert_eq!(
            convert(&body, None, Some(&numbering)).unwrap(),
            "99. Before boundary\n\n    1. Child\n\n100. After boundary\n\n     1. Child\n"
        );

        let bullets = numbering.replace(
            "<w:numFmt w:val=\"decimal\"/><w:lvlText w:val=\"%1.\"/>",
            "<w:numFmt w:val=\"bullet\"/><w:lvlText w:val=\"•\"/>",
        );
        assert_eq!(
            convert(
                &(numbered(1, 0, "Bullet")
                    + &numbered(1, 1, "Nested")
                    + &numbered(1, 0, "Bullet again")
                    + &numbered(1, 1, "Nested again")),
                None,
                Some(&bullets)
            )
            .unwrap(),
            "- Bullet\n\n    1. Nested\n\n- Bullet again\n\n    1. Nested again\n"
        );
    }
    #[test]
    fn output_bounds_and_cancel_refuse_without_partial_success() {
        assert_eq!(
            convert(&p(&"a".repeat(LIMIT - 1)), None, None)
                .unwrap()
                .len(),
            LIMIT
        );
        assert!(matches!(
            convert(&p(&"a".repeat(LIMIT)), None, None),
            Err(Failure::Limit)
        ));
        assert!(matches!(
            convert(&p(&"&amp;".repeat(LIMIT / 4)), None, None),
            Err(Failure::Limit)
        ));
        let xml = format!(
            r#"<w:document xmlns:w="{W}"><w:body>{}</w:body></w:document>"#,
            p("No conversion")
        );
        let doc = Document::parse(&xml).unwrap();
        let cancel = AtomicBool::new(false);
        cancel.store(true, Ordering::SeqCst);
        assert!(matches!(
            render(&doc, None, None, &BTreeMap::new(), &cancel),
            Err(Failure::Cancelled)
        ));
    }
}
