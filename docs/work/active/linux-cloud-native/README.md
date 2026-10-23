# Cloud native Linux UI qualification, 22 October 2026

## Assignment and source

Owner requested moving builds to the saved cloud machine and testing the real
Linux UI independently of the Mac screen. The first milestone was local-only.
The owner subsequently authorized draft PR220, the complete applicable deferred
UI inventory, and merge after independent review and exact-head required checks.
Release, provider access and private data remain outside scope. See the
[expanded Linux evidence](deferred/README.md) for current results.

Started from saved `work` at `aebfa12b`, clean. Fetched origin and created isolated
`codex/linux-cloud-native-20261022` from verified main
`24a77f4f44072a084423f71bca0356f140dcd968`. Also fetched campaign branch
`codex/agent-first-followthrough` at `f0c1c0ba4d68c9c0d92a7922cc6ebd7a27c65f7c`;
its published record says the selected headless queue is clear and Mac checks
remain deferred. No Mac process or unpushed work was inspected. This separate
task record avoids overwriting that campaign's independently maintained state.

## Implementation and capability

Existing main already supports Linux through pinned GPUI Kit 0.7.3 and its Linux
backend. No alternative UI, dependency change or platform abstraction was needed.
The four-line Linux-only addition sets the native window title to `BRN`, allowing
reliable window-manager/automation identification without exposing note titles or
workspace paths. Mac code is unchanged. The [reproduction guide](../../../development/linux-cloud-ui.md)
documents the retained setup and separates native evidence from toolkit tests.

Environment: Debian 13.6 x86_64, Rust 1.98.1, Xvfb at `:98` with 1280×900×24,
Openbox, XTEST/xdotool and Mesa 25.0.7 llvmpipe (LLVM 19.1.7), CPU Vulkan.
No `/dev/dri` hardware GPU or pre-existing DISPLAY was available. Rootless signed
Debian dependencies were already retained. Started only task-owned local display
and window-manager processes, without TCP or access-control disabling flags.

The initial offline build failed because the old cache only had rustls 0.23.45
while current main locks 0.23.46. `cargo fetch --locked` refreshed the cache;
the lockfile and toolchain were unchanged. Current native/CLI build then passed.

## Actual native observation

The current production binary (native-ui, not the GPUI test platform) launched
with a fresh synthetic `init --purpose test` workspace under `/tmp`. The CLI was
used only for initialization and independent reads; no SQL or CLI repaired UI state.

Observed through X11 input and settled screenshots:

1. `xdotool search --onlyvisible --name '^BRN$'` found the titled native window.
2. Super+N opened New note; typed title and clicked Create note.
3. Clicked the real editor and typed the body shown in [edited.png](evidence/edited.png).
   Independent recovery reads showed the in-progress edit before Save.
4. Super+S saved version 2: [saved.png](evidence/saved.png) and
   [independent saved read](evidence/saved-notes.json).
5. Alt-F4 closed the production process with exit 0. Relaunched the same binary
   and workspace, clicked Notes then the note: [reopened.png](evidence/reopened.png).
6. [Independent reopened read](evidence/reopened-notes.json) matches the exact
   saved text at v2; [recovery is empty](evidence/reopened-recovery.json).

Exact expected body:

> This note was typed through X11 into the real Linux BRN window. Save and reopen must preserve these exact words.

Binary identities are in [binaries.sha256](evidence/binaries.sha256). Source was
main24a77f4f plus only the Linux title addition; documentation was written during
qualification. Initial immediate screenshots preceded queued input/frame
completion; retained acceptance screenshots show settled state. Software rendering
and concurrent compilation are not a latency benchmark.

## Review and gates

Independent reviewer found one reproducibility issue: immediate display readiness
checking and unbounded window search. Corrected with occupied-display refusal,
bounded readiness checks against the owned PID and a timed search. Re-review found
no blocking findings and independently inspected the saved screenshot/JSON.

Current-source native/CLI build, formatting, diff whitespace and Markdown links
pass. Exact text/version/empty-recovery assertions pass. Desktop aggregate passed
**351/0/1 ignored** in 18.28s. The ignored shared CLI/desktop useful-intake case
requires a separately prepared `BRN_DWR_SHARED_*` fixture and was not run here.
Strict all-target native-test-support Clippy passed with `-D warnings` in 41.04s.
Final Markdown check passed 627 files / 2824 local links / 0 failures.

The first aggregate attempt failed **62 passed / 289 failed / 1 ignored** because
the sandbox mounted `/var/tmp/brn-rust-cloud-fixtures` read-only. The supported
narrow `with_additional_permissions` write grant for that existing synthetic
directory succeeded. Re-running the identical command passed without changing
tests or path validation. Both [original failure](evidence/tests.log.gz) and
[passing result](evidence/tests-fixture-access.log.gz) are retained. This was a
filesystem capability failure, not a native-window failure or an approval denial.

Commands (after sourcing `/workspace/brn-environment/activate.sh`):

```bash
cargo build -p brn-desktop --features native-ui -p brn --locked --offline --jobs 1
python3 /workspace/brn-environment/run-with-reaping.py cargo test -p brn-desktop --features native-test-support --bin brn-desktop --locked --offline --jobs 1
cargo clippy -p brn-desktop --features native-test-support --all-targets --locked --offline --jobs 1 -- -D warnings
cargo fmt --all -- --check
git diff --check
python3 scripts/check-markdown-links.py
```

Raw current-run logs and intermediate captures are retained outside Git at
`/workspace/brn-environment/logs/linux-cloud-20261022`. `current-fixture-path`
there names the retained fresh synthetic workspace. Initial old-binary rendering
was only a capability probe and is not the current-source acceptance witness.
Remote main and campaign heads were rechecked after the native journey and
remained at the hashes above.

No Mac close/menu/accessibility, IME, clipboard, native file picker, hardware GPU,
Wayland, live AI, retrieval model, all-journey or owner acceptance is claimed.
Linux Super shortcuts and some Mac glyph help text remain; conventional Ctrl
application shortcut adaptation is outside this minimal working-cloud milestone.
Windows and unrelated refactors are untouched. The initial milestone was retained locally.
The application exited normally after the reopened capture. Only this task's
recorded Openbox/Xvfb PIDs were stopped. Synthetic data, tools, builds and evidence
remain available; `/workspace/brn-environment/start-linux-current.md` points to
this handoff. Subsequent authorization published draft PR220; the deferred record
above owns the expanded qualification and integration gates.
