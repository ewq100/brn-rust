# Rig 0.45 migration

Active local branch `codex/rig045-migration-20261023`, stacked on maintenance
`8f7143f34191ad53e4c7e74f7c061e8dce5e6e35`, with UI branch `125f8ef0` preserved.
Main fetched unchanged at `7b416591`. No publication, merge or release authority.
No live accounts, credential transfer, new persistent access or native-picker retry.

## Compatibility checklist before implementation

- Adapt AgentBuilder, contextual tools/handlers and Arc-owned messages/requests.
- Preserve accepted-final ownership, exact history, bounded argument correction,
  cancellation/approval fences and tool-call/result pairing under new projections.
- Use supported Rig conversation/summary/budget facilities in place of removed
  memory helpers. Retain BRN-owned persisted records/cache DTOs and exact bytes.
- Align coupled MCP to Rig's released `rig-rmcp` requirement (3.5); no independent
  latest-dependency chase or parser/helper graph changes.
- Compare old credential cache shapes, explicit login and refresh/finalizer policy
  with released authenticators. Prefer Rig TokenStore adapters; do not duplicate
  OAuth. No irreversible rewrite or new persistence without separate approval.
- Qualify synthetic old stored state, provider wire/schema/history/cancellation,
  workspace/native behavior and independent source/evidence review.

Read-only persistence audit found no Rig types in canonical core/intake/app stored
records; summary caches serialize BRN Entry/Semantic DTOs, not Rig Message. No DB
migration is indicated. Authentication uses a private Rig TokenStore adapter that reads/writes the existing
ChatGPT five-field JSON and separate Copilot bootstrap/session files. It combines
Copilot records only in memory. Selected Codex access tokens remain read-only in
a Rig MemoryTokenStore. Existing provider mutexes, checked paths and finalizers
remain in charge; no new lockfile, credential location or DB migration is needed.
No real credential action has been taken.

Official release: https://github.com/0xPlaygrounds/rig/releases/tag/v0.45.0
(released 2026-10-19). Its tagged MIGRATING.md stops at 0.44. The current official
guide covers 0.44→0.45; every used API is cross-checked against released crates:
https://github.com/0xPlaygrounds/rig/blob/main/MIGRATING.md
The released rig-rmcp 0.45.0 manifest requires rmcp3.5.

Raw inspection/build evidence: `/workspace/brn-environment/logs/rig045-migration`.
## Implementation checkpoint

Workspace all-target compilation passed after adapting shared Message ownership,
RegisteredTool execution, middleware Exchange, provider construction and MCP fixture
responses. Rig's released rmcp requirement is 3.5.0, including macros 3.5.0;
Tokio 1.53.3 and Schemars 1.2.3 remain unchanged. Helper graphs are unchanged.

The removed Compactor trait wrapper is retired. BRN keeps its existing bounded
Rig model call and structured Semantic/Entry DTOs: the new generic Summarizer
serializes/trims conversational text, which cannot preserve BRN's exact coverage,
source IDs and JSON provenance contract. Recent history uses Rig's supported
TokenBudget/view facility. No alternate model runtime was introduced.

MCP uses Rig McpConnection and DynamicTool::run_with. Sessionless/replayed GET,
MRTR continuation fields and task RPCs remain refused; approved invocation scope
cannot expand through the new protocol. New synthetic regressions cover those
boundaries and old credential formats/partial writes/unsafe destinations.

Independent source review found no blocker in auth/Codex/history/summary adapters;
provider behavior qualification passed; aggregate/native qualification is pending. Earlier compile failures are retained in raw
logs. First offline provider run: 499 passed, 3 failed, 1 ignored. Two failures retain
the old Rig user-agent version; one MCP byte-cap refusal now returns Transport
instead of Timeout. The user-agent assertions are corrected. Released rmcp 3.5 now emits
transport_closed for an unfinished response stream (streamable_http_client.rs
1767–1790), whereas 2.2 only warned and left the RPC pending. The MCP assertion
now expects Transport and strengthens prompt refusal timing while preserving
raw-byte-cap, exactly-one-call and zero-redirect checks; no production fix was
needed. The corrected full provider suite passed: 505 passed, 0 failed, 1 ignored
(502 library tests plus 3 capability-example tests). Auth/history/summary/cancellation and approval tests passed in that run.
Native and old-store checks, strict lint, aggregate tests and final review remain.

