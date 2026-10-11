# Pre-release cleanup implementation plan

**Goal:** Remove obsolete code and misleading instructions aggressively while preserving the implemented Threads product and its safety checks.

**Authority/spec:** On 11 October 2026 the owner approved the proposed cleanup and requested an aggressive pass: update agent/setup/status docs, archive superseded plans, remove obsolete experiments/tooling after checking references, and remove proven unused production code/dependencies. Prior authorization covers isolated worktrees, GPT-6.1 Sol implementation, independent review and required-CI-gated publication/merge without routine reauthorization. Release and actual user data remain outside this task. The [Threads target](../../../architecture/threads-target.md) owns product behavior.

**Baseline:** `3a3aa7ac6f2a70815ffc778a694663f729203a79` (PR #120), clean isolated `brn-pre-release-cleanup` / `work/pre-release-cleanup`. Native observation is already deferred. Rust remains the repository pin 1.98.1; no dependency upgrades or schema changes.

**Architecture:** Keep the eight production workspace crates, shared checked writer, current provider/converter/retrieval implementations and optional feature gates. Retire standalone experiments and the old source-note materialization contract. Current docs become an explicit small entry path; historical material is recoverable at the fixed baseline in Git, with selected Threads evidence retained under `docs/archive/threads-rebuild/`.

## Boundaries and review focus

- Preserve every current runtime/CLI/native behavior, schema and recorded operation hash. Test-only code must remain available in its existing test/capability feature.
- Copy twelve live intake fixtures byte-for-byte before deleting experiments; retain current converter/figure/security assertions. Removed tests must cover only removed obsolete APIs, not current guarantees.
- Keep current CI jobs and required contexts, retirement guards, native feature checks and failure visibility. Change Markdown historical policy only to match the explicit archive directory; current links into archives still validate.
- Existing test examples and packaging helper are not obsolete merely because they are old. Keep provider capability fixtures, current runtime qualification, native retrieval, maintained converters and local packaging scripts.
- Historical decisions and evidence remain available through an exact baseline Git link. Retained current documents must not contain broken paths, expired task authority, obsolete dependency versions, old build commands or fake completion claims. Recovery/native acceptance remain outstanding.
- User-selected aggressive cleanup replaces a blanket keep-everything policy. It does not authorize deleting user files, build caches outside this worktree, current feature implementations, signing/releasing, provider access or weakening protection.

## Task 1 — production code and live fixture ownership

Owner: production Sol helper; files under `crates/`, root `Cargo.lock` only. Tooling helper waits for its fixture-ready message before deleting experiments.

- [x] Copy p1-office-mime fixtures `{forecast.xlsx,harbor.docx,harbor.pptx,hostile-dtd.docx,hostile-inflate.docx,hostile-members.docx,hostile-path.docx,plural.eml,single.eml,water-use.png}` and p2-api-check `{features.docx,prefix-variation.docx}` into `crates/brn-intake/tests/fixtures/`. Compare SHA-256 bytes, rewrite `include_bytes!` consumers in maintained/pptx/protocol/restricted_helper tests, record provenance in a fixture README. No test asset regeneration.
- [x] Remove unused direct deps: `brn` uuid; desktop libc/serde/serde_json/sha2 and corresponding optional feature entries; AI thiserror; threads-app dev tokio. Regenerate lockfile offline without version changes; review diff and dependency graph.
- [x] Remove `brn-intake` retired `asset_file_name_for_source`, `source_namespace`, `SourceTextEdit`, `apply_source_edits`, `materialized_offset` and `Extraction::{source_asset_edits,source_local_edits,materialize_for_source,original_source_range}`. Repository-wide symbol search shows no current caller. Remove four protocol tests solely for that contract and their repeated-image helper. Replace only the old materialization block in the attachment-occurrence test with raw quote correspondence; preserve all current figure, provenance, limits and decoder checks.
- [x] Gate AI `HistoryPair` and `MAX_REWRITE_BYTES` definitions/reexports behind `cfg(any(test, feature = "capability-spike"))`, matching their only consumers. Retain capability fixtures and current protocol tests.
- [x] Verify maintained intake helper/all-target tests, complete workspace tests/Clippy, capability fixtures, native UI/test-support and native retrieval/combined feature checks. Use locked/offline commands and fresh synthetic directories, shared existing build caches. No test needed solely to mirror dependency deletion; existing behavior suites supply evidence.

## Task 2 — obsolete experiments and tooling

Owner: tooling Sol helper; `experiments/`, `artifacts/`, `scripts/`, `.gitignore`; parent owns docs and instruction files. Preserve CI workflow and action checks.

- [x] After fixture transfer, delete the entire 162-file experiments tree, and `verify-trial.sh`, `verify-editor-trial.sh`, `verify-retrieval-trial.sh`, `verify-retrieval-state.sh`. No remaining executable/build consumers may refer to experiments.
- [x] Remove the 55 tracked historical MacBook UI campaign files under `artifacts/` after confirming no current executable/build consumer. Their synthetic snapshots, logs and manifests remain recoverable through a fixed baseline Git link in the archive index. Do not open or modify any actual user vault, untracked artifact or external directory. Keep `.gitattributes`, which marks still-current Threads PDF fixtures as binary.
- [x] Remove the completed `scripts/qualification/rig-decoder-comparison` standalone 0.43/0.44 harness; its result remains in archived Threads evidence and exact baseline Git history. Current Rig runtime/capability regression tests and `scripts/qualification/threads-runtime/live-journeys.py` remain.
- [x] Remove experiment-only ignore entries, retired trial exception from provider checker, absent vendor/rig-agent link-check exception, and unsupported `verify-desktop-shell.sh --native` evidence mode. Replace broad scattered historical link exclusions with `docs/archive/`; adapt tests to preserve current-to-archive target/fragment checking and rejection of broken current links. Delete only tests exclusively for the removed vendor exception.
- [x] Run `python3 -m unittest discover -s scripts/tests -v`, both retirement checks, shell syntax checks and package launcher tests. Markdown checker final run follows parent docs cleanup. Do not use blanket exclusions to hide failures.

## Task 3 — current docs and historical archive

Owner: parent or separate documentation Sol helper; parent retains this plan/evidence and integration authority.

- [x] Keep a compact `AGENTS.md` router, root README and pointer-only CLAUDE/Copilot instructions. Current references: status; canonical Threads target/behavior/invariants; setup/verification/workflow; active cleanup plan. Remove long lists of superseded policies and completed task mandates.
- [x] Update docs index, status, roadmap, setup and tooling against current source and merged PRs #115–120. Use manifests/toolchain files as version authority; remove the expired Rust1.99 build assignment and unprotected-main configuration payload. Keep the actual protection/verification contract and actual incomplete recovery/native gates.
- [x] Retain current product/architecture docs; rewrite obsolete glossary/dependency snapshot around current domain and dependency boundaries. Keep canonical unique product requirements. Replace pre-implementation instructions in target/behavior with correct implemented-versus-unqualified status, not an unrequested redesign.
- [x] Retain selected Threads review/evidence locally under `docs/archive/threads-rebuild/`: review, rebuild evidence, Phase0 evidence, Phase1 delegation/steering/backup/restore/folder evidence and Rig comparison report. Record old locations and fixed baseline provenance; convert historical relative file references to baseline Git links where needed. Archive index points to complete baseline docs/experiments and makes historical instructions non-operative.
- [x] Remove superseded docs/audits, superpowers specs, old architecture decisions/contracts, historical checkpoint, old UI screenshots/backlog, old active/completed work folders and completed Threads plans/build prompts from current checkout after ensuring authoritative requirements/evidence are retained or indexed in Git. Keep only cleanup under active work. Current source/crate README links must be fixed, including stale retrieval README framing.
- [x] Run link checks and current-doc/reference scans. Record tracked file/byte/line reductions and production removals separately; do not imply installed size savings without measurement.

## Integration gates

- [x] Independent read-only original plan review approved before implementation (SHA-256 `ed88f06232e9995a1327edf21372efc6a603b9e20cde653713240ff67a1afa8f`); historical-artifacts addendum separately reviewed before its deletion. User already authorized execution method and scope; repository workflow governs continuation.
- [x] Candidate checks pass. Independent whole-diff review checks deletion justification, fixture byte identity, legacy-only removed tests, active feature builds and documentation authority. Fix concrete findings and rerun affected checks.
- [ ] Commit owned changes, publish a tree-identical PR, attach it, merge after all required checks pass, verify merged tree and required main CI, record final evidence on PR. No release.
