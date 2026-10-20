# Inactive Project skill controls — qualification and evidence limits

**Candidate:** `codex/inactive-project-skill-controls` at `28474ab705dceb718dedbdb96a027f2c5c36c371`, based on `19d592c69223aa1f106e4c0f379dab868f13542b`. The worktree was clean after commit. The final patch changes only `crates/brn-desktop/src/threads_skills.rs` and `crates/brn-desktop/src/threads_skills_tests.rs`.

## Behavior

The desktop now marks workspace-inventory-only package rows unavailable when the current Project is archived, explains that they cannot be enabled or selected there, and omits stale “Enable current version” actions in inactive Project scope. It reuses the archived bit already captured in `SkillStamp`. The checked service/store authorization is unchanged.

Removal has its own availability rule: exact current package bytes remain removable from workspace-wide inventory even while the current Project is inactive. Missing or stale bytes cannot be removed using an older displayed digest. The synthetic UI test also confirms inspection and disabling a retained stale binding still work, and that active/global controls and stale-current adoption remain covered by existing positive tests.

## Source binding

The base tree is `cfefa28782eafa8cd1db3fd06b333a9451813fa5`; the final tree is `a50370a20a83144f94da8ae43de4a459f9934007`. Base file hashes, final file hashes, and final patch SHA256 (`bb136a0f4f191f5d008db70fdf8742a428df650f756f66351a62422983ebb987`) are in `/tmp/brn-inactive-skill-controls-source-binding.json`. The exact final source/test patch is `/tmp/brn-inactive-skill-controls-final.patch`.

## Red/green evidence

The meaningful pre-fix counterfactual rendered-click run clicked the inactive inventory Enable control and produced the rejected Store notice `invalid request: enabled skill requires a current active Project`; the expected notice was unchanged. Canonical policy stayed unchanged. Its reconstructed receipt is `/tmp/brn-inactive-skill-controls-original-red.json` and `.log`.

This was not a clean checkout of the exact base commit: the test was an uncommitted synthetic regression and the temporary source reversion left an unused `inactive_project` local, which the compiler warning exposed. The temporary source/test tree was not frozen or hashed. The receipt labels this as a counterfactual old-behavior witness, not an exact-base build. Earlier failed assertions, a private-field diagnostic compile error, a callback type edit mistake, and the first formatting diff are retained in `/tmp/brn-inactive-skill-controls-original-attempts.json` and `.log`; the invalid disabled-state assertion is explicitly distinguished from product evidence.

The lead’s Remove feedback arrived before the final commit. The first approach would have disabled a valid exact-digest global Remove action along with scoped Enable. The final diff gives Remove a separate digest equality predicate and the final focused test covers exact Remove success and stale Remove no-op. No separate intermediate commit/hash was made; this limitation is recorded in `/tmp/brn-inactive-skill-controls-prepin-remove-feedback.json` and `.log`.

## Frozen-candidate checks

All final checks used Rust 1.98.1, `--locked --offline`, one job, and the designated target directory `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/target`.

- Focused skill UI suite: 4 passed, 0 failed.
- Full affected desktop suite: 341 passed, 0 failed, 1 ignored. The test listing in the active transcript was truncated, but the final summary line was visible.
- Strict all-target Clippy with `-D warnings`: passed. Cargo emitted only its future-compatibility notice for `block v0.1.6`.
- Workspace `cargo fmt --all -- --check`, `git diff --check`, and clean-tree check: passed.
- Ordinary `native-ui` build: passed. The app binary was not launched.

The exact executed command strings and observed output excerpts are in the `final-focused`, `final-full-desktop`, `final-clippy`, `final-format-diff`, and `final-native-build` JSON/log receipt pairs under `/tmp`. These are **reconstructed transcript receipts**, not raw terminal captures: the commands were not redirected to log files at execution time and wrapper exit codes were not retained. The full-suite listing was truncated. Only the later focused receipt repair was rerun; the other original checks remain transcript-backed.

## Artifact copies

The prior mutable-target manifest `/tmp/brn-inactive-skill-controls-28474ab7-artifact-manifest.json` was preserved unchanged (SHA256 `93929b2a25cb48d1dbb3e3d042040eb8cb79ac47cd5dbaf41a807bffdde08ce0`). Its app/helper paths were mutable, so hash-verified read-only copies and the final patch are now at `/Users/evokessler/.codex/artifacts/brn-inactive-skill-controls/28474ab7/`. The new read-only `artifact-manifest.json` SHA256 is `b3cda778f0f40e4239de78bbba0faddb35dcb590b4616378fe0f07a760be1b2e`.

The app copy SHA256 is `7cb86bff4f9fc69558d7ed2c2672bbe6bbde4bac71a71d2405c01f9335ba354d`; helper copies are `4f46c81f3e649b4f86bd4a66c109fc86889e3fefac83b8777d9956662e74a36b` and `24662cd03614003e42687d7ec070c3e219a0ccbf65d38c2bf1160457fbce9ccc`. The helper paths came from the supplied qualified desktop-suite record `/tmp/brn-draft-dismiss-scope-05-final-desktop-suite.json`; this task did not select or rebuild their source commits. All copy hashes and that provenance caveat are in the artifact manifest.

## Evidence limits

No native application was launched, no provider/account/live actions occurred, and no source changed after the final commit. Raw command log files and exact hashes for the temporary red tree and pre-Remove intermediate tree were unavailable. This report preserves those limits instead of presenting reconstructed excerpts as immutable original logs.

### Focused suite raw receipt repair

Because the earlier focused result had only transcript capture, the `skills_tests` suite was rerun on clean frozen commit `28474ab705dceb718dedbdb96a027f2c5c36c371`, without source changes, using `tee` and pipefail. This repeat exited 0 and passed 4/4. Raw stdout is `/tmp/brn-inactive-skill-controls-final-focused-repair-rerun.log` (SHA256 `5563178c77e23cc7be691ae0c86ee46f170b65bef4a0f8226822a19c89d53639`); exact command and exit status are in the matching JSON receipt. This does not convert the old red witness or other transcript-backed results into raw captures.

The later [permission repair](inactive-skill-artifact-permission-repair.json) preserves the original0444 file set and creates separate0555 app/helper copies with identical hashes and executable access. Use that source-bound executable set for any later native session; no launch occurred.
