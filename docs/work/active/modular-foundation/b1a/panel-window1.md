# B1a panel extraction: first local qualification window

**16 October 2026.** Qualified local source pin for fresh independent actual-diff review; **B1a acceptance remains pending**. Base is `df5e48e38549ad19a70daeaf47f9b6b185ef6d0e`, branch `work/b1a-cw2-20261016`, bound checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc`. Earlier [source-only receipts](panel-source-prep.md) retain the preparation chronology. [Machine receipt](panel-window1.json) records all24 command/source/log identities, failures, artifact hashes/features and final production manifest.

Production manifest SHA-256: `87a2340c017378c11e435fc6132420a3979f08111791e136d5436d7d514af95e` (37 paths, including the exact principal CI/script hunks and local lock edge). It excludes prose to avoid a self-referential hash. The local commit SHA/tree is reported after creation; all test commands ran before that commit against the recorded source.

## Actual implementation

The real Note/comments/Read/format/review/Compare render bodies and local editor/view/draft state live in brn-writing-ui. Desktop owns a plain WritingPanel and short-lived WritingView borrows of its one Workspace and checked batch. Concrete control requests synchronously invoke the existing host handlers. First typing still calls begin_edit/update_buffer before derived Read/selection/notify. The current editor, saved-selection guards, drafts, v1/v2 readers, exact Apply/Dismiss/groups/recovery and receipt Undo remain the same checked paths. There is no new worker/store, editor trait, host-operation trait or runtime route; B1b adapter retirement, B3 and B4 remain outside this slice.

The complete CW2 token/Note/scope/conversation/exact-candidate cache remains in desktop, refreshed first once per render and cleared before reload/error. Early private-content refusal, mandatory disclosure outside Inputs, scoped presence/status/feedback/worker filtering and canonical context calls remain intact. [CW2 qualification limits](../cw2/principal-disposition.md) still apply.

WritingView construction now borrows progress and worker identity and stores concrete presentation function pointers. Scoped assets, latest Run and control-display policy resolve only at consuming sites. Message assets retain checked-request conversation membership and empty-on-absent-scope behavior; Note assets keep the current Note.chat/record_in_chat policy. No all-assets fallback or second cache. The obsolete desktop note-assets wrapper is removed; two existing assertions call the same moved policy through WritingView. Only test-support builds count actual asset/latest-run reads.

Shared primitives/palette/markdown/fit helpers and necessary unchanged row_text/pill/scroll/terminal_notice/Compare dialog support have one implementation with desktop reuse. Desktop retains chat/composer, linked tile, shell layout/navigation, lists/forms, runtime, queue and authority. Old document/workbench files are removed after caller forwarding. Approved formatting/diff bytes remain unchanged. AI/core/app, root workspace/toolchain/dependency versions and principal CI/script bytes are unchanged. The lockfile adds only the existing brn-writing-ui package's existing local brn-threads-app dependency edge; no package/source/version/checksum churn.

## Actual first-window results

All commands used repository Rust1.98.1, locked/offline after the allocated lock update, `CARGO_INCREMENTAL=0`, task target `/private/tmp/brn-b1a-20261016-target` and fresh synthetic TMPDIR under `/private/tmp/brn-b1a-panel-window1-20261016/`. This target belonged to the same bound helper checkout; no shared C0 or cross-root artifacts were adopted.

| Gate | Actual result |
|---|---|
| Native library build | Passed on final source. |
| Native/test-support library all-target Clippy, `-D warnings` | Passed on final source. |
| Library tests | **44 passed, 0 failed/ignored/filtered**; zero doctests are reported separately, not used as proof. |
| Desktop native/test-support build and strict all-target Clippy | Passed on final source. |
| Focused real-editor F15 | **1 passed, 270 filtered**. |
| Focused actual work-bound witness | **1 passed, 270 filtered** after fixture corrections below. |
| One complete desktop native/test-support invocation | **270 passed, 0 failed, 1 existing ignored, 0 filtered**, 4.38s test execution. Includes original A/B and CW2 privacy/context/access/cache/real-typing witnesses plus F15/work-bound. |
| `cargo fmt --all -- --check` | Passed. |
| Default workspace normal and feature trees | Passed; **zero GPUI entries in both**. |
| Default workspace build | Passed; no GPUI compilation. Default full Clippy/tests remain deferred. |
| Maintained intake helper | Built in the task target; exact artifact identity retained. |

The complete native command was `cargo test -p brn-desktop --features native-ui,native-test-support --locked --offline`. Its sole ignored test is the existing `a_desktop_supply_reuses_the_chat_a_cli_useful_intake_left`, requiring the three BRN_DWR_SHARED_* inputs from a CLI useful intake. No new ignore, filter or skip was added. This is the post-helper-move suite: the base had277 desktop registrations;8 whole pure tests move out here and2 meaningful desktop witnesses are added, yielding271. The library's44 are the approved34 helpers +3 theme +3 workbench +2 markdown +2 threshold portions. The original two host-layout tests retain all their remaining assertions and names, so the split adds two combined registrations. A raw historical310 count cannot be reused as the current desktop count.

F15 first demonstrates working real-editor Undo with simulated typing. It saves owner typing, clicks a real checked Apply, presses Cmd-Z then Cmd-S, and repeats after an actual saved-record change/programmatic refresh. Exact editor/saved text, absent dirty session, saved record/version and receipt counts remain intact. It passed existing set_value/undo-history behavior; no new production guard or fabricated failing-before is claimed.

The work-bound witness uses actual scoped-asset and latest-run read counters plus the existing checked-batch counter. Four mixed pure-query/Write-notify cycles, including12 per-candidate lookups over3 unrelated pending Notes and a real linked conversation, add **zero** asset reads, latest-run reads or batch reloads. Actual saved-status and formatted Read consumers are positive controls. This proves the bounded work property, not a broad latency guarantee or constructor-field mirror.

## Failures retained and corrections

Six failed commands are retained with their source snapshots and complete log hashes in the machine receipt/scratch evidence:

- Initial library build: two opaque callback lifetime errors and unused imports. Callbacks now use the exact weak-entity update sequence of pinned Context::listener, with owned captures and no heap-boxing or delayed dispatch.
- Initial strict library lint: complex scoped-image callback type and needless as_deref on borrowed Run ID. A narrow WritingAssets alias and direct borrowed ID fix those; no policy changed.
- Initial desktop build: moved PopupMenu import and unused imports; corrected in owned modules.
- Initial strict desktop lint: empty moved-policy test module and two existing note-assets assertion callers. The empty module is removed and the two calls mechanically target the single library policy, preserving their assertions. Unused production wrappers are removed or test-gated.
- Initial work-bound run: fixture tried three active candidates for one Note; the checked service correctly refused. It now uses three distinct Notes. A subsequent compile caught a missing Window binding in that fixture; corrected. Neither failure was a production guard regression. No original assertion was weakened or rerun without a source correction.

One Clippy controller was queued2.42s before the helper controller exited; Cargo explicitly waited on the same task-target build lock before compiling. Both timings/logs remain recorded. Subsequent gates were explicitly serialized. The existing external block0.1.6 future-compatibility warning remains; no dependency upgrade or lint suppression was introduced.

## Remaining acceptance and handoff

Default full workspace Clippy/tests, combined native/retrieval and full headless qualification are **deferred to the next principal allocation/integration window**. F22's graph/build portion passed; its full qualification is not claimed. Native pdftoppm is still absent from PATH; the carried PDF gap remains open. The existing shared CLI/native intake fixture was not supplied or silently skipped by a wrapper. No new mutation/stress/benchmark campaign, GUI/human observation, provider/model/owner-data work, publication or merge occurred. Widget B's historical original-cause uncertainty and global CW2 qualifications remain unchanged.

Fresh independent Copilot Opus actual-diff review and principal source acceptance remain pending. The worker retains UI source ownership for review/fixes. The heavy slot is released to the principal immediately after the local pin/self-check, with an empty process receipt retained in scratch. No further heavy commands are authorized by this receipt.
