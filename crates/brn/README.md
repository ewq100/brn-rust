# BRN Threads CLI

The CLI uses the same checked SQLite service as the desktop. Supply a fresh absolute data directory; other BRN formats refuse before opening. See `brn --help` for the exact commands.

```sh
brn --data-dir /absolute/fresh/threads init
brn --data-dir /absolute/fresh/threads note-create --title Harbor --text 'Working context'
brn --data-dir /absolute/fresh/threads thread-start --title 'Harbor handoff'
```

Save uses persistent session/base/generation checks. Review apply and Undo are concrete owner actions; neither bypasses expected versions or open edit guards. Intake offers useful information or a protected full readable note. Raw originals remain external. Export and backup require fresh absolute destinations.

The selected provider, model, reasoning effort and credential location are explicit Settings. Codex uses the normal selected subscription account in memory. There is no paid fallback or external message delivery.

Implementation: [entry point](src/main.rs), [shared app](../brn-threads-app/src/lib.rs), [checked core](../brn-threads-core/README.md). [Threads acceptance evidence](../../docs/work/active/threads-rebuild/evidence.md) distinguishes code, offline/live checks, native interaction and owner acceptance.
