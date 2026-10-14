# Writing workbench polish implementation plan

**Goal:** comments beside readable note text, a separate full chat panel, compact formatting and smooth feedback collection.
**Spec:** [accepted owner outcomes and toolkit approach](spec.md).
**Baseline:** fetched `origin/main` `2213eb4f5c67e2b2f53287ee60f8cf2bbbef46ce`; isolated `codex/writing-polish` checkout. Original checkout is `work/live-search-evaluation` at `d9255b0`, with unrelated `experiments/` preserved. Research worktree is preserved separately.
**Architecture:** one Markdown EditorState owns typed text; one derived selectable TextViewState presents formatted text using exact source mappings. All saves/comments/rewrites/reviews retain existing checked services. Lead owns UI integration. Sol owns bounded pure formatting helpers. Luna owns read-only plan review; final code gets independent review.
**Stack:** pinned Rust 1.98.1, GPUI Kit 0.7.3, existing Markdown renderer, existing shared service and lockfile.

## Constraints and review focus

- Dirty/recovered/stale buffers cannot become guessed saved comment anchors or bypass checked Save.
- Partial rendered selections containing Markdown markup use exact source offsets, not text search; repeated quotes and Unicode must retain the selected occurrence.
- Programmatic editor `replace` can bypass its readonly flag; host handlers must reject blocked/reviewing/recovery states.
- Read/Write switching preserves the buffer, selection, drafts and focus without saving or starting AI. Review candidates retain exact text and grant no automatic Apply.
- Narrow layouts keep reading and feedback usable and chat reachable; comments cannot steal the full chat's durable destination.

## Tasks

- [x] **Formatting helper:** create `crates/brn-desktop/src/threads_format.rs`. Pure `Format` enum and `Edit` result transform exact selected UTF-8 ranges for bold, italic, heading, list and quotation using local editor replacement. Tests cover toggling, empty selections, Unicode, invalid boundaries, multiline formatting and unchanged surrounding text. Lead integrates toolbar and native action tests in `threads_ui.rs` and `threads_writing_tests.rs`.
- [x] **Formatted reading and exact comments:** add per-window Read/Write presentation and a derived `note_read_view` to `threads_ui.rs`; integrate safe renderer in `threads_writing.rs`. Use `selected_source_range`, source byte validation, source highlights and `reveal_range`. Opening/switching/revealing keeps the same canonical buffer. Tests demonstrate exact repeated-quote/Unicode source anchoring, dirty view switches and no implicit writes.
- [x] **Layout and feedback polish:** refine note typography, note/feedback sizing and cards in `threads_writing.rs`, and Write preset proportions in `threads_layout.rs` only if needed. Feedback stays beside note, chat stays separate. Give note toolbar and rewrite context clear labels; hide version/technical details in ordinary success presentation while exposing stale/unlocated context. Save-and-comment snapshots note ID/generation/range/quote, saves through the existing path and revalidates the committed text before continuing; tests cover success and refused Save.
- [x] **Review and validation:** keep chat/feedback visible during note review and offer formatted candidate reading without changing candidate identity. Run focused native tests, full `verify-desktop-shell.sh`, format/docs checks; native-observe a synthetic long formatted note with comments, separate chat, formatting, save, reviewed Apply and Undo. Record gaps rather than substitute automated tests for native observation. Independent review, local commits and reviewable delivery follow. No merge/release is assigned.

## Checks and execution

Plan-review corrections: Save and comment only opens the composer after Save; Add is separate and retains its existing operation identity on failure/retry, without undoing the successful human Save. Comment drafts remain window-local. Read is refreshed from every current buffer update and source selections are invalidated/revalidated rather than carried over edits. Formatting is disabled during displayed candidate review; a kept candidate retains its identity after a human Save, and stale Apply remains refused. Tests explicitly cover typing→Read→Save-and-comment, recovery/stale Save preservation, failed Add retry, and kept-review→human Save→stale Apply.

Use the compatible shared target cache `/Users/evokessler/repos/brn-rust/target` with `CARGO_TARGET_DIR`; it does not change application data. Synthetic application data lives outside Git in a task-owned directory. Required native build/lint/tests follow `scripts/verify-desktop-shell.sh`; documentation uses `scripts/check-markdown-links.py` and `git diff --check`. Source and schema do not change outside task-owned UI/helper/docs/tests.

## Evidence

Preflight found the pinned compiler, protoc and Xcode available. Sol verified supported exact editor geometry and rendered-to-source selection APIs, and the absence of supported WYSIWYG marker elision. Luna approved the corrected plan. Implementation, build/lint/tests and scoped synthetic native observation are complete; [evidence](evidence.md) records the tested flows, review dispositions and remaining owner/hosted gates.

## Preview correction: writing stays in its tile

- [ ] Add backward-compatible overall feedback to the immutable request, bounded/replay-bound through the same checked service. Sol owns core/app request helpers and tests.
- [ ] Lead adds per-note draft preservation and an Overall feedback field in the note footer. New Rewrite captures it without chat Send; saved-input disclosure shows it; retry preserves the saved value.
- [ ] Route writing proposals and writing-specific continuation/progress/Stop into the Note tile; filter writing request rows out of chat. Keep ordinary chat continuation intact and support writing with Chat closed.
- [ ] Luna simplifies only the comment margin; preserve include/reveal/addressed behavior and truthful unlocated states.
- [ ] Verify closed-chat generation/review/Apply/Undo, draft navigation, snapshot replay/legacy/size and independent review. Rebuild and native-observe fresh synthetic data; refresh PR #135 and required CI. No merge/release assignment.
