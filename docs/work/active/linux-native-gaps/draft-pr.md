# Draft: Upgrade GPUI Kit and verify Linux native accessibility

Pin released GPUI Kit 0.8.0 for desktop and writing UI while retaining the
GPUI pre 0.3.9 backend. Use upstream disabled-node metadata support, verified
in the existing unavailable-skill regression. Linux enabled/sensitive translation
remains an explicit unpassed adapter limitation; no incompatible AccessKit upgrade.

Project selection rows now expose their full names and selected state as actions;
a shared dropdown bridge advertises activation through GPUI's existing Popover.
The released toolkit does not supersede these app fixes. Retain actual AT-SPI,
menu/focus, immediate Connections retirement/Undo, save/reopen and restored-copy
evidence. Native file-picker admission is blocked; no Mac-only acceptance claim.

Validation: desktop 352 passed/0 failed/1 ignored, writing UI 44+12 passed,
production native build, strict Clippy, formatting/documentation/retirement
checks; native interactions and independent source/lock review. Source-specific
0.7.3 and 0.8.0 evidence remain separate. Maintenance/Rig migration is excluded.
This description is prepared locally; publication is not authorized for this branch.
