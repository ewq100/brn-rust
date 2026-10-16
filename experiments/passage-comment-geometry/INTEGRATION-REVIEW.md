# Read selected-comment integration review

Selected scope: keep the current editor and Read viewer; position one selected
saved comment beside the first rendered glyph of its exact passage in Read.
Keep the comment list for navigation. No Zorite, Write comments, full-range
geometry, collision layout, publication, merge or release.

## Review batch and pinned evidence

- Lead retains this session's selected runtime configuration. One fresh,
  read-only independent reviewer was requested as GPT-6.1 Sol/high (requested
  configuration; runtime spawn returned reviewer identity, not model telemetry).
  Concurrency allowance: lead plus one reviewer, no delegated implementation.
- Reviewer: `read_geometry_review`; two lenses (requirements and correctness).
  Stop/reassess on concrete review findings or missing C/source authority;
  do not create a competing writing build or an unapproved dependency fork.
- Initial reviewed candidate: clean commit `38e0c95fd46e8ee96f5397d7f66e08662315e974`,
  executable prototype `f64a9d50766428ac162d1e2e2d5d7db4c875bb3c`, verified base
  `7e9d0376d677ed2a46bdbcd8933f55f9e1746a02`. The reviewer inspected its exact
  four-file gpui-base patch, full host/tests and proposed BRN wiring, then
  re-reviewed corrected clean commit **`c20bda2dda16257164f7c584f0c5fc20a301075f`**.
  This is an isolated review reference, not a production integration candidate.
- Original Z1 remains `b9bd1a14862fe2efe246790e887797c949f13c50`, unchanged.

## Current integration authority and dependency decision

Fresh `git fetch origin` still gives main
`8bef439b133e91ad68b0b76792bcd155d3d06237`. There is no active C0 `plan.md`,
recorded C integration ref, or writing extraction (`brn-writing-ui`) in that
ref. Available local worktrees and the 50 returned app tasks do not identify
an assigned C task. This does not establish that C does not exist elsewhere.
The owner replied **“no c yet”** on 15 October 2026. No C exists for this
handoff yet; no C-confirmed integration ref, released writing paths or toolkit
source approval is available. No unrelated task was messaged. This is an
explicit dependency/ownership blocker, not permission inferred from silence.

**Specific integration blocker:** C has not confirmed the actual integration
ref/released writing paths or approved a toolkit source containing the passive
geometry API. Stock gpui-base 0.7.3 lacks that API. The experiment's ignored
patched build copy is not an approved production source. No shared manifest,
lockfile or production file is changed and no toolkit tree is vendored.

Until C approves those prerequisites, retain the existing comment list/reveal
behavior. Existing writing/document/review/recovery/Undo checks can still be
run against the unchanged production code. The user authorized integration
only once dependencies and ownership are approved; this record does not
self-assign C's authority or treat fetched main as C's integration ref.

## Gate record

| Gate | State |
|---|---|
| Selected direction and bounded local integration-review assignment | Satisfied by current owner request. |
| Fresh main and overlap inspection | Satisfied as above; live C confirmation pending. |
| C integration ref, ownership and toolkit source | Blocked on specific missing confirmation/approval above. |
| Independent toolkit/proposal review | Complete for corrected reference `c20bda2`: all five findings fixed, no remaining actionable finding; reviewer independently reran 21/21 tests. Not production approval. |
| Production integration commit and ported geometry fixtures | Pending dependency/ownership approval; no integration commit exists. |
| Existing fallback production regression checks | Passed: 150 desktop tests; unchanged production code and stock toolkit. See exact command/evidence below. |
| Native acceptance | Pending: console reports `CGSSessionScreenIsLocked = true`; no GUI launched. Also requires an approved actual integration candidate. |
| Publication, merge, release | Outside assignment; not performed. |

## Corrected review artifacts

The corrected implementation remains wholly under this experiment. Baseline
is `7e9d0376d677ed2a46bdbcd8933f55f9e1746a02`; corrected reviewed code commit
is **`c20bda2dda16257164f7c584f0c5fc20a301075f`**, separate from the original
qualified prototype `f64a9d50766428ac162d1e2e2d5d7db4c875bb3c`. The final
receipt changes only documentation/evidence, not this reviewed implementation.

- [Exact corrected-reference changed files](review/CHANGED-FILES.txt), relative
  to the prior handoff `38e0c95`. The final receipt adds only docs/evidence.
- [Current four-file toolkit patch](patches/gpui-base-0.7.3-range-start.patch),
  SHA-256 `de6dcb320028f4d1848a44c3b2141fc87cd8edd7d23adb930500e4f40839b6d4`.
  Relative to stock pinned gpui-base 0.7.3, inside upstream `crates/base`.
  The old patch remains at immutable commit `38e0c95` with its old hash.
- [Corrected BRN synthetic source patch](patches/brn-review-prototype-source.patch),
  SHA-256 `bd62b0c757312333bb0dc280386f9b9e0bd8c94e1509bc64a1fddb8d99bebe17`. Exact four implementation file diff from the
  baseline to this reference. It is not a patch to production writing paths.
- [Reconstruction/scope/native-availability audit](evidence/review-integration-audit.stdout)
  proves the patch applied to the checksum-verified archive reproduces all four
  tested source files, zero toolkit files are tracked, production/root files
  are unchanged, original Z1 is clean and unchanged, and the Mac is locked.
  Run `integration-audit.py` for the current review; `handoff-audit.py` checks
  the historical qualified toolkit hashes and belongs to the earlier tree.
- Build, strict own-target Clippy and formatting passed:
  [build](evidence/review-build-final.stderr),
  [Clippy](evidence/review-clippy-final.stderr),
  [format](evidence/review-fmt-check.stderr). Scope/version and repository
  Markdown-link checks passed; no pin upgrade or external crate was added.

## Review findings and lead dispositions

[Independent initial report](review/independent-initial.md) is pinned to the
previous clean candidate. The lead added five repro tests before editing:
[evidence/review-regressions-before.stdout](evidence/review-regressions-before.stdout)
records 14 old tests passing and **all five new tests failing**. The stderr
records the actual failures, including an old reveal scrolling the identical
new document from list offset `(0, 0)` to `(22, 56px)`.

| Finding | Lead validation and disposition |
|---|---|
| P1 vector index used as selected comment identity | Valid/fixed. Reproduced removal putting another comment at the selected index. Bind synthetic stable record ID plus complete comment payload, note/version/source and rendered snapshot; reorder preserves the selected record, removal/addressed state/payload changes clear it. Production must use actual Store IDs and lifecycle. |
| P2 clear leaves pending explicit reveal | Valid/fixed. Reproduced before-first-frame identical-source switch scrolling the new document. Narrow toolkit `clear_reveal` API cancels pending explicit requests. Clear on invalidation and before replacement selection; both identical-document and rejected replacement tests pass. Does not reverse completed scroll. |
| P2 large async source change/revert revives old probe | Valid/fixed. Reproduced with a source over 4 KiB, both mutations and paint in one update before either parse commits. Clear probe synchronously in every effective text mutation (`increment_update`), independently of committed parse identity. |
| P2 clean Read/Write/Read restores old anchor | Valid/fixed. Reproduced without editing. Toggle clears selection/probe/highlight/pending reveal; returning requires a new validated selection. |
| P2 newline-start passage misses covered first glyph | Valid/fixed. Reproduced source passage `\nworld` after `hello  `. Probe skips CR/LF only inside resolved covered text pieces; actual glyph measurement still requires an exact glyph. Newline-plus-text matches the `w` box; newline-only remains unavailable and never borrows a glyph beyond its range. |

These are bounded corrections to the experiment/reference, not another editor
evaluation or a production integration. The reviewed v1 remains immutable at
`38e0c95`; original Z1 remains unchanged. The new [integration proposal](PROPOSED-INTEGRATION.md)
adds pending-reveal cancellation and distinguishes addressed/resolved comments
from `Comment.unresolved` (an unmappable passage).

[Independent corrected review](review/independent-corrected.md) found no remaining
actionable issue in the bounded reference, verified its patch hash and
independently reran **21/21**, zero ignored. Its proposal verdict explicitly
requires shared reveal ownership: existing outline navigation also calls the
same TextView reveal path. Lead disposition: **valid integration requirement,
deferred to the C/B production gate**; the proposal now names outline/comment
supersession tests and conditional cancellation. The prototype has no outline
owner and does not qualify that actual production interaction. No toolkit
source approval or production acceptance is inferred from the clean review.

## Executed checks and practical limits

- **21/21** headless GPUI geometry tests pass, zero ignored:
  [output](evidence/review-headless-final.stdout). The 14 earlier checks plus
  removal/reorder/addressed lifecycle, same-source version/payload changes,
  pending reveal cancellation on identical-document navigation and invalid
  replacement, clean mode round trip, rapid large async edit/revert, and hard
  newline-plus-text/no-glyph range cover the corrected reference.
- **150/150** actual existing desktop tests pass, zero ignored:
  [output](evidence/review-fallback-desktop.stdout), command
  `cargo test -p brn-desktop --features native-ui,native-test-support --locked -- --nocapture`.
  This includes document/formatting/writing, renderer policy, review/Apply,
  recovery, clean/dirty draft and editor/durable Undo journeys. They execute
  GPUI test context with synthetic/offline fixtures, not native observation.
  Root manifests/lockfile and production code are unchanged, so this qualifies
  retained fallback behavior only; geometry assertions are **not yet ported**
  to production fixtures because there is no approved integration.
- The two initial `--offline` attempts could not resolve uncached `calamine`.
  Normal `--locked` crate acquisition populated the cache, without pin changes,
  system/model installs or provider calls. The intake-helper build then passed.
  The installed compiler emitted a nonfatal `rust-objcopy` missing-libLLVM
  stripping warning; helper and desktop build/test commands still exited zero.
- The experiment's editor/Store witnesses remain green: geometry emits no
  editor Change, creates no clean edit session, changes no text/generation/
  recovery record/revision/receipt, and performs no implicit Save. Real input
  is the positive control; stale-generation Save refusal, explicit Save/replay
  and checked durable Undo remain covered. Actual geometry+ThreadsUi coupling,
  indicators, Save and Undo acceptance remain pending production integration.
- [Exact commands/environment/timings](evidence/commands.jsonl) and stdout/stderr
  remain recorded, including failed attempts. Timings with competing builds
  are provisional. No native, provider or model qualification is inferred.

## Next action and exact integration status

**Production integration commit: none.** The corrected experiment commit is a
review reference, not an integration commit. With the owner's “no c yet” reply,
there is no approved dependency source or released writing ownership/ref.
The main writing build keeps the current comment list/reveal behavior.

When C is assigned, obtain the exact ref and moved/shared-writing destinations,
then C's supported toolkit-source and root dependency decision. Only then can B
port the reviewed binding/lifecycle contract and assertions into actual writing
fixtures, independently review that exact candidate and run the consolidated
native checklist. An unlocked accessible Mac is also required. Publication,
merge and release remain outside this assignment.
