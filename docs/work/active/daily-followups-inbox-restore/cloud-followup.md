# Post-merge cloud verification

Owner request: finish useful remaining cloud verification and provide a headless
Mac prompt for the live-model part. This does not authorize Mac execution, new
features, calculator activation, release, deployment or another merge.

Baseline: PR227 merged at `90e5bcebac1d64fc98ef315675fd2810e1e26f1e`, with the
same tree as reviewed `c6930139302681bc732bad342f187cca5815ba63`.
[Resulting-main CI](https://github.com/ewq100/brn-rust/actions/runs/40182655027)
is still being followed separately to terminal results.

The [existing qualification](qualification.md) already covers Action fields,
review/Undo/reopen/stale versions; inbox consent/private context, canonical receipt
replay, source/settings races, cancellation and restart; Restore corrupt data,
fresh destinations, cancellation, killed-process staging recovery and reopen.
Do not repeat entire suites merely to add counts.

Independent inventory selected two uncovered boundaries:

1. In the existing native test harness, click Scan and observe the real inbox
   worker, actual Stock helper, durable Imported state, reopened result and inert
   private queue. Existing native tests used direct service calls/injected finishes.
2. In the existing inbox integration suite, kill an owned child after its first
   import commits and its second conversion starts. Reopen and prove lock release,
   receipt reuse without reconverting the first file, and safe remaining work.

A bounded real Linux Action observation may additionally cover waiting-on,
distinct follow-up, invalid-date correction, clearing and cancellation. It uses
synthetic data only and the already supported window-relative input route.
No actual picker/portal retry and no unchanged live-admission workaround.

The [Mac live-model prompt](mac-live-model-prompt.md) is ready for a separately
executing Mac build agent. It is headless, uses the normal existing login, begins
with admission and one smoke, then caps representative Runs and tool rounds.
Authored fixtures/deterministic outcomes remain distinct from live semantic
correctness. Mac UI and owner acceptance are outside that prompt.

This file records selection, not completed new checks. Results and any scoped
repairs must name exact source, retained failures, commands and independent review.
Any follow-up PR remains draft pending a separate merge decision.
