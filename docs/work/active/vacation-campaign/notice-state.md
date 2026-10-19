# Notice state ownership

Source: `36bf2718c22f7838a3e82963cec7e01a53c32f52`, integrated with reviewed
continuation/status documentation at `9b2ad729`. Crate bytes match exactly.

## Result and boundary

The private native `NoticeState` owns notice text, existing Tone, remembered work
completion and whether the current presentation is that work status. Ordinary
notices retain the latest completion; completing work remembers and displays one
payload atomically. Clear hides only the current presentation. Restore requires
no active worker, an empty presentation and remembered work, so it cannot replace
a newer notice or restart work. Equal text in a later ordinary notice does not
establish work identity or inherit the work's severity.

This replaces independent text/tone/guard/last-work fields and duplicate shell
restoration rules. Existing setter routes, labels, worker policy and persistence
authority stay at their existing boundaries. There is no new framework, crate or
saved state. This is an ownership improvement with a specified provenance rule;
no old production bug reproduction or performance improvement is claimed.

## Retained guarantees and retirement

Only the sixth arm of the former
`outcome_tones_and_a_direct_overwrite_never_inherit_an_older_tone` test is retired:
it directly overwrote the old String to exercise a string-guard fallback. Private
typed ownership removes that unsupported write and the fallback together.
`outcome_notices_keep_their_expected_tones` retains all five actual outcome cases:
Stale, Applied needing refresh, Applied, NeedsReview and failure.

Retained GPUI tests exercise repeated actual Dismiss/Layout restoration, recorded
Run/terminal/error severity, newer error and active-worker refusal, replacement-run
clear and backup presentation. Their fixtures now use supported owner operations
and immutable text reads. The newer-status test also checks an ordinary notice
with identical completion text. Two focused owner sequence tests cover memory,
replacement, repeated clear/restore, absence/busy refusal and explicit provenance.
No other behavioral assertion or test was retired.

## Qualification

Local macOS27.0.1 arm64, pinned Rust1.98.1 and locked dependencies. The compact
build uses one job, no incremental output and no dev/test debug information.
Both current stock/adapted helpers were built from their separate literal locked
graphs and declared by absolute paths for the final suite. Exact commands, source,
environment, helper and log hashes belong to the qualification receipt.

[Qualification receipt](evidence/notice-qualification.json),
[exact commands](evidence/notice-commands.txt) and
[worker results](evidence/notice-results.txt) retain the local evidence. The lead
checked raw final results, exact crate identity, immutable binary and both helper
hashes; the receipt hashes the compressed logs and their original bytes.

| Check | Actual result on source36bf2718 |
|---|---|
| Focused owner sequences | 2 passed /0 failed |
| Seven retained focused GPUI cases | Each1 passed /0 failed |
| Full desktop native state | 317 passed /0 failed /1 ignored,6.26s test bodies |
| Strict all-target native Clippy | Passed,2m34s |
| Final format and whitespace | Passed |
| Production native-ui build without native-test-support | Passed,2m43s; binary receipt is separate from OS interaction |

The ignored existing CLI/desktop reuse case requires explicit `BRN_DWR_SHARED_*`
external CLI fixture values. It was not run for this slice and is not a passing
gate. The carried dependency future-incompatibility warning for `block0.1.6` is
retained; no dependency/toolchain upgrade was introduced.

Independent Sol high review approved actual integrated `9b2ad729` with no actionable
findings. Source diff SHA-256 against37c87985:
`fe6c342e57cb65058a83544bad4dc6250e6e8c078a74279518e89eac2ea48241`.
Hosted required checks, normal merge and resulting-main verification remain separate.
[MAC-03](native-queue.md) queues real Mac Dismiss/Layout/backup interaction after
candidate gates; OS input, IME, accessibility, subjective acceptance and release
are not established by the state tests or production build.
