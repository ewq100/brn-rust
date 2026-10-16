# Review: Rig-first agent capability plan

**Reviewed:** 16 October 2026, the owner-supplied `BRN_Rig_First_Implementation_Plan.md` (prepared 15 October, inspected main `7e9d0376`).
**Against:**

- main `cafb24c2`;
- the three unpushed finish-first branch tips: `work/ui-polish` `40930c09`, `work/daily-workflow-reliability` `b76cc5a7` and `work/cli-agent-contract` `2537db54`;
- Rig tag `v0.44.0` sources;
- crates.io metadata.

**Method:** source inspection and public package metadata only. No builds, tests, provider calls or downloads.

## Verdict

**Sound direction. Ready for Slice 0 after amendments A1–A9; Slices 3 and 4 also need two owner scope decisions.**

The plan reuses Rig correctly. It refuses opaque checkpoints and keeps BRN's authority model, and its acceptance registry is strong. Its main gaps come from its timing:

- it predates about 16k lines of in-flight work that already delivers much of Slice 2;
- it would break a measured prompt-cache contract;
- it would expand canonical scope without naming the decision.

Run it as **workstream D** under the same integration lead and C0 as the modular-foundation program, not as a parallel program.

## Verified claims

| Plan claim | Result | Evidence |
|---|---|---|
| Rig `=0.44.0` with `agent`, `derive`, `reqwest` | Confirmed | `crates/brn-ai/Cargo.toml` |
| `TokenWindowMemory`, `MemoryPolicy`, `CompactingMemory` exist; summary state is process-local | Confirmed | `rig-memory` `lib.rs`: `state: StdMutex<HashMap<ConversationId, …>>`, "within this process lifetime" |
| `TemplateCompactor` rolls up into a system message | Confirmed | `TextSummary` → `Message::System` (`lib.rs` ~1073–1094) |
| `Compactor` and `ConversationMemory` interfaces | Confirmed | `rig-core` `memory.rs` lines 86, 237 |
| `rig-rmcp` adapts MCP tools into dynamic Rig tools with list-change refresh | Confirmed | `rig-rmcp` `lib.rs` (`McpTool`, `McpClientHandler`) |
| `AgentRun` is serializable; direct construction runs no hooks/tools/memory | Confirmed | `rig-agent` README, "The run protocol" |
| Facade features are published, not yet in the graph | Confirmed | `memory`/`rmcp` features in `rig` 0.44.0; `rig-memory`/`rig-rmcp` 0.44.0 on crates.io (MIT); absent from `Cargo.lock` |
| 40 recent messages, truncated; automatic memory off | Confirmed | `.take(40)`/`.without_memory()` at `threads.rs` 1930/2520 (main), 2146/2826 (reliability); 16 KiB per message |
| Four compiled skills; `read_skill`; intake preloads two | Confirmed | `threads_guides.rs` `CATALOG`, `preloaded()` |
| No opaque persisted provider transcripts | Confirmed | invariants #13 |

## Findings

| # | Severity | Finding | Amendment |
|---|---|---|---|
| F1 | High | **Within-run trimming contradicts a measured, tested contract** (detail below) | A1 |
| F2 | High | **Stale baseline** (detail below) | A2 |
| F3 | High | **Unnamed scope expansion** (detail below) | A3 |
| F4 | Medium | **Two leads on one hot file.** Both programs claim a single lead for `threads.rs`, core schema and the lockfile; this plan's Slice 0 duplicates C0. Two baselines and two leads would race. | A4 |
| F5 | Medium | **Order against the writing program.** 1A budgets "document/reference context", but CW1/CW2 define writing request v2. Budgeting first means doing it twice. CW also unblocks the writing UI lane, while D slices unblock nothing else. | A4 |
| F6 | Medium | **No model-limit source.** BRN has no context-window data, only classification of the provider's `context_length_exceeded` error. "Validated model/provider information" does not yet exist. | A5 |
| F7 | Medium | **Summary storage left open** (detail below) | A6 |
| F8 | Medium | **MCP dependency facts missing** (detail below) | A7 |
| F9 | Low | **Run states.** "Budget-limited" and "awaiting review" are not `RunState` values, which are Working, Interrupted, Completed, Cancelled and Failed. Adding variants changes serialized records. Reliability's run-attention rules can express them. | A2 |
| F10 | Low | **Skill manifests.** Built-in descriptions are hard-coded in Rust and differ from the files' frontmatter. A YAML dependency is not required for a two-key header. A catalogue change also alters the cached preamble. | A8 |
| F11 | Low | **Compaction model and cost.** The default (the run's own model) is right. The existing experimental helper-model setting is an explicit, visible option worth naming. A compaction request should not reuse the chat's cache key. | A6 |
| F12 | Low | **External-data grant is new core authority.** It belongs to the lead, reuses the scope predicate, and any schema change is serialized. | A7 |

**F1 — within-run trimming.** `threads-behavior.md` ("Prompt caching (Codex route)") pins `prompt_cache_key` per chat, keeps the request prefix append-only, and records that earlier tool results are not trimmed because measurement showed no net saving. The plan's "bound tool-result growth during the run" would trim earlier items. A sliding window applied every run would also change the history prefix every run.

**F2 — stale baseline.** The plan names two finish-first branches, but there are three. Together they add run leases and orphan recovery, one terminal notice per chat, killed-run candidate review, "Continue starts fresh", `withdraw_proposal`, `prepare_run`, scoped owner message search and caller-keyed CLI operations. Much of Slice 2, and 1A's older-discussion route, already exists.

**F3 — unnamed scope expansion.** `threads-target.md` specifies a fixed skill catalogue with "no general plugin loader, filesystem discovery". The vision, roadmap and target defer "broad connectors". Master plan section 10 C4 says no "dynamic skill platform". Slices 3 and 4 need recorded owner amendments, or an implementing agent faces contradictory canonical documents.

**F7 — summary storage.** The plan leaves storage open. A `brn.sqlite` schema increment would contend with CW1's. A derived cache beside the search index is rebuildable and needs no schema change; losing it costs only repeated compaction calls.

**F8 — MCP dependency facts.**

- `rig-rmcp` 0.44.0 depends on `rmcp` major 2 (latest 2.2.0, while upstream is 3.5.1), so BRN stays on 2.x until a Rig upgrade.
- `rig-rmcp` enables only `client`, so BRN must add one transport feature itself.
- The streamable HTTP client reuses locked `reqwest` 0.13 and adds no process spawning.
- The child-process transport spawns executables.

## Amendments applied

The amended plan is in `amended/`; `amendments.patch` reproduces it from `original/`.

- **A1 (F1).** Section 2 records the cache contract. 1A applies the policy once at run start, keeps within-run prefixes append-only, caps tool results at admission, and ends with a limit outcome. Across runs it moves the cut only at coarse compaction boundaries. New acceptance case C07; Slice 0 adds a prefix-stability proof.
- **A2 (F2, F9).** Section 2 records the in-flight work, and section 3 names all three branches. Slice 2 starts from the landed work and builds only the delta, using existing `RunState` values plus run attention. 1A reuses the scoped owner message search. Earlier Runs' progress joins the budget.
- **A3 (F3).** New section 1 subsection with owner decisions D-S and D-M, recorded in the canonical target. Slices 3 and 4 start only after them.
- **A4 (F4, F5).** One integration lead and a combined C0. Default lane order: C0 → CW1 → CW2 → 1A → 1B → 2 → 3 → 4 → 5, with the owner able to move 1A earlier. Slice 5 and the writing milestone are separate milestones.
- **A5 (F6).** Slice 0 chooses a limit source: provider metadata, else a dated per-model table, labelled as an estimate. `context_length_exceeded` becomes the same truthful limit outcome (C03).
- **A6 (F7, F11).** Default storage is a derived cache in `derived/`, excluded from export, search and backups; `brn.sqlite` only if Slice 0 shows it is needed. Compaction runs as a separate request without the chat's cache key. A helper-model compaction setting is an owner option.
- **A7 (F8, F12).** Section 2 and Slice 4 record the `rmcp` 2.x line. The default transport is streamable HTTP via `reqwest`; the child-process transport is deferred. The external-data grant is lead-owned core authority.
- **A8 (F10).** One source for skill metadata. The parser is chosen in Slice 0: a limited YAML crate or a strict documented subset. `BUNDLE_VERSION` gains the catalogue digest, and catalogue changes apply only at run start.
- **A9 (housekeeping).** Header review line. The source ledger gains B9–B11, R4 and R5. Slice 0 reuse proofs may run early as isolated experiment R0. I02 covers the CLI contract.

## Kept as written

- Rig-first reuse and no replacement agent loop.
- Small BRN continuation records plus fresh invocations as the production design; checkpoints only in a disposable experiment.
- Instruction-only skills with package-confined references and no execution.
- MCP read-only by owner allowlist, not by `readOnlyHint`.
- The acceptance registry and its evidence distinctions.
- No Codex parity claim.
