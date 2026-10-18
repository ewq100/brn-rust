# Findings and limits

The requested dialog failure was **not reproduced locally**, and no fix is claimed. The unchanged exact test passed1/0/0. The pinned macro's supported `SEED=0 ITERATIONS=100` mode passed seeds0–99; it is one diagnostic campaign, not100 independent checks. One unchanged full widget context run followed because isolation did not reproduce the hosted symptom. It finished300passed/5failed/1ignored,3filtered; the dialog case passed. Every failed result is preserved, with no retry-to-green.

## Ranked hypotheses stated before testing

1. The dialog moves during the click's frames: predict changing confirm bounds between target lookup and down/up, causing the saved coordinate to miss.
2. Enter leaves an intercepting focus/overlay state: predict stable geometry but different event delivery from a click-only control.
3. The rendered confirm handler is not active: predict a correct stable hit without callback delivery.
4. Suite context or scheduler seed is required: predict isolated success but pinned repetition or one fullcontext failure of this same dialog case.

None is established. Source inspection found a250ms slide-down animation in pinned gpui-component0.7.1; gpui-kit0.7.3 `Window.click` computes one center before mouse move/down/up and renders between events. Its own dialog fixture uses `set_reduce_motion(true)` and explicit completed frames. `on_window` in BRN already drains `run_until_parked`. These facts justify a geometry probe; they are not a causal witness or authority to add generic draining, sleep, double-click, handler bypass, or weaken Enter refusal. No instrumentation or structured frame/event observations were implemented before the stop gate.

## New setup failure and stop

The context run had five new Documents fixture failures: `documents_native_worker_runs_selected_helper_without_credentials` (missing imported Note, unwrap at388), `documents_useful_scope_deleted_project_refuses` (conversionUnavailable at544), and `documents_useful_scope_general_to_project`, `documents_useful_scope_project_a_to_b`, `documents_useful_scope_project_to_general` (missing actual Source observation at554). These are separate from the requested dialog failure. This run is not a passing widget-suite receipt.

The worker omitted explicit helper bindings before that run. `DocumentHelpers::discover` defaults to siblings of `current_exe`, whose parent is `target/debug/deps`. Both helper siblings are absent there; the receipt records no overrides. Existing helpers under `target/document-helpers/artifacts` and `target/debug` match prior qualified stock SHA256 `a2c223701c93399b06acc143c3190a2a908befa411f1c674fc7a596fdb545619` and BRN SHA256 `bc8296e57e18ab63f0388871763c26544a850a9d1de517f26bca3c9cf2674952`. This is a concrete setup explanation; unchanged failed cases were not rerun to establish its correction. Helpers were only read/hashed, not copied, executed or rebuilt.

Per the explicit first-new-failure stop gate, no further heavy command, source edit or fixture correction followed. The initial20minute budget was not consumed waiting at that gate. No production defect or causal fixture fix has been proved.
