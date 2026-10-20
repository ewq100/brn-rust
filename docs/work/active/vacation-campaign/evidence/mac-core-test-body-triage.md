# Mac Core test-body triage

## Scope and identity

Inspected the bounded task in `docs/work/overview.md` / `campaign.json`, plus the required prior `ci-cost-triage.md`, `residual-mac-ci-audit.md` and `ci-pr190-mac-cache-observation.json`. They measured completed job/step elapsed time and cache context, not this run's body timing. This audit uses only the completed preceding-main run in `pr193-premerge.json`: run `39502704935`, source `2658f8883b47f224b02ed428055efaaba1bb47a4`; selected job `118904878450`, **Core and CLI (macos-15)**, success, 21:46:04–21:57:45Z (701 s / 11m41s). The overall matrix receipt reports failure because other jobs failed; this Mac job itself succeeded. No current PR193 resulting-main run was queried.

Two GitHub read-only job-log calls were made (within the two-call cap). The full decoded log could not be saved from the bounded tool response; I preserved a clearly labeled partial tail excerpt at `/tmp/brn-mac-core-test-body-triage/mac-core-118904878450.log.excerpt.txt`, not a complete raw log. A compact measured-data file is at `/tmp/brn-mac-core-test-body-triage/summary.json`. Consequently, exact cargo summaries below are limited to the visible excerpt.

## Measured elapsed time

Times are job-step wall elapsed values from the local run receipt, not CPU time or test-body sums.

| Step | Elapsed |
|---|---:|
| Rust setup | 0m45s |
| Prepare both locked helper graphs | 1m43s |
| Build default workspace | 0m12s |
| Lint default workspace | 0m13s |
| Test default workspace | **4m13s** |
| Both helper normal-API/restriction test and lint | **2m21s** |
| Scripted document CLI route (test + lint) | 0m24s |
| Provider capability fixtures offline | 1m31s |
| Provider capability lint | 0m10s |
| Offline end-to-end fixtures | 0m01s |

The 4m13s default-workspace `cargo test --workspace --no-fail-fast --locked --offline` step is the largest measured step; helper tests plus lint take 2m21s. These are not body-only measurements: cargo compiles test targets in the same steps. The separate default workspace build was 12s and Clippy 13s. Helper preparation is a separate 1m43s step (fetch both locked graphs, generate adapted workspace, build both helpers); the later helper step sets a separate target directory for stock/adapted and runs each helper's all-target tests and Clippy. It is therefore not evidence that helper builds were repeated inside the test step, though compilation of other test targets can occur there.

The provider fixture step is the only visible test-body timing in the fetched excerpt. Workflow source (`.github/workflows/ci.yml:117-122`) runs `cargo test -p brn-ai --features capability-spike --all-targets --locked --offline` followed by a separate Clippy step. The excerpt reports 492 passed, 0 failed, 1 ignored, harness `finished in 13.88s`, followed by 3 example tests, `0.00s`. The containing step took 91s. The difference shows at least about 77s outside those reported harness durations (compilation/launch/step overhead and between binaries); the available excerpt does not safely split those components further. It does not identify a long-running test body.

## Test mechanisms and inference limits

The helper step at `.github/workflows/ci.yml:80-111` prepares stock and adapted helpers, exports their paths, and tests both independently with `cargo test --all-targets --locked --offline --jobs 1` plus Clippy. The step's “normal API and restrictions” contract and isolated per-mode targets remain necessary evidence; no helper coverage deletion is supported by elapsed step time.

Provider capability fixtures are explicitly offline; the job also clears `BRN_NATIVE_MODEL_DIR` (`ci.yml:17-22`) and uses locked/offline Cargo. The scripted CLI route uses `scripted-document-tests` and is expressly no-live-provider (`ci.yml:112-116`). End-to-end fixtures run `verify-end-to-end.sh --fixtures-only` under a fresh temporary parent (`:133-141`). These mechanisms support retaining their distinct guarantees; none presents an evident live-provider or model-download cost to remove.

The captured cargo output has test completion lines but not per-test start/finish intervals. Test harnesses run tests concurrently unless told otherwise; completion timestamp gaps cannot establish any particular test duration. Thus no named test over 60 seconds is evidenced. The 492-test summary's 13.88s is a harness duration, not a sum of individual body times. The 4m13s workspace step summary and 2m21s helper step summary are likewise insufficient to attribute cost to a crate or test body. No named-test timing was available in the earlier reports either.

## Disposition

No CI change, gate reduction, parallelization or test retirement is justified. The evidence does identify the default-workspace test step as the largest elapsed segment and a clear distinction in the capability fixture step between 13.88s reported harness time and much longer cargo-step wall time, but it does not establish which workspace test bodies or crates dominate, nor a safe cost-reduction boundary. Preserve the current required gates. Any later cost proposal would first need a retained complete log or equivalent per-target compilation/body evidence on a comparable source/cache/toolchain; this report does not select that work.

Source lines cited are from `.github/workflows/ci.yml` at the current inspected checkout. No repository files, tests, builds, caches, CI settings or accounts were changed. Both GitHub calls were read-only log retrievals for the already-completed specified job; no rerun or current-run query occurred.

## Lead follow-up: complete log retained after bounded response limit

The helper's two-call bounded result above remains unchanged historical evidence. To answer the selected compilation/body question, the lead made one distinct successful read with `gh run view 39502704935 --job 118904878450 --log`, redirecting the complete decoded output to a file instead of a bounded response. The 512,163-byte full log, original excerpt and derived summaries are retained in `mac-core-test-body-triage-raw.tar.gz`. No workflow was rerun or changed.

Actual Cargo reports identify the cost:

| Step | Cargo build/lint reports | Reported sequential harness elapsed | Step wall time |
|---|---|---|---|
| Default workspace test | test-profile build171s | 79.56s | 253s |
| Both helper test/lint graphs | test builds51.90s +53.43s; lint16.12s +17.70s | 0.97s | 141s |
| Scripted CLI route | test build17.39s; lint5.91s | 0.47s | 24s |
| Provider capability fixtures | test-profile build76s | 13.88s | 91s |

Harness values are rounded reported elapsed time of sequential test binaries, not per-test CPU sums. Individual tests inside a binary may run concurrently. Their sum does not include all launch/step overhead. The longest workspace harness is `brn/tests/agent_bounds.rs`,12 checks in22.40s; default `brn-ai`523/0/2 takes15.79s and `brn/tests/edit_tokens.rs`14checks9.59s. Deadline/lock checks deliberately exercise process boundaries; these measurements do not justify removing them. No reported harness exceeds60s and no over60-second named-test warning appears in this full log.

The helper step's repeated dependency/test/lint builds dominate its actual test execution; workspace test-profile compilation also dominates that step. The existing Mac caches cover dependencies and keep required source rebuilding/checking; full cache restoration alone cannot imply zero compilation. This closes the attribution gap without establishing a safe faster cache/build profile. No gate reduction, test deletion, configuration change or new CI experiment selected. Revisit only for a bounded comparable-source proposal with measured build time, archive size, source invalidation and preservation of existing checks.
