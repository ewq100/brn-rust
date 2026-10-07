# H4 findings: published DOCX readers vs BRN's converter

**Recommendation: adopt none of the four readers now.** Keep BRN's own bounded
converter (decision **build**, unchanged). Evaluated 2026-10-06 for the
[H4 task](../../docs/work/active/v1-handoff.md#h4--evaluate-published-docx-reader-before-broader-stage8).
The first pass covered docx-rs 0.4.22, the reader the task named. At the owner's
request a second pass added rdocx 0.15.0, office_oxide 0.1.13 and
betteroffice-docx-parse 0.3.0. This is a result record. It changes no product
code. Review and integration state is under
[review and integration](#review-and-integration).

## Question and stop rule

Can a published reader replace a meaningful part of BRN's OOXML interpretation
behind a small admission/mapping adapter? The task stops on the first
irrecoverable silent loss, unavailable original asset or occurrence, unbounded
allocation, or need for a second full interpreter.

Every candidate hit the stop on BRN's supported profile or on content an adapter
cannot see:

| Reader | First silent loss in matrix order | All losses an adapter cannot see |
| --- | --- | --- |
| docx-rs 0.4.22 | S3: alt text and title | S3, S4, M4, M7, M8, M10, M13 |
| rdocx 0.15.0 | S3: picture title | S3 |
| office_oxide 0.1.13 | S3: picture title | S3, M6, M12, M15 |
| betteroffice-docx-parse 0.3.0 | M11: VML text box text | M11, M12, M15 |

The readers fail in different places.

- **rdocx is the closest.** Its reader facade reports elements it does not model
  as `UnsupportedXml` items, and it lists revisions. So altChunk, ruby, customXml
  and the unknown wrapper (M10, M12, M14, M15) all leave something an adapter can
  refuse on. Its one silent loss is the picture title (`wp:docPr/@title`) in BRN's
  supported profile. No typed API exposes it and no flag marks it. It survives only
  in raw captured XML (`StoryItemSnapshot::xml()`), which this method excludes
  because re-reading source XML is the job being delegated. rdocx also left an
  internal-DTD entity unresolved as literal text (B5).
- **office_oxide and betteroffice-docx-parse** drop text inside an element they
  don't model (M15). They also drop both the base text and the phonetic guide of a
  real ruby annotation (M12, as used in Japanese and Chinese text). Neither leaves
  an error, warning or trace. For these two, "preserve completely or refuse
  explicitly" still needs BRN's strict walk over every element next to the reader.
  That is the second interpreter the stop rule excludes.
- **betteroffice-docx-parse** has the strongest admission (DTD, alias, CRC and
  encoding refusal) and keeps alt text and title, but has the M11/M12/M15 losses.

Nothing here is a verdict on these libraries for their own purposes. All four
read S1, S2 and the image bytes of I1 to I3 correctly.

## Identity and environment

| Item | Value |
| --- | --- |
| Baseline | `origin/main` `450eaa2` (PR79 merge), branch `codex/h4-docx-reader-evaluation`. The branch was rebased onto `6781838` (H1/H2 merged) for the PR. `crates/brn-store` is identical between the two, so the BRN replay at `450eaa2` still applies. |
| docx-rs | 0.4.22, MIT. `.crate` SHA-256 `7fdf00e8af6d0b3e92d4bbf9b76f773d8b84ea80f310324ad16cbdc2e653e02c` matches the index and the lock. Source identical (`diff -r`) to `bokuweb/docx-rs` `f04cf8b4` `docx-core`. `git ls-remote --tags` on 2026-10-06 listed 23 tags, newest `0.4.21`; no `0.4.22` tag. |
| rdocx | 0.15.0, MIT OR Apache-2.0, first release 2026-02-22. `.crate` SHA-256 `9e295053c3b11857677607d05a50b2687db159b316b406c698a619f70d0f50bc` matches the lock. VCS `tensorbee/rdocx` `9d019472` (`crates/rdocx`). |
| office_oxide | 0.1.13, MIT OR Apache-2.0, first release 2026-04-28. `.crate` SHA-256 `22582556784e5c9005e12e2c2e78c2959674ca082edf806d939fff4ffceca9de` matches the lock. VCS `yfedoseev/office_oxide` `7fce6094`. |
| betteroffice-docx-parse | 0.3.0, Apache-2.0, first release 2026-07-18 (with `betteroffice-opc`). `.crate` SHA-256 `697674f35c2ff2eb311e892b015697412c8e1c7671126b74d58f9adcf4e795f3`; `betteroffice-opc` 0.3.0 `18653484735ca856523c32d3b8b59aaf3e2c9980aa00b8dddea71f82296d9689`. Both match the lock. VCS `openooxml/betteroffice` `34b9e93a`. |
| Source checks | docx-rs source was compared with its upstream commit. For the other three only the crate checksum was checked, not upstream source identity. |
| Platform | Darwin 25.5.0 arm64, Rust 1.98.1 (pinned), evaluator lock in this directory, target `/private/tmp/brn-h4-eval/target` |

How each reader was called, with BRN's budgets wherever the reader accepts them:

| Reader | Call | Limits | What the oracle inspects |
| --- | --- | --- | --- |
| docx-rs | `read_docx_with_options`, previews off, `image` feature off | none available | `Docx::json()`, `Debug`, `Docx::images` |
| rdocx | `Document::from_bytes_with_limits` (default features off) | 256 entries, 8 MiB part, 32 MiB total | `to_markdown`, `to_html`, `images`/`image_data`, `links`, `footnotes`, `revisions`, `comments`, story item text, and the reader facade (`body_items` → paragraph, hyperlink and run items): `UnsupportedXml` names, field instructions, hyperlink tooltips, `has_unmodeled_semantic_attributes` flags |
| office_oxide | `Document::from_reader(.., Docx)` + `to_ir` | process-global `set_max_package_entries(256)`, `set_max_package_bytes(32 MiB)`; the fixed per-part limit is 512 MiB (`MAX_PART_SIZE`) | `to_markdown`, IR JSON (base64 image bytes), IR `warnings`, and `as_docx()`: body and header/footer model, image parts by relationship id, `unreadable_parts` |
| betteroffice | `parse_docx_s9_wire_with_limits` | `ParseLimits` with 8 MiB XML and depth 64; the ZIP layer's 512 MiB/5,000-entry ceiling is fixed | wire JSON (per-occurrence image data URLs, media entries, relationships), `warnings` |

Raw package XML is never part of what the oracle searches, and neither are the
raw bytes behind rdocx's `UnsupportedXml` items (only their names). Re-reading the
source is the job the library is supposed to take over.

Dependency cost against BRN's lock (normal graph of each reader):

| Reader | Packages | Not in BRN's lock |
| --- | --- | --- |
| docx-rs | 36 | 2 (`docx-rs`, `zopfli`) |
| rdocx | 149 | 45 (fonts, shaping, HTML parsing, PDF/raster layout, `oxml-*`, `rdocx-*`) |
| office_oxide | 38 | 6 (incl. a second `quick-xml` 0.42) |
| betteroffice-docx-parse | 46 | 6 (incl. `libc` 0.2.190, which differs from BRN's) |

## Commands and results

From `experiments/docx-reader-eval`, with `CARGO_TARGET_DIR=/private/tmp/brn-h4-eval/target`:

```sh
cargo fmt --check                                     # passed
cargo clippy --locked --all-targets -- -D warnings    # passed
cargo run --locked --release                          # exit 0, 33 cases x 4 readers, 0 unexpected
```

Every (case, reader) pair runs in a separate child process with a 120 s limit, so
an abort or hang would be recorded rather than ending the run. None happened.
Each verdict is checked against the expectation recorded in `src/main.rs`, and any
difference exits 1. Two consecutive full runs gave identical verdicts.

The BRN column came from replaying the identical bytes (`DOCX_EVAL_FIXTURES=DIR`)
through BRN's `convert_source` at `450eaa2`, using the temporary test under
[BRN replay](#brn-replay). That run had 1 passed and 0 failed, and the test was
removed afterwards.

Verdicts:

- **Retained:** the wording or asset is in the reader's output.
- **Refused:** the reader returned an error.
- **SilentLoss:** read succeeded, and content is missing with no trace, warning or
  unmodelled-item report.
- **DetectableGap:** content is missing, but a trace (relationship id, revision
  author, shape style), a warning or an unmodelled-item report remains, so an
  adapter could refuse.
- **Misread:** read succeeded and produced wrong or arbitrary content without
  saying so (foreign namespace read as text, one of two aliases chosen, an
  unresolved entity kept as literal text).
- **Lenient:** read succeeded on input BRN's admission refuses, and the content is
  right (UTF-16, backslash part name, declared size that disagrees with the data).
- **Flattened:** the wording survives but its structure is gone.
- **Ambiguous:** an image occurrence can't be mapped to one set of bytes.
- **Panic / AdvertisedAllocation / Abort:** as named.

| ID | Case | BRN at 450eaa2 | docx-rs | rdocx | office_oxide | betteroffice |
| --- | --- | --- | --- | --- | --- | --- |
| S0 | Minimal package, no `word/_rels/document.xml.rels` | Ok DocxTextV1 | Refused | Retained | Retained | Retained |
| S1a | Unicode paragraphs, Stored | Ok, exact | Retained | Retained | Retained | Retained |
| S1b | Unicode paragraphs, Deflate | Ok, exact | Retained | Retained | Retained | Retained |
| S2 | Heading style, numbered list, external link, simple table | Ok, exact | Retained | Retained | Retained | Retained |
| S3 | One inline PNG: exact bytes, alt text and title | Ok DocxInlinePngV1 (bytes, position, alt, title) | SilentLoss (alt, title) | SilentLoss (title) | SilentLoss (title) | Retained |
| S4 | External hyperlink with a tooltip | Ok DocxTextV1 (tooltip as link title) | SilentLoss (tooltip) | Retained | Retained | Retained |
| M1 | Final-section default header | docx_unsupported | Retained | Retained | Retained | Retained |
| M2 | Header referenced only by an earlier section | docx_unsupported | DetectableGap | Retained | Retained | Retained |
| M3 | Final-section default footer | docx_unsupported | Retained | Retained | Retained | Retained |
| M4 | Footnote reference in the same run as text, plus footnotes part | docx_unsupported | SilentLoss | Retained | Retained | Retained |
| M5 | Comment range, reference and comments part | docx_unsupported | Retained | Retained | Retained | Retained |
| M6 | Tracked insertion and deletion | docx_unsupported | Retained | DetectableGap (deleted text not rendered; `revisions()` lists it) | SilentLoss (deleted text gone) | Retained |
| M7 | Second prefix bound to WordprocessingML inside a `w:` run | Ok, both wordings | SilentLoss | Retained | Retained | Retained |
| M8 | WordprocessingML as the default namespace | Ok, wording kept | SilentLoss | Retained | Retained | Retained |
| M9 | `w` prefix bound to a foreign namespace | docx_unsupported | Misread | Refused | Refused | Misread |
| M10 | Body `w:altChunk` with imported part | docx_unsupported | SilentLoss | DetectableGap (`UnsupportedXml:w:altChunk`) | Retained | DetectableGap (relationship kept) |
| M11 | VML text box (`w:pict/v:shape/v:textbox`) | docx_unsupported | DetectableGap | Retained | Retained | SilentLoss |
| M12 | Ruby annotation | docx_unsupported | Flattened | DetectableGap (`UnsupportedXml:w:ruby`, both texts gone) | SilentLoss (both texts) | SilentLoss (both texts) |
| M13 | `w:fldSimple` HYPERLINK | docx_unsupported | SilentLoss (destination) | Retained (field instruction) | Retained | Retained |
| M14 | Inline `w:customXml` | docx_unsupported | Flattened | DetectableGap (`UnsupportedXml:w:customXml`) | Flattened | Flattened |
| M15 | Unknown WordprocessingML element wrapping a run | docx_unsupported | Flattened | DetectableGap (`UnsupportedXml:w:unknownWrapper`) | SilentLoss | SilentLoss |
| B1 | Entry advertising 3.75 GiB (`0xF0000000`) uncompressed | docx_limit | AdvertisedAllocation (4,026,531,840 bytes) | Refused (8 MiB part limit) | Refused (512 MiB part limit) | Lenient (size mismatch ignored; largest allocation 47 KB) |
| B2 | Stored entry with a corrupted byte (CRC mismatch) | docx_invalid | Panic | Refused | DetectableGap (corrupt text read, CRC warning) | Refused |
| B3 | Main part named `word\document.xml` | docx_invalid | Refused | Refused | Lenient | Refused |
| B4 | Case alias `WORD/document.xml` beside `word/document.xml` | docx_invalid | Misread | Refused | Refused | Refused |
| B5 | Internal DTD entity | docx_unsupported | Refused | Misread (`A&s;B` as literal text) | Misread (`A&s;B` as literal text) | Refused |
| B6 | UTF-16 main part | docx_unsupported | Refused | Refused | Lenient | Refused |
| B7 | 7,464,492-byte XML body, 8,001 paragraphs, 48,016 `<`/`=` (inside BRN's XML guard) | docx_limit (1 MiB output cap) | Retained, 10 ms, 42 MiB | Retained, 35 ms, 74 MiB | Retained, 24 ms, 20 MiB | Retained, 35 ms, 121 MiB |
| I1 | Two PNGs, two relationships, two occurrences | docx_unsupported | Retained | Retained | Retained | Retained |
| I2 | One relationship used twice | docx_unsupported | Retained | Retained | Retained | Retained |
| I3 | Ordinary 8x8 JPEG | docx_unsupported | Retained | Retained | Retained | Retained |
| I5 | Header and body images both `rId1`: body occurrence maps to body bytes (header image not verified) | docx_unsupported | Ambiguous | Retained | Retained | Retained |
| I6 | Image relationship to a missing part | docx_invalid | DetectableGap | DetectableGap | DetectableGap (`as_docx()` keeps the drawing; Markdown shows only italic alt text and the IR omits the image) | DetectableGap |

B7 times and peak heap cover the library's parse call only (for office_oxide,
parse plus `to_ir`). They are from one release-mode run and vary between runs (an
independent rerun saw 10 to 36 ms). None of the readers offers a cancellation hook.

## Per-reader notes

- **docx-rs 0.4.22.** It dispatches on the literal `w` prefix, so M7 and M8 lose
  text, and it has no tooltip or docPr title support. It has no limits API, sizes
  buffers from ZIP headers (B1) and unwraps decompression errors (B2 panic).
  Source-only: header, footer and media read failures are discarded with
  `filter_map`/`if let Ok`; `read_width` panics on a non-numeric width. Unreleased
  main `4ff72dc8` adds part-name validation but keeps the B1/B2 code and the prefix
  dispatch.
- **rdocx 0.15.0.** It is the closest to BRN's rule.
  - It is namespace-correct, takes caller-supplied ZIP limits and refuses case
    aliases.
  - Its facade reports unmodelled body, paragraph, hyperlink and run children as
    `UnsupportedXml`, and it flags unmodelled attributes on hyperlinks, fields and
    tables.
  - It still drops the picture title from its typed model with no flag, because
    `DrawingRef` exposes only kind, name, description, size and relationship. The
    title remains only in raw captured XML. It leaves an internal-DTD entity
    unresolved as the literal text `&s;` (B5).
  - M6 is conservative: a second `Document` after `reject_all()` renders the
    deleted text, so a richer adapter could keep it.
  - Its parser models `w:ruby` only as a paragraph child; ECMA-376 puts it inside
    `w:r`, as M12 does, so it lands in `UnsupportedXml`.
  - An adapter would still need BRN's XML guard (B5), BRN's own docPr parsing for
    the title, a facade walk that also covers tables, headers, notes, content
    controls, equations and revisions (not exercised here), and mapping code that re-expresses BRN's refusal policies
    (paint, list forms, table shapes) over rdocx's model.
  - Cost: the `rdocx` crate alone is about 114,000 lines of source, first released
    2026-02-22, with 17 releases through 0.15.0 (2026-10-04, two days before this
    evaluation). Its normal graph adds 45 packages (fonts,
    shaping, HTML parsing, PDF and raster layout) even with default features off,
    for a read-only use.
- **office_oxide 0.1.13.** It has the most permissive admission and the only
  warnings channel that fired (B2). It reads altChunk content and field
  destinations. It drops tracked deletions, ruby and text inside unknown elements
  with no trace. It accepts backslash names, UTF-16 and literal entity text. Its IR
  and Markdown turn a missing image into italic alt text, though `as_docx()` still
  shows the drawing. Its package limits are process-global setters, so one BRN
  setting would affect every caller in the process. Parsing may run on a spawned
  thread with a larger stack.
- **betteroffice-docx-parse 0.3.0.** It has the strongest admission: bounded XML
  (`ParseLimits`), DTD refusal, alias and CRC refusal, exact image data per
  occurrence, alt text, title, tooltip and a `warnings` field. It still drops VML
  text boxes, ruby and text inside unknown elements silently. It reads
  foreign-namespace elements with the `w` prefix as document text, and ignores an
  entry's declared size. Source-only: styles, settings and theme are looked up at
  the literal paths `word/styles.xml`, `word/settings.xml` and
  `word/theme/theme1.xml` rather than through relationships.

## Adapter scope if one were adopted anyway

No adapter was built; this is an estimate.

- **docx-rs, office_oxide, betteroffice-docx-parse.** Each loses content silently
  in a case only a strict element-by-element walk catches (M7/M8, or M12/M15).
  That walk is BRN's `document.rs` (about 1,370 lines) plus the drawing parser in
  `image.rs`. Each also differs from BRN's package and XML admission somewhere
  (B1, B3, B5, B6, M9), so `package.rs` (339), `xml.rs` (92) and `opc.rs` (580)
  stay too. That keeps about 3,300 lines of BRN code and adds a dependency and a
  second parse of every part.
- **rdocx.** It is not excluded by an element-level loss. Its blockers are
  narrower:
  - the silent title loss in BRN's supported profile, which needs BRN's own docPr
    parsing;
  - B5, which needs BRN's `xml.rs` guard in front;
  - the dependency weight and the lack of cancellation.

  Whether an rdocx adapter would be smaller than BRN's `document.rs` is
  unmeasured. It would need a facade walk over every story and container, plus
  BRN's refusal and Markdown policies re-expressed over rdocx's model. Answering
  that takes a bounded adapter prototype, which is not justified while the title
  loss and dependency cost stand.

## Next Stage8 acceptance

- Broader DOCX support extends BRN's existing strict converter.
- Re-evaluate a reader if it reports every unmodelled element and attribute
  (rdocx does this for the element positions exercised here), takes caller-scoped limits and supports
  cancellation. If the owner wants reuse later, the useful next step is a bounded
  rdocx adapter prototype measured against this matrix and BRN's `document.rs`
  tests. Before that, the title gap and the 45-package graph need an answer
  (upstream or by accepting them).
- Multiple inline raster occurrences (I1 and I2) are the nearest extension of the
  integrated PNG profile. They reuse BRN's image and relationship code, with exact
  bytes, alt/title and position for each occurrence. An ordinary JPEG would follow
  as its own decoder-validation decision. The owner has not selected that slice.
  No JPEG/PDF/PPTX work follows from this result.
- Keep these fixtures as acceptance witnesses. M7 and M8 already pass in BRN. I5
  and M2 need explicit per-part relationship scoping once headers are supported.
  M12 (ruby) and M15 must refuse until BRN models them.

## Limitations

- Synthetic fixtures only; no private Office files or Word-generated samples.
  Word writes the `w` prefix, so M7/M8 show namespace handling, not common Word
  output. M15's element is not in the schema. It stands for any element a reader
  doesn't model, which M12 shows with a real one.
- The oracle checks for sentinels in each reader's public output, plus image
  occurrence order and counts. It does not check text position; office_oxide's IR,
  for example, moves an inline image after its paragraph, while `as_docx()` keeps
  it in place. It covers only the APIs listed above. rdocx's facade walk skips
  tables, headers, notes, content controls, equations and revision contents.
- Any warning or unmodelled-item report counts as making a gap detectable, even
  when it is about something else. No case had an unrelated report; the dumps were
  checked. rdocx M15's "trace kept: unknownWrapper" comes from the adapter's own
  recorded `UnsupportedXml` name, which is the reader's report. The Picture check
  does not use the trace rule, and the Advertised check does not verify content
  (betteroffice's B1 content was checked by hand).
- I5 checks only that the body occurrence maps to the body bytes; the header
  image occurrence is not verified.
- Case I4 from the first pass (docx-rs's default `read_docx` without the `image`
  feature omitted a JPEG; commit `ff85ed8`) is not in the generic matrix, because
  it tests a docx-rs-specific option.
- B1 asks docx-rs for a 3.75 GiB allocation. That works on macOS because memory
  is committed lazily. On a host with strict overcommit the child may abort, which
  is recorded as Abort. A panic in the evaluator's own child code is also reported
  as Abort.
- Allocation figures come from the evaluator's counting allocator. They measure
  requested sizes, not resident memory.
- Only the docx-rs source was compared with its upstream commit; the other three
  were checked by crate checksum only.
- Not run: BRN workspace checks, since no product code changed. The BRN replay
  used a focused `brn-store --lib` test.

## Review and integration

**First pass (docx-rs only).** An independent read-only review re-ran the
evaluator, regenerated byte-identical fixtures, replayed them through BRN from a
`git archive` of `450eaa2`, and checked the stop witnesses against the release
source. It found no blocking defects. All findings were validated and addressed:
B7 resized into BRN's XML budget, image checks tightened, panic hook scoped,
wording corrected, a footer case added.

**Second pass (four readers).** Fixtures were split per case (`S1a`/`S1b`). M15 and
S4 were added. Unique trace ids were added to M2, M4, M6 and M10, and I6's alt text
was renamed. The harness moved to one child process per (case, reader), and it
now times only the parse call.

An independent read-only review of this pass found two blockers. Both were about
rdocx, and both were valid:

- **Rdocx adapter.** The rdocx adapter missed its reader facade. That facade
  reports unmodelled children as `UnsupportedXml` and exposes field instructions.
  Fix: the adapter now walks the facade, recording names and flags only.
- **Rdocx conclusions.** FINDINGS and the shared docs stated the resulting rdocx
  losses as fact. Fix: rdocx's M10, M12, M14 and M15 are now DetectableGap and M13
  is Retained, and the conclusions are rewritten around rdocx's actual blockers
  (S3 title, B5, dependencies, cancellation).

The other findings were also fixed:

- The structure check can now return DetectableGap.
- The office_oxide adapter now includes `as_docx()`, which makes I6 a
  DetectableGap.
- I5 was renamed to state its limit.
- The removal of I4 is recorded.
- `Misread` was split from `Lenient`.
- B4 accepts either alias.
- An empty filter now exits 1.

A focused re-review of these corrections found no remaining blocker. Its wording
fixes are applied: the title survives only in raw captured XML, B5 is an
internal-DTD entity, I6's IR omits the image, the coverage limits are wider, the
release count is corrected, and the I1–I3 summary is narrowed.

Adding S4 (tooltip) after the review showed that rdocx exposes tooltips through
`HyperlinkRef::tooltip`. The adapter now records them.

The focused result PR, its CI and merge are pending. This record does not claim
them.

## BRN replay

Temporary test appended to `crates/brn-store/src/work/inbox_source/docx/tests.rs`
and then reverted (not committed):

```rust
#[test]
fn h4_replay_evaluator_fixtures() {
    let Some(dir) = std::env::var_os("H4_FIXTURES") else { return };
    let mut names: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    names.sort();
    for path in names {
        let bytes = std::fs::read(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        match convert_source(&bytes, &AtomicBool::new(false)) {
            Ok(c) => {
                let v = c.visual.as_ref().map(|v| format!(" png={} alt={:?} title={:?}", v.part_name, v.alt_text, v.title)).unwrap_or_default();
                let body: String = c.body.chars().take(160).collect();
                println!("H4 {name}: Ok {:?}{v} body={body:?}", c.format);
            }
            Err(e) => println!("H4 {name}: {e:?}"),
        }
    }
}
```

```sh
mkdir -p /private/tmp/brn-h4b/fixtures
DOCX_EVAL_FIXTURES=/private/tmp/brn-h4b/fixtures cargo run --locked --release -- docx-rs   # in this directory
CARGO_TARGET_DIR=/private/tmp/brn-h4-eval/brn-target H4_FIXTURES=/private/tmp/brn-h4b/fixtures \
  cargo test -q -p brn-store --lib --locked --offline h4_replay -- --nocapture             # at the repository root
```
