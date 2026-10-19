# Bounded worker event draining

Source `1e89f4e86435c75bfc320d741eed84abfacb496a` changes the existing private
worker owner and its tests. No new abstraction, dependency or setting is needed.

A bounded channel limits queued slots at one instant, but the old drain loop could
continue reading while a producer refilled it. Each poll now admits at most128
events, leaving Pending when more events may remain. FIFO content is retained for
later polls; only observing a terminal/disconnect establishes that result. The
first terminal returns immediately with its exact payload. The UI then consumes
it and retires the old receiver before starting a successor, so malformed trailing
events cannot reach new work. Existing backpressure, cancellation, queue policy
and partial-result persistence retain their owners.

This establishes an event-count bound. It does not establish a wall-time limit,
measured latency improvement or reproduced native freeze.

Three deterministic prefilled-channel tests cover multiple full batches followed
by terminal, terminal exactly at the budget edge with unread tail, and a closed
channel with a remainder. The old owner failed all three meaningful assertions;
four existing protocol cases passed. An initial wrong module filter selected0 tests
and is not evidence. After the fix, protocol7/0 and full desktop320/0/1 pass,
including retained Stop/Steer, original-target partial persistence, disconnect and
Converted-to-successor journeys. Strict all-target native lint, format/whitespace
and a production native-ui-only build also pass. The existing external CLI fixture
remains ignored; no extra live/native acceptance is inferred.

[Qualification](evidence/drain-qualification.json) hashes original baseline/final
logs and the immutable current production binary.
[Commands](evidence/drain-commands.txt) include a
[reconstructed test-only baseline patch](evidence/drain-baseline-test-only.patch)
for reproducibility; it preserves the old drain and adds only the expected-budget
constant and new tests. This reconstruction is not a fresh baseline rerun.

Independent Sol high review approved actual integrated `2bf8eb08` plus its policy
records, with no actionable findings. Source diff SHA-256 against7bbd7d6e:
`92c42ad5078e75c62adda4529f9fb0c0d79eafab12f0d83ec6ea8f7414b17731`.
Hosted required checks, normal merge/resulting-main verification and applicable
Mac interaction remain separate from these local source checks.
