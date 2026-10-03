# UI slice 2 (chat polish) and vault search tool: paused brainstorm

Date: 1 October 2026. State: **paused by the user before any design approval**. No spec, plan or code exists. The user intends a significant architecture redesign that moves BRN to a full [Rig](https://github.com/0xPlaygrounds/rig) setup, replacing the Codex App Server provider. Resume this work after that redesign. Re-check every provider-specific item below against the new architecture.

> 2026-10-03 integration note: Rig-based Step 4 chat is now merged through
> PR #14. The decisions and observations below remain a historical, unapproved
> brainstorm, not instructions to resume automatically. Reconcile any renewed
> scope with the [current simple notes roadmap](../simple-rig-notes/plan.md);
> older provider APIs, tool names and status observations are not current facts.

## Origin

Slice 2 covers the five "2 · Chat polish" rows in the [UI feature backlog](../../../ui/feature-backlog.md): model and effort control, context usage, explicit context chips, evidence inspection with Return, and connection status with actionable errors. The handoff source is `/Users/evokessler/repos/brn/brn-ui-handoff`, mainly `UI-SPEC.md`, `COMPONENT-INVENTORY.md`, `DECISIONS.md`, `OPEN-QUESTIONS.md` and `mockups/brn-recommended-workspace.html`.

## Decisions taken (user answers)

### Slice 2

| Topic | Decision |
| --- | --- |
| Scope | All five backlog items in one slice, with one spec and one plan |
| CLI | Parity: `brn models`; `ask --model/--effort/--attach`; usage in JSON |
| Attachable context | Approved source revisions only, pinned to the exact revision |
| Ask with an attachment | Attached files are sent whole, plus vault search ("file + search") |
| Attachment size | No fixed cap; bounded by the model's context window, with a clear error if the turn won't fit |
| Sequencing | Design the vault search tool **first**; slice 2 builds on it |

### Vault search tool ("agentic retrieval")

| Topic | Decision |
| --- | --- |
| Pre-search | Removed: tool-only, so the model searches when it needs to |
| Tools | `search_vault(query)`, `read_source(source_id, optional byte range)` and `list_sources(paged)`; read-only, approved current sources only; titles from `list_sources` are not citable |
| Search method | Best available: Hybrid with a native index, otherwise Keyword. Profile buttons leave the composer; optional override in Settings and CLI `--profile` |
| Manual Search | Moves to a search box in the vault rail; CLI `brn search` is unchanged |
| Answers without evidence | Allowed and labelled "No vault sources used"; the label is stored with the turn |
| Pre-tool conversations | Continuing one starts a fresh provider thread; the BRN transcript stays visible |
| Wiring (Codex era) | Approach A: generic tool hook in the provider, tools implemented in `brn-workflow` |
| Codex pin (Codex era) | 0.159.2; superseded if Rig replaces Codex |

### Draft design, never approved

- **Workflow:** `VaultTools` lives in the workflow and reuses today's evidence validation. A per-conversation evidence ledger numbers `[n]` monotonically so the model can cite evidence from earlier turns.
- **Store:** additive. Evidence gains `ref_no`, `origin` (`pre_search` | tool call) and `cited`. A new `turn_tool_calls` audit table records each call. `sessions` gains a tool-protocol marker.
- **Citations:** passages the model saw are recorded as *consulted*. `[n]` markers in the final answer mark them *cited*; unknown numbers are shown as unresolved, not as links.
- **Instructions:** tool output is untrusted data, not instructions. The model gets no filesystem, shell, web or publication tools.

## Open questions at pause

1. **Provider direction:** the user plans to move to Rig. Rig's `providers::chatgpt` reuses the Codex CLI OAuth client ID, and `providers::copilot` sends VS Code Copilot Chat integration headers. Both use subscriptions through unofficial client impersonation, and BRN would hold and refresh OAuth tokens. The redesign must settle credential storage and terms-of-service risk.
2. With Rig, BRN owns the agent loop, so tool registration, thread history, compaction and context-window reporting move into BRN. The Codex resume question below then disappears, and the slice 2 usage and model-list sources change.
3. Tool limits per turn (call count, per-call byte caps) were not decided.
4. Whether slice 2's connection status needs a long-lived provider connection; today the workflow starts a provider process for each ask.

## Facts gathered (1 October 2026)

- `brn-provider` pins Codex `0.155.0-alpha.16.4` (`crates/brn-provider/src/lib.rs:15`). That binary is no longer installed: ChatGPT.app bundles 0.159.2 at `/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex`, and PATH has 0.154.0. Live asks would fail the version check until BRN is re-pinned or replaced.
- Codex 0.159.2 supports `model/list` (per-model `supportedReasoningEfforts` and `defaultReasoningEffort`), `model` and `effort` on `turn/start`, `model/rerouted`, `thread/tokenUsage/updated` with `modelContextWindow`, `account/rateLimits/updated`, and structured quota errors (`codexErrorInfo: "usageLimitExceeded"` with a reset time).
- `dynamicTools` exists only in the experimental schema. It requires `initialize.capabilities.experimentalApi`, is accepted on `thread/start`, and has no field on `thread/resume`. Resume emits a deprecation notice recommending `excludeTurns: true`.
- A live spike (synthetic data, Codex-owned credentials) confirmed initialisation, `thread/start` with tools, `model/list` and cross-process resume. It **could not** confirm whether tools persist across resume because the account hit its usage limit (reset 3 October 2026, 20:24). The throwaway script lives only in the session workspace and is not part of the repository.
- `docs/status.md` still describes the workspace shell as unmerged, although PR #12 merged it to `main` (`1a30db0`).

Neither this note nor any answer above authorizes implementation.
