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

Unsigned `BRN Threads Candidate.app` assembled from the final combined native binary plus maintained helper, using only fresh `data/native-candidate-schema3`. It includes a36,852-byte synthetic protected long-note/managed-figure fixture. Actual final native binary headless startup passed (`threads-candidate-packaged-startup.log`, `threads-candidate-seed.json`). No UI launch/unlock was attempted.

The single pending [native acceptance task](acceptance.md) covers long-note rendering/IME/selection/table/figure/focus, comments before/inside/deletion plus reopen, protected review, Save/recovery/Undo, search/Actions, cancellation/Continue and export/backup. Native interaction and owner acceptance remain unverified because the Mac reported locked. Immediate Quit only requests token cancellation; abrupt process exit cannot guarantee a terminal Interrupted receipt, so saved Working state is shown honestly and can Continue. General complex PDF hierarchy/vector composition/scans/math fidelity, external sending, Office/HTML generation and semantic model installation are not claimed. Required hosted results and exact candidate source commit are recorded below; no main merge or release.


### Published candidate source identity

Candidate implementation commit: `1f7e7fd80111cb29d2fb562fb2c548b685721093`. All210 retained source/guide/gate files were matched byte-for-byte to the tested candidate manifest after commit; digest `6222fead400e7af0f86dbcad655ee62bb96b8568f937641a5a901d4555cd2b69` (`threads-candidate-source.json`). The following handoff documentation commit does not alter product source. The packaged native binary/helper and bundle hashes are retained in `threads-candidate-bundle.json`. Final Markdown result:93 files,344 local links,0 failures; formatter and whitespace passed.

The isolated branch was reconciled with remote before publication (no divergence, no force push). PR #114 stays draft. Hosted checks start on the published handoff head and remain separately pending until observed; local checks above do not stand in for them.
