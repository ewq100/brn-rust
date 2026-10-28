# Consent-controlled web search adapter

This slice reuses locked Rig 0.45.0 `DynamicTool` and the existing Rig/rmcp channel. It does not upgrade dependencies, add an orchestration layer or assume provider-native search is available or free. BRN continues to own policy, private/project scope, exact input consent, cancellation, byte bounds and result admission.

A host may attach one web search to a durably prepared Run only after the owner selects an already-approved anonymous local MCP tool and reviews the exact query. `with_web_search_input` is immediate review; delayed UIs must use `with_web_search_input_at_snapshot`. Helpers and frozen writing cannot acquire this capability. Configuration alone grants no input dispatch. Continue does not inherit a grant. The offered model name is `web_search`; the actual remote tool name and metadata remain pinned separately by the existing transport gate. New queries, repeat calls, policy revocation and context drift remain refused.

The CLI opt-in is `ask ... --mcp-tool EXACT_APPROVED_NAME --mcp-input '{"query":"owner-reviewed query"}' --mcp-purpose web-search`. This does not configure a connection. Ordinary MCP calls retain their existing behavior. There is no new native setup surface in this slice.

The adapter must advertise the already-supported Query input contract and return one bounded MCP text result containing JSON:

```json
{"results":[{"title":"Page title","url":"https://example.invalid/source","snippet":"Provider-reported search excerpt","published_at":null}]}
```

At most ten entries are accepted, with titles up to 512 UTF-8 bytes, snippets 4096 bytes, URLs 2048 bytes and optional provider-reported publication metadata 64 bytes. The existing response limit is 32 KiB after bounded wire admission. Empty results are explicit. Unknown fields, malformed results, duplicate URLs and unsupported links fail admission. Only HTTP/HTTPS named origins without credentials or nondefault ports are admitted; local/IP links are rejected. These are citation links, not instructions to fetch: **BRN never resolves or retrieves these result URLs**. Source identity and transport metadata accompany host observation time. Snippets, URLs and dates are untrusted provider reports, not independently verified page contents or canonical BRN records. Results grant no read ledger, prepared-change, Apply or completion authority. Full traces also omit the reviewed query and result text.

No compatible configured search service has been qualified in this task. Search is absent by default and unavailable without configuration plus fresh consent. Activation requires an owner-selected compatible local service, its own independently approved provider/cost arrangements, metadata approval and scope selection through the existing MCP controls, and one exact reviewed query per invocation. This task neither installs nor enables a service, creates credentials, calls a paid API, sends real private material, nor qualifies live search quality. Synthetic loopback fixtures qualify the adapter only.

A broad read/search facade is not part of this slice. Existing note ranges, read coverage, Source inputs and candidate lifecycle operations have distinct permission contracts; no demonstrated simplification benefit justifies combining them. Prepare and Apply stay separate.

## Candidate verification — 28 October 2026

- Seven focused `web_search` tests passed: typed/no-result admission, hostile/oversized inputs, exact one-query review, absent configuration/grant, revoked/stale review, invalid citations and frozen-writing refusal.
- All 30 existing and extended MCP tests passed. They include generic and web modes for same/other/same selected-tool cache identity, repeated-call refusal, metadata/context drift and in-flight stop/revocation. The separate helper-exclusion test passed for both modes. Counts overlap with the focused run; they are not summed as unique cases.
- The CLI opt-in validation test passed, including malformed, extra-field, blank, control-character and oversized query rejection before workspace creation. Strict all-target AI lint with `scripted-runs` and strict all-target CLI/application lint passed.
- Independent source/security review found and corrected validation-after-prepare and selected-tool cache-identity issues before qualification. Initial compilation then failed on a `Result` alias collision; explicit standard-library result typing corrected it. These failures are retained rather than represented as passing first attempts.
- No live provider, configured production search service, real user data or native UI was exercised. No model or package upgrade was made. Hosted exact-head CI and publication remain subsequent gates.

Server read-only annotations are not proof of upstream behavior. BRN's adapter does not establish the selected service's upstream network, credentials or pricing policy; those remain an explicit activation decision outside this synthetic qualification.
