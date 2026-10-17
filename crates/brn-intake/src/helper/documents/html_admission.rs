//! Conservative byte-level admission for the first Xberg HTML profile (`html-textual-1`).
//!
//! This is not an HTML parser or text extractor. It admits only a small agreement language on
//! which the WHATWG tokenizer (html5ever), Xberg's tier-2 parser (astral-tl) and the converter
//! walk cannot disagree about structure: lowercase allowlisted tags immediately after `<`, exact
//! `</name>` closes, explicit closing of every non-void element, restricted parent/child content
//! models, a single fully quoted `href` attribute on `a`, a small entity set and no comments,
//! declarations, raw-text elements, tables, images or embedded resources. Everything else is
//! refused before any backend work. Tier 1 is unreachable because Xberg forces document
//! structure, which the upstream router sends to tier 2. Input without any `<` takes
//! upstream's `fast_text_only` path before tier 2 (no DOM, no document structure); its output
//! is integrity-checked like every other result.
//!
//! The scanner uses only `std`, so it can be qualified independently of the backend graph.

/// Stable identifier of the admitted language. Changing the language requires a new value.
pub const PROFILE: &str = "html-textual-1";
/// Upper bound of admitted HTML, equal to the extraction text ceiling.
pub const MAX_HTML_BYTES: usize = 8 * 1024 * 1024;
/// Explicit source element stack bound. Xberg's walk guard is pinned at 128.
pub const MAX_DEPTH: usize = 32;
/// Conservative parser node bound: `2 * count('<') + 1`.
pub const MAX_NODES: usize = 200_000;
pub const MAX_ENTITIES: usize = 65_536;
pub const MAX_ATTRIBUTE_BYTES: usize = 64 * 1024;
pub const MAX_HREF_BYTES: usize = 2048;
pub const MAX_LINKS: usize = 4096;

/// One structural token in source order. Text is entity-decoded with CRLF folded to LF,
/// matching the tokenizer's input-stream preprocessing. Adjacent text is merged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanToken {
    Start {
        name: &'static str,
        href: Option<String>,
    },
    End(&'static str),
    Text(String),
}

/// Facts recorded by a successful admission scan.
#[derive(Debug, Default)]
pub struct Admitted {
    pub tokens: Vec<ScanToken>,
    /// Maximum element depth reached (a void element counts one level below its parent).
    pub depth: usize,
    /// Count of `<` bytes, the basis of the conservative node bound (test facts only).
    #[cfg(test)]
    pub lt_count: usize,
    pub elements: usize,
    pub text_bytes: usize,
    pub entities: usize,
    pub attribute_bytes: usize,
    pub links: usize,
    root_html: bool,
    root_content: bool,
    /// One `body` at most: the WHATWG builder merges a second body, astral-tl keeps two.
    body_seen: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Model {
    /// `html`: only `body` and inter-element whitespace.
    Document,
    /// Blocks, phrasing and text.
    Flow,
    /// Phrasing elements and text only.
    Phrasing,
    /// `li` and inter-element whitespace only.
    List,
    /// Text or `code` only.
    Pre,
    /// Text only.
    TextOnly,
    Void,
}

#[derive(Clone, Copy, Debug)]
struct Element {
    name: &'static str,
    model: Model,
    block: bool,
}

const fn el(name: &'static str, model: Model, block: bool) -> Element {
    Element { name, model, block }
}

const ELEMENTS: &[Element] = &[
    el("html", Model::Document, true),
    el("body", Model::Flow, true),
    el("div", Model::Flow, true),
    el("section", Model::Flow, true),
    el("article", Model::Flow, true),
    el("blockquote", Model::Flow, true),
    el("li", Model::Flow, true),
    el("ul", Model::List, true),
    el("ol", Model::List, true),
    el("p", Model::Phrasing, true),
    el("h1", Model::Phrasing, true),
    el("h2", Model::Phrasing, true),
    el("h3", Model::Phrasing, true),
    el("h4", Model::Phrasing, true),
    el("h5", Model::Phrasing, true),
    el("h6", Model::Phrasing, true),
    el("pre", Model::Pre, true),
    el("hr", Model::Void, true),
    el("code", Model::TextOnly, false),
    el("a", Model::Phrasing, false),
    el("em", Model::Phrasing, false),
    el("strong", Model::Phrasing, false),
    el("b", Model::Phrasing, false),
    el("i", Model::Phrasing, false),
    el("br", Model::Void, false),
];

const RESOURCE: &[&str] = &[
    "img", "picture", "source", "video", "audio", "track", "iframe", "embed", "object", "link",
    "svg", "math", "canvas", "image", "frame", "frameset", "portal", "base", "meta", "input",
    "form", "button",
];
const TABLE: &[&str] = &[
    "table", "caption", "thead", "tbody", "tfoot", "tr", "td", "th", "col", "colgroup",
];
const RAW_TEXT: &[&str] = &[
    "script",
    "style",
    "textarea",
    "template",
    "noscript",
    "title",
    "xmp",
    "plaintext",
    "noembed",
    "noframes",
    "head",
];

/// Static refusal labels; no source bytes, paths or upstream messages are included.
pub mod refusal {
    pub const SIZE: &str = "HTML admission: empty or above the 8 MiB textual profile ceiling";
    pub const UTF8: &str = "HTML admission: input is not strict UTF-8";
    pub const NUL: &str = "HTML admission: NUL byte refused";
    pub const CONTROL: &str = "HTML admission: control, BOM or noncharacter text refused";
    pub const NODES: &str = "HTML admission: conservative node bound exceeded";
    pub const DEPTH: &str = "HTML admission: source element depth above 32";
    pub const DECLARATION: &str =
        "HTML admission: comment, declaration or processing instruction refused";
    pub const TAG_SYNTAX: &str =
        "HTML admission: tag syntax outside the agreement language (whitespace, case or form)";
    pub const RESOURCE: &str =
        "HTML admission: image, media, form or embedded-resource element refused before backend";
    pub const TABLE: &str = "HTML admission: tables are outside the first textual profile";
    pub const RAW_TEXT: &str = "HTML admission: raw-text or head element refused";
    pub const ELEMENT: &str = "HTML admission: element outside the admitted textual profile";
    pub const ATTRIBUTE: &str = "HTML admission: attribute outside the admitted textual profile";
    pub const HREF: &str = "HTML admission: link target outside inert http(s)/mailto/fragment";
    pub const DATA_URI: &str = "HTML admission: data URI refused";
    pub const CONTENT_MODEL: &str =
        "HTML admission: content model would rely on implicit structure";
    pub const CLOSE: &str = "HTML admission: missing, unmatched or implicit end tag";
    pub const TEXT_SYNTAX: &str = "HTML admission: raw '>' outside an admitted form";
    pub const ENTITY: &str = "HTML admission: '&' or entity outside the admitted set";
    pub const QUOTA: &str = "HTML admission: text, entity, attribute or link quota exceeded";
}

struct Open {
    element: Element,
    /// Within an `a` ancestor (nested links are refused).
    in_link: bool,
}

/// Scan `bytes` and admit them only if they belong to `html-textual-1`.
///
/// `max_text_bytes` is the caller's decoded-text quota (already bounded by request limits).
/// Strict size, NUL, UTF-8 and node-bound checks run before any tag is interpreted.
pub fn admit(bytes: &[u8], max_text_bytes: usize) -> Result<Admitted, &'static str> {
    if bytes.is_empty() || bytes.len() > MAX_HTML_BYTES {
        return Err(refusal::SIZE);
    }
    if bytes.contains(&0) {
        return Err(refusal::NUL);
    }
    let html = std::str::from_utf8(bytes).map_err(|_| refusal::UTF8)?;
    let lt_count = bytes.iter().filter(|b| **b == b'<').count();
    let node_bound = lt_count
        .checked_mul(2)
        .and_then(|n| n.checked_add(1))
        .ok_or(refusal::NODES)?;
    if node_bound > MAX_NODES {
        return Err(refusal::NODES);
    }
    #[cfg(test)]
    let mut out = Admitted {
        lt_count,
        ..Admitted::default()
    };
    #[cfg(not(test))]
    let mut out = Admitted::default();
    let mut stack: Vec<Open> = Vec::new();
    let mut at = 0usize;
    while at < bytes.len() {
        if bytes[at] == b'<' {
            at = tag(html, at, &mut stack, &mut out)?;
            continue;
        }
        let end = bytes[at..]
            .iter()
            .position(|b| *b == b'<')
            .map_or(bytes.len(), |offset| at + offset);
        // Directly after `<pre>` the WHATWG tree builder drops one newline; astral-tl keeps it.
        let after_pre = matches!(
            out.tokens.last(),
            Some(ScanToken::Start { name: "pre", .. })
        );
        text(&html[at..end], &stack, after_pre, max_text_bytes, &mut out)?;
        at = end;
    }
    if !stack.is_empty() {
        return Err(refusal::CLOSE);
    }
    Ok(out)
}

fn classify_unknown(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if RESOURCE.contains(&lower.as_str()) {
        refusal::RESOURCE
    } else if TABLE.contains(&lower.as_str()) {
        refusal::TABLE
    } else if RAW_TEXT.contains(&lower.as_str()) {
        refusal::RAW_TEXT
    } else if name.bytes().any(|b| b.is_ascii_uppercase())
        && ELEMENTS.iter().any(|e| e.name == lower)
    {
        refusal::TAG_SYNTAX
    } else {
        refusal::ELEMENT
    }
}

/// Interpret one tag starting at `<`; return the index after `>`.
fn tag(
    html: &str,
    at: usize,
    stack: &mut Vec<Open>,
    out: &mut Admitted,
) -> Result<usize, &'static str> {
    let bytes = html.as_bytes();
    let mut i = at + 1;
    let closing = bytes.get(i) == Some(&b'/');
    if closing {
        i += 1;
    }
    match bytes.get(i) {
        Some(b'!' | b'?') if !closing => return Err(refusal::DECLARATION),
        Some(b) if b.is_ascii_alphabetic() => {}
        // `< img`, `</ div>`, `<>`, `<3` and a trailing `<` are outside the language. Text uses
        // `&lt;` for a literal `<`.
        _ => return Err(refusal::TAG_SYNTAX),
    }
    let name_start = i;
    while bytes.get(i).is_some_and(u8::is_ascii_alphanumeric) {
        i += 1;
    }
    let raw_name = &html[name_start..i];
    let element = ELEMENTS
        .iter()
        .find(|e| e.name == raw_name)
        .copied()
        .ok_or_else(|| classify_unknown(raw_name))?;
    if closing {
        // Exact `</name>` only: no whitespace, attributes or other bytes.
        if bytes.get(i) != Some(&b'>') {
            return Err(refusal::TAG_SYNTAX);
        }
        if element.model == Model::Void {
            return Err(refusal::CLOSE);
        }
        let open = stack.pop().ok_or(refusal::CLOSE)?;
        if open.element.name != element.name {
            return Err(refusal::CLOSE);
        }
        out.tokens.push(ScanToken::End(element.name));
        return Ok(i + 1);
    }
    let mut href = None;
    match bytes.get(i) {
        Some(b'>') => i += 1,
        Some(b' ') if element.name == "a" => {
            let (value, next) = href_attribute(html, i, out)?;
            href = Some(value);
            i = next;
        }
        Some(b' ' | b'\t' | b'\n' | b'\r' | b'\x0c') => return Err(refusal::ATTRIBUTE),
        // `<br/>`, `<p/>` and any other byte directly after the name.
        _ => return Err(refusal::TAG_SYNTAX),
    }
    admit_child(stack, element, out)?;
    out.elements += 1;
    let in_link = element.name == "a" || stack.last().is_some_and(|open| open.in_link);
    out.tokens.push(ScanToken::Start {
        name: element.name,
        href,
    });
    let level = stack.len() + 1;
    if level > MAX_DEPTH {
        return Err(refusal::DEPTH);
    }
    out.depth = out.depth.max(level);
    if element.model != Model::Void {
        stack.push(Open { element, in_link });
    }
    Ok(i)
}

/// Exact ` href="value">` with no other attribute or spacing variant.
fn href_attribute(
    html: &str,
    at: usize,
    out: &mut Admitted,
) -> Result<(String, usize), &'static str> {
    let bytes = html.as_bytes();
    const PREFIX: &[u8] = b" href=\"";
    if !bytes[at..].starts_with(PREFIX) {
        return Err(refusal::ATTRIBUTE);
    }
    let start = at + PREFIX.len();
    let len = bytes[start..]
        .iter()
        .position(|b| *b == b'"')
        .ok_or(refusal::ATTRIBUTE)?;
    let end = start + len;
    if bytes.get(end + 1) != Some(&b'>') {
        return Err(refusal::ATTRIBUTE);
    }
    let raw = &html[start..end];
    if raw.is_empty() || raw.len() > MAX_HREF_BYTES {
        return Err(refusal::HREF);
    }
    out.attribute_bytes = out
        .attribute_bytes
        .checked_add(raw.len())
        .ok_or(refusal::QUOTA)?;
    out.links += 1;
    if out.attribute_bytes > MAX_ATTRIBUTE_BYTES || out.links > MAX_LINKS {
        return Err(refusal::QUOTA);
    }
    if raw
        .get(..5)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("data:"))
    {
        return Err(refusal::DATA_URI);
    }
    if !(raw.starts_with("https://")
        || raw.starts_with("http://")
        || raw.starts_with("mailto:")
        || raw.starts_with('#'))
    {
        return Err(refusal::HREF);
    }
    let mut value = String::with_capacity(raw.len());
    let rb = raw.as_bytes();
    let mut i = 0;
    while i < rb.len() {
        let b = rb[i];
        if b == b'&' {
            if !rb[i..].starts_with(b"&amp;") {
                return Err(refusal::HREF);
            }
            out.entities += 1;
            if out.entities > MAX_ENTITIES {
                return Err(refusal::QUOTA);
            }
            value.push('&');
            i += 5;
            continue;
        }
        if !(b.is_ascii_alphanumeric() || b"-._~:/?#[]@!$()*+,;=%".contains(&b)) {
            return Err(refusal::HREF);
        }
        value.push(char::from(b));
        i += 1;
    }
    Ok((value, end + 2))
}

/// Restricted parent/child content models. Anything the WHATWG tree builder would repair
/// (a block inside phrasing, `li` outside a list, nested links, a misplaced `body`/`html`)
/// is refused.
fn admit_child(stack: &[Open], child: Element, out: &mut Admitted) -> Result<(), &'static str> {
    let parent = stack.last();
    if parent.is_none() {
        // `html` must be the only root content; nothing may follow or precede it.
        if out.root_html || (child.name == "html" && out.root_content) {
            return Err(refusal::CONTENT_MODEL);
        }
        if child.name == "html" {
            out.root_html = true;
        } else {
            out.root_content = true;
        }
    }
    if child.name == "body" {
        if out.body_seen {
            return Err(refusal::CONTENT_MODEL);
        }
        out.body_seen = true;
    }
    let parent_model = parent.map_or(Model::Flow, |open| open.element.model);
    let ok = match child.name {
        "html" => parent.is_none(),
        "body" => parent.is_some_and(|open| open.element.name == "html"),
        "li" => parent_model == Model::List,
        "code" => matches!(parent_model, Model::Flow | Model::Phrasing | Model::Pre),
        "a" => {
            !parent.is_some_and(|open| open.in_link)
                && matches!(parent_model, Model::Flow | Model::Phrasing)
        }
        _ => match parent_model {
            Model::Document | Model::List | Model::TextOnly | Model::Void | Model::Pre => false,
            Model::Flow => true,
            Model::Phrasing => !child.block,
        },
    };
    if ok {
        Ok(())
    } else {
        Err(refusal::CONTENT_MODEL)
    }
}

fn is_html_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// Validate and decode one text run between tags.
fn text(
    raw: &str,
    stack: &[Open],
    after_pre: bool,
    max_text_bytes: usize,
    out: &mut Admitted,
) -> Result<(), &'static str> {
    let model = stack.last().map_or(Model::Flow, |open| open.element.model);
    let whitespace_only = raw.chars().all(is_html_space);
    if !whitespace_only {
        if matches!(model, Model::Document | Model::List) {
            return Err(refusal::CONTENT_MODEL);
        }
        if stack.is_empty() {
            // Text beside a root `html` element would be relocated by the tree builder.
            if out.root_html {
                return Err(refusal::CONTENT_MODEL);
            }
            out.root_content = true;
        }
    }
    if after_pre && raw.starts_with(['\n', '\r']) {
        return Err(refusal::CONTENT_MODEL);
    }
    let mut decoded = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(c) = rest.chars().next() {
        match c {
            '>' => return Err(refusal::TEXT_SYNTAX),
            '&' => {
                let (value, len) = entity(rest)?;
                out.entities += 1;
                if out.entities > MAX_ENTITIES {
                    return Err(refusal::QUOTA);
                }
                decoded.push(value);
                rest = &rest[len..];
                continue;
            }
            '\r' if rest.as_bytes().get(1) == Some(&b'\n') => {}
            '\t' | '\n' => decoded.push(c),
            c if forbidden_char(c) => return Err(refusal::CONTROL),
            c => decoded.push(c),
        }
        rest = &rest[c.len_utf8()..];
    }
    out.text_bytes = out
        .text_bytes
        .checked_add(decoded.len())
        .ok_or(refusal::QUOTA)?;
    if out.text_bytes > max_text_bytes {
        return Err(refusal::QUOTA);
    }
    if decoded.is_empty() {
        return Ok(());
    }
    match out.tokens.last_mut() {
        Some(ScanToken::Text(previous)) => previous.push_str(&decoded),
        _ => out.tokens.push(ScanToken::Text(decoded)),
    }
    Ok(())
}

/// C0/C1 controls (a lone CR included), DEL, BOM and Unicode noncharacters.
fn forbidden_char(c: char) -> bool {
    let v = u32::from(c);
    v < 0x20
        || (0x7f..=0x9f).contains(&v)
        || v == 0xfeff
        || (0xfdd0..=0xfdef).contains(&v)
        || (v & 0xfffe) == 0xfffe
}

/// Decode one entity at the start of `s`; return the scalar and the entity's byte length.
fn entity(s: &str) -> Result<(char, usize), &'static str> {
    for (name, value) in [
        ("&amp;", '&'),
        ("&lt;", '<'),
        ("&gt;", '>'),
        ("&quot;", '"'),
        ("&apos;", '\''),
        ("&nbsp;", '\u{a0}'),
    ] {
        if s.starts_with(name) {
            return Ok((value, name.len()));
        }
    }
    let rest = s.strip_prefix("&#").ok_or(refusal::ENTITY)?;
    let (hex, digits) = match rest.strip_prefix(['x', 'X']) {
        Some(digits) => (true, digits),
        None => (false, rest),
    };
    let len = digits
        .bytes()
        .take_while(|b| {
            if hex {
                b.is_ascii_hexdigit()
            } else {
                b.is_ascii_digit()
            }
        })
        .count();
    if len == 0 || len > 6 || digits.as_bytes().get(len) != Some(&b';') {
        return Err(refusal::ENTITY);
    }
    let value = u32::from_str_radix(&digits[..len], if hex { 16 } else { 10 })
        .map_err(|_| refusal::ENTITY)?;
    let scalar = char::from_u32(value).ok_or(refusal::ENTITY)?;
    // Refuse every value the WHATWG tokenizer remaps or flags (NUL, controls, surrogates,
    // noncharacters) and encoded whitespace, so both parsers decode identically.
    if forbidden_char(scalar) {
        return Err(refusal::ENTITY);
    }
    Ok((scalar, 2 + usize::from(hex) + len + 1))
}

/// Ordered, whitespace-separated words of every text token. Each text token is split on its
/// own, so inline markup boundaries never merge words across elements.
pub fn words(admitted: &Admitted) -> impl Iterator<Item = &str> {
    admitted
        .tokens
        .iter()
        .filter_map(|token| match token {
            ScanToken::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .flat_map(|text| {
            text.split(|c: char| is_html_space(c) || c == '\u{a0}')
                .filter(|word| !word.is_empty())
        })
}

/// Admitted link targets in source order.
pub fn hrefs(admitted: &Admitted) -> impl Iterator<Item = &str> {
    admitted.tokens.iter().filter_map(|token| match token {
        ScanToken::Start {
            href: Some(href), ..
        } => Some(href.as_str()),
        _ => None,
    })
}

#[cfg(test)]
pub mod tests_support {
    /// `depth` nested `div` elements around one word.
    pub fn nested(depth: usize) -> String {
        format!("{}x{}", "<div>".repeat(depth), "</div>".repeat(depth))
    }
}

#[cfg(test)]
mod tests {
    use super::{refusal as r, tests_support::nested, *};

    fn ok(html: &str) -> Admitted {
        admit(html.as_bytes(), MAX_HTML_BYTES).unwrap_or_else(|e| panic!("{html:?}: {e}"))
    }
    fn no(html: &str) -> &'static str {
        match admit(html.as_bytes(), MAX_HTML_BYTES) {
            Ok(_) => panic!("unexpectedly admitted {html:?}"),
            Err(error) => error,
        }
    }
    #[test]
    fn admits_the_textual_profile_with_exact_facts() {
        let a = ok(concat!(
            "<html><body>\n<h1>Title &amp; more</h1>\n<p>Caf\u{e9} <em>na\u{ef}ve</em>",
            " <a href=\"https://example.test/a?b=1&amp;c=2\">link</a>&nbsp;&#x2603;</p>\n",
            "<ul><li>one</li><li><p>two</p><ol><li>three</li></ol></li></ul>\n",
            "<blockquote><p>quoted</p></blockquote><pre><code>let x = 1 &lt; 2;</code></pre>",
            "<hr><p>end<br>line</p></body></html>",
        ));
        // html > body > ul > li > ol > li
        assert_eq!(a.depth, 6);
        // html body h1 p em a ul li li p ol li blockquote p pre code hr p br
        assert_eq!(a.elements, 19);
        assert_eq!(a.links, 1);
        assert_eq!(
            hrefs(&a).collect::<Vec<_>>(),
            ["https://example.test/a?b=1&c=2"]
        );
        assert_eq!(
            words(&a).collect::<Vec<_>>(),
            [
                "Title",
                "&",
                "more",
                "Caf\u{e9}",
                "na\u{ef}ve",
                "link",
                "\u{2603}",
                "one",
                "two",
                "three",
                "quoted",
                "let",
                "x",
                "=",
                "1",
                "<",
                "2;",
                "end",
                "line"
            ]
        );
        assert_eq!(a.entities, 5);
    }

    #[test]
    fn refuses_parser_disagreement_forms_before_backend() {
        assert_eq!(no("< img src=\"x\">"), r::TAG_SYNTAX);
        assert_eq!(no("< table><tr><td>x</td></tr></table>"), r::TAG_SYNTAX);
        assert_eq!(no("<DIV>x</DIV>"), r::TAG_SYNTAX);
        assert_eq!(no("<Img>"), r::RESOURCE);
        assert_eq!(no("<div>x</div >"), r::TAG_SYNTAX);
        assert_eq!(no("<div>x</ div>"), r::TAG_SYNTAX);
        assert_eq!(no("<div>x</DIV>"), r::TAG_SYNTAX);
        assert_eq!(no("<p>a<p>b"), r::CONTENT_MODEL);
        assert_eq!(no("<p>a</p><p>b"), r::CLOSE);
        assert_eq!(no("<ul><li>a<li>b</ul>"), r::CONTENT_MODEL);
        assert_eq!(no("<p><div>x</div></p>"), r::CONTENT_MODEL);
        assert_eq!(no("<li>x</li>"), r::CONTENT_MODEL);
        assert_eq!(
            no("<a href=\"#a\"><a href=\"#b\">x</a></a>"),
            r::CONTENT_MODEL
        );
        assert_eq!(
            no("<a href=\"#a\"><em><a href=\"#b\">x</a></em></a>"),
            r::CONTENT_MODEL
        );
        assert_eq!(no("<ul>text<li>x</li></ul>"), r::CONTENT_MODEL);
        assert_eq!(no("<pre><p>x</p></pre>"), r::CONTENT_MODEL);
        assert_eq!(no("<code><em>x</em></code>"), r::CONTENT_MODEL);
        assert_eq!(no("<div>x</span>"), r::ELEMENT);
        assert_eq!(no("<div>x</p>"), r::CLOSE);
        assert_eq!(no("<br/>"), r::TAG_SYNTAX);
        assert_eq!(no("<br>x</br>"), r::CLOSE);
        assert_eq!(no("<pre>\nx</pre>"), r::CONTENT_MODEL);
        assert_eq!(no("<html><body>x</body></html>tail"), r::CONTENT_MODEL);
        assert_eq!(no("lead<html><body>x</body></html>"), r::CONTENT_MODEL);
        assert_eq!(no("<html><body>x</body></html><p>y</p>"), r::CONTENT_MODEL);
        assert_eq!(no("<body>x</body>"), r::CONTENT_MODEL);
        assert_eq!(no("<html><p>x</p></html>"), r::CONTENT_MODEL);
        assert_eq!(
            no("<html><body>a</body><body>b</body></html>"),
            r::CONTENT_MODEL
        );
        assert_eq!(no("x <"), r::TAG_SYNTAX);
        assert_eq!(no("1 <3"), r::TAG_SYNTAX);
        assert_eq!(no("a > b"), r::TEXT_SYNTAX);
        assert_eq!(no("a & b"), r::ENTITY);
        assert_eq!(no("&amp"), r::ENTITY);
        assert_eq!(no("<div>open"), r::CLOSE);
    }

    #[test]
    fn refuses_comments_raw_text_resources_and_tables() {
        assert_eq!(no("<!-- x -->"), r::DECLARATION);
        assert_eq!(no("<!doctype html><p>x</p>"), r::DECLARATION);
        assert_eq!(no("<?xml version=\"1.0\"?>"), r::DECLARATION);
        for raw in [
            "script", "style", "textarea", "template", "noscript", "title", "head",
        ] {
            assert_eq!(no(&format!("<{raw}>x</{raw}>")), r::RAW_TEXT, "{raw}");
        }
        for resource in [
            "img", "svg", "math", "iframe", "video", "object", "link", "form",
        ] {
            assert_eq!(no(&format!("<{resource}>")), r::RESOURCE, "{resource}");
        }
        assert_eq!(no("<table><tr><td>x</td></tr></table>"), r::TABLE);
        assert_eq!(no("<p>x</p><td>y</td>"), r::TABLE);
        assert_eq!(no("<span>x</span>"), r::ELEMENT);
        assert_eq!(no("<p class=\"x\">y</p>"), r::ATTRIBUTE);
        assert_eq!(no("<a href='x'>y</a>"), r::ATTRIBUTE);
        assert_eq!(no("<a  href=\"#x\">y</a>"), r::ATTRIBUTE);
        assert_eq!(no("<a href=\"#x\" title=\"t\">y</a>"), r::ATTRIBUTE);
        assert_eq!(no("<a href=\"#x\"title>y</a>"), r::ATTRIBUTE);
        assert_eq!(
            no("<a href=\"data:image/png;base64,AA\">y</a>"),
            r::DATA_URI
        );
        assert_eq!(no("<a href=\"DATA:text/html,x\">y</a>"), r::DATA_URI);
        assert_eq!(no("<a href=\"javascript:alert(1)\">y</a>"), r::HREF);
        assert_eq!(no("<a href=\"/relative\">y</a>"), r::HREF);
        assert_eq!(no("<a href=\"https://x.test/a b\">y</a>"), r::HREF);
        assert_eq!(no("<a href=\"https://x.test/?a&b\">y</a>"), r::HREF);
        assert_eq!(no("<a href=\"https://x.test/<x\">y</a>"), r::HREF);
    }

    #[test]
    fn strict_bytes_entities_and_quotas() {
        assert_eq!(admit(b"", MAX_HTML_BYTES).unwrap_err(), r::SIZE);
        assert_eq!(admit(b"<p>\xff</p>", MAX_HTML_BYTES).unwrap_err(), r::UTF8);
        assert_eq!(admit(b"<p>a\0b</p>", MAX_HTML_BYTES).unwrap_err(), r::NUL);
        let big = vec![b'a'; MAX_HTML_BYTES + 1];
        assert_eq!(admit(&big, usize::MAX).unwrap_err(), r::SIZE);
        assert_eq!(no("a\u{7}b"), r::CONTROL);
        assert_eq!(no("a\u{85}b"), r::CONTROL);
        assert_eq!(no("\u{feff}a"), r::CONTROL);
        assert_eq!(no("a\u{fffe}b"), r::CONTROL);
        assert_eq!(no("a\rb"), r::CONTROL);
        assert_eq!(ok("a\r\nb").tokens, [ScanToken::Text("a\nb".into())]);
        assert_eq!(no("&#0;"), r::ENTITY);
        assert_eq!(no("&#x80;"), r::ENTITY);
        assert_eq!(no("&#xD800;"), r::ENTITY);
        assert_eq!(no("&#x110000;"), r::ENTITY);
        assert_eq!(no("&#10;"), r::ENTITY);
        assert_eq!(no("&#x1234567;"), r::ENTITY);
        assert_eq!(no("&#65"), r::ENTITY);
        assert_eq!(no("&copy;"), r::ENTITY);
        assert_eq!(ok("&#65;&#x42;").tokens, [ScanToken::Text("AB".into())]);
        assert_eq!(admit(b"<p>abcd</p>", 3).unwrap_err(), r::QUOTA);
        assert!(admit(b"<p>abc</p>", 3).is_ok());
        assert_eq!(no(&"&amp;".repeat(MAX_ENTITIES + 1)), r::QUOTA);
        let href = format!(
            "<a href=\"https://x.test/{}\">y</a>",
            "a".repeat(MAX_HREF_BYTES)
        );
        assert_eq!(no(&href), r::HREF);
        let links = "<a href=\"#a\">y</a>".repeat(MAX_LINKS + 1);
        assert_eq!(no(&links), r::QUOTA);
    }

    #[test]
    fn depth_and_node_bounds_are_checked_conservatively() {
        assert_eq!(ok(&nested(31)).depth, 31);
        assert_eq!(ok(&nested(32)).depth, 32);
        for depth in [33, 127, 128, 129] {
            assert_eq!(no(&nested(depth)), r::DEPTH, "{depth}");
        }
        // A void element counts as one level beneath its parent.
        let void_at =
            |open: usize| format!("{}<br>{}", "<div>".repeat(open), "</div>".repeat(open));
        assert_eq!(ok(&void_at(31)).depth, 32);
        assert_eq!(no(&void_at(32)), r::DEPTH);
        // 2 * 99_999 + 1 = 199_999 nodes are admitted; 2 * 100_000 + 1 are not.
        assert_eq!(ok(&"<br>".repeat(99_999)).lt_count, 99_999);
        assert_eq!(no(&"<br>".repeat(100_000)), r::NODES);
        // The bound is applied before any tag is interpreted.
        assert_eq!(no(&"<".repeat(100_000)), r::NODES);
    }
}
