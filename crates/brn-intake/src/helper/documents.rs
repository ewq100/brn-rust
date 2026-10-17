//! Private Xberg document boundary for the restricted helper.
//!
//! The first admitted family is standalone schema-1 HTML in the `html-textual-1` profile. The
//! flow is: strict byte admission (std-only scanner) → maintained html5ever tokenizer
//! corroboration → one pinned `xberg::extract` call on one bytes input → ordered content
//! integrity check → existing schema-1 finalization in `helper.rs`. Inputs and decoded text are
//! bounded before the backend; converted output is bounded only after conversion (the 8 MiB
//! text ceiling and the protocol output budget), and memory is not hard-capped. No Xberg type crosses this
//! module; no URI/file/batch/cache/OCR/post-processing/child dispatch or model is reachable.
//! Unsupported constructs refuse before the backend. There is no fallback reader.
use super::{Budget, append, gap};
use crate::{Extraction, MAX_TEXT_BYTES};
use html5ever::tokenizer::{BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer};
use std::{
    future::Future,
    pin::pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

pub(super) mod html_admission;
use html_admission::{Admitted, ScanToken};

pub(super) const HTML_MEDIA: &str = "text/html";
/// Pinned upstream walk guard. Admission keeps source depth at most 32, far below it.
const XBERG_HTML_MAX_DEPTH: usize = 128;

mod refusal {
    pub const CORROBORATION: &str =
        "HTML admission: maintained tokenizer disagrees with the agreement scan";
    pub const BACKEND: &str = "HTML backend: maintained Xberg conversion failed";
    pub const ENVELOPE: &str = "HTML backend: unexpected result envelope (errors or result count)";
    pub const WARNING: &str =
        "HTML backend: upstream reported a processing warning; result not accepted";
    pub const UNEXPECTED: &str =
        "HTML backend: unexpected tables, images, children or URIs outside the admitted profile";
    pub const INTEGRITY: &str =
        "HTML backend: converted content does not retain every admitted word in order";
    pub const EXECUTOR: &str = "HTML backend: future did not complete on the local executor";
}

#[cfg(test)]
thread_local! {
    /// Test-only: output bytes consumed by integrity searches (match end minus search start,
    /// or the remaining text on failure). Not a CPU-comparison count.
    pub(super) static SEARCH_WINDOW: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
thread_local! {
    /// Count of actual `xberg::extract` calls on this thread (tests only).
    pub(super) static BACKEND_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Convert one admitted standalone HTML original into `extraction`.
pub(super) fn html(
    bytes: &[u8],
    extraction: &mut Extraction,
    budget: &mut Budget,
) -> Result<(), String> {
    let text_quota = budget
        .limits
        .max_expanded_bytes
        .saturating_sub(budget.expanded_bytes)
        .min(MAX_TEXT_BYTES);
    let admitted = html_admission::admit(bytes, text_quota)?;
    // admit() has already proven strict UTF-8.
    let source = std::str::from_utf8(bytes).map_err(|_| html_admission::refusal::UTF8)?;
    corroborate(source, &admitted)?;
    budget.expanded_bytes += admitted.text_bytes;
    let markdown = convert(bytes)?;
    integrity(&admitted, &markdown)?;
    append(extraction, &markdown)?;
    extraction.sources[0].text = markdown;
    gap(
        extraction,
        format!(
            "{}: HTML converted by maintained Xberg in the restricted {} profile (headings, paragraphs, lists, quotes, code, inert links); styling and layout are not retained and no linked resource was fetched",
            extraction.sources[0].id,
            html_admission::PROFILE
        ),
    )?;
    Ok(())
}

/// Exact Xberg configuration. Every safety-relevant default is pinned rather than inherited.
fn html_config() -> xberg::ExtractionConfig {
    let html_options = xberg::ConversionOptions {
        // Default preprocessing (Standard preset) silently drops nav/forms/header-like
        // content; the admitted profile needs none of it.
        preprocessing: xberg::extraction::html::PreprocessingOptions {
            enabled: false,
            remove_navigation: false,
            remove_forms: false,
            ..Default::default()
        },
        extract_metadata: false,
        max_input_size: Some(html_admission::MAX_HTML_BYTES as u64),
        max_depth: Some(XBERG_HTML_MAX_DEPTH),
        // Xberg forces these two on internally; set them explicitly so the pin is visible.
        include_document_structure: true,
        extract_images: true,
        skip_images: true,
        capture_svg: false,
        infer_dimensions: false,
        strip_tags: Vec::new(),
        preserve_tags: Vec::new(),
        exclude_selectors: Vec::new(),
        base_url: None,
        wrap: false,
        // Literal source text such as `*star*` or `_u_` must stay literal, not become Markdown
        // emphasis. The extraction-level `escape_markdown` does not reach pre-rendered HTML.
        escape_asterisks: true,
        escape_underscores: true,
        escape_misc: true,
        ..Default::default()
    };
    xberg::ExtractionConfig {
        use_cache: false,
        enable_quality_processing: false,
        extraction_timeout_secs: None,
        max_concurrent_extractions: Some(1),
        content_filter: None,
        output_format: xberg::OutputFormat::Markdown,
        html_options: Some(html_options),
        concurrency: Some(xberg::ConcurrencyConfig {
            max_threads: Some(1),
            max_concurrent_ocr: Some(1),
        }),
        postprocessor: Some(xberg::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        disable_ocr: true,
        ocr: None,
        force_ocr: false,
        images: None,
        captioning: None,
        qr_codes: None,
        chunking: None,
        language_detection: None,
        token_reduction: None,
        pages: None,
        include_document_structure: false,
        // No plain-text recovery after a structured parse failure.
        strict_parse: true,
        cache_namespace: None,
        url: xberg::UrlExtractionConfig {
            allow_local_file_inputs: false,
            allow_file_uris: false,
            ..Default::default()
        },
        ..xberg::ExtractionConfig::default()
    }
}

/// One real backend call on exactly one bytes input with exact `text/html`.
fn convert(bytes: &[u8]) -> Result<String, &'static str> {
    let config = html_config();
    // Copy 1 of the original; Xberg makes a second (UTF-8 String) and, at >=512 KiB, a third
    // for its joined 16 MiB conversion thread.
    let input = xberg::ExtractInput::from_bytes(bytes.to_vec(), HTML_MEDIA, None);
    #[cfg(test)]
    BACKEND_CALLS.with(|calls| calls.set(calls.get() + 1));
    let result = block_on(xberg::extract(input, &config))?.map_err(|_| refusal::BACKEND)?;
    if !result.errors.is_empty() || result.results.len() != 1 {
        return Err(refusal::ENVELOPE);
    }
    let document = result.results.into_iter().next().ok_or(refusal::ENVELOPE)?;
    if !document.processing_warnings.is_empty() {
        return Err(refusal::WARNING);
    }
    if !document.tables.is_empty()
        || document.images.as_ref().is_some_and(|i| !i.is_empty())
        || document.children.as_ref().is_some_and(|c| !c.is_empty())
        || !document.formulas.is_empty()
    {
        return Err(refusal::UNEXPECTED);
    }
    Ok(document.content)
}

/// Minimal std executor for Xberg's no-Tokio future. The calling thread polls and parks;
/// a missing wake is bounded by `park_timeout` and a poll ceiling, never a spin loop.
fn block_on<F: Future>(future: F) -> Result<F::Output, &'static str> {
    struct Unpark(std::thread::Thread);
    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    for _ in 0..16 {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return Ok(output),
            Poll::Pending => std::thread::park_timeout(std::time::Duration::from_secs(1)),
        }
    }
    Err(refusal::EXECUTOR)
}

#[derive(Debug, PartialEq, Eq)]
enum Corroborated {
    Start(String, Option<String>),
    End(String),
    Text(String),
}

#[derive(Default)]
struct Corroborator {
    tokens: Vec<Corroborated>,
    disagreement: bool,
    limit: usize,
}
impl TokenSink for Corroborator {
    type Handle = ();
    fn process_token(&mut self, token: Token, _: u64) -> TokenSinkResult<()> {
        if self.disagreement {
            return TokenSinkResult::Continue;
        }
        let next = match token {
            Token::TagToken(tag) => {
                if tag.self_closing {
                    self.disagreement = true;
                    return TokenSinkResult::Continue;
                }
                let name = tag.name.as_ref().to_owned();
                match tag.kind {
                    TagKind::StartTag => {
                        let href = match (name.as_str(), tag.attrs.as_slice()) {
                            (_, []) => None,
                            ("a", [attribute])
                                if attribute.name.local.as_ref() == "href"
                                    && attribute.name.prefix.is_none() =>
                            {
                                Some(attribute.value.to_string())
                            }
                            _ => {
                                self.disagreement = true;
                                return TokenSinkResult::Continue;
                            }
                        };
                        Corroborated::Start(name, href)
                    }
                    TagKind::EndTag => {
                        if !tag.attrs.is_empty() {
                            self.disagreement = true;
                            return TokenSinkResult::Continue;
                        }
                        Corroborated::End(name)
                    }
                }
            }
            Token::CharacterTokens(text) => {
                if let Some(Corroborated::Text(previous)) = self.tokens.last_mut() {
                    previous.push_str(&text);
                    return TokenSinkResult::Continue;
                }
                Corroborated::Text(text.to_string())
            }
            Token::EOFToken => return TokenSinkResult::Continue,
            // Any parse error, comment, doctype or NUL is a disagreement with the scan.
            _ => {
                self.disagreement = true;
                return TokenSinkResult::Continue;
            }
        };
        if self.tokens.len() >= self.limit {
            self.disagreement = true;
        } else {
            self.tokens.push(next);
        }
        TokenSinkResult::Continue
    }
}

/// The maintained WHATWG tokenizer must produce exactly the scanner's token sequence.
fn corroborate(source: &str, admitted: &Admitted) -> Result<(), &'static str> {
    let mut tokenizer = Tokenizer::new(
        Corroborator {
            limit: admitted.tokens.len(),
            ..Corroborator::default()
        },
        Default::default(),
    );
    let mut input = BufferQueue::default();
    input.push_back(source.into());
    let _ = tokenizer.feed(&mut input);
    tokenizer.end();
    let sink = tokenizer.sink;
    if sink.disagreement || sink.tokens.len() != admitted.tokens.len() {
        return Err(refusal::CORROBORATION);
    }
    let agrees = sink
        .tokens
        .iter()
        .zip(&admitted.tokens)
        .all(|pair| match pair {
            (Corroborated::Start(a, ah), ScanToken::Start { name, href }) => {
                a == name && ah == href
            }
            (Corroborated::End(a), ScanToken::End(name)) => a == name,
            (Corroborated::Text(a), ScanToken::Text(b)) => a == b,
            _ => false,
        });
    if agrees {
        Ok(())
    } else {
        Err(refusal::CORROBORATION)
    }
}

/// Symmetric normalization for integrity matching: every backslash is removed from both the
/// converted Markdown and the admitted words. Upstream escapes source backslashes in text
/// (`\\`) but not in code, so pairwise unescaping cannot be applied blindly; removing all
/// backslashes on both sides keeps the comparison correct for text and code alike.
fn strip_backslashes(text: &str) -> std::borrow::Cow<'_, str> {
    if text.contains('\\') {
        std::borrow::Cow::Owned(text.replace('\\', ""))
    } else {
        std::borrow::Cow::Borrowed(text)
    }
}

/// Find `needle` in `plain` at or after `*cursor` and advance the cursor past the match.
/// Cursors only move forward, so the total search window over one pass is linear in the
/// output length.
fn advance(plain: &str, cursor: &mut usize, needle: &str) -> Result<(), &'static str> {
    let found = plain[*cursor..].find(needle);
    #[cfg(test)]
    SEARCH_WINDOW.with(|w| {
        w.set(w.get() + found.map_or(plain.len() - *cursor, |offset| offset + needle.len()))
    });
    let offset = found.ok_or(refusal::INTEGRITY)?;
    *cursor += offset + needle.len();
    Ok(())
}

/// Truncation defense, not a fidelity proof: every admitted word must appear in source order
/// in the actual converted content, and every link target must appear in source order.
/// Words and links use separate monotonic cursors (a link target renders after its text).
fn integrity(admitted: &Admitted, markdown: &str) -> Result<(), &'static str> {
    let plain = strip_backslashes(markdown);
    let mut cursor = 0usize;
    for word in html_admission::words(admitted) {
        let word = strip_backslashes(word);
        if !word.is_empty() {
            advance(&plain, &mut cursor, &word)?;
        }
    }
    let mut cursor = 0usize;
    for href in html_admission::hrefs(admitted) {
        advance(&plain, &mut cursor, href)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HTML_CONVERTER, HelperRequest, IntakeLimits, digest};

    fn calls() -> usize {
        BACKEND_CALLS.with(std::cell::Cell::get)
    }
    fn run(html: &str) -> (Result<Extraction, String>, usize) {
        let before = calls();
        let result = super::super::extract(HelperRequest {
            limits: None,
            kind: "html".into(),
            bytes: html.as_bytes().to_vec(),
        });
        (result, calls() - before)
    }
    fn ordered(markdown: &str, facts: &[&str]) {
        let mut cursor = 0;
        for fact in facts {
            let offset = markdown[cursor..]
                .find(fact)
                .unwrap_or_else(|| panic!("missing ordered fact {fact:?} in {markdown:?}"));
            cursor += offset + fact.len();
        }
    }

    // ---- X2 review round 1 witnesses (B1/B2). Identical in the bbf107e3 before copy and in the
    // corrected candidate.
    fn window() -> usize {
        SEARCH_WINDOW.with(std::cell::Cell::get)
    }
    /// Test-only CommonMark backslash unescape for non-code text (`\` + ASCII punctuation).
    fn md_text(markdown: &str) -> String {
        let mut out = String::new();
        let mut chars = markdown.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' && chars.peek().is_some_and(char::is_ascii_punctuation) {
                out.push(chars.next().unwrap());
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn b1_many_links_after_large_text_keep_integrity_window_linear() {
        let text = "steady ".repeat(150_000);
        let links: String = (0..1500)
            .map(|i| format!("<a href=\"https://x.test/u{i}\">l{i}</a> "))
            .collect();
        let html = format!("<p>{text}</p><p>{links}</p>");
        let start = window();
        let (result, backend) = run(&html);
        let extraction = result.unwrap();
        assert_eq!(backend, 1);
        let consumed = window() - start;
        let output = extraction.markdown.len();
        assert!(
            consumed <= 2 * output,
            "integrity searches consumed {consumed} bytes for {output} output bytes"
        );
        for i in [0, 749, 1499] {
            assert!(
                extraction
                    .markdown
                    .contains(&format!("[l{i}](https://x.test/u{i})"))
            );
        }
    }

    #[test]
    fn b2_backslashes_in_text_and_code_are_admitted_and_literal() {
        let html = r#"<p>C:\x\*y a\*b regex\. two\\back</p><pre><code>printf("a\"b\\n");</code></pre><p>inline <code>s\'q</code></p>"#;
        let (result, backend) = run(html);
        assert_eq!(backend, 1);
        let markdown = result.unwrap().markdown;
        let first = markdown.lines().next().unwrap();
        assert_eq!(
            md_text(first),
            r"C:\x\*y a\*b regex\. two\\back",
            "{markdown:?}"
        );
        assert!(
            markdown.contains("\n```\nprintf(\"a\\\"b\\\\n\");\n```\n"),
            "{markdown:?}"
        );
        assert!(markdown.contains(r"`s\'q`"), "{markdown:?}");
    }

    const TEXTUAL: &str = concat!(
        "<html><body>\n<h1>Harbour report</h1>\n",
        "<p>Caf\u{e9} <em>na\u{ef}ve</em> <strong>\u{65e5}\u{672c}</strong> ",
        "<a href=\"https://example.test/a?b=1&amp;c=2\">source link</a>.</p>\n",
        "<h2>Findings</h2>\n<ul><li>first item</li><li>second item</li></ul>\n",
        "<ol><li>step one</li><li>step two</li></ol>\n",
        "<blockquote><p>quoted words</p></blockquote>\n",
        "<pre><code>let tide = 3;</code></pre>\n<p>end</p>\n</body></html>",
    );

    #[test]
    fn real_xberg_textual_html_through_schema1() {
        let (result, backend) = run(TEXTUAL);
        let extraction = result.unwrap();
        assert_eq!(backend, 1);
        extraction.validate().unwrap();
        assert_eq!(extraction.converter, HTML_CONVERTER);
        assert_eq!(extraction.schema, 1);
        assert_eq!(extraction.original_sha256, digest(TEXTUAL.as_bytes()));
        assert_eq!(extraction.sources.len(), 1);
        let root = &extraction.sources[0];
        assert_eq!(root.id, "source-0");
        assert_eq!(root.media_type, "text/html");
        assert_eq!(root.name, "original.html");
        assert_eq!(root.bytes, TEXTUAL.as_bytes());
        assert_eq!(root.text, extraction.markdown);
        assert!(extraction.assets.is_empty() && extraction.occurrences.is_empty());
        assert_eq!(extraction.limits, IntakeLimits::default());
        let usage = extraction.consumed.as_ref().unwrap();
        assert_eq!(usage.input_bytes, TEXTUAL.len());
        assert_eq!(usage.decoded_bytes, 0);
        assert!(usage.expanded_bytes > 0 && usage.output_bytes > 0);
        ordered(
            &extraction.markdown,
            &[
                "# Harbour report",
                "Caf\u{e9}",
                "na\u{ef}ve",
                "\u{65e5}\u{672c}",
                "[source link](https://example.test/a?b=1&c=2)",
                "## Findings",
                "- first item",
                "- second item",
                "1. step one",
                "2. step two",
                "> quoted words",
                "```",
                "let tide = 3;",
                "```",
                "end",
            ],
        );
        assert!(extraction.gaps[0].contains(html_admission::PROFILE));
    }

    #[test]
    fn entities_and_markdown_significant_text_stay_literal() {
        let html = "<p>1 &lt; 2 &amp; 3 &gt; 0 &quot;q&quot; *star* _u_ [x](y)</p>";
        let (result, backend) = run(html);
        let extraction = result.unwrap();
        assert_eq!(backend, 1);
        let markdown = &extraction.markdown;
        // Literal emphasis/link syntax from source text must not become Markdown structure.
        assert!(!markdown.contains(" *star* "), "{markdown:?}");
        assert!(!markdown.contains(" _u_ "), "{markdown:?}");
        assert!(!markdown.contains(" [x](y)"), "{markdown:?}");
        assert_eq!(
            md_text(markdown.trim_end()),
            "1 < 2 & 3 > 0 \"q\" *star* _u_ [x](y)"
        );
        // Code keeps its bytes without escapes.
        let (result, _) = run("<pre><code>a *b* _c_ &lt;d&gt;</code></pre>");
        let markdown = result.unwrap().markdown;
        assert!(markdown.contains("a *b* _c_ <d>"), "{markdown:?}");
    }

    #[test]
    fn unsupported_rich_profile_refuses_before_backend() {
        for html in [
            "<table><tr><td>x</td></tr></table>",
            "<p><img src=\"https://x.test/a.png\"></p>",
            "< img src=\"x\">",
            "< table><tr><td>x</td></tr></table>",
            "<p>a<p>b",
            "<ul><li>a<li>b</ul>",
            "<DIV>x</DIV>",
            "<div>x</div >",
            "<script>x</script>",
            "<!-- c --><p>x</p>",
            "<a href=\"data:text/html,x\">y</a>",
            "<svg></svg>",
            "<math></math>",
            "<p>a & b</p>",
        ] {
            let (result, backend) = run(html);
            let error = result.expect_err(html);
            assert!(error.starts_with("HTML admission:"), "{html}: {error}");
            assert_eq!(backend, 0, "{html}");
        }
    }

    #[test]
    fn depth_boundaries_refuse_before_backend_and_admitted_depth_keeps_all_text() {
        for depth in [31usize, 32] {
            let words: Vec<String> = (0..depth).map(|i| format!("w{i}")).collect();
            let html = format!(
                "{}{}",
                words
                    .iter()
                    .map(|w| format!("<div>{w} "))
                    .collect::<String>(),
                "</div>".repeat(depth)
            );
            let (result, backend) = run(&html);
            let extraction = result.unwrap_or_else(|e| panic!("{depth}: {e}"));
            assert_eq!(backend, 1, "{depth}");
            let refs: Vec<&str> = words.iter().map(String::as_str).collect();
            ordered(&extraction.markdown, &refs);
        }
        for depth in [33usize, 127, 128, 129] {
            let html = html_admission::tests_support::nested(depth);
            let (result, backend) = run(&html);
            assert_eq!(result.unwrap_err(), html_admission::refusal::DEPTH);
            assert_eq!(backend, 0, "{depth}");
        }
    }

    #[test]
    fn quotas_refuse_before_backend() {
        let before = calls();
        let mut budget = Budget {
            limits: IntakeLimits {
                max_expanded_bytes: 3,
                ..IntakeLimits::default()
            },
            ..Budget::default()
        };
        let mut extraction = Extraction {
            limits: budget.limits.clone(),
            consumed: None,
            schema: 1,
            converter: HTML_CONVERTER.into(),
            original_sha256: digest(b"<p>abcd</p>"),
            markdown: String::new(),
            sources: Vec::new(),
            assets: Vec::new(),
            occurrences: Vec::new(),
            gaps: Vec::new(),
        };
        let error = html(b"<p>abcd</p>", &mut extraction, &mut budget).unwrap_err();
        assert_eq!(error, html_admission::refusal::QUOTA);
        assert_eq!(calls(), before);
        let (result, backend) = run(&"&amp;".repeat(html_admission::MAX_ENTITIES + 1));
        assert_eq!(result.unwrap_err(), html_admission::refusal::QUOTA);
        assert_eq!(backend, 0);
        let (result, backend) = run(&"<br>".repeat(100_000));
        assert_eq!(result.unwrap_err(), html_admission::refusal::NODES);
        assert_eq!(backend, 0);
        let (result, backend) = run("<p>\u{7}</p>");
        assert_eq!(result.unwrap_err(), html_admission::refusal::CONTROL);
        assert_eq!(backend, 0);
    }

    #[test]
    fn large_input_uses_the_joined_upstream_thread_and_keeps_every_word() {
        let paragraph = "<p>tide gauge reading steady</p>\n";
        let count = 512 * 1024 / paragraph.len() + 64;
        let html = format!("{}<p>final marker</p>", paragraph.repeat(count));
        assert!(html.len() >= 512 * 1024);
        let (result, backend) = run(&html);
        let extraction = result.unwrap();
        assert_eq!(backend, 1);
        assert_eq!(
            extraction
                .markdown
                .matches("tide gauge reading steady")
                .count(),
            count
        );
        assert!(extraction.markdown.trim_end().ends_with("final marker"));
    }

    #[test]
    fn integrity_refuses_a_truncated_prefix() {
        let admitted = html_admission::admit(b"<p>alpha beta gamma</p>", MAX_TEXT_BYTES).unwrap();
        assert!(integrity(&admitted, "alpha beta gamma\n").is_ok());
        assert_eq!(
            integrity(&admitted, "alpha beta\n"),
            Err(refusal::INTEGRITY)
        );
        assert_eq!(
            integrity(&admitted, "gamma alpha beta\n"),
            Err(refusal::INTEGRITY)
        );
        assert!(integrity(&admitted, "alpha \\beta gamma").is_ok());
    }

    #[test]
    fn href_integrity_preserves_order_and_repeated_target_multiplicity() {
        let source = concat!(
            "<p><a href=\"https://x.test/a%20b?x=1&amp;y=(2)\">one</a> ",
            "<a href=\"#part\">two</a> ",
            "<a href=\"#part\">three</a></p>"
        );
        let (result, backend) = run(source);
        let markdown = result.unwrap().markdown;
        assert_eq!(backend, 1);
        ordered(
            &markdown,
            &[
                "[one](https://x.test/a%20b?x=1&y=(2))",
                "[two](#part)",
                "[three](#part)",
            ],
        );
        let admitted = html_admission::admit(source.as_bytes(), MAX_TEXT_BYTES).unwrap();
        // Keep all visible words but omit one target, or reverse the distinct target order.
        for damaged in [
            "one two three https://x.test/a%20b?x=1&y=(2) #part",
            "one two three #part #part https://x.test/a%20b?x=1&y=(2)",
        ] {
            assert_eq!(integrity(&admitted, damaged), Err(refusal::INTEGRITY));
        }
    }

    #[test]
    fn corroboration_matches_the_maintained_tokenizer() {
        let admitted = html_admission::admit(TEXTUAL.as_bytes(), MAX_TEXT_BYTES).unwrap();
        corroborate(TEXTUAL, &admitted).unwrap();
        // A forged scan result that disagrees with the tokenizer must refuse.
        let mut forged = html_admission::admit(b"<p>x</p>", MAX_TEXT_BYTES).unwrap();
        forged.tokens[1] = ScanToken::Text("y".into());
        assert_eq!(
            corroborate("<p>x</p>", &forged),
            Err(refusal::CORROBORATION)
        );
    }

    /// Depth/element facts of the parser Xberg actually runs (astral-tl, tier 2).
    fn tl_facts(html: &str) -> (usize, usize, Vec<String>) {
        fn walk(
            parser: &tl::Parser<'_>,
            handles: &[tl::NodeHandle],
            depth: usize,
            facts: &mut (usize, usize, Vec<String>),
        ) {
            for handle in handles {
                if let Some(tl::Node::Tag(tag)) = handle.get(parser) {
                    facts.0 = facts.0.max(depth + 1);
                    facts.1 += 1;
                    facts.2.push(tag.name().as_utf8_str().into_owned());
                    walk(parser, tag.children().top().as_slice(), depth + 1, facts);
                }
            }
        }
        let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
        let mut facts = (0, 0, Vec::new());
        walk(dom.parser(), dom.children(), 0, &mut facts);
        facts
    }

    #[test]
    fn differential_astral_tl_structure_equals_admitted_structure() {
        let mut admitted_cases = vec![
            TEXTUAL.to_owned(),
            "<div><p>a <em>b</em></p><ul><li><p>c</p><ol><li>d</li></ol></li></ul></div>".into(),
            "<p>one<br>two</p><hr><blockquote><blockquote><p>q</p></blockquote></blockquote>"
                .into(),
        ];
        admitted_cases.push(html_admission::tests_support::nested(32));
        for html in &admitted_cases {
            let admitted = html_admission::admit(html.as_bytes(), MAX_TEXT_BYTES).unwrap();
            let names: Vec<String> = admitted
                .tokens
                .iter()
                .filter_map(|t| match t {
                    ScanToken::Start { name, .. } => Some((*name).to_owned()),
                    _ => None,
                })
                .collect();
            let (depth, elements, tl_names) = tl_facts(html);
            assert_eq!(depth, admitted.depth, "{html}");
            assert!(depth < XBERG_HTML_MAX_DEPTH);
            assert_eq!(elements, admitted.elements, "{html}");
            assert_eq!(tl_names, names, "{html}");
            assert!(2 * admitted.lt_count + 1 >= elements);
        }
        // The refused forms are exactly where astral-tl and WHATWG parsing diverge: astral-tl
        // nests repeated `<p>`, turns `< img` into an element and never pops `</DIV>`.
        let (depth, _, _) = tl_facts("<p>a<p>b<p>c");
        assert_eq!(depth, 3);
        let (_, _, names) = tl_facts("< img src=\"x\">");
        assert!(names.iter().any(|n| n.contains("img")));
        for refused in ["<p>a<p>b<p>c", "< img src=\"x\">", "<div>x</DIV>"] {
            assert!(html_admission::admit(refused.as_bytes(), MAX_TEXT_BYTES).is_err());
        }
    }

    #[test]
    fn config_pins_every_reviewed_setting() {
        let config = html_config();
        assert!(!config.use_cache && !config.enable_quality_processing && config.strict_parse);
        assert_eq!(config.extraction_timeout_secs, None);
        assert_eq!(config.max_concurrent_extractions, Some(1));
        assert!(config.content_filter.is_none() && config.images.is_none());
        assert!(matches!(
            config.output_format,
            xberg::OutputFormat::Markdown
        ));
        assert_eq!(config.concurrency.as_ref().unwrap().max_threads, Some(1));
        assert!(!config.postprocessor.as_ref().unwrap().enabled);
        assert!(config.disable_ocr && config.ocr.is_none());
        assert!(!config.needs_image_data());
        let html = config.html_options.as_ref().unwrap();
        assert!(!html.preprocessing.enabled);
        assert_eq!(html.max_input_size, Some(8 * 1024 * 1024));
        assert_eq!(html.max_depth, Some(128));
        assert!(html.include_document_structure && !html.extract_metadata);
    }

    // ---- X2 review round 1: A5 executor, B4e tagless path, A6 literal text, B3 corpus. ----

    #[test]
    fn executor_parks_on_pending_and_completes_after_wake() {
        struct Once(bool);
        impl Future for Once {
            type Output = u8;
            fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u8> {
                if self.0 {
                    Poll::Ready(7)
                } else {
                    self.0 = true;
                    // Wake before returning Pending: the unpark token makes park return at once.
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }
        let started = std::time::Instant::now();
        assert_eq!(block_on(Once(false)), Ok(7));
        assert!(started.elapsed() < std::time::Duration::from_millis(900));
    }

    #[test]
    fn tagless_input_takes_the_upstream_text_path_and_is_integrity_checked() {
        let (result, backend) = run("plain words only &amp; more");
        let extraction = result.unwrap();
        assert_eq!(backend, 1);
        assert_eq!(
            md_text(extraction.markdown.trim_end()),
            "plain words only & more"
        );
    }

    #[test]
    fn literal_block_starts_backticks_links_and_nbsp_stay_literal() {
        for text in [
            "# not heading",
            "- not list",
            "+ not list",
            "1. not ordered",
            "> not quote",
            "``` not fence",
            "~~~ not fence",
            "a | b",
            "[x](y) not link",
        ] {
            let html = format!("<p>{}</p>", text.replace('>', "&gt;"));
            let (result, backend) = run(&html);
            let markdown = result.unwrap_or_else(|e| panic!("{text}: {e}")).markdown;
            assert_eq!(backend, 1);
            let line = markdown.trim_end();
            assert!(!line.contains('\n'), "{text}: {markdown:?}");
            assert_eq!(md_text(line), text, "{text}: {markdown:?}");
            assert_ne!(
                line, text,
                "{text}: markup-significant text was not escaped"
            );
        }
        let (result, _) = run("<pre><code>a ``` b ```` c</code></pre>");
        let markdown = result.unwrap().markdown;
        assert!(markdown.contains("\na ``` b ```` c\n"), "{markdown:?}");
        let fence = markdown.lines().next().unwrap();
        assert!(
            fence.len() >= 5 && fence.bytes().all(|b| b == b'`'),
            "{markdown:?}"
        );
        let (result, _) = run("<p>x <code>a`b</code> y</p>");
        let markdown = result.unwrap().markdown;
        assert!(markdown.contains("``a`b``"), "{markdown:?}");
        let (result, _) = run(concat!(
            "<p><a>bare anchor</a> <a href=\"mailto:ops@example.test\">mail</a> ",
            "<a href=\"#part-2\">jump</a> <a href=\"https://x.test/p(1)\">paren</a></p>"
        ));
        let markdown = result.unwrap().markdown;
        ordered(
            &markdown,
            &[
                "bare anchor",
                "[mail](mailto:ops@example.test)",
                "[jump](#part-2)",
                "[paren](",
                "https://x.test/p",
            ],
        );
        assert!(!markdown.contains("[bare anchor]"), "{markdown:?}");
        let (result, _) = run("<p>a&nbsp;b</p>");
        let markdown = result.unwrap().markdown;
        assert!(
            markdown.trim_end() == "a\u{a0}b" || markdown.trim_end() == "a b",
            "{markdown:?}"
        );
    }

    /// A small source tree whose structural facts are known by construction.
    enum Node {
        E(&'static str, Option<&'static str>, Vec<Node>),
        T(String),
    }
    fn e(name: &'static str, children: Vec<Node>) -> Node {
        Node::E(name, None, children)
    }
    fn t(text: impl Into<String>) -> Node {
        Node::T(text.into())
    }
    const VOID: [&str; 2] = ["br", "hr"];
    /// Known facts: preorder tags `(name, parent tag index, raw attributes)` and raw text
    /// nodes `(parent tag index, raw text)`.
    type Tags = Vec<(String, Option<usize>, Vec<(String, String)>)>;
    type Texts = Vec<(Option<usize>, String)>;
    fn render(
        nodes: &[Node],
        parent: Option<usize>,
        html: &mut String,
        tags: &mut Tags,
        texts: &mut Texts,
    ) {
        for node in nodes {
            match node {
                Node::T(text) => {
                    html.push_str(text);
                    texts.push((parent, text.clone()));
                }
                Node::E(name, href, children) => {
                    let index = tags.len();
                    let attrs = href
                        .map(|h| vec![("href".to_owned(), h.to_owned())])
                        .unwrap_or_default();
                    tags.push(((*name).to_owned(), parent, attrs));
                    match href {
                        Some(h) => html.push_str(&format!("<{name} href=\"{h}\">")),
                        None => html.push_str(&format!("<{name}>")),
                    }
                    if !VOID.contains(name) {
                        render(children, Some(index), html, tags, texts);
                        html.push_str(&format!("</{name}>"));
                    }
                }
            }
        }
    }
    /// The same facts read from the parser Xberg actually runs (astral-tl).
    fn tl_tree(html: &str) -> (Tags, Texts) {
        fn walk(
            parser: &tl::Parser<'_>,
            handles: &[tl::NodeHandle],
            parent: Option<usize>,
            tags: &mut Tags,
            texts: &mut Texts,
        ) {
            for handle in handles {
                match handle.get(parser) {
                    Some(tl::Node::Tag(tag)) => {
                        let index = tags.len();
                        let attrs = tag
                            .attributes()
                            .iter()
                            .map(|(k, v)| {
                                (
                                    k.into_owned(),
                                    v.map(|v| v.into_owned()).unwrap_or_default(),
                                )
                            })
                            .collect();
                        tags.push((tag.name().as_utf8_str().into_owned(), parent, attrs));
                        walk(
                            parser,
                            tag.children().top().as_slice(),
                            Some(index),
                            tags,
                            texts,
                        );
                    }
                    Some(tl::Node::Raw(raw)) => {
                        texts.push((parent, raw.as_utf8_str().into_owned()))
                    }
                    Some(tl::Node::Comment(_)) | None => {
                        panic!("unexpected tl node in admitted corpus")
                    }
                }
            }
        }
        let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
        let (mut tags, mut texts) = (Vec::new(), Vec::new());
        walk(dom.parser(), dom.children(), None, &mut tags, &mut texts);
        (tags, texts)
    }
    /// Independent statement of the html-textual-1 content models (parent -> allowed children).
    fn allowed(parent: &str, child: &str) -> bool {
        const BLOCKS: [&str; 15] = [
            "div",
            "section",
            "article",
            "blockquote",
            "ul",
            "ol",
            "p",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "pre",
            "hr",
        ];
        const INLINE: [&str; 7] = ["code", "a", "em", "strong", "b", "i", "br"];
        match parent {
            "html" => child == "body",
            "ul" | "ol" => child == "li",
            "body" | "div" | "section" | "article" | "blockquote" | "li" => {
                BLOCKS.contains(&child) || INLINE.contains(&child)
            }
            "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "em" | "strong" | "b" | "i" => {
                INLINE.contains(&child)
            }
            "a" => INLINE.contains(&child) && child != "a",
            "pre" => child == "code",
            _ => false,
        }
    }
    const NAMES: [&str; 25] = [
        "html",
        "body",
        "div",
        "section",
        "article",
        "blockquote",
        "li",
        "ul",
        "ol",
        "p",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "pre",
        "hr",
        "code",
        "a",
        "em",
        "strong",
        "b",
        "i",
        "br",
    ];
    fn sample(name: &'static str, word: &str) -> Node {
        match name {
            "br" | "hr" => e(name, vec![]),
            "ul" | "ol" => e(name, vec![e("li", vec![t(word)])]),
            "html" => e(name, vec![e("body", vec![t(word)])]),
            "pre" => e(name, vec![e("code", vec![t(word)])]),
            _ => e(name, vec![t(word)]),
        }
    }
    fn in_context(parent: &'static str, child: Node) -> Node {
        match parent {
            "li" => e("ul", vec![e("li", vec![child])]),
            "body" => e("html", vec![e("body", vec![child])]),
            _ => e(parent, vec![child]),
        }
    }
    /// Admission, the WHATWG tokenizer (inside `run`), astral-tl and real Xberg must agree.
    fn agree(nodes: Vec<Node>) {
        let (mut html, mut tags, mut texts) = (String::new(), Vec::new(), Vec::new());
        render(&nodes, None, &mut html, &mut tags, &mut texts);
        let admitted = html_admission::admit(html.as_bytes(), MAX_TEXT_BYTES)
            .unwrap_or_else(|e| panic!("{html:?}: {e}"));
        let scanned: Vec<&str> = admitted
            .tokens
            .iter()
            .filter_map(|t| match t {
                ScanToken::Start { name, .. } => Some(*name),
                _ => None,
            })
            .collect();
        let known: Vec<&str> = tags.iter().map(|(n, _, _)| n.as_str()).collect();
        assert_eq!(scanned, known, "{html:?}");
        let mut stack = Vec::new();
        let mut scan_parents = Vec::new();
        for token in &admitted.tokens {
            match token {
                ScanToken::Start { name, .. } => {
                    let index = scan_parents.len();
                    scan_parents.push(((*name).to_owned(), stack.last().copied()));
                    if !VOID.contains(name) {
                        stack.push(index);
                    }
                }
                ScanToken::End(_) => {
                    stack.pop().unwrap();
                }
                ScanToken::Text(_) => {}
            }
        }
        let known_parents: Vec<_> = tags.iter().map(|(n, p, _)| (n.clone(), *p)).collect();
        assert_eq!(scan_parents, known_parents, "scanner parents {html:?}");
        let (tl_tags, tl_texts) = tl_tree(&html);
        assert_eq!(tl_tags, tags, "tags/parents/attributes {html:?}");
        assert_eq!(tl_texts, texts, "raw text spans {html:?}");
        let (result, backend) = run(&html);
        result.unwrap_or_else(|e| panic!("{html:?}: {e}"));
        assert_eq!(backend, 1, "{html:?}");
    }

    #[test]
    fn grammar_corpus_matches_independent_models_tl_structure_and_real_xberg() {
        let mut admitted_pairs = 0;
        for parent in NAMES {
            for child in NAMES {
                if VOID.contains(&parent) || child == "html" {
                    continue;
                }
                let node = in_context(parent, sample(child, &format!("w{parent}{child}")));
                let (mut html, mut tags, mut texts) = (String::new(), Vec::new(), Vec::new());
                render(
                    std::slice::from_ref(&node),
                    None,
                    &mut html,
                    &mut tags,
                    &mut texts,
                );
                let scan = html_admission::admit(html.as_bytes(), MAX_TEXT_BYTES);
                assert_eq!(
                    scan.is_ok(),
                    allowed(parent, child),
                    "{parent} > {child}: {html:?}"
                );
                if scan.is_ok() {
                    admitted_pairs += 1;
                    agree(vec![node]);
                }
            }
        }
        assert_eq!(admitted_pairs, 6 * 22 + 1 + 2 + 11 * 7 + 6 + 1);
        // Every non-html/body/li element (and each void) is admitted at the root.
        for name in NAMES.iter().filter(|n| !["body", "li"].contains(n)) {
            agree(vec![sample(name, &format!("root{name}"))]);
        }
        // Entities: raw spans for astral-tl, independently known decoded scalars for the scan.
        for (raw, decoded) in [
            ("&amp;", "&"),
            ("&lt;", "<"),
            ("&gt;", ">"),
            ("&quot;", "\""),
            ("&apos;", "'"),
            ("&nbsp;", "\u{a0}"),
            ("&#65;", "A"),
            ("&#x2603;", "\u{2603}"),
            ("&#X1F600;", "\u{1f600}"),
        ] {
            let html_text = format!("x{raw}y");
            agree(vec![e("p", vec![t(html_text.clone())])]);
            let admitted =
                html_admission::admit(format!("<p>{html_text}</p>").as_bytes(), MAX_TEXT_BYTES)
                    .unwrap();
            assert_eq!(
                admitted.tokens[1],
                ScanToken::Text(format!("x{decoded}y")),
                "{raw}"
            );
        }
        // Attributes: href forms, raw value for astral-tl and decoded value for the scan.
        for (raw, decoded) in [
            ("https://x.test/a?b=1&amp;c=2", "https://x.test/a?b=1&c=2"),
            ("https://x.test/p(1)", "https://x.test/p(1)"),
            ("http://x.test/", "http://x.test/"),
            ("mailto:ops@example.test", "mailto:ops@example.test"),
            ("#part-2", "#part-2"),
        ] {
            agree(vec![e("p", vec![Node::E("a", Some(raw), vec![t("go")])])]);
            let admitted = html_admission::admit(
                format!("<p><a href=\"{raw}\">go</a></p>").as_bytes(),
                MAX_TEXT_BYTES,
            )
            .unwrap();
            assert_eq!(
                html_admission::hrefs(&admitted).collect::<Vec<_>>(),
                [decoded]
            );
        }
        agree(vec![e("p", vec![e("a", vec![t("no href")])])]);
        // Inter-element whitespace in html/ul/ol, CRLF text and a void at depth 32.
        agree(vec![e(
            "html",
            vec![
                t("\n "),
                e(
                    "body",
                    vec![
                        t("\n"),
                        e("ul", vec![t("\n "), e("li", vec![t("x")]), t("\n")]),
                        e("ol", vec![t("\t"), e("li", vec![t("y")])]),
                    ],
                ),
                t("\n"),
            ],
        )]);
        agree(vec![e("p", vec![t("first\r\nsecond")])]);
        let mut deep = e("br", vec![]);
        for _ in 0..31 {
            deep = e("div", vec![deep]);
        }
        agree(vec![e("p", vec![t("before")]), deep]);
    }

    #[test]
    fn astral_tl_divergence_witnesses_are_refused_before_backend() {
        // Repeated `li`: WHATWG closes the first item; astral-tl nests the second inside it.
        let (tags, _) = tl_tree("<ul><li>a<li>b</ul>");
        assert_eq!(tags[2].0, "li");
        assert_eq!(
            tags[2].1,
            Some(1),
            "astral-tl nests the second li inside the first"
        );
        // Raw-text element: WHATWG treats `<b>` inside `<script>` as text; astral-tl builds it.
        let (tags, _) = tl_tree("<p><script>x<b>y</b></script></p>");
        assert!(
            tags.iter().any(|(n, p, _)| n == "b" && *p == Some(1)),
            "{tags:?}"
        );
        for html in [
            "<ul><li>a<li>b</ul>",
            "<p><script>x<b>y</b></script></p>",
            "<p>a<p>b<p>c",
            "< img src=\"x\">",
            "<div>x</DIV>",
            "<html><body>a</body><body>b</body></html>",
        ] {
            let (result, backend) = run(html);
            assert!(result.unwrap_err().starts_with("HTML admission:"), "{html}");
            assert_eq!(backend, 0, "{html}");
        }
    }
}
