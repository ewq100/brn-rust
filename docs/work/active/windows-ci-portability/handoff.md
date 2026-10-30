# Windows CI portability candidate

30 October 2026. Local candidate based on main
`89e6221afc8d6b036b7b5a1bcfe4b7cc23871583`, branch
`fix/windows-ci-portability`. No push, hosted run, PR mutation, merge or release
was performed. Windows CI is **not yet qualified**.

## Diagnosed failures

Downloaded job logs, rather than relying on the previous error count:

| Run / job | Result and cause |
| --- | --- |
| [41027043703 / 123967509410](https://github.com/ewq100/brn-rust/actions/runs/41027043703/job/123967509410) | PR244 Core and CLI: 33 errors from Unix imports, ownership/modes, no-follow flags and descriptor identity checks in `auth.rs` and `codex_auth.rs`. Two intake cfg warnings and two subscription-search adapter cfg warnings. |
| [40990750144 / 123852285876](https://github.com/ewq100/brn-rust/actions/runs/40990750144/job/123852285876) | Main Core and CLI: same 33 authentication errors; two intake warnings. |
| [40990750144 / 123852285939](https://github.com/ewq100/brn-rust/actions/runs/40990750144/job/123852285939) | Main Native UI: same authentication errors and intake warnings. |
| [40990750144 / 123852285849](https://github.com/ewq100/brn-rust/actions/runs/40990750144/job/123852285849) | Main native retrieval: 14 errors from Unix-only staging, link counts and inode/device APIs in `native/download.rs`. |

PR244's logged checkout was merge commit `0a78bf9acc1c7fdfabdf6244f7127018c0f33213`,
combining its head `2764b52f73f2269d3ba770516f5314b2056c8522` with the main
baseline above. PR244 did not schedule Windows native UI/retrieval on that PR run;
main/manual CI schedules all three Windows jobs.

## Changes and explicit capability limits

- Retain the Unix credential implementation and its ownership, private modes,
  no-follow, hard-link and opened-file identity checks. Non-Unix authentication
  refuses before reading/writing credential files or sending provider requests,
  with the fixed message “Secure credential storage is unavailable on this
  platform.” A qualified Windows ACL/handle adapter remains future work; this
  candidate does not claim Windows account connection support.
- Use the installer's existing `ExclusiveInstallUnavailable` error before HTTP
  client construction or filesystem effects outside Linux/macOS. Those platforms
  already lacked exclusive publication support. Model loading and pure asset
  verification remain separate from installation.
- Non-Unix intake scratch creation returns its existing `Unavailable` outcome
  before root creation or expired-scratch cleanup. Unix scratch behavior is kept.
- Match canonical backup/restore's existing Unix directory-flush convention for
  derived summary publication. File flush and atomic replacement remain intact;
  Windows standard file handles cannot open a directory for this Unix flush.
- Keep synthetic provider transport tests portable through an in-memory fixture,
  tested on Linux across ChatGPT and both Copilot routes. The protected-storage
  auth tests run only on Unix. Additionally, 31 runner tests that actually use
  protected-file authentication are Unix-only (cache 7, research 6, budget 2,
  summary 16). Shared mock-model/runtime tests and four portable summary tests
  remain enabled. New non-Unix tests assert explicit refusal, untouched files,
  no HTTP/progress callbacks and cancellation precedence. These non-Unix tests
  still require execution on Windows.

No workflow checks were deleted or weakened. Linux/macOS behavior and Windows
UI feature parity are separate gates.

## Checks actually performed

Pinned compiler: Rust 1.98.1. Only `x86_64-unknown-linux-gnu` is installed.

| Command | Outcome |
| --- | --- |
| `cargo fetch --locked --target x86_64-unknown-linux-gnu` | Passed; refreshed the older saved dependency cache. |
| `cargo build --workspace --locked --offline` | Passed. |
| `cargo clippy -p brn-ai -p brn-threads-intake --all-targets --locked --offline -- -D warnings` | Passed on final Rust source. |
| `cargo test -p brn-ai --lib --locked --offline` | 528 passed, 1 ignored. |
| `python3 /workspace/brn-environment/run-with-reaping.py cargo test -p brn-threads-intake --lib --locked --offline` | 4 passed, 1 ignored. |
| `cargo fmt --all -- --check`; `git diff --check` | Passed. |
| `cargo test -p brn-retrieval --features native --lib --test model_download --locked --offline` | Blocked before retrieval compilation by `ort-sys 2.0.0-rc.13` native runtime binary download: `UnknownIssuer`. System-CA curl to the same pinned URL returned HTTP403. TLS checks were retained; no alternate source or model assets fetched. |

Tests sourced `/workspace/brn-environment/activate.sh`, used a new explicit
`TMPDIR=/var/tmp/brn-windows-offline-tests`, and required filesystem escalation.
The initial `/tmp` run had 372 passed / 156 failed / 1 ignored because the sandbox
injects `.git` ancestors into writable roots: credential validation correctly
refused those paths. Narrow additional filesystem grants also inject those
markers. The final approved offline run used a fresh unmarked fixture parent;
no validation was relaxed. The initial intake run had 3 passed / 1 failed /
1 ignored because this container's PID1 leaves killed descendants unreaped;
the existing environment subreaper wrapper resolved that failure without source
or assertion changes. One test compilation was interrupted to correct fixture
environment selection.

Local full logs: `/tmp/brn-windows-ai-test-unsandboxed.log`,
`/tmp/brn-windows-clippy-final.log`, `/tmp/brn-windows-workspace-build.log`,
`/tmp/brn-windows-intake-test-reaped.log`,
`/tmp/brn-windows-retrieval-test.log`. Earlier failures remain in the sibling
`brn-windows-ai-test*.log` and `brn-windows-intake-test.log` files.

Independent review identified and resolved: an overbroad provider fixture gate,
further Unix-only authentication fixtures, a newly unused scratch-cleanup helper,
an overbroad summary-test gate, and derived-summary directory flushing. Final
review found no actionable issue in the in-memory fixture or summary correction.
The lead independently reviewed the retrieval and test-gating edits.

## Handoff and remaining verification

1. A real Windows executor/toolchain is required. No Windows target, linker or
   runtime was installed, and no Windows compile/test pass is claimed. Fix the
   cloud native-runtime download trust/access issue before local native retrieval
   regression qualification; do not bypass TLS or substitute older binaries.
2. After the parent confirms restored Actions allowance and publication authority,
   push the reviewed local branch and use `gh workflow run ci.yml --ref
   fix/windows-ci-portability`. Do not dispatch while the budget hold applies.
   Record the exact candidate SHA and require all three Windows jobs to finish.
3. Windows core must run `cargo build --workspace --locked --offline`,
   `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`,
   `python3 scripts/ci-workspace-tests.py`, and CLI help. Native UI must run its
   existing build/lint/tests and two fresh headless startups. Retrieval must run
   `cargo test -p brn-retrieval --features native --lib --test model_download
   --locked --offline`, the native application tests, native CLI build and combined
   desktop build/lint/tests exactly as declared in `.github/workflows/ci.yml`.
   Newly reachable failures must be diagnosed; static review is not a green run.
4. No changed file overlaps PR244's current changed-file list, so no direct text
   conflict is expected. Its separate `subscription_search.rs` two Windows cfg
   warnings remain outside this main-based patch and can fail main/manual strict
   lint after integration; its owner must fix/qualify them on the combined source.
   No Mac unpushed branch or commit was accessed or changed.

Windows secure credentials and exclusive model installation remain clearly
unsupported rather than silently using unsafe portable fallbacks. Restore those
features only with qualified platform adapters and real Windows security tests.
