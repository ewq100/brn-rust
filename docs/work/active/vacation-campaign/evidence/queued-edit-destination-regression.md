# Queued edit destination regression handoff

Candidate branch: `codex/queued-edit-destination-owner`
Base: `dee96c9ed4542eb3c8b595280116628580dc896a`
Test-only source: `crates/brn-desktop/src/threads_chat_tests.rs`
Test: `editing_a_queued_message_does_not_redirect_the_next_chat_send`

The focused synthetic GPUI witness seeds one waiting A message and a paused queue, holds a local synthetic Working worker with its lease and sender alive, activates the actual `queue-edit-0` control, and types an A replacement. It navigates to B through `open_thread`, types B's independent message and clicks the actual `ask-delegate` action. Its first post-send assertion checks that A's original queued text remains unchanged, so the old source should fail at the suspected cross-destination mutation. It then checks B's queue item, returns to A, verifies the exact edit draft is restored, clicks the composer action to save that edit, and checks that A's original row is updated with B unchanged and no duplicate item. Provider dispatch remains blocked by both the paused queue and local held worker.

No build or test was run by the author. Only the pinned rustfmt was run using `/tmp/brn-launcher-focus-qualification/environment.sh`.

Safe focused command for the lead to run against this candidate:

```sh
source /tmp/brn-launcher-focus-qualification/environment.sh
cargo test -p brn-desktop --features native-ui editing_a_queued_message_does_not_redirect_the_next_chat_send -- --nocapture
```

Expected old-source red: the first queue-content assertion after B's actual composer action sees A changed from `Original A message` to `B's independent message` (queue remains one item), before the later B and return-to-A assertions.
