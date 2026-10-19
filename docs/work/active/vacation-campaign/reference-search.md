# Retire a hidden scoped-reference search

Source `a9600e641c18598e52340df5366918a1df7ce242`, integrated8431e70b,
clears finished-search pending state for its exact captured setup even when that
draft is hidden by navigation. Page publication separately requires the current
displayed context. Generation, captured target/token, replacement queries and
checked service scope still fence stale results. No new state framework or
canonical policy is introduced.

The existing delayed-result test now starts a real UI search, observes its owned
cancel ticket, navigates without replacing the draft, waits for real callback
retirement, and returns to the unchanged original Note. It checks no stale
matches/counts, no stuck pending indicator, and the actual retry action. Existing
replaced-form and stale-canonical cases remain; an actual replacement query stays
pending when offered an older-generation reply. No test-only counter remains.

The counterless witness compiled and failed at the intended pending assertion
against byte-exact old UI source. A definitive replay then used the exact retained
test patch and committed test file, failed at the same assertion, restored the
frozen production bytes exactly and passed focused1/0. The patch SHA and resulting
test-file SHA name different objects; the [qualification receipt](evidence/reference-search-qualification.json)
and [definitive replay record](evidence/reference-search-v3-evidence-receipt.txt)
label both explicitly. No extra test instrumentation remains.
Focused1/0, related16/0, full desktop322/0/1, strict native lint/format and a
production-only build after source freeze passed. Independent Sol approved source843
without findings. Lead crate/manifests/lock/toolchain match frozen workera960.

[Final commands](evidence/reference-search-final-commands.txt) and
[worker results](evidence/reference-search-final-results.txt) retain gate details;
compressed patches/logs preserve raw hashes. Earlier instrumented iteration,
harness compile failure and misleading pre-freeze artifact name remain historical,
with an explicit artifact note. Only the a960-named immutable binary qualifies the
final source. Stored patches use lossless gzip to preserve their context bytes
without treating those bytes as repository source whitespace.

No OS input/IME/accessibility, owner acceptance, live provider or release is claimed.
Evidence reconciliation, hosted exact-head and resulting-main checks remain gates.
