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
