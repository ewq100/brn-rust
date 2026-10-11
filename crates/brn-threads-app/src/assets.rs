use crate::{AppError, AppResult};

/// Replace complete managed URLs, never an ID prefix inside a different URL.
/// Unknown identities (including suffixes, paths, and queries) stay invalid.
pub(crate) fn rewrite(
    markdown: &str,
    mut resolve: impl FnMut(&str) -> Option<String>,
) -> AppResult<String> {
    const PREFIX: &str = "brn-asset://";
    let mut output = String::with_capacity(markdown.len());
    let mut remaining = markdown;
    while let Some(start) = remaining.find(PREFIX) {
        output.push_str(&remaining[..start]);
        let url = &remaining[start + PREFIX.len()..];
        let end = url
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | '>' | '<' | '\"' | '\'' | '`'))
            .unwrap_or(url.len());
        let identity = &url[..end];
        let replacement = resolve(identity).ok_or_else(|| {
            AppError::Invalid(format!("Retained figure identity is missing: {identity}"))
        })?;
        output.push_str(&replacement);
        remaining = &url[end..];
    }
    output.push_str(remaining);
    Ok(output)
}
