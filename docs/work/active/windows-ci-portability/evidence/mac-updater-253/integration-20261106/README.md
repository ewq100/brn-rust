# Updater refreshed integration — 6 November 2026

PR258 final head0438bd603a86b48d4956195b89aea0fb54e648d8 normally merged
main6897fe56 (Rig259, Copilot261, C262). Normal merge28051188f7d20a817202ddf098dff5dc2f07e653
has exactly the reviewed tree; [tree identity](tree-identity.json) records it.

[PR CI42276377509](https://github.com/ewq100/brn-rust/actions/runs/42276377509):
all7 lanes pass. [Captured receipt](pr-ci.json.gz). Local tooling360total =
349passed +11skipped: [raw log](local-tooling.log.gz). Earlier353total =
342passed +11skipped remains the corrected historical result.

[Main CI42279604581](https://github.com/ewq100/brn-rust/actions/runs/42279604581)
passed all4 required contexts; its overall failure is confined to the three
informational Windows lanes. [Captured full run receipt](main-ci.json.gz).
Exact excerpts:

- [Mac native job128104813019](main-mac-updater-excerpt.log): 25 packaging/updater
  checks passed, including all19 updater failure/concurrency/recovery cases.
- [Ubuntu Core job128104812944](main-ubuntu-fixture-excerpt.log): the previously
  failing response-identity/limits test explicitly passed at19:34:22Z.
- [Windows native128104813140](main-windows-native-failure.log.gz) and
  [combined128104812903](main-windows-combined-failure.log.gz): inherited
  threads_composer unresolved components/theme/ThreadsUi/Context/Window errors.
- [Windows Core128104813293](main-windows-core-failure.log.gz): inherited
  skills.rs dead-code Clippy errors for COMMAND_DESCRIPTION_CHARS and
  command_description. Both affected files have identical blobs before/after
  the updater merge; no Windows repair or suppression is included.

Independent read-only reviewer integration_review inspected the complete task
delta on0438bd60:206 main changes preserve blob/mode/deletion identity,48 prior
updater paths are identical, main's4 prior task objects/baseline/authority are
preserved,39 original evidence checksums match. Updater validation, locking and
recoverable replacement, launcher, packager receipts, Mac CI, static Linux
fixture, stock preparation/qualifier and conflict resolution were reviewed.
No actionable code/security findings. Seven preparation tests, overview, links,
shell syntax and diff checks passed. No new native/provider/build evidence was
claimed by that reviewer.

[Review receipts](review-receipts.json) retain current0438bd60 Codex code and
separate security no-issue reports, the independent review checkpoint and the
clarification of private full-detail report access. The browser required account
login, so the private report was not inspected. Independent review is additional
security evidence; workflow text does not itself grant a fallback waiver.

This source integration does not select the final package ahead of A264, prove
native usability, qualify stock published HTTPS delivery or authorize publication.
The later acceptance branch's proposed notes/manifests are documentation only;
lead inspection, link/checksum/overview/diff checks apply without another
independent behavioral review. The actual release manifest is unchanged.
