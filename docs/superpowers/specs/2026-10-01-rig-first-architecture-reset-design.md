# BRN: Rig-first architecture reset

> **Superseded** on 2 October 2026 by the [simple Rig-based notes app specification](2026-10-02-simple-rig-notes-design.md). Retained as historical design input.

Date: 1 October 2026

Status: Three design sections and the written specification approved on 1 October 2026, including the revised credential-storage and distribution requirements. The user subsequently authorized preparation and commit of the [implementation plan pack](../../work/active/rig-first-reset/plan.md). Product execution, live authentication, model acquisition and release remain unauthorized.

Inspected repository baseline: `1a30db049b7ed82d285614e598bb95676cbf5ec2`.
Inspected Rig candidate: upstream tag `v0.43.0`, released 30 September 2026.

Integration update: `main@6323e53` subsequently added managed Markdown editing and store schema V6 through PR #13. Section 2 retains observations from the inspected baseline, not a claim that these existing capabilities remain unimplemented. The [reset plan](../../work/active/rig-first-reset/plan.md) requires baseline reconciliation before execution; the reset itself remains unimplemented.

This specification revises the user-supplied "BRN Rust --- Rig-First Architecture Reset" handoff. That handoff was supplied in conversation, not present in the inspected checkout. Repository source and pinned upstream source were inspected rather than treating its claims as evidence.

## 1. Decisions and scope

BRN owns product semantics, durable user work, note identity, permissions, provenance and file operations. Rig owns generic model execution and the agent loop.

| Decision | Approved scope |
| --- | --- |
| Subscription providers | ChatGPT/Codex and GitHub Copilot, both directly through Rig. |
| Provider fallback | None: no Codex App Server, Copilot CLI/SDK, alternate account or paid-API substitution. |
| Distribution | Distribution to other users is included; release remains blocked until both direct routes qualify. |
| Platforms | macOS first. Platform-neutral filesystem and credential interfaces support a later Windows port; Windows is not implemented or qualified by this reset. |
| Compatibility | No installed-base or Codex-thread compatibility requirement. Replace obsolete architecture rather than build permanent compatibility shims. |
| Saving | Explicit, basic editor-grade Markdown saving, with bounded local recovery and documented concurrent-writer limitations. |
| Credential persistence | Protected application-private local files using Rig auth. Keychain is deferred. |
| Context | Rig conversation mechanics plus BRN currentness enforcement; no centralized Context Engine and no automatic summarization initially. |
| Retrieval | FTS5/BM25, local embeddings, sqlite-vec and reciprocal-rank fusion, preserving exact provenance. |

No compatibility requirement is permission to delete an existing vault, credentials or unfinished work. Qualification uses synthetic fixtures and disposable directories. Old application directories are not silently rewritten or purged.

Excluded: Windows release, cross-device collaboration, lossless simultaneous editing, cloud sync qualification, arbitrary shell/SQL/file tools, publication, broad historical tools, MCP, graph retrieval and automatic note adoption.

## 2. Verified baseline and corrections

The following repository observations describe the inspected `1a30db0` baseline, not the later integration baseline or this target design. The [architecture overview](../../architecture/overview.md) describes current implementation:

- `brn-workflow::Workspace::ask_full` retrieves evidence before connecting to Codex App Server, persists local session/thread associations, and records known or uncertain execution outcomes.
- At the inspected baseline, `brn-store` owned imported content and immutable versions; the live Markdown registry/save design was approved but not implemented. PR #13 subsequently implemented that earlier design, separately from this reset.
- `brn-retrieval::Index::keyword` uses substring matches. Its `fuse` function already implements reciprocal-rank fusion; replacement retrieval should reuse that behavior and its provenance contracts.
- Native retrieval uses FastEmbed `7.1.0` and LanceDB `0.39.0`. Store pins rusqlite `0.40.2`.
- Rig `v0.43.0` includes direct ChatGPT subscription and Copilot implementations. This establishes source-level capability, not successful BRN authentication, account entitlement or distribution qualification.
- Rig's workspace uses rusqlite `0.32`, tokio-rusqlite `0.6` and FastEmbed `4.5`. Native SQLite/ONNX linkage and feature compatibility need actual combined dependency resolution. No combined resolution or product build was performed for this documentation task.
- Rig memory does not automatically provide BRN durability. Its append contract is not transactional or exactly-once; its in-memory backend does not survive restart.
- Rig hook scratchpads are in-process. Appended run entries are not disk acknowledgements.
- Rig's `IntoFilter` returns unfiltered history after a policy error. It must not enforce permissions or evidence currentness.

Registry package pages were available, but a registry API probe was refused. Planning must verify exact published package manifests and resolved features rather than assume the tag alone establishes the downstream graph.

## 3. Modules and interfaces

```text
brn-desktop ----+
               +--> brn-workflow --> brn-ai --> Rig --> selected subscription
brn CLI -------+          |
                          +--> brn-store
                          +--> brn-retrieval --> SQLite FTS5 / sqlite-vec
                          +--> private platform filesystem adapter
```

Desktop and CLI use the same workflow interface. Neither calls providers, retrieval SQL or filesystem mutations directly.

`brn-ai` is a small execution module: provider/model setup, selected-account authentication, Rig agent construction, streaming/event mapping and execution outcomes. Its interface exposes BRN run requests and outcomes, not Rig's complete type surface or a chain of provider-manager/completion-service wrappers.

`brn-workflow` owns durable operation identity, selected capabilities, evidence validation, candidate validation and safe file actions. `brn-store` owns transactional application records. `brn-retrieval` owns derived generations, passage search and provenance, not authoritative note content.

Use Rig companion crates when their graph and behavior fit. If rig-sqlite or rig-fastembed cannot coexist cleanly with BRN, prefer a narrowly scoped adapter to Rig's interfaces over downgrading unrelated storage solely to accommodate a companion crate. Do not replace Rig's agent loop.

Keep filesystem and credential details private behind small interfaces. Expose save/conflict/uncertain outcomes rather than macOS syscall results. Do not implement speculative Windows adapters; qualify the pinned desktop stack, Windows file replacement and credential handling during that later port.

## 4. Authority, eligibility and archive

| Data | Authority |
| --- | --- |
| Current saved note bytes | Registered Markdown file in the selected vault. |
| Note identity, location, lifecycle and exact-version permission | SQLite registry. |
| Unfinished buffers, submitted save inputs and operation outcomes | SQLite recovery/application records. |
| Retained original revisions, comments and citations | Immutable application records with original provenance. |
| Search snapshots, embeddings and vector rows | Rebuildable derived data. |
| Conversations and execution records | Durable BRN records; Rig messages are their execution representation. |

Active, archive and history are distinct. Archive retires a whole note; history retains earlier revisions. Neither participates in normal retrieval. Existing dated or similarly named files are not automatically classified as obsolete.

An explicit lifecycle/exclusion configuration defines active versus archived content. Normal tools cannot include archive roots, historical rows, recovery artifacts, unfinished drafts or unrelated candidates. Changes to lifecycle invalidate current eligibility immediately, independently of index cleanup.

One workflow predicate governs listing for AI, reads, indexing, search, memory handoff and provider submission: the note is active, permitted for the exact current content, freshly validated at its reconciled location, and has no unresolved save affecting its current state.

Preserve the existing 1 MiB note/editor byte limit and exact UTF-8, BOM, line endings and frontmatter. Opening does not grant permission. A content-changing save or observed external change invalidates previous version-bound permission. A stale index never grants eligibility; missing approval or indexing produces an explicit reapproval/reindex-needed state.

Revalidate selected evidence before provider submission and at answer completion. Streaming output is provisional. If evidence becomes stale during a turn, retain the response with stale/currentness information rather than acknowledge a successful current answer. Already-sent content cannot be unsent.

## 5. Basic Markdown saving

This section supersedes the stronger save protocol in the [earlier note-editing specification](2026-10-01-open-and-safely-edit-markdown-notes-design.md) for this reset. Its prepared [implementation plan](../../work/active/markdown-note-editing/plan.md) must not execute unchanged.

### Workflow

1. Open a validated, bounded regular Markdown file in the selected vault and retain its exact baseline and identity.
2. Recover the editing buffer locally. Distinguish "Recoverable in BRN" from "Saved to Markdown"; a failed recovery acknowledgement does not establish protection.
3. On explicit Save, durably retain the submitted generation, baseline, bytes and operation identity before mutation.
4. Prepare an operation-owned temporary file in the destination directory. Write exact bytes, preserve supported destination attributes and complete the required flush operations; surface failures.
5. Recheck destination existence, identity and exact bytes against the opened baseline. An observed change/deletion or unsupported target stops the save.
6. Atomically replace the destination, verify the observed result and record its known or uncertain outcome. No in-place truncating write or silent non-atomic fallback.
7. Acknowledge only the submitted generation. Later typing remains dirty/recoverable and cannot be overwritten by a late result.

Unchanged saves revalidate without rewriting the file. Retain exclusive application-data ownership and serialize BRN's original-path saves per note. Paths must remain within validated vault locations; links or unsupported file types cannot bypass these checks. Recovery/temp files are excluded from all current-content surfaces.

Conflict actions are read-only comparison, explicitly confirmed reload/discard or save to a separately chosen copy. A copy must not overwrite an occupied destination and does not inherit search permission. There is no force-overwrite or automatic merge.

Keep one rolling editing buffer and the latest completed save's baseline/submission recovery pair per note. Retain unresolved inputs without automatic expiry. Compact operation receipts preserve payload binding and prevent blind replays.

After interruption, observe and reconcile before another original-path write. If execution cannot be established, report uncertain, preserve recovery data and do not repeat the write merely because bytes happen to match.

### Explicit limits

Atomic replacement prevents a partial-file write; it is not atomic compare-and-swap against an external editor. Another writer can change or delete the destination after the final check. Its changes can be overwritten, or a deleted path can be recreated.

The reset does not require NSFileCoordinator/NSFilePresenter, atomic exchange with displaced-file retention, or the earlier full coordination protocol. It does not promise lossless concurrent editing, preservation of every racing writer's bytes or universal power-loss durability.

Attribute/durability/bookkeeping failures after possible replacement are uncertain outcomes, not proof of no effect. Best-effort observations cannot be advertised as stronger filesystem guarantees.

## 6. Agent tools, conversations and candidates

Use Rig agents directly. Instructions teach tool use; deterministic workflows enforce permissions.

The narrow target tools are `search_vault`, `read_note`, `list_notes`, `list_comments`, `read_comment` and `create_revision_candidate`. Read tools ship with the first agent slice. Candidate creation is enabled only when its separate validation/persistence slice qualifies.

Comment tools require an explicitly selected review context and its permitted sources. Original comment anchors remain labeled provenance; they cannot inject superseded quotes into ordinary current-vault conversations. Selecting a review context grants neither unrestricted archive access nor permission to save.

Do not expose `save_note`, arbitrary shell, raw SQL, unrestricted paths, arbitrary file writes or a model-controlled CLI bypass. Manual adoption/save crosses the same workflow as desktop/CLI actions; generated output is never approval or publication.

BRN owns conversation IDs and durable ordered records: provider/model/account association per turn, questions/messages, tool calls/results, evidence references, usage when supplied and execution outcome. Provider-native thread IDs are not identity. Provider/account changes require explicit selection and permission to send retained context.

Provide a BRN-backed implementation of Rig's memory interface or explicit history loading, with operation-aware durable commits. Do not equate a successful model response, a successful Rig memory append and a committed BRN turn. Preserve partial/failed/interrupted runs independently of the successful-history append contract.

Before current-mode continuation, validate application-supplied note evidence in retained tool results, answers and derived context. Stale evidence blocks continuation and requires a fresh conversation. Preserve the old one for labeled historical viewing; truncation must not hide dependencies in a way that revives stale knowledge.

Start with Rig's history-window mechanics, without automatic summarization. Currentness enforcement must fail closed before model dispatch; do not use `IntoFilter` as its implementation. No separate Context Planner is required.

Rig hooks observe/steer execution and connect to BRN operation recording. They do not replace durable journaling. Bound agent/tool execution and output budgets; never automatically retry a potentially effectful call whose prior outcome is uncertain. Local cancellation is distinct from confirmed remote interruption.

Typed revision proposals identify exact base revision/hash, addressed comments, half-open UTF-8 byte edits with replacements/rationales, and unresolved comments. Validate character boundaries, non-overlap, exact base bytes, comment ownership and operation payload identity. Persist valid proposals as separate candidates; malformed, stale or ambiguous proposals fail explicitly.

## 7. Provider selection and protected file credentials

Select provider, model and account explicitly. Capabilities are qualified per provider/model pair; unsupported requested behavior reports unavailable rather than switching provider/model/account.

Reuse Rig OAuth rather than invent another authentication protocol. Explicit Connect permits device login; ordinary execution may refresh the selected credentials but must return reconnect-required instead of silently starting interactive login.

Rig v0.43.0 accepts caller-configured cache paths. ChatGPT's OAuth cache includes access/refresh tokens, optional ID token, expiry and account ID. Copilot uses a GitHub-token text file and a JSON session-token cache. Passing no paths disables that file persistence. Environment-token clients are a separate mechanism, not automatic OAuth login.

The inspected cache writer uses ordinary filesystem writes, not encryption, Keychain or explicit owner-only permissions. BRN must establish and verify an owner-only application-private directory and files outside the vault, database, logs, fixtures and distribution artifacts. Reject unsafe existing paths/permissions and unintended credential sources.

Qualification must cover restart reuse, refresh/revocation failures, concurrent/partial cache writes, account continuity and secret redaction. Reuse the inspected implementation and add only required narrowly scoped protections/upstream fixes. Do not assume safe persistence or automatic refresh on every completion.

Disconnect removes BRN's local selected credentials and stops their reuse. It does not claim universal remote revocation or sign-out from other applications. Tokens, device codes and raw auth/error payloads must not enter ordinary logs or committed cassettes.

Keychain is later work, not a release requirement here.

## 8. Retrieval and embeddings

Replace substring scoring with FTS5/BM25. Treat queries as data, not caller-supplied SQL. Preserve note/revision identity, source hash, exact passage bytes and ranges.

Use one stable local embedding model independently of conversational provider. Its identity includes model/artifact hashes, tokenizer, pooling/normalization, dimension and relevant implementation identity. Provider selection alone must not rebuild the semantic index; incompatible embedding identity does.

Prefer Rig's local embedding integration when compatible. Preserve explicit model-acquisition authorization and verified local assets; do not adopt automatic-download defaults silently.

Use sqlite-vec with eligibility filters and BRN revalidation. Rig's SQLite integration is a vector adapter, not a replacement for brn-store. Indexed scalar filters and post-filter/exhaustive-search behavior must be included in correctness and performance qualification.

Reuse existing reciprocal-rank fusion and deterministic tie-breaking. Evaluate a reranker only after evidence demonstrates a useful quality gain.

SQLite is the common data technology, not a mandate to let retrieval mutate authoritative tables or to use a single physical database file. Separate rebuildable retrieval storage is acceptable.

Before removing LanceDB, compare relevance, filtered-search correctness, warm-query latency, indexing time, memory and disk use on the same fixtures, local model and named target Mac. Freeze corpus, relevance judgments and workload before comparison; include both small and intended maximum personal-vault workloads. Accept replacement only with no unexplained quality/performance regression or an explicitly reviewed trade-off. A successful toy query is insufficient.

## 9. Qualification and delivery sequence

| Slice | Outcome and prerequisites |
| --- | --- |
| 1. Qualification and proposed architecture | Disposable harness; exact Rig packages/features; combined default/native desktop dependency resolution; both subscriptions and protected credentials; deterministic fixtures. Product replacement waits for this evidence. |
| 2. Managed Markdown and FTS5 | Basic saving, registry/eligibility, archive exclusion, current snapshots and exact lexical provenance. No dependency on live provider calls. |
| 3. Complete Rig ask flow | Thin brn-ai, explicit configuration, durable conversations, guarded memory, read tools, streaming/cancellation and failure/redaction tests together. Depends on qualified Rig/provider graph and current-note workflows. |
| 4. Semantic/hybrid retrieval | Verified local model, sqlite-vec, rebuildable generations, filtered benchmarks and reused RRF. Depends on registry/eligibility and qualified dependency choices. |
| 5. Revision candidates | Typed output, exact-base validation, replay-safe separate candidates and manual adoption interface. Depends on stable agent/tool/conversation and note interfaces. |
| 6. Retirement and distribution | Remove App Server after replacement qualification; remove LanceDB after semantic acceptance; update contracts and verify signed/notarized clean-Mac installation and restart. Depends on the delivered target features. |

Provider/account integration and FTS5 work can proceed independently after shared types/dependency choices are settled. Do not split durable conversations, initial read tools and execution-error honesty into disconnected late follow-ups. Cancellation, interruption, privacy and deterministic testing start in qualification, not a final observability PR.

Retire Codex process/thread/config/packaging assumptions completely in the resulting implementation. Historical experiments and evidence may remain clearly historical. No compatibility layer is a target deliverable; intermediate replacement work does not imply automatic deletion of existing data.

Implementation plans must name exact files, checks, prerequisites and acceptance evidence after qualification choices are known. This specification does not execute the earlier plan or authorize account actions.

## 10. Acceptance and evidence

Separate implemented, deterministically verified, live-qualified, accepted and distributable states.

Qualification for each subscription covers authentication, authenticated completion, streaming, multi-turn use, restart credential/conversation reuse, tools, typed output, hooks, refresh/revocation, reconnect, network loss and cancellation. Cassettes/replay use synthetic inputs and are checked for secrets. Record the selected model, capabilities, dependencies, platform and actual results.

Distribution requires both direct subscriptions, explicit selection without fallback, protected credential persistence, durable/replay-safe BRN outcomes, current-only tool and memory context, FTS5/sqlite-vec provenance, accepted local retrieval results, candidate separation and clean-Mac packaging.

Assess applicable usage terms, documented integration support and unresolved direct-route distribution risks separately from technical results. No special OpenAI/GitHub approval process has been established or mandated. Vendor confirmation is one possible evidence source, not a compulsory email/application gate. Unresolved material distribution risks must be reviewed explicitly rather than called resolved because a request succeeded.

Both subscriptions are mandatory: if either remains unqualified, distribution is blocked. No vendor-runtime or paid-API fallback is authorized.

Tests must exercise stale evidence/memory, archive exclusions, save generations/conflicts/interruption, uncertain writes/tool effects, malformed candidates, unsafe cache paths and redaction. Use deterministic failures and process-crash tests where durability claims depend on interruption behavior. Fixture replay is not fresh account qualification; builds are not native usability evidence.

The reset can be implemented and deterministically verified while release remains blocked. No live provider call, original-vault access, credential inspection, model download, merge or release was performed for this specification.

## 11. Supersession and documentation

The earlier Markdown-first direction remains: current files, bounded working state, exact provenance, archive/history exclusion and explicit adoption.

For this reset, the earlier note-editing specification's coordination/exchange requirements are replaced by section 5. Its implementation subsequently landed through PR #13; retained historical observations are not rewritten. Reconcile the reset plan with that implemented baseline before execution, without changing current saving behavior in this documentation merge.

Current architecture/invariants still describe implemented SQLite/App Server behavior until production changes land. Update them at implementation checkpoints, not by declaring an unimplemented reset complete. Retain credential/account continuity and operation honesty as product requirements while retiring Codex-specific representations.

## 12. Primary-source references

- [Rig v0.43.0 release](https://github.com/0xPlaygrounds/rig/releases/tag/v0.43.0) and [workspace dependency manifest](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/Cargo.toml).
- [ChatGPT subscription dialect](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-core/src/providers/chatgpt/mod.rs), [OAuth interface](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-core/src/providers/chatgpt/auth/mod.rs) and [native implementation](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-core/src/providers/chatgpt/auth/native.rs).
- [Copilot provider](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-core/src/providers/copilot/mod.rs), [OAuth interface](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-core/src/providers/copilot/auth/mod.rs) and [native implementation](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-core/src/providers/copilot/auth/native.rs).
- [Shared cache writer](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-core/src/providers/internal/device_auth.rs).
- [Memory durability contract](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-core/src/memory.rs), [memory policies](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-memory/src/lib.rs) and [hook/run-state contract](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-agent/src/agent/hook.rs).
- [SQLite adapter](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-sqlite/src/lib.rs), [FastEmbed adapter](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-fastembed/src/lib.rs) and [cassette infrastructure](https://github.com/0xPlaygrounds/rig/blob/v0.43.0/crates/rig-cassette/README.md).
- [OpenAI authentication](https://developers.openai.com/codex/auth) and [App Server integration guidance](https://developers.openai.com/codex/app-server); these do not by themselves establish equivalent support for direct Rig calls.
- [GitHub SDK authentication guidance](https://docs.github.com/en/copilot/how-tos/copilot-sdk/auth/authenticate) and [SDK architecture](https://github.com/github/copilot-sdk); these document a different route, which this reset explicitly excludes.
