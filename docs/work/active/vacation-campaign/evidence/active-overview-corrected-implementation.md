# Active overview history compaction — final handoff

## Final candidate

Branch: `codex/concise-active-overview`, based on freshly fetched `origin/main` `e918b2b303f44cfca6cbd49347ea5b4fd4e335f1`.

Final HEAD: `7ccb552efc152443f1ae57a044673500e7fe91c3`.

The review correction commit is `41d38d427b52e5e5c4469d9fd1c97d59e47a938f`; its branch-local overview derivative is committed separately as `7ccb552efc152443f1ae57a044673500e7fe91c3`. The initial source-only candidate `a61325dbc0e17c85bc8ed30207483d2f6d71e4cf` remains in history.

Final changed paths from main:

- `scripts/render-work-overview.py`
- `scripts/tests/test_work_overview.py`
- `docs/work/overview.md` (generated only from this branch's unchanged committed campaign record)

## Behavior and review correction

The startup overview still includes current task title/ID, owner, state, observation time/origin, activity, next action, dependencies, resource waits, gate values and the real dependency graph. It no longer repeats raw evidence paths or correction/attempt history from active and blocked task records. The exact selected record remains available through the existing validated `--task` route.

Independent review found the first compaction made history hard to find when there were no completed tasks: the detail command and selected-record link had appeared only in the Completed section, and active task headings lacked IDs. The corrected renderer places one selected-record link and one custom-`--record`/`--task` instruction in the shared overview header, and displays each noncompleted task ID beside its summary (including the compact active-with-wait presentation). Completed task IDs remain in the compact index. No new option, tracker, file, or framework was added.

The new growth regression gives both active and blocked tasks 120 evidence entries and 40 attempts, compares their rendered projection to equivalent records with minimal evidence and no attempts, then verifies both complete task objects remain exactly retrievable via `--task`. A second regression renders a valid no-completed active/ready/blocked record using a custom record and custom output path, and verifies the linked record path, one detail instruction, and all three task IDs. Existing custom task lookup and output-preservation tests remain in place.

## Verification

- Pre-correction focused regression failed against the old renderer because raw history changed the summary; unittest reported a 58,696-character diff.
- Final source: `python3 -m unittest scripts.tests.test_work_overview` — **15 tests passed**.
- `git diff --check` passed.
- Normal renderer generation succeeded on this worker branch. `python3 scripts/render-work-overview.py --check` passed afterward.
- The branch-local input record hash was `c54e90923ff70271416636dedf06f235f06e7daca58ef0dd3432ce8869283aa5` before and after generation. The generated overview changed from **18,880** to **10,565 bytes**. The campaign record was not modified.

The regenerated overview is internally consistent with this worker branch's own committed record; it is not asserted to be the current lead task authority. No lead campaign, current overview, evidence, or other generated file was changed. No Rust/build/native/account/provider checks were run. Independent review of the complete corrected candidate and integration remain with the lead.

The checkout is clean at final HEAD `7ccb552efc152443f1ae57a044673500e7fe91c3`.
