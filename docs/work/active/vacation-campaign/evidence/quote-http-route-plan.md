# Minimal actual provider-route witness for `prepare_note_edit` null/null

## Finding

Feasible without a new production harness or public API. The strongest small witness belongs in `crates/brn-ai/src/threads_note_edit_tests.rs` (included as a child module of `threads` at `threads.rs:68`), where the existing `Scope`, fixture, and production `run_model`/tool registration are accessible. Reuse the existing loopback provider client and scripted HTTP response machinery from crate-root `provider_formats_tests.rs`, exposing only test-only helpers at `pub(crate)` visibility if needed (`client`, `success`, `tool_sse`, and body/request capture accessors). `ROUTES` is currently declared inside several test submodules, so either promote one shared `pub(crate)` test-only matrix or spell the same three tuples in the focused test. This changes test visibility, not the shipped surface.

## Recipe

1. In a new async test in `threads_note_edit_tests.rs`, loop over the existing three `ROUTES`: ChatGPT Responses, Copilot Chat, Copilot Responses. Build the normal synthetic Threads fixture and Scope with the current `tests::fixture()` / `begin()` helpers.
2. Have the loopback HTTP script return one `tool_sse(responses, [("prepare_note_edit", args)])` containing the complete required argument object with `start:null`, `end:null`, a unique exact quote, and ordinary metadata; then a text/final response. No live provider or credential lookup is needed: `provider_formats_tests::client` already creates a synthetic auth directory and injects `ScriptHttp` into `Auth`.
3. Extract the actual Rig completion model from the returned `ProviderClient` (`OwnedClient::{Chatgpt,Copilot}`) and call production `run_model` with the fixture Scope and matching `responses` flag. This takes the same production `register_tools` path as the Threads run: `PrepareChanges::for_scope`, `PrepareNoteEdit`, actual `EditArgs` typed dispatch, `ToolContext` scope, and `BatchHook` argument validation.
4. Assert completion, one recorded prepared candidate, candidate Note has precisely the intended replacement with untouched bytes/metadata preserved, and exactly the expected scripted HTTP request was consumed. Inspect captured request JSON: find `prepare_note_edit` in actual provider tool definitions, assert `start` and `end` remain required with `type:["integer","null"]`, then confirm the response tool call was dispatched and the subsequent tool result appears in provider history (the existing mock history test already asserts null serialization; the HTTP witness adds actual provider format and typed dispatch).

## Existing seams and bounds

- Provider route matrix and HTTP utilities: `crates/brn-ai/src/provider_formats_tests.rs` (`ROUTES` in existing modules, `client` near line 1621, `tool_sse` near 1581, `ScriptHttp` near 1403).
- Production Threads runtime seam: `crates/brn-ai/src/threads.rs` (`run_model` near 4259 and actual `register_tools` near 899); no need to call public `run_prepared` or alter production APIs.
- Local edit state and test helpers: `crates/brn-ai/src/threads_note_edit_tests.rs`; it already proves schema, typed dispatch, explicit nulls, deserializer rejection of omitted keys, and operation behavior with `MockCompletionModel`.

The test must run the local scripted HTTP transport through a real Rig provider completion, not call `prepare_edit` directly and not merely serialize the schema. `threads_note_edit_tests.rs` is a descendant of `threads`, so it can reach the private `run_model`; the crate-root provider fixture functions need `pub(crate)` only under `#[cfg(test)]` if visibility prevents reuse. Do not expose a production endpoint or accept a test provider through `ThreadRunRequest`.

No tests/builds/network/account/native actions were run for this assessment.
