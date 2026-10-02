# Rig spike (throwaway)

Answers whether Rig 0.43.0 works for ChatGPT and Copilot subscriptions in BRN's dependency graph.
Not product code; nothing depends on it. Results: [FINDINGS.md](FINDINGS.md).

    cargo run --locked -- graph
    cargo run --locked -- login chatgpt|copilot CREDS_DIR
    cargo run --locked -- models chatgpt|copilot CREDS_DIR
    cargo run --locked -- chat chatgpt|copilot CREDS_DIR MODEL "PROMPT"
    cargo run --locked -- cancel chatgpt|copilot CREDS_DIR MODEL "PROMPT" MILLISECONDS

CREDS_DIR must be outside the repository; it is created with mode 0700.
