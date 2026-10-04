//! Narrow managed identity in ordinary Markdown frontmatter. Other bytes are opaque.
use crate::{MAX_NOTE_BYTES, Result, invalid};
use uuid::Uuid;

const BOM: &str = "\u{feff}";

struct Frontmatter {
    insertion: usize,
    newline: &'static str,
    id: Option<Uuid>,
}

fn horizontal(text: &str) -> &str {
    text.trim_matches([' ', '\t'])
}

fn key_token(text: &str, key: &str) -> bool {
    text.strip_prefix(key)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with([':', ' ', '\t', '#', '=']))
}

fn recognizable_key(line: &str, key: &str) -> bool {
    key_token(line, key)
        || ['\'', '"'].iter().any(|quote| {
            line.strip_prefix(*quote)
                .and_then(|line| line.strip_prefix(key))
                .is_some_and(|rest| rest.starts_with(*quote))
        })
}

/// Recognizable alternate key forms are refused, not interpreted as absence.
fn unsupported_key(line: &str, key: &str) -> bool {
    let line = line.trim_start_matches([' ', '\t']);
    recognizable_key(line, key)
        || ['?', '{', '-'].iter().any(|prefix| {
            line.strip_prefix(*prefix)
                .is_some_and(|rest| recognizable_key(horizontal(rest), key))
        })
}

fn scalar_value<'a>(value: &'a str, key: &str) -> Result<&'a str> {
    let value = value.trim_start_matches([' ', '\t']);
    let (value, rest) = if let Some(quote @ ('\'' | '"')) = value.chars().next() {
        let after = &value[1..];
        let end = after
            .find(quote)
            .ok_or_else(|| invalid(&format!("managed {key} scalar has an incomplete quote")))?;
        (&after[..end], &after[end + 1..])
    } else {
        let end = value.find([' ', '\t']).unwrap_or(value.len());
        (&value[..end], &value[end..])
    };
    if !rest.is_empty()
        && (!rest.starts_with([' ', '\t'])
            || !horizontal(rest).is_empty()
                && !rest.trim_start_matches([' ', '\t']).starts_with('#'))
    {
        return Err(invalid(&format!(
            "managed {key} scalar has ambiguous trailing text"
        )));
    }
    Ok(value)
}

fn identity_scalar(uuid: &str) -> Result<Uuid> {
    let id = Uuid::parse_str(uuid)
        .map_err(|_| invalid("managed brn_id must contain a canonical hyphenated UUID"))?;
    if uuid.len() != 36 || !id.hyphenated().to_string().eq_ignore_ascii_case(uuid) || id.is_nil() {
        return Err(invalid(
            "managed brn_id must contain a nonnil canonical hyphenated UUID",
        ));
    }
    Ok(id)
}

fn line_content(line: &str) -> &str {
    line.strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .unwrap_or(line)
}

/// This only identifies opaque indented content belonging to an ordinary root
/// metadata field. It does not interpret that field's YAML value.
fn ordinary_root_field(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, value)| {
        !horizontal(key).is_empty()
            && !key.starts_with(['#', '{', '[', '-', '?', '!', '&', '*'])
            && (value.is_empty() || value.starts_with([' ', '\t']))
    })
}

/// Only the selected ordinary root scalar fields are interpreted. All unrelated
/// metadata remains opaque; this is the shared managed-field walk, not a YAML parser.
pub(crate) struct ScalarFields<'a, const N: usize> {
    insertion: usize,
    newline: &'static str,
    pub(crate) values: [Option<&'a str>; N],
}

pub(crate) struct RawField<'a> {
    pub(crate) value: &'a str,
    pub(crate) line: std::ops::Range<usize>,
    pub(crate) newline: &'static str,
}

pub(crate) struct RawFields<'a, const N: usize> {
    pub(crate) insertion: usize,
    pub(crate) newline: &'static str,
    pub(crate) body_start: usize,
    pub(crate) fields: [Option<RawField<'a>>; N],
}

/// Locate selected ordinary root fields without interpreting their value syntax.
/// Line ranges include their original newline so callers can replace only that
/// managed line while leaving all unrelated bytes opaque.
pub(crate) fn raw_fields<'a, const N: usize>(
    text: &'a str,
    keys: [&str; N],
    strict_assignment: bool,
) -> Result<Option<RawFields<'a, N>>> {
    let (offset, text) = text
        .strip_prefix(BOM)
        .map_or((0, text), |text| (BOM.len(), text));
    let mut lines = text.split_inclusive('\n');
    let first = lines.next().unwrap_or("");
    let opening = line_content(first);
    if opening != "---" {
        if horizontal(opening) == "---"
            || ["--- ", "---\t", "---\r"]
                .iter()
                .any(|prefix| opening.starts_with(prefix))
        {
            return Err(invalid(
                "managed note frontmatter opening delimiter is unsupported",
            ));
        }
        return Ok(None);
    }
    if !first.ends_with('\n') {
        return if strict_assignment {
            Err(invalid("managed note frontmatter is incomplete"))
        } else {
            Ok(None)
        };
    }
    if !strict_assignment
        && matches!(
            frontmatter_candidate(text, &keys),
            FrontmatterCandidate::None
        )
    {
        // An unmanaged thematic break has no header boundary. Recognize that
        // before its ordinary Markdown body can resemble unsupported YAML.
        return Ok(None);
    }
    let newline = if first.ends_with("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut fields = std::array::from_fn(|_| None);
    let mut opaque_indented = false;
    let mut managed_scalar: Option<usize> = None;
    let mut cursor = offset + first.len();
    for line in lines {
        let start = cursor;
        cursor += line.len();
        let content = line_content(line);
        if content.starts_with([' ', '\t']) && opaque_indented {
            continue;
        }
        if let Some(index) = managed_scalar
            && content.starts_with([' ', '\t'])
            && !horizontal(content).is_empty()
            && !content.trim_start_matches([' ', '\t']).starts_with('#')
        {
            return Err(invalid(&format!(
                "managed {} cannot have an indented scalar continuation",
                keys[index]
            )));
        }
        if matches!(content, "---" | "...") {
            return Ok(Some(RawFields {
                insertion: offset + first.len(),
                newline,
                body_start: cursor,
                fields,
            }));
        }
        if ["---", "..."].iter().any(|delimiter| {
            content
                .strip_prefix(delimiter)
                .is_some_and(|rest| rest.starts_with([' ', '\t', '\r']))
        }) {
            return Err(invalid(
                "managed note frontmatter closing delimiter is unsupported",
            ));
        }
        if horizontal(content).starts_with(['{', '[']) {
            return Err(invalid(
                "managed note frontmatter does not support a root flow layout",
            ));
        }
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        if let Some((index, value)) = keys.iter().enumerate().find_map(|(index, key)| {
            content
                .strip_prefix(key)
                .and_then(|rest| rest.strip_prefix(':'))
                .map(|value| (index, value))
        }) {
            let key = keys[index];
            if fields[index].is_some() {
                return Err(invalid(&format!(
                    "managed note frontmatter contains duplicate {key} fields"
                )));
            }
            if !value.starts_with([' ', '\t']) {
                return Err(invalid(&format!(
                    "managed {key} needs whitespace after its field colon"
                )));
            }
            fields[index] = Some(RawField {
                value,
                line: start..cursor,
                newline: if line.ends_with("\r\n") { "\r\n" } else { "\n" },
            });
            opaque_indented = false;
            managed_scalar = Some(index);
        } else if let Some(key) = keys.iter().find(|key| unsupported_key(content, key)) {
            return Err(invalid(&format!(
                "managed {key} needs an unindented ordinary {key}: scalar field"
            )));
        } else if !content.starts_with([' ', '\t']) {
            opaque_indented = ordinary_root_field(content);
            if opaque_indented {
                managed_scalar = None;
            }
        }
    }
    if strict_assignment || fields.iter().any(Option::is_some) {
        Err(invalid("managed note frontmatter is incomplete"))
    } else {
        // A leading Markdown thematic break without managed metadata is ordinary
        // readable text. Assignment still refuses its ambiguous incomplete header.
        Ok(None)
    }
}

pub(crate) fn selected_fields<'a, const N: usize>(
    text: &'a str,
    keys: [&str; N],
    strict_assignment: bool,
) -> Result<Option<ScalarFields<'a, N>>> {
    raw_fields(text, keys, strict_assignment)?
        .map(|metadata| {
            let mut values = [None; N];
            for (index, field) in metadata.fields.iter().enumerate() {
                values[index] = field
                    .as_ref()
                    .map(|field| scalar_value(field.value, keys[index]))
                    .transpose()?;
            }
            Ok(ScalarFields {
                insertion: metadata.insertion,
                newline: metadata.newline,
                values,
            })
        })
        .transpose()
}

/// Only inspect possible root field positions in the frontmatter. This allows a
/// new optional field's reader to leave legacy unsupported layouts alone when
/// that field is absent, while its selected-field parser still refuses malformed
/// managed syntax. Nested/block values and body text are opaque.
pub(crate) fn has_field(text: &str, key: &str) -> bool {
    matches!(
        frontmatter_candidate(text, &[key]),
        FrontmatterCandidate::ManagedField
    )
}

enum FrontmatterCandidate {
    None,
    ManagedField,
    ClosingDelimiter,
}

/// This conservative lookahead reuses the same opaque root/indented distinction
/// for field presence and possible supported or malformed closing delimiters.
fn frontmatter_candidate(text: &str, keys: &[&str]) -> FrontmatterCandidate {
    let text = text.strip_prefix(BOM).unwrap_or(text);
    let mut lines = text.split_inclusive('\n');
    let first = horizontal(line_content(lines.next().unwrap_or("")));
    if !first
        .strip_prefix("---")
        .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t', '\r']))
    {
        return FrontmatterCandidate::None;
    }
    let mut opaque_indented = false;
    for line in lines {
        let content = line_content(line);
        if content.starts_with([' ', '\t']) && opaque_indented {
            continue;
        }
        if ["---", "..."].iter().any(|delimiter| {
            content
                .strip_prefix(delimiter)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t', '\r']))
        }) {
            return FrontmatterCandidate::ClosingDelimiter;
        }
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        if keys
            .iter()
            .any(|key| unsupported_key(content, key) || flow_has_field(horizontal(content), key))
        {
            return FrontmatterCandidate::ManagedField;
        }
        if !content.starts_with([' ', '\t']) {
            opaque_indented = ordinary_root_field(content);
        }
    }
    FrontmatterCandidate::None
}

fn flow_has_field(text: &str, key: &str) -> bool {
    if !text.starts_with(['{', '[']) {
        return false;
    }
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    let mut field_start = false;
    for (position, ch) in text.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if active_quote == '"' && ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if field_start && depth == 1 {
            if matches!(ch, ' ' | '\t') {
                continue;
            }
            if recognizable_key(&text[position..], key)
                || ch == '?' && recognizable_key(horizontal(&text[position + 1..]), key)
            {
                return true;
            }
            field_start = false;
        }
        match ch {
            '{' | '[' => {
                depth += 1;
                field_start = depth == 1;
            }
            '}' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 1 => field_start = true,
            '\'' | '"' => quote = Some(ch),
            _ => {}
        }
    }
    false
}

fn frontmatter(text: &str, strict_assignment: bool) -> Result<Option<Frontmatter>> {
    selected_fields(text, ["brn_id"], strict_assignment)?
        .map(|metadata| {
            Ok(Frontmatter {
                insertion: metadata.insertion,
                newline: metadata.newline,
                id: metadata.values[0].map(identity_scalar).transpose()?,
            })
        })
        .transpose()
}

/// Reads only the documented managed field; unrelated metadata and body are opaque.
pub fn read(text: &str) -> Result<Option<Uuid>> {
    Ok(frontmatter(text, false)?.and_then(|metadata| metadata.id))
}

/// Return the exact saved body byte offset after supported leading frontmatter,
/// or after only a leading BOM when no complete frontmatter is recognized.
/// Managed field layouts are checked; their values retain separate readers.
pub fn body_start(text: &str) -> Result<usize> {
    if text.len() > MAX_NOTE_BYTES {
        return Err(invalid("note body inspection exceeds the 1 MiB note limit"));
    }
    Ok(raw_fields(
        text,
        ["brn_id", "brn_kind", "brn_state", "brn_provenance"],
        false,
    )?
    .map_or_else(
        || if text.starts_with(BOM) { BOM.len() } else { 0 },
        |metadata| metadata.body_start,
    ))
}

/// Returns complete proposed bytes. It never writes a note or mints an identity.
pub fn assign(text: &str, id: Uuid) -> Result<String> {
    if id.is_nil() || text.len() > MAX_NOTE_BYTES {
        return Err(invalid(
            "identity assignment needs a nonnil UUID and a bounded note",
        ));
    }
    let (insertion, addition) = if let Some(metadata) = frontmatter(text, true)? {
        if let Some(existing) = metadata.id {
            if existing != id {
                return Err(invalid("managed note already has a different brn_id"));
            }
            return Ok(text.to_owned());
        }
        (
            metadata.insertion,
            format!("brn_id: {id}{}", metadata.newline),
        )
    } else {
        let newline = text.find('\n').map_or("\n", |position| {
            if text[..position].ends_with('\r') {
                "\r\n"
            } else {
                "\n"
            }
        });
        (
            if text.starts_with(BOM) { BOM.len() } else { 0 },
            format!("---{newline}brn_id: {id}{newline}---{newline}"),
        )
    };
    let size = text
        .len()
        .checked_add(addition.len())
        .filter(|size| *size <= MAX_NOTE_BYTES)
        .ok_or_else(|| invalid("identity assignment exceeds the 1 MiB note limit"))?;
    let mut assigned = String::with_capacity(size);
    assigned.push_str(&text[..insertion]);
    assigned.push_str(&addition);
    assigned.push_str(&text[insertion..]);
    Ok(assigned)
}

/// Full review edits preserve an established proposed identity. Exact legacy bytes
/// remain allowed, including records predating this managed format.
pub fn protect(current: &str, edited: &str) -> Result<()> {
    if current == edited {
        return Ok(());
    }
    let before = read(current)?;
    let after = read(edited)?;
    if before.is_some() && before != after {
        return Err(invalid(
            "full proposal edit must preserve its established brn_id",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MAX_NOTE_BYTES;

    const ID: &str = "9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d";

    fn id() -> uuid::Uuid {
        uuid::Uuid::parse_str(ID).unwrap()
    }

    #[test]
    fn assignment_preserves_bom_unicode_metadata_body_and_line_endings() {
        for newline in ["\n", "\r\n"] {
            for close in ["---", "..."] {
                let original = format!(
                    "\u{feff}---{newline}custom: '日本語 🦀'{newline}opaque: [a, b]{newline}literal: |{newline}  ---{newline}  ...{newline}{close}{newline}# Body λ{newline}brn_id: literal body text{newline}"
                );
                assert_eq!(read(&original).unwrap(), None);
                let assigned = assign(&original, id()).unwrap();
                let insertion = format!("brn_id: {ID}{newline}");
                assert_eq!(
                    assigned,
                    original.replacen(
                        &format!("---{newline}"),
                        &format!("---{newline}{insertion}"),
                        1
                    )
                );
                assert_eq!(read(&assigned).unwrap(), Some(id()));
                assert_eq!(assign(&assigned, id()).unwrap(), assigned);
            }
        }
        let original = "\u{feff}# 日本語\r\nbody 🦀\r\n";
        assert_eq!(
            assign(original, id()).unwrap(),
            format!("\u{feff}---\r\nbrn_id: {ID}\r\n---\r\n# 日本語\r\nbody 🦀\r\n")
        );
        assert_eq!(
            assign("", id()).unwrap(),
            format!("---\nbrn_id: {ID}\n---\n")
        );
        let no_final_newline = "---\ncustom: exact\n...";
        assert_eq!(
            assign(no_final_newline, id()).unwrap(),
            format!("---\nbrn_id: {ID}\ncustom: exact\n...")
        );
    }

    #[test]
    fn supported_scalars_comments_and_opaque_body_read_without_normalization() {
        for scalar in [
            ID.to_owned(),
            format!("'{ID}'"),
            format!("\"{ID}\""),
            format!("{ID} # retained comment"),
            format!("'{ID}'\t# retained comment 🦀"),
            format!("\"{}\"", ID.to_uppercase()),
        ] {
            let text =
                format!("---\ncustom: 'brn_id: nope'\nbrn_id: {scalar}\n---\nbrn_id: not metadata");
            assert_eq!(read(&text).unwrap(), Some(id()), "{scalar}");
            assert_eq!(assign(&text, id()).unwrap(), text, "{scalar}");
        }
        assert_eq!(read("brn_id: not metadata\n").unwrap(), None);
        assert_eq!(read("---\n# brn_id: invalid\n---\n").unwrap(), None);
    }

    #[test]
    fn ambiguous_and_noncanonical_managed_metadata_never_gets_assigned() {
        for metadata in [
            "brn_id:",
            "brn_id: invalid",
            "brn_id: 00000000-0000-0000-0000-000000000000",
            "brn_id: 9ba6f3d86a654fd4b8b647dc3185657d",
            "brn_id: {9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d}",
            "brn_id: urn:uuid:9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d",
            "brn_id: '9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d",
            "brn_id: 9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d extra",
            "brn_id: 9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d#ambiguous",
            "brn_id: \"9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d\"#ambiguous",
            "brn_id: >\n  9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d",
            "brn_id: !!str 9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d",
            "brn_id:9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d",
            "brn_id : 9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d",
            "'brn_id': 9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d",
            " brn_id: 9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d",
            "{\"brn_id\": 9ba6f3d8-6a65-4fd4-b8b6-47dc3185657d}",
            "brn_id",
        ] {
            let text = format!("---\n{metadata}\n---\nbody\n");
            assert!(read(&text).is_err(), "{metadata}");
            assert!(assign(&text, id()).is_err(), "{metadata}");
        }
        let duplicate = format!("---\nbrn_id: {ID}\nbrn_id: '{ID}'\n---\n");
        assert!(read(&duplicate).is_err());
        assert!(assign(&duplicate, id()).is_err());
        for incomplete in [
            "--- \ncustom: metadata\n---\n",
            "---\ncustom: metadata\n--- # incomplete close\nbrn_id: invalid body\n---\n",
        ] {
            assert!(read(incomplete).is_err());
            assert!(assign(incomplete, id()).is_err());
        }
    }

    #[test]
    fn opaque_indented_metadata_never_becomes_a_managed_root_field() {
        for metadata in [
            "summary: |\n  brn_id: literal text\n  ---\n  ...\n  'brn_id': quoted literal",
            "nested:\n  brn_id: unrelated nested value\n  child: opaque",
            "'custom': >-\n  brn_id: folded literal text\n\n  continued",
            "custom: {title: hi, brn_id: unrelated nested scalar}",
        ] {
            let original = format!("---\n{metadata}\n---\nBody 日本語 λ\n");
            assert_eq!(read(&original).unwrap(), None, "{metadata}");
            let assigned = assign(&original, id()).unwrap();
            assert_eq!(
                assigned,
                format!("---\nbrn_id: {ID}\n{metadata}\n---\nBody 日本語 λ\n")
            );
            assert_eq!(read(&assigned).unwrap(), Some(id()));
            protect(&assigned, &assigned.replace("Body", "Reviewed body")).unwrap();
        }
    }

    #[test]
    fn read_keeps_incomplete_raw_markdown_while_assignment_refuses_ambiguity() {
        for raw in [
            "---",
            "---\n",
            "---\n# Work\n",
            "\u{feff}---\r\ncustom: retained\r\n",
        ] {
            assert_eq!(read(raw).unwrap(), None);
            protect(raw, &format!("{raw}more ordinary text\n")).unwrap();
            assert!(assign(raw, id()).is_err());
        }
        let incomplete_managed = format!("---\nbrn_id: {ID}\n# Work\n");
        assert!(read(&incomplete_managed).is_err());
        assert!(assign(&incomplete_managed, id()).is_err());
        protect(&incomplete_managed, &incomplete_managed).unwrap();
        assert!(protect(&incomplete_managed, "changed text").is_err());
    }

    #[test]
    fn root_flow_layout_is_refused_even_when_managed_key_is_later() {
        for mapping in [
            format!("{{title: hi, brn_id: {ID}}}"),
            format!("{{brn_id: {ID}, title: hi}}"),
            format!("  {{title: hi, 'brn_id': {ID}}}"),
            "{title: hi}".into(),
        ] {
            let text = format!("---\n{mapping}\n---\nBody\n");
            assert!(read(&text).is_err(), "{mapping}");
            assert!(assign(&text, id()).is_err(), "{mapping}");
            protect(&text, &text).unwrap();
        }
    }

    #[test]
    fn managed_scalar_refuses_continuation_but_keeps_comments_and_unrelated_blocks() {
        for scalar in [ID.to_owned(), format!("'{ID}'"), format!("\"{ID}\"")] {
            let continued = format!(
                "---\nbrn_id: {scalar}\n  # allowed comment\n \t\n  extra scalar text\n---\nBody\n"
            );
            assert!(read(&continued).is_err(), "{scalar}");
            assert!(assign(&continued, id()).is_err(), "{scalar}");
            protect(&continued, &continued).unwrap();
            assert!(protect(&continued, &continued.replace("Body", "Changed body")).is_err());
            let valid = format!(
                "---\nbrn_id: {scalar}\n  # allowed comment\n \t\nsummary: |\n  brn_id: unrelated literal text\n  ---\nnested:\n  child: preserved metadata\n---\nBody\n"
            );
            assert_eq!(read(&valid).unwrap(), Some(id()));
            assert_eq!(assign(&valid, id()).unwrap(), valid);
            protect(&valid, &valid.replace("Body", "Changed body")).unwrap();
        }
    }

    #[test]
    fn exact_legacy_text_stays_editable_but_recognized_identity_cannot_change() {
        let malformed = "---\nbrn_id: old invalid value\n---\nbody\n";
        protect(malformed, malformed).unwrap();
        assert!(protect(malformed, "new text").is_err());
        let current = assign("body λ\n", id()).unwrap();
        let changed = current.replace("body λ", "reviewed body 🦀");
        protect(&current, &changed).unwrap();
        assert!(protect(&current, "body λ\n").is_err());
        assert!(
            protect(
                &current,
                &current.replace(ID, &uuid::Uuid::new_v4().to_string())
            )
            .is_err()
        );
        protect("body λ\n", &current).unwrap();
        assert!(protect("body λ\n", malformed).is_err());
        assert!(assign(&current, uuid::Uuid::new_v4()).is_err());
        assert!(assign("body", uuid::Uuid::nil()).is_err());
    }

    #[test]
    fn assignment_checks_final_bytes_including_metadata_before_allocating_result() {
        let header = format!("---\nbrn_id: {ID}\n---\n");
        let at_limit = "a".repeat(MAX_NOTE_BYTES - header.len());
        let assigned = assign(&at_limit, id()).unwrap();
        assert_eq!(assigned.len(), MAX_NOTE_BYTES);
        assert_eq!(assign(&assigned, id()).unwrap(), assigned);
        assert!(assign(&(at_limit + "λ"), id()).is_err());
        assert!(assign(&"a".repeat(MAX_NOTE_BYTES + 1), id()).is_err());
    }
}
