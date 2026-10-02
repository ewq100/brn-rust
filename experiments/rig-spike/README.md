# Rig spike (throwaway)

Answers whether Rig 0.43.0 works for ChatGPT and Copilot subscriptions in BRN's dependency graph.
Not product code; nothing depends on it. Results: [FINDINGS.md](FINDINGS.md).

Run from this directory (or add `--manifest-path experiments/rig-spike/Cargo.toml`
when running Cargo from the repository root). Replace PROVIDER with `chatgpt`
or `copilot`; each invocation accepts one provider:

    cargo run --locked -- graph
    cargo run --locked -- login PROVIDER CREDS_DIR
    cargo run --locked -- models PROVIDER CREDS_DIR
    cargo run --locked -- chat PROVIDER CREDS_DIR MODEL "PROMPT"
    cargo run --locked -- cancel PROVIDER CREDS_DIR MODEL "PROMPT" MILLISECONDS

CREDS_DIR must resolve outside the repository (including through symlinked
ancestors); it is created with mode 0700 and must be owned by the current user.

Chat streams text as it arrives and fails on the first stream error or a missing
final response. Successful streams print final output and usage. Cancellation
drops the stream and reports whether partial text arrived; this does not establish
provider-side cancellation or billing behavior.
