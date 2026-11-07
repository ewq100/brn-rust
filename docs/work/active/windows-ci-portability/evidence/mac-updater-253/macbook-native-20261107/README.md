# MacBook alternate-directory native functional observations

These are preserved a921-source observations. The [new combined source82657cab
qualification](../combined-82657cab-20261107/README.md) owns the later272+267+target-fix
packages/native rechecks and current locked-screen closure prerequisite.

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
- Native Models showed the exact separate Copilot folder, no Codex auth and
  helper Off. The tester completed a fresh normal device flow after the earlier
  flow expired. Copilot/Luna/high remained selected; high is requested effort,
  not proof the provider honored it. Device codes, account identity and credential
  files are excluded from evidence. [Login receipt](native-login.json).

## Post-login native continuation

- Copilot Luna used `read_note` and cited saved v4, continued the conversation,
  and later read v7 after normal reopen without another sign-in. Web Off returned
  no search sources. A first Web On reply contained only a model-written URL and
  is explicitly **not** counted as search success. A fresh native search then
  returned **13 actual run-owned web sources** and cited one returned URL. Stop
  interrupted a working turn; no late sources appeared on recheck. This does not
  prove cancellation of an upstream HTTP request or quota charge.
  [Chat/web/Stop and correction](native-copilot-chat-web-stop.json),
  [reopened tool-backed reply](native-retained-session-reply.json).
- Native DOCX refinement On succeeded through the selected Copilot Luna route,
  retaining the120→72L/day table, date, pending exception and figure asset. The
  synthetic variant differs from the earlier Off fixture to prevent deduplication.
  [Fixture identity](refinement-fixture.json), [result](native-luna-refinement-on.json).
- Referenced writing Discard leftv4 unchanged. A second exact proposal with the
  original source link applied asv5; native operation Undo restored exact baseline
  asv6. A third candidate prepared fromv6 refused Apply after a checked synthetic
  owner save advanced tov7; the newer owner text and pending proposal survived.
  [Apply](native-writing-apply-before-undo.json), [Undo](native-writing-undo.json),
  [stale refusal](native-writing-stale-refusal.json).
- Consolidation read two contradictory source notes and proposed one new article
  with two real source Link records. Review retained current12m/s/steel/P8-only
  requirements and superseded9m/s/timber/original exception history. Native Apply
  saved the new3-record group; Undo archived all3 asv2. Both originals remainedv1
  and the newer working notev7 was unchanged.
  [Applied group](native-consolidation-applied.json), [Undo](native-consolidation-undo.json).
- **Finding:** that new-article review showed the previously open note's title/
  Saved state and a misleading “Working Note is unavailable” banner. Its actual
  request, Apply and Undo targets were correct. The source correction
  now passes the exact red-before/green-after regression and all18review tests.
  New-package native reobservation remains pending. This is not hidden
  behind the successful canonical-write checks.
  [Exact pending request and observation](consolidation-before-apply.json).
- Native project creation and linking succeeded. Normal Quit/reopen retained the
  unsaved UTF-8 recovery draft and exact notes/projects/threads/history/settings.
  Copilot then completed another tool-backed reply with no fresh device login.
  [Reopen](native-lifecycle-reopened-A.json),
  [baseline digests](native-lifecycle-baseline-digests.json).
- A synthetic model absent from the authenticated catalogue refused before a Run,
  retained the owner message, and offered supported models. Luna was restored.
  This is not policy/quota/absent-Luna or network-outage qualification. During this
  settings experiment CmdS saved the underlying recovered synthetic note; the
  recovery pass preceded that action. Actual settings submission used the visible
  Save settings button. [Bounded refusal and limitation](native-unavailable-model.json).
- A second native full backup/new-copy restore passed after login, with exact
  comparison of all6categories:58records,5notes,83history entries,1project and
  5threads, including imported figures and run sources. Credentials remain external.
  [Post-login backup/restore](native-post-login-backup-restore.json).

The first local transport-substituted update attempt exited1 because BRN windows
were running; its [original refusal](local-substituted-update.log.gz) remains.
After normal Quit, process inspection found no BRN desktop. The same qualifier
then passed real signed local A3→B4 replacement, unchanged external files, B
headless reopen and backup/restore. [Passing report](local-substituted-update-passed.json),
[rerun log](local-substituted-update-rerun.log.gz). It uses an independent disposable
synthetic store and substituted transport, without live credentials. It does not
establish native B reopening or stock published HTTPS delivery. The functional
wrapper was normally quit at the end; native app inventory confirmed not running.

## Remaining source and user gates

Current main has since advanced to75f37221 through auth-race repair
[PR272](https://github.com/ewq100/brn-rust/pull/272). That repair prevents an earlier
token exchange from overwriting a later sign-in/sign-out. It is not in thesea921
artifacts. Preserve these source-bound functional observations; reconcile the
final release source and requalify affected account behavior before publication.
No account-switch success on the old runtime is claimed.

No further device code is pending. Current-account functional work has progressed
without creating an OS account. Remaining local work includes the review-target
presentation correction and final-source/package reconciliation with affected
qualification. Actual second-account/in-flight account switch, policy/quota denial,
absent Luna and network outage remain unobserved optional/account-bound cases.

Main now includes provider refactor267 as8b07e035. The first-alpha instructions
exclude that refactor, while the existing release workflow publishes only an exact
current-main SHA. The owner subsequently selected currentmain with272and267 plus the focused
review-target fix for new qualification only, as recorded below. Publication
policy is unchanged; currenta921packages/observations retain their exact scope.

For the later clean-user/ordinary-launch leg, the minimum owner action is to create
and sign into a disposable **standard** macOS account with no BRN data/credentials,
then provide its short name/readiness and remain available for ordinary Gatekeeper
and Copilot sign-in. B will use the unchanged release launcher there. A new account
does not remove system-wide developer tools. Actual published browser/Gatekeeper
and stock HTTPS A→B still require explicit release approval and approved hosted
artifacts. No alpha is published, approved or owner-accepted by these checks.

## Qualified source correction and next candidate

The [focused correction](review-target-correction.json) avoids opening an unsaved
creation as an existing working note, names the exact proposed target and withholds
unrelated retained-editor actions. Shared checked prepare/Apply/Undo is unchanged.
The test reproduced the old title before the fix and passes after it; all18review
tests pass. The full native-state suite passed451tests with1existing CLI-supplied
fixture ignore at2testthreads and canonical task-owned TMPDIR. First default-temp
and parallel attempts are preserved, including the remaining worker failure under
default parallelism; its isolated run passed. No assertions, deadlines or CI gates
were relaxed, and baseline/default-parallel reliability is not claimed.

The owner has now explicitly selected currentmain including272and267 plus this
focused fix for **new candidate qualification only**. This resolves the earlier
first-alpha source exclusion for that qualification. Publication remains unapproved.
Merge currentmain normally, preserve all track records, pin/build the new source,
record new package identities and repeat affected account/chat/web/refinement and
review/lifecycle native checks. Existinga921packages/receipts stay unchanged.
