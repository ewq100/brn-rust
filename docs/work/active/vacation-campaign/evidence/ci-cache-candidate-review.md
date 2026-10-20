# Independent complete CI cache/resource candidate review

Verdict: **one actionable current-task record finding; no CI source correctness finding**. The source changes meet the selected bounded resource/cache policy. Lead has acknowledged the record finding as valid/fix; correction and targeted reconciliation verification are pending.

## Exact candidate and scope

- Checkout `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, branch `codex/agent-first-followthrough`.
- Complete candidate/observed HEAD `1938a849ea23104bbb004171917ff499e83dcd3a`; checkout clean including untracked files at initial and final inspection.
- Base and independently verified merge base `bc8a08e64b0dc1e2b33ff181dfdb54b62f59f5e4`.
- Complete actual inventory: 31 paths. The only source/config changes are `.github/workflows/ci.yml` (MacCore timeout/comment) and `.github/actions/setup-rust/action.yml` (main-only cache-save input/comment); remaining paths are task/current overview, investigation/evidence/integration records and archived raw evidence. No Rust, tests, dependency, toolchain, helper/generator, preview-release, permission, protection or runtime-prompt change is hidden in the diff.
- Timeout source exactly matches independently reviewed `fe65b8ec7f1be643108bb25fbbb5033a67683a76`; both changed CI files exactly match selected source `4b50fc9684ad98236ac74b41f3ff5c0e96a77c8d`.

Applied previously loaded reviewing-product-candidates/resource guidance and repository workflow/verification/invariant boundaries. Reviewed requirements and correctness together. No delegation, builds, network/account/Actions/native/Page/PR operations, cache mutations or Git mutations occurred. The only output write is this report.

## Finding

**[P2] Reconcile completed prerequisites in the current MacCore task.** Location: `docs/work/active/vacation-campaign/campaign.json:808–811` (also integration text at line 822), projected into `docs/work/overview.md` under “Allow the complete Mac Core check to finish within a bounded job”.

The newly added current task still directs the next owner to wait for PR182 normal integration and says `PR182 integration sequence; original-main bounded retry running.` This same complete candidate records PR182 normally merged to `bc8a08e6` at 11:29:57Z and the original-main retry passed at 11:14:19Z; the 11:37 current record already uses these facts in adjacent tasks. The old task observation time does not make an obsolete prerequisite useful in the active task/Needs-attention projection. A resuming owner or scheduler can wait/recheck an already satisfied condition instead of following the composed candidate's actual review/publication/hosted gates.

Update this current task's next action, resource waits, integration text and observation to the known composed-candidate state, then regenerate the overview. Preserve the dated timeout qualification/review and original cancellation/retry receipts unchanged. Lead disposition received during review: **valid/fix**, correction pending. This is the sole finding; no source change is requested.

## Source and all callers

Cache switch at `.github/actions/setup-rust/action.yml:54–55`: `${{ github.ref == 'refs/heads/main' }}` is a boolean expression passed to the pinned cache action's `save-if` input. The archived pinned `action.yml` defines false as restore-only. `src/save.ts` lowercases the input string and returns unless it equals `true`, before cleanup/save; the nonempty string `false` does not fall back to its default. `src/restore.ts` does not consult `save-if` and retains cache key/fallback restore behavior. The source therefore suppresses future non-main uploads without disabling restores or deleting existing caches. Existing main cache-on-failure eligibility is preserved, not silently changed to success-only.

Caller inventory: the three CI lanes (`headless`, `native-ui`, `native-retrieval`) and the separately inspected `preview-release.yml` caller. PR refs are restore-only; main push/manual-main jobs remain save-eligible, and manual other-branch runs are restore-only. The preview workflow is triggered by main `workflow_run` and calls setup only after its existing exact-source/approval/four-required-check eligibility gate. It retains its distinct `preview-release-native-ui` lane and main-ref save eligibility. Its source checkout, gates, packaging, publishing, permissions and timeout are unchanged. Review performed no release execution or authorization.

Whole-document Ruby standard YAML comparison passed for both changed files: after restoring the single timeout scalar and removing the single `save-if` scalar in memory, parsed candidates equal parsed bases exactly. Thus OS/architecture/lane/compiler/profile/lock-related key dimensions, cache action pin, restore configuration, generated-source removal and admission guards, every step/test/condition/context/failure rule, and other platform limits remain unchanged. The read-only helper/generator inspection confirms separate stock/adapted targets, source-root association, exact patches/source/engine/lock admission and regeneration remain intact.

MacCore expression selects numeric 60 only for actual matrix `macos-15`; Ubuntu/Windows stay 30. Documentation stays 15, native lanes 60, and preview stays 90. Job-level allowance includes finalization, adds at most 15 minutes to MacCore's configured bound and does not guarantee all future runs fit. No test skip, `continue-on-error`, retry policy, context removal or protection bypass is introduced.

The cache tradeoff is accurately stated: PRs with new dependency/profile keys cannot seed their own future reruns and may remain cold until eligible main cache population. Existing PR archives remain. No future measured speedup, proven eviction cause, paid storage change, broad helper cache or cross-engine target sharing is claimed/selected.

## Evidence validation

- Independently verified all ten archives listed by `ci-cache-investigation.json` against compressed hashes, decompressed hashes and original byte counts; verified the eight-file pinned-source archive hash `72fe615962596a99d9624cb16a1c599376ccda6f3845cd5295a35f0e3b3dff92`. Read the relevant pinned action implementation. This is inspection of supplied archived pinned source, not a new upstream fetch or hosted execution.
- Published cache-cause report exactly equals archived v3; compile report exactly equals its archived original. Earlier cause-report versions remain preserved as superseded evidence. Final source/log-based corrections explicitly distinguish configured root-workspace lock discovery from the README's broader description.
- Logs show the same final key available at 08:22:38Z, absent at 09:29:54Z and PR182 at 09:46:22Z, then restored by the retry at 10:59:57Z. This supports availability change, not its mechanism. Original cancelled archive is also byte-identical to the separately retained timeout log archive.
- Retry JSON binds `20fcd738` to successful MacCore job118591050475, 10:59:33Z–11:14:19Z = 886 seconds. Original receipt binds the same source/job118555511333 to 2708 seconds and the explicit 45m timeout annotation. Helper preparation 899→243 seconds and test/lint 610→203 seconds agree with step receipts. Raw test output supports about 1.8 seconds of cold helper bodies; compile/check work dominates. One differing-runner pair is correctly not presented as a controlled benchmark or qualification of the new source.
- Inventory independently sums to 8,936,986,355 bytes: PR refs 5,886,330,190; main refs 3,050,656,165, six entries. It shows duplicate scoped storage, not historical quota pressure or an eviction audit trail.
- Lock comparison independently confirms 136 helper name/version tuples and 82 names absent from root lock; stock/adapted each have 409 package identities with 408 shared, differing in Xberg registry/path source. Pinned root-metadata keep-set and recursive target cleanup support helper-output pruning. No missing-name count is represented as a measured deleted-artifact count.
- Premerge full-snapshot archive matches its hash/length receipt. Its main SHA, PR head, live required-context list, and successful four preceding-main plus four PR-head checks agree with concise `pr182-premerge.json`. PR182 integration records normal merge to base `bc8a08e6`; resulting-main39415372292 remains pending. Original main20f cancellation is preserved alongside retry success, without attributing source repair to rerun.
- Reviewed all other changed records: current/overview/evidence routes, project/Trash bounded audit, timeout qualification/prior review, PR181 recovery reconciliation, PR182 chronology and remote update/readback receipts. Remote updates remain dated projections with rendering limitations. Aside from the located current-task wait, no unsupported success, acceptance, live state or future-performance claim was found.

## Proof limits and next action

Read-only Git/source, YAML structure, archive/hash/log/receipt and lock comparison only. Standard YAML parsing is not GitHub-hosted expression validation. Existing checks reported by the lead (overview, Markdown 611 files/2872 links, whitespace) were not rerun by the reviewer; no Rust rerun was needed for unchanged Rust source. No new live cache listing, protection inspection, Page readback or account request occurred.

Fix the one current-record discrepancy, regenerate/check and pin the corrected candidate; request targeted verification of that reconciliation. CI source may remain unchanged. Hosted exact-head and resulting-main gates, integration, native observation and owner acceptance remain separate and lead-owned. No passing hosted result for this cache/time-budget candidate is inferred from old-source retry success.
