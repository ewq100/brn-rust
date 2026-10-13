//! Deterministic Markdown chunking for derived retrieval indexes.
//!
//! Splits at ATX headings, then at paragraphs. Fenced code blocks and tables
//! stay whole up to [`HARD_CAP`]. Chunks are byte ranges into the original
//! text that exactly cover it, so quotes and anchors stay exact.

/// Identity recorded in derived indexes; bump on any output change.
pub const CHUNKER_VERSION: &str = "markdown-1";
/// Preferred maximum when packing paragraphs.
pub const TARGET_BYTES: usize = 2000;
/// Packing tries to reach this before closing a chunk (soft).
pub const MIN_BYTES: usize = 1200;
/// No chunk may exceed this.
pub const HARD_CAP: usize = 4000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Half-open byte range into the original markdown, on UTF-8 boundaries.
    pub start: usize,
    pub end: usize,
    /// Text of the ATX headings in force at `start`, outermost first.
    pub heading_path: Vec<String>,
}

enum Kind {
    Heading {
        level: usize,
        text: String,
    },
    Text,
    /// Fenced code block or table: never split when it fits in `HARD_CAP`.
    Atomic,
    /// Blank lines not attached to a paragraph; joins whatever chunk fits.
    Glue,
}

struct Unit {
    start: usize,
    end: usize,
    kind: Kind,
}

pub fn chunks(markdown: &str) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut path: Vec<String> = Vec::new();
    let mut cur: Option<(usize, usize)> = None;

    for unit in units(markdown) {
        let glue = matches!(unit.kind, Kind::Glue);
        if let Kind::Heading { level, text } = unit.kind {
            close(&mut out, cur.take(), &path);
            while stack.last().is_some_and(|(l, _)| *l >= level) {
                stack.pop();
            }
            stack.push((level, text));
            path = stack.iter().map(|(_, t)| t.clone()).collect();
        }
        for (s, e) in split_oversize(markdown, unit.start, unit.end) {
            cur = Some(match cur {
                None => (s, e),
                Some((cs, ce)) => {
                    let total = e - cs;
                    let fits = total <= TARGET_BYTES
                        || ((ce - cs < MIN_BYTES || glue) && total <= HARD_CAP);
                    if fits {
                        (cs, e)
                    } else {
                        close(&mut out, Some((cs, ce)), &path);
                        (s, e)
                    }
                }
            });
        }
    }
    close(&mut out, cur, &path);
    out
}

fn close(out: &mut Vec<Chunk>, range: Option<(usize, usize)>, path: &[String]) {
    if let Some((start, end)) = range {
        out.push(Chunk {
            start,
            end,
            heading_path: path.to_vec(),
        });
    }
}

/// Splits a unit larger than `HARD_CAP` at line boundaries, else at the last
/// whitespace, else at a char boundary. Never splits a `\r\n` pair.
fn split_oversize(md: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let bytes = md.as_bytes();
    let mut out = Vec::new();
    let mut pos = start;
    while end - pos > HARD_CAP {
        let mut limit = pos + HARD_CAP;
        while !md.is_char_boundary(limit) {
            limit -= 1;
        }
        let window = &md[pos..limit];
        let mut cut = window
            .rfind('\n')
            .map(|i| pos + i + 1)
            .or_else(|| {
                window
                    .char_indices()
                    .rev()
                    .find(|(_, c)| c.is_whitespace())
                    .map(|(i, c)| pos + i + c.len_utf8())
            })
            .unwrap_or(limit);
        if cut < md.len() && bytes[cut - 1] == b'\r' && bytes[cut] == b'\n' && cut - 1 > pos {
            cut -= 1;
        }
        out.push((pos, cut));
        pos = cut;
    }
    out.push((pos, end));
    out
}

fn units(md: &str) -> Vec<Unit> {
    let mut lines = Vec::new();
    let mut pos = 0;
    for l in md.split_inclusive('\n') {
        lines.push((pos, pos + l.len()));
        pos += l.len();
    }
    let line = |i: usize| &md[lines[i].0..lines[i].1];
    let n = lines.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        let text = line(i);
        let (start, mut j) = (lines[i].0, i + 1);
        let kind = if let Some(fence) = fence_open(text) {
            while j < n {
                let closes = fence_close(line(j), fence);
                j += 1;
                if closes {
                    break;
                }
            }
            Kind::Atomic
        } else if let Some((level, text)) = atx_heading(text) {
            Kind::Heading { level, text }
        } else if is_blank(text) {
            while j < n && is_blank(line(j)) {
                j += 1;
            }
            Kind::Glue
        } else if is_table_row(text) {
            while j < n && is_table_row(line(j)) {
                j += 1;
            }
            Kind::Atomic
        } else {
            while j < n && !is_blank(line(j)) && !interrupts_paragraph(line(j)) {
                j += 1;
            }
            while j < n && is_blank(line(j)) {
                j += 1;
            }
            Kind::Text
        };
        out.push(Unit {
            start,
            end: lines[j - 1].1,
            kind,
        });
        i = j;
    }
    out
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

fn is_table_row(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

fn interrupts_paragraph(line: &str) -> bool {
    atx_heading(line).is_some() || fence_open(line).is_some() || is_table_row(line)
}

fn indent_of(line: &str) -> Option<usize> {
    let indent = line.bytes().take_while(|&b| b == b' ').count();
    (indent <= 3).then_some(indent)
}

/// Fence character and opening length.
fn fence_open(line: &str) -> Option<(u8, usize)> {
    let s = line.trim_end_matches(['\n', '\r']);
    let s = &s[indent_of(s)?..];
    let ch = *s.as_bytes().first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let len = s.bytes().take_while(|&b| b == ch).count();
    let info = &s[len..];
    (len >= 3 && !(ch == b'`' && info.contains('`'))).then_some((ch, len))
}

fn fence_close(line: &str, (ch, len): (u8, usize)) -> bool {
    let s = line.trim_end_matches(['\n', '\r']);
    let Some(indent) = indent_of(s) else {
        return false;
    };
    let s = &s[indent..];
    let run = s.bytes().take_while(|&b| b == ch).count();
    run >= len && s[run..].trim().is_empty()
}

/// Level and text of a CommonMark ATX heading line.
fn atx_heading(line: &str) -> Option<(usize, String)> {
    let s = line.trim_end_matches(['\n', '\r']);
    let s = &s[indent_of(s)?..];
    let level = s.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &s[level..];
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    let rest = rest.trim();
    let body = rest.trim_end_matches('#');
    let text = if body.is_empty() || body.ends_with([' ', '\t']) {
        body.trim()
    } else {
        rest
    };
    Some((level, text.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SplitMix64(u64);

    impl SplitMix64 {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }

        fn chance(&mut self, percent: usize) -> bool {
            self.below(100) < percent
        }
    }

    const WORDS: [&str; 8] = [
        "alpha",
        "beta",
        "gamma",
        "õäöü",
        "😀",
        "日本語",
        "delta",
        "x",
    ];

    fn words(rng: &mut SplitMix64, count: usize) -> String {
        let mut out = String::new();
        for k in 0..count {
            if k > 0 {
                out.push(' ');
            }
            out.push_str(WORDS[rng.below(WORDS.len())]);
        }
        out
    }

    #[derive(Default)]
    struct Expect {
        headings: Vec<(usize, usize, String)>,
        fences: Vec<(usize, usize)>,
        tables: Vec<(usize, usize)>,
    }

    struct Gen {
        rng: SplitMix64,
        doc: String,
        eol_mode: usize,
        expect: Expect,
    }

    impl Gen {
        fn eol(&mut self) {
            let crlf = match self.eol_mode {
                0 => false,
                1 => true,
                _ => self.rng.chance(50),
            };
            self.doc.push_str(if crlf { "\r\n" } else { "\n" });
        }

        fn heading(&mut self) {
            let level = 1 + self.rng.below(6);
            let n = 1 + self.rng.below(4);
            let text = words(&mut self.rng, n);
            let start = self.doc.len();
            self.doc.push_str(&" ".repeat(self.rng.below(4)));
            self.doc.push_str(&"#".repeat(level));
            self.doc.push(if self.rng.chance(10) { '\t' } else { ' ' });
            self.doc.push_str(&text);
            if self.rng.chance(30) {
                self.doc.push(' ');
                self.doc.push_str(&"#".repeat(1 + self.rng.below(3)));
            }
            self.eol();
            self.expect.headings.push((start, level, text));
        }

        fn paragraph(&mut self) {
            for _ in 0..1 + self.rng.below(5) {
                match self.rng.below(20) {
                    0 => self.doc.push_str("#tag not a heading"),
                    1 => self.doc.push_str("####### seven hashes"),
                    2 => {
                        let long = ["õ", "日", "😀", "a"][self.rng.below(4)];
                        self.doc.push_str(&long.repeat(300 + self.rng.below(3000)));
                    }
                    3 => {
                        let n = 100 + self.rng.below(900);
                        let line = words(&mut self.rng, n);
                        self.doc.push_str(&line);
                    }
                    _ => {
                        let n = 1 + self.rng.below(15);
                        let line = words(&mut self.rng, n);
                        self.doc.push_str(&line);
                    }
                }
                self.eol();
            }
            self.blank_lines(0, 3);
        }

        /// Appends `min..min + spread` blank (sometimes whitespace-only) lines.
        fn blank_lines(&mut self, min: usize, spread: usize) {
            for _ in 0..min + self.rng.below(spread) {
                if self.rng.chance(20) {
                    self.doc.push_str("  ");
                }
                self.eol();
            }
        }

        /// Appends an unbounded run of rows from `row`; returns the span.
        fn rows(&mut self, size: usize, row: fn(&mut Gen) -> String) -> (usize, usize) {
            let start = self.doc.len();
            while self.doc.len() - start < size {
                let r = row(self);
                self.doc.push_str(&r);
                self.eol();
            }
            (start, self.doc.len())
        }

        fn size_class(&mut self) -> usize {
            match self.rng.below(4) {
                0 => 1 + self.rng.below(300),
                1 => 1000 + self.rng.below(2000),
                2 => 3600 + self.rng.below(300),
                _ => 4100 + self.rng.below(5000),
            }
        }

        fn table(&mut self) {
            let size = self.size_class();
            let span = self.rows(size, |g| {
                let (a, b) = (words(&mut g.rng, 2), words(&mut g.rng, 1));
                let indent = if g.rng.chance(20) { " " } else { "" };
                format!("{indent}| {a} | {b} |")
            });
            self.expect.tables.push(span);
            self.eol();
        }

        /// Returns true when the fence is unclosed, which ends the document.
        fn fence(&mut self) -> bool {
            let ch = if self.rng.chance(50) { '`' } else { '~' };
            let open_len = 3 + self.rng.below(3);
            let start = self.doc.len();
            self.doc.push_str(&ch.to_string().repeat(open_len));
            if self.rng.chance(50) {
                self.doc.push_str(" rust");
            }
            self.eol();
            let size = self.size_class();
            let body_start = self.doc.len();
            while self.doc.len() - body_start < size {
                let line = match self.rng.below(8) {
                    0 => "# fake heading".to_string(),
                    1 => "| a | b |".to_string(),
                    2 => String::new(),
                    3 => ch.to_string().repeat(open_len - 1),
                    4 => {
                        let other = if ch == '`' { '~' } else { '`' };
                        other.to_string().repeat(3)
                    }
                    _ => {
                        let n = 1 + self.rng.below(12);
                        words(&mut self.rng, n)
                    }
                };
                self.doc.push_str(&line);
                self.eol();
            }
            let unclosed = self.rng.chance(10);
            if !unclosed {
                self.doc
                    .push_str(&ch.to_string().repeat(open_len + self.rng.below(3)));
                self.eol();
            }
            self.expect.fences.push((start, self.doc.len()));
            unclosed
        }
    }

    fn generate(seed: u64) -> (String, Expect) {
        let mut g = Gen {
            rng: SplitMix64(seed),
            doc: String::new(),
            eol_mode: 0,
            expect: Expect::default(),
        };
        g.eol_mode = g.rng.below(3);
        for _ in 0..1 + g.rng.below(25) {
            match g.rng.below(10) {
                0..=2 => g.heading(),
                3..=5 => g.paragraph(),
                6 => g.blank_lines(1, 3),
                7 => g.table(),
                _ => {
                    if g.fence() {
                        break;
                    }
                    g.blank_lines(0, 2);
                }
            }
        }
        (g.doc, g.expect)
    }

    fn expected_path(headings: &[(usize, usize, String)], at: usize) -> Vec<String> {
        let mut stack: Vec<(usize, &str)> = Vec::new();
        for (_, level, text) in headings.iter().filter(|h| h.0 <= at) {
            while stack.last().is_some_and(|(l, _)| l >= level) {
                stack.pop();
            }
            stack.push((*level, text));
        }
        stack.iter().map(|(_, t)| t.to_string()).collect()
    }

    fn check_coverage(doc: &str, cs: &[Chunk]) {
        if doc.is_empty() {
            assert!(cs.is_empty());
            return;
        }
        assert_eq!(cs.first().map(|c| c.start), Some(0));
        assert_eq!(cs.last().map(|c| c.end), Some(doc.len()));
        let mut prev = 0;
        for c in cs {
            assert_eq!(c.start, prev, "contiguous");
            assert!(c.end > c.start, "non-empty");
            assert!(c.end - c.start <= HARD_CAP, "hard cap");
            assert!(doc.is_char_boundary(c.start) && doc.is_char_boundary(c.end));
            if c.end < doc.len() {
                let b = doc.as_bytes();
                assert!(!(b[c.end - 1] == b'\r' && b[c.end] == b'\n'), "split CRLF");
            }
            prev = c.end;
        }
    }

    fn contained(cs: &[Chunk], (s, e): (usize, usize)) -> bool {
        cs.iter().any(|c| c.start <= s && e <= c.end)
    }

    #[test]
    fn randomized_properties() {
        for seed in 0..2500u64 {
            let (doc, expect) = generate(seed);
            let cs = chunks(&doc);
            assert_eq!(cs, chunks(&doc), "seed {seed}: determinism");
            check_coverage(&doc, &cs);
            for (start, ..) in &expect.headings {
                assert!(
                    cs.iter().any(|c| c.start == *start),
                    "seed {seed}: heading at {start} must start a chunk"
                );
            }
            for c in &cs {
                assert_eq!(
                    c.heading_path,
                    expected_path(&expect.headings, c.start),
                    "seed {seed}: heading path at {}",
                    c.start
                );
            }
            for &(s, e) in expect.fences.iter().chain(&expect.tables) {
                if e - s <= HARD_CAP {
                    assert!(contained(&cs, (s, e)), "seed {seed}: split block {s}..{e}");
                }
            }
        }
    }

    fn texts<'a>(doc: &'a str, cs: &[Chunk]) -> Vec<&'a str> {
        cs.iter().map(|c| &doc[c.start..c.end]).collect()
    }

    fn path(c: &Chunk) -> Vec<&str> {
        c.heading_path.iter().map(String::as_str).collect()
    }

    #[test]
    fn empty_input() {
        assert!(chunks("").is_empty());
    }

    #[test]
    fn heading_path_stack() {
        let doc = "# A\n## B\n### C\n## D\ntext\n# E\n";
        let cs = chunks(doc);
        let got: Vec<_> = cs.iter().map(path).collect();
        assert_eq!(
            got,
            [
                vec!["A"],
                vec!["A", "B"],
                vec!["A", "B", "C"],
                vec!["A", "D"],
                vec!["E"]
            ][..]
        );
        assert_eq!(
            texts(doc, &cs),
            ["# A\n", "## B\n", "### C\n", "## D\ntext\n", "# E\n"]
        );
    }

    #[test]
    fn preamble_has_empty_path() {
        let doc = "intro\n\n# T\nbody\n";
        let cs = chunks(doc);
        assert_eq!(texts(doc, &cs), ["intro\n\n", "# T\nbody\n"]);
        assert!(cs[0].heading_path.is_empty());
        assert_eq!(path(&cs[1]), ["T"]);
    }

    #[test]
    fn closing_hashes_are_stripped() {
        let cs = chunks("## Title ##\nx\n#### C# ####  \n##\n");
        assert_eq!(path(&cs[0]), ["Title"]);
        assert_eq!(path(&cs[1]), ["Title", "C#"]);
        assert_eq!(path(&cs[2]), [""]);
    }

    #[test]
    fn hash_tag_is_not_a_heading() {
        let doc = "#tag\n####### seven\n    # indented\n";
        let cs = chunks(doc);
        assert_eq!(cs.len(), 1);
        assert!(cs[0].heading_path.is_empty());
    }

    #[test]
    fn heading_inside_fence_is_ignored() {
        let doc = "# Real\n```\n# Fake\n```\n~~~\n# Also fake\n";
        let cs = chunks(doc);
        assert_eq!(cs.len(), 1);
        assert_eq!(path(&cs[0]), ["Real"]);
    }

    #[test]
    fn fence_closing_rules() {
        // A shorter or different-character fence does not close; a longer one does.
        let doc = "````\n```\n~~~~\n`````\n# H\n";
        let cs = chunks(doc);
        assert_eq!(texts(doc, &cs), ["````\n```\n~~~~\n`````\n", "# H\n"]);
    }

    #[test]
    fn small_paragraphs_pack_together() {
        let doc = "one\n\ntwo\n\nthree\n";
        assert_eq!(chunks(doc).len(), 1);
    }

    #[test]
    fn splits_at_paragraph_boundaries_above_target() {
        let para = format!("{}\n\n", "a".repeat(898));
        let doc = para.repeat(6);
        let cs = chunks(&doc);
        assert_eq!(cs.len(), 3);
        for t in texts(&doc, &cs) {
            assert_eq!(t, para.repeat(2));
        }
    }

    #[test]
    fn small_chunk_absorbs_next_paragraph_up_to_hard_cap() {
        let doc = format!("{}\n\n{}\n\n", "a".repeat(998), "b".repeat(2498));
        assert_eq!(chunks(&doc).len(), 1);
        let doc = format!("{}\n\n{}\n\n", "a".repeat(998), "b".repeat(3098));
        assert_eq!(chunks(&doc).len(), 2);
    }

    #[test]
    fn whole_fence_and_table_kept_together() {
        let body = "line of code\n".repeat(250);
        let doc = format!("para\n\n```\n{body}```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");
        let cs = chunks(&doc);
        check_coverage(&doc, &cs);
        let fence_start = doc.find("```").unwrap();
        let fence_end = fence_start + 3 + 1 + body.len() + 4;
        assert!(contained(&cs, (fence_start, fence_end)));
        let table_start = doc.find("| a").unwrap();
        assert!(contained(&cs, (table_start, doc.len())));
    }

    #[test]
    fn oversize_paragraph_without_whitespace_is_split() {
        let doc = "é".repeat(5000);
        let cs = chunks(&doc);
        check_coverage(&doc, &cs);
        assert!(cs.len() >= 3);
    }

    #[test]
    fn oversize_prefers_lines_then_whitespace() {
        let lines = format!("{}\n", "w".repeat(299)).repeat(50);
        let cs = chunks(&lines);
        check_coverage(&lines, &cs);
        assert!(texts(&lines, &cs).iter().all(|t| t.ends_with('\n')));

        let words = "word ".repeat(2000);
        let cs = chunks(&words);
        check_coverage(&words, &cs);
        assert!(texts(&words, &cs).iter().all(|t| t.ends_with(' ')));
    }

    #[test]
    fn oversize_fence_is_split_within_cap() {
        let doc = format!("```\n{}```\n", "code line\n".repeat(1000));
        let cs = chunks(&doc);
        check_coverage(&doc, &cs);
        assert!(cs.len() >= 3);
    }

    #[test]
    fn crlf_pair_is_not_split() {
        let doc = format!("{}\r\n{}", "a".repeat(3999), "b".repeat(10));
        let cs = chunks(&doc);
        check_coverage(&doc, &cs);
        assert_eq!(cs[0].end, 3999);
        let doc = "# T\r\ntext\r\n\r\n## U\r\nmore\r\n";
        let cs = chunks(doc);
        assert_eq!(texts(doc, &cs), ["# T\r\ntext\r\n\r\n", "## U\r\nmore\r\n"]);
    }

    #[test]
    fn version_is_stable() {
        assert_eq!(CHUNKER_VERSION, "markdown-1");
    }
}
