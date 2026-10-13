# BRN Threads

BRN is a local workspace where an AI agent maintains useful knowledge, carries work forward, and opens a thread when the owner needs to decide something.

The SQLite core, shared checked writer, provider runtime and native workspace are implemented. Phase 0/F1, Phase 1, cleanup and the tiled writing workbench are merged through [PR #123](https://github.com/ewq100/brn-rust/pull/123), including the app-wide tiled UI. [Status](docs/status.md) records verification and remaining recovery/native acceptance. The Omakase tiled shell ([#125](https://github.com/ewq100/brn-rust/pull/125)) and GPUI Kit 0.7.3 ([#126](https://github.com/ewq100/brn-rust/pull/126)) ship in the [`v0.1.0-preview.2` macOS preview](https://github.com/ewq100/brn-rust/releases/tag/v0.1.0-preview.2); [preview evidence](docs/work/active/tiled-app-preview/evidence.md) separates package verification from pending native acceptance.

- [Documentation index](docs/README.md) and [agent guide](AGENTS.md)
- [Canonical Threads target](docs/architecture/threads-target.md) and [behavior](docs/architecture/threads-behavior.md)
- [Development setup](docs/development/setup.md) and [verification](docs/development/verification.md)
- [macOS preview installation and release limits](docs/development/preview-release.md)
- [CLI](crates/brn/README.md), [desktop](crates/brn-desktop/README.md) and [historical evidence](docs/archive/README.md)

One authoritative SQLite store supports Notes, Threads, Actions and History. AI maintains ordinary working notes under explicit delegation; protected notes preserve decisions, writing and imported references. Routine intake extracts useful information. Selected documents can become full readable notes with meaningful tables and figures; mathematical equation fidelity is outside the first release. Original emails/files stay external.

Markdown export is on demand; backup/restore covers complete BRN state. DOCX/PPTX/HTML generation follows later. Command-line launches require a fresh explicit absolute `--data-dir`; the preview app launcher supplies its stable per-user directory. Other BRN formats refuse; old data is not opened or migrated.
