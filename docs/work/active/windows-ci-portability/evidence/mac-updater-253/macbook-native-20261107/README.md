# MacBook alternate-directory native functional observations

Owner's 7 November #253 clarification permits current-account functional testing
before a clean OS account exists. These results use fixed source
`a921e32533dc03134ed8a2001bd82f27194c3302`, synthetic data and separately configured
Copilot credentials. Ordinary release Finder first-launch, clean-user acceptance,
published browser/Gatekeeper delivery, stock HTTPS and owner acceptance remain
separate pending gates. [Detailed observed results](functional-results.json).

## New local artifacts

The original transfer folder remains unavailable on this MacBook. The existing
assignment permits rebuilding in that case. These are new MacBook artifacts, not
copies of the original Mac mini ZIPs; original Mac mini evidence is unchanged.

| Candidate | Build | New ZIP SHA256 |
| --- | --- | --- |
| v0.2.0-alpha.1 | 3 | `09466cd6ad7ef0e6b803739475563c41ba489d0c3535986d3ddfb4520dee077e` |
| v0.2.0-alpha.2 | 4 | `13cbecfc3e64e5ecda1bd71a4c9107a71a7cfbe6bd2db8d6de4d8af3634c428b` |

Desktop native-ui release and both document helpers built from clean pinned
source with Rust1.98.1/locked/offline, macOS15 deployment; desktop jobs4 and helper
jobs1. Target graphs match completed helper receipts. Verified source/legal
notices were prepared from the exact cached archives. Normal packager validated
arm64, minimum15, system-library inventory, ad-hoc signatures and relocated copies.
External manifests/checksums are retained here. Installed A/B copies passed strict
deep codesign verification. [A identity](package-A-identity.json),
[B identity](package-B-identity.json), [A manifest](package-A.manifest.json),
[B manifest](package-B.manifest.json). Local manifests are unapproved proposals;
source still has legacy preview.2 approval. No alpha publication occurred.

[Relocated qualification](packaged-qualification.log.gz) passed startup/reopen,
foreign-data preservation, both helpers/all8formats and16 CLI imports. Builds and
packaging logs are retained. The separate matching test CLI remains the verified
MacBook CLI from the preceding continuation.

## Actual native route and isolation

Root: `/Users/Shared/BRN-native-functional-253-20261107` (mode0700).
Workspace is its `workspace`; Copilot folder is its `copilot-credentials`, initially
empty/mode0700. CLI configured Copilot/Luna/high, helper Off, no Codex auth file
and review-first before desktop startup. [Prelaunch settings](prelaunch-settings.json),
[isolation](isolation.json), [synthetic fixture hashes](fixture-identities.json).
Native Models and Data screens displayed those exact configured paths.

The packaged desktop launched directly with supported `--data-dir` and unchanged
HOME. Process inspection observed its database/derived-index files only under the
test root, and no open normal BRN workspace or sign-in file:
[process receipt](native-A-process.json), [open-file observation](process-open-files.json).
This is an observation of the inspected process, not an audit of all unrelated
applications or a claim to have read/hashed personal credentials.

Native automation could not select the unregistered direct process by executable
path/name. A separate test-only app wrapper was therefore generated using the
repository's supported `make-macos-app.sh --data-dir` mode, with unique bundle ID
`dev.brn.native-functional-253` and task-owned logs, then ad-hoc signed. All four
packaged desktop/helper/updater executable files were copied unchanged; the
desktop hash also matches after sealing. The original release app and its signed
launcher are unchanged. [Harness receipt](native-harness.json). This wrapper is
functional test scaffolding and cannot satisfy the ordinary release launch leg.
The initial direct test process has not been force-terminated; normal closure of
all test-owned BRN instances remains necessary before updater qualification.

## Observed functional results

- Native DOCX full import with AI Off succeeded without sign-in. Read view showed
  the120→72L/day table and chart, retained source/figure reference, Protected badge
  and honest unchecked-coverage/Xberg/AI-Off disclosure.
- A synthetic native-created note saved text and UTF-8 atv2. Discard confirmation
  removed an unsaved sentence while retainingv2. A later save becamev3; Undo on
  that exact newest Recent changes entry restored prior content asv4. Native
  search for12cm returned the saved article with its matching text.
- Native Data settings selected the task-owned backup folder and displayed full
  backup success with its exact path. SQLite quick_check passed.
- Native restore selected that backup and reviewed a new path under task-owned
  `recovery-copies` before confirmation. It displayed recovery success and opened
  a `BRN — Recovered workspace` window showing thev4 note and preserved text.
  The recovered window was then closed normally. Records (including Asset),
  notes, history, settings, projects and threads match the original; backup bytes
  are unchanged. [Native backup/observations](native-offline-so-far.json),
  [restore](native-restore.json), [state comparison](native-restore-full-comparison.json).
- Native Models again showed the exact separate Copilot folder, no Codex auth,
  helper Off and Luna/high. Connect started the normal device flow. Tester action
  is pending; no chat/tools/web/Luna-On/AI consolidation or writing pass is claimed.
  Device codes, account identity and credential files are excluded from evidence.

The new local transport-substituted update attempt exited1 because BRN windows
were running. The stock guard refused normal replacement and terminated no
process. [Original refusal log](local-substituted-update.log.gz),
[incomplete qualification disposition](local-update-status.json). Rerun only after
normal closure; no force quit, process-guard bypass or stock HTTPS result claimed.

## Remaining source and user gates

Current main has since advanced to75f37221 through auth-race repair
[PR272](https://github.com/ewq100/brn-rust/pull/272). That repair prevents an earlier
token exchange from overwriting a later sign-in/sign-out. It is not in thesea921
artifacts. Preserve these source-bound functional observations; reconcile the
final release source and requalify affected account behavior before publication.
No account-switch success on the old runtime is claimed.

The next actual user action is normal Copilot device login in the test app's
already verified folder. B then continues live and lifecycle checks. No new OS
account is needed for that leg. A standard disposable account is a straightforward
later method for the still-pending clean-user/ordinary-launch leg; describe its
minimum action when the available functional checks are finished. Release approval
and actual approved hosted A/B remain independent prerequisites for published
browser/Gatekeeper and stock HTTPS acceptance.
