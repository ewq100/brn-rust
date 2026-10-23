# Align Tokio patches and update supported schema derives

Local draft only; publication has not been authorized. This branch stacks on
`codex/linux-native-gaps-20261023` at `125f8ef0`; review the maintenance delta
against that commit, not the combined UI changes against main.

Align the root and both isolated document-helper lock graphs on Tokio 1.53.3,
and update the direct Schemars pin and matching derive to 1.2.3. Existing root
Tokio ranges and Xberg's compatible caret requirement stay intact. No source
patch, runtime policy change, Rig migration, GPUI update or unrelated dependency
churn is included in this delta.

Validation is recorded in [the task receipt](README.md). The Linux accessibility
translation caveat and blocked native file-picker acceptance remain inherited
limitations from the UI candidate; this maintenance does not claim to resolve
them or replace Mac acceptance. No live-model qualification was attempted.
