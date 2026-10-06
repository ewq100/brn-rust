# H4 findings: docx-rs 0.4.22 as BRN's DOCX reader

**Recommendation: do not adopt.** Keep BRN's own bounded converter (decision
**build**, unchanged). Evaluated 2026-10-06 for the
[H4 task](../../docs/work/active/v1-handoff.md#h4--evaluate-published-docx-reader-before-broader-stage8).
This is a result record. It changes no product code. Review and integration
state is under [review and integration](#review-and-integration).

## Question and stop rule

Can immutable docx-rs 0.4.22 replace a meaningful part of BRN's OOXML
interpretation behind a small admission/mapping adapter? The task stops on the
first irrecoverable silent loss, unavailable original asset or occurrence,
unbounded allocation, or need for a second full interpreter.

The stop fired on the first silent-loss case, S3, in BRN's supported profile.
The reader drops the inline picture's alt text and title, which BRN preserves
today. (S0, the case before it, is a false refusal: docx-rs requires a part BRN
treats as optional. That is a compatibility gap, not a loss.) More general
witnesses followed. M7 and M8 drop wording when WordprocessingML uses a prefix
other than `w`. M4, M10 and M13 drop footnote text, altChunk content and a field's
link destination with nothing left in the model. An adapter can only catch these
by walking every XML element itself. That walk is BRN's current `document.rs` and
`image.rs`, so adoption would mean running two interpreters. All observations come
from one harness run. No adapter was built.

## Identity and environment

| Item | Value |
| --- | --- |
| Baseline | `origin/main` `450eaa2` (PR79 merge), worktree branch `codex/h4-docx-reader-evaluation` |
| Candidate | crates.io `docx-rs` 0.4.22, latest published release at evaluation time |
| Release checksum | index `cksum`, downloaded `.crate` SHA-256 and the evaluator lock all `7fdf00e8af6d0b3e92d4bbf9b76f773d8b84ea80f310324ad16cbdc2e653e02c` |
| Source identity | `.cargo_vcs_info.json` names `bokuweb/docx-rs` `f04cf8b47a925b2b6fff850dcb75069aae89dab4` (`docx-core`), 2026-07-21. `diff -r` of the crate `src/` against that commit's `docx-core/src`: identical. `Cargo.toml.orig` identical. `git ls-remote --tags` on 2026-10-06 listed 23 tags, the newest `0.4.21`; there is no `0.4.22` tag. |
| Unreleased main | `4ff72dc8` (the ADR's earlier inspection) adds ASCII part-name validation and picture rotation. `read_zip` still sizes buffers from the header and unwraps reads. The run reader still dispatches on the literal `w` prefix. Conclusions below are for the release. |
| Features | `default-features = false`, so the `image` preview decoder is off. BRN would call `read_docx_with_options(bytes, ReadDocxOptions::default().with_image_previews(false))`. |
| Platform | Darwin 25.5.0 arm64, Rust 1.98.1 (pinned), evaluator lock in this directory, target `/private/tmp/brn-h4-eval/target` |
| Dependency cost | Against BRN's lock, the evaluator adds only `docx-rs` 0.4.22 and `zopfli` 0.8.3 (from docx-rs's `zip/deflate` feature). It would also unify `quick-xml/encoding` (pulling `encoding_rs`) into BRN's graph. |

## Commands and results

From `experiments/docx-reader-eval`, with `CARGO_TARGET_DIR=/private/tmp/brn-h4-eval/target`:

```sh
cargo fmt --check                                     # passed
cargo clippy --locked --all-targets -- -D warnings    # passed
cargo run --locked --release                          # exit 0, 31 rows, 0 unexpected
```

Each case asserts the verdict recorded in `src/main.rs`. A different observation
prints `(unexpected)` and exits 1. Verdicts come from the read model: the
compact `Docx::json()` (only its `document` subtree where order or occurrence
counts matter), its derived `Debug` form, and `Docx::images`. A sentinel missing
from all of these cannot be recovered by an adapter that only consumes docx-rs
output.

The BRN column came from replaying the identical bytes (`DOCX_EVAL_FIXTURES=DIR`)
through BRN's `convert_source` at `450eaa2`, using the temporary test under
[BRN replay](#brn-replay). That run had 1 passed, 0 failed, and the test was removed
afterwards.

| ID | Case | BRN at 450eaa2 | docx-rs 0.4.22 | Observation |
| --- | --- | --- | --- | --- |
| S0 | Minimal package, no `word/_rels/document.xml.rels` | Ok DocxTextV1 | Refused | `Err(ZipError(FileNotFound))`. The reader requires that optional part. |
| S1 | Unicode paragraphs, Stored and Deflate | Ok, exact | Retained | |
| S2 | Heading style, numbered list, external link, simple table | Ok, exact | Retained | Style id, `%1)` level text, decoded link target and cells all in the model |
| **S3** | **One inline PNG with alt text and title** | Ok DocxInlinePngV1, exact bytes/position/alt/title | **SilentLoss** | Bytes and rId order kept. `wp:docPr` `descr` and `title` are absent; `Pic` has no field for them. |
| M1 | Final-section default header | docx_unsupported | Retained | |
| M2 | Header referenced only by an earlier section | docx_unsupported | DetectableGap | Header text dropped; the paragraph `sectPr` keeps `headerReference` id `h1` with no content |
| M3 | Final-section default footer | docx_unsupported | Retained | |
| M4 | `w:footnoteReference` in the same run as text, plus footnotes part | docx_unsupported | SilentLoss | Footnote text and the reference both gone; the run holds only `Claim`. 0.4.22 never reads footnotes. |
| M5 | Comment range, reference and comments part | docx_unsupported | Retained | |
| M6 | Tracked `w:ins`/`w:del` | docx_unsupported | Retained | Kept as Insert/Delete, so an adapter could refuse them |
| **M7** | **Second prefix bound to the WordprocessingML URI inside a `w:` run** | Ok, both wordings | **SilentLoss** | `<x:t>` text vanishes. `Run::read` only matches prefix `w`. No trace remains. |
| **M8** | **WordprocessingML as the default namespace** | Ok, wording kept | **SilentLoss** | Paragraph and run shells exist, all text gone |
| M9 | `w` prefix bound to a foreign namespace | docx_unsupported | Misread | Non-OOXML wording read as document text. Body dispatch uses local names only. |
| M10 | Body `w:altChunk` with imported part | docx_unsupported | SilentLoss | Element and part both absent |
| M11 | VML text box (`w:pict/v:shape/v:textbox`) | docx_unsupported | DetectableGap | Text dropped; a bare `shape` child with only `style` remains |
| M12 | Ruby annotation | docx_unsupported | Flattened | Phonetic guide and base become two consecutive plain runs |
| M13 | `w:fldSimple` HYPERLINK | docx_unsupported | SilentLoss | Result text kept as a plain run; instruction and link destination gone |
| M14 | Inline `w:customXml` | docx_unsupported | Flattened | Wording kept; wrapper gone |
| B1 | Entry advertising 3.75 GiB (`0xF0000000`) uncompressed | docx_limit | AdvertisedAllocation | Read succeeds after one 4,026,531,840-byte allocation (`Vec::with_capacity(entry.size())`) |
| B2 | Stored entry with a corrupted byte (CRC mismatch) | docx_invalid | Panic | `read_zip` unwraps `read_to_end`: "Invalid checksum" |
| B3 | Main part named `word\document.xml` | docx_invalid | Refused | Not found |
| B4 | Case alias `WORD/document.xml` beside `word/document.xml` | docx_invalid | Misread | One alias read, the other ignored |
| B5 | Internal DTD entity | docx_unsupported | Refused | `XMLReadError` |
| B6 | UTF-16 main part | docx_unsupported | Refused | `XMLReadError` |
| B7 | 7,464,492-byte XML body, 8,001 paragraphs, 48,016 `<`/`=` delimiters (inside BRN's 8 MiB and 50,000-delimiter XML guard) | docx_limit (1 MiB rendered-output cap) | Retained | 14 ms (release) in one call with no cancellation hook; 42 MiB peak heap |
| I1 | Two PNGs, two relationships, two occurrences | docx_unsupported | Retained | Exact bytes; both drawings in document order by rId |
| I2 | One relationship used twice | docx_unsupported | Retained | One asset; exactly two drawings in the document name it |
| I3 | Ordinary 8x8 JPEG, previews off | docx_unsupported | Retained | Exact original bytes |
| I4 | Same JPEG through default `read_docx` without the `image` feature | docx_unsupported | SilentLoss | Image missing from `Docx::images`; the drawing remains |
| I5 | Header image and body image both `rId1` | docx_unsupported | Ambiguous | Two `images` entries share id `rId1`, header first. A body drawing cannot be mapped to its bytes from the model. |
| I6 | Image relationship to a missing part | docx_invalid | DetectableGap | No image entry; the drawing still names the rId |

Source-only observations, not executed: `read_headers`/`read_footers` and
`add_images` discard parse or read failures with `filter_map`/`if let Ok`;
`Paragraph::read` drops a failing `pPr`; `read_width` panics via `expect` on a
non-numeric width. Through ZIP64, an entry can advertise more than ZIP32's
4 GiB. Above `isize::MAX` the `Vec::with_capacity` call panics, and a smaller
allocation the host cannot satisfy aborts the process. None of these changes the
decision.

## Adapter scope if adopted anyway

docx-rs reads the content BRN already supports: paragraph text, styles,
numbering, external links, simple tables, comments, tracked changes and raw image
bytes. That is not enough. To keep "preserve completely or refuse explicitly",
BRN would still have to keep the following production code. This is an estimate
from the evidence; no adapter was built.

| BRN production code (lines) | Why it stays |
| --- | --- |
| `package.rs` (339) | B1 advertised allocation, B2 panic, B4 alias acceptance. Every byte must pass BRN's inventory before docx-rs sees it, and docx-rs then inflates every part a second time. |
| `xml.rs` (92) | docx-rs has no item or depth limit and no cancellation. Every part would be parsed twice. |
| `opc.rs` (580) | docx-rs parses `[Content_Types].xml` and then discards it, and it never looks at unreferenced parts. M4 and M10 are only detectable from relationships. |
| `image.rs` (822) | docx-rs does not validate PNGs, and S3 means BRN must parse `wp:inline`/`docPr` itself |
| `document.rs` (about 1,370) | M7, M8, M9, M12, M13 and M14 are undetectable in the model. Only a strict namespace-aware walk that refuses unknown elements catches them, and that walk is this file. |
| `docx.rs` (111) | Entry points and failure mapping |

That is about 3,300 lines. Adoption would add a dependency and a second parse
while all of it stays.

## Next Stage8 acceptance

- Broader DOCX support extends BRN's existing strict converter. A future reader
  library needs a new evaluation that covers this matrix first: namespace-correct
  dispatch, explicit unknown-element reporting, caller-supplied size limits and
  cancellation, and no panics on corrupt input.
- Multiple inline raster occurrences (the I1 and I2 shapes) are the nearest
  extension of the integrated PNG profile. They reuse BRN's existing image and
  relationship code, with exact bytes, alt/title and position for each occurrence.
  An ordinary JPEG would follow as its own decoder-validation decision. The owner
  has not selected that slice, and no JPEG/PDF/PPTX work follows from this result.
- Keep the evaluator fixtures as acceptance witnesses for any later slice that
  touches these shapes. M7 and M8 already pass in BRN. I5 and M2 need explicit
  per-part relationship scoping once headers are supported.

## Limitations

- Synthetic fixtures only; no private Office files or Word-generated samples.
  Word itself always writes the `w` prefix. M7/M8 show that the reader is not
  namespace-correct, not that common Word files lose text.
- The sentinel oracle checks presence, not position, except in S3, I1 and I2.
  "Retained" means the wording is somewhere in the model, not that BRN-equivalent
  Markdown would follow.
- B1 succeeds on macOS because the 3.75 GiB allocation is committed lazily. On a
  host with strict overcommit, the evaluator itself could abort there.
- B7 timing is from one release-mode run on this Mac and is only a rough figure.
- Allocation figures come from the evaluator's counting global allocator. They
  measure requested sizes, not resident memory.
- Not run: BRN workspace checks, since no product code changed. The BRN replay
  used a focused `brn-store --lib` test.

## Review and integration

Independent read-only review (a separate agent with its own Cargo target) re-ran
the evaluator, regenerated byte-identical fixtures, replayed them through BRN at
`450eaa2` from a `git archive` copy, and checked S3, M4, M7, M8, M10, B1 and B2
against the release source. It found no blocking defects. Its findings were all
validated and addressed:

- B7 was outside BRN's XML guard (72,016 delimiters). It was resized to fit, and
  BRN's refusal reason is now the output cap.
- I1, I2 and S3 checks could pass on wrong output. They now use the `document`
  subtree, an exact count of two, and distinct alt and title sentinels. The oracle
  also searches the derived `Debug` form.
- Wording fixes: "first silent-loss case", 3.75 GiB, `read_docx_with_options`,
  the content-types and DTD wording, line totals labelled as an estimate, and how
  the tag list was checked. M13 was relabelled SilentLoss. A footer case (M3)
  was added.
- The evaluator now silences panics only around the reader call, and pins
  `serde_json`.
- Shared docs now say "evaluated, not adopted" and leave integration pending.

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
mkdir -p /private/tmp/brn-h4-eval/fixtures
DOCX_EVAL_FIXTURES=/private/tmp/brn-h4-eval/fixtures cargo run --locked --release   # in this directory
CARGO_TARGET_DIR=/private/tmp/brn-h4-eval/brn-target H4_FIXTURES=/private/tmp/brn-h4-eval/fixtures \
  cargo test -q -p brn-store --lib --locked --offline h4_replay -- --nocapture       # at the repository root
```
