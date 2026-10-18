**Verdict: APPROVED.** I found no production defects. The one Low issue is test and evidence precision only.

I reviewed the actual `5e98..601` diff (tree `d7ce499f`), the code it calls, and the specs. I ran no builds or tests.

**Requirement check**
- **Shared start:** `WritingPanel::rewrite_run_start` (`native.rs:159-196`) uses the existing context reader (`writing_request_in_context`), keeps the old stale-input check, and builds the same `ThreadRunStart` as before. It sets no queue claim.
- **Old route removed:** `retry_rewrite` (`threads_ui.rs:1794-1832`) now goes through `begin_run_start` and then `start_request`. `start_run` no longer checks for `RewriteNote`. Ordinary chat queue and EditNote handling, the busy-then-refresh order, the error and completion messages, and the event, cancel and worker code are unchanged.
- **Fallback:** the first-linked fallback was already gone before this change. `writing_threads` is filled only from the checked context; I confirmed this at lines 1153-1163, 1604-1610 and 1656.
- **Chat Continue:** this also calls `retry_rewrite`, but the recovery card is hidden for that path (`threads_writing.rs:353`). Requiring the matching Note there is not a reachable regression.
- **Boundaries:** `brn-writing-ui` gains no `brn-ai` dependency. The re-export is one line. The dev-dependencies match `brn-threads-app`'s existing ones, and resolver 2 keeps their test features out of production builds. The lock change adds exactly one edge (desktop → rig).
- **Tests:** both fixtures are appended after a byte-identical prefix. Fixture 1 drives real clicks, the shared preparer and the scripted runtime, and checks the inputs the model actually received.

**Low: test/evidence precision**
- **Where:** `threads_writing_tests.rs:1765-1790` (the archive leg of fixture 2), plus the "archived Unarchive" row in `verification.md`.
- **Evidence:** the context reader still accepts an archived conversation. `active_writing_context_from` only checks that the record is a thread, and `writing_setup_scope` includes archived chats. So this leg most likely stops at the `offline-fixture` provider-selection refusal, and it asserts only "no worker, no writes".
- **Consequence:** archived-context refusal and Unarchive guidance on retry are not shown. Behaviour is unchanged from base, which is why this is not a blocker.
- **Fix:** assert the actual notice for this leg, or drop "archived context" and "Unarchive" from the fixture name and the evidence wording.

**Preferences (non-blocking)**
- The new context-refusal message (`self.fail`) could say that feedback is kept, as the stale-input message does.
- After the second "retry-rewrite" click, the fixture can't tell whether the retry actually ran. The notice text would be the same either way.

**Still open (not blocking source approval)**
- The targeted recheck of the changed context reader was not run.
- The carried PDF (pdftoppm) failure.
- Hosted Linux/macOS F22, native-human, owner and F21 program gates.
- I did not independently re-check the 17 receipt or fingerprint hashes.
