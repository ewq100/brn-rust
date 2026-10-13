# Ordinary wiki guidance evidence

**Baseline:** `65119a73` (origin/main confirmed 13 October 2026). **Isolated branch:** `codex/ordinary-wiki-guidance`; prior research commit `f7e150a9`. Main has unrelated untracked `experiments/`, preserved.

**Owner scope:** [spec](spec.md). Runtime implementation and controlled comparison are authorized. UI is design/prototype first. Comparison selection is Codex `gpt-6.1-sol`, medium. No owner knowledge is used.

| Gate | State |
|---|---|
| Defined | Satisfied by current owner request and two clarification answers. |
| Environment | Read-only preflight passed: pinned Rust 1.98.1, Cargo, Python and Git available. |
| Plan review | Approved in round 2. Applied input-lineage inspection was added after the first review; protected import is explicitly a reference, not confirmed owner authority. |
| Baseline capture | Locked offline CLI/helper build passed before guide edits. Baseline SHA-256: `20c5151df943979f04d31312a1b594f4309d177cc23bfc7efe193c05d7228e16`, bundle 2. |
| Implementation | Bundle 3 guidance and controlled CLI harness implemented. No schema/write-path/native UI change. |
| Offline checks | Baseline and candidate each passed 363 runtime/core/app tests (one pre-existing ignored test). Relevant strict Clippy, formatting, nine harness tests and Markdown links passed. Offline pilot seeded six arms successfully; semantic results correctly marked not run. |
| Live comparison | Pending harness and baseline/candidate binaries. |
| UI prototype preview | Pending. Native UI implementation is outside this pass. |
| Owner acceptance | Runtime live results and UI variant acceptance not yet observed. |
| Publication / merge | Pending draft publication; merge and release not authorized by this assignment. |

The comparison must distinguish structural/write safety from semantic quality; failed/incomplete runs remain evidence, and elapsed time does not establish token/dollar cost. Prior research checks do not qualify the new guidance.

## Reproduction and local artifacts

Builds and focused tests use `CARGO_TARGET_DIR=/private/tmp/brn-wiki-evaluation/build`, `--locked --offline`, and explicit `BRN_INTAKE_HELPER` for tests. Logs are in `/private/tmp/brn-wiki-evaluation/logs/`. The harness red witness is the missing module before implementation; green is nine meaningful harness integrity checks. Existing provider mocks verify tools/write boundaries, not semantic superiority of this guidance.

`python3 scripts/evaluate-wiki-guidance.py --baseline-bin /private/tmp/brn-wiki-evaluation/bin/baseline-brn --wiki-bin /private/tmp/brn-wiki-evaluation/bin/wiki-brn --helper /private/tmp/brn-wiki-evaluation/build/debug/brn-intake-helper --root /private/tmp/brn-wiki-evaluation/offline-1 --baseline-commit 65119a73 --candidate-commit guidance-3-working-candidate --repetitions 1` passed fixture preparation, producing six synthetic arms with semantic evaluation not run. Live comparison adds explicit `--live --provider Codex --model gpt-6.1-sol --effort medium --repetitions 2`, uses a new root and exact candidate commit, and keeps profile paths outside this evidence.
