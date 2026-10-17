//! Private Xberg document boundary for the restricted helper.
//!
//! The first admitted family is standalone schema-1 HTML in the `html-textual-1` profile. The
//! flow is: strict byte admission (std-only scanner) → maintained html5ever tokenizer
//! corroboration → one pinned `xberg::extract` call on one bytes input → ordered content
//! integrity check → existing schema-1 finalization in `helper.rs`. No Xberg type crosses this
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
    // Every admitted element begins at its own `<`; the node bound is derived from that count.
    if admitted.elements > admitted.lt_count {
        return Err(refusal::CORROBORATION.into());
    }
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

/// Remove Markdown backslash escapes before ASCII punctuation for word matching.
fn unescape(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut chars = markdown.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek().is_some_and(char::is_ascii_punctuation) {
            continue;
        }
        out.push(c);
    }
    out
}

/// Every admitted word must appear, in source order, in the actual converted content, and
/// every admitted link target must appear. A truncated or content-dropping result refuses.
fn integrity(admitted: &Admitted, markdown: &str) -> Result<(), &'static str> {
    let plain = unescape(markdown);
    let mut cursor = 0usize;
    for word in html_admission::words(admitted) {
        let offset = plain[cursor..].find(word).ok_or(refusal::INTEGRITY)?;
        cursor += offset + word.len();
    }
    for href in html_admission::hrefs(admitted) {
        if !plain.contains(href) {
            return Err(refusal::INTEGRITY);
        }
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
            unescape(markdown).trim_end(),
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
}
