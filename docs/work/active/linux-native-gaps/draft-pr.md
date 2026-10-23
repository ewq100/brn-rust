# Draft: Expose native Project and menu actions to accessibility

Project selection rows were absent from the Linux accessibility tree, and
menu triggers did not advertise activation. Give Project rows their existing
full labels/selection with an actionable role and bridge dropdown activation to
GPUI's existing Popover behavior. Preserve shared non-actionable row semantics.

Retain timestamped native Connections retirement/Undo evidence, offline document
conversion and backup/restore integrity results, and native recovered-copy tests.
Document the portal admission blocker and the two upstream disabled-state layers
without claiming those requirements passed or replacing Mac-only acceptance.

Validation: native desktop352passed/0failed/1ignored, production native build,
strict Clippy, formatting/documentation/retirement checks; actual AT-SPI Project
and menu journeys. Independent source review has no unresolved blocking finding.
No dependency updates. This description is prepared locally; publication is not
yet authorized for this branch.
