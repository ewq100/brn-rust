# Accessibility navigation qualification

The source adds opt-in named, focusable chat rows, contextual follow-up groups and task controls, and focused Capture footer-button Enter handling. Deterministic checks pass. Actual Linux observation confirms named controls and groups, **but native keyboard selection and Capture navigation remain unqualified**. This is not Mac or owner acceptance. Current batch routing: [selected work record](record.json).

## Source and scope

Test-only red commit `066086fd` precedes implementation/final tests at `20a028b889934189a00d5f7fe2b9cfe0fb9f7da8`. These commits were developed on reviewed rehearsal `f19538c`; integration must transplant only the new commits onto reconciled main, not publish rehearsal ancestry.

Chat rows opt into maintained GPUI Button role, tab order and focus-visible styling; static rows remain unchanged. Task group and text semantics add no tab stops; task controls retain their existing checked callbacks. Capture handles Dialog Confirm only on each footer button's ancestry, sharing the checked pointer/keyboard callback and one request identity. Body Enter cannot approve; snapshot, busy-worker and scope guards remain in force.

## Deterministic checks and overlap

Commands used `--locked --offline`, `-p brn-desktop`, and `--features native-test-support` with the saved native environment and an explicitly created synthetic TMPDIR.

| Check | Result | Scope |
| --- | --- | --- |
| `cargo test … threads_ui::pages_tests` | 23 passed | Includes the chat Enter/Space test and contextual follow-up test below. |
| `cargo test … threads_ui::capture::tests` | 3 passed | Body Enter inert; Cancel Enter/Escape restore trigger focus; focused Propose Enter invokes stale-destination error without request/Run. |
| Focused chat and follow-up tests | 1 passed each | Earlier targeted runs; **not additional unique tests** beyond the 23 pages tests. |
| `cargo clippy … --all-targets -- -D warnings` | Passed | Before the final test-only expected-label correction. |
| `cargo build …` | Passed | Copied executable before releasing the shared build slot. |

The final page suite ran after correcting an older generic-label assertion; production source was unchanged. Capture tests, lint and binary were not redundantly rerun for that string-only test correction. No full native suite or live provider run is claimed here.

Preserved failures distinguish fixture problems from product evidence: missing TMPDIR caused initial setup failures; valid fixtures then showed absent chat Button/group roles. An initial 80-Tab Capture fixture started inside the multiline editor; it was corrected to use production panel navigation. A fixed-count chat Tab sequence was replaced with bounded traversal to a known preceding control. Focused Cancel Enter then exposed Dialog's ancestor Confirm interception, corrected by the scoped footer handlers. The broader pages run initially had 22 passes/1 obsolete generic-label failure before the final 23/0 result. Raw logs remain under `/workspace/brn-next-tracks-evidence/accessibility`.

## Actual Linux observations

Both runs used disposable synthetic fixtures, no pointer fallback, no picker/portal, no Propose/Send/Continue, and no model dispatch. The source-matched desktop copy has SHA256 `7fcfc074cbc84b92d21c8c269039536d7573214fd2196bbcf28f0c09058fd233`.

CLI and fixture were reused from qualified source `ef09669eb0e688e13325718b0ca6ede741c9cb95`; integration verified their production paths unchanged through the rehearsal baseline. Their hashes were checked before use: CLI `4552b8a922cb7b599a45285b7e836002bda9452ca8d7079ba416711d5c47a399`; fixture `4c28aeb6ee4ac47f6e33c2570a3e569e761e88371cd08a352ded220612af4dbc`. This is mixed-source binary reuse, not a new build claim for those tools.

1. `outcomes.n1KBwe`: the complete 69-node Tasks AT tree exposes the named chat Button, all five follow-up groups and contextual task controls. See the [bounded exact-field receipt](evidence/accessibility/named-controls.json). Keyboard traversal began in Tasks, entered the editor Tab domain and refused after its bounded 80 steps.
2. `outcomes.3VYER1`: the follow-up route activated Recent navigation and traversed by keyboard. The AT probe reported chat focus, but [the subsequent screenshot](evidence/accessibility/second-enter-opened-panel-menu.png) shows Enter opened the Tasks panel menu. The chat was not selected and Capture did not become available; the bounded sequence refused. No third attempt was made.

The initial suggestion of AT focus lag is **unsupported and is not the diagnosis**. The observed mismatch requires investigation of GPUI popup/focus dispatch and the observation sequence. Passing test-platform activation does not resolve the native mismatch.

After both sessions exited, the qualified CLI reopened each fixture and proved unchanged records, history and candidates, with zero Runs: [first receipt](evidence/accessibility/first-invariance.json), [second receipt](evidence/accessibility/second-invariance.json). Ten static Label nodes had empty Accessible.Name; the probe did not query their Text interface, so description/fact text accessibility is not established by that dump alone.

## Remaining acceptance

Diagnose the popup/focus dispatch mismatch before another bounded keyboard qualification. Demonstrate native chat selection and Capture Cancel/Escape with reliable observed focus, no pointer fallback and unchanged canonical state. Qualify static text through the appropriate accessibility interface. Actual Mac review, native focus/accessibility acceptance and owner acceptance remain separate, unrun gates.

## Integration and discriminator, 28 October 2026

Only the four slice commits were transplanted onto actual main `d2f03975`;
the resulting production tree matches the qualified lane exactly. The additional
[test-only dispatch discriminator](evidence/accessibility/dispatch-discriminator.md)
passed both variants but did not reproduce the native mismatch. Its test adapter
was inactive and its key helpers render frames; it does not close native keyboard,
static-text or Mac acceptance gates. No speculative production change followed.
