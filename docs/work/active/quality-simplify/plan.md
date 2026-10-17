# Quality simplification batch — plan

**Completion note, 17 October 2026:** the original rendering-policy batch landed in [PR #146](https://github.com/ewq100/brn-rust/pull/146). A later owner-assigned formatting batch landed in [#153](https://github.com/ewq100/brn-rust/pull/153) and closed fourteen behavior-test gaps. The original scope description below is historical; its “healthy, tested” description of `threads_format.rs` is superseded by the later [mutation evidence](evidence.md). Neither record assigns more quality work.

**Goal:** One bounded code-quality pass: consolidate a genuinely duplicated
Markdown rendering policy, strengthen the tests that protect it, and add a
minimal cargo-mutants pilot for the touched surfaces. No product behavior
change, no schema/provider/dependency change, no release.

**Authority/spec:** Implementation assignment of 14 October 2026 (owner):
bounded quality/simplification pass with investigation, test strengthening,
refactor, verification and small reviewable commits; publication only with
separate authorization. The [Threads target](../../../architecture/threads-target.md)
owns product behavior. This plan does not assign unrelated work.

**Baseline:** fetched `main` at `771116f0` (PR #145 merge, equal to
`origin/main`), isolated worktree `work/quality-simplify`. Repository Rust pin
1.98.1, lockfile unchanged, GPUI Kit 0.7.3. The owner checkout stays on its
own branch and is not touched.

## Selected targets

1. **Duplicated Markdown rendering policy.** `markdown()` in
   `threads_ui.rs` and `safe_message_for_thread()` in `threads_writing.rs`
   each configured the same policy: `LiteralHtml` plugins with parser
   revision 1, image resolution from only the caller's stored asset map with
   an empty-PNG fallback, and an http(s)-only external link check. The
   copies could drift and the closures were untestable inline.
2. **Tests for that policy.** No test failed when the plugin or link check
   was mutated away (confirmed by the pilot below). Add behavior tests
   through the real note Read view plus unit tests for the extracted
   helpers.
3. **cargo-mutants pilot** on exactly these surfaces, documented in
   [evidence.md](evidence.md), leaving reusable commands rather than a
   repository-wide campaign.

Out of scope: `threads_format.rs` and other pure modules (healthy, tested),
test-fixture consolidation (each fixture varies meaningfully), the
source-locator http(s) checks in `threads_writing.rs` (a different trust
boundary: explicit controls over trusted stored locators, not untrusted
rendered Markdown).
