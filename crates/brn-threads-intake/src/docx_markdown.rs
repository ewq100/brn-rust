/// BetterOffice's raw comments name converter anchors/stories, not authored text.
/// Its authored text is escaped, so match only the exact emitted comment syntax.
/// Other HTML and comments remain literal; this is not an HTML sanitizer.
pub(crate) fn readable(markdown: &str) -> String {
    fn generated(comment: &str) -> bool {
        let Some(body) = comment
            .strip_prefix(' ')
            .and_then(|body| body.strip_suffix(' '))
        else {
            return false;
        };
        if let Some(number) = body.strip_prefix("docx-export:") {
            return !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit());
        }
        if let Some(story) = body.strip_prefix("docx-story: ")
            && let Some((name, kind)) = story.rsplit_once(' ')
        {
            return !name.is_empty()
                && !name.contains(['\n', '\r'])
                && matches!(
                    kind,
                    "body" | "header" | "footer" | "footnote" | "endnote" | "comment"
                );
        }
        false
    }

    let mut output = String::with_capacity(markdown.len());
    let mut remainder = markdown;
    while let Some(start) = remainder.find("<!--") {
        output.push_str(&remainder[..start]);
        remainder = &remainder[start..];
        let Some(end) = remainder.find("-->") else {
            break;
        };
        if !generated(&remainder[4..end]) {
            output.push_str(&remainder[..end + 3]);
        }
        remainder = &remainder[end + 3..];
    }
    output.push_str(remainder);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_only_generated_markers_inside_prose_and_table_cells() {
        let source = "<!-- docx-story: body body -->\n\n<!-- docx-export:0 -->\n# Harbor\n\n| <!-- docx-export:1 -->Role | <!-- docx-export:2 -->Decision |\n| --- | --- |\n| <!-- docx-export:3 -->Liis | <!-- docx-export:4 -->Go |\n\n<!-- docx-story: footnote:1 footnote -->\n[^fn-1]: retained reference\n";
        assert_eq!(
            readable(source),
            "\n\n\n# Harbor\n\n| Role | Decision |\n| --- | --- |\n| Liis | Go |\n\n\n[^fn-1]: retained reference\n"
        );
    }

    #[test]
    fn preserves_authored_and_malicious_html_comments_and_escaped_marker_text() {
        let authored = "<!-- owner comment -->\n<script>alert('keep literal')</script>\n<!-- docx-export:not-a-number -->\n<!-- docx-export:1 extra -->\n<!-- docx-story: body unknown -->\n\\<!-- docx-export:5 --\\>\n&lt;!-- docx-export:6 --&gt;\n<!-- unfinished";
        assert_eq!(readable(authored), authored);
    }
}
