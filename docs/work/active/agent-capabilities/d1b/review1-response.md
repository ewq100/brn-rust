# D1B round1 correction preparation — BUILD NEED

Source/test preparation only on `work/d1b-20261017`, bound to `/Users/evokessler/.t3/worktrees/brn-rust/t3-9527c407`. Actual HEAD remains evidence commit `e2e2b5a12353366648377eb8651eb3a651b4574b`; material baseline is `a6736855ea5bc6af7406cf433bf60a08217a6d36`. The correction is WIP, not a new verified source pin. B3 owns the sole heavy slot. No Cargo/metadata/build/test/Clippy/mutation/list or slot polling occurred during this correction preparation.

The [full Opus report](opus-review-round1.md) is copied verbatim, SHA256 `aa44196ab8b0c78aee9b19f55e8d1a87737830a4e158f42fa7490ea4d2487e0b`. Its “mutation pending” sentence is superseded by the actual original [mutation receipt](mutation-results.json) and final release, not edited in the report. Original a673 results remain **6 caught / 2 MISSED**, with **534.402553319931s charged / 1265.597446680069s remaining** of the single D1B1800s cap. No correction recheck or retroactive catch is claimed.

## Exact correction ownership

| Path | Prepared hunk |
|---|---|
| `crates/brn-ai/src/threads_summary.rs` | Optional final-prefix admission/degradation; stable account key helpers; existing Compactor's bounded same-model Copilot fallback; relevant Project/guide key projection; full pre-lease cache hit; focused private filesystem fixtures. |
| `crates/brn-ai/src/threads_context_budget.rs` | Existing zero-history budget fallback returns no compaction chunks. No cap, estimate, permission or runtime-hook change. |
| `crates/brn-ai/src/threads.rs` | Same Auth/client construction returns an optional bootstrap digest sidecar for parent summary; missing identity conservatively disables summary/cache reuse; shared existing empty-refusal retry predicate used by parent/helper loop and compactor. Legacy `provider_client` signature preserved for existing callers. |
| `crates/brn-ai/src/threads_summary_tests.rs` | Genuine existing-API before witnesses and focused route/account/coverage/FS-adjacent behavior fixtures. |
| `crates/brn-ai/src/auth.rs` | EXACT additional allocation: one adjacent `pub(crate) github_token_fingerprint -> AiResult<Option<String>>` wrapper calling existing PRIVATE `github_token` and SHA256 over its validated trimmed bytes. No copied reader, new dependency, public API or other Auth behavior change. |

Manifest, lock, app exclusion test, core/app/CLI/UI production, helper/source/history readers and permission predicates remain unchanged. All288material files are fingerprinted in [source preparation](review1-source-preparation.json); exactly these five AI source files differ from a673. The original app-test prefix/all assertions and existing sha2 version/lock edge are retained.

## Responses prepared, not yet behaviorally verified

1. **Optional budget:** zero-history fallback skips compaction. The actually rendered summary+coverage+plain-input block is checked with the existing initial estimate. If it fails, return byte-identical plain input with explicit Activity/result notice and `omitted_budget` trace; preserve actual summary-call usage, report zero delivered summary chunks. Mandatory preflight and whole growing CompletionRequest guard still fail honestly on mandatory overflow. No trim, model change or response cap. Replacement positive dispatch-degradation and separate mandatory-zero-dispatch fixtures are prepared.
2. **Stable account:** ChatGPT keys endpoint+nonempty account ID, token SHA only if absent/blank. Copilot keys endpoint+stable checked GitHub-bootstrap digest from the SAME resolved Auth instance/folder used by existing `client_pinned`. Read digest before/after that one authentication; a missing/unsafe/changing digest produces no reusable unknown/global key: summary/cache reuse is skipped with an explicit gap, while ordinary chat authentication/outcome stays intact. No raw bootstrap/access/exchanged bytes enter cache, logs or trace. Synthetic refresh fixtures use explicit expired sessions, not timing assumptions; same account refresh must cache, actual account change must rebuild. The wrapper has synthetic trim/rotation/control-character checks.
3. **Copilot retry:** factor the existing empty ModelRefused Chat-to-Responses condition once. The same client/model/effort builds both tool-free compactor routes; at most one retry within the existing overall timeout, same budget hooks/cancellation/usage, no chat prompt-cache key or output cap. A partial-text observation prevents the strict collector's erased failure text from causing a false “empty” retry. Context-limit, cancellation, nonrefusal and partial-output outcomes do not switch route. Actual HTTP refusal/fallback and overflow/partial-refusal negative fixtures are prepared.
4. **Relevant key:** project projection is ID/instructions/use_brn_knowledge/archive; unrelated member lists/title/version/current source presence no longer churn it. Exact compactor PREAMBLE, packaged guide bundle version, actual settings/provider/model/effort/account/security transitions and canonical source identities remain bound. Existing visibility/config/move/archive/restore tests stay, with a new unrelated-members/source-presence cache-reuse witness.
5. **Busy lease:** a fully covering validated pre-lease entry returns before acquiring the writer lease. Final fresh snapshot/source/key revalidation remains before rendering. Partial busy coverage still retains valid prior with truthful gap. A held real lease fixture distinguishes those cases.

Private filesystem fixtures cover symlink/FIFO refusal without touching an external synthetic sentinel, oversized cache refusal, failed freshness callback stage cleanup, and positive private-file publication. These are helper-bound filesystem checks, not model-loop or crash-orphan cleanup proof. The original crash-orphan/unmeasured full-record-load/conservative global-change-token limitations remain.

## Before identity and deferred execution

[Before preparation](review1-before-preparation.json) captures a test-only patch on exact a673 production BEFORE any correction production edits. Source `/private/tmp/brn-d1b-20261017-review1-before-source` is an exact archive with genuine synthetic local `.git`, no remote/history/auth copy. Its target `/private/tmp/brn-d1b-20261017-review1-before-target` is separate from the candidate target. All287other material files still match a673.

Original prepared patch `6e16a07b5cbd1ee1f83c24c0ac4fe963d446d8eb512258da7a5656f4e52271c4` and tests are preserved. Static fixture refinement explicitly expires the exchanged session before account-switch authentication (independent of Rig's own bootstrap binding); revised full test-only patch is `ae398fbb72ba18d8a9a93a048b98e182d629565f20a570c455de6315849e27c2`, test file `ad565b1e8307d9f5fcc4cbf8b0574d6d562c3123840e3863fa33fed5dff72fd0`. No before fixture is compiled/executed yet; expected behavioral failures are not red proof. Compiler/fixture errors must be retained and corrected as test-only errors before claiming a behavioral failure.

M02 now tampers ONLY a well-formed nonempty cached content_digest while leaving key/first/last/reference stamps valid. M07 now uses a unique derived prior requirement absent from ALL canonical Message text, and asserts its delivery after timeout and busy-cache degradation. Their original MISSED verdicts remain unchanged; new tests count as catches only if actually rechecked later.

## BUILD NEED and later commands

Static-only checks pass: direct pinned rustfmt `--check` on all five owned files and the before test file, `git diff --check`, and byte-unchanged manifest/lock/app-test diff checks. Exact commands/log hashes and the WIP source fingerprints are in [source preparation](review1-source-preparation.json). Candidate material diff SHA256 is `b3f54d2acbc2c798d033396e0ad916de916414b6746ae458db602ac00e824282`. This is e2e2/a673+WIP provenance, not a future commit association or compiled proof.

After a NEW explicit heavy grant: precise process check, fresh synthetic TMPDIR, Rust1.98.1, CARGO_INCREMENTAL=0, locked/offline. Candidate target `/private/tmp/brn-d1b-20261017-target` remains bound to this exact literal candidate source root only. Before/mutant source roots must never share it. Run actual prepared before cases first, then corrected focused cases and normal gates serially; preserve raw failures/artifact+dep-info/source associations before a new immutable source/test pin and fresh principal-owned Opus2.

Planned commands (NOT EXECUTED in this stage):

```sh
cargo +1.98.1 test -p brn-ai --features scripted-runs --locked --offline --lib threads::summary_tests -- --nocapture
cargo +1.98.1 test -p brn-ai --features scripted-runs --locked --offline --lib threads::summary::filesystem_tests -- --nocapture
cargo +1.98.1 test -p brn-ai --features scripted-runs --lib --tests --locked --offline
cargo +1.98.1 test -p brn-threads-core --test note_read --test message_search_scope --locked --offline
cargo +1.98.1 test -p brn --locked --offline
cargo +1.98.1 clippy -p brn-ai -p brn-threads-core -p brn --all-targets --features brn-ai/scripted-runs --locked --offline -- -D warnings
cargo +1.98.1 fmt --all -- --check
```

The first command must be run separately in the fixed before source/target and candidate source/target. Additional broad gates follow the new grant's proportional scope, not an automatic repeat while source-only. Any later mutation supplement uses only the original1265.597446680069s remainder, charges all setup/baseline/controller/cleanup, and keeps old per-ID verdicts unchanged. No mutation command/setup/list or new spend now.

Prior AI407/0/1, core15/0/0, CLI161/0/0, app5/0/0, workspace1129/1PDF-Unavailable/6 and all old lint/build results belong a673; none qualifies this WIP. L still blocks live semantic quality, structured JSON/timeout behavior, native/hosted/owner/full program qualification remain separate. No source acceptance, D2 work or publication. Preparation stops here at BUILD NEED while B3 retains heavy ownership.
