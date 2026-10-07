//! H4 evaluator: can a published DOCX reader replace part of BRN's bounded
//! converter behind a small admission/mapping adapter? Every fixture is
//! synthetic. Each (case, reader) pair runs in its own child process, so a
//! reader abort or hang is recorded instead of ending the run. Every verdict is
//! compared with the recorded expectation; any difference exits 1.
use std::alloc::{GlobalAlloc, Layout, System};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};

mod fixtures;
mod readers;

use readers::{Model, READERS};

// ---------------------------------------------------------------- allocation

struct Counting;
static TRACK: AtomicBool = AtomicBool::new(false);
static LARGEST: AtomicUsize = AtomicUsize::new(0);
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn note(size: usize) {
    if TRACK.load(Relaxed) {
        LARGEST.fetch_max(size, Relaxed);
        let now = CURRENT.fetch_add(size, Relaxed) + size;
        PEAK.fetch_max(now, Relaxed);
    }
}
fn forget(size: usize) {
    if TRACK.load(Relaxed) {
        let _ = CURRENT.fetch_update(Relaxed, Relaxed, |c| Some(c.saturating_sub(size)));
    }
}
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        forget(layout.size());
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        forget(layout.size());
        note(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
#[global_allocator]
static GLOBAL: Counting = Counting;

// ------------------------------------------------------------------- reading

enum Read {
    Ok(Model),
    Err(String),
    Panic(String),
}
struct Measured {
    read: Read,
    largest: usize,
    peak: usize,
    millis: u128,
}
fn measure(reader: &str, bytes: &[u8]) -> Measured {
    LARGEST.store(0, Relaxed);
    CURRENT.store(0, Relaxed);
    PEAK.store(0, Relaxed);
    // Silence only the reader's own panic; evaluator panics stay visible.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let start = Instant::now();
    TRACK.store(true, Relaxed);
    let parsed = catch_unwind(AssertUnwindSafe(|| readers::parse(reader, bytes)));
    TRACK.store(false, Relaxed);
    let millis = start.elapsed().as_millis();
    // Building the inspection model (rendering, serialization) is not timed,
    // but a panic there is still the reader's.
    let result = match parsed {
        Ok(Ok(parsed)) => catch_unwind(AssertUnwindSafe(|| readers::model(parsed))),
        Ok(Err(e)) => Ok(Err(e)),
        Err(payload) => Err(payload),
    };
    std::panic::set_hook(hook);
    let read = match result {
        Ok(Ok(model)) => Read::Ok(model),
        Ok(Err(e)) => Read::Err(e),
        Err(payload) => Read::Panic(
            payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default(),
        ),
    };
    Measured {
        read,
        largest: LARGEST.load(Relaxed),
        peak: PEAK.load(Relaxed),
        millis,
    }
}

// ------------------------------------------------------------------ verdicts

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// Content/asset retained in the reader's output.
    Retained,
    /// Reader returned an error (an explicit refusal BRN could map).
    Refused,
    /// Read succeeded; meaningful content is absent and nothing in the output
    /// (no trace, no warning) shows that it was dropped.
    SilentLoss,
    /// Read succeeded and produced wrong or arbitrary content without
    /// signalling it (foreign namespace read as text, one alias chosen,
    /// corrupt bytes, an unresolved entity kept as literal text).
    Misread,
    /// Read succeeded on input BRN's admission refuses, and the content is
    /// right (UTF-16, backslash name, declared size that disagrees with data).
    Lenient,
    /// Wording survives but its structure/semantics are gone unobservably.
    Flattened,
    /// Asset/occurrence mapping cannot be resolved from the output alone.
    Ambiguous,
    /// Content is missing but a trace, a warning or an unmodelled-item report
    /// remains (adapter can refuse).
    DetectableGap,
    /// Reader panicked (caught).
    Panic,
    /// Allocation driven by an advertised, untrusted size.
    AdvertisedAllocation,
    /// Child process died (abort, signal) or exceeded the time limit.
    Abort,
}
use Verdict::*;

enum Check {
    /// All `wanted` must appear; a missing one with a `trace` id or a warning
    /// is a DetectableGap.
    Text(&'static [&'static str], &'static [&'static str]),
    /// Success with any `wanted` alternative present means input BRN
    /// refuses was accepted: Lenient when `true` (content right), otherwise
    /// Misread. A warning makes it a DetectableGap.
    Accepts(&'static [&'static str], bool),
    /// `wanted` present but none of the `structure` alternatives: Flattened.
    /// The structure needles double as traces when wording is missing.
    Structure(&'static [&'static str], &'static [&'static str]),
    Picture,
    TwoImages,
    SharedImage,
    Jpeg,
    SameRid,
    MissingImage,
    Advertised,
    Timed,
}

struct Case {
    id: &'static str,
    group: &'static str,
    title: &'static str,
    /// BRN's converter at baseline 450eaa2 on the identical bytes.
    brn: &'static str,
    check: Check,
    /// Recorded verdicts in `READERS` order.
    expected: [Verdict; 4],
}

fn cases() -> Vec<Case> {
    use Check::*;
    let c = |id, group, title, brn, check, expected| Case {
        id,
        group,
        title,
        brn,
        check,
        expected,
    };
    vec![
        c(
            "S0",
            "supported",
            "Minimal package, no word/_rels/document.xml.rels",
            "Ok DocxTextV1",
            Text(&["Minimal õ"], &[]),
            [Refused, Retained, Retained, Retained],
        ),
        c(
            "S1a",
            "supported",
            "Unicode paragraphs, Stored",
            "Ok DocxTextV1 (exact)",
            Text(&["First õ 日本語", "Second preserved"], &[]),
            [Retained, Retained, Retained, Retained],
        ),
        c(
            "S1b",
            "supported",
            "Unicode paragraphs, Deflate",
            "Ok DocxTextV1 (exact)",
            Text(&["First õ 日本語", "Second preserved"], &[]),
            [Retained, Retained, Retained, Retained],
        ),
        c(
            "S2",
            "supported",
            "Heading style, numbered list, external link, simple table",
            "Ok DocxTextV1 (exact)",
            Text(
                &[
                    "Heading õ",
                    "First item",
                    "Reference",
                    "https://example.invalid/a?q=one&b=two",
                    "left cell",
                    "right cell",
                ],
                &[],
            ),
            [Retained, Retained, Retained, Retained],
        ),
        c(
            "S3",
            "supported",
            "One inline PNG: exact bytes, alt text and title",
            "Ok DocxInlinePngV1 (bytes, position, alt, title)",
            Picture,
            [SilentLoss, SilentLoss, SilentLoss, Retained],
        ),
        c(
            "S4",
            "supported",
            "External hyperlink with a tooltip",
            "Ok DocxTextV1 (tooltip as link title)",
            Text(
                &[
                    "Linked words",
                    "https://example.invalid/tip",
                    "TOOLTIP-SENTINEL",
                ],
                &[],
            ),
            [SilentLoss, Retained, Retained, Retained],
        ),
        c(
            "M1",
            "meaningful",
            "Final-section default header",
            "docx_unsupported",
            Text(&["Body", "HEADER-SENTINEL"], &[]),
            [Retained, Retained, Retained, Retained],
        ),
        c(
            "M2",
            "meaningful",
            "Header referenced only by an earlier section",
            "docx_unsupported",
            Text(&["Section one", "EARLY-HEADER-SENTINEL"], &["rIdEarlyHdr7"]),
            [DetectableGap, Retained, Retained, Retained],
        ),
        c(
            "M3",
            "meaningful",
            "Final-section default footer",
            "docx_unsupported",
            Text(&["Body", "FOOTER-SENTINEL"], &[]),
            [Retained, Retained, Retained, Retained],
        ),
        c(
            "M4",
            "meaningful",
            "Footnote reference (same run as text) and footnotes part",
            "docx_unsupported",
            Text(&["Claim", "FOOTNOTE-SENTINEL"], &["7731"]),
            [SilentLoss, Retained, Retained, Retained],
        ),
        c(
            "M5",
            "meaningful",
            "Comment range, reference and comments part",
            "docx_unsupported",
            Text(&["Commented", "COMMENT-SENTINEL"], &[]),
            [Retained, Retained, Retained, Retained],
        ),
        c(
            "M6",
            "meaningful",
            "Tracked insertion and deletion",
            "docx_unsupported",
            Text(&["INSERTED-SENTINEL", "DELETED-SENTINEL"], &["ReviserZ9"]),
            [Retained, DetectableGap, SilentLoss, Retained],
        ),
        c(
            "M7",
            "meaningful",
            "Second prefix bound to WordprocessingML inside a `w:` run",
            "Ok DocxTextV1 (both wordings)",
            Text(&["Visible ", "PREFIX-SENTINEL"], &[]),
            [SilentLoss, Retained, Retained, Retained],
        ),
        c(
            "M8",
            "meaningful",
            "WordprocessingML as the default namespace",
            "Ok DocxTextV1 (wording kept)",
            Text(&["DEFAULT-NS-SENTINEL"], &[]),
            [SilentLoss, Retained, Retained, Retained],
        ),
        c(
            "M9",
            "meaningful",
            "`w` prefix bound to a non-WordprocessingML namespace",
            "docx_unsupported",
            Accepts(&["FOREIGN-SENTINEL"], false),
            [Misread, Refused, Refused, Misread],
        ),
        c(
            "M10",
            "meaningful",
            "Body-level altChunk imported part",
            "docx_unsupported",
            Text(&["Before chunk", "ALTCHUNK-SENTINEL"], &["rIdAltChunk9"]),
            [SilentLoss, DetectableGap, Retained, DetectableGap],
        ),
        c(
            "M11",
            "meaningful",
            "Legacy VML text box (w:pict/v:shape/v:textbox)",
            "docx_unsupported",
            Text(&["Anchor", "VML-SENTINEL"], &["VmlShape42", "width:101pt"]),
            [DetectableGap, Retained, Retained, SilentLoss],
        ),
        c(
            "M12",
            "meaningful",
            "Ruby annotation (base and phonetic text)",
            "docx_unsupported",
            Structure(
                &["RUBY-TEXT", "RUBY-BASE", "Tail"],
                &["<ruby", "\"ruby\"", "Ruby"],
            ),
            [Flattened, DetectableGap, SilentLoss, SilentLoss],
        ),
        c(
            "M13",
            "meaningful",
            "Simple HYPERLINK field: destination vs cached result",
            "docx_unsupported",
            Text(&["FIELD-RESULT", "field.invalid"], &[]),
            [SilentLoss, Retained, Retained, Retained],
        ),
        c(
            "M14",
            "meaningful",
            "Inline customXml wrapper around a run",
            "docx_unsupported",
            Structure(&["CUSTOMXML-WORDING"], &["urn:example:tags", "secret"]),
            [Flattened, DetectableGap, Flattened, Flattened],
        ),
        c(
            "M15",
            "meaningful",
            "Unknown WordprocessingML element wrapping a run",
            "docx_unsupported",
            Structure(&["Seen", "UNKNOWN-WRAPPED"], &["unknownWrapper"]),
            [Flattened, DetectableGap, SilentLoss, SilentLoss],
        ),
        c(
            "B1",
            "budget",
            "Main part advertising 3.75 GiB (0xF0000000) uncompressed",
            "docx_limit",
            Advertised,
            [AdvertisedAllocation, Refused, Refused, Lenient],
        ),
        c(
            "B2",
            "budget",
            "Stored main part with a corrupted byte (CRC mismatch)",
            "docx_invalid",
            Accepts(&["XRC-SENTINEL"], false),
            [Panic, Refused, DetectableGap, Refused],
        ),
        c(
            "B3",
            "budget",
            "Main part stored under a backslash name",
            "docx_invalid",
            Accepts(&["BACKSLASH-SENTINEL"], true),
            [Refused, Refused, Lenient, Refused],
        ),
        c(
            "B4",
            "budget",
            "Case-alias second main part (word/ vs WORD/)",
            "docx_invalid",
            Accepts(&["LOWER-SENTINEL", "UPPER-SENTINEL"], false),
            [Misread, Refused, Refused, Refused],
        ),
        c(
            "B5",
            "budget",
            "Internal DTD entity in document text",
            "docx_unsupported",
            Accepts(&["AENTITY-SENTINELB", "A&s;B"], false),
            [Refused, Misread, Misread, Refused],
        ),
        c(
            "B6",
            "budget",
            "UTF-16 encoded main part",
            "docx_unsupported",
            Accepts(&["UTF16-SENTINEL"], true),
            [Refused, Refused, Lenient, Refused],
        ),
        c(
            "B7",
            "budget",
            "Body inside BRN's XML budget (8,001 paragraphs, ~7.1 MiB XML)",
            "docx_limit (1 MiB output cap)",
            Timed,
            [Retained, Retained, Retained, Retained],
        ),
        c(
            "I1",
            "images",
            "Two PNGs, two relationships, two occurrences",
            "docx_unsupported",
            TwoImages,
            [Retained, Retained, Retained, Retained],
        ),
        c(
            "I2",
            "images",
            "One relationship used by two occurrences",
            "docx_unsupported",
            SharedImage,
            [Retained, Retained, Retained, Retained],
        ),
        c(
            "I3",
            "images",
            "Ordinary 8x8 JPEG",
            "docx_unsupported",
            Jpeg,
            [Retained, Retained, Retained, Retained],
        ),
        c(
            "I5",
            "images",
            "Header and body images both rId1: body occurrence maps to body bytes (header image not verified)",
            "docx_unsupported",
            SameRid,
            [Ambiguous, Retained, Retained, Retained],
        ),
        c(
            "I6",
            "images",
            "Image relationship whose part is missing",
            "docx_invalid",
            MissingImage,
            [DetectableGap, DetectableGap, DetectableGap, DetectableGap],
        ),
    ]
}

fn present(model: &Model, needle: &str) -> bool {
    model.dump.contains(needle)
}

/// Content is missing: detectable if a trace, warning or unmodelled-item
/// report remains, otherwise silent.
fn gap(model: &Model, what: &str, trace: &[&str]) -> (Verdict, String) {
    if let Some(t) = trace.iter().find(|t| present(model, t)) {
        return (DetectableGap, format!("Ok; {what}; trace kept: {t}"));
    }
    if !model.unmodelled.is_empty() {
        return (
            DetectableGap,
            format!("Ok; {what}; reported unmodelled {:?}", model.unmodelled),
        );
    }
    if !model.warnings.is_empty() {
        return (
            DetectableGap,
            format!("Ok; {what}; warnings: {:?}", model.warnings),
        );
    }
    (
        SilentLoss,
        format!("Ok; {what}, no trace, warning or unmodelled report"),
    )
}

fn judge(check: &Check, m: &Measured) -> (Verdict, String) {
    let model = match &m.read {
        Read::Ok(model) => model,
        Read::Err(e) => {
            if matches!(check, Check::Advertised) {
                return (
                    Refused,
                    format!("Err({e}); largest allocation {} bytes", m.largest),
                );
            }
            return (
                Refused,
                format!("Err({})", e.chars().take(160).collect::<String>()),
            );
        }
        Read::Panic(p) => return (Panic, format!("panic: {p}")),
    };
    let warned = if model.warnings.is_empty() {
        String::new()
    } else {
        format!("; warnings: {:?}", model.warnings)
    };
    match check {
        Check::Text(wanted, trace) => {
            let missing: Vec<_> = wanted.iter().filter(|s| !present(model, s)).collect();
            if missing.is_empty() {
                return (Retained, format!("Ok; all present{warned}"));
            }
            gap(model, &format!("missing {missing:?}"), trace)
        }
        Check::Accepts(wanted, lenient) => {
            if let Some(found) = wanted.iter().find(|s| present(model, s)) {
                if !model.warnings.is_empty() {
                    (
                        DetectableGap,
                        format!("Ok; {found:?} read as content{warned}"),
                    )
                } else if *lenient {
                    (Lenient, format!("Ok; accepted, {found:?} read correctly"))
                } else {
                    (Misread, format!("Ok; accepted, {found:?} read as content"))
                }
            } else {
                gap(model, &format!("nothing refused, {wanted:?} absent"), &[])
            }
        }
        Check::Structure(wanted, structure) => {
            let missing: Vec<_> = wanted.iter().filter(|s| !present(model, s)).collect();
            if !missing.is_empty() {
                return gap(model, &format!("missing {missing:?}"), structure);
            }
            match structure.iter().find(|s| present(model, s)) {
                Some(s) => (
                    Retained,
                    format!("Ok; wording and structure ({s}) kept{warned}"),
                ),
                None if !model.unmodelled.is_empty() || !model.warnings.is_empty() => (
                    DetectableGap,
                    format!(
                        "Ok; wording kept, structure absent; reported unmodelled {:?}{warned}",
                        model.unmodelled
                    ),
                ),
                None => (
                    Flattened,
                    "Ok; wording kept as plain text, structure absent".into(),
                ),
            }
        }
        Check::Picture => {
            let image = fixtures::png_bytes(fixtures::PNG_ONE);
            let exact = model.images.iter().any(|(_, b)| *b == image);
            let alt = present(model, "ALT-SENTINEL 日本語");
            let title = present(model, "TITLE-SENTINEL");
            let one = model.occurrences.len() == 1;
            let detail = format!(
                "Ok; exact bytes={exact}, one occurrence={one}, alt text={alt}, title={title}{warned}"
            );
            if exact && one && alt && title {
                (Retained, detail)
            } else {
                (SilentLoss, detail)
            }
        }
        Check::TwoImages => {
            let (a, b) = (
                fixtures::png_bytes(fixtures::PNG_A),
                fixtures::png_bytes(fixtures::PNG_B),
            );
            let exact = model.images.iter().any(|(_, x)| *x == a)
                && model.images.iter().any(|(_, x)| *x == b);
            let detail = format!(
                "Ok; occurrences {:?}, both exact={exact}",
                model.occurrences
            );
            if exact && model.occurrences.len() == 2 {
                (Retained, detail)
            } else {
                (SilentLoss, detail)
            }
        }
        Check::SharedImage => {
            let shared = fixtures::png_bytes(fixtures::PNG_SHARED);
            let exact = model.images.iter().any(|(_, x)| *x == shared);
            let detail = format!("Ok; occurrences {:?}, exact={exact}", model.occurrences);
            if exact && model.occurrences.len() == 2 {
                (Retained, detail)
            } else {
                (SilentLoss, detail)
            }
        }
        Check::Jpeg => {
            let exact = model.images.iter().any(|(_, x)| x == fixtures::jpeg());
            let detail = format!(
                "Ok; occurrences {:?}, exact original JPEG={exact}",
                model.occurrences
            );
            if exact && model.occurrences.len() == 1 {
                (Retained, detail)
            } else {
                (SilentLoss, detail)
            }
        }
        Check::SameRid => {
            let body = fixtures::png_bytes(fixtures::PNG_BODY);
            let header = fixtures::png_bytes(fixtures::PNG_HEADER);
            // Ambiguous when one id the reader uses for occurrences maps to both
            // the body and the header bytes.
            let mut ambiguous = false;
            let mut body_mapped = false;
            for id in &model.occurrences {
                let bytes: Vec<_> = model
                    .images
                    .iter()
                    .filter(|(i, _)| i == id)
                    .map(|(_, b)| b)
                    .collect();
                let (has_body, has_header) = (bytes.contains(&&body), bytes.contains(&&header));
                ambiguous |= has_body && has_header;
                body_mapped |= has_body && !has_header;
            }
            let detail = format!(
                "Ok; occurrences {:?}; image ids {:?}",
                model.occurrences,
                model
                    .images
                    .iter()
                    .map(|(i, _)| i.as_str())
                    .collect::<Vec<_>>()
            );
            if ambiguous {
                (Ambiguous, detail)
            } else if body_mapped {
                (Retained, detail)
            } else {
                (SilentLoss, detail)
            }
        }
        Check::MissingImage => {
            let detail = format!(
                "Ok; occurrences {:?}, {} images{warned}",
                model.occurrences,
                model.images.len()
            );
            if !model.occurrences.is_empty() || !model.warnings.is_empty() {
                (DetectableGap, detail)
            } else {
                (SilentLoss, detail)
            }
        }
        Check::Advertised => {
            let detail = format!("Ok; largest single allocation {} bytes{warned}", m.largest);
            if m.largest >= fixtures::B1_DECLARED {
                (AdvertisedAllocation, detail)
            } else {
                (
                    Lenient,
                    format!("{detail}; entry whose declared size disagrees with its data accepted"),
                )
            }
        }
        Check::Timed => {
            let (len, delimiters) = fixtures::b7_stats();
            let ok = present(model, "LAST-SENTINEL");
            let detail = format!(
                "Ok; {} ms in one call, no cancellation hook; peak heap {} MiB; XML {len} bytes, {delimiters} delimiters",
                m.millis,
                m.peak / (1024 * 1024)
            );
            (if ok { Retained } else { SilentLoss }, detail)
        }
    }
}

// ---------------------------------------------------------------- processes

const CHILD_TIMEOUT: Duration = Duration::from_secs(120);

fn child(reader: &str, id: &str) {
    let case = cases()
        .into_iter()
        .find(|c| c.id == id)
        .expect("known case");
    let bytes = fixtures::build(id);
    let m = measure(reader, &bytes);
    if std::env::var_os("DOCX_EVAL_DUMP").is_some()
        && let Read::Ok(model) = &m.read
    {
        eprintln!("{}", model.dump);
    }
    let (verdict, detail) = judge(&case.check, &m);
    println!("{verdict:?}\t{}", detail.replace(['\n', '\t'], " "));
}

fn run_child(reader: &str, id: &str) -> (Verdict, String) {
    let exe = std::env::current_exe().expect("own path");
    let mut process = Command::new(exe)
        .args(["--child", reader, id])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn child");
    let start = Instant::now();
    loop {
        if let Some(status) = process.try_wait().expect("wait") {
            let output = process.wait_with_output().expect("output");
            let line = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if status.success()
                && let Some((v, d)) = line.split_once('\t')
            {
                let verdict = parse_verdict(v);
                return (verdict, d.to_owned());
            }
            return (Abort, format!("child exited {status}; stdout {line:?}"));
        }
        if start.elapsed() > CHILD_TIMEOUT {
            let _ = process.kill();
            return (
                Abort,
                format!("no result after {}s", CHILD_TIMEOUT.as_secs()),
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn parse_verdict(v: &str) -> Verdict {
    [
        Retained,
        Refused,
        SilentLoss,
        Misread,
        Lenient,
        Flattened,
        Ambiguous,
        DetectableGap,
        Panic,
        AdvertisedAllocation,
        Abort,
    ]
    .into_iter()
    .find(|x| format!("{x:?}") == v)
    .unwrap_or(Abort)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--child") {
        child(&args[1], &args[2]);
        return;
    }
    // Optional filters: reader names and/or case IDs.
    let only_readers: Vec<&str> = READERS
        .iter()
        .copied()
        .filter(|r| args.iter().any(|a| a == r))
        .collect();
    let only_cases: Vec<&String> = args
        .iter()
        .filter(|a| !READERS.contains(&a.as_str()))
        .collect();
    let readers: Vec<(usize, &str)> = READERS
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, r)| only_readers.is_empty() || only_readers.contains(r))
        .collect();
    let cases: Vec<Case> = cases()
        .into_iter()
        .filter(|c| only_cases.is_empty() || only_cases.iter().any(|a| *a == c.id))
        .collect();

    // `DOCX_EVAL_FIXTURES=DIR` writes every package to DIR/<case>.docx so the
    // identical bytes can be replayed through BRN's current converter.
    if let Some(dir) = std::env::var_os("DOCX_EVAL_FIXTURES") {
        for case in &cases {
            let path = std::path::Path::new(&dir).join(format!("{}.docx", case.id));
            std::fs::write(path, fixtures::build(case.id)).expect("write fixture");
        }
    }

    if cases.is_empty() || readers.is_empty() {
        eprintln!("no case or reader matches {args:?}");
        std::process::exit(1);
    }
    let mut results = Vec::new();
    let mut mismatches = 0;
    for case in &cases {
        let mut row = Vec::new();
        for &(index, reader) in &readers {
            let (verdict, detail) = run_child(reader, case.id);
            let unexpected = verdict != case.expected[index];
            mismatches += usize::from(unexpected);
            row.push((reader, verdict, detail, unexpected));
        }
        results.push((case, row));
    }

    print!("| ID | Group | Case | BRN at 450eaa2 |");
    for (_, r) in &readers {
        print!(" {r} |");
    }
    println!();
    println!(
        "| --- | --- | --- | --- |{}",
        " --- |".repeat(readers.len())
    );
    for (case, row) in &results {
        print!(
            "| {} | {} | {} | {} |",
            case.id, case.group, case.title, case.brn
        );
        for (_, verdict, _, unexpected) in row {
            print!(
                " {verdict:?}{} |",
                if *unexpected { " **(unexpected)**" } else { "" }
            );
        }
        println!();
    }
    for &(_, reader) in &readers {
        println!("\n### {reader}\n");
        println!("| ID | Verdict | Observation |");
        println!("| --- | --- | --- |");
        for (case, row) in &results {
            if let Some((_, verdict, detail, _)) = row.iter().find(|r| r.0 == reader) {
                println!(
                    "| {} | {verdict:?} | {} |",
                    case.id,
                    detail.replace('|', "\\|")
                );
            }
        }
    }
    if mismatches > 0 {
        eprintln!("{mismatches} result(s) differ from the recorded expectation");
        std::process::exit(1);
    }
}
