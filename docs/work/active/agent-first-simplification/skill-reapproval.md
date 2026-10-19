# Explicit skill version reapproval

M2 changes the packaged maintain-notes bytes. A saved old exact digest correctly
becomes unavailable, but the existing desktop also disabled Inspect and offered
no explicit way to accept current validated bytes. The CLI already supported this
through inspect/current-digest enablement. This follow-up closes the desktop gap
without automatic rebinding or changing shared authority.

The scoped view retains the validated current package inventory and shows approved
and current identities separately. Inspect binds the rendered current digest;
Enable current version explicitly rechecks digest and project scope through the
existing service, then leaves the enabled skill unselected. Refresh does not write
policy. Missing/corrupt packages remain unavailable, and inventory errors are shown.
Selecting the newly enabled skill remains a separate explicit owner action.

Candidate5cf8 (two desktop files) is independently source-approved; the focused
synthetic stale-binding journey passed, with strict native all-target lint/pinned
format. The first actual compile failure and alias/GPUI/test visibility corrections
remain in [qualification](../vacation-campaign/evidence/skill-reapproval-qualification.json).
The fixture uses checked owner policy writes and the real current builtin digest;
it is not a historical executable or native observation. No unchanged-old-source
semantic execution is claimed. Root19efe contains the exact approved source;
combined desktop327/runtime505/CLI15/app journeys8, strict native lint/format and production build passed; see the [combined receipt](../vacation-campaign/evidence/agent-first-combined-qualification.json). Normal hosted/main gates remain. Actual Mac/owner acceptance remains separate.
