# Experimental Codex App Server provider trial

This is a disposable Rust protocol probe for BRN Rust chunk 01. It is not part of the production `brn` binary. It sends only synthetic prompts and one hard-coded, read-only fixture response. No user documents or credentials are read by this program.

## Mac and tool versions (2026-09-27)

- macOS (`Darwin`) on `arm64` Apple Silicon.
- Rust `rustc 1.98.1 (48a229cea 2026-09-01)`; Cargo `1.98.1`.
- Codex CLI `0.155.0-alpha.16.4`, bundled at `/Applications/ChatGPT.app/Contents/Resources/codex`.
- Codex App Server uses newline-delimited JSON over stdio. The installed CLI generates a JSON schema using `codex app-server generate-json-schema --out DIR --experimental`.
- `serde_json 1.0.151` is the only direct Rust dependency.

## Commands

From `experiments/codex-app-server`:

```sh
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo +1.98.1 test --offline
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo +1.98.1 fmt --check
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo +1.98.1 clippy --offline --all-targets -- -D warnings
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo +1.98.1 run --offline -- live
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo +1.98.1 run --offline -- resume-live
```

The live command requires an existing **managed ChatGPT** Codex login. It calls `account/read` with `refreshToken: false` and refuses to start a model turn unless `account.type` is `chatgpt`. It never asks for, logs, or sends an API key. On this Mac, running the live command inside Codex's restricted command sandbox could not start App Server; an approved unrestricted run succeeded. A packaged BRN app will need the normal process and filesystem access required by the Codex sidecar.

## Results

Four credential-free unit tests passed: (1) deltas concatenate but incomplete streams fail, (2) a locally cancelled run stays unconfirmed until the server reports `interrupted`, (3) the fixed `alpha` fixture is returned while unknown keys fail, and (4) a resume response must identify the requested thread. These tests use synthetic protocol messages; they do not prove a real model or server.

One short managed-ChatGPT run then reported:

| Check | Observation |
| --- | --- |
| Streamed text | `STREAM_OK` reconstructed from App Server message deltas (9 characters). |
| Read-only tool | One `fixture_lookup({"key":"alpha"})` request was answered with `alpha: fixture value 17`; response mentioned `17`. |
| Continuation | A later turn on the same thread recalled `17`. |
| Interruption | `turn/interrupt` returned successfully and the server emitted `turn/completed` with `status: interrupted`. |
| Resume after restart | `resume-live` created a stored synthetic thread, completed a marker turn, stopped App Server, started a second App Server process, called `thread/resume` with the original ID, and received the marker in a new turn. |

The interruption check is server-confirmed interruption of the Codex turn, not proof that all upstream computation stopped at a particular instant. The probe has no retry loop. Token refresh was not tested: `account/read` explicitly used `refreshToken: false`, and the runs were too short to observe expiry. ChatGPT account sign-in was already present, so no new ChatGPT login flow was tested. Persisted resume was checked across two App Server processes, not a macOS reboot. It did not test remote network cancellation or macOS app packaging.

## Recommendation

Use official Codex App Server as the leading **subscription** provider for the next BRN integration slice. It already manages ChatGPT login and token refresh, provides streaming, persisted thread continuation, and a server interruption API. Keep the BRN retrieval interface and tool authorization in Rust; expose narrowly scoped read-only fixture or retrieval tools to App Server. Do not run a second Rig agent loop around a Codex agent loop.

Direct Rig integration remains a useful alternative if BRN later needs model-provider portability or tighter control of agent orchestration. The prior Rig trial showed six offline successes, but no account sign-in or live subscription request. Its ChatGPT credential snapshot and JSON cache imply additional refresh and secure-storage work. This trial does not establish that direct Rig works with this Mac's subscription.

App Server is a native **sidecar dependency**: a standalone BRN distribution must either bundle a compatible Codex executable or locate a supported installed copy, start and supervise its process, coordinate managed login, and test signed/notarized packaging. The dynamic tool API used here is experimental and requires `capabilities.experimentalApi = true`; pin and revalidate the protocol against the bundled Codex version. There is no Node, Electron, or Python runtime dependency in this Rust harness.

Official references: [App Server](https://developers.openai.com/codex/app-server), [Codex authentication](https://developers.openai.com/codex/auth).

## Checkout and native starter build

The private `ewq100/brn-rust` repository was cloned after a normal GitHub device sign-in as its owner. Before adding this trial, the clean `main` checkout passed `cargo build --workspace --locked`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked` on macOS Apple Silicon with Rust 1.98.1. Five CLI smoke checks passed: `--help` and `--version` succeeded; missing, unknown, and extra arguments failed. The starter has no desktop UI yet, so this verifies native CLI compilation rather than a desktop app bundle.
