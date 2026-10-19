# Export folder-picker error feedback

Source `980358889a231b497c1a409113e079d890b4b8d3`, integrated at `945a0a7a`,
reports folder-picker receiver/platform errors in the existing export status.
Previously the callback accepted only a successful path and silently ignored errors.
Cancellation and an empty path list remain quiet; successful selection still uses
`output` and its existing busy check. The small result handler follows the existing
backup-picker pattern. Backup preferences, destination naming and service policy
are unchanged; no general picker framework is introduced.

One new journey uses the actual export action and supported picker simulation for
cancel and successful retry. Error injection enters the same completion handler
because the pinned toolkit cannot simulate transport errors. It checks the explicit
status, absence of worker/output on error/cancel, available retry action, a fresh
snapshot directory on retry and refusal when a different worker has started.
This establishes handler feedback and action wiring, not actual OS error delivery
or an independently inspected rendered error label.

The focused new case passed1; the related backup family passed6. Temporarily
suppressing only the candidate error-status/notify branch failed at the intended
assertion. Original production bytes were restored exactly (SHA256 in the receipt).
This is a candidate mutation, not a failing run against unchanged old production;
the exact mutant source digest was not recorded. Final desktop322/0/1, strict
all-target native lint, formatting and production-only native-ui build passed.
The earlier full-suite pass and original mutant failure are retained.

[Commands](evidence/export-picker-commands.txt) and the
[qualification receipt](evidence/export-picker-qualification.json) retain source,
logs/hashes and immutable artifact identity. At integrated945, the only additional
crate delta is the separate test-only provider fixture helper consolidation;
that module is cfg(test) and does not enter the desktop dependency/production build.
Its own provider-fixture gates remain separate. No OS/private/live-provider check
or owner acceptance is claimed. Independent Sol approved complete945 without
actionable findings; separate provider-helper checks, evidence reconciliation and
hosted/main integration remain gates.
