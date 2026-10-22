# Precise edit trace implementation handoff

- Current candidate: commit `777e346150ab718756002bb868d3bd6ff723ed3e` on `codex/precise-edit-trace`, based on `2ecf634d7de93fbf7381317168109b6d199531e4`. This supersedes earlier candidate `0903b0591bab8ad0b11829d9edd9cda583b32881`.
- Changed source: `crates/brn-ai/src/threads_trace.rs` only.
- Source SHA-256: `6a1435bba00af9456477a242e2ac466810edc84b96e3e0608d864ad142b361b7`.
- Implementation: `prepare_note_edit` arguments project ID/version/start/end and UTF-8 byte counts for expected/replacement text. The ID is projected only when it is a string of at most 256 bytes; numeric fields are projected only as unsigned integers. Invalid shapes become null, never nested payloads. Results project successful prepared operation IDs, or `Invalid` and at most 256 Unicode scalar values from the host reason. No request key, successful-preparation reason, expected/replacement content, request hash or prepared write data is projected. Summary and Full use the same privacy boundary.
- Rejection-source inspection: all four `threads_note_edit.rs::invalid` callsites use static host strings; no model request text is interpolated. No range diagnosis is claimed.
- Added existing-seam unit cases for literal Summary/Full argument projections, Unicode byte counts, malformed object/array argument values, successful operation-only result, Invalid reason projection, and Unicode-scalar reason truncation.
- The final commit includes the lead's formatter-only changes in the same task-owned source file, preserved and committed with this bounded improvement.
- Checks run: `git diff --check` passed before commit. Builds, tests and formatting were not run, as required by the bounded assignment. No account, provider, process, network or native activity occurred.
- Limits: implementation candidate only; independent review and lead-owned builds/tests remain pending. This trace change does not identify the underlying cause of the prior Invalid range outcome.
