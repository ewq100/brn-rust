# BRN Threads desktop

Launch the native build with a fresh explicit absolute data directory:

```sh
cargo run -p brn-desktop --features native-ui -- --data-dir /absolute/fresh/threads
```

Home shows open threads and one Needs You row per thread with attention. Notes support rendered Markdown, headings, current search, selected-passage comments, protected-note review, and explicit Save. Recovery buffers survive restart; delayed generations cannot overwrite typing or reopen a closed edit. Threads show conversation, linked work, current note context and independently tracked Actions. History exposes actual receipts and checked Undo.

Intake offers useful information or a protected full readable note. DOCX and EML use the sibling `brn-intake-helper`; PDF uses configured Poppler `pdftotext` and `pdfimages`. Markdown/text remain verbatim. Import gaps are visible; originals stay external. Managed figures render only retained SQLite asset bytes. Export and backup publish to fresh destinations.

Provider/model/effort and maintenance controls are in Settings. A changed configuration or resolved thread fences working runs; Continue invokes the provider fresh using BRN-owned state.

Implementation: [native views](src/threads_ui.rs), [state rules/tests](src/threads_state.rs), [shared app](../brn-threads-app/src/lib.rs). Compilation and state tests do not prove typing, focus, dialogs, IME or selection behavior. Follow the single pending Mac interaction task in [evidence](../../docs/work/active/threads-rebuild/evidence.md).
