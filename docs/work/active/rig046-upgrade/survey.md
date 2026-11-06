# Focused runtime architecture survey (#253)

One `/improve-codebase-architecture` pass over `crates/brn-ai` before Copilot
becomes the second real provider adapter. Scope: provider/account/model selection,
model-using paths, and release risks in `threads.rs`, `auth.rs`, `conversion.rs`,
`document_refine.rs`, `subscription_search.rs`, `threads_research.rs` and
`threads_summary.rs`. There is no repository glossary or ADR directory; the
[invariants](../../../architecture/invariants.md) and
[Threads target](../../../architecture/threads-target.md) were the constraints.

## Model-using paths

| Path | Provider/model chosen | Captured at start | Silent fallback |
|---|---|---|---|
| Chat run and summaries | `prepare_run` checks the request against workspace settings and persists provider/model in the Run (`threads.rs` ~3487–3600, ~3828) | Provider and model yes; account no (credentials re-read at dispatch, `provider_client_context` ~3046) | None. Copilot's single Responses retry keeps the same client and model |
| Helpers/research | `HelperSettings` → frozen `HelperChoice` (`threads_research.rs` ~243–320) | Provider and model yes; account re-read per call | None; unavailable helpers continue without another model |
| Model catalogue/account | Explicit provider on every `Auth` call (`auth.rs`) | n/a | None |
| Document refinement (Luna) | Hard-coded `Provider::Chatgpt`, `gpt-6-luna` (`conversion.rs` ~16, ~76–119) | Yes | None, but Copilot cannot be selected |
| Native web search | Rejects every non-ChatGPT selection (`subscription_search.rs` ~178–184), Codex App Server with fallback disabled | Yes | None, but Copilot cannot be selected |
| Intake, titles | No model (`threads_intake.rs` header); no title model path | n/a | n/a |

A Codex auth file serves only ChatGPT; Copilot always uses the credentials folder.
This is documented in `provider_client_context` and the run settings must match
exactly, so it is not a silent fallback.

## Release risks and routing

1. **Account identity is not captured at operation start.** Provider and model are
   frozen, but credential contents are read at dispatch. A sign-out or account
   switch between preparation and dispatch could run the same Run under another
   account. Copilot's bootstrap fingerprint protects only summary-cache reuse.
   → #252 (Copilot PR): capture an account fingerprint at operation start and
   refuse dispatch if it changed. This is the account-isolation requirement there.
2. **Luna refinement and native web search are ChatGPT-only.** Release blockers for
   the Copilot alpha journey, not fallback bugs. → #252.

No high-confidence unbounded retry or missing cancellation was found: the Copilot
route retry runs once, web search is capped at three calls, and the App Server loops
check a 60 s deadline. The Rig PR's own changes (stall classification, explicit
serde features) were the small repairs selected here; no further source change is
safe within the runtime-upgrade scope.

## Deepening candidates

| Candidate | Strength | Routing |
|---|---|---|
| **Provider operation module.** One interface for account resolution, client construction, route retry, account identity and capabilities. Provider branching is duplicated across chat, helpers, summaries, refinement and search, so the deletion test shows the complexity would reappear across six callers. Copilot makes it a real seam (two adapters). | Strong | #255 follow-up; #252 keeps its changes local to the existing seams |
| **Account snapshot.** Account identity as part of the frozen operation selection. | Strong | #252 (risk 1) |
| **Provider-aware refinement.** `conversion_response` takes the operation's selection instead of owning a ChatGPT/Luna choice. | Strong | #252 |
| **Provider-native search interface.** Separate ChatGPT (Codex App Server) and Copilot adapters behind the existing bounded `SearchResult`. | Strong | #252 |
