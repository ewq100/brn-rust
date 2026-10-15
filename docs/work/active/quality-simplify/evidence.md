# Quality simplification batch — evidence

**Date:** 14 October 2026. **Baseline:** `771116f0` (fetched `main`, equal to
`origin/main`). **Branch:** `work/quality-simplify` (isolated worktree
`.worktrees/quality-simplify`; the owner checkout on `main` was left
untouched). Repository Rust pin 1.98.1, lockfile unchanged, GPUI Kit 0.7.3 /
GPUI 0.3.9, offline `--locked` builds throughout. Candidate is the commit
containing this record.

## Implemented

`safe_markdown()` in `crates/brn-desktop/src/threads_ui.rs` now owns the one
Markdown policy shared by every rendered note and message: literal-HTML
plugins with parser revision 1, image resolution through `stored_image()`
(only the caller's own asset map; everything else falls back to an empty PNG
placeholder), and `external_link()` gating which links may open. `markdown()`
keeps the note Read view's typography and scrolling; `safe_message_for_thread()`
keeps the message construction and its chat-scoped asset map. Diff stats
(`git diff --numstat 771116f0..HEAD`, the two UI files): +89/−47 total,
of which +41 added lines are the `policy_tests` module — production is
**+48/−47** (`threads_ui.rs` +44/−25, `threads_writing.rs` +4/−22):
essentially line-neutral, as expected when replacing two copies with one
function plus two named helpers. The value is the single policy location and
testability, not line count. Tests add 136 lines (95 Read-view journey, 41
unit). No behavior change intended; the source-locator http(s) checks in
`threads_writing.rs` were deliberately not consolidated (explicit controls
over trusted stored locators are a different trust boundary than untrusted
rendered Markdown).

Tests: `read_view_shows_block_html_literally_and_renders_all_link_text`
(real note Read view, stored and missing figures, https and javascript
links, exact rendered plain copy) and unit tests for `external_link`/
`stored_image` in `threads_ui.rs::policy_tests`.

## Mutation pilot (cargo-mutants 27.1.0)

cargo-mutants 27.1.0 was installed with `cargo install cargo-mutants
--locked` as a development tool only; no manifest or lockfile change. Runs
happened inside the isolated worktree; cargo-mutants copies the tree to
disposable scratch directories and applies each mutation there, so no
mutated source reached any checkout. Every run used the package suite
`cargo test -p brn-desktop --features native-ui,native-test-support`
(tempdir fixtures only, no real data, no provider).

**Run 1 — before the refactor** (baseline `771116f0`):

```
cargo mutants -p brn-desktop --features native-ui,native-test-support \
  -f 'crates/brn-desktop/src/threads_ui.rs' \
  -f 'crates/brn-desktop/src/threads_writing.rs' \
  -f 'crates/brn-desktop/src/threads_chat.rs' \
  -F 'LiteralHtml|markdown -> TextView|safe_message|assets_in_chat|note_assets' \
  -j 6 -t 300 --cargo-arg=--locked --cargo-arg=--offline -o <out>
```

27 mutants (18 selected + 9 struct-field drag; see the quirk below):
**15 missed, 4 caught, 8 unviable** (23 min). The selected-area misses were
six: three genuine gaps and three headless-unobservable survivors.

- **Caught (4):** the asset-visibility boundary (`assets_in_chat`/
  `note_assets` emptied or `&&`→`||`) is protected by the existing
  private-asset tests.
- **Genuine gaps (3):** `LiteralHtml::parse → None`, the http(s) link check
  `|| → &&` in `safe_message_for_thread`, and `is_block → false` survived —
  no test covered the rendering policy. All three are closed by the new
  tests (see run 2). The Read-view test was also demonstrated by hand:
  passes on unmodified production code, fails under `parse → None` on the
  block-HTML literalness assertion.
- **Headless-unobservable (3):** `LiteralHtml::name` ×2 (renderer identity
  only) and `is_block → true` (inline-plugin registration; inline literal
  glyphs are excluded from the plain copy by Kit design). Native
  observation remains the gate for these.
- **Unviable (8):** `Default::default()`/`from_iter` replacements that do
  not compile. Not test evidence.
- **Out-of-scope drag (9):** `delete field …` struct-literal mutants.
  cargo-mutants 27.1.0 generates these outside the name-filter mechanism
  (`visit_expr_struct` bypasses `allows_mutant`; file globs still bound
  them), so they ride along with any `-F` selection. They hit diff-highlight
  styling and window options — not required-behavior assertions; recorded,
  not acted on.
- **Scope note:** the `-F` pattern did not match `markdown()`'s own link
  check (`threads_ui.rs:290:44` pre-refactor). After the refactor that
  expression lives in `external_link`, inside run 2's scope.

**Run 2 — after the refactor** (candidate commit; same package/features):

```
cargo mutants -p brn-desktop --features native-ui,native-test-support \
  -f 'crates/brn-desktop/src/threads_ui.rs' \
  -f 'crates/brn-desktop/src/threads_writing.rs' \
  -f 'crates/brn-desktop/src/threads_chat.rs' \
  -F 'LiteralHtml|safe_markdown|markdown -> TextView|external_link|stored_image|safe_message|assets_in_chat|note_assets' \
  -j 6 -t 300 --cargo-arg=--locked --cargo-arg=--offline -o <out>
```

31 mutants (22 selected + 9 drag): **12 missed, 9 caught, 10 unviable**
(11 min). Selected-area view: 9 caught, 3 missed, 10 unviable.

- **Caught (9):** `external_link` → true / → false / `||`→`&&` (unit
  tests), `LiteralHtml::parse → None` and `is_block → false` (Read-view
  test), and the four asset-visibility mutants above. All three run-1 gaps
  are confirmed closed; mutation locations moved into the extracted helpers
  as expected.
- **Missed (3 in area):** `name` ×2 and `is_block → true` — the same
  headless-unobservable survivors classified in run 1. Plus the 9 dragged
  struct-field mutants (styling/window options), unchanged.
- **Unviable (10):** non-compiling replacements (`TextView`, `Arc<Image>`,
  map literals, `MarkdownNode::default()`); compile failures, not
  caught-behavior evidence.
- **Not mutated:** `stored_image`'s fallback construction (method-call
  chain no mutant genre covers) is covered by its unit test;
  `parser_revision(1)` and builder-chain calls generate no mutants.

Interpretation: every runnable mutation of the shared policy's
security-relevant decisions — asset visibility, scheme gate, literal block
HTML — is now caught. Visual-only properties (inline HTML glyphs, styling,
link-cursor behavior) remain native checks. Mutation testing is evidence
about these selected tests, not proof of overall correctness.

## Automated verification

All commands ran in the isolated worktree on the final candidate commit:

- `cargo test -p brn-desktop --features native-ui,native-test-support --locked --offline`:
  **150 passed, 0 failed** (147 baseline + 3 new). The same suite passed
  with 147 tests at the untouched baseline before any change.
- `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings`:
  **passed** (the pre-existing `block 0.1.6` future-incompatibility notice
  from pinned dependencies remains, unrelated to this change).
- `cargo fmt --check`: **passed** after formatting the new code.
- `git diff --check`: **passed**.
- `python3 scripts/check-markdown-links.py`: **passed** including the new
  task documents.

## Incident record

Mid-task, one batch of shell commands briefly ran against the owner checkout
instead of the task worktree (each shell starts in the original directory).
It added one local commit to the owner's `main` and one stash entry; no
owner content was modified or lost, nothing was pushed. The commit was
cherry-picked onto `work/quality-simplify` (as `331a537b`), the stash
recovered into the worktree, the owner's `main` reset to its original HEAD
`771116f0`, and the extra stash dropped; the owner's own pre-existing stash
and untracked files were verified untouched. All subsequent work used the
worktree explicitly.

## Independent review

An independent read-only review (reviewer agent, separate context; no file
access beyond the diff and logs) covered behavior preservation, duplication
removal, abstraction simplicity, test strength, docs accuracy and safety
boundaries. Verdict: **approved**. It verified builder-order semantics
against GPUI Kit 0.7.3 sources, confirmed `safe_markdown` is the crate's
only policy registration point, confirmed the asset-visibility functions
are untouched, and reproduced the mutation-run counts from the artifacts.
Two low-severity documentation findings — imprecise diff-stat arithmetic
and an undercounted run-1 gap list (`is_block → false` was a third genuine
gap) — are fixed in this record. Human owner review remains open with the
handoff.

## Gate separation

- **Implemented:** policy consolidation + tests + bounded mutation pilot on
  `work/quality-simplify`.
- **Automatically verified:** desktop native suite, strict clippy, fmt,
  whitespace and link checks as listed above.
- **Mutation-tested:** exactly the scope above (policy helpers, literal-HTML
  plugin, asset-visibility functions in `threads_chat.rs`); not a
  repository-wide result.
- **Native-observed:** no — inline HTML literalness and link-cursor
  behavior remain native checks; collect with the existing owner UI
  qualification checklist.
- **Owner-accepted / merged / published:** opened as
  [PR #146](https://github.com/ewq100/brn-rust/pull/146) with explicit
  owner authorization and merged at `b139221f` after all four required PR
  contexts passed. Merged-tree verification and required main CI (run
  38693947454) are green; receipts are recorded on the PR. Non-required
  optional-platform failures (windows, ubuntu native-retrieval) match the
  pre-existing main pattern. Integration recorded in
  [status](../../../status.md).

## Second batch — formatting rules and deferred items (15 October 2026)

Owner requested the deferred list be executed. Branch
`work/quality-format-batch` from merged main `8bef439b`.

### Formatting mutation area (`threads_format.rs`)

Full-file pilot on the merged-main baseline (same command shape as above,
`-f crates/brn-desktop/src/threads_format.rs`; 175 mutants, 24 min):
**147 caught, 21 missed, 4 unviable, 3 timeouts.**

Investigation of the 21 survivors classified:

- **12 genuine gaps** in required behavior, closed by new behavior tests
  (commit `df21d6fd` plus the follow-up below): a mid-line selection on a
  non-final line (the most common editing case, never exercised — the
  `line_end` arithmetic could be replaced with a panicking variant);
  one-sided, exact and neighboring-marker emphasis selections (refused, not
  mangled); complete-marker unwrap at line start and on later lines; a fence
  with an info string still enclosing following lines; four-space content
  inside a quote being indented code, not a fence; deeply indented container
  content staying untouched; selections mapping across every prefix newline
  and anchoring inside removed prefixes; reversed selections at valid
  boundaries; syntax elsewhere on the line; a tab-indented short line after
  short lines still being code.
- **3 timeouts** (`prefix_len → Some(0)`, `strip_ordered_list → Some("")/
  Some("xyzzy")`): infinite/looping replacements; the timeout budget
  terminates them. Not counted as caught behavior.
- **4 unviable**: non-compiling `Default::default()` replacements.
- **equivalent survivors**, each with the reason: `inline` threshold
  `>`→`>=` and `*`→`+` (the exact-length cases where they differ cannot have
  both marker edges, or are refused either way); `selection.end + n`→`−`
  (the newline between the line end and any following text can never equal
  the marker); `map_offset` `<`→`<=` and `+ new_len`→`− new_len` (reachable
  inside-change cases always have `new_len == 0` in remove mode, and the
  boundary case accumulates identically); `safe_lines` break `>`→`==`/`>=`
  (`offset == target.end` is unreachable at the break because inline/block
  targets end at the newline, not after it); `strip_containers` `>`→`>=`
  (content indent 3 classifies identically to 0 in every consumer; probed
  directly under both mutants).

Re-run after the first test commit (175 mutants, 37 min under heavy external
machine load): **159 caught, 9 missed**, timeouts/unviable unchanged — the
12 targeted mutants are caught, no catch regressed. Two further tests closed
`safe_lines` offset advance (`242:30`) and pinned the true behavior of
indented pseudo-fences.

Final full re-run on the finished test set (175 mutants, 39 min, same
external load): **160 caught, 8 missed, 4 unviable, 3 timeouts**. The eight
missed are exactly the equivalents classified above; the two additional
tests caught `242:30` (verified independently by applying that single
mutation) and no previously caught mutant regressed.

### Deferred items resolved without code changes

- **Styling mutants** (diff-highlight/proposal-card `HighlightStyle`
  fields): investigated against GPUI Kit 0.7.3's public API — decoration
  collections expose `get_ranges` only, with no style getter, and the plain
  copy excludes styling; wholesale decoration removal would be observable,
  individual style fields are not. Stays a native/snapshot check; no test
  machinery added.
- **cargo-mutants filter quirk**: 27.1.0 is the newest published version
  (crates.io search; the upstream repository is not reachable from this
  environment), so no upgrade exists yet. The documented workaround —
  file globs bound the `delete field` drag that bypasses name filters —
  remains in effect.

