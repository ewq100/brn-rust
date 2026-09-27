# Chunk 02: provider lifecycle trial evidence

Date: 2026-09-27. Scope: disposable harness only; no desktop UI, release, merge or account-setting changes.

## Verified starting point

The handoff named `c8f01c8`, but actual local and remote `trial/chunk-01-codex-app-server` pointed to `d3b2410d91a7e7a6553ad8d88c30c04875ddd7d4`. The later commit already demonstrated resume across two App Server processes within one harness process. The original checkout was clean and remains untouched by source changes. No applicable `AGENTS.md` was found in the repository or inspected ancestor chain.

Work proceeds on `trial/chunk-02-provider-lifecycle` in `/private/tmp/brn-chunk-02`, based on `d3b2410`. The production `brn` crate stays unchanged. The plan is [recorded here](../../docs/chunk-02-plan.md).

## Environment

- macOS Darwin arm64; Rust and Cargo 1.98.1.
- `/Applications/ChatGPT.app/Contents/Resources/codex`: Mach-O 64-bit arm64, `codex-cli 0.155.0-alpha.16.4`.
- `codex app-server generate-json-schema --out /private/tmp/brn-chunk02-schema --experimental` succeeded; generated local schema was consulted for this installed version and is not committed.
- Codex owns existing managed ChatGPT sign-in and server conversation storage. No credential file is read or copied by the harness or verification steps.

## Baseline commands and results

Run from repository root with `PATH=/opt/homebrew/opt/rustup/bin:$PATH`:

```sh
cargo +1.98.1 build --workspace --locked --offline
cargo +1.98.1 fmt --all -- --check
cargo +1.98.1 clippy --workspace --all-targets --locked --offline -- -D warnings
cargo +1.98.1 test --workspace --locked --offline
cargo +1.98.1 test --manifest-path experiments/codex-app-server/Cargo.toml --locked --offline
```

All passed at the baseline: starter 0 unit tests, harness 4 unit tests. Starter subprocess checks passed: `--help`/`--version` exit 0; absent/unknown/extra arguments exit 1.

## Authentication and packaging evidence boundaries

Official [App Server documentation](https://learn.chatgpt.com/docs/app-server) and the installed `GetAccountParams.json` schema describe `account/read` with `refreshToken: true` as the managed proactive refresh path. A successful call demonstrates that supported operation, not natural expiry or revocation recovery. Offline error simulations must be labeled as simulations. The trial never manufactures a revoked credential, logs out, or falls back to API-key billing.

See [PACKAGING.md](PACKAGING.md) for official distribution/license sources, the installed-executable decision, and packaging/signing limitations. No binary is bundled by this change.

## Reproducible checks

Run the complete credential-free suite from repository root:

```sh
PATH=/opt/homebrew/opt/rustup/bin:$PATH bash scripts/verify-trial.sh
```

For explicitly initiated live checks, use synthetic data only, the same existing managed ChatGPT sign-in and the same Codex home/store. Choose a **new** state path outside the checkout; start refuses an existing state file. Each line below launches and exits a separate harness process:

```sh
export CODEX_BIN=/Applications/ChatGPT.app/Contents/Resources/codex
trial=./experiments/codex-app-server/target/debug/brn-app-server-trial
"$trial" sidecar-check
"$trial" auth-check
"$trial" auth-refresh
"$trial" persist-start /private/tmp/brn-chunk02-manual-state.json
"$trial" persist-resume /private/tmp/brn-chunk02-manual-state.json
```

Do not delete or replace a saved state merely because a request failed. A pending record means a turn may have been accepted; inspect the conversation using Codex before deciding how to proceed. Do not edit IDs to point at unrelated user conversations. Missing/deleted server history must fail clearly without creating a replacement.

`auth-refresh` requests one supported managed refresh; it does not print tokens or run a logout/login flow. For actual expired/revoked sign-in, follow the harness's recovery instructions using the supported Codex login UI or CLI, then rerun `auth-check`. That manual authentication interaction is required only if the real sign-in fails; do not revoke working credentials for this test.

Run `"$trial" live` for the prior stream/tool/continuation/interruption checks and `"$trial" resume-live` for the earlier two-sidecar-process probe.

## Completion evidence

The following live checks completed on the installed arm64 executable with the existing managed ChatGPT sign-in. The command sandbox prevented sidecar initialization (`app-server exited before response`); authorized unrestricted execution succeeded.

| Command / check | Observed result |
| --- | --- |
| `sidecar-check` | Initialization and shutdown completed, exit 0. |
| `auth-check` | Managed ChatGPT account confirmed, exit 0; account details omitted. |
| `auth-refresh` | Supported proactive refresh request completed and managed account returned, exit 0. No token values were inspected. |
| `persist-start /private/tmp/brn-chunk02-verified-state-20260927.json` | Created a stored thread, completed synthetic marker turn, wrote state, exited 0. |
| `persist-resume` with that same file in a separate command/process | Resumed matching stored thread and recalled marker without supplying it in the prompt, exit 0. |
| State-file inspection | Schema v1, completed outcome, mode `0600`; file remains outside Git. Thread ID not copied into this report. |
| `live` | Streamed 9-character `STREAM_OK`; exactly one fixture tool call; continuation recalled `17`; server emitted interrupted terminal status. |
| `resume-live` | Earlier two-App-Server-process resume regression passed. |

`PATH=/opt/homebrew/opt/rustup/bin:$PATH bash scripts/verify-trial.sh` passed after the first implementation: starter build/fmt/Clippy/tests, harness equivalents (14 tests), five starter and four harness smoke cases. Final verification after review fixes is recorded below.

Astra's initial code review identified two important issues: auth-check transport failures were incorrectly classified as login failures, and blocking pipe writes could escape operation deadlines. Sol fixed both with targeted regressions; Astra's scoped re-review approved both fixes and found no new breakage. No critical finding was reported.

## Limits and recovery

- Refresh is observed through the supported App Server response, not direct observation of token rotation or identity-provider traffic. Natural expiry, actual revocation, interactive re-login and post-revocation recovery remain unverified; their error handling is tested with fake protocol responses only.
- Persistence is verified across normal harness/App Server exits on the same Mac and Codex store. Reboot, power-loss durability, deleted server history, changed Codex home, and migrating between accounts/stores remain outside the live proof. Keep the Codex store available; the local state file cannot reconstruct server history.
- The trial is for sequential invocations, not concurrent writers to one state file. It does not implement multi-process locking. A pending or uncertain record refuses model submission. Even a startup/auth failure may leave a conservative pending record with no thread ID; inspect it before explicitly choosing a new state path. No automatic replay or file replacement occurs.
- In-flight termination and transport failures can leave the server outcome unknown. Local process shutdown is not proof that all upstream computation stopped.
- Packaging, signing/notarization, clean-machine installation, app sandbox/Keychain integration and process-tree behavior on a forcible parent crash remain unverified; see PACKAGING.md.
- Trial state version 1 has no migration path. Future versions and corrupt files fail closed. This is not production authoritative storage.

## Final verification and review

- Final offline command: `PATH=/opt/homebrew/opt/rustup/bin:$PATH bash scripts/verify-trial.sh` — exit 0. Both targets built, formatting passed, Clippy passed with warnings denied, starter tests passed (0 tests), harness tests passed (**16 tests**), five starter and four harness smoke checks passed.
- Added tests exercise private versioned state and failure cases, auth mode/rejection/redaction, terminal unauthorized recovery, request-ID namespaces, event correlation, notification deadlines, missing/early-exit sidecars, graceful and forced shutdown, blocked writes with timeout/reaping, and auth-read transport diagnostics.
- Post-fix live regressions: `live`, `resume-live`, and a separate `persist-resume` invocation all exited 0. This exercised the final writer/deadline code against the installed sidecar. The proactive refresh was observed once before these transport fixes; it was not repeated unnecessarily.
- Independent Astra task review required two corrections; both were fixed and approved by scoped re-review. Final whole-branch Astra review approved all eight changed/authored files with no Critical, Important, or Minor findings. It also manually checked apparent secret material; only synthetic test strings were found.
- Final `git diff --check` and a secret-pattern scan of the eight changed/new text files passed. No credential cache, runtime state, binary, target directory or generated schema is staged for delivery.
- Original Chunk 01 checkout remains clean at `d3b2410`; the new work is isolated on `trial/chunk-02-provider-lifecycle`.

Delivery uses an ordinary commit and `git push -u origin trial/chunk-02-provider-lifecycle`, followed by `git ls-remote origin refs/heads/trial/chunk-02-provider-lifecycle` compared with `git rev-parse HEAD`. Nothing is merged or released.

Delivery checkpoint: `7c49770407a84b4bb62eb5047fcc4a2f48964323` was committed and pushed successfully. `git ls-remote origin refs/heads/trial/chunk-02-provider-lifecycle` returned that exact SHA, and `git status --short --branch` was clean. This documentation-only follow-up records that observation; the final response supplies the final remote-verified branch HEAD.
