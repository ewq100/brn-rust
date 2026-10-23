# Linux native gaps and file workflows

Observed 2026-10-23 UTC. Base main is
`7b416591275f2552823cb18fe357f5a20617168a` (merged PR220); isolated branch
`codex/linux-native-gaps-20261023`. This batch is local only. Publication, a new
PR and merge require separate authorization. No Mac access, live model accounts,
Windows expansion, credentials, persistent remote access or release occurred.

## Results

- [Accessibility](accessibility.md): implemented named actionable Project rows
  and one shared bridge exposing existing dropdown activation to AT-SPI. Actual
  Project selection and menu Click/pointer/Enter/Space/Escape passed. Disabled
  state translation remains unpassed. The subsequently authorized
  [GPUI Kit 0.8.0 qualification](gpui08.md) fixes toolkit disabled-node metadata
  through upstream code; the existing Unix translator still reports enabled/sensitive.
- [Connections](connections.md): ordinary native recordings show immediate private
  neighbor retirement while the replacement view is pending, followed by settled
  Unlink and exact Undo. Repeated retirement and restored membership verified;
  complete retained Notes/provenance unchanged. No production Connections change.
- [File workflows](file-workflows.md): all eight formats imported through existing
  offline Xberg CLI services; repeat/error checks, backups, fresh restore, refusal
  cases and recovery integrity tested. Actual native restored content and unsaved
  recovery were opened; recovered draft saved without changing original vault.
  Native picker remains access-denied before dialog dispatch. Full-backup Restore
  has no desktop control; CLI coverage is not native picker/Restore acceptance.

See [exact commands and retained logs](verification.md).

## Validation and review

Final source checks after GPUI Kit 0.8.0: desktop native-test-support suite
**352 passed, 0 failed, 1 ignored**; writing UI **44 unit + 12 toolkit host tests passed**;
production native-ui build passed; all-target strict Clippy passed;
formatting, diff whitespace, Markdown links, generated overview, provider and
intake retirement checks passed. Earlier helper compilation and blanket row-role
failures were corrected and the complete desktop suite rerun. Relevant menu,
Project, backup/cancel/error and recovery regressions remain intact.

Independent source review found no unresolved blocking issue after narrowing
roles to Project selection rows. Actual native observations supplement toolkit
checks. Mac-only close/menu/accessibility acceptance remains pending; neither
Linux results nor this batch establish release or live-AI qualification. No CI
was triggered because this branch has not been published.

## Remaining actions, in order

1. Assess a compatible upstream AccessKit state-translation correction with the
   parent. GPUI Kit 0.8.0 adoption is qualified locally; it does not alone establish
   Linux disabled-state acceptance. Keep maintenance and Rig migrations isolated.
2. Resolve legitimate portal caller-inspection access before native file-picker
   tests. Exact denied action: `org.freedesktop.portal.FileChooser.OpenFile` for
   this task's BRN; reason: unable to open `/proc/19341/root`. No file selection
   occurred; no identity spoofing or validation bypass attempted. Native import
   Cancel/Stop/error/repeat and backup folder workflows remain blocked/unverified.
3. Obtain publication approval for this new branch if desired. PR220 authority
   does not authorize publishing this batch. Mac-specific acceptance needs a Mac.

Raw recordings/logs and disposable fixtures are retained under
`/workspace/brn-environment/logs/linux-native-gaps` and
`/tmp/brn-linux-files-20261023`; selected synthetic evidence and checksums live
beside this record. Never include the disposable edit-token file in publication.
