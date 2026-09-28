# First end-to-end workflow evidence

> Historical record: branch names, commands, approvals and results below describe the recorded task, not new instructions or current authorization. See [current status](../../../status.md) for the present checkout and remaining gaps.

2026-09-28 · Branch `trial/end-to-end-flow` · Base `a2fa7d5aff7c4c6bfb5875eed11915fa246a872c`.

## Demonstrated flow

The shared application workflow now connects selected Markdown/text import, explicit search approval, immutable version storage, index construction, keyword/semantic/hybrid retrieval, a grounded Codex App Server answer, evidence inspection and saved-session reopening. [The plan](plan.md) scopes this to the roadmap's first useful flow through chunk 10. Publication, writing/comments and graph work remain later milestones.

A native-feature `brn-flow` executable imported two newly created synthetic documents, built a real FastEmbed/LanceDB index from those documents, and retrieved attributable passages through all three profiles. Each search ran in a separate process and reopened the saved generation. Build took approximately 2.54 seconds; observed keyword search was 0.02 seconds and semantic/hybrid reopening plus query about 2.15–2.16 seconds. These are two tiny documents, not representative-corpus performance measurements.

Using the installed Apple Silicon Codex executable and existing managed ChatGPT sign-in, the integrated hybrid question asked when the synthetic Aurora mission launches and its launch code. The server streamed and completed an answer identifying Tuesday and BLUE-HERON with citation `[1]`. The application persisted its user message, frozen exact evidence, provider turn ID, usage and completed answer. The first successful turn took approximately 10.32 seconds including index reopening and sidecar startup.

A fresh process resumed the saved provider thread and used semantic retrieval to answer a second question: Niko is the botanist, with citation `[1]`. This completed in approximately 7.10 seconds. A third invocation reused the completed operation UUID **without a provider executable configured** and returned the identical saved record. Fresh-process history contained both completed turns. A genuine protocol account identity was available through `workspaceRouting.chatgptAccountId`; its value was stored for association checks and was not printed or copied into this report.

The first attempted thread creation was rejected before any question was prepared/submitted. Removing unnecessary unqualified per-thread overrides resolved the compatibility failure while keeping the qualified launch-time tool restrictions. The original unlinked session is retained as requiring recovery; it was not silently assigned a replacement thread. BRN did not directly read credential files; credentials were not printed, copied or committed. No account setting or source original was changed.

## Implementation boundaries

- `brn-store` schema v3 transactionally imports by stable origin, preserves source UUIDs and immutable versions, tracks explicit retrieval approval and current version, attaches provider threads, and couples chat preparation/completion to operation state and local transcript records. Search approval is not publication approval. An unattached session or second active turn is rejected.
- `brn-retrieval` uses UTF-8-safe passages of at most 1,600 bytes, exact revision hashes/byte ranges, immutable generations and a synced completion marker. Keyword does not require native model assets. Semantic/hybrid lazily load verified local model/database resources and keep them loaded in the desktop worker; missing/corrupt resources fail without fallback. Hybrid uses reciprocal-rank fusion. Native ORT calls cannot be interrupted internally; cancellation is checked between batches and before activation.
- `brn-workflow` bounds selected imports to nonempty UTF-8 `.md`/`.txt` files up to 1 MiB, preserves originals, validates active-index/current-corpus fingerprints and revalidates evidence against authoritative current/approval state. Session/thread association and pending question/evidence commit before external submission. Duplicate IDs return stored outcomes before retrieval or provider connection; uncertainty never triggers replay.
- `brn-provider` supervises an explicitly selected executable, uses the observed App Server version/protocol, manages read-only/no-approval turns with network access disabled, disables unused tools through launch configuration, denies client-handled tool/approval requests, and bounds framing, queues, output and request deadlines. Credentials remain owned by Codex. Provider history remains authoritative; BRN messages are projections.
- `brn-desktop` exposes these same actions through an owned background worker, profile buttons, source/evidence views and saved conversation history. Generation checks prevent changed query/session/profile state from accepting stale visible results. Source approval/import changes invalidate current evidence. Progress and streamed text are coalesced; completed events are bounded and retained.

## Verification record

Final verification on macOS arm64 passed:

- `bash scripts/verify-end-to-end.sh`: workspace formatting, locked/offline build, Clippy with warnings denied, 65 tests (12 core, 5 desktop CLI, 13 provider, 8 retrieval, 18 store, 9 workflow), and disposable CLI import/reimport/build/search/change/rebuild smoke checks.
- `bash scripts/verify-trial.sh`: existing starter/provider regressions, including 16 separate provider-harness tests and nine CLI smoke checks.
- `cargo build -p brn-desktop --features native-retrieval`, `cargo test -p brn-desktop --features native-retrieval` (five tests), and `cargo clippy -p brn-desktop --features native-retrieval --all-targets -- -D warnings`: passed. The built native binary also passed headless completion, cancellation and stale-result checks. The final evidence-invalidation adjustment was followed by native UI Clippy and another full native link.
- Native retrieval: real-model smoke test and native Clippy passed; `brn-flow` with `native-retrieval` linked and performed the live checks above.
- Astra independently reviewed the integrated changes, requested lifecycle/correlation/evidence fixes, then approved the corrected personal trial with no remaining Critical or Important findings. Its independent provider rerun passed 13 tests.

Native builds reused the existing retrieval target directory with four build jobs. The dev/test profiles omit debug information to keep combined native artifacts manageable. The source diff and staged files were checked for whitespace errors, accidental local state and credential material before commit. The integrated workflow tests include provider-side read-only assertions that a saved thread and pending question already exist before `turn/start`, unchanged/changed imports, stale and wrong-generation rejection, reopening, conflicting request IDs, uncertain outcomes, cancellation and provider-store mismatch.

The live fixture data is retained locally outside the repository. This is synthetic trial state, not a private-vault import. The initial native attempt was blocked by the locked Mac. After the user unlocked it, the native acceptance checks below ran through supported computer-use.

## Native acceptance after unlock (2026-09-28)

A local, unsigned `.app` launcher invoked the built `brn-desktop` binary with an isolated temporary data directory, the explicitly selected installed Codex executable, and the existing verified model directory. It did not copy or bundle Codex or model files. All input and window actions used Computer Use; database/process inspection was read-only verification of the synthetic trial.

Observed on macOS arm64:

1. Entered the path to a newly created 98-byte Markdown fixture and clicked **Import and approve for search**. One approved source appeared. **Build index** completed.
2. Entered a question and selected **Hybrid**, then **Search**. One exact passage appeared. Clicking it showed the complete quote, source/revision identifiers and bytes `0..98`.
3. **Ask from sources** completed using the existing sign-in. Activity showed the answer that Aurora launches Tuesday with code BLUE-HERON, citation `[1]`, completed status and a provider turn identifier. Clicking the saved citation showed the frozen passage labeled as potentially historical.
4. Closed the window and confirmed the app process exited. Reopened the same workspace; its session and answer were visible. Submitted a semantic follow-up asking who the botanist is. Activity showed Niko, citation `[1]`, and two completed turns in the same session.
5. Started a longer synthetic answer and confirmed a running acknowledged provider turn. **Cancel** returned the app to idle and saved an interrupted turn with its partial answer.
6. Started another acknowledged turn, then closed the window. The recorded app and direct sidecar process IDs no longer existed on the following check. SQLite retained the fourth turn as interrupted with partial text. Reopening displayed two completed and two interrupted turns, with no automatic replay.
7. On the rebuilt UI, keyword search and passage selection worked with an unclipped single-line result button. **Withdraw** cleared current evidence; Search clearly reported a stale index. Reapproved the synthetic source and rebuilt the index.

The initial search button embedded a multiline quote in a fixed-height control and visibly clipped its first line. The fix uses a compact source title/range label and preserves the complete quote/revision in the detail view. Saved citation labels are similarly compact. A multiline-label regression test accompanies the fix. The rebuilt native app was visually checked; the native-feature desktop test suite passed six tests (one unit regression and five CLI tests). Formatting and native Clippy/build checks passed. Astra approved the scoped label fix with no material findings. This closes the previously blocked first-flow native acceptance gate, without claiming a packaged release or comprehensive UI accessibility qualification. Static text was visually readable but was not exposed in the observed accessibility tree, so screen-reader acceptance remains open.

## Reproduction

The credential-free suite creates and removes its own disposable fixture directories:

```sh
bash scripts/verify-end-to-end.sh
```

For a local developer trial, create an existing absolute data directory and build the native feature. Supply a selected Codex executable and the verified model directory produced by the earlier retrieval trial:

```sh
cargo run -p brn-desktop --features native-retrieval -- \
  --data-dir /absolute/existing/trial-data \
  --codex /absolute/path/to/codex \
  --model-dir /absolute/path/to/verified/model
```

In the Workspace page, select a file by entering its path, choose **Import and approve for search**, then **Build index**. Select a profile, search, inspect a passage, and ask from sources. Activity lists saved sessions, answers and frozen evidence; reopen the same data directory to continue. Import/approval controls affect retrieval eligibility only.

The headless driver uses the same workflow and prints structured results:

```sh
cargo run -p brn-workflow --features native-retrieval --bin brn-flow -- --help
# Subsequent commands use --data-dir /absolute/existing/trial-data:
# import --file /absolute/fixture.md --approve yes
# build --model-dir /absolute/path/to/verified/model
# search --query 'Your question' --profile hybrid
# ask --query 'Your question' --profile hybrid --codex /absolute/path/to/codex
# history --session <saved-session-uuid>
# ask --query 'Follow-up question' --session <saved-session-uuid> --codex /absolute/path/to/codex
```

Use a fresh operation UUID for a deliberate new question. Reusing one retrieves its recorded state; an interrupted/uncertain operation is never automatically retried. Missing provider history, a changed Codex store/account association or an unlinked session requires explicit recovery. Rebuilding derived indexes does not reset authoritative records.

## Remaining limits

This is a personal-trial implementation, not a release. Native acceptance now covers the synthetic workflow below; broad usability, smaller-window layouts and assistive-technology coverage remain open. Native model initialization and individual inference calls are not internally cancellable; local work may finish on an owned background reaper after window close, and cannot then enter the provider phase. Provider-active shutdown waits for cancellation and owned child cleanup. Closing during an active provider turn was observed to terminate the app and owned sidecar and retain an interrupted record; this was not a timing benchmark or a native-inference shutdown test.

Natural credential expiry/revocation, clean-machine sidecar installation, model acquisition UX, signing/notarization, power-loss/disk-exhaustion behavior, backup/restore and representative-corpus ranking remain unqualified. The known large unwind-table linker warning remains for the combined retrieval binary. Keyword ranking is a deterministic term-match baseline; semantic quality is not established by two synthetic documents. No merge, release, publication or private-document migration occurred.

Current official protocol/config references: [App Server](https://developers.openai.com/codex/app-server) and [configuration](https://developers.openai.com/codex/config-basic). The installed executable's generated schema was also checked because its read-only sandbox shape differs from newer documentation; unsupported filesystem-root restrictions are not claimed.
