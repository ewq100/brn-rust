# X2 Office1 preparation handoff

18 October 2026. Task: Kiro Sonnet 5.5 bounded next-family preparation for the owner-approved all-document Xberg integration. Source-only. Docs only.

## What exists

- [implementation-slice.md](implementation-slice.md): the one slice proposal, six sections as asked.
- [source-map.json](source-map.json): pinned versions and checksums, 36 Xberg/ZIP/BetterOffice anchors and 13 BRN anchors with file hashes, fixture survey, the upper-bound list of 33 root-lock additions with checksums, three corrections to earlier audit statements, and the explicit list of unexecuted limits.

## Branch and custody

Branch `work/x2-office-prep-20261018` was absent, created with `git switch -c` at `bbf107e370a5c5dc35a7588e97c477d5639401a6` in the allocated worktree `t3-d611a161`. The worktree was clean at start. No other worktree or ref was touched. The commit that holds these three files is recorded by Git (`git log -1` on the branch). The three paths above are the only paths written.

## What was checked

Text, Git, hash and link checks only. File hashes in the source map were computed at reading time. Relative links in the two documents were resolved against the tree. Nothing was compiled, resolved, tested, linted, formatted or run. No Store, CLI, Auth, provider, network, GUI, download or mutants work. No heavy slot was held, none is held now, no process is running, and nothing was delegated.

## Findings the principal should read first

1. Xberg 1.3.10 cannot supply the current DOCX/PPTX evidence contract. Seven classes are unavailable (S1 to S7 in section 1), among them byte ranges for figure occurrences, DOCX comment metadata, table spans, PPTX hidden state and picture binding. One of them, namespace-prefix independence, is a predicted silent loss on the existing `prefix-variation.docx` fixture. It is source-read, not executed.
2. Admission is boundable with a maintained seam (`SecurityLimits` plus ZIP 9.0.5 declared-size reads). Three pins matter: `max_archive_depth = 0` (the default of 3 recurses into embedded objects), `images = None`, and a ratio limit large enough not to refuse inputs accepted today.
3. The smallest projector for the unavailable set is most of today's evidence layer. That makes D-OFFICE a real owner choice. It is needed only before production promotion and does not block O1.
4. No existing fixture has tracked changes, comments, `vMerge`/`rowSpan` or a DOCX embedding. Only `harbor-process.docx` has a complete independent inventory.
5. Three earlier source1b statements are corrected in the source map (PPTX hidden state, DOCX rId, fixed 100 MiB truncation).

## Not claimed

No implementation grant, no code authority, no source acceptance, no review, no PR, push or merge. HTML at `bbf107e3` stays under its independent review and is not assumed accepted. The parent's implementation grant follows its own component gate.

## Next principal actions

1. Decide D-OFFICE, or release O1 steps 1 to 3 without waiting.
2. Run the offline metadata on the exact manifest if O1 step 2 is released, and compare each added identity with the source-map list.
3. Choose the PDF seam route in section 6, independent of Office.
