# Quote selector provider HTTP witness handoff

Commit: `28fd581ea8451ecd95d899248132b62c8f890d03` (`test: exercise note quote selector over provider routes`)

Bounded files:
- `crates/brn-ai/src/provider_formats_tests.rs`: test-only crate visibility for the existing scripted client, SSE response builders, schema extractor, and request body/script assertions.
- `crates/brn-ai/src/threads_note_edit_tests.rs`: an async route test that creates a fresh Threads fixture for each of ChatGPT Responses, Copilot Chat, and Copilot Responses; constructs the actual pinned Rig 0.44 completion model; calls production `run_model` and its production tool registration/dispatch; sends explicit null offsets with `Mon` -> `Tues`; and checks required nullable closed schema, call-id-correlated tool result, operation-to-candidate identity, exact candidate Note contents/metadata, unchanged canonical record, and consumed scripts.

No production API or provider harness was added. No builds, tests, network/account/native/credential operations were run. Formatting and whitespace validation only:

```sh
source /tmp/brn-launcher-focus-qualification/environment.sh
cargo fmt --manifest-path crates/brn-ai/Cargo.toml -- crates/brn-ai/src/provider_formats_tests.rs crates/brn-ai/src/threads_note_edit_tests.rs
git diff --check
```

`git diff --check` passed before commit. The lead should run the focused test after the frozen handoff, followed by independent candidate review.
