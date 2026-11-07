# Proposed alpha release notes — not approved or published

This text is for the existing release-manifest approval process. Final source,
package hashes, checks and native observations must be filled from actual
receipts before approval. It is not an announcement or a published version.

## A: v0.2.0-alpha.1 / build3

The proposed Apple Silicon/macOS15+ alpha combines the native desktop, Rig0.46,
GitHub Copilot and the manual versioned updater. Agent A's response-limit
hardening264/265 is merged; selected runtime/local package source is
a921e32533dc03134ed8a2001bd82f27194c3302. The app
includes both existing document helpers; no additional Copilot CLI/SDK binary,
provider credentials or embedding model ships in the archive.

Install A once from its announced ZIP. Previously published previews have no
updater. Move BRN Threads.app into a writable application folder, open it normally
and follow macOS's normal per-app approval if requested. The app uses an ad-hoc
integrity signature without Developer ID or notarization. The stable preview
workspace path is preserved; use disposable data until native acceptance is
recorded. No security setting change or quarantine removal is part of setup.

Copilot connects through Settings using the tester's own normal device login.
Local extraction with refinement Off remains offline. Account/model availability
and policy restrictions remain explicit; no fallback account/provider is implied.

Local source/artifacts: qualifieda921e325, recorded in the current alpha evidence.
Actual hosted source/artifacts: pending owner-approved release manifest/workflow.
Native acceptance: pending one coordinated B/A disposable-profile session.

## B: v0.2.0-alpha.2 / build4

The proposed increasing build exercises delivery and recoverable replacement
from command-enabled A. Record any runtime change from A explicitly; the local pair uses identical executables and differs only in version/build/receipts.

Save work, make a normal backup and quit BRN normally. For a user-owned
installation, request the announced version with:

```sh
"$HOME/Applications/BRN Threads.app/Contents/Resources/bin/brn-update" v0.2.0-alpha.2
```

Use your actual installation path. The command verifies the exact release,
retains the old app and prints reopening/recovery instructions. Reopen normally,
approve the replaced app through macOS if requested, and verify notes, drafts,
assets, projects, history, settings and Copilot continuity or normal reconnection.
The updater never copies credentials or changes workspace files. An old app
bundle is not a database rollback after a schema upgrade; use the matching
pre-upgrade backup and compatible app, or a forward fix.

Source/artifact identities and actual published HTTPS A→B result: pending.
The earlier local A→B test substituted transport and does not satisfy this gate.
Publication, native/owner acceptance and invited distribution remain separate.
