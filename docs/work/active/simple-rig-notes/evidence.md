# Simple Rig-based notes app: evidence

Actual commands and results, recorded at the end of each step. Expected results in the plans are instructions, not evidence.

## Planning

- 2026-10-02: specification approved and committed at `c912480`. Roadmap, [spike](spike.md) and [store and vault](store.md) plans written. No product code changed; no builds, live calls or model downloads performed.
- Checked while planning: Rig `v0.43.0` source (ChatGPT/Copilot auth, client `authenticate`, agent streaming and `Tool` signatures, facade features); `rig-sqlite` uses rusqlite `0.32` and `rig-fastembed` uses fastembed `4.5`, so neither is used; SQLite `VACUUM INTO ?1` accepts a bound path (checked with Python's sqlite3).

## Plan review (2026-10-02)

The user requested a GPT-6.1 Sol (high reasoning) review of the step 1–2 plans. Accepted and applied:

- Missing `brn.sqlite` is restored from the newest backup, like a corrupt one (spec section 3).
- Existing databases and backups must carry BRN's application ID; unbranded or foreign databases are refused unchanged; empty or damaged backups are skipped.
- Backups use SQLite's online backup API (spec section 3), and a new backup's number is always above existing ones, so pruning never deletes it.
- `read_note` compares the checked file identity with the opened file, refusing a symlink swapped in between.
- Non-UTF-8 note content is listed as unreadable by step 3's indexing.
- Spike: credentials folder must be a real folder owned by the user with mode 700; ChatGPT email read from the cached `id_token`; rule against copying raw errors, IDs or emails into evidence; cancel check records whether text arrived first.

Partly accepted: a full descriptor-based vault walk and sanitizing code for the throwaway spike were judged too heavy for this app; the identity check and the evidence rule replace them.

Check: the step 2 store and vault code and tests from the revised plan were copied into two scratch crates outside the repository (stub crate-root helpers, rusqlite `=0.40.2` with `bundled`/`backup`, sha2, Rust 1.98.1, offline). Results: store 14 tests passed, vault 6 tests passed, `cargo clippy --all-targets -- -D warnings` clean for both. Scratch crates deleted afterwards. The spike code was not compiled (Rig is not in the local registry).

## Step 1: spike

- 2026-10-02; macOS 26.5, Rust 1.98.1; commits `52b9756`, `065f454`, `745024d`, `7c8b392`, plus the commit containing this entry (`docs: record Rig spike findings`). [Findings and decision](../../../../experiments/rig-spike/FINDINGS.md).
- Tasks 1–2 **PASS** after direct TLS correction: build, graph, dependency trees, strict Clippy and four synthetic tests; one libsqlite3-sys 0.38.2, FTS5 hit count 1. Initial facade TLS resolution failed; corrected graph passed.
- Earlier offline commands (repository root):
  `cargo build --manifest-path experiments/rig-spike/Cargo.toml --locked`;
  `cargo run --manifest-path experiments/rig-spike/Cargo.toml --locked -- graph`;
  `cargo tree --manifest-path experiments/rig-spike/Cargo.toml --locked -i libsqlite3-sys`;
  `cargo tree --manifest-path experiments/rig-spike/Cargo.toml --locked -d --depth 0`;
  `cargo clippy --manifest-path experiments/rig-spike/Cargo.toml --all-targets --locked -- -D warnings`;
  `cargo test --manifest-path experiments/rig-spike/Cargo.toml --locked`.
- Task 3 **PASS** for both logins/identity; first device codes expired, retries succeeded. Copilot models **PASS**; ChatGPT models **FAIL** (HTTP 400).
- Controller's live commands, in generic form: `$S` = experiment debug binary, `$C` = external mode-0700 credentials folder; prompts are scenario placeholders, not retained literal text:
  `$S login chatgpt "$C"`; `$S login copilot "$C"` (each retried);
  `$S models chatgpt "$C"`; `$S models copilot "$C"`;
  `$S chat chatgpt "$C" MODEL "TOOL_PROMPT"` for gpt-5.4, gpt-5.3-codex, gpt-5.5, gpt-5.6-sol, gpt-5.6-terra, gpt-6-sol, gpt-6.1-sol;
  `$S chat copilot "$C" MODEL "TOOL_PROMPT"` for gpt-4o, gpt-5.4, claude-sonnet-5;
  `$S cancel copilot "$C" gpt-4o "TOOL_PROMPT" 1500`; `$S cancel copilot "$C" gpt-4o "LONG_PROMPT" 4000`;
  `rm "$C/copilot.json"`; `$S chat copilot "$C" gpt-4o "TOOL_PROMPT"` (session refresh).
- Task 4 Copilot **PASS**: tool/stream, restart reuse, local cancel (partial text at 4000 ms), session refresh. ChatGPT **BLOCKED**: supported routes hit HTTP 429 usage limit; tool/restart/cancel not observed.
- Task 5 **PASS** (fresh checks): `cargo check --manifest-path experiments/rig-spike/Cargo.toml --locked --features cassette`; `cargo check --manifest-path experiments/rig-spike/Cargo.toml --locked --features test-utils`. Both exit 0; no lockfile change. HTTP cassette engine/fixtures not exercised.
- Decision: **Copilot GO; ChatGPT CONDITIONAL GO**. Rerun ChatGPT Task 4 with gpt-5.5 after 2026-10-03 20:24 EEST before accepting step 4's ChatGPT work. No live calls by Task 5; no native acceptance or server-side cancellation claim.

## Step 2: store and vault

- 2026-10-02; commits `ad6753c`, `b9f2c80`, `8281224`, `2cfa465`, `307c58a`, `1fd4051`, `131e241`, plus the commit containing this entry (`docs: adopt simple notes app rules for new code`).
- Platform/toolchain: macOS `26.5` (`sw_vers -productVersion`); `rustc 1.98.1 (48a229cea 2026-09-01)` (`rustc --version`).
- Implemented beside existing code: `brn_store::work::WorkStore` (open/check/restore/backups/settings/unsaved edits) and `brn_workflow::vault` (paths/scan/exact-byte reads); updated new-code rules and transition documentation.
- Review fix round 1: header-branded foreign or newer databases are refused before SQLite opens them; unbranded foreign databases are opened before being refused (their WAL may be checkpointed; accepted limit).
- final review fix: empty live database is restored from the newest backup.
- Review fix round 2: vault read size bound and hidden non-UTF-8 names.
- `cargo fmt --all -- --check`: **PASS**, exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: **PASS**, exit 0, no warnings.
- `cargo test -p brn-store -p brn-workflow --locked`: **PASS**, exit 0; 272 reported passed, 0 failed, 0 ignored across 19 result groups (including empty binary/doc-test groups); WorkStore 16 and vault 8.
- The non-UTF-8 filename test self-skips on APFS (`EILSEQ`, os error 92), counted by the runner as passed; its filename assertions were not exercised.
- Confirmed with `cargo test -p brn-workflow --test vault scan_reports_non_utf8_names_but_skips_hidden_ones --locked -- --nocapture`: exit 0, 1 reported passed, 7 filtered out; self-skip message observed.
- `git diff --check`: **PASS**, exit 0; `git diff --cached --check`: **PASS**, exit 0.
- New relative Markdown links resolve; the staged active README excludes the user's UI slice 2 row, which remains unstaged.
- Accepted read race limit: a folder swapped for a symlink during a note read can still lead outside the vault; this needs write access inside the vault.
- Checks used repository-local disposable fixtures via `TMPDIR`; no original vault/old data, live provider calls, model downloads or native usability acceptance exercised.

## Step 4: AI chat planning

- 2026-10-02: [AI chat implementation plan](chat.md) written and linked from the roadmap, active-work index and current-status transition note. Eight test-first slices cover protected subscription login, WorkStore V2 conversations, Rig read tools/stream policy, current-vault adapters, an independent chat lane, CLI/desktop cutover and App Server retirement.
- Planning baseline: `main@99013df`. Other Step 3 retrieval edits appeared in the shared worktree during planning and HEAD advanced to `10cd354`; those changes and the pre-existing UI slice 2 documentation remain untouched by this task. Reconcile newer Step 3 inputs before execution.
- Inputs read: approved simple-notes spec, spike findings/evidence, WorkStore/vault and note-index source, current workflow/CLI/native ownership, search plan and BRN theme tokens. Pinned public Rig `v0.43.0` source confirms `DeviceCodeHandler::new`, prompt `history`/`max_turns`, `on_model_turn_finished` and mock-model support. Proposed product interfaces have not been compiled.
- Gates retained: Step 3 Library/local-embedder completion before real read/search integration; ChatGPT live chat/restart/cancel recheck only on a separate request after its observed quota-reset time. Unsaved-edit concurrency can be checked in Step 4; actual simple Markdown Save concurrency is a Step 6 acceptance check.
- Documentation checks: `git diff --check` **PASS**; inline local Markdown file/fragment checker **PASS** (57 links across chat plan, roadmap, active index and status; final pass 61 including this evidence file); new-file whitespace checks **PASS**; eight-task/spec-coverage and placeholder self-review performed. No Rust builds or product tests run for these documentation/artifact changes.
- Local review surface: `.lavish/task-4-ai-chat.html`, with a copy of the complete plan as `chat-plan.md`. Uses BRN's existing Menlo/Georgia light/dark palette, local-only assets, annotated SVG architecture and an explicit plan-only feedback form. `lavish-axi .lavish/task-4-ai-chat.html` returned `status: opened`; no third-party publication.
- Artifact checks: cached offline Playwright lookup initially failed (`ENOTCACHED`); package restored through npm after that missing-dependency failure. Headless dark/light/375px render and screenshot inspection **PASS**; no horizontal page overflow/page errors; theme control works; changing a choice sends nothing, submitting queues exactly one feedback prompt. The full-plan asset matches the source.
- This task changes documentation and a local review artifact only: no product implementation, account actions, live calls, model downloads, original-vault access, migration, commits, push, merge or acceptance claimed.

## Step 3: search

Final review fixes: index open checks the file's identity before damage and refuses anything not ours; semantic hits are read in one statement; embeddings are stored only for unchanged passages and model; the model identity covers all five files.

- 2026-10-02; tested `main@94de72d` with documentation-only dirty changes; implementation commits `80664a9`, `5e41142`, `99013df`, `10cd354` (fix), `180451f`, `9e6a735`, `94de72d` (fix), plus this documentation commit (`docs: record search step`).
- `rustc --version`: `rustc 1.98.1 (48a229cea 2026-09-01)`; macOS; deterministic fixtures in an explicit disposable `TMPDIR`.
- Implemented: disposable notes/passages/FTS5 index, stored embeddings/fusion, local embedder and workflow `Library`; LanceDB removed. CLI `search`/`notes list` moved to step 4 by controller ruling.
- Review fix round 1: `NoteIndex::open` refuses other healthy databases unchanged; rebuilds only corruption, its own outdated/incomplete schema or empty files; propagates other errors. Vector normalisation uses `f64`.
- Review fix round 2: every search reports `keyword_only` without an installed model; note titles skip YAML frontmatter.
- `cargo fmt --all -- --check`: **PASS**, exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: **PASS**, exit 0, no warnings.
- `cargo test -p brn-retrieval -p brn-workflow --locked --no-fail-fast`: **PASS**, exit 0; 181 reported passed, 0 failed, 0 ignored across 19 result groups.
- `note_evidence::managed_missing_import_fails_as_stale_instead_of_uncategorized_io`: **PASS** this run; the known failure on `main@d739364` did not reproduce. `cli_ask` is outside these selected packages, so its load-sensitive deadline test was not exercised.
- `cargo test -p brn-retrieval --locked --features native`: **PASS**, exit 0; 33 reported passed, 0 failed, 0 ignored across 7 result groups; includes one self-skipped local model test, not model verification.
- `BRN_NATIVE_MODEL_DIR` unset; no model downloaded. Confirmed skip with `cargo test -p brn-retrieval --locked --features native --test local_embedder local_model_embeds_by_meaning -- --exact --nocapture`: exit 0, 1 reported passed, 1 filtered out; explicit skip message observed.
- `cargo check -p brn-desktop --locked --features native-ui,native-retrieval`: **PASS**, exit 0 without `protoc` (unavailable); confirms Task 4's earlier build report after LanceDB removal. Upstream GPUI dependency `block v0.1.6` future-incompatibility warning remains.
- `git diff --check`: **PASS**, exit 0; final staged whitespace and relative-link checks recorded in the Task 6 report.
- No live provider calls, original-vault/old-data access, model inference, native usability acceptance or release qualification claimed.

## Step 4: Opus plan review and revisions

- 2026-10-02: the user requested Claude Opus 5.5 with high reasoning to review [the chat plan](chat.md), then apply improvements agreed with. One read-only reviewer returned R1–R10; its initial verdict was not ready to execute because ownership, authentication, concurrency and dispatch contracts were incomplete. The revisions below are the author's verified dispositions, not a second reviewer approval or user acceptance.
- Reviewed baseline advanced from `b0d6b17` to `main@e24b104` during this task. Read and preserved the concurrent retrieval correction: foreign-file refusal, consistent semantic rows, unchanged-passage/model insertion and five-file model identity. No Rust source, original vault, old data or unrelated UI-slice-2 records were changed by this task.

| Finding | Disposition | Plan change |
| --- | --- | --- |
| R1: stale Step 3 baseline and download promise | Adopted; trailer sub-suggestion declined | Completed Library/LocalEmbedder are inputs; historical skipped inference stays explicit. Model installation is planned rather than silently deferred. Keep the currently required `Copilot` trailer, not the historical `Copilot App` name. |
| R2: undefined query embedder ownership/policy | Adopted | One SharedEmbedder, shared Library/tool search helper, retrieval-owned read-only queries and model-identity/dimension checks. |
| R3: mutable Auth held across stream/login | Adopted | Owned provider clients release per-provider guards before streaming; responsive dispatch, other-provider concurrency and target-provider Disconnect fence/cancel/join/delete. |
| R4: frontend-only mixed-store guard | Adopted | Symmetric Store/WorkStore checks after the shared owner lock, before SQLite; sidecars and recognized work backups included. |
| R5: CLI command collisions/UUID regression | Adopted | Explicit command-by-mode matrix; preserve `notes show UUID`, legacy search/history and empty-folder choice; retire `ask --profile` with documented usage failure. |
| R6: undefined desktop owner/default/vault choice | Adopted | AppWorker-only views; BRN-simple default, explicit legacy route and first BindVault action. CLI Ask also uses the owner lane, not a preflight-bypassing direct chat call. |
| R7: begin-turn and attached-lock lifetime | Adopted | None allocates; unknown Some fails before insertion; ChatStore holds the owner's Arc<File>; owned shutdown/Drop joins. |
| R8: cancellation/tool failures/refresh ambiguity | Adopted | Typed budget flag, safe model-visible failed tool results, bound-vault/refresh preflight and no network on terminal replay. |
| R9: missing consent/native download wiring | Adopted | Pinned five-asset installer, explicit consent/decline/cancel/progress, size/digest/exclusive-install checks, offline fixtures and typed non-native refusal. |
| R10: mock transport cannot test continuations | Adopted with correction | Recording client has a fixed response; sequenced streaming helper serves one stream's chunks. Use a private queue-backed multi-completion transport, not that helper alone. |

- Verification basis for the decisions: current store/workflow/CLI/desktop source and manifests; pinned public Rig `v0.43.0` auth, transport, runner/hook and test-helper source. Authenticate captures credentials into an owned client; the model-finished hook precedes tool dispatch. These observations do not compile the proposed interfaces.
- Installer manifest was pinned from public Hugging Face metadata only: immutable revision `751bff37182d3f1213fa05d7196b954e230abad9`, ONNX LFS SHA-256 and small-file Git blob IDs/sizes. No model assets fetched or inference run. Synthetic installer tests are future instructions, not completed qualification.
- Roadmap/status/active index and saved `.lavish` review files were synchronized. The earlier interactive review session had been explicitly ended; it was not reopened. Product implementation, account actions, live calls, migration, commits/push/merge/release and user acceptance remain pending.
- Fresh documentation checks on `main@e24b104` with this planning diff: `git diff --check` **PASS**; inline local Markdown file/fragment checker **PASS**, 64 links across chat/roadmap/evidence/status/active index; `git diff --no-index --check /dev/null FILE` checks for the new plan/HTML/copied plan **PASS**, with no whitespace diagnostics. Eight tasks, no TODO/TBD/FIXME placeholders, revised interface/dispatch/feature contracts and 91,100,408-byte asset total checked; complete-plan copy matches byte-for-byte.
- Fresh saved-artifact check: cached `npm exec --offline --yes --package=playwright -c 'command -v playwright'`, then the session's `render-chat-plan.cjs` **PASS** for dark/light/375px layouts, theme control, no page errors/horizontal overflow and exactly one mock feedback queue on submit. Narrow/light diagram screenshots inspected; shortened a diagram label to stay inside its box. This is headless artifact rendering, not native product acceptance. No Rust build/tests run for documentation-only changes.

## Step 4: implementation execution

- 2026-10-02: user authorized implementation with GPT-6.1 Sol (medium reasoning) and Claude Opus 5.5 (medium reasoning) reviews, then explicitly approved isolated worktree/branch `task-4-ai-chat` from `e24b104`. The original main checkout and unrelated dirty records remain untouched; only task-owned planning records were imported. Execution does not authorize live logins/provider calls, model downloads, old-data migration, push/merge/release or native acceptance.
- Fresh isolated baseline: `cargo build --workspace --locked --offline --quiet` and `cargo test --workspace --locked --offline --quiet` **PASS**, exit 0, default features, explicit disposable TMPDIR. Some existing tests emit their expected CLI JSON. No native/UI/inference/provider qualification claimed.

## Step 4: durable Tasks 1–7 implementation/review handoff

Recorded 2026-10-03 from the complete controller ledger and ignored task reports.
These are **historical predecessor results**, not newly repeated Task 8 checks.
Tasks 1–7 received independent Opus spec/quality approval, including scoped
re-review of fixes. Review is not user acceptance or merge authorization.
The execution-start documentation commit is
`ea3efcf99a1edfe082a55ad8bbf4f5bd2a56e6af`.

| Task | Exact implementation and follow-up commits | Reviewed result and essential evidence |
| --- | --- | --- |
| 1 authentication | `73ba8a69321efe788b5ce71f996483a0e58459da`; fix `7eab5a18928c264fc194b3b8aababb169811352b` | 29 original synthetic tests; fix 33 passed. Real pinned authenticators with private HTTP fixtures, explicit login, permission finalization, secret-safe errors, provider independence and safe Disconnect. RED: default macOS ancestor caused 23 failures; stale 0644 cache/deletion regressions then 2 failures. Fix canonicalizes fixture roots, never weakens production ancestry checks. |
| 2 WorkStore V2 | `d0d03ca761074aebae941ec6f1237f66b2e6d2ff`; accidental report commit `c5ec5c05e270c196b0082c02c4ffee8b8b7bf056`; removal `0fd9091d200cc163b4951b667ba0efb0652eedba` | 18 work +12 chat tests passed; V1/legacy V6 preserved, exact paired text/provenance, UUID conflict/replay, restart Interrupted before backup, 21 retained turns. Behavioral RED: 9 passed/2 failed before reconciliation. Review blocker was tracked scratch only; removed without history rewrite. Reports now ignored. Immediate transactions were a required downstream obligation, completed in Task 5. |
| 3 Rig streamed read tools | `cd40c77517195ef17ce895af4d67cf7e3d09da2c` | 54 AI tests passed, strict checks passed. Three actual Rig wire routes intercepted by synthetic transports; exact emitted partial text, text-only history, safe typed errors and read-only caps. Reversible off-by-one mutations failed: max_turns 8 loses the ninth final answer; ninth budget allowed 18 rather than 16 tool calls. Restored policy proves eight rounds, ninth dispatch **zero**. |
| 4 App/current tools/installer | `acfb0519d81241aa46d5861771b4831d35827a17`; fix `57895371aeec2f0fe3f17d96473fc7f8d7c52cba` | Original 102 default +16 native synthetic passes. Fix five behavioral REDs, then 13 default +6 native passes and strict/downstream checks. Exact `.credentials` suffix; default reopening ignores saved native model without touching work/assets; explicit unsupported Download refuses before consent changes or prompt. Native saved-model precedence/invalid-model errors preserved. |
| 5 owned lanes | `fcd6067b1a7f82d83896a8b232bf2b0f8721f818`; fix `3a1af1b15f5adbd90e2395d44c457104a0bcb46f` | Final private workers 24 default and 24 native; integration AppWorker 6 +ChatWorker 3 in both builds, strict checks. Shared owner Arc<File>, Immediate WAL transactions and 50 synchronized owner/attachment cycles; target Disconnect fences/cancels/joins before deletion; persistence failure remains unsaved. Fix REDs showed false late indexing failure without vault; startup/activation/batches now gate on both model and available vault, while BindVault/restored Refresh resume indexing and real errors remain visible. |
| 6 CLI cutover | `bef8d5589cdfa1be1bb115976369c9eb64a820e5`; fix `311971ffc2d2f1be1768db7fa48c9dbc3dca3f5d` | Original full CLI 156 passes; final focused 17 AI/Ask/library passes. Status fix real subprocess RED, then AI 6 +Ask 6, projection 9 default/9 native and strict checks. Authority dispatch, saved credentials Option, frozen terminal replay, deadline/signal/pipe truth, one envelope, typed errors and fresh consent. Stale selection is a safe diagnostic alongside both actual account statuses, never fallback or selection rewrite. |
| 7 native cutover | `a6bf6f42d47ec04aa57aac68d0921c79e6ce2836`; fix `9c2f3ac60a20e04fdd149a485dd677c6af08e8f8` | Final 56+6 default and 122+6 native desktop passes; default/native strict Clippy and native build passed. High composer-generation RED discarded late text; search generation is now independent of frozen chat/history generation. Medium final-quit RED exposed undrained real accepted Save; legacy critical notes synchronously join **before** returning the timed future, while guarded close remains off GPUI. No new final flush; simple Dock deadline remains an honest limit. |

Historical reproduction selectors (run from this worktree; pinned Rust 1.98.1,
`TMPDIR=/Users/evokessler/.brn-task5-fixtures` for credential ownership):

```sh
cargo test -p brn-ai --lib --locked --offline
cargo test -p brn-store --test work --test work_chat --test work_chat_attachment --test workspace_modes --locked --offline
cargo test -p brn-workflow --test app --test ai_tools --test models --test app_mode_cli --locked --offline
cargo test -p brn-retrieval --features native --lib --test model_download --locked --offline
cargo test -p brn-workflow --features native-retrieval --lib models_tests --locked --offline
cargo test -p brn-workflow --lib simple_worker_tests --locked --offline
cargo test -p brn-workflow --features native-retrieval --lib simple_worker_tests --locked --offline
cargo test -p brn-workflow --test app_worker --test chat_worker --locked --offline
cargo test -p brn --test cli_ai --test cli_ask --test cli_library --locked --offline
cargo test -p brn --bin brn cli::library::tests --features native-retrieval --locked --offline
cargo test -p brn-desktop --locked --offline
cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline
```

Counts above apply to their recorded commits, not necessarily today's evolved
suite. No zero-filtered integration run is claimed as coverage. Installer and
loader factories are private cfg-test seams; they replace synthetic transport/
loading only, retaining production storage/activation/settings/lifetime checks.
No real model, ONNX inference, accounts or graphical acceptance is implied.

### Every explicit controller Ruling and downstream costs

The complete progress ledger contains one explicitly labelled `Ruling:`:
**validate safe directory at Auth::open, isolate cache reuse validation by
provider, and let explicit Disconnect unlink recognized own-UID cache entries
safely without reading/reusing unsafe content.** Rationale: provider independence
and recovery after process kill. Cost if wrong: unsafe target files could be
reused, followed, repaired or deleted too broadly. Tight tests retain reuse
refusal, no-follow/identity checks, other-provider/unknown-file preservation and
preflight refusal of foreign owners/directories. Fix `7eab5a1` passed scoped
Opus review. Same-UID adversarial cross-process replacement is not atomically
eliminated by those checks.

Other reviewed carry-forward decisions and their failure costs:

- Immediate transactions and one shared owner descriptor prevent WAL snapshot
  upgrade conflicts and premature owner release; wrong ordering would lose
  finalization or admit a second owner.
- Private ChatWorker, public AppWorker-only admission ensures every new Ask
  refreshes/validates tools; exposing raw chat submission would bypass currentness.
- `.credentials`/Option None preserve saved/default location; the wrong suffix
  would split accounts, and reading an extra frontend store would violate ownership.
- Default model gates preserve native-used history and refuse unsupported
  installation before consent; wrong gating would block local work or falsely
  authorize requests.
- Vault/model indexing gates distinguish unavailable prerequisites from real
  embedding failures; suppressing all errors would hide a stale/broken index.
- Stale selection status remains diagnostic, not an Ask fallback; the wrong
  projection would hide accounts or silently change model/provider.
- Composer/search separation preserves streaming and follow-up identity; legacy
  final-quit synchronous admitted drain avoids GPUI deadline abandonment. Wrong
  routing would discard visible text or lose accepted critical note writes.

Still deferred to controller whole-branch review: fail-closed startup pair
validation/full-text counting cost, replay Selection-query deadline edge,
shutdown reattachment/discovery error edges and refresh restoration counts.
Failed/cancelled Connect consumers query real Status; pre-admission Stop intent
is retained/retried or joined. RecoverEdit proves independent SQLite recovery
while a model is pending, **not future Step 6 Markdown Save concurrency**.

## Step 4 Task 8: production retirement and fresh integrated offline gate

2026-10-03, macOS 26.5, `rustc 1.98.1 (48a229cea 2026-09-01)`.
Worktree `.worktrees/task-4-ai-chat`, branch `task-4-ai-chat`, clean starting base
`9c2f3ac60a20e04fdd149a485dd677c6af08e8f8`.
Product/contracts commit: **`9263adcb87254de2296257eea6f8186da90c0fa5`**.
Tested as that base plus the then-uncommitted Task 8 delta; all compiled Rust,
manifests, lockfile and Python fixture/checker bytes are identical in that
commit. Only a documentation-index clarification and restoring the shell
script executable bit followed the gate; it was invoked through Bash.
This appended handoff is documentation-only, after the gate.
Pre-gate tracked binary-diff SHA-256:
`54e9c4cb652044b35a9b3aa3df7a260b74362703f02a5f90283133cd88f60750`.
New-file SHA-256: legacy_retirement
`322efffbd01ee1edf4affd9fd49d15a420b805a91bc0774c5ff118528cabe029`;
retirement checker
`e773589e627f59bea7378ead1b9cd3d9815c5045bc6601db72c5795812c412d0`;
fixture runner
`0258e496c726ab1f7634eb43a24d93ad89bc15784bd3069de912119c42ea1bea`.

Meaningful retirement **RED**:
`bash scripts/verify-end-to-end.sh --retirement-only`, exit 1, actual existing
root/member/dependency/source/configuration references detected, ending
“Production App Server reference remains”. The initial checker excluded code
after any test declaration; tightened it to trailing inline private test modules,
so early out-of-line test declarations do not hide production. Integration/
private rejection tests may spell the rejected flag; production tokens are
removed, not disguised. Historical docs and standalone trials remain evidence.
Final same command/checker **GREEN**, exit 0.

Removed the four tracked files of `crates/brn-provider` after removing its
consumers, then root member. Removed Workspace's implementation, provider-active
worker phase and unused resume/context validation helpers; legacy entries now
return local typed LegacyAiRetired before lookup/callback/storage/network work.
Retained every non-provider `flow` (4) and `note_evidence` (13) case, plus added
2 retirement/history-wire checks. Removed only 5 old fake-provider flow cases
and 7 fake-provider note-evidence cases; Rig/worker tests cover current
signals/persistence/late completion invariants. All legacy Store/recovery/editor/
draft/comment/local-search/history implementations and admitted-critical-note
join behavior remain, with legacy wire fields unchanged.

Lockfile correction: an initial offline generate-lockfile unnecessarily selected
new cached compatible transitive versions. Discarded that regenerated file,
restored HEAD's lockfile and let one unlocked offline workflow check remove only
the retired package/edge. Final lock diff is **9 deleted lines, zero upgrades**;
all subsequent Cargo checks used `--locked --offline`. Nothing was fetched.
An early targeted command's name filter ran 2 library tests but **zero**
integration tests; corrected unfiltered run passed flow 4 +note_evidence 13
+retirement 2, and CLI Ask 6 +basic 11 +signals 3.

Fresh one-time integrated gate, all commands **exit 0**:

```sh
export TMPDIR=/Users/evokessler/.brn-task5-fixtures
unset BRN_NATIVE_MODEL_DIR
cargo fmt --all -- --check
cargo build --workspace --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo test -p brn-retrieval --features native --lib --test model_download --locked --offline
cargo test -p brn-workflow --features native-retrieval --lib --test models --locked --offline
cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo build -p brn --features native-retrieval --locked --offline
bash scripts/verify-end-to-end.sh --fixtures-only
bash scripts/test-make-macos-app.sh
git diff --check
```

| Fresh output | Actual count/features/limits |
| --- | --- |
| Workspace tests | **667 reported passed**, 0 failed/ignored/filtered, 63 result groups (including empty binary/doc/native-disabled groups); CLI 158, AI 55, core 12, desktop 56+6, retrieval 40, store 147, workflow 193 |
| Native installer | **9 library +2 model_download passed**; synthetic pinned-manifest/HTTP/exclusive install, no production assets |
| Native workflow | **79 library +5 models passed**; includes all 24 private worker cases and native model-loader case, unfiltered; no ONNX |
| Full native desktop | **122 unit +6 CLI passed**; actual production state/routing, accepted legacy quit Save drain and byte round-trip, not GUI usability |
| Fixture-only end-to-end | **47 assertions passed**, simple saved-vault read/search/exclusions/refresh/keyword-only + separate legacy import/reimport/search/staleness/history/retired Ask; no duplicate workspace suite |
| Launcher | Script exit 0, “macOS launcher checks passed”; custom quoted paths, new default, explicit legacy, missing paths/binary and launch failure checked; no runner test count emitted |
| Format/build/strict workspace lint/native builds/diff | All exit 0; native build warning remains for pre-existing `block v0.1.6` future compiler incompatibility |

The 667 runner total includes one APFS non-UTF-8-filename self-skip.
Confirmed separately with unfiltered exact selector/nocapture: 1 reported pass,
7 filtered, explicit EILSEQ skip; filename assertions were **not exercised**.
Native-disabled `local_embedder`/`model_download` zero groups are not coverage.
Real-model local_embedder target was not run; no production assets were available
or downloaded. Feature pass totals overlap tests, not distinct-case counts.
Ten exclusively new gate layout fixture directories were removed by exact
pre/post ownership comparison; all pre-gate directories were preserved.
Shell fixtures remove only their exclusively created entries, not broad
recursive paths; no named-process killing or original-data access.

Current qualification is implemented/offline verified, Tasks 1–7 reviewed;
Task 8 independent review and then whole-branch Opus review belong to the
controller. No Task 8 reviewer/subagent, account actions, live providers,
original vault/trial-workspace inspection, migration, push, amend, merge,
release or user acceptance. Both providers still need separately authorized
login/tool/stream/restart credential reuse/refresh/error/partial Stop observation;
ChatGPT's historical quota-blocked scenarios remain unaccepted. Native build/
state checks do not establish native usability. Simple readers only: Step 5
proposal/approval tools and Step 6 Markdown Save/cleanup remain unimplemented.

## Final review / fixwave1: conversation display and Ask deadline projection

2026-10-03, macOS 26.5, pinned `rustc 1.98.1 (48a229cea 2026-09-01)`.
Authorized worktree `.worktrees/task-4-ai-chat`, branch `task-4-ai-chat`,
clean starting HEAD `3d98c0578848cb8dd719c8c3db10c3c098d199ff`.
Controller-reported final Opus **APPROVE WITH FIXES** had F1 Medium and F2 Low;
this fixwave implements those two findings, not final approval.
Tested product commit: **`6a82b782ad7c3ba968088bf9a0f5902518acc4eb`**.
All final gates below ran against its identical Rust/README bytes before the
product commit. This appended evidence follows separately, documentation-only.
Scoped controller Opus re-review of F1/F2 and new fix bugs remains pending.

### Root causes and bounded fixes

- **F1:** navigation increments display generation, but the owned request keeps
  its admission generation. Progress and terminal rendering incorrectly required
  equality with display generation, so revisiting C discarded deltas and left a
  historical Running row. Exact envelope UUID, request UUID and request-generation
  correlation is retained; owned text/tool progress now accumulates in every
  display. Existing-conversation rendering/Finished uses conversation identity.
  First-chat None still requires its original blank display generation.
  The native renderer uses the tested `display_active`/`display_turns` production
  helpers, hiding the actually live Running row in favor of its provisional
  stream. Finished upserts by turn UUID, clears global active even in another
  display, and never inserts C into D/new blank. A correlated Turns snapshot
  preserves known terminal rows in this display over older Running snapshots.
  Navigation clears that bounded display overlay; no global terminal cache,
  new module or workflow API was introduced.
- **F2:** the CLI queried Selection even for frozen replay, potentially joining
  at a deadline, then mapped a Cancelled Ask submission through the deadline-blind
  classifier. Recorded replay now skips Selection. New Ask still queries it and
  requires a saved selection. Actual Ask submission maps through
  `lane.command_error`, retaining 124 for deadline and 130 for SIGINT without
  relabeling genuine typed failures. `ask` is generic over the existing private
  EventLane seam solely to exercise its real consumer path deterministically.
  No public fake feature, sleep, provider flag, dependency or lockfile change.

### Behavioral RED, then GREEN

Commands ran from the authorized worktree with
`TMPDIR=/Users/evokessler/.brn-task5-fixtures`:

```sh
cargo test -p brn-desktop --bin brn-desktop followup_ --locked --offline
cargo test -p brn --bin brn actual_ask_submit_after_join --locked --offline
cargo test -p brn-desktop --bin brn-desktop late_running_snapshot_preserves_known_terminal --locked --offline
```

- First desktop RED: **1 passed, 2 failed, 55 filtered**, exit 101.
  Same-conversation click expected `full partial`, got empty; C→D→C expected
  `away back`, got empty. Existing composer/follow-up test passed.
- CLI RED: **0 passed, 1 failed, 36 filtered**, exit 101: the actual `ask`
  consumer submitted after joined deadline and returned **130 instead of 124**.
- Separate late-snapshot RED with the snapshot merge withheld:
  **0 passed, 1 failed, 60 filtered**, exit 101: a locally observed Completed
  became Running. Restored the merge before final verification.
- Initial desktop invocation mistakenly requested nonexistent `--lib` (exit
  101); corrected to `--bin`. Initial standalone snapshot test was accidentally
  nested, yielding zero selected tests plus an inner-item warning; moved it to
  module scope before the substantive RED. Neither is counted as coverage.
- Intermediate scoped GREEN: **22 AiState +10 CLI private consumer cases**
  passed. Final unfiltered suites include all new cases below: same-C history
  reload, away/back full partial, strict stale envelope/generation exclusion,
  snapshot-before/after-Finished ordering, duplicate Finished, off-display end,
  fresh history after off-display end, blank first-chat unsaved Copy truth and
  production render predicates. Actual CLI Ask tests cover joined deadline 124,
  retained SIGINT 130, mandatory new-Ask selection and frozen Completed replay.
  Existing late Completed-wins, typed storage failure, obsolete profile,
  unbound/no-auth replay and explicit conflicting-payload cases also passed.

### Final exact-source offline gates

All commands **exit 0**. Tests were captured with `set -o pipefail` and `tee`
to ignored `.superpowers/sdd/chat/final-fixwave1-*.log`; summary counts were
computed from every runner result group, not filtered output alone.

```sh
cd /Users/evokessler/repos/brn-rust/.worktrees/task-4-ai-chat
unset BRN_NATIVE_MODEL_DIR
export TMPDIR=/Users/evokessler/.brn-task5-fixtures
cargo test -p brn --locked --offline
cargo test -p brn --bin brn cli::library::tests --features native-retrieval --locked --offline
cargo clippy -p brn --all-targets --locked --offline -- -D warnings
cargo clippy -p brn --all-targets --features native-retrieval --locked --offline -- -D warnings
export TMPDIR="$PWD/target/desktop-fixtures"
cargo test -p brn-desktop --locked --offline
cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo build -p brn-desktop --locked --offline
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo clippy -p brn-desktop --all-targets --locked --offline -- -D warnings
cargo clippy -p brn-desktop --all-targets --features native-ui,native-retrieval --locked --offline -- -D warnings
TMPDIR=/Users/evokessler/.brn-task5-fixtures cargo test -p brn-workflow --test legacy_retirement --locked --offline
bash scripts/verify-end-to-end.sh --retirement-only
cargo fmt --all -- --check
git diff --check
```

| Fresh check | Actual result |
| --- | --- |
| FULL default CLI | **160 passed**, 17 result groups, 0 failed/ignored/filtered; 38 unit plus all 122 integration cases |
| Native-retrieval CLI private consumer | **11 passed**, 27 filtered unit cases; this is not a full native CLI suite |
| Default desktop | **61 unit +6 CLI passed**, 0 failed/ignored/filtered |
| Full native desktop (`native-ui,native-retrieval`) | **127 unit +6 CLI passed**, 0 failed/ignored/filtered |
| Default/full-native desktop builds | Passed; no GUI launched |
| CLI/default+native and desktop/default+full-native strict Clippy | All four commands passed |
| Workflow legacy retirement | **2 passed**, 0 failed/ignored/filtered; actual legacy authority storage/history and admission guards |
| Production retirement checker / format / diff | Passed |

The pre-existing `block v0.1.6` future-compiler incompatibility warning remains
on full-native commands; no strict Clippy warning/error remains. Feature totals
overlap cases, not unique-case counts. No unchanged whole-workspace suite,
native model assets, graphical interaction, account action, live provider,
original vault, future Step 5/6 implementation, main-worktree modification,
amend, push, merge or release. Offline native state/build checks do not establish
native usability, real credential validity, model inference or user acceptance.

CLI auth-related tests used the canonical outside-Git synthetic fixture parent.
Desktop layout checks used the explicit owned
`target/desktop-fixtures` parent. An exact before/after comparison removed only
**10 new resolved, own-UID, non-symlink layout directories**; there were zero
preexisting layout entries, and unrelated fixture directories were preserved.
Ignored reports/logs were not force-added. Both commits carry Co-authored-by.
Next action belongs to the controller: scoped final Opus re-review of the
product diff `3d98c05..6a82b78`, including both findings and fix-introduced bugs.
Implementation/offline verification is complete; final review/acceptance and
branch integration remain pending.

## Final reviewed offline handoff

2026-10-03, isolated `task-4-ai-chat`, tested/reviewed
`dccc24a0bd20992c6e22e0844202b25726fc5985`. This section supersedes the
earlier pending-review status, not its dated test observations.

All eight tasks received independent Claude Opus 5.5 medium reviews. Task 8's
only finding was a test-assertion filename gap: the legacy database is
`brn.sqlite3`, not `workspace.sqlite3`. Commit `3d98c05` corrected both
no-storage assertions; two retirement and three signal tests passed, and the
whole-branch reviewer confirmed the exact paths.

The whole-branch review of `e24b104..3d98c05` found one Medium conversation
navigation/display bug and one Low replay deadline exit-code bug. GPT-6.1 Sol
medium fixed both in `6a82b78`; exact red/green evidence is above. Opus scoped
re-review of `3d98c05..dccc24a` found both addressed and no new Critical or
Important fix-introduced bugs, approving offline local implementation. Final
review sampled native wiring/scripts rather than auditing every GUI callback,
and relied on existing synthetic tests for Rig hook order; it did not run GUI,
live provider or ONNX checks.

Fresh controller verification at `dccc24a`, before this documentation-only
handoff, used pinned Rust 1.98.1 on macOS with canonical outside-Git synthetic
`TMPDIR` and no real-model environment:

```sh
export TMPDIR=/Users/evokessler/.brn-task5-fixtures
unset BRN_NATIVE_MODEL_DIR
cargo test -p brn -p brn-desktop --bin brn --bin brn-desktop --locked --offline
cargo test -p brn -p brn-workflow --test cli_ask --test cli_signals --test legacy_retirement --locked --offline
cargo build -p brn -p brn-desktop \
  --features brn/native-retrieval,brn-desktop/native-ui,brn-desktop/native-retrieval \
  --locked --offline
cargo fmt --all -- --check
bash scripts/verify-end-to-end.sh --fixtures-only
git diff --check
```

All exited 0: 38 CLI unit +61 desktop unit +6 Ask +3 signal +2 retirement
tests, both native binaries built, and 47 provider-free simple/legacy fixture
assertions passed. These 110 focused test passes overlap previous suites;
they are not an additional unique-case total or a fresh whole-workspace run.
The pre-existing `block v0.1.6` future-compiler warning remains. Worktree was
clean before the documentation-only status/index/roadmap/plan/handoff changes.

Deferred observations are dispositioned, not silently fixed: fail-closed full
startup validation and conversation read cost are known scale limitations;
same-provider status serialization, pre-admission cancellation acknowledgements,
post-login missing-name status, shutdown reattachment/discovery outcomes and
restored-Refresh counts retain their documented policies. The replay deadline
issue is fixed. Simple recovery read/clear and Markdown Save belong to Step 6.
The credential unlink Ruling and its failure cost remain recorded above.

Implementation, offline verification and review are complete. Acceptance and
integration are not: no live accounts, production model download/inference,
graphical usability, original-vault migration, push, merge or release occurred.
Keep the isolated branch/worktree for the next authorized action; Steps 5/6
remain future work and require their own scope.

## PR #14 merge and local-main synchronization

2026-10-03: PR #14 merged at `50f898a0f9637988a8ec4eca261f3c84bb9b7c6d`
(GitHub `mergedAt`: `2026-10-03T04:02:41Z`). The user subsequently requested
merged-branch cleanup, local/remote main synchronization, and publication of
remaining local files. This authorization does not include live accounts,
model downloads, original-data migration or release.

Local main fast-forwarded from `e24b104` to the merge commit. Its older
AI-chat planning changes were already incorporated or superseded by PR #14;
the newer implementation/review evidence was retained rather than overwritten
by stale pre-implementation status. Three additional local documents were
preserved: the product vision, original reset handoff, and paused UI brainstorm.
The latter two are explicitly historical/unapproved; adding them implements no
new product scope. The product vision changes only Markdown hard-break spelling
and a surplus trailing blank line, not its requirements.

Fresh `cargo test --workspace --locked --offline --quiet` passed on the merged
main code with the documentation-only synchronization delta and a unique,
outside-Git disposable fixture directory. Fixtures were cleaned after the
successful run. This is default-feature offline verification, not a new native,
live-provider, ONNX or graphical qualification.

Current status, roadmap and indexes now record the merge while retaining all
native/live/user-acceptance limitations. Finder metadata is excluded via
`.gitignore`; it is not product documentation. Ignored AI-chat execution
artifacts are preserved in the session archive before removing only that
merged task's worktree/branch. Unrelated worktrees, existing trial workspaces
and original data remain untouched. The credential-safety Ruling and its
failure cost remain recorded in this file.
