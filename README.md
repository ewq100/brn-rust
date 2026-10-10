# BRN Threads rebuild

BRN is being redesigned as a local workspace where an AI agent maintains useful knowledge, carries work forward, and opens a thread when the owner needs to decide something.

**Branch:** `rebuild/threads`. **Status:** the rebuild environment and instructions are prepared; the new product core is not implemented. Existing application code is retained temporarily as a component source and reference.

## Start

- [Agent instructions](AGENTS.md)
- [Canonical Threads target](docs/architecture/threads-target.md)
- [Build plan](docs/work/active/threads-rebuild/plan.md)
- [Independent review brief](docs/work/active/threads-rebuild/review-brief.md) and [review status](docs/work/active/threads-rebuild/review.md)
- [Build agent prompt](docs/work/active/threads-rebuild/build-prompt.md)
- [Development setup](docs/development/setup.md) and [current progress](docs/status.md)

## The target

One authoritative SQLite store supports Notes, Threads, Actions, and History. The AI maintains ordinary working notes under delegation, while protected notes preserve decisions, writing, and imported references.

Routine intake extracts useful information. Selected key documents can become complete readable notes, including meaningful tables, equations, and figures. Original emails and document files stay in their existing locations.

The first usable release includes a native GPUI workspace, direct writing, review where needed, current search, Actions, recoverable local changes, Markdown export on demand, and complete BRN backup/restore. DOCX/PPTX/HTML generation follows later.

## Development baseline

The reference is [main at af9239c](https://github.com/ewq100/brn-rust/commit/af9239c741c7ab0983e62f0253e607b9607727e6), including PR #113. The rebuild uses a fresh core and schema in this repository. No user-data migration or permanent compatibility engine is required.

Use the setup guide before running the existing binaries: they still implement the baseline behavior, and old default data paths must not be used for rebuild trials.
