# INT-D history-only harness/preflight review, round 2 (Copilot Opus 5.5 high, Standards+Spec)

**Verdict: APPROVE with qualifications.** There are no blocking findings. B1 and M1 are both resolved within their allocated bounds. Preflight `79d8c613…` is technically ready for a separate principal activation decision. This review is not that activation, and grants no source acceptance or paid start.

I reviewed read-only and did not delegate. I ran no Cargo, tests, lint, scripts, Store or SQLite, Auth, network or Git mutation. My checks were Git reads, hashes, static reads and `ps`. Any check result below that I did not run myself is labelled "recorded".

## Identity (verified directly)
- **Checkout:** branch `work/int-d-20261017`, HEAD `bac93496…`, clean. `39a67e4f..HEAD` changes docs only.
- **Material `39a67e4f`:** tree `b7e1222c…` and crates `bdc0a22a…` both match.
- **Diff SHAs:** `b28f4c0c..pin` = `56dbcbe5…` and `db757..pin` = `6373b928…`, both match. I read both diffs in full, not just the responses.
- **What changed versus `b28f4c0c`:** only `crates/brn/tests/int_d.rs` and `crates/brn/tests/support/int_d.rs`.
  - `prepare-qualification.py` is unchanged.
  - The legacy `main()` prefixes of `observe-workflow.py` and `check-observer.py` are unchanged.
  - `metadata_copy_context` and the generator are unchanged.
  - Production, the lockfile and manifests are unchanged (334/334 inputs match).

## B1: RESOLVED
`context_input_allowed` (`support/int_d.rs:33-81`) admits only the manifest's exact chat and exact Project. It mirrors core `current_input_matches` (`brn-threads-core/src/store.rs:2679-2693`):
- **Chat input:** expectation is ChatContext. The historical Thread revision must exist, the current Thread must be unarchived, and both epoch and `private_context` must equal the current values. Membership in the current Project context is required.
- **Project input:** expectation is `Current`, the version must equal the live Project version, and the record must be a Project.
- **Unchanged:** the target, Message, Prepared, no-receipt, current-target, output and 1 MiB guards. No Historical/v1 rule is imposed on Messages.

The runtime positive (`int_d.rs:461`) uses the real normal CLI, Rig and the maintained loopback stub. It proves Thread revision 2 ≠ epoch 1 ≠ later current version 3, an exact Project `Current` input, a registered Run operation, the exact exported request, hash and content, no receipt, and an unchanged protected Note.

The before-red ran in a separate source root `d8503365` against the db757 exporter, byte-identical, and recorded exit 0/1, with the nested exporter failure counted once. The first fixture failure is correctly kept as a separate result.

## M1: RESOLVED within its stated limits
- Everything after the reservation goes through a `BaseException` boundary (`observe-workflow.py:481`) with a fixed class vocabulary (`:363`). The first cause is kept and secondary failures are appended.
- Each owned workflow and local child is reaped independently (`:483-500`). If any cleanup fails, the case is removed from `finished_cases` and `window_failed` is set.
- Failure-file and ledger writes are attempted independently (`:413-424`).
- An interrupt exits 130 only after STOP handling (`:512`).
- The fake matrix has 49 history cases (35 original + 14 new) and 10 legacy cases, matching the recorded counts. It includes canary-absence checks, next and retry refusal with no extra fake ask, and an owned-group survival check.

## Preflight `79d8c613…`
Independently re-hashed, every binding matches:
- all 334 inputs, against both the pin's Git blobs and the working bytes;
- all 3 scripts, against both the pin and the working bytes;
- the binary `1f66cf32…` and generator `ce2936b4…`, plus both `.d` files, with no uncovered workspace dependency;
- method and rubric, which equal the pinned docs;
- corpus, overlays and the check receipt;
- all 4 prompts, manifests, contexts and records;
- all four stores are MAIN-only with WAL and SHM absent.

Other facts I confirmed:
- **Order:** r1 control, r1 candidate, r2 candidate, r2 control.
- **Limits:** max 4 starts, 8 rounds, 120 s; no Stop, Continue or configure command in any row.
- **Arms:** control is Settings v4 with no binding; candidate is v6 with the binding on the case's own Project.
- **Prompts:** identical to the old ones apart from the IDs.
- **Preparation actions:** 39, all exit 0. Two generators, four reference-only `configure` calls with the exact argv, and four closed-copy interops where Rust equals Python in field order, with originals never opened.
- **Absent:** `activation.json`, `observations/` and every export path. No owned PID is alive and no cargo or brn process is running.
- **Old artifacts:** preflight `67e54848` and its four stores are byte-unchanged. The current gate would also refuse it structurally, because the observer and checker scripts and both test inputs have changed.

## Non-blocking findings (optional or deferred)
1. **Stale `int-d/plan.md:1` checkpoint (LOW-MED, docs).** It still says "prospective preflight 67e54848 are prepared offline", with no superseded or MUST-NOT-ACTIVATE marker. The gate refuses that preflight anyway, but the pointer should be updated to `79d8…`.
2. **Out-of-corpus inputs declared by the model become a technical STOP (LOW).** `check_scope` (`threads.rs:1473-1501`) admits any chat-visible input. If the model adds, for example, the Thread at `Current`, the ChatControl record, or the run's own new Messages, the exporter refuses and the whole window stops. The generic "export failure" clause covers this, but unlike the >1-target case it is not labelled as a model-behaviour outcome. Adding one method sentence is suggested.
3. **Some B1 guard branches have no meaningful negative test (LOW).**
   - The "+99" negatives (`support/int_d.rs:849,915`) only hit an identity guard (`:42`) that can never fire on the real export path.
   - The Note-as-chat and Note-as-Project negatives fail on Project context before the kind checks are reached.
   - Nothing negatively tests a changed epoch or privacy, or an archived chat.

   I read the code as correct, but deleting the `:70` check would not fail any test.
4. **The second ledger write is not truly independent (LOW).** `save` creates a `.stage` file with create-new (`:25`). If the first write fails after creating it, the retry hits FileExistsError deterministically. The run still fails closed through the reservation and the case order, and the fakes inject before disk, so they don't exercise this.
5. **Signal and interrupt windows (LOW).**
   - An interrupt during the reservation write or the gap before `try` escapes without a persisted STOP (still fail-closed).
   - SIGTERM, SIGHUP or SIGKILL to the observer skips `finally`. The workflow, in its own session, can then run on until the CLI's 120 s timeout.
   - Operator rule: treat any abnormal observer death as a window STOP and check `ps` for an orphaned `brn ask`.
6. **Smaller residuals.**
   - Residual 7 (PGID reuse) now also applies to already-reaped local children.
   - The class vocabulary is lossy: JSONDecodeError and UnicodeDecodeError become "Exception", though the stage still localises the failure.
   - The final test bytes were not re-run red; the substance is equivalent.
   - The B1 runtime test is macOS-only, like the existing int_d tests.
7. **Round-1 LOW items 1, 3–12** stay deferred or qualified exactly as listed in the response. Item 2 is now declared in method.md.

## Qualifications that remain
- The original three causes remain UNKNOWN, and the earlier windows are CLOSED.
- SDK 474/4/1 and widget 314/1/1 remain unexplained.
- Still open: PDF/pdftoppm, platform/hosted/native-human, compatibility, the strict Copilot route, and egress (there is no foreign canary).
- Auth HTTP and account details remain UNKNOWN.
- Still separate: INT-W, main, owner, program, I01 semantics, Stop/Continue, and owner live Apply.
- The reference `configure` calls neither authenticated nor read credentials.

This review makes no claim of remote Auth success, semantic superiority, or full-program acceptance.
