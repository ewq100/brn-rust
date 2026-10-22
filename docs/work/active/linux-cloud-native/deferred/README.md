# Deferred native queue: Linux equivalents

Owner approved draft PR220, full meaningful Linux equivalents, focused fixes and
later merge only after independent review and exact-head required CI. No Mac
access, provider credentials, new live-account attempts or unrelated work.

Inventory source: campaign branch `f0c1c0ba`, latest native-queue and linked
acceptance/preflight records, independently audited before execution. Application
source at entry: `4d0c01b7` (base main `24a77f4f`). Original Mac statuses below
remain unchanged by Linux tests.

| Original case | Original Mac status | Linux work selected |
|---|---|---|
| MAC08 grouped Review | Deferred | Actual AT-SPI pressed true/false/true; other group unchanged; no Apply/canonical mutation |
| MAC11 Notes rows | Deferred | Named actionable Recent/Linked/All; named activation; filter then Tab/Space; unchanged notes/drafts |
| MAC12 Needs you | Deferred | Outer MenuItem full title/reason, named activation, visual one-line bounds |
| MAC13 stale recovery privacy | Deferred | Private refusal and public positive copy; exact historical recovery; general search/export privacy |
| MAC14 scoped Connections | Deferred | B unlink removes private neighbor; exact Undo restores; retained bytes/provenance unchanged |
| MAC15 Close/Quit | Deferred | Separate Unicode/multiline recovery; conflicting editor; Enter/Cancel/refusal/discard |
| MAC06 selected feedback v1 | Historically unobserved | Select one captured comment; exact Apply and Note rewrite Undo; excluded/sentinel preserved |
| Inactive-Project skills | Native-unobserved | Inactive scope refuses enable/adoption; exact Remove; stale binding Inspect/current enable/separate Select |
| Review draft failure/navigation | Native-unobserved | Edited proposal + checked external dismissal; failed Prepare/Keep retains; Cancel restores |

MAC01/03/04/07/09/10 have historical actual Mac passes and are not newly deferred
cases. MAC02 is an umbrella, not a separate executable scenario. MAC05 has no
catalogue definition. CLI preview-input cancellation is not a desktop journey.
GUI backup Restore remains distinct from historical CLI restore. No new account
fixture will be regenerated for failed/exhausted provider sessions.

Linux AT-SPI is an actual platform accessibility observation, not Mac AX or
VoiceOver acceptance. Mac OS Close/Quit/menu integration, IME and subjective owner
acceptance remain Mac-only gaps. Reproducible setup uses a task-local session bus,
verified Debian AT-SPI GI and xclip packages; no host security changes.

## Results observed on the real Linux application

Application source: `4d0c01b71039e7ffdcbbe74fb9a5067b1defde84`; production code
has not changed during these journeys. Two new offline fixture examples prepare
Connections and Needs-you states through checked services; they do not establish
native acceptance themselves. All data is synthetic. No Run/provider request,
account attempt or Mac interaction occurred.

| Case | Linux result and evidence prefix | Remaining original platform requirement |
|---|---|---|
| MAC08 | PASS: actual AT-SPI `pressed` true/false/true, repeated twice; other group unchanged; full durable snapshots identical. `mac08-*` | Mac AX boolean/VoiceOver |
| MAC11 | PASS: three named actionable Note rows in Recent/Linked/All; each named activation; unrelated Note then filter/Tab/Space opens target; unchanged durable snapshots. `mac11-*` | Mac AX/keyboard observation |
| MAC12 | PASS: actual outer menu item has full title + reason and click action; named activation opens exact writing context; bounded 26px row; unchanged durable snapshots. `mac12-*` | Mac menu/AX |
| MAC13 | PASS: public stale copy exact; private refusal repeated, no new Note/receipt/recovery changes; general search has zero private hits and export excludes private text. Both use clean v1 UI, checked external v2, real stale Save banner. `mac13-*` | Actual Mac UI |
| MAC14 | PASS: Project B Unlink removes private neighbor; native history Undo restores exact operation; same B scope and center, exact Note bytes/provenance unchanged. Immediate retirement before refresh was not measured. Repeated settled Undo shows 1 of 1 neighbor. `mac14-repeat-*` | Actual Mac UI |
| MAC15 | PASS: distinct OS Alt-F4 and registered Super+Q preserve exact multiline Unicode recovery/session/base; external edit conflict refuses both exits, Enter does not discard, Cancel retains live-only õ; explicit Discard and quit preserves external edit and canonical data. `mac15-*` | macOS Close/Quit lifecycle, IME |
| MAC06 v1 | PASS: only selected captured comment addressed with Apply, exact revised note, excluded comment pending; native Note rewrite Undo restores original bytes and pending feedback, sentinel unchanged. `mac06-*` | Actual Mac v1 session; no v2/provider qualification |
| Inactive skills | PASS: inactive scope Enable unavailable; attempted click unchanged; exact-current inventory Remove succeeds; mismatched stale Remove disabled. Active scope Inspect shows v2 bytes; Enable current stores v2 digest with selected=false, separate native selection sets true. `skills-*` | Actual Mac controls; legitimate disabled stale binding differs from raw enabled/stale headless seed |
| Review failure | PASS Prepare/Keep refusal retains draft after checked external dismissal; native Cancel restores exact candidate text, no post-dismissal durable mutations. Separate active proposal: Home then same Review row reopens exact edited draft with unchanged durable state. `review-repeat-*`, `review-nav-*` | Mac native observation pending |

The final review Cancel witness is `review-repeat-cancel.png` plus
`review-repeat-cancel-exact.txt`, not the earlier incorrectly timed
`review-draft-cancelled.png` (which still shows editing). Initial xdotool Unicode
input dropped bytes; real clipboard paste corrected the harness. Clipboard reads
must wait for the native copy event; immediate reads can return the previous
selection. Pointer moves should settle before clicking. The first MAC08 harness
assumed a stable Include/Omit label and failed; corrected lookup/state assertions
were repeated. Connections fixture's initial compile errors were corrected before
native use. Original failure logs/captures remain in the raw archive.

## Evidence and reproduction

[Raw evidence archive](evidence.tar.gz) contains synthetic CLI snapshots, AT-SPI
trees/action receipts, all screenshots (including diagnostic/unsettled frames),
fixture manifests, setup commands and the driver. [Checksums](SHA256SUMS) bind it
and the selected screenshots. Extract into a new directory; inspect the case
prefixes above. The driver requires the saved cloud activation and task-local
AT-SPI activation; it never supplies a pass/fail result merely from a click receipt.

Selected settled views: [private refusal](mac13-private-refused.png),
[Connections restored](mac14-repeat-undo-settled.png),
[menu](mac12-menu.png), [review Cancel](review-repeat-cancel.png),
[inactive skills](skills-inactive-controls.png),
[current package inspected](skills-active-inspection-bottom.png).

Use the [Linux setup guide](../../../../development/linux-cloud-ui.md). Build
fixtures with `cargo build -p brn-threads-app --example native_connections_fixture
--example native_attention_fixture --locked --offline --jobs 1`; run each with
`--output /absolute/new-root`. They refuse reuse and write manifests. Existing
`native_writing_fixture` and grouped-review fixture provide offline review input.
`prepare-skill-fixtures.py` in the archive records all public CLI fixture commands.
For MAC13/15, follow the ordered scenarios in the campaign native queue, using the
saved driver and a fresh `init --purpose test` root. Never reuse owner data.

## Limits and integration gates

Linux AT-SPI is exposed and was actually queried. Some dropdown triggers lack a
click action and Project rows lack named actions; pointer routes are explicitly
behavioral evidence. Offscreen actions can report accepted without changing UI;
always verify settled visual and persisted outcomes. Initial role assumptions
(button versus toggle button) caused missing-target errors; do not interpret
those harness failures as application mutations. Disabled skill controls remain incorrectly exposed as AT-SPI enabled/sensitive
despite being visually disabled/actionless. No full accessibility pass is
claimed. No hardware GPU, Wayland, macOS menu/AX/VoiceOver/IME, native file-picker,
GUI backup Restore, fresh live-model or release readiness qualification is claimed.

Independent review found no blocking source findings. Required PR checks passed
on entry head4d0; final follow-up exact-head checks and merge remain pending until
GitHub confirms them. Optional Windows default-workspace build fails on baseline
main24a77 as well; current log download was denied, so exact current diagnostics
are not claimed identical. Full protection endpoint is inaccessible (HTTP403);
PR required-check rollup is readable and is used without bypassing protections.


## Local checks for this follow-up

- `cargo test -p brn-threads-app --locked --offline --jobs 1` through the saved
  reaping wrapper: **274 passed, 0 failed, 1 ignored** across unit/integration/doc
  targets. No live provider credentials are used.
- Both new example builds/runs and strict example Clippy (`-D warnings`) pass.
- Reused unchanged production desktop evidence: **351 passed, 0 failed, 1 ignored**,
  native build and strict all-target desktop Clippy. The ignored desktop useful-
  intake test needs its separately prepared shared fixture.
- `cargo fmt --all -- --check`, `git diff --check`, Markdown links (628 files,
  2835 local links), generated overview, provider/intake retirement checks pass.
- Documentation/tooling Python suite: **191 run, 0 failures, 1 skipped** through
  `run-with-reaping.py`; skipped Apple Silicon signing witness requires Mac.
  Initial direct suite had one process-group-liveness failure because killed
  orphan `sleep` remained zombie under PID1. The same suite passed using the
  existing wrapper; no test assertion or production code was weakened.

All raw check logs and original failures are retained in the archive. GitHub
required checks must be revalidated against the final published head; entry-head
success is not final-head success.
