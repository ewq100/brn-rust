//! One literal, bounded OPC package interpretation. Relationships are never fetched.
use super::{Failure, Result, check_cancel, document, xml};
use roxmltree::{Document, Node};
use std::{collections::BTreeMap, sync::atomic::AtomicBool};

const CT: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const WORD: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
const STYLES: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml";
const NUMBERING: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml";
const REL_TYPE: &str = "application/vnd.openxmlformats-package.relationships+xml";
const CORE: &str = "application/vnd.openxmlformats-package.core-properties+xml";
const EXTENDED: &str = "application/vnd.openxmlformats-officedocument.extended-properties+xml";
const CUSTOM: &str = "application/vnd.openxmlformats-officedocument.custom-properties+xml";

fn metadata_type(kind: &str) -> Option<&'static str> {
    match kind {
        "settings" => {
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml")
        }
        "webSettings" => {
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.webSettings+xml")
        }
        "fontTable" => {
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.fontTable+xml")
        }
        "theme" => Some("application/vnd.openxmlformats-officedocument.theme+xml"),
        "core-properties" => Some(CORE),
        "extended-properties" => Some(EXTENDED),
        "custom-properties" => Some(CUSTOM),
        _ => None,
    }
}

fn metadata(doc: &Document<'_>, kind: &str) -> Result<()> {
    let (namespace, tag) = match kind {
        "settings" => (
            "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
            "settings",
        ),
        "webSettings" => (
            "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
            "webSettings",
        ),
        "fontTable" => (
            "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
            "fonts",
        ),
        "theme" => (
            "http://schemas.openxmlformats.org/drawingml/2006/main",
            "theme",
        ),
        "core-properties" => (
            "http://schemas.openxmlformats.org/package/2006/metadata/core-properties",
            "coreProperties",
        ),
        "extended-properties" => (
            "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties",
            "Properties",
        ),
        "custom-properties" => (
            "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties",
            "Properties",
        ),
        _ => return Err(Failure::Unsupported),
    };
    let root = doc.root_element();
    let strict_namespace = match namespace {
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main" => {
            "http://purl.oclc.org/ooxml/wordprocessingml/main"
        }
        "http://schemas.openxmlformats.org/drawingml/2006/main" => {
            "http://purl.oclc.org/ooxml/drawingml/main"
        }
        "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" => {
            "http://purl.oclc.org/ooxml/officeDocument/extendedProperties"
        }
        "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties" => {
            "http://purl.oclc.org/ooxml/officeDocument/customProperties"
        }
        _ => namespace,
    };
    if root.tag_name().name() != tag
        || ![Some(namespace), Some(strict_namespace)].contains(&root.tag_name().namespace())
    {
        return Err(Failure::Invalid);
    }
    if root.descendants().filter(Node::is_element).any(|n| {
        matches!(
            n.tag_name().namespace(),
            Some(
                "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    | "http://purl.oclc.org/ooxml/wordprocessingml/main"
            )
        ) && matches!(
            n.tag_name().name(),
            "document"
                | "body"
                | "p"
                | "r"
                | "t"
                | "tbl"
                | "hyperlink"
                | "drawing"
                | "pict"
                | "object"
                | "txbxContent"
                | "footnotes"
                | "endnotes"
                | "hdr"
                | "ftr"
                | "comments"
        )
    }) {
        return Err(Failure::Unsupported);
    }
    Ok(())
}

struct Relationship {
    kind: String,
    target: String,
    external: bool,
}

fn ordinary_children<'a, 'input>(node: Node<'a, 'input>) -> Result<Vec<Node<'a, 'input>>> {
    if node
        .children()
        .any(|n| n.is_text() && !n.text().unwrap_or("").trim().is_empty())
    {
        return Err(Failure::Invalid);
    }
    Ok(node.children().filter(Node::is_element).collect())
}

fn attributes(node: Node<'_, '_>, required: &[&str], optional: &[&str]) -> Result<()> {
    if required.iter().any(|key| node.attribute(*key).is_none()) {
        return Err(Failure::Invalid);
    }
    if node.attributes().any(|a| {
        a.namespace().is_some() || !required.contains(&a.name()) && !optional.contains(&a.name())
    }) {
        return Err(Failure::Unsupported);
    }
    for a in node.attributes() {
        let max = if a.name() == "Target" {
            crate::MAX_NOTE_BYTES
        } else {
            2048
        };
        if a.value().is_empty() || a.value().len() > max || a.value().chars().any(char::is_control)
        {
            return Err(Failure::Invalid);
        }
    }
    Ok(())
}

fn content_types(
    doc: &Document<'_>,
    parts: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, String>> {
    let root = doc.root_element();
    if !root.has_tag_name((CT, "Types")) {
        return Err(Failure::Invalid);
    }
    attributes(root, &[], &[])?;
    let mut defaults = BTreeMap::new();
    let mut overrides = BTreeMap::new();
    for node in ordinary_children(root)? {
        if node.has_tag_name((CT, "Default")) {
            attributes(node, &["Extension", "ContentType"], &[])?;
            let extension = node.attribute("Extension").ok_or(Failure::Invalid)?;
            if !extension.is_ascii()
                || extension.contains(['/', '.', '%', ':', '\\'])
                || extension.chars().any(char::is_whitespace)
                || defaults
                    .insert(
                        extension.to_owned(),
                        node.attribute("ContentType")
                            .ok_or(Failure::Invalid)?
                            .to_owned(),
                    )
                    .is_some()
            {
                return Err(Failure::Invalid);
            }
        } else if node.has_tag_name((CT, "Override")) {
            attributes(node, &["PartName", "ContentType"], &[])?;
            let name = node
                .attribute("PartName")
                .and_then(|v| v.strip_prefix('/'))
                .ok_or(Failure::Invalid)?;
            if !parts.contains_key(name)
                || overrides
                    .insert(
                        name.to_owned(),
                        node.attribute("ContentType")
                            .ok_or(Failure::Invalid)?
                            .to_owned(),
                    )
                    .is_some()
            {
                return Err(Failure::Invalid);
            }
        } else {
            return Err(Failure::Unsupported);
        }
        if !ordinary_children(node)?.is_empty() {
            return Err(Failure::Invalid);
        }
    }
    let mut types = BTreeMap::new();
    for name in parts.keys().filter(|n| n.as_str() != "[Content_Types].xml") {
        let value = overrides
            .get(name)
            .or_else(|| name.rsplit_once('.').and_then(|(_, ext)| defaults.get(ext)))
            .ok_or(Failure::Invalid)?;
        types.insert(name.clone(), value.clone());
    }
    Ok(types)
}

fn relationship_kind(value: &str) -> Result<&str> {
    const PREFIXES: [&str; 3] = [
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/",
        "http://purl.oclc.org/ooxml/officeDocument/relationships/",
        "http://schemas.openxmlformats.org/package/2006/relationships/metadata/",
    ];
    PREFIXES
        .iter()
        .find_map(|p| value.strip_prefix(p))
        .filter(|v| !v.is_empty() && !v.contains('/'))
        .ok_or(Failure::Unsupported)
}

fn resolve(base: &str, target: &str) -> Result<String> {
    if !target.is_ascii()
        || target.contains(['\\', '%', ':', '?', '#'])
        || target.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(Failure::Invalid);
    }
    let mut segments: Vec<_> = if target.starts_with('/') {
        Vec::new()
    } else {
        base.rsplit_once('/')
            .map_or(Vec::new(), |(parent, _)| parent.split('/').collect())
    };
    for segment in target.strip_prefix('/').unwrap_or(target).split('/') {
        match segment {
            "" => return Err(Failure::Invalid),
            "." => {}
            ".." => {
                segments.pop().ok_or(Failure::Invalid)?;
            }
            _ => segments.push(segment),
        }
    }
    if segments.is_empty() {
        return Err(Failure::Invalid);
    }
    Ok(segments.join("/"))
}

fn relationships(
    doc: &Document<'_>,
    base: &str,
    parts: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Relationship>> {
    let root = doc.root_element();
    if !root.has_tag_name((REL, "Relationships")) {
        return Err(Failure::Invalid);
    }
    attributes(root, &[], &[])?;
    let mut records = BTreeMap::new();
    for node in ordinary_children(root)? {
        if !node.has_tag_name((REL, "Relationship")) {
            return Err(Failure::Unsupported);
        }
        attributes(node, &["Id", "Type", "Target"], &["TargetMode"])?;
        if !ordinary_children(node)?.is_empty() {
            return Err(Failure::Invalid);
        }
        let id = node.attribute("Id").ok_or(Failure::Invalid)?;
        if id.len() > 512 || id.chars().any(char::is_whitespace) {
            return Err(Failure::Invalid);
        }
        let kind = relationship_kind(node.attribute("Type").ok_or(Failure::Invalid)?)?;
        let external = match node.attribute("TargetMode") {
            None | Some("Internal") => false,
            Some("External") => true,
            _ => return Err(Failure::Invalid),
        };
        let target = node.attribute("Target").ok_or(Failure::Invalid)?;
        let target = if external {
            if kind != "hyperlink" {
                return Err(Failure::Unsupported);
            }
            let scheme = target
                .split_once(':')
                .map(|(s, _)| s.to_ascii_lowercase())
                .ok_or(Failure::Unsupported)?;
            if !matches!(scheme.as_str(), "https" | "http" | "mailto")
                || target.chars().any(|c| c.is_control() || c.is_whitespace())
            {
                return Err(Failure::Unsupported);
            }
            target.to_owned()
        } else {
            let path = resolve(base, target)?;
            if !parts.contains_key(&path) {
                return Err(Failure::Invalid);
            }
            path
        };
        if records
            .insert(
                id.to_owned(),
                Relationship {
                    kind: kind.into(),
                    target,
                    external,
                },
            )
            .is_some()
        {
            return Err(Failure::Invalid);
        }
    }
    Ok(records)
}

fn relationship_part(base: &str) -> String {
    base.rsplit_once('/').map_or_else(
        || format!("_rels/{base}.rels"),
        |(parent, name)| format!("{parent}/_rels/{name}.rels"),
    )
}

pub(super) fn render(parts: &BTreeMap<String, Vec<u8>>, cancel: &AtomicBool) -> Result<String> {
    let mut budget = xml::Budget::default();
    let ct = budget.parse(
        parts.get("[Content_Types].xml").ok_or(Failure::Invalid)?,
        cancel,
    )?;
    let types = content_types(&ct, parts)?;
    let mut docs = BTreeMap::new();
    for (path, bytes) in parts
        .iter()
        .filter(|(p, _)| p.as_str() != "[Content_Types].xml")
    {
        check_cancel(cancel)?;
        let kind = types.get(path).ok_or(Failure::Invalid)?.as_str();
        if !matches!(
            kind,
            WORD | STYLES
                | NUMBERING
                | REL_TYPE
                | "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"
                | "application/vnd.openxmlformats-officedocument.wordprocessingml.webSettings+xml"
                | "application/vnd.openxmlformats-officedocument.wordprocessingml.fontTable+xml"
                | "application/vnd.openxmlformats-officedocument.theme+xml"
                | "application/vnd.openxmlformats-package.core-properties+xml"
                | "application/vnd.openxmlformats-officedocument.extended-properties+xml"
                | "application/vnd.openxmlformats-officedocument.custom-properties+xml"
        ) {
            return Err(Failure::Unsupported);
        }
        docs.insert(path.as_str(), budget.parse(bytes, cancel)?);
    }
    let root_rels = relationships(docs.get("_rels/.rels").ok_or(Failure::Invalid)?, "", parts)?;
    let mut main = None;
    for relationship in root_rels.values() {
        match relationship.kind.as_str() {
            "officeDocument" if !relationship.external && main.is_none() => {
                main = Some(relationship.target.as_str())
            }
            "core-properties" | "extended-properties" | "custom-properties"
                if !relationship.external
                    && types.get(&relationship.target).map(String::as_str)
                        == metadata_type(&relationship.kind) =>
            {
                metadata(
                    docs.get(relationship.target.as_str())
                        .ok_or(Failure::Invalid)?,
                    &relationship.kind,
                )?;
            }
            _ => return Err(Failure::Unsupported),
        }
    }
    let main = main.ok_or(Failure::Invalid)?;
    if types.get(main).map(String::as_str) != Some(WORD) {
        return Err(Failure::Invalid);
    }
    let relation_path = relationship_part(main);
    let main_rels = docs
        .get(relation_path.as_str())
        .map(|d| relationships(d, main, parts))
        .transpose()?
        .unwrap_or_default();
    let mut style_path = None;
    let mut numbering_path = None;
    let mut links = BTreeMap::new();
    let mut admitted = std::collections::BTreeSet::from(["_rels/.rels", main]);
    if docs.contains_key(relation_path.as_str()) {
        admitted.insert(relation_path.as_str());
    }
    for relationship in root_rels.values() {
        admitted.insert(relationship.target.as_str());
    }
    for (id, relationship) in &main_rels {
        match relationship.kind.as_str() {
            "hyperlink" if relationship.external => {
                links.insert(id.clone(), relationship.target.clone());
            }
            "styles"
                if !relationship.external
                    && style_path.is_none()
                    && types.get(&relationship.target).map(String::as_str) == Some(STYLES) =>
            {
                style_path = Some(relationship.target.as_str())
            }
            "numbering"
                if !relationship.external
                    && numbering_path.is_none()
                    && types.get(&relationship.target).map(String::as_str) == Some(NUMBERING) =>
            {
                numbering_path = Some(relationship.target.as_str())
            }
            "settings" | "webSettings" | "fontTable" | "theme"
                if !relationship.external
                    && types.get(&relationship.target).map(String::as_str)
                        == metadata_type(&relationship.kind) =>
            {
                metadata(
                    docs.get(relationship.target.as_str())
                        .ok_or(Failure::Invalid)?,
                    &relationship.kind,
                )?;
            }
            _ => return Err(Failure::Unsupported),
        }
        if !relationship.external {
            admitted.insert(relationship.target.as_str());
        }
    }
    // Unreferenced parts and subordinate relationships can carry meaningful
    // content too. Never hide them behind a successful main-body extraction.
    if docs.keys().any(|path| !admitted.contains(path)) {
        return Err(Failure::Unsupported);
    }
    for path in admitted
        .iter()
        .filter(|p| types.get(**p).map(String::as_str) == Some(REL_TYPE))
    {
        if *path != "_rels/.rels" && *path != relation_path {
            return Err(Failure::Unsupported);
        }
    }
    document::render(
        docs.get(main).ok_or(Failure::Invalid)?,
        style_path.and_then(|p| docs.get(p)),
        numbering_path.and_then(|p| docs.get(p)),
        &links,
        cancel,
    )
}
