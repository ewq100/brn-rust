# Independent re-review — corrected bounded reference

Reviewer `read_geometry_review`; same independently briefed read-only reviewer,
requested GPT-6.1 Sol/high (configuration requested, not runtime telemetry).
Reviewed commit **`c20bda2dda16257164f7c584f0c5fc20a301075f`**.
Verified toolkit patch SHA-256:
**`de6dcb320028f4d1848a44c3b2141fc87cd8edd7d23adb930500e4f40839b6d4`**.

## Returned result

**No remaining actionable findings in the bounded corrected reference. All five
original findings are resolved.**

| Original finding | Re-review disposition |
|---|---|
| Comment index could bind another record's body to old geometry | Fixed. `SelectedComment` captures stable synthetic ID, complete payload, note/version/source and rendered snapshot. Reordering preserves identity; missing, duplicate, addressed or changed records invalidate it. |
| Clearing presentation left an earlier reveal pending | Fixed. `clear_reveal` cancels the pending request; anchor clearing and replacement selection call it. Identical-source navigation and rejected replacement have regression coverage. |
| Large async change/revert could revive a probe | Fixed. `increment_update` clears the probe synchronously before mutation revision/parse handling, independently of committed snapshot equality. |
| Clean Read/Write/Read restored an old anchor | Fixed. Toggle and render guards clear binding and derived state; returning requires selection again. |
| Newline-start passage missed its first covered glyph | Fixed. Probe construction skips CR/LF only within resolved covered pieces. Newline-plus-text measures `w`; newline-only cannot borrow it. |

The reviewer inspected corrected host, toolkit patch and regression tests.
The tests exercise the actual failure windows, including mutation and paint
before background parse completion.

The reviewer independently reran, with the requested installed pinned toolchain
and `/private/tmp/brn-passage-geometry-target`:

```text
cargo test --locked --manifest-path experiments/passage-comment-geometry/Cargo.toml --test geometry
```

**21 passed, 0 failed, 0 ignored; exit 0.** The worktree remained clean.
The reviewer inspected before-fix failures, corrected build/lint/format evidence
and the recorded 150-test unchanged-production result, without rerunning the
production suite. No GUI, provider/model calls, installs or helper delegation.

## Returned verdicts and remaining gates

**Toolkit:** clean for this bounded experimental reference and further upstream
API review. Passive reporting, frame clearing, clipping, snapshot checks and
explicit-reveal priority remain consistent with the ordinary paint path.

**Proposal:** binding, invalidation, pending-reveal cancellation and single-frame
consumption are adequately specified. Production must enforce reveal ownership
at the shared TextView boundary: `clear_reveal` cancels unconditionally, and
existing BRN outline navigation uses this viewer's reveal path too. The actual
candidate must test supersession between outline and comment requests.

**Not production integration approval.** No assigned C, approved integration
ref, released ownership or supported production toolkit source exists. Actual
ThreadsUi geometry coupling, clean-session/recovery/Save/Apply/Undo checks and
independent review remain required after implementation. Native fonts/Retina,
cached ancestors, outer scrollers, rapid navigation, focus/accessibility and
preserved reference/proposal behavior remain acceptance limits. The locked Mac
was not used.
