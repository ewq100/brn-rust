# Development maintenance batch integration review

**Exact reviewed base:** actual main207 **`8c18845ca5b70a43d36b0bc0d2811da573086a79`**.

**Exact reviewed candidate:** clean **`7dde6a41a4fbc66468e0400d2cc3c859f04c8ef5`**, branch `codex/backup-process-witness-bounds`, in `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`. Verified actual branch/HEAD, clean initial and final working/staged/untracked status, base ancestry, the complete eight-file diff and local merge history. During the overlapping lead gates, one intermediate status observed untracked `crates/brn-threads-core/.tmpqpmHNz/`; it was not read or removed by this reviewer, and the immediately subsequent final status was clean with no working/staged source diff. This transient path is not being accepted as a product source addition. Candidate consists of normal local merges: base main207 into the backup maintenance branch, then approved task-route component `cadcec0373e4542ffb1e43a8152bba343e142f8f`. Complete diff against actual main207: **eight files, 223 insertions and98 deletions**. No further source edits or dirty overlay are included.

Attribution: independent read-only integration review by the separately assigned reviewer agent, who authored neither component nor the integration. Prior independent component reviews were performed by this same reviewer and are explicitly reused below after exact-content verification. No tests, builds, renderer/CLI execution, network/account/native action, source edits or nested helpers were performed. This report is the sole write.

## Verdict

**Approved at exact batch `7dde6a41a4fbc66468e0400d2cc3c859f04c8ef5`, with no actionable findings.** The actual merged candidate is the union of the two approved maintenance components with no additional source resolution or interaction that changes their scope. A single lead-owned qualification/PR cycle is reasonable for this selected integration; it does not merge away each outcome's evidence or boundaries.

## Complete file/content verification and explicit reuse

The actual base-to-batch changed-file inventory is exactly:

- `crates/brn-threads-core/src/backup.rs`
- `crates/brn-threads-core/src/restore.rs`
- `crates/brn-threads-core/src/lib.rs`
- new `crates/brn-threads-core/src/process_test.rs`
- `scripts/render-work-overview.py`
- `scripts/tests/test_work_overview.py`
- `docs/development/workflow.md`
- generated `docs/work/overview.md`

Every actual changed file and its full base-to-batch diff was inspected. Read-only Git comparisons returned exact empty differences between the batch and approved **`dc16973f711828ae8fc3c7923ab150bd1091f963`** for all four Rust files, and between the batch and approved **`cadcec0373e4542ffb1e43a8152bba343e142f8f`** for all four task-route files. These compare entire file contents, not merely changed hunks or parent commit messages. The complete actual changed-file set excludes production Project code/tests/contracts, campaign JSON, dependencies, toolchain, other documentation and runtime code. Thus main207's Project change is retained in the base and no unexpected file is lost or modified by composition.

Reused complete source reviews:

1. `/tmp/brn-backup-process-bounds-review.md`, exact approved `dc16973f711828ae8fc3c7923ab150bd1091f963`: shared Unix test-only ownership, readiness bounds, actual SIGKILL/lock release, public backup retry, restore preservation and ignored-helper retirement. Component content is byte-identical in the batch.
2. `/tmp/brn-selected-task-route-final-review.md`, exact approved `cadcec0373e4542ffb1e43a8152bba343e142f8f`: complete read-only lookup/validation/test review and valid/fixed custom-record route P2. Component content is byte-identical in the batch. Initial finding remains preserved separately in `/tmp/brn-selected-task-route-review.md`.

The prior backup missing-readiness diagnostic overlay is absent from the approved component/batch content. Its separate receipt records one expected10second failure, owned PID54274 wait status with Unix signal9, restored source hashes/clean HEAD and the parser-spelling correction without rerun. That diagnostic executable remains distinct from qualified source/artifact identity; it is not being integrated or reclassified as a normal passing suite here.

## Integrated requirements/correctness assessment

The Rust portion remains entirely test-scoped: `lib.rs` admits `process_test` only under `cfg(all(test, unix))`, and imports/callers are inside existing tests. It extracts the established restore guard for two real consumers and removes the ignored backup helper/stdout parser, rather than adding a parallel permanent witness. Exact self-test child names and task-specific environment branches prevent recursive parent spawning; private canonical roots and actual owned Child handles keep data/process ownership bounded. Atomic temporary-to-ready rename,10second parent polling/early exit and30second child fail-safe replace the indefinite stdout readiness read. Explicit SIGKILL, retained full stage/tree, absent final destination and released lock assertions remain meaningful.

The killed backup stage remains present until the first public `Store::backup` retry; no post-kill private staging setup can sweep it early. Source Note data is checked in the still-open source and through the published backup's restore, and the child-source sibling marker remains. The separate same-process live-stage test is unchanged. Restore's earlier corrected no-intervening-sweep ordering, public retry precondition, live sibling/earlier output/source/unrelated preservation and signal assertions are preserved exactly by the helper extraction. Nothing in the developer reading tool invokes these tests or changes their process environment.

The Python portion remains a small branch in the existing validated renderer. `--task` and `--check` exclude each other; selected task lookup validates the complete chosen record, prints exactly one object, then returns before render, output reads, directory creation or atomic writes. Unknown IDs return2 without output. Workflow guidance supports both the concise default route and custom `--record`; generated Completed guidance includes portable `--record PATH_TO_SELECTED_RECORD --task TASK_ID` and substitutions. New tests cover exact Unicode object output, record/output preservation, absent parent, invalid/unknown cases, mode exclusion and the corrected custom-record instruction. They neither touch Rust process actors nor introduce a second tracker/schema.

Default renderer generation and check semantics are unchanged except the intended Completed instruction. The actual campaign/selected source record is unchanged by this integration, and the generated overview content exactly equals the reviewed route component. The integration does not claim that this checkout's saved campaign observations are live or reconcile the separate lead campaign automatically. Lead regeneration/checking against the selected record remains the existing responsibility.

Both components retain their existing owners: authoritative storage behavior remains production core; process lifetime support stays private test code; record validation/rendering/lookup stays in the one developer tool. No configuration/provider/dependency/public ABI/privacy/Undo or semantic task-policy interaction is added. There is no need for a new common framework, duplicate tests or additional isolated architectural change to integrate these parallel selected maintenance outcomes.

## Qualification and integration limits

The lead's combined core219/Projects9/Python188, strict lint/format, overview check, links and whitespace qualification was separately starting/overlapping after source freeze. This reviewer did not execute those commands or infer their outcome from prior component passes. Reused component evidence and the negative control remain attributed to their original identities; actual combined gates must name `7dde6a41a4fbc66468e0400d2cc3c859f04c8ef5` and their true results.

The lead reports main207 was normally merged with exact-head/preceding-main/protection checks and whole qualified Project-tree equality, while its resulting-main gate remains separately pending. This batch review verifies actual base ancestry/content, not live remote CI or protection, and does not turn that pending gate green. Publication, required exact-head/resulting-main checks, authorized normal merge and reconciliation remain lead-owned and distinct from this source approval. Native usability, owner acceptance, release, mid-copy interruption and power-loss/hardware durability are not established by this maintenance batch.
