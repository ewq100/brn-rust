# Reference-search request ownership assessment

## Scope and source identity

Read-only assessment in `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`, branch `codex/import-target-owner`, HEAD `787ffbec3ca6362b9728810a628e2958f8a35e3f`; `git status --short` was clean. No source edit, Git mutation, build, test, network, account or native action was performed. The selected task record is `docs/work/active/vacation-campaign/campaign.json`; its campaign expires 2026-11-01 21:41:51 UTC. Applicable routing, architecture/invariants and `docs/development/strategic-programming.md` were read, together with the existing hidden-search retirement record.

## Finding

Recommend a small private owner for the reference-search request generation and cancellation handle. This is a narrow ownership improvement, not evidence of a new behavior defect. In `threads_ui.rs`, the same two request-lifecycle facts are independently updated together in `recheck_writing_setup`, `replace_writing_setup`, `search_writing_references`, `finish_reference_search`, and `WritingRequest::CloseSetup`. The state is initialized together too. A paired owner can centralize invalidation/completion ordering without taking ownership of setup drafts, visibility, pagination, or canonical search policy.

The current hidden-draft retirement fix in `docs/work/active/vacation-campaign/reference-search.md` is already integrated at 8431e70b. Preserve it: a matching reply retires `search_pending` on its exact retained draft even when hidden, while page publication remains limited to a currently displayed form. This assessment does not recommend redoing that fix or claim it remains broken.

## Existing responsibilities and consumers

- The pair is declared and initialized only on `ThreadsUi`. Production lifecycle consumers are the four methods/call sites above; direct test consumers in `threads_writing_tests.rs` observe the cancel handle and manually advance generation for controlled stale-reply witnesses.
- `finish_reference_search` currently performs two distinct gates: generation first, then captured `SetupTarget` plus `ViewChangeToken` against the retained setup. It clears the current cancel handle only after the generation gate, clears pending only on the matching captured draft, and publishes matches/cursor/count only when that form is displayed/current. Keep these gates and their order.
- Search admission validates the displayed form, selects the original query or saved query for cursor paging, clears old page values, marks pending, records the query, invalidates the previous request, then asks the checked workspace `search_handle()` for a scoped page. If handle acquisition fails, pending is cleared and the error is shown. This behavior and exact service scope belong in the UI/service path, not a generic request owner.
- `threads_search.rs` has its own `SearchView` and lane-level request IDs, tasks, cancellation, cursor history and shared first-page behavior; `LauncherSearch` has another lifecycle. Those owners implement different paging and publication contracts, so reusing them would add coupling. `ConnectionsView` has a generation/cancel/task/pending/result pattern, but its graph query lifecycle is likewise distinct. No reusable owner was found that safely captures this reference-form lifecycle.
- `CancelOnDrop` cancels when dropped; the ticket also cancels on drop. The slot should retain that established RAII behavior and add no independent cancellation flag.

## Smallest coherent API and states

Keep one private `ReferenceSearchRequest`-style slot with exactly the existing generation and `Option<CancelOnDrop>` fields. Give it operations equivalent to:

1. `invalidate()` — drop the prior cancel guard and advance the generation, returning the new token for a subsequently admitted search. Preserve the existing checked `u64 += 1` behavior unless a deliberate compatibility decision changes it.
2. `retain_cancel(token, guard)` — associate the returned guard with the current admitted generation (or accept the guard as part of a `begin` operation after the search ticket exists).
3. `finish(token) -> bool` — return false without touching a newer guard when stale; for the current token, drop/clear its guard and return true.

The conceptual states are idle/current generation and one admitted request at that generation. Invalidation transitions either state to idle at the next generation; a matching finish transitions admitted to idle; stale finish is a no-op. Handle-acquisition failure remains an idle new generation, with UI pending/error cleanup as today. No second active-request enum, duplicate generation, target/token copy, or public API is needed. If the implementation cannot make the generation/guard pairing materially harder to misuse than these three call sites, retaining the two fields is an acceptable lower-cost choice.

## Caller retirement map

- `recheck_writing_setup`: replace the paired clear/increment with one invalidation call before existing context checks.
- `replace_writing_setup`: use the same invalidation before clearing/reloading the form.
- `search_writing_references`: invalidate before service-handle acquisition so an older result cannot land even if acquisition fails; retain the new ticket's guard on that token after a ticket is created.
- `finish_reference_search`: use `finish(token)` as the first generation gate. Only a current token may clear the guard; keep exact target/observed matching, hidden-draft pending retirement, displayed-only page publication, and error handling in this method.
- `CloseSetup`: use the same invalidation operation.
- Initialization: default the owner once. Adjust tests that directly mutate the field pair to use the private owner API or a narrowly scoped helper; do not expose it publicly merely to preserve field-level test access.

The owner should not absorb `SetupDraft.search_pending`, query/cursor/matches/count/error, target and observed token, `writing_setup_form_current()`, checked service acquisition, or GPUI task management. Those are separate responsibilities and preserving them avoids accidentally converting displayed-only publication or changing pagination/error behavior.

## Useful existing witnesses

Existing test `b3_delayed_scoped_search_reply_cannot_publish_titles_or_counts_into_another_form` exercises a real admitted UI request, navigation with the draft retained, exact hidden-draft retirement, no stale matches/counts, retry, replaced-form protection against an older generation, and stale canonical visibility/configuration. Keep it as the principal lifecycle witness and adapt only its direct field access if needed. Existing writing-reference pagination coverage verifies first-page size/cursor and that Next continues the saved Alpha query after the input changes to Beta, then a new search uses Beta. Existing nearby writing setup tests exercise service failure/validation and retained-form behavior. No additional tests were run for this assessment.

## Cost, risks, and no-change option

The owner adds a small type and call indirection while removing repeated paired lifecycle knowledge from several call sites. Its main risk is a misleading abstraction that suggests it owns draft/result state; keep its name and API explicitly request-oriented, and keep the result gates visible in the UI. Tests must continue proving that stale finish neither clears a replacement guard nor clears the replacement draft's pending state. Generation overflow remains as today and is not a demonstrated practical issue here.

No change is defensible if extraction would merely move the two assignments without centralizing the invalidation/finish ordering or would force tests to inspect internals through awkward scaffolding. The current source is understandable, and strategic programming does not require an abstraction quota. Any implementation decision and verification remain for the lead; this report makes no source-change or qualification claim.

## Lead selection22October — bounded implementation

Select the small owner: retire the two loose UI fields and repeated generation/cancellation lifecycle updates. This centralizes one existing ordering rule; it does not establish a new behavior defect or introduce a general reducer framework. Keep a private `ReferenceSearchRequest` next to the existing local UI support types in `threads_ui.rs`, rather than coupling the writing form to the general/launcher paging owners or adding a public crate/API. Store exactly the existing `u64` generation and optional `CancelOnDrop`.

- `invalidate() -> u64`: clear the old guard, increment with existing `+= 1` behavior and return the new generation. Use it at recheck, replace, start and Close. Close previously wrote the pair in reverse order within one synchronous UI callback; there is no intermediate UI callback/publication between these statements. No overflow-policy change is selected.
- `retain_cancel(guard)`: retain the admitted ticket guard immediately after synchronous acquisition. No asynchronous gap or new generation parameter/duplicate identity is needed at this existing call site.
- `finish(generation) -> bool`: stale generation returns false and preserves the current guard; matching generation clears it and returns true, including an idle current generation exactly as before. Do not require an invented active state or change repeated-current-reply semantics.

Keep target/observed-token matching, hidden-draft pending retirement, displayed-only publication, query/paging/result/error state, service acquisition, scope and toolkit tasks in their existing owners. Do not absorb the form's `search_pending` flag. Tests may inspect the owner's private guard to observe the real admitted request; controlled stale-reply fixtures must use `invalidate()` instead of mutating generation. Retain and adapt the existing complete delayed-result and reference-pagination witnesses; no new getter/test-only production state or forwarding-only unit suite.

Base this isolated writer on the fully reviewed/locally qualified quote candidate `28fd581ea8451ecd95d899248132b62c8f890d03`, preserving the frozen import branch. This allows independent implementation/desktop checks while trace and quote integrations proceed; publish only after that prior source chain integrates and actual main is reconciled. One bounded Luna author, one consequential independent Sol candidate review, one shared build slot owned by lead. No account/native/network/provider/schema/model/config changes. Reassess after two failed approaches or any required behavior/authority expansion.
