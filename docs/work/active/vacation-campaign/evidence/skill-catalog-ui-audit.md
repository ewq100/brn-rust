# Skill catalog UI audit

Date: 2026-10-20

## Boundary and source

Read-only source audit at checkout `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, HEAD `1d75088f` (`codex/agent-first-followthrough`). The only dirty files were the campaign task record, generated overview, and the named draft-recovery evidence; they were left untouched. The audited skill source is the supplied source base `main63345a8e`; relevant paths have no task-local source edits. No Cargo/build/test/native/provider/account actions were run. This report records static source reasoning; the sequence below was not executed.

Scope was the desktop skill controls and their direct owner service/store behavior, with the current skill contract and existing direct tests. No credentials, private installed-package payloads, or global configuration were inspected.

## Finding: archived Project inventory row offers an Enable action that the owner service must reject

**Accepted invariant.** `docs/architecture/threads-behavior.md:216` says clearing an inactive Project selection may only demote the selected flag and cannot enable or select a new identity, digest, or scope. The desktop also labels inactive Project packages as unavailable in the shared-service contract (`crates/brn-threads-app/src/skills.rs:61-67`). The core service enforces the active-Project requirement before committing a new enabled binding (`crates/brn-threads-core/src/store.rs:1224-1245`).

**Source path.** `Workspace::scoped_skills` marks rows already present in canonical policy as unavailable for an archived Project (`crates/brn-threads-app/src/skills.rs:48-69`). In `ThreadsUi::refresh_skills`, however, package inventory entries absent from those policy rows are appended with `available: true` (`crates/brn-desktop/src/threads_skills.rs:65-85`). The settings control enables its ordinary Enable button whenever that status is available (`threads_skills.rs:301-345`). Its handler rechecks that the view's Project scope is still current, then calls `set_skill_enabled` (`threads_skills.rs:111-140`); the service checks the package digest and submits the enabled binding (`crates/brn-threads-app/src/skills.rs:82-105`), which the store rejects for an archived Project (`crates/brn-threads-core/src/store.rs:1224-1245`).

**Reproducible sequence, inferred from source (not executed).** Install a valid package; create and archive a Project with no binding for that package; open its skill settings in that scope (an existing chat retains the Project association because `project_for_chat` does not filter archived records, `crates/brn-threads-core/src/projects.rs:150-153`); refresh the package list. The package has no scoped-policy row, so the desktop appends it as available and renders an enabled “Enable” button. Clicking it passes the current-scope check and reaches the shared service, which returns `enabled skill requires a current active Project` rather than applying a change.

**Expected / actual.** Expected: an inventory-only package in an inactive Project scope is represented as unavailable for enabling, consistent with existing policy rows and the contract; no new binding can be offered there. Actual: the button is enabled, but the core correctly rejects the write. This is a misleading/dead-end UI choice, not an observed canonical-state mutation.

**Missing witness.** Existing desktop coverage exercises inventory-only enablement in general and active Project scopes (`crates/brn-desktop/src/threads_skills_tests.rs:133-265`) and stale digest inspection/adoption plus stale-context action rejection (`:267-408`). Core coverage exercises an already-bound skill after Project archival and rejects new identities/selections (`crates/brn-threads-core/tests/skill_policy.rs:248-367`). There is no desktop witness combining an inventory-only package with an archived Project scope, which is the gap in this path.

## Other audited choices

No additional actionable defect was established in this bounded pass. Source shows explicit Refresh invalidates the cached view; Inspect checks the captured digest against current installed bytes; adopting a stale binding has a separate “Enable current version” action and stores the exact current digest while leaving selection off; ordinary Enable uses the approved digest; Select goes through the scoped owner service and cannot select unavailable bytes; Remove passes the displayed digest and the package removal path verifies exact identity. Delayed folder selection only performs workspace-wide installation, does not bind or select in the then-current scope, and subsequent scoped actions capture and recheck scope. These are source observations, not runtime verification.
