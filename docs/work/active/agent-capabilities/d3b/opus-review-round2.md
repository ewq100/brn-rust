**Verdict: APPROVED** (source level only)

This approves the source at `90f50da0`. It is not merge, release, owner or full-program acceptance. I did not build, test, lint or run anything live, and I made no edits. My checks were read-only: `git` reads and hashing of the retained files.

## Target identity
- **Commit and trees:** commit `90f50da0`, tree `a1259450`, crates tree `9ea12cfd`. The worktree is clean.
- **Diff hashes:** both diff files match their stated SHAs. `git diff 6ff..90f` is byte-identical to `correction-actual.diff`, and `000ed..90f` hashes to `39e32404…`.
- **Qualified inputs:** all 274 hashes in `round1-qualified-wip.json` match the 90f blobs (0 mismatches). The later commit `9aed00ec` changes docs only and touches no material file.
- **Receipts:** I re-hashed every receipt, raw log and source receipt in both check indexes (18 normal, 6 before) and found 0 mismatches. Every log's `test result` totals equal the recorded counts. The only hash mismatch any receipt lists is `skill_policy.rs`, which is the disclosed redundant-borrow fix. The final core and lint receipts after that fix list none.
- **Before reds failed at the intended spots:**
  - Core fails with `Invalid("enabled skill requires a current active Project")` at the deselect step.
  - Native fails with "global owner inventory must remain available in another scope".
  - The AI 9/1 run failed on "instruction body required", as disclosed.

## Production change: exactly three hunks, as claimed
- **D1 — `brn-desktop/src/threads_skills.rs:69`.** Only the cross-scope suppression was removed. An inventory package with no row in the current scope gets a row with `project` set to the current scope and `enabled/selected:false`. Other-scope policy is not copied. The composer still lists only enabled rows (line 197), so the new rows appear only in the owner Settings list (line 248). Native actions keep their stale-scope guard. Confirmed.
- **D2 — `brn-threads-core/src/store.rs:2070–2078`.** A binding passes if it is unchanged from the old Settings, or if the new binding is unselected and an old selected binding has identical ID, digest, Project and enabled state.
  - The check runs only on enabled bindings, and selected implies enabled, so the clause cannot newly enable anything.
  - Unique scope/ID rules out a duplicate. Reselecting fails the exact-match check.
  - `Select{None}` only demotes bindings in the same project (lines 1311–1315). The cost is bounded by 128 bindings, so performance is fine.
  - No new permission. Confirmed.
- **Empty-identity skip — `brn-ai/src/threads.rs:3420–3422`.** `preflight` is pure (`threads_context_budget.rs:167`). Skipping `update_run` for an empty identity list also skips its `live()` call. That is safe: `run_prepared_over` (2503–2522) checks `live()` before any credential or provider access, and the watcher fences after that. The no-skill case behaves as 000ed did, and the legacy creation fence is unchanged. Confirmed.

## Test adequacy
- **Core:** the negatives start from enabled, unselected `b`. Each one is the demotion shape with exactly one of digest, ID or Project changed. Each refusal comes from `apply`, not `prepare`, and the record is asserted unchanged, which is stronger than the old `is_err()||!Applied`. The test also covers:
  - unchanged Configure plus replay;
  - deselect plus replay;
  - raw reselect refusal;
  - inactive enable and select refusal with the record unchanged;
  - disable, then Clear.
- **Native:** a real P1 enable+select, then general and P2 both show the package, the visible Enable control is checked, and enablement is independent. The P1 binding is preserved, and the P1 ID is absent from the general and P2 row JSON.
- **G1:**
  - Start 20 falls inside `Õ`. Both the direct `read_skill` (`text.get(start..)`) and the turn-level batch gate refuse it before `retain_progress` takes effect. The Run record is unchanged.
  - For pagination, the cursor comes from the production reader. History is checked to be append-only, start/end/next_start/total are exact, and the concatenated text equals the source. `complete` is false on the resumed page, which matches `start==0 && end==len`. Results fit, and exactly one identity is stored.

## Findings
No unresolved high-risk or blocking findings.

- **Nonblocking test preference:** in the range test, the multibyte character sits at offset 0, so the cut lands between ASCII bytes. A multibyte character at the cut would test boundary handling harder. Exact concatenation and the existing `fit_prefix` coverage mitigate this. No change required.
- **Docs:** the README, behavior and setup wording on portion admission, inactive deselection and built-in digest drift matches the source. Nonblocking items 1 and 4–7 remain as disclosed.

## Residual limits (not passed)
- Native-human and hosted runs were not executed, and only aarch64-darwin was exercised.
- No live run or mutation testing at 90f.
- Live results remain as recorded for 6ff:
  - Candidate 1 hit a generic preparation failure whose cause was not retained.
  - Neither candidate loaded the package reference checklist.
- The 900s mutation budget is closed: 3 caught, 1 timed out without a verdict, 3 not executed.
- The PDF `pdftoppm` failure is carried over.
- The 6ff full workspace, capability and offline qualifications were not rerun.
- D3a residuals remain.
