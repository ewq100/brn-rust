# PR217 Ubuntu Note-exit setup — bounded independent reassessment

**Approve the proposed test-only selection correction for a selected bounded requalification.** The diagnosed cause matches actual source and evidence; do not change production admission, expected bytes, assertions, platform coverage or ignored-test policy. This portability gap was missed in my earlier complete-candidate review.

Reviewed clean `155bb86776eea697135de8d6782bddc4481b3de0` in `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`, branch `codex/note-exit-preservation-20261022`. Read the actual journey/setup and pinned Kit/Base 0.7.3 input implementation. No edits, builds, tests, network/native/account actions performed.

## Actual failure and cause

Independently recomputed `/tmp/brn-pr217-ubuntu-ui/job.log` SHA256: `46d49feb5f1e3209e45987540415aa595aa7d260524e55ba2a2c8063b137442c`, matching the supplied identity. Ubuntu's new journey failed at `threads_writing_tests.rs:1827`, before Close: actual whole editor text `"õtere tere õun"`, expected `"õ"`. The toolkit suite ended 346 PASS / 1 FAIL / 1 ignored / 3 filtered. Preserve this original log/failure. This is neither a SQLite crash nor an exit-admission result.

The fixture focuses the editor then unconditionally presses `cmd-a` (`threads_writing_tests.rs:1813–1820`). Pinned `gpui-base-0.7.3/src/input/base/state.rs:248–251` binds `cmd-a` to SelectAll only on macOS; non-macOS binds `ctrl-a`. Therefore Ubuntu did not select the saved body. The first real `õ` inserted at the existing cursor; its checked first-dirty refusal then made the editor readonly, leaving the reported prefix-plus-saved-body. The failure is explained without a new product/data-loss hypothesis.

## Minimal correction

Inside the existing editor update, select the actual current full range, then retain focus as used by the journey's later end-of-text input:

```rust
let end = editor.value().len();
editor.set_selected_range(0..end, cx);
editor.focus(window, cx);
```

Remove only the hard-coded `window.press("cmd-a", cx)` setup operation. Continue using the same ordinary `cx.simulate_input(f.window, attempted)` with `attempted = "õ"`; do not set/replace the body through a test setter. Keep the blocked-state, exact whole-body, canonical/recovery/history, installed Close, production Cmd-Q, consequence/Cancel/Enter, stale-intent and explicit-discard assertions intact.

This is an existing public API, not a new hook. The Note editor is the exported `EditorState = InputBaseState<EditorMode>` from Base; component reexports it. `set_selected_range` explicitly accepts **UTF-8 byte offsets**, clips/expands to character boundaries and removes additional cursors (`state.rs:3146–3164`). `value().len()` supplies exactly the document byte extent, including its multibyte `õ`. The setter changes selection, not document bytes, and has no platform conditional. The same actual journey already calls it with `end..end` before the real `x` keystroke. This neutral setup preserves the behavioral input/writer boundary while avoiding a platform-specific shortcut assumption.

## Evidence reuse and next stop

The original meaningful Mac before-source failure at `1cf745d9` remains valid: it established the exact live `õ`, unchanged canonical Note/recovery/history, then failed at the actual installed Close predicate. The revised selection setup produces that same required state; it neither weakens nor retroactively replaces the baseline. Exact155bb Mac/local success remains historical evidence for that source; it does not become a Linux pass or native termination claim.

The installed Quit binding is explicitly registered by the application and remains unchanged; the current Ubuntu failure never reached it. No additional route defect is established by this log. Lead should inspect the small actual correction and run the selected affected check/normal CI without another mutation if the same setup or a different boundary fails. A further failure requires a fresh causal assessment, not a platform skip, relaxed assertion or repeat-until-green loop. Native OS Close/Cmd-Q remains owner-deferred; headless predicates and explicit test-window removal retain their prior limits.

## Final targeted candidate disposition

**APPROVED source `660a737174030320ca4c412248117a168e69c8d6`; no new findings.** Independently verified clean HEAD on the same branch, direct parent `155bb86776eea697135de8d6782bddc4481b3de0`, and the complete parent-to-final delta: solely `threads_writing_tests.rs`, 5 insertions / 4 deletions. The editor update now obtains `editor.value().len()`, selects `0..end` through the supported UTF-8 range API and focuses the editor; only the hard-coded `window.press("cmd-a")` call is removed. No body setter, input change, expected-byte change, assertion change, ignore or platform condition was added. Test-file SHA256: `15a09c0a528c04737e1f380d36f6dd184fd8401813bc4680f1deed0f28ab251e`.

The complete exact155bb review at `/tmp/brn-note-exit-candidate-review.md` remains reusable for production, components, exports, contracts and the unchanged rest of the journey: the only changed file/hunk is this selection setup. The actual correction implements the source-supported diagnosis above and retains real single-`õ` input plus all installed Close/Quit/consequence/durability/intent checks. It preserves the original Mac before-source failure and the separate Ubuntu setup-failure evidence.

No workload, build, test, format or lint command was run by this reviewer. Source approval does not turn prior-head checks into exact660a qualification. Lead owns affected final Mac checks and normal exact-head CI after publication; Ubuntu remains unqualified until that corrected check actually passes. All required checks remain required, and another failure requires causal reassessment rather than an automatic retry. Native termination/owner acceptance remains outside this review.
