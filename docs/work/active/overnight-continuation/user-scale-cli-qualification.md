# Shipping CLI qualification at 5,000 Current notes

Selected next independent V1 outcome,9October2026. Vision44 targets roughly5000
active notes. Baseline Activity279395d56ab744d1a1d0138ca069fa92294c4d39, qualified
immutable runtime sourceb873f273832f881f03b483c079c94fd06a1a6d51. This is a bounded
qualification slice, not a production feature/optimization/SLA or GUI acceptance.

## Reuse and bounded campaign

Reuse shipping brn status/notes/search/proposals/inbox commands, existing scoped
validated readers and disposable index rebuild. Pythonstdlib only for synthetic
fixture/receipts/hash/timing orchestration. No second document interpreter, schema,
new product query/dependency/cache/search policy/native model or provider. Existing
scanner has1MiB per-note cap but no5000countcap; NotePage uses200+1 keyset pagination.
Each freshprocess refreshes the full corpus; measure it honestly, no claimed warm
inprocess latency. Existing201-note/keyword/rebuild witnesses remain applicable.

Fixed invocation scripts/qualify-user-scale-cli.py with explicit absolute --runtime
and --case-root. Require exact runtime manifest CLI SHA and new nonexistent owned
case path; refuse reuse/recreation/reset. Root selects
/private/tmp/brn-overnight-20261008/user-scale-case. Retain every input/output/stderr,
status/error/timing/manifest. Empty synthetic credentials and modeldirs only.
Generate5000 deterministic managedUUID Current Markdown notes across50folders, exact
manifest of path/UUID/fullbytesSHA andphysicalidentity, Unicode/BOM/CRLF notes and
unique ASCII tokens for first/middle/last. Source/history/semantic retrieval outside
this slice. No inferredrealcontent. Keep one ordinary unapprovednoteproposal and
one ordinary TextInboxoriginal as operational/originalretention guards; no approval.

Hard campaigndeadline600seconds inclusive, atmost60seconds perchildcommand. On error/
timeout stop and preserve fixture/logs for diagnosis; never reset/retry automatically.
These are qualificationcampaign limits, not permanent product restrictions. All
processes must exit before moving only the explicitlyowned index.sqlite family into
retained subfolder; never movebrn.sqlite/vault/Inbox/editorfiles. No competing Cargo
or running native application; no optionalmodeldownloads or privateownerdata.

## Acceptance

1. Recordcold status startup. Traverse noteslist Current to nullcursor: exactly25
   pages/5000unique sortedpaths/UUIDs, fullfactsSHAequalfixturemanifest; no clipping.
2. Unique first/middle/last tokens: keyword-only Currentsearch returns exactpath,
   rangebytes andquote. Fullnotesshowtexthashes matchmanifest. CLIsearch currently
   omits note_sha256; obtainexplicitfullhash fromlistedfacts/fulltext instead, never
   invent absenttelemetry. Repeatthree search/read witnesses in freshprocesses.
3. Retain/moveowned disposableindexfiles afterprocess exit; normalstartup/rebuild.
   Repeatall25inventorypages andthree search/readwitnesses. Typedproposal/Inboxguards
   and originalphysicalproofs,5000note bytes/identities,emptyconversations/Actions and
   emptycredentials/models remainexact. OperationalSQLite rawbytes neednotmatch.
4. Reporttotal/startup/perpage/search/rebuild observedtimes anddatasetbytes/environment.
   No inventedthreshold, comparativeperformancegain, semantic/nativeGUI/usability/
   original-format acceptance or5000complexapprovalrecovery claim. Meaningful failure
   staysfailed evenifpartial output retained; no slowtest/fixture shrinking.
5. Independent read-only complete script/contract/result review, scriptcompile/help,
   actual boundedcampaign, links/diff andactualrequiredCI/protectedintegration ifready.
   Productcode/manifests unchanged, existing productqualification reused. New GUI
   checklist only inONE morningtask, optional5000note browsing/search expectedpending;
   functionalcore essentialpath doesnotneed thisoptionalextendedscaleobservation.

Rootowns plan/status/morningtask/report/artifacts/integration. Boundedhelper owns
ONLYscripts/qualify-user-scale-cli.py; no Cargo,provider/GUI/privatefiles/account/
spawning/sharedfiles/productcode/fixtureexecution untilrootgrant. Leadretainsselected
model/effort; helperbenefit isolatedorchestration implementation whilelead handles
ActivityCI/merge andreview. Atmost2helpers/no recursion/oneCargo. Campaign16/16BRN
investigationsused, no newmodelallowance. Finalqualification04:30UTC,cutoff05UTC;
stop/reassess for timeout/newauthority/current-private-data dependency. Preserve all
existing fixtures/branches/userwork and exactapproval/provenance/recovery guarantees.


## Qualified result — 9 October 04:11 UTC

The reviewed corrected script at `8c4524c2f3846c40bc1f0c8b6723cb972d0809f0`
(SHA-256 `ab6213c009a6ef8db45094cec1e2aef83d9afc5adcd8681cb0d058fcb302b3ec`)
passed the complete campaign in **80.957 seconds**. All **83 fresh CLI processes**
completed successfully within both deadlines. Dataset: 5,000 Current managed notes,
50 folders, 4,745,663 bytes, including Unicode, BOM and CRLF. No provider calls,
model assets, downloads, account activity or GUI observation occurred.

Both complete 25-page inventories matched all paths, UUIDs and full note hashes.
First/middle/last unique-token searches returned exact paths and UTF-8 quote ranges;
full reads matched manifest hashes before restart, after restart and after ordinary
rebuild of the retained disposable index. The unapproved proposal, Text Inbox
original/physical proof, all 5,000 note byte/identity proofs and empty Actions,
conversations, credentials and model folders stayed exact. No reset/recreation,
re-import, conversion or inference was needed. Every input/output/error/timing and
index-retention receipt remains in `/private/tmp/brn-overnight-20261008/user-scale-case`; do not rerun this guarded case.

| Observed CLI operation | Seconds |
| --- | --- |
| Cold startup/status | 1.452 |
| Fresh-process restart/status | 0.613 |
| Index rebuild/startup | 1.424 |
| Inventory page, median / maximum of 50 | 1.098 / 1.113 |
| Anchor search, median / maximum of 9 | 1.049 / 1.076 |
| Full anchor read, median / maximum of 9 | 0.591 / 0.634 |

These are wall times on this Mac and short synthetic Current notes, including
fresh process startup/scans. They establish this bounded correctness/retention
witness, not a numerical SLA, an optimization gain, in-process latency, native UI
performance, semantic retrieval, larger History or complex approval recovery.

Complete seven-file independent review found one P2: cancellation between child
creation and cleanup registration could leave a child unjoined. The lead reproduced
it with a deterministic signal-boundary witness before executing any BRN campaign.
It was fixed by deferring handled signals during creation/registration, registering
the child, then immediately killing/reaping before reporting interruption. Child
signals are not masked. A local finally cleanup also owns returned children.
One regression test passed seven subcases: all three handled signals at two
boundaries plus one real short Python child. The original red witness passed after
correction. Fresh independent correction review is clean at8c4524c; complete delta
`e6132973869da0028963dd16bfb5da94adb7d65d8716315d4a57f688c76be2c5`.
Initial review patch: `41059022cc6ebab0e2b28ab8dd2a63aae46b9ae2bfa92b2b3c110d6ce574d182`.
No product code, dependencies or runtime changed; existing product qualification
and exact shipping runtime were reused. All-candidate Python tooling/links checks,
final documentation review and required hosted CI/protected integration remain gates.
Optional native scale observation is solely morning Workspace P / step27.
