# Migrate the BRN runtime to Rig 0.45

Rig 0.45 changes agent, message ownership, authentication and MCP interfaces.
This migration updates those interfaces while preserving BRN's stored records,
structured summary provenance, credential filenames and run approval boundaries.
It uses the released Rig family and the required rmcp 3.5 family, with no fork.

Credential compatibility is implemented through Rig's TokenStore interface under
BRN's existing locks and finalizers; no new credential access, persistent lockfile
or data migration is introduced. Selected Codex tokens stay read-only. MCP keeps
its session/replay and single-invocation restrictions despite new continuation
protocol support. Existing regression assertions are adapted to shared messages
and the new registered-tool interface.

This branch is stacked on the separate Tokio/Schemars maintenance commit
`8f7143f34191ad53e4c7e74f7c061e8dce5e6e35`, which is stacked on UI commit
`125f8ef053895e9dac1a54ecc3e8243188ccbc6c`. Publication is not authorized yet.

Validation: provider/capability 505 passed; native state 352 passed; final-source
workspace 1,313 passed (exact tested head c16b986a); final strict lint/build and 108-step/1,266-check CLI journey
passed. The final real native app reopened the 0.44 synthetic store unchanged,
saved Unicode version 3, quit/reopened with exact bytes and exercised menus.
Independent source review has no blocking findings. See the task record for source
bindings, ignored tests, initial failures/corrections and evidence. Offline provider fixtures are not live-model validation;
Linux native checks do not establish macOS-specific acceptance. The existing
Linux AT-SPI enabled/sensitive caveat and blocked native file-picker journey are
not claimed fixed by this runtime migration.
