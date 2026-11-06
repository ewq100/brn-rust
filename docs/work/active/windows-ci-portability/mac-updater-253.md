# Mac updater evidence and remaining acceptance

Agent B owns [#253](https://github.com/ewq100/brn-rust/issues/253)'s updater,
packaging and associated acceptance. The current task is `mac-updater-253` in the
[existing selected record](record.json). This document is supporting evidence,
not another task tracker. Agent A owns UI/chat, Rig and Copilot; Agent C owns
#255. Ownership/build-slot coordination is linked from #253, #252 and #255.

## Candidate and scope

Baseline: main `3b72813df7cfcb857cdef55fff2c3f17f3fe86f6`, including PR248.
Production updater/package candidate: `832e8888`; subsequent qualification-helper
and evidence changes do not alter packaged production inputs. No Rust/runtime,
provider, lockfile or approved `release/preview.json` changes. No release/tag or
announcement was created. Final integration/publication authority is separate.

Reused the existing Finder launcher, ad-hoc packager, source/legal custody and
approved-manifest/immutable-release workflow. The only new runtime component is
the bundled built-in-tools script. It requests one explicit version from the
fixed HTTPS repository and validates receipts/hashes, bundle identity/version,
increasing build, supported OS, signed arm64/deployment identity and portable
library inventory before replacing the app. Actual library inspection remains
the packager's responsibility; verified binary hashes bind the inventory.

The updater stages outside the bundle, uses a fully prepared hard-link ownership
lock, and retains the old app with same-volume renames. Its normal launcher
refuses during update/recovery. It never kills BRN, reopens/resumes automatically,
changes security controls or writes workspace/credential files. A shell-owned
descriptor lock also excludes recovery while an orphan filesystem child is still
active. Recovery restores an absent installation or retains an existing app; it
does not automatically roll back a potentially migrated database.

## Observed checks — 6 November 2026

Host: Evos-Mac-mini, Apple Silicon arm64, macOS 27.2/26B123. Rust 1.98.1 and existing
Python 3.11 were selected explicitly because shell `cargo` was absent and system
Python lacked `tomllib`. Only task-local PATH entries were used.

| Gate | Result and receipt |
|---|---|
| Desktop | Pinned `cargo build --release -p brn-desktop --features native-ui --locked --jobs 4`, deployment target15.0: passed. Existing rust-objcopy debug-strip warning recorded; executable built. [Log](evidence/mac-updater-253/build-desktop.log.gz). |
| CLI/helpers | Pinned release CLI, locked stock and adapted helpers, separate task-owned targets: passed. [CLI log](evidence/mac-updater-253/build-cli.log.gz), [helper log](evidence/mac-updater-253/build-helpers.log.gz). |
| Tooling | `python3 -m unittest discover -s scripts/tests -v`: 353 tests passed, 11 explicit platform/configuration skips. [Log](evidence/mac-updater-253/tooling-tests.log). |
| Focused | Packaging/release/updater suites: 41 tests passed, including 19 actual macOS updater cases. [Log](evidence/mac-updater-253/focused-tests.log). |
| Real local BRN A→B | Actual signed local packages alpha.1/build101→alpha.2/build102; replacement, B headless reopen, unchanged state and backup/restore passed. [Report](evidence/mac-updater-253/real-update.json), [A receipt](evidence/mac-updater-253/package-A.manifest.json), [B receipt](evidence/mac-updater-253/package-B.manifest.json). |
| Relocated product | B startup/reopen, foreign-data refusal, both helpers/all 8 typed formats and 16 CLI imports with conversion Off passed. [Log](evidence/mac-updater-253/packaged-qualification.log), [captures](evidence/mac-updater-253/packaged-fixture-captures.json.gz). |
| Documentation | Markdown links: 723 files/3519 links/0 failures; generated overview and final diff checks passed. |

The local A/B tags are test identities, not approved or published versions. Both
source apps were signature-verified. A's **test copy** substitutes only its
absolute curl executable with a local asset fixture; B's signature, hashes,
receipt, Mach-O identity and filesystem replacement are real. This does not prove
GitHub HTTPS delivery of a published A/B pair. Native acceptance is not claimed.

The real synthetic workspace contains notes, a linked project, chat/queued work,
an active unsaved editor recovery draft, history, a protected import and one
actual retained figure Asset. It uses a synthetic Copilot model selection and
credential sentinel; no valid credential or provider call is used. Every external
file hash is unchanged immediately after updating. B's actual headless startup
then preserves the inspected logical state. A pre-update backup restores to a
fresh store. Real Copilot connectivity/expiry/reconnect remains a native/live gate.

## Failure and independent-review evidence

The 19 updater cases cover offline/missing/corrupt downloads, manifest/version/build
and platform refusal, corrupt ZIP, traversal/symlinks, wrong bundle and tampered
signature/binary, unsupported OS/host architecture, portable-library inventory,
non-writable parent, running app without termination, normal-launcher exclusion,
failed replacement, download interruption, concurrent attempts, SIGKILL between
renames/after installation, and recovery interruption after claim/restore/before
unlock. PID-only recovery-shell kill retains the orphan worker's guard; a live
second recovery refuses, then recovery/retry succeeds. Recipient tool invocation
also passes with a nonexistent `DEVELOPER_DIR`; this simulates unavailable Xcode
tools, not a clean-user native installation.

Independent read-only review covered the whole candidate and Mac-only CI step.
Findings and dispositions:

- **Valid/fixed:** recipient `lipo`/`otool` were Xcode shims. Replaced runtime use
  with stock codesign metadata and hash-bound inventory; build-host checks remain.
- **Valid/fixed:** permanent recovery claim prevented retry after SIGKILL.
  Replaced it with kernel descriptor ownership and idempotent recovery.
- **Valid/fixed:** first repair's lockf supervisor could die while its child kept
  running and later remove another transaction's lock. The actual shell now owns
  FD9; inherited filesystem children retain it. The reviewer independently
  reproduced the PID-only case and verified refusal/retry after the fix.

Final reviewer: no remaining actionable findings. Reviewed production updater
SHA256 `7f5ea95caa55c0f552d54fd215a28c18d3a55e16b1011db7bfe6889d565157b5`;
the qualification-helper-only follow-up also received a clear review. macOS 15
descriptor-lock support was independently checked against [Apple's source](https://github.com/apple-oss-distributions/shell_cmds/blob/14a2692995de90c2fde0c3c1e639a8c7d9e2312d/lockf/lockf.c).

The first real-product witness failed report emission after behavioral checks,
because Project creation returns writes rather than a direct ID. The [original
failure](evidence/mac-updater-253/qualifier-report-failure.log.gz) is retained.
The helper now reads that write identity; the subsequent whole witness passed.

## Reproduce local package/update qualification

Use the [normal reproduction commands](../../../development/preview-release.md#receipts-and-reproduction)
for the pinned desktop/CLI/helpers, target graphs and notices. Use local copies
of the approved manifest with test versions `v0.2.0-alpha.1`/build101 and
`v0.2.0-alpha.2`/build102; do not modify `release/preview.json`. Run the normal
packager for each into absent absolute directories, then:

```sh
python3 scripts/tests/test_macos_updater.py --qualify-update \
  --delivery-a "$PWD/work/real-A" --delivery-b "$PWD/work/real-B" \
  --host-cli "$PWD/target/release/brn" --evidence "$PWD/work/real-update-evidence.json"
```

The helper uses fresh disposable synthetic directories and removes them after
the run. Runtime recipients use the actual versioned command in the setup guide;
they do not need this Python qualification helper.

## One remaining native acceptance session

Required inputs: Agent A's selected verified runtime/Copilot SHA and package
dependencies; an unlocked Mac mini with a disposable clean user/profile; a
tester available for normal Gatekeeper approval and their own Copilot device
login. The Mac reports `CGSSessionScreenIsLocked=true`; no native UI was driven.

1. Package the combined selected candidate with actual runtime dependencies and
   public version/build receipts; install command-enabled A in a writable app
   folder. Observe ordinary launch/Gatekeeper approval without development tools.
2. Use only a fresh explicit synthetic workspace and tester-owned credentials.
   Exercise Copilot device sign-in/catalogue, chat/tool/multi-turn/Stop, native web
   search with real sources, Web Off and cancellation/account isolation, document
   extraction with Luna refinement Off/On, knowledge maintenance/local search,
   and referenced writing Apply/Discard/Undo/stale refusal.
3. Add notes/projects/chats/assets/history/drafts/settings, make a normal backup,
   quit normally, and run the documented command for the approved real B from
   the fixed repository. Observe any replacement Gatekeeper approval.
4. Reopen B and verify useful data/settings/connected Copilot or normal expired
   reconnect, web search and Luna refinement. Exercise native backup/restore and
   relevant interrupted-import/cancellation paths; recheck compact archive
   disclosure at 900×600 and reconcile remaining #243 native/owner acceptance.
5. Record screenshots/observations against exact source/package/model/route;
   owner use and explicit release approval precede invited distribution. Follow
   the existing immutable-publication gates. No merge/upload equals acceptance.

Do not automatically run an old app against a migrated store. Recovery after
schema changes requires a compatible app plus matching pre-upgrade backup, or a
forward fix. Retained app bundles alone are not database rollback authority.
