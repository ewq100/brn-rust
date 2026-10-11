# App-wide tiled UX and preview implementation plan

**Goal:** Extend the accepted native tiled style to every existing screen, publish a reviewed PR, integrate after required CI, and publish the first installable macOS preview.

**Baseline:** `7c189c71d3eb664ef6493294d5b73d3e12b90b7d` (merged PR #122), isolated `work/tiled-app-preview`. PR #122's four required jobs passed before merge and on actual merged main; unrelated optional platform failures remain visible.

**Spec:** [Accepted scope](spec.md). **Architecture:** presentation helpers over the existing GPUI views and checked services; portable packaging over existing binaries; an explicit version manifest and exact-commit CI gate for release. **Stack:** Rust 1.98.1, GPUI Kit 0.7.1, Rig 0.44.0 and unchanged lockfile/dependency graph. Repository workflow governs execution; the owner already selected Sol 6.1 subagents and authorized the work.

## Ownership and interfaces

- UI helper owns `crates/brn-desktop/src/threads_ui.rs` and a small shared presentation module only if warranted. Keep one palette/theme initialization, existing toolkit states and callback IDs. Reuse note/chat/review entities.
- Test helper owns a new `crates/brn-desktop/src/threads_tiled_tests.rs`; UI helper registers it. Tests use the existing headless GPUI harness and synthetic offline fixtures. Shared observation IDs: `workspace-nav`, `workspace-import`, `search-results`, `search-empty`, `home-needs-you`, `home-notes`, `home-recent-changes`, `actions-group-{status}`, `receipt-{operation}`, `history-empty`, `settings-provider`, `settings-copies`, `composer-tile`, `workspace-notice`. Preserve all older IDs needed for current behavior tests.
- Release helper owns `scripts/make-macos-app.sh`, `scripts/package-macos-preview.py`, `scripts/release-preview.py`, packaging/release-gate Python tests, `.github/workflows/preview-release.yml`, an approved preview version manifest and `docs/development/preview-release.md`. No desktop/core edits. Preserve existing local script arguments; portable mode is explicit.
- Lead owns this plan/spec, current status/target/README updates, final integration, independent review and publication. No concurrent helper commits. Parent uses the existing compatible target cache and fresh synthetic data.

## Review focus

1. Narrow/short windows, long Action descriptions and credential paths: controls remain reachable without hiding checked feedback or Undo.
2. Search, review and screen navigation while edits/chat drafts exist: no retargeted draft, lost buffer or unsolicited save.
3. Action/status/settings buttons and History Undo: UI layout changes preserve exact underlying receipts and authority.
4. Relocation, spaces in paths, missing helper/converter, preexisting foreign workspace: launcher uses current user's preview directory and refuses invalid state without overwriting it.
5. Old CI events, optional failed jobs, moved main, reused version and interrupted uploads: only the approved exact main commit may become a complete published prerelease.

## Task 1 — common tiled native presentation

- [x] Read existing render paths/toolkit theme and apply shared colors, tile/pane/heading/control treatment across the entire shell, including notes/threads/review.
- [x] Recompose Home, search/results, existing Action status groups, receipt history and labelled settings into consistent tiles. Keep all current callbacks/IDs, dirty buffers, provider status, error notices, keyboard shortcuts and Undo.
- [x] Add focused headless tests for navigation with pending edits/drafts; current-note search/open/clear; Action status change and receipt Undo; settings persistence preserving unrelated fields; meaningful empty states; narrow layout accessibility and existing writing review/chat behavior.
- [x] Run desktop native-test-support tests and strict native/all-target lint. Existing writing/focus/steering/backup suites must stay green. No live provider/native launch is claimed.

## Task 2 — portable preview packaging and gated publication

- [x] Extend local packaging with explicit portable preview mode, required sibling intake helper and runtime per-user data path. Keep core storage checks authoritative and reject output collisions.
- [x] Add distribution script that verifies arm64 native executables/dependencies, signs nested binaries/app ad hoc, emits relocatable archive/checksums/source manifest and refuses incomplete output.
- [x] Add fixture tests for launcher relocation/space-containing paths, runtime workspace selection, missing helper, failures and collisions. Test release policy decisions with synthetic workflow/job/release metadata, including stale/moved/reused versions and optional failure versus missing required success.
- [x] Add trusted-main workflow_run release lane. Validate same-repo push/main CI identity, current exact main SHA, explicitly changed approved version and all four required completed-success job names from latest attempt. Build pinned locked native-ui/helper on arm64, package, run fresh headless startup/reopen/refusal checks, recheck main and create draft release; upload/verify every asset, then immediately recheck exact current-main SHA and approved manifest/version identity before publishing that draft as a prerelease. Leave the draft unpublished on any mismatch or interrupted upload. Do not overwrite an existing release.
- [x] Document installing the preview, stable data location, provider setup, external Poppler, signing/notarization status and remaining qualification. Native retrieval/model downloads are not part of this preview.

## Task 3 — verified integration and first release

- [x] Independent read-only plan review before implementation.
- [x] Independent full-diff code/packaging/security review after helpers finish. Fix supported findings and repeat affected checks.
- [ ] Run formatting/docs/tooling tests, default workspace checks as affected, native UI/widget and combined optional-feature checks. Build the actual optimized preview and validate package contents/startup against synthetic directories. Record exact commands/results and limits.
- [ ] Publish PR and attach it. Wait for required CI on exact head, merge without bypass, confirm actual main tree and required main CI. Preserve visibility of unrelated platform failures.
- [ ] Allow the approved release lane to publish only the intended preview; inspect release/assets/checksums and give the owner the download and PR links. If publication is blocked, preserve the verified local installable artifact and report the specific unmet gate rather than claiming release.

## Gate record

- Defined: owner requested app-wide tiled presentation, PR and release with Sol 6.1 helpers. First preview scope stated while work continues; native observation remains deferred.
- Plan review: approved by independent read-only GPT-6.1 Sol after adding a final exact-main/manifest check immediately before release publication.
- Implementation and independent code review: complete. Local workspace/native/combined/tooling checks pass; [evidence](evidence.md) records commands and limits. Exact stamped package qualification, PR integration and publication receipts follow on the PR/release.
- Native and live-provider acceptance: not newly performed; preview limitations must remain explicit.
