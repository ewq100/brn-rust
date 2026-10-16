# B4 minimal real-Workspace writing host

Provider OpenAI/Codex, model gpt-6.1-sol. Bound checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc`; new branch `work/b4-20261017` created only after clean exact accepted B3 HEAD `c5b257fb643794e0802dd1cb4d3fee5ff3ee0edf`, crates `0afb215e543bc5a2512b1fac24107169a39818af`. Local origin/main is inspected without fetch; no B3 ref or other worktree changes. No D1B/D2 integration.

Authority: [master §9 B4, F18/F22 and §7 seams](../proposal/final/01-master-plan.md), [lead prompt](../proposal/final/02-foundation-lead-prompt.md), [Stage2 B4/INT-W](../../build-sequence/README.md), canonical target/behavior/invariants and development workflow. [Accepted B3](../b3/principal-disposition.md) supplies reviewed reusable panel/context/evidence behavior. B4 proves that same implementation over the real service without BRN navigation or desktop dependency; it is not INT-W or another product.

## Exact ownership and composition

- New `crates/brn-writing-ui/examples/writing_host.rs`: native-only example entry, automatic own disposable data lifetime.
- New sibling `examples/writing_host/{fixture,host,journey}.rs`: synthetic complete-conversion receipt through existing importer; thin concrete host borrowing the public panel and real Workspace; native owner-event/shared scripted service journeys. No desktop code/module imports.
- Exact allocated `crates/brn-writing-ui/Cargo.toml` additions: required-native-ui example; development-only existing `brn-ai/scripted-runs`, `rig =0.44.0/test-utils`, `tokio 1/rt-multi-thread`, `tempfile 3`, `serde_json 1`, `brn-threads-intake` path. Existing production dependencies and feature definitions remain byte-identical.
- Conditional granted Cargo.lock reconciliation: only six existing-package dependency edges from brn-writing-ui. Whole package/source/version/checksum inventory must remain exact; stop on unexpected changes.
- B4 docs and minimal status/plan/evidence links only. All existing library, desktop, app/core/AI/CLI, other manifests, CI and scripts remain principal-owned. No public seam edit is currently needed.

The public `WritingPanel`/`WritingView::note_tile`, controls, comment UI, review UI and document callbacks remain single library implementations. Host requests call existing Workspace operations; exact saved request goes through panel.rewrite_run_start -> Workspace.prepare_thread_run -> brn_ai::threads::run_prepared_scripted. There is no provider selection/search/runtime/preparer copy, host trait, navigation/task state or second permission store. One ordinary Tokio executor runs the bounded offline script synchronously; the example has no background worker, network or production run engine. The script is visibly synthetic and development-only.

## Bounded sequence and acceptance

1. Inspect public panel, checked active context, request snapshot, contextual display/evidence batch, importer and review/Undo APIs; prepare the small concrete composition and real owner-event fixture.
2. Finish source, record fingerprints, then use the principal's explicit exclusive heavy grant: initial binding/process check, offline lock resolution with exact package inventory audit, focused example compile/lint/journey.
3. Verify F22 default graph/build/lint/tests and native library/desktop regressions proportionate to added dev-dependency feature unification. No unrelated retrieval/model/benchmark campaign; retain baseline PDF failure and any new failures at actual source identities.
4. Pin only compiled coherent source/test plus receipts for fresh principal Opus1. Freeze source and explicitly release heavy after reaping; principal owns acceptance, shared source disposition and INT-W.

F18 journey: real disposable Workspace, synthetic canonical working Note/reference+brief, same editor/comment controls, shared active context, captured passage/feedback, actual shared preparer/scripted provider/tools/ReadLedger, exact candidate and checked Sources used/review/Apply/Undo. No desktop dependency. F22: example only native-ui, default workspace GPUI-free. F04/F06/F09/F10/F12/F15/F16 remain shared service/panel contracts; host adds proportional witnesses, not another app acceptance suite. Unsupported development actions report unavailable rather than a success. Imported partial-original coverage, retained bytes read, model claims and verified quotes stay distinct through existing service evidence.

No missing-example compile error is a behavioral before proof. New host journey is the independence witness. B3 mutation budget 217.265250/600 is closed and not transferred; B4 mutation is not granted. Native-human/IME/picker, hosted/full F21/owner/program, live converter fidelity and missing-pdftoppm PDF qualification remain open. No live provider, credentials, model download, owner data, GUI, publication, push, PR, merge or release.


## Verified-window boundary

The prepared implementation and exact six lock edges are locally checked as recorded in [verification](verification.md). Existing production library/services remain unchanged. The compiled source will be pinned for principal Opus1, then the heavy slot explicitly released; no further feature work is assigned. Current B4 native tests need their own explicit example lane, listed in that handoff for principal-owned CI/scripts. No mutation is authorized.
