# BRN Threads setup evidence

**Date:** 10 October 2026. **Scope:** repository preparation and handoff, not product implementation.

## Observed baseline

- Remote main: `af9239c741c7ab0983e62f0253e607b9607727e6`.
- Base tree: `b2d7917d15b0fd7a83b436a1f5462a5cf729b3e4`.
- Main includes PR #113. Existing branches and open PRs were inspected; none was merged, closed, deleted, or force-updated by this setup.
- A fresh clone was made from main and local branch `rebuild/threads` was created. The earlier checkout on `ci/hosted-platform-checks` was preserved.
- Root `AGENTS.md` was the only preexisting tracked agent instruction file. Small Claude/Copilot entry points now refer to the same new guide.

## Changes prepared

The latest proposal, including full readable imports and on-demand Markdown export, becomes one canonical Threads target. Current product, architecture, workflow, status, roadmap, and active-work entry points route to it.

Baseline crate/dependency documentation is labeled as reference. Links in retained current baseline documents that depended on overwritten old contract sections now point to the same files at the fixed baseline commit. Historical evidence was not rewritten as current results.

The setup adds the build plan, review brief, pending review record, and exact build prompt. Product source, Cargo manifests/lockfile, provider patches, CI workflow, and repository protection are unchanged.

## Checks

| Check | Observed result |
|---|---|
| Remote baseline and branch inventory | Completed through the repository connection. No existing rebuild/threads branch was observed before preparation. |
| Isolated checkout identity | Main baseline confirmed before local branch creation and setup edits. |
| Development preflight with native flag | Exited 1: Git, Bash, and Python available; Cargo, Rustup, pinned Rust, and protoc absent. This is an environment prerequisite gap, not a product test failure. No Mac native session is available here. |
| Git whitespace check | Passed: git diff --check returned 0. |
| Current Markdown file/fragment links | Passed: 84 files, 408 local links, 0 failures. |
| Setup scope and consistency review | Completed in a separate read-only agent context. A fetch-ref correction was accepted. The actual single-branch checkout additionally exposed a missing tracking mapping; the setup now registers the branch and fetches into its explicit tracking ref. Both fetch and upstream setup succeeded after correction. No other concrete setup blockers were found. This was not the external Opus/Astra architecture review. |
| Rust compilation, model tests, native interaction | Not run: no product code changed, and this environment lacks required tooling/native resources. |
| Independent external Opus/Astra review | Not performed. The supplied brief enables the owner's next review assignment. |
| Remote publication | Setup commit c7e3eda380c1fc069ff4bafbb162c4e56557a95d was published on rebuild/threads. Its complete tree matched the local staged files exactly. This evidence update and the tracking correction are a follow-up documentation commit; use the branch head for the latest handoff. No main merge or release is implied. |

## Build machine handoff

The build agent must fetch `rebuild/threads`, inspect current HEAD and [review status](review.md), and run the [setup preflight](../../../development/setup.md) on its own machine. Use a fresh data directory and separate Cargo target. This evidence does not claim that the owner's Mac, credentials, model access, or new core are configured or verified.

## Review reconciliation and behavior design, 10 October 2026

This later update starts from review commit `a2af886e5442af0cfa4b8884181446c7d98d1eea`, which changed only the independent report after setup. The original setup evidence above describes that earlier assignment. The report names Claude Opus 5.5 as reviewer and assesses setup commit `3519bfb85e3c68cb17dd6df31061e9f5dc155e73`; its original findings are preserved, with lead dispositions appended separately.

The owner clarified that mathematical equation imports are not important, asked for concrete thread and agent-guide behavior, and pointed to new Rig and GPUI Kit releases. The target, plan, current summaries and build prompt now incorporate those decisions. The new behavior design specifies thread/attention/run/Action distinctions, a small packaged guide catalog and four planned SKILL.md playbooks. This task created design documentation, not the runtime guide files or their loader.

Three bounded assessments examined core-review corrections, Rig reuse and GPUI/import reuse. Root reconciled their findings and inspected official release/source evidence. Current pins remain Rig 0.43.0 and Kit 0.6.6. Rig 0.44.0 and Kit 0.7.1 are qualification targets, not completed upgrades. The direct shared ChatGPT page could not be fetched; prior-context retrieval recovered the earlier Rig recommendation and its important retry/logging claims were checked against release source.

Documentation checks were run after the changes; see the final recorded result below. All changed paths in this update are Markdown. No product source, dependency pin/lockfile, runtime behavior, native environment or provider qualification changed. The build environment limitations from setup still apply.

Final documentation result for this reconciliation: `git diff --check` passed; `python3 scripts/check-markdown-links.py` passed with **85 files, 434 local links, 0 failures**. The original independent-report body was checked unchanged before the lead-disposition section. The published commit/branch head is the handoff identity; no runtime test result is implied.

## Toolchain and remaining dependencies, 10 October 2026

This follow-up assessed the clean branch at `c2881e84958fde525fa03b361973baa30801007e` after the owner asked about Rust 1.99.0 and other updates. Three bounded read-only assessments covered the compiler, remaining direct dependencies and CI/dependency hygiene; root checked official releases, registry entries and the actual repository pins.

The plan now selects Rust 1.99.0 for compiler-only qualification before the separate Rig/GPUI changes. It identifies six verification scripts forcing Rust 1.98.1 and requires one consistent repository-selected compiler. CI setup and preflight already read the root pin. The remaining shortlist keeps current SQLite/BetterOffice/image versions, defers ZIP/HTML decisions to import qualification, avoids low-value patch churn and records a separate checkout-action maintenance candidate.

This is a documentation handoff update. Rust 1.98.1, Rig 0.43.0, Kit 0.6.6, the lockfile, scripts and CI remain unchanged. No compiler/native/provider qualification or complete advisory audit was run; the earlier machine limitations still apply. Source findings support the chosen next checks, not a claim that the upgrades pass.

Documentation validation: `git diff --check` passed; `python3 scripts/check-markdown-links.py` passed with **85 files, 437 local links, 0 failures**. Only six current Markdown handoff files changed in this follow-up; historical review and implementation evidence were preserved.


## Build started on the target Mac, 10 October 2026

Baseline: `a1c61e245ec90a0eed98ffc5bbfcab57ddeb1926`; isolated checkout `/Users/evokessler/repos/brn-threads`, tracking the fetched remote branch. Other worktrees/dirty files are preserved. Preflight found Rustup1.98.1, protoc and Apple clang. Cargo proxies are available under `/opt/homebrew/opt/rustup/bin`; initial PATH omission was corrected. Rust1.99.0 installed with rustfmt/Clippy; formatter check passed against the unchanged dependency lock SHA-256 `16fe749fc0e4592f2bf2065c660203ded43e5c1c7b03c803d52abceaa9a8ffb7`. Full compiler qualification remains in progress, separate from library upgrades.

The first workspace test attempt failed four CLI intake cases because the existing helper binary had not been built. The documented helper build then passed and the affected workspace qualification was restarted. This is a baseline prerequisite correction, not a Threads behavior result. Task-owned targets/evidence are outside Git under `/Users/evokessler/repos/BRN-local-builds/threads-20261010`. No old BRN data was opened.

Native inventory reports the Mac locked. No unlock attempted. One native interaction qualification task is retained in the [implementation record](implementation.md); compilation is not native usability. Selected provider/model settings reside in old app data; the exact route and credentials-directory path were requested from the owner without inspecting it. Offline work continues. Official Homebrew Poppler26.10.0 installed for the bounded PDF qualification path. No provider call, external message, merge or release occurred.

Retained verification scripts now invoke Cargo from the repository root to respect its pin. Experiment binaries also respect the task-owned target directory. Script syntax and whitespace checks passed; the compiler pin remains1.98.1 until qualification justifies adoption.


## M1 implementation and dependency qualification, 11 October 2026

The new final-use `brn-threads-core` and transient `brn-threads-intake` are ready for a source milestone. The core has **38 passing locked tests** on Rust1.98.1 and passing all-target Clippy with `-D warnings`. Tests cover durable identity before prepare, immutable requests, atomic grouped writes, receipt replay, exact target/action grants, paused delegation, generations/guards/Save, actual-write-set Undo, backup/restore and incompatible-data refusal. A scoped independent review found four concrete authority/replay defects and then one paused-grant fallback defect; all were reproduced and fixed. Later writer tests cover atomic configuration fences, stale selected-model starts, one Working Run per Thread, candidate replacement/attention references, and UTF-8 comment mappings. Intersecting edits remain unresolved. Closed buffers retain generation tombstones with cleared text.

The converter has **10 passing acceptance tests** (plus two child entry points invoked by their parent tests) on the Mac. Independent source inventories cover an ordinary text-layer PDF (19 items) and process DOCX (22 items), including one retained 720x200 PNG each. Missing a necessary figure yields Partial; full imports without an independent inventory remain Partial. Markdown/text preserve UTF-8 verbatim, EML uses the maintained helper, and source originals/attachments are removed from returned knowledge and temporary directories. Cancellation kills owned descendants; the Mac bounded process tests refuse external file/network access. General complex PDF hierarchy, vector composition, scans and equation fidelity are not claimed.

Compiler qualification used the original dependency lock SHA `16fe749fc0e4592f2bf2065c660203ded43e5c1c7b03c803d52abceaa9a8ffb7`: Rust1.99 formatter, workspace Clippy, Mac native/combined builds, retrieval and native state tests passed. The broad old workflow run had 454 pass/1 backup deadline failure/16 ignored during concurrent large builds; the exact maximum-asset backup case passed alone on retry. Native interaction is pending because the Mac is locked. The root pin therefore remains1.98.1 under the plan's adoption gate; retained scripts respect that pin.

Published Rig0.44 passed the real stderr regression but failed actual malformed-SSE fixtures after adaptation: later text was appended and a failed stream became Completed. Its isolated trial had138 pass/15 failures/1 ignored, including fixture adaptations and that concrete parser regression. Production retains Rig0.43 and its logging patch. The new runtime adds whole-batch typed admission before dispatch rather than relying on the model to correct invalid calls. GPUI Kit0.7.1/GPUI0.3.8 passed the bounded source/build trial after removing obsolete manual dialog rendering; native interaction remains pending separately.

Owner selection is now explicit: **Codex, GPT-6.1 Sol**, with medium reasoning as the stated default. The normal selected Codex subscription account passed an ephemeral synthetic availability check. A later new-runtime smoke returned `BRN_THREADS_READY` with Completed; its source/runtime milestone and broader journey evidence follow separately. No credentials were printed, no old BRN data inspected, and no external message, merge or release occurred.


## First-release candidate implementation, 11 October 2026

M1 source commit `ca0342bc0437af293b26cf0b891bc14c85227875` was tested from an independent archive:38 core tests passed on the repository1.98.1 pin. Formatting follow-up `1db8d07bb5edd4b90c44e5b1ff7a15fe400742c0` changes only intake formatting. The final candidate source identity is recorded below after publication; the commands in this section ran against the staged candidate source, not against the old setup-only tree. No other checkout, dirty work, user database or branch was reset.

### Implemented behavior

The final application service serves the new CLI, desktop and runtime. Notes/search use committed current revisions. Durable Run controls/settings fences prevent superseded work from applying. Continue begins a fresh invocation from intentional messages, semantic progress, source observations, current records and receipt outcomes. Normal Codex subscription credentials are read in memory from the explicitly selected normal auth file; no credential copy, printing, refresh or fallback. Fixed packaged base guide plus four allowlisted skills retain bundle/loading identity without granting authority. Whole-batch typed admission rejects invalid/oversized/unknown tools before any tool dispatch or extra model request. Direct instructions authorize only the exact target/action; canonical explicit decision creation authorizes one new decision, without widening authority.

Full import retains meaningful Markdown/figures atomically in protected notes, uses original source SHA and stable source/version links, and avoids duplicate full notes on retry. Export checks exact managed asset identities and publishes a verified complete snapshot exclusively. Review replacement remaps structural references and asset URLs by exact identity. Accept/dismiss removes only matching attention; stale dismissal frees the pending slot and preserves the current note and unrelated questions across restart/retry. Save persists comment mappings only where the supported edit mapping is known, otherwise unresolved. Native views implement Home/Needs you, conversations/linked work, notes/sections/Save/recovery, review, comments, current search, Actions, history/Undo, export/backup and settings. A saved Working run with no local worker offers honest fresh Continue; opening does not cancel another process.

The old `brn-store`, `brn-workflow`, their CLI command families and old desktop workers/views/tests are removed coherently. Provider route fixtures remain test/capability-only, with no old production runtime. Maintained intake/provider/retrieval capabilities remain. Old standalone experiments are historical; production has no old-engine dependency. Required CI contexts/events/permissions remain; tests assert the replacement behavior, not a no-op retirement gate.

### Local candidate verification

Target: Apple Silicon Mac; repository Rust1.98.1; all Cargo commands `--locked`; synthetic data and targets outside Git under `/Users/evokessler/repos/BRN-local-builds/threads-20261010`. Source credentials and private mailbox data are excluded.

| Check | Actual result / evidence file under task `evidence/` |
|---|---|
| `cargo build --workspace` | Passed; `threads-candidate-workspace-build.log`. Maintained helper built with `brn-intake/helper` beforehand. |
| `cargo test --workspace` |301 passed,0 failed,3 intentional ignores; `threads-candidate-workspace-tests.log`. Includes core38, application integration12 plus one5,000-note test, CLI3, Mac intake10 and provider/runtime cases. Two ignored intake child entry points execute through their parent tests. |
| `cargo clippy --workspace --all-targets --features brn-ai/capability-spike -- -D warnings` | Passed; `threads-candidate-clippy.log`. |
| `cargo test -p brn-ai --features capability-spike --all-targets` |168 passed,0 failed,1 intentional process child ignore; `threads-candidate-provider-tests.log`. Retained stderr/transport cases plus9 new runtime cases and2 normal-auth safety cases. |
| `cargo build -p brn-desktop --features native-ui,native-retrieval` | Passed; `threads-candidate-native-build.log`. |
| Combined native/test-support all-target Clippy with `-D warnings` | Passed; `threads-candidate-native-clippy.log`. Upstream `block0.1.6` future-incompat notice remains recorded, not suppressed. |
| Combined native/test-support desktop bin tests |14 passed; `threads-candidate-native-tests.log`. Initial new fixture used a mismatched model; fixed the fixture to current Settings without weakening core checks, then all14 passed. |
| `cargo test -p brn-retrieval --features native --lib --test model_download` |15 passed, no model assets/private downloads; `threads-candidate-native-retrieval-tests.log`. |
| `cargo test -p brn-threads-app --features native-retrieval` |13 passed; `threads-candidate-native-app-tests.log`. Current search refresh/Save/archive and5,000-note witness included. |
| `verify-end-to-end.sh --fixtures-only` against final binaries | Passed Save/replay/recovery/search/export/backup/startup/foreign-data refusal; `threads-candidate-end-to-end.log`. |
| Development tooling / local launcher |18 Python tests passed; launcher argument quoting/refusal tests passed; `threads-candidate-tooling-tests.log`, `threads-candidate-launcher-tests.log`. |
| Formatter / whitespace / current Markdown links | Formatter and whitespace passed; final link result recorded below after staging. |

Counts across suites overlap and must not be summed as unique coverage. The5,000-note current-search witness passed in about5 seconds on this machine; this is headless evidence, not native responsiveness.

### Scoped independent integrity review

The review scope stayed bounded; the completed whole-plan review was not repeated. Core findings corrected asset protection/guard bypass, cancelled-run resurrection, Undo replay identity, confirmation clearing and paused fallback. Service/replay findings corrected selected-account capture races, asset-URL prefix matching, changed configure payload replay, candidate replacement after lost response and ChatGPT/Codex alias start validation. Runtime findings corrected loaded-skill fencing, contextual quoted-title grant expansion, arbitrary prose identity remapping, malformed/oversized/missing-field tool admission, auth/cancellation terminal handling and Copilot Responses routing. Independent rechecks reproduced the supported issues and passed their correction witnesses; no unresolved supported issue remained in those rechecked scopes. Recheck logs and reviewed hashes remain under task `review/` (`service-fixed-witnesses.log`, `account-selection-fixed.log`, `runtime-fixed-witnesses.log`, `runtime-recheck-tests.log`). This does not claim review of every possible behavior. The final stale-dismiss/crash-Continue corrections have focused product/native regressions.

### Selected live runtime journey

Owner reply: “use codex , sol 6.1”. All live work used that normal configured subscription route with `gpt-6.1-sol`, medium effort, bounded turns and public synthetic Harbor data. No external message was sent. A fresh runtime smoke returned `BRN_THREADS_READY` with persisted Completed. An earlier smoke exposed a terminal fence omission; that was fixed and the fresh smoke passed (`threads-runtime-live-smoke.json`).

Useful synthetic EML intake Completed, loaded the intake guide and retained Ana's reported Tuesday10:00 handoff as a report, not owner agreement; unspecified date/timezone stayed explicit. It updated the ordinary context, linked provenance and created a Suggested internal Action. A new invocation in the same Thread, without source body, Completed and saved an internal reply draft asking about the checklist; the Action remained Suggested, nothing sent. A protected Monday decision was then full-imported. Conflict investigation Completed with exactly one Conflict question in that Thread; protected note version1, protection and Markdown stayed unchanged. This checks semantic outcomes as well as receipts (`threads-live-useful-result.json`, `threads-live-continue-result.json`, `threads-live-conflict-result.json`, corresponding record snapshots). The deliberately excluded raw canary was absent from logical retained records/progress after continuation. External source fixtures deliberately remain external; no storage-page forensic erasure claim.

### Dependency hygiene and retained limits

Final retained-lock `cargo-audit0.22.2`:0 vulnerabilities,6 informational unmaintained notices; advisory database1296 entries, commit `7eebec69c352c7191b1f13eb95dd510eeca5d1de`, last updated2026-10-09T10:12:02+02:00 (`threads-candidate-audit.json`). Target/feature trees traced `instant` through Kit base/component; `paste` through component/Metal/image/tokenizers; `rustybuzz` and `ttf-parser` through GPUI SVG/font paths. `rustls-pemfile` belongs to cross-target Kit assets/GPUI reqwest and is absent from the Mac graph. `smallstr` is locked for optional `yrs`, absent from the selected workspace feature graph even with all targets; no CRDT capability is enabled. These maintenance notices do not justify a renderer fork or blanket update. SQLite0.40.2, BetterOffice0.3, image0.25.10, ZIP8, narrow HTML/EML tokenizer/parser and FastEmbed7.1.0 retain the focused plan rationale and bounded malformed/quota tests. Checkoutv7.0.1 is pinned to verified full SHA `3d3c42e5aac5ba805825da76410c181273ba90b1`.

### Mac candidate and remaining acceptance

Unsigned `BRN Threads Candidate.app` assembled from the final combined native binary plus maintained helper, using only fresh `data/native-candidate-schema3`. It includes a36,852-byte synthetic protected long-note/managed-figure fixture. Actual final native binary and copied app-bundle binary headless startup passed (`threads-candidate-packaged-startup.log`, `threads-candidate-bundle-startup.log`, `threads-candidate-seed.json`). No UI launch/unlock was attempted.

The single pending [native acceptance task](acceptance.md) covers long-note rendering/IME/selection/table/figure/focus, comments before/inside/deletion plus reopen, protected review, Save/recovery/Undo, search/Actions, cancellation/Continue and export/backup. Native interaction and owner acceptance remain unverified because the Mac reported locked. Immediate Quit only requests token cancellation; abrupt process exit cannot guarantee a terminal Interrupted receipt, so saved Working state is shown honestly and can Continue. General complex PDF hierarchy/vector composition/scans/math fidelity, external sending, Office/HTML generation and semantic model installation are not claimed. Required hosted results and exact candidate source commit are recorded below; no main merge or release.


### Published candidate source identity

Candidate implementation commit: `1f7e7fd80111cb29d2fb562fb2c548b685721093`. All210 retained source/guide/gate files were matched byte-for-byte to the tested candidate manifest after commit; digest `6222fead400e7af0f86dbcad655ee62bb96b8568f937641a5a901d4555cd2b69` (`threads-candidate-source.json`). The following handoff documentation commit does not alter product source. The packaged native binary/helper and bundle hashes are retained in `threads-candidate-bundle.json`. Final Markdown result:93 files,344 local links,0 failures; formatter and whitespace passed.

The isolated branch was reconciled with remote before publication (no divergence, no force push). PR #114 stays draft. Hosted checks start on the published handoff head and remain separately pending until observed; local checks above do not stand in for them.


### Hosted candidate check observation

[Hosted run38089632653](https://github.com/ewq100/brn-rust/actions/runs/38089632653) tests handoff head `df0cd00ef02389baf06fa010b2b15ceeeb9d3297`, whose product source is exactly candidate `1f7e7fd80111cb29d2fb562fb2c548b685721093`. Documentation/tooling and required Linux core/CLI passed. The three required Mac contexts are still running at this observation; their final status is recorded in the handoff below. The informational Windows build failed in retained/new Unix-specific credential filesystem handling (`auth.rs`, `codex_auth.rs`,26 compiler errors). This lane remains present and visibly failed; no job or protection was suppressed. Windows support is not claimed for the Mac candidate. Logs/status are under task `evidence/hosted-candidate-*`.


### Final hosted result and handoff

Run38089632653 completed at handoff head `df0cd00ef02389baf06fa010b2b15ceeeb9d3297`: all four required contexts passed — Core/CLI Ubuntu, Core/CLI Mac, Native UI Mac and Native retrieval/combined Mac — plus Documentation/tooling. Overall run conclusion is failure because the informational Windows build failed as described above; it is not reported as an all-platform green run. Product source remains `1f7e7fd80111cb29d2fb562fb2c548b685721093`. Final follow-up updates only handoff documentation and causes a new normal CI run; that run's status is not substituted for the observed result above.

The isolated checkout is published and the draft PR rewritten around the actual candidate. The source and delivered bundle hashes, live outcomes, scoped review dispositions and acceptance limits are retained. Final next action: perform [the single native interaction task](acceptance.md) when the Mac session becomes available, record observed behavior/owner acceptance and fix concrete failures. No independent product implementation or headless qualification remains open at this handoff. Native usability and owner acceptance are pending; no main merge or release.

## MacBook source repairs and focused verification, 11 October 2026

This follow-up began at fetched head `36dfef5edd317f560cbf26ad7f84c1f5464f31c0`, whose product source matched candidate `1f7e7fd80111cb29d2fb562fb2c548b685721093`. The checks below ran against task-owned working-tree repairs in the isolated MacBook checkout, not the unchanged published head. The machine reports **macOS 26.5 (25F71), arm64**. Rust remains **1.98.1**, Rig remains the qualified patched **0.43.0**, and `Cargo.lock` is unchanged. Builds use the explicit task-owned `threads-macbook-local/target`; fresh synthetic databases and generated apps remain outside Git. The final committed source and local bundle identity belong to the lead's completed native handoff.

### Useful intake and coverage findings 1 and 6

Native and CLI useful intake now pass the converted fingerprint, typed coverage, and transient figures to the shared runtime. `read_source` exposes opaque figure links and coverage categories; checked Note preparation retains only selected figure bytes as managed assets with source provenance. Unselected figures and complete raw source bodies are excluded from retained records. Source and Note provenance distinguish detected omissions, converter limitations, and coverage not independently verified, including backup/restore. Multiple selected sources aggregate their qualifications, so a later checked source cannot hide an earlier omission. Existing full-import notes retain their original coverage snapshot; model proposals cannot invent a full-import classification or replace host reports, omissions, or fingerprints. Label improvements remain delegated.

Independent source review found a Prepare-to-Apply race: historical derivation inputs alone did not reject a source changed after preparation. Used sources now contribute unchanged Source writes with exact expected versions to the immutable candidate; the shared SQLite writer rejects stale evidence atomically, including owner review. An early runtime check returns Stale, while an already committed receipt remains replayable. Preparation retries reuse the original source snapshot. Action-only review selection preserves these shared Source guards when a sibling Note is excluded. Forged typed reports, source identities, and duplicate locator aliases are refused, including the supported fingerprint-without-report case.

### Mac conversion compatibility

The native hardcoded `/opt/homebrew/bin` assumption did not match this MacBook's Homebrew prefix. Native and CLI conversion now share explicit absolute `BRN_PDFTOTEXT`/`BRN_PDFIMAGES` overrides, executable PATH discovery, and Finder-compatible fallbacks under `/opt/homebrew/bin`, `/usr/local/bin`, and `$HOME/.local/bin`. Invalid explicit overrides do not silently fall back. Converter isolation derives the necessary Cellar runtime prefix from the resolved executable, preserving external-content and network denial instead of broadening access to arbitrary build PATH directories.

Actual initial native DOCX display exposed literal BetterOffice `docx-export`/`docx-story` comments, including table cells. DOCX conversion now removes only the exact converter-generated syntax from retained readable Markdown, preserving provenance and coverage. Authored or malicious HTML/comments remain literal; Markdown import is unchanged. The real Harbor DOCX regression passed with all 22 independently inventoried objects, its retained figure, readable table syntax, and temporary-input cleanup.

### Automated evidence and remaining limits

- `cargo test --locked -p brn-ai threads --lib`: **20 passed**, including nine source regressions. Witnesses cover the actual mocked agent `read_source → prepare_changes → apply_changes` path, reopened exact selected PNG bytes, omitted raw canary/unselected JPEG, immutable retry/replay, forged coverage refusal, multiple-source omissions, fingerprint-only authority, and a separate-connection source race. Direct core Owner apply also refuses the stale candidate; Action source checks exclude unused sources from the Apply guard.
- `cargo test --locked -p brn-threads-app --test import_coverage`: **1 passed**; full import preserves distinct omission/unverified/limitation categories through backup and restore. The separate Action-only subset/source-advance regression passed in the application checks.
- `cargo clippy --locked -p brn-ai -p brn-threads-app -p brn --all-targets -- -D warnings`: **passed** after the final source guards. Formatting and `git diff --check` passed.
- Focused intake library checks previously passed **11 tests**, with two intentional child entry points exercised by parent tests. Discovery, exact marker handling, and the custom Cellar isolation witness passed; the Harbor DOCX conversion regression passed using the locally built helper.

These are source and automated results. Corrected native PDF/DOCX imports, figure rendering, and the useful-intake live interaction remain pending at this entry; Poppler installation/qualification and the combined shipping rebuild are owned by the lead's ongoing MacBook acceptance task. Mocked agent tests do not establish live semantic selection or native usability. General unsupported PDF layouts, scans/OCR, vector composition and mathematical fidelity remain outside the qualified claim. Owner acceptance remains separate and incomplete; no merge or release is implied.


### Native worker ownership, comparison and bounded Rig evidence

A local native Run now retains its originating Thread identity. Provisional text is rendered only in that Thread; viewing or resolving a different Thread neither relabels the text nor cancels its worker. Resolving the originating Thread cancels its local worker, and global progress/completion names the originating work. The real GPUI regression delivers a synthetic worker event in A, switches to/resolves B, returns to A, and resolves A; it checks both text visibility and cancellation ownership without a provider call.

Refused Save now reloads the latest canonical saved text for comparison while preserving the owner's editor and durable recovery buffer. The regression writes a concurrent update through another Workspace, then checks the refused Save's current comparison, retained typed buffer and persisted current revision. Hidden note-editor focus is blurred at navigation/read/review boundaries; keyboard regressions cover Save→typing, Read→typing and Home→Tab→typing. Passage certainty and discussion lifecycle are distinct: a known saved selection starts located, and Thread context no longer calls an open discussion “Comment resolved.”

Whole Note/Action selection prepares a replacement identity before Apply. Real component clicks independently retain a Note or an Action, check that the changed selection cannot apply the old candidate, then apply the exact newly displayed subset while the omitted record stays unchanged. History callbacks open the named current Note through `open_note`, create one receipt-scoped refresh follow-up on repeated clicks, and retain derived writing after updates and Undo. These focused component tests were included in the earlier **19 passing combined native desktop tests** and strict desktop/CLI Clippy; final shipping source/tests are recorded by the lead after subsequent repairs. They do not establish OS IME behavior, rendered pixel quality or owner acceptance.

The separate [Rig decoder comparison](rig-decoder-comparison.md) reproduces published 0.43/0.44 behavior through a six-case, one-request-per-case synthetic Responses harness with its own lockfile and outside-Git target. Both valid controls complete and both literal invalid-JSON cases fail. For recognized text-delta JSON with numeric `delta`, 0.43 errors while 0.44 accepts later text and reports Stop. This is malformed stream content, not malformed tool-call retry admission. The exact command and sanitized output are recorded in that document; the locked harness exited 0. Production retains qualified patched Rig 0.43.0, Rust 1.98.1 and the unchanged workspace lock.


### Independent bounded native source review

A separate reader checked the completed MacBook `threads_ui.rs`/`threads_state.rs` changes and their component regressions against the current Thread/review/recovery boundaries: worker A's stream and cancellation ownership while viewing/resolving B; hidden editor focus; stale Save's latest comparison and retained buffer; complete Note/Action selection followed by a new immutable displayed candidate; named refresh navigation and repeat-safe follow-up; and the search-clear repair. No additional supported correctness defect was found in this scope. Review selection still delegates dependency/shared-effect checks to the application service; the UI does not manufacture authority or apply a mutable preview. Clearing or whitespace-only search now skips retrieval, clears obsolete results/errors and renders the normal workspace list; its real keyboard regression checks reopening a note after Cmd-A/Backspace.

This is a bounded source review using the installed Rust review checklist, distinct from the lead's actual MacBook screen/keyboard observations. The completed whole-plan architecture review was not repeated. Final compilation, lint and interaction results, committed source identity, and remaining blocked acceptance items belong to the lead's final handoff. Owner acceptance is not claimed by this review.


### Successful Continue removes only obsolete terminal attention

After source commit `50931f9`, actual native Stop→Continue completed with the synthetic tail answer, but reopening Home still showed “Run interrupted; Continue starts fresh.” The focused runtime regression reproduced that stale persisted notice before the repair (`threads-macbook-local/evidence/continue-attention-before.log`). A successful fresh invocation now removes only the exact host terminal Blocker for a prior Interrupted/Failed Run in the same Thread, within the checked terminal commit. Starting Continue alone keeps the notice until success. Independent Question/Review/Conflict entries, separate Blockers even when tied to that prior Run, and identical wording tied to a non-Run record remain. A failed/interrupted fresh invocation still creates its own terminal notice; historical Run records/statuses are retained.

`cargo test --locked -p brn-ai threads --lib` passed **21 tests** after the repair. The added test covers both Interrupted and Failed predecessors, successful fresh completion, database reopen and exact preserved unrelated attention (`continue-attention-fixed-threads-tests.log`). `cargo clippy --locked -p brn-ai --all-targets --features capability-spike -- -D warnings` passed (`continue-attention-clippy.log`). These are synthetic runtime/service checks without account/provider requests. The shipping rebuild and actual native retest of the previously affected Thread are separately owned by the lead; owner acceptance remains incomplete.

## Intermediate MacBook candidate and interaction, 11 October 2026

The isolated MacBook checkout fetched `36dfef5edd317f560cbf26ad7f84c1f5464f31c0`; no intervening remote product changes were present. Other repositories/worktrees and unrelated dirty/untracked work were preserved. Repairs are product commit `50931f9af53f906b9c4453d25e7e4dabd9911323` and the subsequent native-discovered Continue repair `39b21c06bcac2af2144bdd9d8e07d8a3b5a51a1b`. The latter was this intermediate candidate source. The completed handoff below supersedes its pending cells and shipping identity. This task did not repeat the general architecture review.

Machine: **macOS 26.5 (25F71), arm64**. Native development preflight passed for the repository **Rust 1.98.1** pin, rustfmt/Clippy, protoc and Apple clang. Production lock SHA-256 remains `b4d211203fd77b490d2bd9ce5f19dd04ab638b997e652fb1ff7700277d198c24`. Qualified patched Rig **0.43.0**, Kit 0.7.1 and GPUI 0.3.8 remain. Rust 1.99 adoption and Rig 0.44 adoption have not been inferred from these repairs.

All local paths below share task base `/Users/evokessler/.codex/.chatgpt-projects/g-p-6ab8ee911378819187a66bf0ee50da76/threads-macbook-local`, outside the isolated Git checkout:

- Candidate: `BRN Threads Owner Trial.app`, locally assembled by `scripts/make-macos-app.sh` from the same task target's desktop/helper, with explicit absolute data selection.
- Fresh synthetic data: `data/native-acceptance`. No existing BRN user database was opened or migrated.
- Shared target: `target`; locked fetch, helper build, CLI build and combined native desktop build used this target.
- Shipping app manifest SHA-256: `c58158a3c4cde73b23f50d5f757b96dac21ee0a5d689e57e77daeefdbfcd8438`. `evidence/owner-trial-bundle.json` lists each copied executable/launcher/plist hash, exact product source, data path and compiler. Copied shipping desktop headless startup passed (`Threads ready: 57 records` before the subsequent live intake).
- Installed PDF tools and exact hashes: `evidence/poppler-local-build.md`. Finder-like PATH qualification passed with overrides unset; supported discovery reaches `$HOME/.local/bin` and retains subprocess restrictions.

The synthetic long note, repeated phrases, Estonian content, wide table and managed gate-order PNG were created through supported CLI/import/Workspace paths, not SQL mutation. Additional synthetic protected/group/refresh/orphan fixtures use checked host preparation/Apply. A separate **174,100-byte** saved note has its final heading at byte **174,038**, beyond the former 128 KiB prefix. Fixture helper source, snapshots, generated apps/databases and export output remain outside Git.

### Saved-note reading, direct refresh and independent groups

Finding 3: `read_records` now advertises actual saved length, continuation and bounded headings. `read_note` reads at most 128 KiB at an exact current version/UTF-8 byte offset or section, advances without splitting a character and refuses stale/invalid ranges. Section scanning excludes fenced-code headings. Reopened long-note regressions read the final fact and stale-version witnesses refuse before returning mixed revisions. The native live request after app restart returned **47** for the specifically named saved tail note; regression tests provide the deterministic range/section witness. This is not a claim that a truncated prefix is complete.

Finding 4: receipt input/effect identities identify directly derived records affected by meaningful newer content. Save, updates and actual Undo expose named records and an idempotent ordinary follow-up Thread; they do not automatically rewrite those records. Re-derivation provenance survives exact Undo and differentiates identical text produced from distinct inputs. Routine status/protection/confirmation metadata and Source observation/label changes do not create false meaningful refresh. Actual native input Save showed two named derived records in History; clicking the summary opened that current note, and the follow-up linked the current input and both records without starting a worker. Native Undo again exposed the two records while preserving their writing (`native-refresh-observed.json`, `native-ime-refresh-undo.json`).

Finding 5: independent whole Note/Action groups can be included/omitted, but require preparing a refreshed immutable candidate. The old candidate cannot Apply after changing selection. Shared source guards and required links remain; dependency/stale-base refusals stay atomic. Real component clicks tested both Note-only and Action-only selection. Actual native Note-only selection prepared a new candidate, applied only the displayed note and left the omitted Action absent (`native-selected-group-prepared.json`, `native-selected-group-applied.json`). Action-only Source advance refusal is additionally covered by the shared-service regression.

### What was actually observed on screen

Observed interactions ran through progressively rebuilt local apps, ending with `BRN Threads MacBook Final.app` built from `50931f9`. Evidence below remains valid where the subsequent `39b21c0` change touches only terminal attention. The last bundle is newly packaged from that attention fix; its actual window launch and affected Continue notice retest are still pending. Native screen/keyboard/mouse observations are distinct from component tests and from owner acceptance.

1. Home/Needs you, Notes, Actions, History and Settings rendered. The long note displayed headings, a wrapped wide table and the managed CHECK→TEST→RELEASE PNG. Scroll, section navigation, repeated selection/copy, typing and editor Undo were exercised. Estonian text persisted. On the owner's Estonian input with English language settings, native dead-key input displayed marked composition, then committed `à`; Save persisted exactly that character. This establishes observed composition, not a Pinyin test. Keyboard settings were restored after an earlier unsuccessful Pinyin shortcut attempt. Search clear initially left an error; after repair, actual final-app Cmd-A/Backspace returned to the normal list without that error.
2. Reading did not establish a dirty guard. While the actual native buffer was dirty, a supported grouped proposal deferred without partial effects. Save/reopen and abrupt-close recovery preserved exact synthetic text. Stale Save refused and kept the typed buffer; Save as new note preserved it. The incorrect cached current comparison was repaired and passed a real component cross-Workspace regression; affected native comparison retest remains pending.
3. A comment attached to the second `tere` occurrence. Prefix edit/Save mapped that occurrence; intersecting replacement made it unresolved while retaining the original quote. Thread context did not guess another occurrence. Resolving its discussion left passage certainty independent (`native-comment-mapped.json`, `native-comment-save.json`, `native-A-working-B-resolved.json`).
4. Protected Compare/edit/Prepare fresh/Apply and immediate Undo were performed. Concurrent current-note change caused stale refusal; Dismiss preserved current text and unrelated Question, then a fresh candidate was prepared. Actual later-edit Undo conflict remains pending. Independent whole-group and refresh/Undo interactions are recorded above.
5. Native DOCX import initially revealed literal converter markers, leading to the narrow conversion fix. Real PDF/DOCX inventory qualification and Finder-compatible extraction passed after repair. All four final native useful/full PDF/DOCX dialogs and resulting rendered imports remain pending; automated conversion and shared-runtime live evidence are not substituted for those interactions.
6. Native normal-account availability completed using Codex / gpt-6.1-sol / medium. A reopened saved Working Run with no local worker honestly offered fresh Continue. While A was Working, B was viewed/resolved: B received no A provisional text and A completed normally. Stop produced Interrupted; fresh Continue in the same Thread completed and returned the tail fact. Home then exposed an obsolete interruption notice; the new cleanup was reproduced/fixed with restart and unrelated-attention preservation tests, and independently reviewed. Native affected cleanup and running-work pause/settings-fence checks remain pending.
7. Resolving a synthetic discussion preserved its linked Waiting human Action. Explicit Confirm done completed that Action; this is synthetic workflow verification, not owner acceptance of BRN. The internal reply draft remained unsent (`native-resolved-waiting-action.json`).
8. Native export published eight Markdown notes and one PNG; all nine manifest hashes matched, including the managed figure (`native-corroboration.json`). Final backup/restore and occupied refusal results are recorded below when available; native backup button observation remains pending.

macOS capture briefly failed while BRN was minimized; the owner restored the window and interaction resumed. This was not recorded as a BRN rendering defect. Later, macOS explicitly reported that the MacBook was locked and automatic unlock failed. An unlock request was sent; no unlock/credential workaround was attempted. [Acceptance](acceptance.md) lists the exact remaining native cells. **Owner acceptance remains pending.**

### Repaired source checks and final selected live intake

The corrected installed-tool broad gate passed **331 workspace tests, 0 failed, 3 intentional ignores** and **178 provider capability tests, 0 failed, 1 intentional ignore**. Two ignored intake child entry points run through parent tests; the provider process child is intentional. Combined native component checks passed **20 tests**. Locked helper/workspace/native/CLI builds, strict workspace/provider/combined-native Clippy, formatter, offline Save/recovery/search/export/backup/startup/foreign-data journey, 18 tooling tests and launcher checks passed. Counts overlap. Earlier sandbox loopback EPERM attempts and incorrectly chosen source-build PDF paths are preserved separately; the corrected complete gate passed, rather than treating those environment failures as product fixes (`final-broad-checks.json`, `final-storage-gate-installed-tools.log`).

After the native-discovered obsolete notice fix, **21 focused runtime tests** and strict capability-feature Clippy passed; locked shipping native and CLI rebuilds passed (`continue-attention-fixed-threads-tests.log`, `continue-attention-clippy.log`, `attention-shipping-native-build.log`, `attention-shipping-cli-build.log`). Independent read-only review confirmed narrow notice removal and atomic Run/Thread/settings fencing. Unaffected broad results were retained rather than rerun without cause.

Final product `39b21c0` also ran bounded CLI useful PDF intake through the actual selected account and agent-to-note path while native interaction was locked. Budget 8; Completed Run `5dff3d3f-8f5c-4233-9d7f-b036750a2527:0`, actual receipt `60272f07-b675-4472-b77e-3507162cf169`. The selected managed PNG is 720×200, 3,257 bytes, SHA-256 `75817c9dd0fb816e03cc61a9edf1a20a85e1a12fe6a4922c57685a4a939a8273`, exactly matching direct local Poppler extraction. The retained note/source carry distinct typed Limitation and Unverified issues, with no invented specific omission. Three protected notes retained exact versions/text. The current context and Suggested Action were semantically refreshed from saved newer input; human completion and internal unsent draft were preserved. This proves live selected asset persistence and coverage along the actual runtime path; it does not count as a native import-button check (`final-live-cli-useful-pdf*.json`). No credential contents or private mailbox were inspected, no account/provider switched, and no external message was sent.


## Completed MacBook verification and owner handoff

This entry supersedes the earlier pending interaction cells, not their historical observations. Remote `rebuild/threads` was fetched again before publication and still pointed to `36dfef5edd317f560cbf26ad7f84c1f5464f31c0`; no intervening concurrent changes required reconciliation. All six supplied source findings applied at that head. Other checkouts/worktrees and dirty work were preserved. No force push, main merge or release is authorized by this handoff.

### Exact tested local candidate

Shipping product source: **`c3a79cd859c222c3da7997697629c7fd6f1c2141`**. Repairs are `50931f9` (six findings and native defects), `39b21c0` (successful Continue cleanup), `08bc9bb` (sequential coverage retention), `00412db` (coverage viewport), and `c3a79cd` (plain bold section labels). Evidence-only commits do not alter that source.

Machine: **macOS 26.5 (25F71), arm64**. Preflight, locked fetch and required helper/CLI/combined-native builds used the repository **Rust 1.98.1** pin and one task-owned target. Qualified patched **Rig 0.43.0**, Kit 0.7.1, GPUI 0.3.8 and production lock SHA-256 `b4d211203fd77b490d2bd9ce5f19dd04ab638b997e652fb1ff7700277d198c24` remain intact. The separate Rig decoder comparison has adaptation, exact command, independent lock and sanitized six-case output; no production 0.44 migration or retry-policy substitution occurred.

Absolute local base: `/Users/evokessler/.codex/.chatgpt-projects/g-p-6ab8ee911378819187a66bf0ee50da76/threads-macbook-local`.

- Open `BRN Threads MacBook Acceptance.app`. It was locally packaged with `scripts/make-macos-app.sh`, copying this MacBook's combined desktop/helper and selecting the explicit absolute fresh `data/native-acceptance` directory.
- Manifest `evidence/acceptance-bundle.json` SHA-256: `5de1656e8fd005d453d728d97bda4d2fdf23e01d4a999dfec7f90037ec2fead6`.
- Desktop SHA-256: `c06291312803f64d630c56e2d00e4b10bc214491ef065ee54b4f475c3f67177d`; helper: `3ff91273cecf74bebaf534e1e73f5b68e2e5fc36f50ee78ca32c6ac081f4771c`; launcher: `20561f34c09db6e67bea0711d9d2b563943f5fb897fc72f904ba77c6595f90f0`.
- Copied desktop startup and actual native launch passed. Shipping build/test/lint logs: `acceptance-native-build.log`, `acceptance-cli-build.log`, `acceptance-bundle-startup.log`, `acceptance-native-tests.log`, `acceptance-native-clippy.log`.
- Local Poppler 26.10.0 under `$HOME/.local/bin` works with Finder-like PATH and no override. Official source/build hashes and qualification are in `evidence/poppler-local-build.md`. No other Mac's executable or absolute path was reused.

All apps, fixtures, databases, backup/export output and raw local logs remain outside Git. Data was created through supported application/import/CLI/checked Workspace paths. Existing BRN user data was neither opened nor migrated.

### Additional defects, fixes and witnesses

Sequential live CLI intake on `39b21c0` revealed that a later source could retain earlier material while dropping its typed coverage from the changed Note. `08bc9bb` carries host qualifications from the exact committed base when substantive paragraphs or managed figures survive. Prior issues are source-scoped; an unchanged complete checked observation of that same source can clear only its qualification. Unrelated unchecked coverage stays explicit, full replacement can discard obsolete qualifications, and full imported references remain immutable. A red runtime witness reproduced the loss. Final focused runtime checks passed **24 tests**; strict AI/application/CLI all-target Clippy passed (`sequential-coverage-final-tests.log`, `sequential-coverage-final-clippy.log`). Independent bounded source review found no remaining defect.

Actual native sequential intake then exposed six long coverage issues consuming the entire note viewport. `00412db` retains every typed issue/legacy gap in an independently scrolling panel bounded to 120 logical pixels, with intent/report summary, section navigation and body visible. Real GPUI regression checks typed and legacy data, last-issue reachability, usable body and exact unchanged stored record. Final shipping **21 combined native tests** and strict all-target native Clippy passed. Independent read-only review confirmed presentation-only scope. Actual screen/wheel retest reached the final issue without moving the body; section clicks and body scrolling reached both managed figures, process table and references. Bold DOCX section titles initially displayed literal `**` in navigation; `c3a79cd` removes only their enclosing bold markers from labels, preserving source text and target ranges. Final native section navigation and plain labels passed.

### Completed affected native retests

On `39b21c0`, stale Save displayed current v4 concurrent text and retained the typed buffer. Fresh protected candidate `51c166c9-f7ab-407d-aad2-0aa256735f0a` applied v6; a later owner Save v7 retained `LATER_OWNER_EDIT_MUST_SURVIVE_UNDO` when native History Undo refused. Successful orphan Continue removed the obsolete interruption notice while retaining unrelated attention/historical Interrupted Run. Pausing an actual active native worker persisted Cancelled Run `881a02ba-b10f-4d82-9634-92b79b606e33:0`, with no operation effects; maintenance was restored enabled. An earlier fast-completing attempt was not counted as the active pause witness (`native-final-retests.json`, `native-pause-records.json`).

Earlier source `50931f9` native observations remain relevant for unchanged behavior: marked dead-key composition/Save on Estonian input with English language settings; selection/copy/Estonian typing/scroll/wide table/figure/editor Undo/focus; dirty grouped-change deferral; abrupt-close recovery; second-occurrence comment mapping then unresolved intersecting edit; protected Compare/edit/new Prepare/Apply/immediate Undo/stale Dismiss; independent Note-only group; named refresh/follow-up after Save and Undo; worker A while viewing/resolving B; Stop/Continue tail answer after restart; human Action completion and internal unsent reply draft. The final three product commits only affect coverage presentation/retention and labels; affected behavior was repeated as described here. [Acceptance](acceptance.md#actual-macbook-observations) records all eight observed outcomes.

### Actual native full and useful imports

All four intake choices used actual native buttons and macOS file choosers with fresh local copies of the Harbor fixtures. Full imports on `39b21c0` created protected PDF Note `90d2b7a8-7237-47f6-8930-c0054139eea7:1` and DOCX Note `87268852-1619-483b-b29a-c496eee6932e:1`. Screen comparison covered study prose/table/figure/appendix/references and process roles/prerequisites/three steps/warning/table/figure/exceptions/reference/record. Stored text matched all **19 PDF / 22 DOCX** inventory objects after Markdown normalization, with one exact managed figure each (`native-full-import-witness.json`). This external verification did not silently mutate host coverage: without an inventory supplied to import, stored state/UI still distinguish Limitation and Unverified, with no invented detected Omission. Final app reopened both full imports; protection, readable content and qualification persisted.

On `08bc9bb`, native Add useful information PDF Run `06feadb0-7877-4e47-a032-cddc00b077cc:0` Completed with receipt `5f2d3d68-5361-43c6-85f8-4027d731fdf1`; DOCX Run `8cca50cf-5337-4899-bdf3-94d619651cc4:0` Completed with receipt `46c6ab73-16f7-4312-8cc3-fae56ac4c1b1`. Each used budget 8 and normal **Codex / gpt-6.1-sol / medium**. The two native intake Threads sequentially updated the same ordinary Note `f275be9f-d13e-4e5b-99cd-881c6fab07a0:0` to v5. Both selected figures remain referenced; earlier PDF Limitation/Unverified and prior DOCX qualification survive. Seven protected notes remained byte/version-equivalent. No protected writing or human confirmation was silently adopted (`native-ready-sequential-witness.json`).

PDF PNG: **3,257 bytes**, SHA-256 `75817c9dd0fb816e03cc61a9edf1a20a85e1a12fe6a4922c57685a4a939a8273`, matching direct local extraction. DOCX PNG: **3,554 bytes**, SHA-256 `750a38a07b42f0d11cb1a7b1953405b457e105f38af6c15eb70ed66ac90a0c76`, matching the source fixture asset. After restart, repaired native layout displayed both CHECK→TEST→RELEASE figures and the 5/3/2 minute responsibility table. Coverage remained separate from claims in the Markdown; unknown full-import completeness was not called detected loss.

### Native export, backup and exact fresh restore

Native Settings buttons/file choosers on `08bc9bb` published `evidence/BRN-export-bf52b4c0-076c-4ad5-a09c-5437d90a95d8` and `evidence/BRN-backup-a95348fc-a6e4-41ac-bf44-4e75385b3b24`. Export contains **16 Markdown notes / 9 managed figures**; all **25** manifest file hashes matched, including both sequential-intake figures and their current Markdown references. Manifest SHA-256: `6daae657d21fce758899683bf8ad2db851008b428361f94dc04a3fd5bc58b3a4` (`native-final-export.json`).

Backup SHA-256: `edb8244b579a1d86dd68611cc8760ff49736270a670f203c4cbe4081bd04c035`. Supported CLI restore into the new explicit `data/native-final-restored` reproduced **112 records, 116 history receipts, one prepared review candidate, two recovery buffers and settings exactly**, including 16 Notes, 13 Threads, 12 Runs, 25 Links, 26 Messages, 9 Assets, 6 Sources, 2 Actions and 2 Comments. Backup into an occupied file refused without changing its hash. Restore into the occupied destination refused without changing any of the five application-state snapshots (`native-final-backup-restore.json`). No direct SQL mutation or existing user-data path was used.

### Remaining scope and owner acceptance

No observed agent acceptance item remains failed or blocked after repair. Temporary native capture failures while minimized/locked/asleep were resolved by the owner's restore/unlock/wake replies; normal pixel updates resumed without a renderer workaround. One transient capture failure at final launch succeeded on selecting the visible app again. No credential workaround was used.

This verifies these synthetic interactions and Harbor fixtures, not every IME or general complex/vector/scanned/math PDF fidelity. Other composition input methods were not tested. Credentials/private mailbox content were not inspected; accounts/providers were not switched and no external message was sent. The separately recorded Windows issue stays outside this Mac task. Historical hosted results remain baseline evidence; newly triggered repair CI is pending until separately observed.

**Owner acceptance remains pending.** The local candidate is open and usable for the [owner walkthrough](acceptance.md#owner-walkthrough). Agent observations and synthetic Confirm done do not accept BRN for the owner. PR #114 remains draft; main integration and release remain separate.


### Publication through the connected GitHub account

This MacBook's ordinary Git HTTPS push failed before changing the remote (`could not read Username`); the shell GitHub API client also returned HTTP 401. The available connected GitHub account published the task-owned changes instead, without inspecting credentials or requiring an account/provider change for live BRN checks.

Published product commit **`3eea93d3876e1b28bf8e01c8d554635126845d1d`** has the exact same Git tree **`b2dd33e813e20fc29eeb090b7146bdb3ce09c2b2`** as locally built/tested source **`c3a79cd859c222c3da7997697629c7fd6f1c2141`**. Every one of the 31 changed file blobs was SHA-verified; tree equality includes untouched files, file modes and the separate decoder harness. The local focused commit history is preserved in the isolated checkout. Published history uses one product snapshot commit with baseline `36dfef5` as parent and a subsequent evidence-only commit. The shared branch update is a checked fast-forward with expected old head, never a force push. The manifest records both tested local and published equivalent source identities.

Final desktop command: `cargo test --locked -p brn-desktop --features native-ui,native-retrieval,native-test-support --bin brn-desktop` with the same task-owned CARGO_TARGET_DIR: **21 passed, 0 failed**, including real GPUI regressions. Corresponding strict all-target Clippy with the same features passed; formatter, whitespace and current Markdown check passed (**94 files, 356 local links, 0 failures**). A preliminary command omitted native-test-support and ran only the 14 state tests; it was replaced by the full component command, not counted as the interaction witnesses.

## Rig 0.44 runtime integration

### Identity and publication

Started from fetched **`0fcb8f737952189724a7156dde2c483719e9a6bf`**, the previous MacBook session's published final checkpoint. That checkpoint was confirmed before any modification; product `3eea93d` and its Mac repairs were reviewed. Work used the isolated `threads-runtime-checkout` / `runtime/threads-rig044`; no synced project source or unrelated checkout was changed. Ending tested local product source is **`ed02bc927c1b0c687c482735fdda12c4b5a3fee8`**. Published product **`81a1131d1fe7e093e7b4743e1cf1e1f31688d942`** has its exact Git tree **`5c5a446e9e5c26c8b23bacc8848f37a954ee2c6e`**. Subsequent commits reconcile that identical publication and record evidence only.

Shell `git push` failed without changing the remote because HTTPS authentication was unavailable. The connected GitHub account published the exact product tree after SHA-verifying all 34 changed non-vendor file blobs and removing the obsolete vendor tree. Fetch independently confirmed tree equality and the remote commit. Branch advancement used `expected_sha=0fcb8f7…`, `force=false`; no force push. Local focused checkpoints remain preserved: `552f2fd` decoder qualification, `fc5afd0` provider fixtures, `f3cf82c` guides/loader, `17fc314` accepted history/candidate refresh, `d908998` runtime integration, `ed02bc9` live qualification and guide clarification. PR #114 remains draft; no main merge or release.

### Dependency and contract disposition

Production uses **published Rig 0.44.0** (`rig`, `rig-core`, `rig-agent`, `rig-reqwest` and related resolved packages), under unchanged Rust 1.98.1. The old vendored Rig 0.43 stderr patch, provenance and root `[patch.crates-io]` entry were removed. **No local Rig patch remains.** The standalone comparison intentionally retains both registry versions and its separate lock; it is not a production fallback.

The unchanged baseline comparison was rerun before changing expectations, then expanded to 12 fixtures × 2 versions. [Exact commands, sanitized results and classifications](rig-decoder-comparison.md) are preserved with the reproducer in `scripts/qualification/rig-decoder-comparison`. Harmless metadata is accepted. A malformed provisional numeric text delta followed by a valid finalized response is accepted under 0.44; literal corrupt JSON remains failure, and absent completion remains truncated. A streamed A followed by completed B yields authoritative B under 0.44. Incomplete `Length` termination is rejected by BRN, and malformed arguments remain invalid/non-executable. The earlier aggregate 15 upgrade failures included fixture/API adaptation and is not reported as 15 library regressions. Retained fixtures were migrated to the new public API and all qualified routes passed.

Old behavior accumulated preview text and ignored Rig's final content. New behavior uses successful `PromptResponse.output()` from Rig's structured final response, replaces the provisional preview and persists that answer once. This corrects provider restatements without duplicating A/B or saving stale fragments. Tool-loop provisional text does not become another saved Message. Both preview and authoritative ordinary final answer have a **128 KiB** limit. Strict full-document extraction retains its distinct 50 MiB bound. Missing/corrupt/incomplete termination and transport/user interruption do not invent successful completion; ordinary partial output is progress, strict extraction failure discards incomplete output.

Explicit policies are unknown/invalid tool handling `Fail`, invalid-call retry budget **0**, consecutive malformed-call budget **0**, unknown finish acceptance **false**, tool concurrency **1**, and Rig max turns `max_tool_rounds + 1` with an independent whole-batch hook stopping before an excess tool round dispatches. The host accepts 1–32 requested tool rounds (default 8). Application checks all proposed writes, current versions, protected scope, writing guards, run/settings fences and receipts. Retries cannot bypass those checks. Rig's marked `invalid_arguments` and typed object arguments are checked before dispatch of any batch member. Existing stderr regressions pass for unknown calls and malformed known calls on all three retained provider fixtures, so the old logging patch has no remaining demonstrated purpose.

Responses subscription/OpenAI routes use typed `GenerationOptions` / `Effort`, replacing BRN's raw reasoning JSON. Copilot Chat retains its small supported `reasoning_effort` field because 0.44 does not expose verified typed generation options for that dialect; silently dropping effort would be incorrect. No unrelated provider/authentication rewrite was made.

### Focused runtime corrections

- Protected content authority now comes from a trusted host `EditNote { id, version }` action, with the actual persisted owner Message identity, exact content-edit class and current invocation/settings fence. `RecordDecision` binds one new protected confirmed target. Native controls and explicit CLI flags produce these modes; they cannot be decoded from source or model text. Normal wording and Estonian content work. Grants do not survive Continue, authorize another protected note, bypass stale versions, or permit unprotect/archive. Ordinary enabled working-note maintenance remains permitted without a protected grant. A live ambiguity in the initial guide was corrected explicitly after the model unnecessarily requested permission for ordinary maintenance.
- Existing current-version long-note range/section/continuation reading from the Mac session was retained. New `read_history` provides bounded accepted-revision pages and exact historical passages; history is not loaded automatically. `refresh_candidate` uses the checked service to validate current bases/inputs, prepare an immutable replacement and retire the stale operation atomically, without granting apply permission or editing proposal rows directly.
- The real registered tool catalog also generates safe activity labels. Starting seven tools (the original six plus the Mac session's `read_note`) now total nine with history/refresh. Production tests traverse the original catalog and new history/refresh loops. No legacy tool-name filter remains. Duplicate unused skill tool/loading implementation and source-result semantic prose were removed; operating guidance is in packaged Markdown.
- Independent source review found and closed one additional defect: model tools could write assistant Messages before final storage. All model Message creation, editing and archiving, including refreshed requests, now fail whole-batch admission. Conversation persistence is host-only. Independent focused regression passed after closure; no additional concrete blocking source defect was found.

### Automated verification

All outputs below are local synthetic logs under `../threads-runtime-local/evidence`, outside Git. Commands were run from the isolated checkout with absolute `CARGO_TARGET_DIR="$PWD/../threads-runtime-local/target"` and existing local PDF utilities, without credentials in output.

| Gate | Exact command / result |
|---|---|
| Final workspace | `scripts/verify-storage.sh` with the absolute target, `BRN_PDFTOTEXT="$HOME/.local/bin/pdftotext"`, `BRN_PDFIMAGES="$HOME/.local/bin/pdfimages"`: fmt, locked offline helper/workspace build, workspace all-target strict Clippy, all workspace tests, startup/refusal checks passed. **352 passed, 3 intentional ignores**, log `verify-storage-final.log`. |
| Production Threads | `cargo test --locked -p brn-ai threads -- --nocapture`: **34 passed** (includes history module), log `runtime-regressions.log`. Tests cover final B vs preview A, ordinary agreement, malformed/unknown calls and batch dispatch zero/no retry, incomplete/unknown finish, protected target/version/source/stale/wording, catalog/tool-loop single final, reopened long-note tail and service refresh/history. |
| AI/provider capability | `cargo test --locked --offline -p brn-ai --features capability-spike`: **194 passed, 1 intentional stderr-child ignore**, log `capability-tests.log`. The child is executed by its parent. Strict all-target Clippy with the same feature passed (`capability-clippy.log`). Provider format subset separately **82 passed, 1 child ignore**. |
| Shared core/app | **45 core tests** and **18 app tests**, included in final workspace counts. Revision reads, continuation, refresh atomicity/ownership/fences/replay/current inputs and guarded/protected behavior passed. |
| Native components | `cargo test --locked --offline -p brn-desktop --features native-ui,native-retrieval,native-test-support --bin brn-desktop`: **21 passed**; matching all-target strict Clippy passed (`native-tests.log`, `native-clippy.log`). The Thread-owner component regression now checks authoritative replacement of provisional A while another Thread is visible. |
| Product fixtures | `TMPDIR="$PWD/../threads-runtime-local" scripts/verify-end-to-end.sh --fixtures-only` with the same absolute target passed provider/intake retirement and Save/recovery/search/export/backup/startup/refusal journeys (`end-to-end.log`). |
| Decoder/dependencies | Locked offline dual-version harness run/fmt/Clippy passed as recorded in its comparison. Production lock resolves registry 0.44.0, no root patch/vendor dependency. Formatter, `git diff --check`, and current local Markdown links passed. |

Counts overlap; they are not added together. Final guide clarification changed packaged model instructions, then the workspace gate and guide tests were rerun and both shipping binaries rebuilt. The retained `block 0.1.6` future-incompatibility notice is upstream baseline, not a Clippy failure.

### Synthetic live provider qualification

Exact repeatable command (fresh directory required):

```sh
python3 scripts/qualification/threads-runtime/live-journeys.py \
  --binary "$PWD/../threads-runtime-local/target/debug/brn" \
  --data "$PWD/../threads-runtime-local/live-final-data" \
  --auth-file /Users/evokessler/.codex/auth.json \
  --evidence "$PWD/../threads-runtime-local/evidence"
```

The configured route was **Codex / ChatGPT subscription, `gpt-6.1-sol`, medium**, on fresh synthetic storage. Authentication was used by the supported adapter; credential contents/private mailbox were not inspected. No external messages were sent. Each successful journey verifies exactly one new host assistant Message equals the returned authoritative final answer. Logs/records are `live-final.log`, `live-journeys.json`, `live-records.json`, `live-state.json`.

| Journey | Observed disposition |
|---|---|
| Normal question | Completed `SYNTHETIC_READY_044`, one saved answer. |
| Tool-assisted maintenance | Working note updated Friday→Tuesday through read/prepare/apply; actual current text and single saved final verified. |
| Long full note after reopening | Imported a synthetic text full note of **171118 bytes** (over the 128 KiB first-chunk limit), then a fresh CLI process/runtime read the actual passage beyond byte **171021**. Returned **AMBER_OTTER_044** and count **29** with passage evidence. |
| Explicit owner decision | New confirmed protected synthetic route note created/applied under the host decision mode. |
| Protected edit with scoped authority | Estonian request changed blue→green at the exact selected version and kept protection. Existing core behavior invalidated prior confirmation on content change; the model reported this honestly rather than claiming confirmation persisted. |
| Same class without grant | Prepare/apply returned NeedsReview; exact current version/data remained unchanged. |
| Malformed arguments | Live model declined the requested invalid `ids:7` call and issued neither mutation. This is a live refusal witness, **not** a claim that the provider emitted malformed arguments. Actual malformed calls are injected through the production runtime tests, proving no batch member dispatches and no retry executes. |
| Cancel and Continue | SIGINT persisted Interrupted and saved no assistant completion (this early interruption had zero preview characters). Fresh same-Thread Continue completed `SYNTHETIC_CONTINUED_044` once. Partial-stream retention is covered by collector and production interruption fixtures. |
| Tool-loop final persistence | All successful tool journeys saved one answer matching final runtime output, without intermediate fragments. |

A preliminary qualification-script check incorrectly assumed records were chronologically ordered; the store orders by ID. That fixture assertion was corrected to identify new Message IDs. It is not a runtime or Rig regression. The first live ordinary-maintenance refusal led to the useful guide clarification described above; the final fresh suite passed.

### Rebuilt Mac candidate and remaining physical blocker

Local final candidate:

`/Users/evokessler/.codex/.chatgpt-projects/g-p-6ab8ee911378819187a66bf0ee50da76/threads-runtime-local/BRN Threads Rig 044 Final.app`

Built on this Mac from tested `ed02bc927c1b0c687c482735fdda12c4b5a3fee8` using:

```sh
cargo build --locked --offline -p brn -p brn-desktop -p brn-intake \
  --features brn-desktop/native-ui,brn-desktop/native-retrieval,brn-intake/helper \
  --bin brn --bin brn-desktop --bin brn-intake-helper
scripts/make-macos-app.sh \
  --output "$PWD/../threads-runtime-local/BRN Threads Rig 044 Final.app" \
  --binary "$PWD/../threads-runtime-local/target/debug/brn-desktop" \
  --intake-helper "$PWD/../threads-runtime-local/target/debug/brn-intake-helper" \
  --data-dir "$PWD/../threads-runtime-local/native-data"
```

The local bundle has a distinct test display name/identifier to disambiguate prior candidates, with no source/runtime change. `mac-candidate.json` records identity, fresh data and hashes. Desktop SHA-256 **`631bbe140640a0503f40a88387ad466375277a7d88d065b37f29a69ded3131a5`**, helper **`75d31964c6057a70c5291363478492e0e8541476ffb2d53e7a3bbe3975d6cd89`**, launcher **`81f2ff9b53efcd33959910f0ae014d78ead4c18e26649094ba74e531f3297ab3`**. Copied binary headless startup passed. Generated bundles/data stay outside Git.

Actual native controls on fresh data were exercised through computer use: normal question completed; selected protected note exposed **Edit selected note · this request**; Estonian request applied blue→green once; an ordinary same-note request produced **Review this change** and preserved the current note; a long native run's Stop persisted **Interrupted** and a run-linked Continue blocker. `native-records-checkpoint.json` / `native-records-final.json` corroborate the current protected v2 text, three successful Runs/Messages and the interrupted run with no assistant completion. The normal question was on the preliminary `d908998` binary; authority/review/Stop used the final guide build.

Screen captures remained frozen on the launch screen even while control accessibility/state and persisted results changed. The computer-use service subsequently explicitly returned: **“The Mac is locked and automatic unlock could not unlock it.”** Manual wake/unlock was requested while independent work continued. There is no claimed visual pass for final stream reconciliation/activity or native Continue. Those affected native observations remain **blocked pending manual unlock**; this pass is not declared fully accepted. No renderer workaround or unrelated system setting change was made.

Prior Mac evidence remains applicable to unchanged editor/focus/IME/Save/recovery/comment/source conversion/assets/coverage/search/current long-note storage/Undo/export/backup behavior. Existing production long-note capability was retained, not reimplemented. Runtime answer/progress and authority presentation changed, so old provider/worker presentation observations do not substitute for the affected visual retest of this binary. Prior automated integrity checks were rerun as above.

Remaining limitations are the physical native gate, independently pending owner acceptance, untested general IMEs/complex-scanned-math PDF fidelity and the previously recorded Windows credential issue. New hosted CI is pending until observed. [The small runtime owner walkthrough](acceptance.md#runtime-owner-walkthrough) follows the separate native retest; owner acceptance has **not** been marked complete.
