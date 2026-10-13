# Ordinary wiki guidance and notes UI exploration

**Owner assignment, 13 October 2026:** implement Karpathy-style wiki logic for ordinary nodes, retaining protection, checked writes and Undo; begin with runtime guidance and evaluate it against current behavior. The owner selected **design and prototype first** for the UI and **Codex `gpt-6.1-sol`, medium** for the live comparison.

## Required behavior

Ordinary knowledge notes are maintained, source-grounded pages with a stable subject and future use. Read related current knowledge before choosing update/create/leave in thread. Prefer revising the existing subject page. Create a distinct entity/project, concept/process or synthesis/comparison page only when independent reuse earns it; roles are writing conventions, not new record kinds. Personal writing and drafts remain supported.

Preserve evidence attribution, effective/observation dates when meaningful, quantities, units, uncertainty, exceptions and disagreements. Distinguish reported claims, AI inference and owner decisions. Record direct input versions and supported Link relationships. Follow generated synthesis back to underlying evidence; repeated AI restatement supplies no independent corroboration. A useful source may update several relevant pages or none; no mandatory source-summary or node count.

Queries first retrieve current related knowledge, then inspect available evidence when needed. Save an answer as a new page only when reusable and authorized. Maintenance is a bounded review of affected topics and recorded stale dependencies; discover duplicates, contradictory claims, missing support and useful links. Runtime guidance grants no permissions, external monitoring, scheduled work or claims of guaranteed correctness.

Keep the existing whole-note protection, host-owned authority, exact immutable candidates, grouped version/dirty-guard checks, replay and receipt Undo. Protected sources and decisions remain distinct from truth. Full imports remain faithful, protected references; raw originals remain external and temporary bodies expire under the existing contract.

## Evaluation and acceptance

Use identical CLI/runtime code with old and new compiled guidance; the old binary is captured before editing. Record executable hashes, baseline commit, guidance identities, scenario identities, provider/model/effort and command results. Use fresh explicit synthetic workspaces, no owner knowledge, no external messages, no fallback provider. Keep the same eight tool rounds for both arms and repeat twice. A failed or interrupted arm remains visible and cannot contribute a success claim.

Scenarios: sequential project updates with Estonian evidence and a protected imported reference recording a contradictory commitment; reusable synthesis from two full imported references; a one-off question that should stay in conversation. The imported commitment reference is protected evidence, not a host-confirmed owner decision; existing authority/confirmation tests cover that distinction. Compare actual stored notes, links, preparation inputs, attention, receipts, answer and elapsed time. Structural assertions test protection, references, input lineage and persistence; a separate rubric inspects factual support, qualifications, organization, duplication and usefulness. Literal matches are evidence signals, not an entailment judge. Do not claim improvement from predetermined mock replies or a larger prompt alone. Token usage is unavailable if the production CLI does not expose it; never infer it from answer length.

Retain existing runtime/core tests for stale current inputs, guarded groups, protected writes, skill loading and Undo. Add meaningful tests for the evaluation harness's exclusive scratch ownership, failure accounting, sensitive configuration redaction, structural comparisons and restoration verification. No new prompt-word presence tests.

## UI exploration

**Follow-up owner choice, 13 October 2026:** B — subject overview with evidence chronology guides a later native UI pass. The owner then asked how hundreds of notes should be navigated and requested 2–3 researched approaches involving node graphs or Obsidian-style categories. The current pass records that selection, demonstrates bounded synthetic navigation and compares approaches; it does not authorize native implementation or a category schema.

Produce three structurally different, interactive, synthetic-data note layouts inside a recognizable Omakase tiled shell. Compare **reading with a contextual rail**, **subject overview with evidence**, and **connected-notes navigation**. Show note purpose independently from protection; sources, related notes, unresolved conflicts, last changes and Undo are reachable while text remains readable. Include working/protected notes and a source whose original is unavailable. Variant switching uses `?variant=` and a floating switcher; prototype controls are clearly synthetic and never mutate BRN.

The artifact is a throwaway standalone HTML preview beside the native presentation module on a separate prototype branch, because this native Rust app has no web route. Document how each proposed control maps to existing checked APIs, which visible information is present today, and which would require later implementation. Native UI implementation and owner acceptance of a variant are separate future gates. Preserve current editor/dirty/review semantics in the design.

## Boundaries

No schema, dependencies, authority, retrieval engine, provider defaults, editor or native UI changes in this pass. No automatically inferred page metadata, confidence badges, perpetual “up to date” labels, mandatory taxonomy, graph engine or raw-source archive. A controlled pilot cannot certify general factual accuracy, native usability or long-term wiki quality. Publication may produce a reviewable draft PR; merge/release require applicable current authority.
