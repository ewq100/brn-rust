**Verdict: APPROVED (source only).** I found no blocking defect in the actual `8ae8a729..d252a505` diff.

I made no edits and ran no Cargo, build, test, lint, mutation or provider commands. Principal acceptance, the mutation decision, live/hosted/native-human/owner/full-program/PDF qualification and integration remain separate and open.

**Pins and scope**
- The pin, tree, crates tree and three file hashes match `self-checks.json`, and the full diff SHA is `741597f4…`.
- HEAD `28fd322d` is docs-only and has the same crates tree.
- Only `threads.rs` and the two new files change. Nothing in core, app, CLI, UI, auth, summary, budget, schema, manifests or lock changed.
- The preserved before log shows 0/5 failing at behavioural assertions, not compile errors. The final log shows 430/0/1 and includes all 11 new tests.

**Requirements I checked in the source**
- **Legacy compatibility:** a summary-only call still persists byte-identically. Unknown fields were already rejected before D2.
- **Bounds:** objective 768, remaining work 8×256, questions 4×256, operations 16×256 and the 4096-byte total are all enforced in bytes before any effect. No new record type was added.
- **Operation identity:** a reference must be in a nonarchived Run of the same chat. ID prefixes and model claims are not trusted. The forged-prefix test is a real discriminator.
- **Operation states:** only a real receipt shows Applied, and a later owner edit leaves it Applied with `current_versions_match:false`. Allocated, Prepared, Stale, Retired and Missing stay distinct. An Applied state without a receipt fails closed.
- **Privacy:** hidden operations are omitted without IDs or counts, and no note text is returned.
- **Continuation context:** it is built once, under fresh liveness and change-token checks. If it doesn't fit the actual initial estimate, the mandatory input goes out unchanged with a visible gap notice. Reported text is JSON-escaped inside the UNTRUSTED block, so it can't pass as host state.
- **Empty-budget fallback:** it runs only after `finish`'s existing `live()` fence. It applies only to tool-limit or context-limit failures with empty progress, keeps only the owner's message ID and a 768-byte excerpt, and is skipped for writing runs.
- **No new authority:** `apply`, `withdraw` and `refresh` already check ownership across the whole chat, so carrying references grants nothing new.
- **Rig reuse:** no checkpoint store, replay engine or new driver was added.

**Non-blocking findings**
1. **Low — a bad optional field fails the whole run.** `threads_continuation.rs:24-44`, called from `threads.rs:397` (`parse_valid`), makes the run fail with `InvalidToolUse` (`threads.rs:3791`).
   - The schema's `maxLength` counts characters, but the host counts bytes. Blank items are also rejected without a `minLength`.
   - Non-ASCII text up to 256 characters can pass the provider's check, then fail the owner-started run. That failure kind doesn't trigger the objective fallback.
   - Legacy `summary` follows the same pattern, but the much smaller limits make it more likely to bite.
   - **Fix:** say "UTF-8 bytes, non-blank" in each field description, or return an is_error result at execution time (still rejecting before any effect).
2. **Low — carried references act as the current run's own operations elsewhere.** `save_progress` (`threads.rs:2002-2015`) adds them to `Run.operations`, which three other places read:
   - `finish` (`threads.rs:3473/3543`) and orphan recovery (`run_attention.rs:142`) can add a Review item for an older active candidate when the carrying run fails or is interrupted.
   - The CLI's `operations_total` (`brn/src/inspect.rs:443`) counts them.
   - The D1A history row lists them under the carrying run.
   - This is truthful and grants nothing, but it isn't documented or tested, and behaviour doc §44 says a run adds Review items for proposals "it left unapplied". **Fix:** document it (docs-only), or add a witness test.
3. **Low — extra full loads.** `report()` (`threads_continuation.rs:206-222`) loads every record and reads the change token twice, even for legacy calls with no operations. That adds a small spurious-rejection window that didn't exist before. `block()` also repeats `history_snapshot`'s full record and receipt load once per invocation. **Fix:** return early in `report()` when `operations` is empty.
4. **Test quality — one test half-checks its claim.** `retained_claims_and_operation_reference_never_inherit_a_protected_edit_grant` (`threads_continuation_tests.rs:610-707`) never asserts that the first run's `retain_progress` call succeeded or that the injected claim reached the fresh input. Its "not in system instructions" assertion can therefore pass without testing anything. The NeedsReview and unchanged-Note assertions are meaningful. **Fix:** assert the progress contains the claim and that it appears only inside the UNTRUSTED block.
5. **Docs — canonical behaviour not updated.** `threads-behavior.md` doesn't describe the structured `retain_progress` fields or the continuation block. A docs-only follow-up is allowed.

**Limits to keep explicit (as specified, not defects)**
- An applied operation that archived a record, or whose target was later archived, is omitted. Referencing it rejects the whole progress update, summary included.
- An Undo of an operation isn't shown; it stays Applied with `current_versions_match:false`.
- Prepared doesn't account for changed sources.
- An oversized first row makes the whole block "unavailable" instead of skipping that row.
- The block duplicates D1A's progress rendering and uses budget for it.
- The Missing state has no corrupt-store test.
- The bounded view can't prove that older work doesn't exist.

**Proposed mutation campaign (not run):** the 4 guards (chat ownership of operations, receipt versus current version, optional-input admission, fenced fallback) target the right risks, and ≤300 seconds including baseline and cleanup is proportionate. Running it needs an explicit principal grant.
