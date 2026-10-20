# Independent inactive-Project skill controls review

Source disposition: PASS; no actionable finding in the complete frozen candidate. Qualification claims below distinguish inspected source/tests from reported execution.

## Identity and complete scope

Worker `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`, branch `codex/inactive-project-skill-controls`, clean HEAD `28474ab705dceb718dedbdb96a027f2c5c36c371`. Direct parent and verified merge base `19d592c69223aa1f106e4c0f379dab868f13542b` (separate PR184 publication source). Entire binary diff SHA-256 `bb136a0f4f191f5d008db70fdf8742a428df650f756f66351a62422983ebb987`.

Only two changed paths: `crates/brn-desktop/src/threads_skills.rs` (+14/-3), `crates/brn-desktop/src/threads_skills_tests.rs` (+266). No service/core, provider/runtime instruction, schema, dependency, permission, CI or other source change. No re-review of unchanged draft/CI source.

Read the complete delta/functions and direct action/render consumers; relevant behavior contract at `docs/architecture/threads-behavior.md:208–216`, app skill service, core skill-policy validation/changes, exact package removal, UI refresh/scope handling and existing adjacent tests. The lead audit `evidence/skill-catalog-ui-audit.md` remains explicitly source-only at633; its inferred initial reproduction is not represented as an executed test. Lead current task facts retain the separate candidate/gate boundary and pending qualification/integration/native status.

## Requirements and correctness assessment

`threads_skills.rs:47–88`: inventory-only rows now use the existing Project stamp's archived flag for availability and provide an inactive-scope explanation. This agrees with `Workspace::scoped_skills`, which already marks bound rows unavailable, and Store's active-Project refusal for new enable/selection. General and active-Project inventory stay available; no canonical policy is copied or changed by projection.

`threads_skills.rs:323`: stale approved-digest rows no longer offer AdoptCurrent in inactive Projects. In an active/general scope the existing explicit current-digest adoption remains available and still does not select automatically.

`threads_skills.rs:352–358`: Remove has a distinct exact-current-package-digest condition instead of inheriting scoped availability. Thus an inactive Project inventory row can still remove the global installed package, while stale-approved/missing-package rows cannot remove a different identity. The callback continues to pass the displayed approved digest; it does not silently substitute the current package digest for Remove. The unchanged Workspace/package removal path independently verifies the exact identity and revokes matching enabled/selected bindings across scopes.

All actual actions were checked: Enable and composer Select use availability; AdoptCurrent is absent for inactive scope; Inspect is allowed and checks the captured current package digest; Disable remains possible without requiring current package bytes; None/Clear remains possible and only demotes selection. Scope-change rejection, explicit package refresh, delayed folder selection (installation only), current loaded-skill projection and non-macOS/Linux fallback remain unchanged.

The archived flag belongs to the same cached stamp as the rendered rows. Project ID/version/archived and policy participate in refresh invalidation; rows are hidden if their Project differs from current scope. Missing/non-Project scopes fail the checked service projection and render no actionable rows. Normal refresh reloads records/settings before refreshing skills. No newly introduced stale-scope permission path or unrequested authority change was found.

## Witness assessment

One added rendered-control test uses disposable local packages and a synthetic archived Project, including an inventory-only package and a stale approved binding. It clicks the inactive inventory Enable control and requires unchanged policy/notice, checks unavailable rows and absence of AdoptCurrent, inspects actual current package text, disables the stale binding preserving its approved digest, clicks stale Remove and requires no notice/package change, then removes the exact inventory package through the real rendered control and requires it absent. These are behavioral consequences through real controls and owner service, rather than mirrored condition assertions alone.

The single fixture is266 lines in the actual pin (larger than the earlier packet estimate), but the cases share setup and exercise distinct consequential consumers, particularly the removal regression caught during lead inspection. No quota-driven duplicate test or new test framework was introduced. Existing active/general inventory positive test, normal install/enable/select/disable/reopen test and active stale-digest adoption/scope-change test are unchanged. Clear/Select service guards and inactive demotion are supported by existing source/core witnesses; this new UI fixture does not claim to retest every service invariant.

## Checks and limits

Reviewer performed read-only Git/source/test/caller inspection, merge-base/diff identity computation and whitespace check (PASS). No Cargo/test execution, target write, Git mutation, native launch, account/provider/API action or delegation.

The inspected `/tmp/brn-inactive-skill-controls-28474ab7-artifact-manifest.json` pins this clean commit and matching full diff digest, reports meaningful pre-fix rendered-click failure, focused4 passed, desktop341 passed/0 failed/1 ignored, strict all-target Clippy/fmt/diff/ordinary native-ui build PASS, and `launched: false`. At this report checkpoint it is a summary manifest; original command/log/source receipts were requested but not yet supplied to this reviewer. Therefore these execution outcomes are author-reported here, not independently raw-log-verified or rerun. The old-source behavioral-red claim is plausible from the actual callback/test but not independently receipt-qualified by this report. Preserve and bind original outputs before claiming stronger proof. This evidence limit is not an actionable source defect.

No actual OS interaction, owner acceptance, hosted qualification, publication, integration or release of this skill candidate is established. PR184's independent draft source and hosted gates are separate. A later source change invalidates affected scope; lead owns proof archival, final local/hosted gates and integration.
