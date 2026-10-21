# Concise completed-task overview

- Checkout: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`
- Branch: `codex/concise-completed-overview`
- Base: `f54fcc2e4f18f590601c1ddb06c4b98470a0df84`
- Commit: `32a61e44080f58b0daf83d29dc5788a825cbbfcc`
- Changed files: `scripts/render-work-overview.py`, `scripts/tests/test_work_overview.py`, `docs/status.md`.

The renderer still validates the complete work record and computes eligibility and dependency edges from all tasks. In the Completed section, it now prints stable ID/title rows and a direct link to the exact selected `--record` file. The introductory text states that completed task scope does not establish every gate, native observation, or owner acceptance, and directs readers to the record for task details, gates, evidence, and attempts. `render` accepts an optional `record_path` defaulting to the existing default record; the CLI supplies its selected record. Status wording now describes a compact index with detailed authority remaining in the task record and receipts.

The prior verbose-history regression now exercises the CLI with a temporary selected record, asserts the index, direct relative record link and completion caveat, confirms verbose content is absent from the generated Completed section, and verifies the input record bytes plus gate/evidence/attempt data are unchanged.

Checks run: `python3 -m unittest discover -s scripts/tests -p test_work_overview.py` passed 8 tests; `git diff --check` passed. No overview/campaign regeneration, Rust build, account, native, or network action was performed. Lead owns final current-record rendering/validation and independent review.
