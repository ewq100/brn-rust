# LLM wikis as BRN's ordinary-note logic

Research date: 13 October 2026 (Europe/Tallinn). BRN reference baseline: `65119a73`. Scope: public primary-source research and compatibility analysis; no implementation changes, private-data access, provider experiments, or BRN comparative evaluation.

## Recommendation

Adopt the **maintained, source-backed synthesis pattern** as an explicit editorial policy for ordinary BRN notes. Treat them as useful working knowledge that can evolve when new evidence arrives. Keep BRN's SQLite authority, checked writes, protection, versioning, review, and Undo. Keep retrieval over both working knowledge and available evidence. Evaluate the policy before introducing a new ontology, graph engine, or third-party wiki runtime.

This is a product recommendation, not an experimentally established result for BRN. The existing behavior contract already delegates coherent ordinary-note maintenance and source-aware intake; the useful improvement is making the semantic choices predictable and observable. The local contract does not currently promise a whole-corpus wiki compiler or continuous source monitoring. [BRN maintenance behavior](../architecture/threads-behavior.md), [derivation and refresh contract](../architecture/threads-target.md#when-other-work-depends-on-it), [invariants](../architecture/invariants.md).

“Capati” is interpreted here as **Andrej Karpathy**. His LLM Wiki is a particular articulation of the broader LLM-maintained-wiki idea, not a separate competing architecture. If another author or system was intended, this identification needs correction.

## What Karpathy actually proposed

Karpathy's gist was created on 4 April 2026; its visible page shows one revision. It describes a configurable idea, not a released application or controlled experiment. It separates immutable source material, an LLM-maintained Markdown wiki, and instructions describing the organization and workflows. Source intake integrates knowledge across pages; queries use existing knowledge with citations and can become saved analyses; periodic maintenance looks for inconsistency, staleness, missing links, and gaps. Entity, concept, source-summary, comparison, and synthesis pages are possible outputs. A catalog aids navigation and an operation log records changes; search is optional as scale increases. The author describes personally supervised ingestion and claims a catalog works around a hundred sources. The appeal is preserving useful synthesis instead of repeating it for every question. The stronger claims that maintenance is almost free and models do not forget cross-reference updates are assertions, not demonstrated reliability guarantees. [Original LLM Wiki gist](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f).

For BRN, “wiki” should describe a **maintained knowledge experience**. It should not mean every source must produce a fixed number of nodes, every named entity needs a page, generated content is automatically true, or a Markdown directory becomes the canonical database. Those are independent product and implementation choices.

## Wiki, RAG, GraphRAG, and agent memory answer different questions

| Approach | What persists | What it helps with | Implication for BRN |
|---|---|---|---|
| Retrieval-augmented generation | A searchable external corpus/index used to supply evidence to generation | Finding and answering from relevant evidence | Keep evidence lookup; retrieval does not require treating generated summaries as originals. |
| Maintained wiki | Human-readable, revisable knowledge artifacts with links and provenance | Repeated understanding, synthesis, and browsing | Ordinary notes can supply this layer while evidence remains separately identifiable. |
| GraphRAG | Extracted entities/relations and generated community reports alongside text units | Entity-oriented and corpus-wide questions | A potential retrieval enhancement; adopting wiki behavior does not require it. |
| Agent memory | Retained history, facts, preferences, decisions, and other context used in later interactions | Continuity across conversations and time | Some BRN notes may serve as memory, but a source's statement must not silently become an owner's preference or decision. |

The original RAG research combines generation with non-parametric memory; it does not impose a universal rule that every intermediate artifact must be discarded. RAPTOR retrieves both original chunks and hierarchical generated summaries. Current Microsoft GraphRAG derives structured artifacts before queries, and offers local, global, DRIFT, and basic search. Therefore “wiki versus RAG” is often a misleading product choice: **what knowledge to maintain** and **how to retrieve evidence** can be designed separately. [Original RAG paper](https://arxiv.org/abs/2005.11401), [RAPTOR paper](https://arxiv.org/html/2401.18059v1), [GraphRAG dataflow](https://microsoft.github.io/graphrag/index/default_dataflow/), [GraphRAG query overview](https://microsoft.github.io/graphrag/query/overview/).

GraphRAG's initial research evaluated global sensemaking over two corpora around one million tokens, using generated questions and LLM preference judgments. It reports better comprehensiveness/diversity than a naive retrieval baseline, but explicitly limits generalization to other tasks and notes that graph-free map/reduce can be competitive. This is evidence for some broad synthesis tasks, not proof that a graph improves ordinary BRN note maintenance. Its current getting-started guide also flags substantial indexing resource consumption. [GraphRAG research](https://arxiv.org/html/2404.16130v1), [official getting-started guide](https://microsoft.github.io/graphrag/get_started/).

## Primary empirical evidence, with methodological limits

The 2026 wiki papers below are assessed from the linked arXiv versions, not promoted as independently replicated production results. Their scores use different tasks, models, corpora, and judgments; they cannot be combined into one ranking. Publication on arXiv alone does not verify peer review.

### 1. WiCER: compilation can destroy the facts users need

The 8 May 2026 paper tests Llama 3.1 8B local inference and Sonnet compilation/judging. Across 17 synthetic RepLiQA domains, Table 3 reports blind wiki scores of 2.14–2.32/5, raw full-context 3.46, and RAG 3.63; omitted detail drives failures. Diagnosis-guided fact preservation improves compiled artifacts. Important limits: Table 4 evaluates the same 80 per-domain probes used for refinement and selects the best iteration, so it does not establish performance on unseen questions; retrieval uses a relatively simple fixed top-five baseline. The paper implements full-context KV caching, whereas a navigable wiki need not fit in one context. It also cites a Karpathy URL resolving to an unrelated commit-message gist. Reported iteration cost includes roughly 130K API input/17K output tokens and about 50 minutes on its setup. The transferable lesson is to test retention of required facts and preserve source access, not to adopt its serving architecture or extrapolate its timing to BRN. [WiCER paper](https://arxiv.org/html/2605.07068v1), [the cited unrelated gist](https://gist.github.com/karpathy/1dd0294ef9567971c1e4348a90d69285).

### 2. Retrieval as Reasoning: linked knowledge plus active navigation can help

The 25 May 2026 paper uses GLM-5.1 across compilation and answering, 500 questions each from HotpotQA/MuSiQue/2WikiMHQA, and AuthTrace. LLM-Wiki beats its listed baselines on multi-hop F1; removing the wiki structure or progressive traversal reduces performance. This includes a flat-chunk agent ablation, so the evidence is stronger than merely comparing an agent to single-shot RAG. However, its main vanilla baselines are top-five retrieval while the wiki agent has up to 15 tool calls: those headline comparisons do not isolate representation alone. AuthTrace's single-document condition favors HippoRAG 2, consistent with detail loss. The paper explicitly leaves large-scale dynamic maintenance, stale facts, and global directory maintenance to future work. An Error Book and structural repair are tested; reliable ongoing source replacement/deletion is not established. [Retrieval as Reasoning paper](https://arxiv.org/html/2605.25480v1).

### 3. A preregistered wiki/RAG comparison finds no universal cost win

The 18 May 2026 comparison covers only 24 papers and 13 questions. The wiki helps cross-paper connections, but uses about 21 times the query tokens of the tested RAG pipeline. An exploratory decomposition-RAG variant recovers about 88% of the synthesis gap at lower token cost. Same answering model is a useful control; multiple architectural differences and tiny subsets limit causal claims. Most importantly, the claim-level citation analysis checks wiki answers against wiki pages, not those pages against underlying PDFs. Better answer-to-page alignment therefore does not prove original-source fidelity. Embedding, reranking, storage, and dollar normalization are excluded from token-cost accounting; ingest caching prevents adjudicating the expected upfront-cost claim. This paper directly challenges the assumption that precompilation necessarily makes later queries cheap. [Preregistered comparison](https://arxiv.org/html/2605.18490v1).

### 4. Better wiki navigation may reduce cost, with a quality tradeoff

The 6 July 2026 progressive-disclosure study freezes page bodies across four arms of a 709-page wiki: 40 questions, three access regimes, two replicates, 960 runs. Summaries plus keyword-ranked retrieval reduce answering cost by roughly 30–34% in agent-routing conditions; pooled quality meets the registered non-inferiority margin. This is a useful content-controlled navigation study, not a wiki-versus-RAG result. Correctness/completeness decline slightly while citation validity/concision improve; forced catalog preload does not establish non-inferiority. Its human-judge reliability gate fails (pooled kappa 0.23), and the prescribed regrade was not performed. The corpus, answers, and gold are private, limiting independent reproduction; one author is the human rater. For BRN this motivates measuring retrieval separately from editorial policy, with component-level quality metrics. [Progressive-disclosure paper](https://arxiv.org/html/2607.04576v1).

### 5. A course-material study is promising but small

The 21 September 2026 course study reports 9.95 versus 9.05/10 and 98% versus 81% judged groundedness on 59 questions, including 14 cross-page questions. Its conclusion discloses the decisive asymmetry: the wiki arm receives the same fixed sections covering every target, with no context-selection step that can fail; RAG selects five chunks from a 21-vector index. This is evidence about retrieved context versus effectively perfect context selection, not an isolated test of autonomous wiki navigation. Other limits include one corpus, one answer/judge pass per question, no measured cost/latency, and no student-learning experiment. Bootstrap intervals cover question sampling, not model randomness. Groundedness against supplied wiki text also does not establish source-to-wiki fidelity. This result supports further controlled trials, not a general claim that wikis beat retrieval. [Course-wiki study, especially conclusion](https://arxiv.org/html/2609.25303v1).

### 6. Dynamic compilation research does not yet solve general note maintenance

The 3 June 2026 streaming paper proposes bounded fact pinning, eviction, and periodic recompilation, tested with finance volatility signals and Wikipedia edit activity. Its finance classification head has AUROC 0.728 but precision only 0.130 and recall 0.583 at the selected threshold; importance scoring is imperfect. The finance QA table exposes a same-backbone judge confound: direct answers can outscore enriched ones. Wikipedia results provide a more favorable context-sensitive signal. The theoretical regret bound depends on its model/assumptions, not a guarantee of fact correctness. These experiments concern prioritizing limited-context updates using domain signals; they do not demonstrate protected decisions, human editing, transitive provenance, owner acceptance, or source-erasure safety. For BRN, usefulness and authority must remain separate from salience. [Streaming compilation paper](https://arxiv.org/html/2606.09877v1).

### 7. Memory evaluation favors a hybrid and distinguishes failure layers

The September 2026 EvalMem paper separates encoding, retrieval, and generation failures, and adds a wiki-like auxiliary retrieval surface to seven systems' own memory exports. It reports mean answer-accuracy gains of 2.5 percentage points on LoCoMo and 2.3 on LongMemEval-S. Auxiliary hits expand to original memory evidence; a 30% auxiliary quota prevents summaries from crowding out evidence. Removing the quota reduces aggregate performance below baseline. Dynamic simulation also exposes stale evidence missed by static replay. Limits include LLM judges, standardized rather than system-specific tuning, synthetic dynamic personas, and limited language/domain coverage. Its arXiv record reports acceptance at EMNLP 2026 Findings. The directly relevant implication is to retain an evidence channel and diagnose where failures happen; creating more derived pages alone is insufficient. [EvalMem paper](https://arxiv.org/html/2609.22231v1), [submission and venue metadata](https://arxiv.org/abs/2609.22231).

## What first-party implementations actually enforce

These repositories are primary evidence about their own software, not independent evidence that the overall concept is reliable. No listed implementation was installed, run against owner data, or audited exhaustively.

| Implementation inspected | Concrete mechanism or limitation | Relevance |
|---|---|---|
| SamurAIGPT/llm-wiki-agent, commit `a17ab5df66da9b74a0eaeb664a67d662d0fb166d` (12 October 2026) | Its schema defines source/entity/concept/synthesis pages, supervised ingest/query workflows, deterministic structural health checks, and more expensive semantic lint. Query citations can be wiki links; source lineage is separately needed. | Good workflow example. Instructions that request consistency or full reading do not themselves enforce either, and automatic page creation can inflate a knowledge base. [Pinned schema](https://github.com/SamurAIGPT/llm-wiki-agent/blob/a17ab5df66da9b74a0eaeb664a67d662d0fb166d/AGENTS.md). |
| Atomicstrata/llm-wiki-compiler, commit `a3eb4473b0b2d2aa747d2f4a7f683d35efe16521` (10 October 2026) | Prompts extract concepts, distinguish extracted/merged/inferred/ambiguous provenance, and request paragraph citations to source line ranges. The code explicitly describes editorial policy prompts as advisory. | Helpful provenance vocabulary; a model's numerical confidence is not calibrated evidence of truth. [Pinned prompts](https://github.com/atomicstrata/llm-wiki-compiler/blob/a3eb4473b0b2d2aa747d2f4a7f683d35efe16521/src/compiler/prompts.ts). |
| Same compiler: freshness | Source ownership and hashes classify ordinary concept pages as fresh/stale/orphaned/unverified. Query pages are always unverified. Typed entity pages lack source ownership in this phase and are explicitly unverified, with ownership/pruning deferred. | “Fresh” means inputs still match, not that the real world has not changed or every claim is true. Source tracking needs to cover every maintained page kind. [Pinned freshness implementation](https://github.com/atomicstrata/llm-wiki-compiler/blob/a3eb4473b0b2d2aa747d2f4a7f683d35efe16521/src/freshness/index.ts). |
| Same compiler: deletion | Removal uses source-to-concept ownership, preserves shared concepts, regenerates derived artifacts, and reports actual rather than merely planned deletion. Typed pages have no ownership for this path. | Deleting a source and cleaning dependent generated knowledge are separate concerns. Whole-page ownership is weaker than claim-level lineage for shared pages. [Pinned removal command](https://github.com/atomicstrata/llm-wiki-compiler/blob/a3eb4473b0b2d2aa747d2f4a7f683d35efe16521/src/commands/rm.ts). |
| Same compiler: semantic evaluation | Samples cited prose-paragraph/source-span pairs deterministically, asks a model whether the span supports the claim, and caches by content/judge configuration/model. Line-less citations are not eligible for this span judge. | Distinguish link validity, citation coverage, and actual support. Semantic checking still needs an independent audit. [Pinned citation-support evaluator](https://github.com/atomicstrata/llm-wiki-compiler/blob/a3eb4473b0b2d2aa747d2f4a7f683d35efe16521/src/eval/citation-support.ts). |
| JanYork/llm-wiki-cli, commit `e1be6ab30e83947e4b0dba97ba910acfe3bd9821` (7 October 2026) | Its LongMemEval-S adapter explicitly measures retrieval, requiring 500 processed/470 scored instances for completeness. Its generic retrieval benchmark creates no compiled pages. Separate contracts record evidence, observed/decision/unresolved fields, time intervals, and supersedes/contradicts relations. | A retrieval score is not proof of synthesis quality, correct updates, or grounded answers. Structured temporal/provenance fields are useful design examples. [Pinned benchmark scope](https://github.com/JanYork/llm-wiki-cli/blob/e1be6ab30e83947e4b0dba97ba910acfe3bd9821/benchmarks/agent_memory/README.md), [raw-only benchmark](https://github.com/JanYork/llm-wiki-cli/blob/e1be6ab30e83947e4b0dba97ba910acfe3bd9821/benchmarks/README.md), [memory contract](https://github.com/JanYork/llm-wiki-cli/blob/e1be6ab30e83947e4b0dba97ba910acfe3bd9821/src/contracts.rs). |

Microsoft GraphRAG is also a concrete precomputation/retrieval hybrid. Its incremental-update issue was closed on 12 January 2026 with a maintainer explanation: append is the supported scope; deletion/document updates use ordinary reindexing with cached model calls, and further incremental-delete/update investment was not planned. This prevents treating “incremental indexing” as a promise of reliable arbitrary replacement. [Maintainer issue and scope](https://github.com/microsoft/graphrag/issues/741).

## The ordinary-note logic BRN should make explicit

The following is a proposed BRN editorial contract, not a claim that all details are implemented.

1. **Start with the owner's purpose.** Intake asks what supplied information contributes to current work. Full-note import preserves a reference under its existing contract. Useful-information intake is selective. An imported request is evidence that somebody asked; it is not proof that the owner agreed.
2. **Read related current notes before writing.** Find the relevant project/person/topic and inspect the current content, protection, and important source context. Similar wording alone is insufficient to merge identities. If relevant knowledge cannot be retrieved reliably, fixing retrieval has priority over forcing a taxonomy.
3. **Choose update, create, or leave alone.** Update an existing ordinary note when evidence changes or extends its subject. Create a new note when it has a distinct recurring purpose, a useful stable subject, or a reusable synthesis worth reading separately. Leave inputs without durable value in their current context. No required entity count or source-to-note ratio.
4. **Keep facts, reports, interpretation, and authority legible.** Attribute material assertions to identifiable evidence. Mark deductions and ambiguity. Preserve dates, scope, exceptions, quantities, and attribution when they change meaning. A generated page citing another generated page is not independent corroboration.
5. **Maintain a coherent current account.** Integrate new material into the note rather than accumulating unrelated bullet dumps. Preserve unresolved disagreements. Supersede or archive ordinary material only when justified and authorized; protection cannot be bypassed by manufacturing a conflicting “current” note elsewhere.
6. **Declare actual derivation inputs.** Bind a candidate to the current versions it used. After a newer observation, identify directly affected knowledge and refresh it through normal checked operations. A dependency marker states that refresh may be needed; it does not assert that all descendants were repaired.
7. **Save answers selectively.** A valuable analysis can become a maintained synthesis when the owner's request or delegation warrants saving it. Casual query answers should not all become nodes, and saving the model's conclusion must not promote it into an owner-confirmed decision.
8. **Validate structure deterministically and semantics separately.** Broken references, missing targets, malformed metadata, stale versions, and unauthorized writes can be checked mechanically. Source entailment, meaningful contradictions, and useful organization require evaluation and sometimes owner judgment.
9. **Make changed knowledge inspectable.** Explain the reason, evidence, and resulting change, with revision history and Undo. During review, approval must bind the displayed candidate. A semantic policy cannot relax those guarantees.

This fits the existing BRN contract's separation between model judgment and checked-service enforcement. The model chooses meaning and organization; the service checks mutation integrity and authority. [Runtime skill responsibilities](../architecture/threads-behavior.md#what-belongs-in-each-skill), [checked writes and authority](../architecture/invariants.md).

### Suggested note roles, without new fundamental node types

| Useful role | Example | Creation rule |
|---|---|---|
| Current subject account | “Project A: current delivery plan” | Maintain when multiple inputs change one ongoing subject. |
| Entity knowledge | “Mari: role and Project A context” | Separate when it serves repeated lookup or spans several contexts. |
| Concept/process explanation | “How we handle an exception request” | Separate when reusable understanding benefits from an independent page. |
| Comparison or synthesis | “Two release approaches and their tradeoffs” | Save when the analysis has durable value and remains tied to evidence. |
| Source-focused summary | “What this report claims” | Use only when the source's identity/argument itself matters. Do not mandate one per import. |

The roles can begin as writing conventions inside the existing Note model. Protection and confirmation should remain independent dimensions. A protected full reference can contain a false claim; an ordinary note can contain careful, well-supported knowledge. The current invariant explicitly says protection is not truth. [BRN protection and retrieval invariants](../architecture/invariants.md).

### The consequential difference: BRN does not archive every raw original

An external reference may later disappear, change, lose access, or become impossible to reconvert. A synthesis cannot promise reconstruction of material never retained. BRN's target deliberately keeps originals external, expires temporary payloads, and allows intentionally retained full notes/assets, excerpts, source metadata, and receipts. Do not add a hidden `raw/` archive to imitate a filesystem wiki. [Original-file and continuation contract](../architecture/invariants.md), [retained context rules](../architecture/threads-behavior.md#tools-authority-and-continuation).

Practical implications: cite a stable source identity plus observation/version when available; make coverage limitations visible; distinguish evidence that remains readable from evidence known only by reference; retain necessary excerpts or a full protected reference under the owner's chosen import intent. “Not observed to change” is weaker than “checked current.” External-source monitoring and a broader retention policy would be separate product decisions, not consequences of calling the notes a wiki.

## Risks the architecture label does not solve

**Compression loss.** A concise note can omit an exception, date, unit, denial, or minority position that becomes important later. Evaluate against the original available evidence, and preserve a path to detail.

**Error accumulation.** Repeated summarization can turn tentative claims into confident statements; later generated pages can amplify one early error. Keep source lineage and explicitly separate inference from extracted statements. Generated agreement must not count as additional independent evidence.

**Contradiction is not simply “newest wins.”** Different dates, definitions, scopes, speakers, and evidence quality may reconcile apparent conflict. A more recent email is not automatically authority over an owner's protected decision. Keep a question visible when the remaining choice is consequential.

**Graph growth can reduce usefulness.** One page per name or source can create duplicates and stubs, increase maintenance fan-out, and hide the useful current account. Count meaningful retrieval successes and owner corrections, not graph nodes or links.

**A larger prompt does not guarantee successful reading.** The controlled Lost in the Middle experiments show positional/context-length effects for the models studied. That is a reason to test context selection and source placement; it is not a numerical prediction about today's BRN model. [Lost in the Middle](https://arxiv.org/html/2307.03172v1).

**Input freshness is not world freshness.** Source hashes and local versions can establish which inputs a note used. They do not establish that an unchanged document is still correct. Source monitoring, obsolescence, and source deletion require explicit policy.

**Prompt text is not a write boundary.** Imported sources can contain instructions. Neither a wiki schema nor an imported note may grant tools, protected-write permission, provider choice, or access. BRN must retain host-owned authority and its checked service. [BRN authority invariants](../architecture/invariants.md).

## Evaluation appropriate for BRN

LongMemEval's five abilities—information extraction, multi-session reasoning, temporal reasoning, updates, and abstention—are a useful checklist. Its official data distinguishes small/medium/oracle histories and its retrieval scorer excludes 30 abstention instances. A “recall score” therefore cannot demonstrate correct abstention. LoCoMo supplies long, human-verified synthetic dialogues and multiple memory tasks, but it does not test BRN's protected-note/review/Undo contract. [LongMemEval paper](https://arxiv.org/html/2410.10813v1), [official dataset and scoring documentation](https://github.com/xiaowu0162/LongMemEval), [LoCoMo ACL paper](https://aclanthology.org/2024.acl-long.747/).

A later independent LoCoMo audit reports wrong answer keys and judge vulnerabilities. That is a primary report of the auditor's own work, not an established benchmark correction accepted here wholesale. It reinforces the need to inspect raw conversations and reviewed gold answers rather than trust a leaderboard. This research did not reproduce the audit. [Audit report and artifacts](https://github.com/dial481/locomo-audit/blob/main/AUDIT_REPORT.md).

Proposed experiment, using fresh explicit directories and synthetic/public fixtures:

| Test dimension | Required observation |
|---|---|
| Factual retention | Required names, dates, quantities, exceptions, and negative facts survive generation. |
| Original-source support | Claims and citations follow from supplied evidence, including source-to-note and note-to-answer steps. |
| Repeated subject intake | Relevant existing notes are updated; repeated sources do not create duplicates. |
| Synthesis | The note connects material across sources without inventing a relationship. |
| Temporal change | A new observation changes the current account when justified and preserves relevant history. |
| Unresolved disagreement | Conflicting sources retain attribution and qualification; the system abstains where needed. |
| Protected conflict | Ordinary maintenance preserves the protected record and presents the actual decision required. |
| Partial/missing evidence | Coverage gaps and unavailable originals remain visible; the system does not fill them with plausible text. |
| Navigation | The agent finds relevant ordinary and protected knowledge, including paraphrases and cross-note paths. |
| Source deletion/Undo | Affected knowledge is identified; stale derived material is not claimed repaired without evidence. |
| Human edit/concurrency | Dirty buffers, stale bases, and exact-candidate review continue to behave correctly. |
| Long-term maintenance | Repeated updates do not accumulate duplication, lost qualifications, or unsupported inference. |

Compare at least three conditions on the same ordered source stream: current BRN behavior; a bounded wiki editorial policy using the same current tools/model; and improved evidence retrieval without changing editorial policy. A combined policy-plus-retrieval condition can establish interaction. Avoid confounding richer prompts, extra tool rounds, and better retrieval with “wiki” as a single variable.

Freeze task/gold evidence before tuning; separate development examples from held-out cases; include temporal replacement and deletion rather than static questions only. Use repeated runs for model variability and blinded independent human review of a representative subset. Report each quality dimension separately so concision or attractive organization cannot conceal worse correctness. No BRN results for this comparison are currently established by this report.

Measure all stages: import/conversion, retrieval, input/output tokens, cache creation/read, generation, verification, refresh, failures/retries, and owner's review/correction time. Count storage and page growth too. A useful comparison is:

`Total effort = initial construction + subsequent updates + queries + validation + human correction.`

For stable corpora with frequent repeated questions, extra upfront synthesis may pay off. For high-churn or rarely queried inputs, it may not. There is no break-even query count if the wiki condition is more expensive both to build and to query. Establish the relevant numbers under BRN's actual provider and budgets instead of importing another paper's prices.

## What to adopt now, test next, and defer

**Adopt as the product model:** ordinary notes are current, evolving, source-backed working knowledge; protected/confirmed status remains distinct. Use existing notes as the first destination, create separate pages for durable retrieval value, preserve uncertainty and evidence, and make changes reviewable/recoverable.

**Test before changing runtime behavior:** clearer creation/merge/synthesis rules, source-support auditing, bounded structural health checks, and retrieval that can find relevant knowledge across phrasing. The current contract favors small packaged skills and scenario examples for semantic behavior, with deterministic integrity in the shared service. [BRN runtime-guidance contract](../architecture/threads-behavior.md#runtime-guidance-is-deliberately-small).

**Defer until demonstrated need:** a compulsory entity/concept/synthesis database ontology, every-source page generation, whole-corpus recompilation, broad automatic lint schedules, external-source monitoring, hidden raw retention, a graph database, and installing a third-party wiki compiler inside BRN. These add authority, cost, or maintenance questions not settled by the pattern.

The unresolved product choices are retention expectations, what should create a durable standalone note, how much maintenance to delegate, and how visibly to surface uncertain/stale knowledge. Research supports making those choices explicit. It does not justify promising an automatically correct wiki.
