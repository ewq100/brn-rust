# BRN Threads

BRN is a local workspace where an AI agent maintains useful knowledge, carries work forward, and opens a thread when the owner needs to decide something.

The SQLite core, shared checked writer, provider runtime and native workspace are implemented. Phase 0/F1 and Phase 1 delegation, steering, backup, restore and remembered backup folder are merged through [PR #120](https://github.com/ewq100/brn-rust/pull/120). [Status](docs/status.md) records verification and remaining recovery/native acceptance. The current assignment is [pre-release cleanup](docs/work/active/pre-release-cleanup/plan.md).

- [Documentation index](docs/README.md) and [agent guide](AGENTS.md)
- [Canonical Threads target](docs/architecture/threads-target.md) and [behavior](docs/architecture/threads-behavior.md)
- [Development setup](docs/development/setup.md) and [verification](docs/development/verification.md)
- [CLI](crates/brn/README.md), [desktop](crates/brn-desktop/README.md) and [historical evidence](docs/archive/README.md)

One authoritative SQLite store supports Notes, Threads, Actions and History. AI maintains ordinary working notes under explicit delegation; protected notes preserve decisions, writing and imported references. Routine intake extracts useful information. Selected documents can become full readable notes with meaningful tables and figures; mathematical equation fidelity is outside the first release. Original emails/files stay external.

Markdown export is on demand; backup/restore covers complete BRN state. DOCX/PPTX/HTML generation follows later. Use a fresh explicit absolute `--data-dir` with either binary. Other BRN formats refuse; old data is not opened or migrated.
