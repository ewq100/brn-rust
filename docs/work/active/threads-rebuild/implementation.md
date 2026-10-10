# Threads implementation record

Build assignment: 10 October 2026. Starting remote commit: `a1c61e245ec90a0eed98ffc5bbfcab57ddeb1926`. Isolated checkout: `/Users/evokessler/repos/brn-threads`. Existing checkouts and dirty files are preserved. The accepted target, behavior design, plan and reconciled review remain authoritative; no second whole-plan review is required.

## Resource and execution bounds

The current selected lead continues implementation and integration. At most two active helpers: Sol/high for the independent core and intake integrity slices; Luna/medium completed the read-only reuse inventory and packaged guidance. The lead owns root Cargo files, CLI/runtime/native wiring, documentation, gates and pushes. Reassess only for a concrete conflicting requirement or repeated evidence-backed failure. Helpers do not delegate further.

Task build/evidence/data parent: `/Users/evokessler/repos/BRN-local-builds/threads-20261010`. Helper targets are separate. No old BRN data is inspected or migrated. Prerequisites: Apple Silicon Mac, Rustup, protoc and Apple compiler tools found. Cargo proxies require `/opt/homebrew/opt/rustup/bin` on PATH. Rust 1.99.0 installed; unchanged-lock qualification is in progress. Baseline lock SHA-256: `16fe749fc0e4592f2bf2065c660203ded43e5c1c7b03c803d52abceaa9a8ffb7`.

The native-session inventory reports the Mac locked. Do not attempt unlocking. One pending interaction task covers long-note reading/typing/selection/Undo, comments and mappings, review, focus/dialogs, search, Actions and recovery. Provider selection is persisted inside old app SQLite; do not inspect it. Exact provider/model/effort/credential path requested from owner; offline work continues.

## Dependency, reuse and deletion map

| Component and source | Reuse/adapt | Replacement and deletion boundary |
|---|---|---|
| `brn-store/src/work`, `brn-workflow/src/{editor,proposal_apply,files}` | SQLite transactions/backup and useful adversarial cases | New `brn-threads-core` owns final schema, revisions, request/candidate/receipt, guards and compensation. Remove old mutable-vault journals, migrations and repair when CLI/desktop consumers switch; never import old data. |
| `brn-ai/src/{auth,chat,error,provider_formats_tests,provider_stderr_tests}` | Secure subscription auth/transport, selected-model/effort, safe errors, streaming and cancellation | Qualify published Rig 0.44 separately. Replace route-specific proposal/rewrite tooling with host-context tools over core. Remove vendor only after actual stderr regression passes. |
| `brn-desktop/src/native`, shell/theme/controls | GPUI platform, editor, Markdown reader and protocol-owned figure handling | Qualify Kit 0.7.1 separately, then replace Inbox/approval/old worker screens with Home/thread/notes/review/history. No permanent old runtime. |
| `brn-intake/src/helper.rs` and helper protocol | BetterOffice, EML/image decoding and bounded process isolation | New transient intake adapter supports intent/coverage and maintained PDF path. Remove workflow original capture/retention only after switching consumers. Keep substantive full-note assets. |
| `brn-retrieval/src/{chunk,note_index,native}` | Current-revision passage search; optional semantic model | Feed new current records, exclude candidates/archives. Indexes remain rebuildable. Remove vault-source library adapter with old workflow. |
| `brn/src/cli`, desktop `main.rs` | Thin entry point shape and explicit arguments | Switch both to shared new core with required explicit fresh data; reject old/mixed markers. Retire old domain command families coherently. |
| `.github/workflows/ci.yml`, scripts | Required context names and meaningful behavioral cases | Include new core/runtime/intake/CLI/native gates, replace retired-byte-authority/migration assertions with new proofs. Do not mask or weaken failures. |

Coupling: workflow imports store, AI, intake and retrieval; desktop and CLI import workflow. Deleting an individual old crate before consumer rewiring breaks the graph. Tests encode both useful behavior and obsolete mechanisms; retire only with their replaced production route. Shared Cargo/lock edits belong to the lead.

## Executable slices

- [ ] Compiler-only qualification: formatter, workspace build/tests/lint, optional native UI/retrieval/combined graphs against unchanged lock; retain 1.98.1 until qualification evidence supports adopting 1.99.0. Retained scripts resolve the repository pin from root, including when invoked elsewhere.
- [ ] M1 core: durable host operation before prepare; immutable preparation; atomic apply and replay receipts; target/action grants; expected record versions; generation-fenced guards and Save; write-set Undo with later-edit preservation; backup/restore and incompatible-data refusal. Highest-seam integration tests use multiple connections/restart and lost responses.
- [ ] Separate Rig qualification: published logging regression, synthetic ChatGPT/Copilot stream/auth/error/cancellation/budget cases, malformed-read + valid-mutation batch with no tool dispatch/extra request. Then new runtime integration.
- [ ] M2: fixed guide catalog + allowlisted `read_skill`; BRN-owned interrupted-run continuation, synthetic email/project/Action/reply journey, protected conflict and raw-input cleanup canary. Retain actual operation receipts and source observations.
- [ ] Import qualification: ordinary text PDF and process DOCX independent inventories, full substantive wording/structure/assets, deliberate gap yields Partial; temporary originals cleaned while retained knowledge remains offline/exportable/backed up.
- [ ] Separate GPUI qualification and M3: native Home/Needs you, Ask/delegate, threads, notes, base/generation Save, review/comments/search/Actions/history/export and backup. Actual interactions stay pending while locked.
- [ ] M4: coherent entry point/CI retirement, scoped independent integrity review, required broader gates, synthetic/live journeys when route available, draft PR maintained, tested commit and limitations recorded. No merge/release.
