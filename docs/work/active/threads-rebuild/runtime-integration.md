# Threads runtime integration pass

Starting commit: `0fcb8f737952189724a7156dde2c483719e9a6bf`. PR #114 remains draft. The Mac verification checkpoint is published; its long-note reader, worker ownership, intake coverage and native repairs are retained.

Implementation follows the accepted architecture and the owner's ordered assignment:

1. Reproduce the original published Rig 0.43/0.44 qualification and extend its bounded synthetic decoder witness before changing expectations. Keep exact locked commands and sanitized results.
2. Use published Rig 0.44 in production. Stream text is provisional; successful `PromptResponse.output()` is authoritative and replaces preview. Bound both surfaces, retain honest interruption/failure, explicitly fail invalid tools, malformed arguments and unknown termination, and use typed reasoning where supported.
3. Replace English command parsing with trusted host request modes carrying the selected note/version or one new decision, owner message identity and invocation lifetime. All writes remain under the shared checked service, run/settings fences and batch validation.
4. Expose bounded accepted revisions and fresh immutable stale-candidate replacement through service authority. Preserve current-version long-note continuation. Connect registered tools to payload-free activity and remove duplicated skill implementation/semantic Rust prose.
5. Run retained workspace, AI/core/app, provider fixtures, format, Clippy and dependency checks. Run bounded selected Codex / gpt-6.1-sol synthetic journeys and affected native interactions using a rebuilt local candidate. Preserve unaffected prior evidence; owner acceptance is separately pending.
6. Commit meaningful checkpoints, reconcile the remote, publish a normal fast-forward to rebuild/threads and record exact candidate/evidence identities. No main merge or release.

The decoder comparison is [repeatable evidence](rig-decoder-comparison.md). Test results and the final disposition belong in [evidence](evidence.md); no fixture adaptation failure counts as a Rig regression.
