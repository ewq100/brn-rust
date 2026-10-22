# Read-only selected-task lookup

- Branch: `codex/selected-task-reading-route`
- Base: `79fa3980c5022e475b07888d9e08f396f45b0238`
- Initial candidate commit: `2cdce8325bee82c2888e4f86c44cca88c82564de`
- Corrected candidate commit: `cadcec0373e4542ffb1e43a8152bba343e142f8f`
- Changed files: `scripts/render-work-overview.py`, `scripts/tests/test_work_overview.py`, `docs/development/workflow.md`, generated `docs/work/overview.md`.

Added `--task ID` to the existing overview renderer, mutually exclusive with `--check`. Lookup uses existing `read_record` validation and emits only the exact selected task JSON (`ensure_ascii=False`, indent 2). Unknown IDs use `InvalidRecord` and return exit 2. The lookup returns before rendering or writing, so `--output` is ignored and remains untouched, including when its parent does not exist. Existing `--record` custom paths still work.

The new tests cover exact selected output including non-ASCII text, byte preservation for record and existing output, no creation of a missing output parent, unknown ID, full-record validation before selection, and `--task`/`--check` mutual exclusion without output creation. The workflow now recommends targeted ID lookup before opening a large record; the generated Completed index points readers to the same command.

Checks run:

- `python3 -m unittest discover -s scripts/tests -v` — **PASS**, 188 tests.
- `python3 scripts/render-work-overview.py --task visibility-adoption` — **PASS**, printed the selected task JSON.
- `python3 scripts/render-work-overview.py --check` — **PASS**, generated overview current.
- `python3 scripts/check-markdown-links.py` — **PASS**, 625 files, 2,850 local links, 0 failures.
- `git diff --check` — **PASS**.

No Rust builds, BRN runtime/CLI workload, provider, account, native, network, dependency, or global-setting work was done. The oversized live lead campaign record was not read wholesale; the available checkout's default campaign was accessed only by the renderer command and narrow ID search. Worktree clean after the candidate commit.


## Independent-review correction

Independent review found the generated Completed-index command omitted `--record`, which could read the default record for a task ID listed in a custom selected record. Commit `cadcec0373e4542ffb1e43a8152bba343e142f8f` corrects the instruction to use `--record PATH_TO_SELECTED_RECORD --task TASK_ID` and explicitly replace the path with the linked selected record and the ID with one listed. The existing custom-completed-overview test now asserts the portable custom path and both replacements. Workflow's short default example and custom-record note are unchanged.

The original commit `2cdce8325bee82c2888e4f86c44cca88c82564de` had a full suite result of 188 passing tests; this remains historical evidence for that candidate. On the corrected candidate, reran `python3 -m unittest discover -s scripts/tests -v` — **PASS**, 188 tests; `python3 scripts/render-work-overview.py --record docs/work/active/vacation-campaign/campaign.json --task visibility-adoption --output /tmp/brn-unused-overview-output/should-not-exist.md` — **PASS**, exact task JSON printed; `python3 scripts/render-work-overview.py --check` — **PASS**; `python3 scripts/check-markdown-links.py` — **PASS**, 625 files, 2,850 local links, 0 failures; `git diff --check` — **PASS**. The explicit output path's parent remained absent. Corrected worktree clean after commit.
