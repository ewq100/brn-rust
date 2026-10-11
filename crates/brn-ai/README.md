# BRN AI runtime

The production Threads runtime uses published Rig 0.44.0. No vendored Rig patch is required. Provider routing retains Codex/ChatGPT subscription Responses, OpenAI Responses, and Copilot Chat completions.

`chat.rs` owns the shared collector. Text deltas are bounded provisional UI. A successful Rig final response supplies the authoritative answer through its structured output API; it replaces the preview and is persisted once by the Threads host. EOF, malformed responses, unknown termination and interruption never invent successful completion. Partial preview remains available as interrupted progress. Both preview and final answer are limited to 128 KiB; strict full-document extraction has its separate bounded contract.

`threads.rs` registers the production tool catalog and enforces complete-batch validation before dispatch. Unknown tools and malformed arguments fail closed with zero retries. Tool rounds and output are bounded explicitly. Conversation Message writes are host-only. Selected-note authority binds an actual owner message, exact note/version, content-edit class and current run/settings fence; decision authority binds one newly created decision. Source text and retries cannot grant authority.

Nine registered tools provide current-record reading, version-aware note passages, transient source reading, packaged skills, prepared/application changes, retained progress, bounded accepted revisions and service-owned immutable candidate refresh. The same registration generates safe activity labels. Operating guidance lives in `agent/BRN.md` and packaged `agent/skills/*/SKILL.md`.

Responses routes use Rig's typed reasoning options. Copilot Chat retains its supported raw `reasoning_effort` field because Rig 0.44 does not expose verified typed generation options for that dialect.

Verification:

```sh
cargo test --locked -p brn-ai
cargo test --locked -p brn-ai --features capability-spike
cargo clippy --locked -p brn-ai --all-targets --features capability-spike -- -D warnings
```

The optional `capability-spike` feature retains bounded qualification fixtures/probes; it is not the production tool runtime. The retired standalone public 0.43/0.44 decoder comparison is preserved with [its historical commands and sanitized results](../../docs/archive/threads-rebuild/rig-decoder-comparison.md). Current integration evidence is in [the rebuild evidence](../../docs/archive/threads-rebuild/evidence.md).
