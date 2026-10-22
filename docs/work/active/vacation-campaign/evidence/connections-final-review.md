# Connections Project freshness — independent complete-candidate review

**APPROVED for qualification: no actionable source findings.** This is source-review approval of the exact candidate below, not a claim that the uncompiled final candidate passes checks or is integrated/native-accepted.

## Identity and coverage

Reviewed `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`, branch `codex/connections-project-freshness-20261022`, clean HEAD `c12966d0bc3fc37ff0f64f618de481c54de8c562`. Verified merge base `10f2ff80a39408ce411e10c77200e6d4a89b12d3`. Complete actual diff: exactly three modified files, 286 insertions: two production lines and one 284-line synthetic regression journey. No dirty/untracked work was present.

Independent SHA256:

- `crates/brn-desktop/src/threads_projects.rs`: `46844511fe84fb9227cb5bac6bc8786826f639f5cf84dbccb267e41072a1834e`.
- `crates/brn-desktop/src/threads_ui.rs`: `9aaed2b1ee2aa4a37a75539acd1909b8e7c74c0ab5f9366ab7de484c7329ec4d`.
- `crates/brn-desktop/src/threads_tiled_tests.rs`: `a5765c9fa783786b2366a0f0f219ad89025d2dc479e0e2e662003f9b9935ada9`.

Read the complete actual patch and relevant whole test/helper context, Project controls/service and visibility predicate, Undo service/control, layout/navigation, graph owner and search-worker snapshot/synchronization flow. Applied the product-candidate and Rust review guidance plus repository workflow, strategic programming, architecture/invariants and existing Project/Connections contracts. I authored neither this candidate nor its qualification.

## Requirement/correctness assessment

The demonstrated boundary is correctly owned by the desktop derived-view reconciliation: a successful checked Project command returns a receipt, normal `refresh` reloads canonical records, and `threads_projects.rs:147` now retires/replaces Connections through its existing owner. Failed Project commands still exit before that successful-command path. The corresponding `threads_ui.rs:2532` call follows the existing Undo outcome reporting, canonical refresh and open-Note reload. Undo errors/deferred outcomes can therefore also trigger a harmless fresh read of unchanged accepted state; no unsuccessful compensation is presented as an accepted mutation.

`refresh_connections` synchronously clears result/error/pending state, drops the previous task and cancellation owner, and increments its private generation before any asynchronous replacement (`threads_connections.rs:61–128`). Completion accepts only the matching generation. It issues no request when Connections is hidden or no Note is open; a new request preserves selected centre, Project scope, depth, source option and relation filter. The worker answers against a fresh authoritative reader snapshot after index synchronization (`search.rs:815–913`), with the existing scoped visibility checks. No render-time database guard, duplicate privacy rule, extra cache, persistence/schema/API/provider change or new authority is introduced.

The existing shared `apply_project` boundary covers this mutation family without scattered row callbacks. Undo uses the existing common reconciliation boundary rather than an unlink-specific exception. Additional bounded background retrieval after a visible Project mutation/Undo is the intended cost; this review makes no measured latency claim. Existing two-hop/filter/recenter tests and their implementation remain intact.

## Regression effectiveness and static fixture risks

The new journey (`threads_tiled_tests.rs:1208–1493`) uses disposable synthetic data, the existing offline/no-poll fixture, checked Project commands and the established owner-write fixture for the relation. A private Note is retained in its origin Project and explicitly linked into the second Project, so unlinking the latter is permitted and does not promote/delete the Note. It opens the centre in that second Project and establishes the completed private neighbor before invoking the actual public Open-beside menu and Unlink control.

The corrected route keeps Note, Connections and Projects in the layout, checks centre/scope/selected Project and completed graph, and requires the Unlink control to be visible/in-window. Direct Unlink dispatch has no executor drain before the independent SQLite revocation and exact retained Note/body assertions. The retained-graph assertion prevents row disappearance or layout changes from producing a false green; the immediate frame additionally requires the Connections surface and absence of the revoked row. A later settled-result phase requires a successful completed result without that neighbor, so merely clearing forever or producing a query error does not satisfy the journey.

The Undo phase identifies the concrete membership-removal receipt from its before/after Project documents, uses the actual Changes row and standard bounded scroll helper, and drains replacement work. Its canonical restored-access and exact-Note checks precede graph/row expectations. It requires the same centre/scope, completed graph and restored neighbor, making Undo a meaningful positive restoration boundary. I found no remaining moved-String/private-field/payload-type access in the added source; the retained Note is correctly destructured from `RecordData::Note`. This is static reasoning, not compile evidence.

The long fixture establishes a real privacy/membership transition and three distinct observations (immediate retirement, settled exclusion, Undo restoration) in one journey; it does not justify a new generic fixture framework or duplicate tests. No obsolete production path or test needed retirement for this two-call repair.

## Evidence and remaining gates

Independently read the durable `connections-baseline-reproduced.{json,log}` evidence in the lead campaign. Actual test-only baseline `ed4e1d2e25ade884fce86b662a5d8138581f8359` compiled, ran one test and exited 101 at **“Connections graph must discard the revoked Note”**, after checked access became false and exact Note preservation passed. Recomputed log SHA256 `3e31ee1642019e4e8227e641cbd961a4f90ee71db9f8e4823e39bcea9856ac21` matches the receipt. The subsequent post-Unlink row assertion was not reached. The earlier E0609, missing-control and private-generation-field failures remain witness-construction failures, not product evidence.

No final-candidate build/test/lint/native action was performed by this reviewer. Lead still owns actual focused and broader affected desktop/Project/Undo qualification, required formatting/lint/build checks and normal integration gates. The added journey does not deterministically force an old in-flight answer to arrive after mutation; pending-response safety is source-supported reuse of the existing cancellation/generation owner, not newly observed race coverage. No hosted/main/native usability/owner acceptance or broad cross-window freshness claim follows from this approval.
