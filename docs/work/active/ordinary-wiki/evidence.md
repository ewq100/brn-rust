# Ordinary wiki guidance evidence

**Baseline:** `65119a73a6aaea13764b8bc6290f7bb6dc79ff2d`, origin/main reconfirmed 13 October 2026. **Isolated branch:** `codex/ordinary-wiki-guidance`; research commit `f7e150a9`; evaluated guidance commit `ef0e2e6abdc0469d757f87f587720bef52a5379e`. Main's unrelated untracked `experiments/` remains preserved.

**Owner scope:** [spec](spec.md). Runtime implementation and controlled comparison authorized; UI design/prototype first; Codex `gpt-6.1-sol`, medium. Only fresh synthetic workspaces were used. No release, owner-data access or external messages.

| Gate | State |
|---|---|
| Defined | Owner requested wiki guidance/evaluation and selected UI prototype plus exact live model/effort. |
| Environment | Read-only preflight passed: repository Rust 1.98.1, Cargo, Python and Git available. Dependencies/lockfile unchanged. |
| Plan review | Independent review approved round 2. Added applied input-lineage inspection and distinguished protected imported evidence from confirmed owner authority. |
| Implementation | Bundle 3 and controlled CLI harness implemented; same four guides, nine tools, schema, checked service, protection and Undo. No native UI change. |
| Automated verification | Baseline and candidate each passed 363 runtime/core/app tests, one pre-existing ignored. Relevant strict Clippy and formatting passed. Final Python suite: 50 passed, including 14 harness integrity tests. Initial Markdown gate: 52 files, 214 local links, zero failures; earlier navigation follow-up: 53 files, 223 local links, zero failures; accepted tiled-flow follow-up: 53 files, 227 local links, zero failures. |
| Live comparison | 10/12 scenario arms completed; 14/16 provider invocations completed. Wiki: 6/6 arms; baseline: 4/6, with two Network failures retained. All 10 completed arms pass final structural checks; six eligible actual Undo groups restored affected data and preserved unrelated records. |
| Semantic assessment | Blinded independent review of frozen inputs, saved notes and answers: all six complete substantive samples preserve tested facts/qualifications/chronology; four arithmetic controls stay brief. No semantic superiority established. |
| Independent candidate review | No remaining actionable runtime/harness defects after three P2 harness corrections. Original live snapshots pass the final validator; no extra provider calls were needed. |
| UI prototype preview | Three separate throwaway variants browser-previewed at desktop/narrow sizes; B selected and extended with topic pages/search and a bounded local graph in adjacent tiles over 500 synthetic Notes; [UI mapping and recommendation](ui-design.md). Independent mapping/prototype review found no material defects. This is not native qualification. |
| Owner acceptance | Wiki policy, exact model and prototype-only scope selected. B — subject overview with evidence chronology, topic pages/search, local Connections and tiled UX owner-selected on 13 October 2026; live results not owner-accepted and native interaction unobserved. |
| Publication / merge | Reviewable draft candidate published on `codex/ordinary-wiki-guidance`; hosted PR/CI state is tracked on GitHub. Not merged or released; current assignment supplies no merge/release authority. |

## Controlled pilot

Two compiled CLIs use identical runtime code and differ in packaged guidance. Fixture content/prompts/rubric were frozen before the guidance edits. Each arm uses eight tool rounds, two repetitions and alternating arm order, with a 180-second command timeout and controlled cancellation. No automatic provider retry or fallback. Both binaries are retained outside Git under `/private/tmp/brn-wiki-evaluation/bin/`.

| Artifact | Identity |
|---|---|
| Baseline CLI / bundle 2 | SHA-256 `20c5151df943979f04d31312a1b594f4309d177cc23bfc7efe193c05d7228e16` |
| Wiki CLI / bundle 3 | SHA-256 `6fcb96ad8e129b8febef00b8cc1eae637491b92f30862edf86f48dbb80aa11be` |
| Frozen fixture | SHA-256 `6f8e0666fccd5e52986702732cc7c041a94f2ce109522b47fe03ee0848c6e7dd` |
| Original live-2 comparison | SHA-256 `46d7cc06ae3e428e332db74e7e7eb9f957eb3c53ac2071e82fc491f947f269e9` |
| Blinded semantic pack | SHA-256 `0b247a6be53086afdf570c9219d3b8201e1c12375c14c5e90a858248b08d0687` |

| Scenario | Baseline | Wiki | Observed result |
|---|---|---|---|
| Sequential Harbor updates | Repetition 1: first update saved, second Network failure. Repetition 2: complete. | Both complete. | Completed arms update the existing subject, retain the conditional 15 November estimate versus the protected source-reported 22 November commitment, and keep older 8 October planning evidence older despite later ingestion. Quantities, meeting and Jaan attribution retained. |
| Saved process comparison | Repetition 1: complete. Repetition 2: Network failure before synthesis. | Both complete. | All three completed comparisons preserve thresholds, units, negative/missing rules and conditional inference. One saved synthesis and its actual writing operation contain current-version inputs and typed relationships to both references. |
| Conversation-only arithmetic | Both complete. | Both complete. | Brief correct 102; no new Note, Action or attention. No eligible note change to Undo. |

Protection/content preservation passed in all 12 after-snapshots, including incomplete arms. Six completed substantive arms had an eligible current applied model Note group; actual checked Undo passed affected restoration/new-note archival and unrelated-record preservation. Four completed arithmetic arms explicitly report no eligible Undo. The two incomplete arms were not undone. These are scenario-specific observations; existing integrity tests cover broader enforcement and concurrency guarantees.

The original live report is immutable. Final tightened validation reread its before/after/lineage/Undo snapshots into `/private/tmp/brn-wiki-evaluation/final-validation.json`: 10 completed, 12 protected snapshots preserved, six successful Undo groups. This validates captured host state, not a new provider run. Both Network failures remain unsuccessful and excluded from semantic comparison; transport completion differences do not demonstrate guidance-caused reliability.

## Semantic conclusions and limits

[Independent blinded assessment](semantic-review.md) used a shuffled pack containing only frozen sources, prompts, rubric, ordinary saved notes and answers; arm labels, repository, raw host report and label map were withheld. The reviewer found no tested factual loss in the six complete substantive samples. Both completed baseline substantive samples were also good. Four wiki substantive samples plus two baseline samples are too small and uneven for a general accuracy claim.

After the review, the label map identified Sample 09 as wiki sequential repetition 1: it names incoming sources/dates/paths without visible inline source-record navigation links. Actual typed relationships and inputs passed host checks. This informs the proposed evidence rail; it is not a provenance failure or proof of better UI. Organization varied across both arms: current-first versus history-first, compact comparison versus redundant caveats/fingerprints.

The semantic pack cannot establish actual source-reader use, external availability, applied input versions, receipt outcomes, Action state or host-confirmed authority. Structural evidence separately checks the specified host facts. Claims of unavailable originals or coverage must be interpreted against recorded observations, not certified by prose. Token usage is not exposed by the production CLI, so token/dollar savings are unmeasured. Elapsed time is recorded but does not establish a speed win. Long-term maintenance, retrieval quality, missing evidence, large wikis and native usability remain unevaluated.

## Failure and review record

The first live attempt (`live-1`) failed before generation because the harness configured both auth-file and credentials-dir; the latter cleared the former and selected an empty cache. Its failure inspection also dropped schema nulls. Evidence/logs and exclusively owned scratch databases are preserved; this attempt supplies no semantic result. The harness now chooses one explicit credential route and preserves nulls while redacting configuration. Existing connection credentials were neither changed nor copied into committed evidence.

Independent candidate review found three P2 harness issues: late verification exceptions could discard collected evidence; synthesis checks could combine disconnected operations/notes; false checks or unsuccessful Undo could still count as completed. Corrections persist staged results, bind both references to the same saved synthesis and its current-version writing operation, and require successful structural/Undo verification for completion/exit 0. Negative regression tests and mocked integrated counterexamples verify each. Legitimately ineligible Undo remains explicit and allowed. Runtime guides and evaluated binaries did not change during these harness fixes.

## Reproduction and checks

Build target: `/private/tmp/brn-wiki-evaluation/build`. Logs: `/private/tmp/brn-wiki-evaluation/logs/`. Use Rust 1.98.1 and the unchanged lockfile. Equivalent build/check commands:

```sh
CARGO_TARGET_DIR=/private/tmp/brn-wiki-evaluation/build cargo build --locked --offline -p brn --bin brn
CARGO_TARGET_DIR=/private/tmp/brn-wiki-evaluation/build cargo build --locked --offline -p brn-intake --features helper --bin brn-intake-helper
BRN_INTAKE_HELPER=/private/tmp/brn-wiki-evaluation/build/debug/brn-intake-helper CARGO_TARGET_DIR=/private/tmp/brn-wiki-evaluation/build cargo test --locked --offline -p brn-ai -p brn-threads-core -p brn-threads-app
CARGO_TARGET_DIR=/private/tmp/brn-wiki-evaluation/build cargo clippy --locked --offline -p brn-ai -p brn-threads-core -p brn-threads-app --all-targets -- -D warnings
cargo fmt --all --check
python3 -m unittest discover -s scripts/tests -v
python3 scripts/check-markdown-links.py
```

Baseline build/tests preceded guide edits; candidate build/tests/Clippy qualified the bundle-3 runtime tree. Later changes affect only the evaluation harness, tests and task documentation. Final Python suite passes 50 meaningful tests; zero-test doc-test sections are not included in the 363 total. The initial missing-module harness red witness and review regression red witnesses remain in local logs.

The final offline harness passed at `/private/tmp/brn-wiki-evaluation/offline-final/comparison.json` and uses the two retained binaries, shared helper, fresh root, baseline/candidate identities and `--repetitions 1`; it prepares six arms and marks provider/semantic evaluation not run. Live-2 used the same arguments plus `--live --provider Codex --model gpt-6.1-sol --effort medium --repetitions 2` and one directly selected connected auth profile. The profile path is intentionally omitted from evidence. Credentials and configuration records are excluded recursively from JSON snapshots.

Applied input lineage is read only from the exclusively owned synthetic `threads.sqlite3`, limited to Run.operations matched with recorded applied receipts and whitelisted input/version/expectation fields. Pending candidate/history CLI output alone does not expose applied inputs. No full operation request or owner database is emitted.

Raw artifacts remain under `/private/tmp/brn-wiki-evaluation/`; they can expire with local temporary storage. The frozen harness, hashes, semantic assessment and this bounded result record are committed. Hosted CI, native observation, owner UI selection and any integration are separately recorded gates.

## Navigation follow-up

On 13 October 2026 the owner selected B, then requested navigation research for hundreds of notes. [Primary-source research](../../../research/notes-navigation-at-scale.md) compares searchable subject indexes, shallow collections/tags and bounded local connections. It recommends indexes plus local connections as a design hypothesis. The B prototype replaces its note strip with a 500-entry synthetic library; 12 visible results per page, result counts/snippets, search by remembered detail, Enter opening, ⌘K, Recent, linked/protected browsing, letter filters and no-results state were browser-inspected, including a 720×1000 view. No native search relevance/performance or all-graph scalability is claimed.

The navigation follow-up changed only research/design records and the separate HTML prototype. Independent review found one incomplete demo text index; full worked-note bodies plus matching snippets corrected it, with Tuesday/Medical emergencies and correct-note opening verified. No remaining material findings. The evaluated bundle-3 runtime and provider pilot are unchanged.

## Accepted topic/search/Connections prototype

The owner selected topic pages and search as main navigation, a local graph under Connections and Omarchy-style tiles. B now implements the synthetic flow described in [UI design](ui-design.md). Three topic index Notes and four subject/reference Notes share explicit links; 493 marked placeholders bring the library to 500. No category metadata, global graph engine, native UI or packaged guide changed.

Browser checks at 1440×1000, 720×1000 and the default 374×853 viewport verified topic → subject → evidence, Jaan search/Enter, local graph recentering, optional sources/second hop, exact neighbor counts with an eight-neighbor cap, retained passage inspection and navigation back. Header drag swapped tile order; gap drag resized navigation; close/reopen and maximization worked. Narrow subject tiles fit available width, selection reveals the relevant tile, and the row/contents scroll separately. Screenshots are retained on the prototype branch; temporary viewport override was reset.

Independent B review found two P2 state defects: unbound thread replies disappeared on rerender, and focus cycling could select a tile hidden by maximization. Separate per-destination reply buffers and the visible-tile focus predicate corrected them. Re-review witnessed reply preservation across context/navigation/maximization, separate composer destinations, retained Note drafts and disabled dirty Undo, with no remaining material findings. Lead browser checks also verified maximized subject/context focus and narrow tile reading. The preview is synthetic and memory-only; it supplies no native interaction qualification or measured large-workspace usability improvement.

This follow-up changes only documentation and the separate prototype. Previously evaluated runtime code/binaries and live comparison remain unchanged; no additional provider calls or broad runtime reruns were needed.

The final consistency review also caught a stale primary-source pointer. The [UI record](ui-design.md) now pins the published prototype and run instructions to `61f33b954591036c7e5b3230fcf8159442f31de0`; no material findings remain after that correction.
