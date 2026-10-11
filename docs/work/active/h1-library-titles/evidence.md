# H1 library titles — task record

The owner selected H1 on 2026-10-06. Only H1 is in scope. The rest of the
paused V1 roadmap stays paused. The spec is
[H1 in next specs](../preparation-checkpoint/next-specs.md#h1--titles-from-saved-markdown),
plus the queue entry in the [V1 handoff](../v1-handoff.md#h1--reuse-markdown-title-parsing)
and [reuse decision D](../../../architecture/decisions/2026-10-06-compatible-reuse.md#d--markdown-titlebody-mechanisms-accepted-later-task-h1).

## Baseline and authorization

- I fetched origin. Preparation [PR79](https://github.com/ewq100/brn-rust/pull/79)
  was OPEN at first check (head `9486c19`). The owner merged it during this task,
  at 2026-10-06T18:24:49Z, as `450eaa2dcd4d9571ae749c57806a192a4dd8e435`.
  The owner chose to wait for that merge, so the baseline is `origin/main` at `450eaa2`.
  `git diff a8deb9d 450eaa2 -- crates Cargo.toml Cargo.lock scripts .github` is
  empty, so the product code is still PR77's.
- The branch is `codex/h1-library-titles`, in its own worktree. The original checkout,
  other worktrees and unrelated `.DS_Store` files were left alone.
- The owner authorized a normal merge of the H1 PR, and only that PR, once the
  required CI passes at the exact head. Nothing here authorizes live calls, model
  downloads, private data or account actions, and none happened.
- Environment: Mac mini (Darwin arm64), pinned Rust 1.98.1, locked dependencies.
  `TMPDIR=/private/tmp/brn-h1-titles/tmp`,
  `CARGO_TARGET_DIR=/private/tmp/brn-h1-titles/target`. The registry cache was
  missing locked `zip 8.6.0` and `typed-path 0.12.3`, so I ran `cargo fetch --locked`
  once. That is an ordinary crates.io dependency fetch, with no model and no provider.

## Reuse decision

| Need | Decision | Evidence |
| --- | --- | --- |
| Body start after a supported header | **Reuse** Store `note_identity::body_start` | Handles BOM, `---`/`...` closings, unmanaged thematic breaks and unclosed headers |
| Body when Store refuses a malformed managed layout | **Reuse** the existing `legacy_body_start` (exact `---` framing), moved unchanged from link approval into `library.rs` and shared | Link approval already uses it for the same compatibility case. It is not a second YAML interpreter |
| Which lines are headings | **Reuse** pinned `markdown` 1.0.0 mdast with byte positions | Same parser and position handling as `knowledge/links/extract.rs` |
| Parse cost and parser panic | **Adapt** with inline constructs off, a parse that stops at the last literal `# ` line, and `catch_unwind` with filename fallback | See review finding 1 and 2 below |
| Literal title policy | **Build** a small adapter in `library.rs` | 50-line window, the exact `# ` line rule and the raw trimmed suffix are BRN policy |

No dependency, manifest or lockfile change.

## Changed behavior

Titles still come from the first nonempty literal `# ` line in the first 50
physical saved lines. Three cases change on purpose:

- `# ` lines inside fenced code, indented code or HTML blocks are no longer titles.
- A `...` closing ends the header, so YAML comment lines such as `# x` inside it
  are no longer titles.
- When the header ends after line 50, the title falls back to the file name. Before,
  YAML comment lines inside that header could become the title.

Refresh also re-derives titles for notes whose bytes have not changed. If the stored
title differs, it re-indexes the note through the existing upsert path and reports
it as `updated`. Without this, existing indexes would keep the old wrong titles
until each note's bytes changed. The upsert drops that note's cached passage
vectors and derived edges one time, and the next refresh after that reports it as
unchanged.

The [brn-workflow contract](https://github.com/ewq100/brn-rust/blob/a1c61e245ec90a0eed98ffc5bbfcab57ddeb1926/crates/brn-workflow/README.md#simple-app-owner-and-read-tools)
documents this policy. UI, CLI, retrieval ranking, metadata classification and
scope admission are unchanged. Notes with invalid managed metadata are still indexed
and reported as before, and stay out of every scope.

## Candidate and verification

Code: `crates/brn-workflow/src/library.rs` (title adapter, shared legacy framing,
unchanged-note re-titling), `src/knowledge/links/approval.rs` (now imports the moved
adapter). Tests: `tests/library.rs` (8 new), `tests/ai_tools.rs` (1 new list/search
witness), and one private unit test in `library.rs` for malformed layouts. Their
titles cannot be seen through any scope.

The new tests cover these inputs through `Library::refresh` and `notes_scoped`:

- backtick, tilde and unclosed fences, HTML blocks and indented code
- raw inline text, Unicode, rejected forms (setext, indented, `#NoSpace`, `#`-tab,
  `##`, quote, empty heading)
- `...` closings with BOM/CRLF, unclosed unmanaged headers, Source scope
- the line 50/51 limit with and without a header, a header ending at line 51, a
  fence that crosses the window
- parser panic fallback, including a title kept ahead of the panic sequence
- an inline-heavy note of more than 400 KB (bounded at 30 s; measured 0.21 s)
- replacement of stale stored titles

They also check exact bytes and hashes.

| Check | Tree | Result |
| --- | --- | --- |
| New tests against baseline `library.rs`/`approval.rs` | candidate-2 tests | 5 failed as expected: fenced, `...`, long header, stale title, ai_tools list |
| `cargo test -p brn-workflow --test library --test ai_tools --locked --offline` | final | library 17/0, ai_tools 6/0 |
| `cargo test -p brn-workflow --lib --locked --offline library::` | final | 1/0 |
| `cargo test -p brn-workflow --test knowledge_links --test knowledge_link_preparation --locked --offline` (moved adapter) | final | 14/0/1 ignored, 7/0 |
| `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | final | passed |
| `bash scripts/verify-end-to-end.sh` (retirement, workspace fmt/build/Clippy/tests, fixtures) | candidate-1 | exit 0; 1,616 passed, 0 failed, 16 ignored, plus 52 fixture assertions |
| same | candidate-2 | exit 0; 1,618 passed, 0 failed, 16 ignored, plus 52 fixture assertions |
| `git diff --check`; `python3 scripts/check-markdown-links.py` | final | passed |

After candidate-2's shared gate, the only Rust change was one added case in a
`tests/library.rs` test (the title before the panic sequence). The focused,
format and Clippy rows above ran on that final tree. The 16 ignored tests are the
existing documented exclusions. No native-feature, GUI, live-provider or model
check ran. None of them applies, because the change has no display, feature or
provider path beyond the corrected title string.

## Independent review

A read-only reviewer reviewed all of candidate-1 against this spec. It probed the
pinned parser with standalone programs and differential fuzzing. I checked each
finding myself before fixing it.

1. **Blocker, fixed.** Pinned markdown 1.0.0 panics with `Cannot push to non-parent`
   on a valid setext heading, an adjacent `---` break and another setext heading.
   For example `Meeting notes\n-------------\n---\nAgenda\n------\n# Title\n`. That
   panic would abort every refresh. I reproduced it. The parse now runs in
   `catch_unwind`, and on a panic the title falls back to the file name, as the spec
   requires for parse failures. A regression test covers it.
2. **Defect, fixed.** Default inline parsing is quadratic on some valid text. The
   same debug build took 129.6 s for a 400 KB note with default options and 0.21 s
   with the adapted parse. The reviewer's differential over 500k documents found
   0 heading-position differences with inline constructs off. The re-review's 2.3M
   release and 100k debug cases found 0 differences for the cut plus the disabled
   constructs.
3. **Advisory, accepted.** Re-titling unchanged notes. Recorded above.
4. **Advisory, accepted.** The behavior changes the spec asks for. Recorded above.
   One more: a type-7 HTML line such as `<br>` hides a `# ` line that follows it,
   as CommonMark specifies.
5. **Advisory, fixed.** I made the thin test assertions stronger. Exact search quote
   and span checks are in.

A focused re-review of candidate-2 found no blocker or defect. Its advisories:

1. **Stderr noise, recorded limitation.** A parser panic still prints the default
   panic hook's message to stderr on every refresh that sees such a note. Changing
   the process-wide hook from refresh would race other threads, so I left it alone.
2. **Fixed.** No test covered the prefix cut. I added a case with the title before
   the panic sequence.
3. **Kept.** The fallback test pins a parser bug, and a comment says so. If a future
   parser upgrade fixes the bug, that test will fail on purpose.

## Limitations and follow-ups outside H1

- Deeply nested block quotes (a 30 KB line of `>`) are still slow to parse in the
  pinned parser, about 19 s in the reviewer's optimised build. The prefix cut only
  helps when such lines come after the last `# ` line.
- Link extraction (`knowledge/links/extract.rs`) calls the same parser without
  `catch_unwind`. That path was already exposed to the panic before H1 and is
  unchanged. A shared guarded parse or an upstream parser fix would be a separate
  task.

## Acceptance

There is nothing to see in the GUI beyond a corrected title string. To check it
by hand, put a note `n.md` with the bytes `` ```md\n# Fake\n```\n# Real\n`` in a
fresh disposable vault. Then run
`brn notes list --data-dir <new absolute data dir> --vault <vault> --json`. I ran
exactly that with the candidate's debug binary on synthetic data. It listed `n.md`
with title `Real`, and the note's SHA-256 was the same before and after
(`33e47682…f59e780`). Owner acceptance is **pending**. Native GUI observation is
not needed for H1 and was not done.

## Integration

Pending. The PR, its exact-head CI, the merge and the post-merge check are recorded
below when they happen.
