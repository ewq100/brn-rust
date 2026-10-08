//! Pure historical image markup reader support. No Word, XML, ZIP or conversion.
use crate::{Result, invalid};
pub(super) fn image_markdown(
    asset: &str,
    alt: Option<&str>,
    title: Option<&str>,
) -> Result<String> {
    let hash = asset
        .strip_prefix("brn-inbox-image-")
        .and_then(|v| v.strip_suffix("-1.png"))
        .ok_or_else(|| invalid("invalid legacy asset name"))?;
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || [alt, title].into_iter().flatten().any(|v| {
            v.len() > 2048
                || v.chars()
                    .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        })
    {
        return Err(invalid("invalid historical image markup"));
    }
    let mut out = String::from("![");
    for c in alt.unwrap_or("").chars() {
        match c {
            '\n' => out.push_str("<br>"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '.'
            | '!' | '|' | '~' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out.push_str("](");
    out.push_str(asset);
    if let Some(title) = title {
        out.push_str(" \"");
        for c in title.chars() {
            match c {
                '|' => out.push_str("&#124;"),
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\\' => out.push_str("&#92;"),
                '\n' => out.push_str("&#10;"),
                '\r' => out.push_str("&#13;"),
                '\t' => out.push_str("&#9;"),
                _ => out.push(c),
            }
        }
        out.push('"');
    }
    out.push(')');
    Ok(out)
}
