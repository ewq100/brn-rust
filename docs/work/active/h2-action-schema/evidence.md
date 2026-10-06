# H2 Action tool schema — task record

The owner selected H2 on 2026-10-06 ("run H2"). Only H2 is in scope. The rest of
the paused V1 roadmap stays paused. The spec is
[H2 in next specs](../preparation-checkpoint/next-specs.md#h2--one-compatible-action-schema),
plus the queue entry in the [V1 handoff](../v1-handoff.md#h2--derive-one-action-tool-schema-compatibly)
and [reuse decision B](../../../architecture/decisions/2026-10-06-compatible-reuse.md#b--tool-schema-structure-accepted-later-task-h2).

## Baseline and authorization

- I fetched origin. The baseline is `origin/main` at
  `f3cf699` (PR81, H1 merged). The local `main` checkout was two commits behind
  and was left alone, as were the other worktrees.
- Branch `codex/h2-action-schema`, in its own worktree.
- "Run H2" covers implementation, review, checks and a focused PR. It does not
  authorize a merge, live calls, model downloads, private data or account actions.
  None of those happened.
- Environment: Mac mini (Darwin arm64), pinned Rust 1.98.1, locked dependencies,
  offline Cargo. `TMPDIR=/private/tmp/brn-h2-tmp`,
  `CARGO_TARGET_DIR=/private/tmp/brn-h2-target`. No fetch was needed.

## Reuse decision

| Need | Decision | Evidence |
| --- | --- | --- |
| Object shape, required list, tags, enum variants, closed objects | **Reuse** Schemars 1.2.2 derive on the existing Serde types | Already locked through Rig. Now a direct `=1.2.2` dependency of `brn-ai`; `Cargo.lock` gains one edge and no new package |
| Nine `Option` fields read through `required_nullable` | **Adapt** with a schema-only `RequiredNullable<T>` | It returns `Option<T>`'s schema, which keeps `null`, and is not an `Option`, so the derive marks the field required. `schemars(required)` alone drops `null` (mutation below) |
| `uuid`/`date` formats with their bounds | **Adapt** with schema-only `WireId`/`WireDate` newtypes | Schemars has no `uuid`/`date` format attribute and `inner(...)` cannot add `format` to `sources` items |
| Self-contained schema the three routes already accept | **Adapt** with one recursive transform: inline subschemas, no `$schema`, `oneOf`→`anyOf`, `const`→one-value `enum`, drop generated `title`/`description` and Rust integer `format` | Matches the pre-H2 schema exactly (modulo set order). The existing route test still forbids `oneOf`/`const` |
| UTF-8 byte limits, UUID/date meaning, full records, approval | **Keep** in Rust `validate` and Workflow | Schema lengths are provider hints in characters. Nothing in `validate` or Workflow changed |

Cost against removed duplication:

- Removed about 45 lines of hand-written JSON (`action_ref_schema`,
  `action_data_schema`, `checked_ref_schema` and the `parameters` body).
- Added 25 `#[schemars]` attribute lines, a 30-line generator/adapter function and
  about 30 lines of schema-only types and comments. Production code grew by
  roughly 40 lines.
- One direct dependency on a version Rig already locks.
- Numeric and string bounds still appear twice: as attributes and in `validate`.
  That was already true with the JSON.

The gain is structural. The field set, the required list, tags, enum variants and
closed objects now come from the Serde types, so they can no longer drift from
what Rust accepts. I judged the adapter small and bounded, so H2 adopts the
derivation. This is not a line-count win.

## Changed behavior

None intended or observed. `ProposeActions::parameters()` now returns the derived
schema, generated once per process and cloned per request. It equals the frozen
pre-H2 schema once `required`, `enum` and `type` arrays are compared as sets. Object
key order in the request bytes may differ. Serde attributes, `validate`, the tool
description, capability selection, dispatch, retries and cancellation are unchanged.
Knowledge, conflict and read tool schemas are untouched.

The [brn-ai contract](../../../../crates/brn-ai/README.md) now says where the schema
comes from.

## Candidate and verification

Commits on the branch: `a6a3f24` adds the witnesses against the unchanged manual
schema, `f4d9a4f` replaces it with the derivation, `bf71b40` applies review
advisories. Code: `crates/brn-ai/Cargo.toml`, `Cargo.lock`,
`src/action_candidates.rs`, `src/proposal_tools.rs`, `src/provider_formats_tests.rs`.

Witnesses:

- `proposal_tools::tests::action_tool_schema_is_equivalent_to_the_frozen_manual_contract`
  keeps the pre-H2 schema as an oracle in test code. It requires equality (sets
  for `required`/`enum`/`type`; `anyOf` order and everything else exact). It also
  checks that all 14 data fields are required and the nine nullable ones still
  admit `null`.
- The registered route test now requires the `propose_actions` parameters seen in
  the ChatGPT Codex Responses, Copilot Responses and Copilot Chat request bodies to
  equal the emitted schema, with no `$ref`. Comparing as sets is needed because the
  routes reorder `required`.
- The existing Rust parsing and bounds tests (missing versus explicit null,
  unknown and nested fields, tags, types, byte limits, whole-JSON limit, strict
  refusal on all routes) are unchanged and still pass.

| Check | Tree | Result |
| --- | --- | --- |
| `cargo test -p brn-ai --lib --locked --offline` | baseline `f3cf699` | 136 passed, 0 failed, 1 ignored |
| Witness and route tests against the manual schema | `a6a3f24` | passed |
| `cargo test -p brn-ai --lib --locked --offline` | `f4d9a4f`, `bf71b40` | 137 passed, 0 failed, 1 ignored |
| Mutations: drop `owner` max length; `schemars(required)` alone on `priority`; plain `Option<WireId>` on one id | `f4d9a4f` | each fails the witness |
| `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | `bf71b40` | passed |
| `bash scripts/verify-end-to-end.sh` (retirement, workspace fmt/build/Clippy/tests, fixtures) | `f4d9a4f` | exit 0; 1,619 passed, 0 failed, 16 ignored, plus 52 fixture assertions |
| same | `bf71b40` | exit 0; 1,619 passed, 0 failed, 16 ignored, plus 52 fixture assertions |
| `git diff --check`; `python3 scripts/check-markdown-links.py` | final | passed; 43 files, 480 local links, 0 failures |

The 16 ignored tests are the existing documented exclusions. No native-feature,
GUI, live-provider or model check ran. None applies: the change has no display
or feature path, and offline schema equivalence does not qualify live endpoint
support beyond the routes already qualified.

## Independent review

A read-only reviewer (separate agent, no cargo) reviewed `f3cf699..f4d9a4f`
against the spec and the pinned Schemars and Rig sources. Verdict: approve, no
blocker or correctness defect. It confirmed from source that the set comparison
cannot hide a duplicate or a reordered `anyOf`, that `i64::MAX` encodes identically,
that the transform never sees property names (so the `title` and `description`
properties survive), that `inline_subschemas` keeps `$ref` out, and that Rig's
strict-mode rewrite rebuilds `required` from property keys, so the route check
would still catch a missing property.

Dispositions:

1. **Medium, fixed.** Missing decision and task-state records. This record, decision
   B, the handoff, next specs, status and the brn-ai contract are updated.
2. **Low, accepted.** State the cost honestly. Done above.
3. **Low, recorded.** The frozen oracle keeps a copy of the structure in test code.
   Retire it at the next reviewed structural change to the Action types. Keep
   targeted bound checks then, because Schemars skips a bound whose type does not
   match the field, and today only the oracle would notice.
4. **Low, fixed.** The test module was made `pub(crate)` to share the helper. The
   helper is now a `#[cfg(test)]` function and the import sits at module level.
5. **Advisory, fixed.** `RequiredNullable` now forwards `schema_id`.
6. **Advisory, kept.** The custom `const` handling overlaps Schemars'
   `ReplaceConstValue`. One closure with all five adjustments is easier to read.
   `.for_deserialize()` is the default and stays as documentation.

## Acceptance

There is nothing for the owner to see: the provider request carries the same
schema and Rust parsing is unchanged. Owner review of this record and the PR is the
acceptance step.

## Integration

Pending. The PR, its exact-head CI, the merge and the post-merge check are recorded
here when they happen. Merge needs the owner's authorization.
