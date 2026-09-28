# Apple Silicon sidecar trial and distribution boundary

Reviewed 2026-09-27. This trial uses an installed executable; **no Codex binary is copied, bundled, or redistributed**.

## Current launch contract

The harness resolves `CODEX_BIN` when explicitly supplied, otherwise `codex` through PATH. A desktop process cannot assume it inherits a terminal's PATH; a future app should retain an explicitly selected absolute executable path, or resolve its own verified bundled resource. For this Mac's authorized local check:

```sh
export CODEX_BIN=/Applications/ChatGPT.app/Contents/Resources/codex
```

That path is a development observation, not a stable installation API or a redistribution permission. The binary inspected for this trial is Mach-O arm64, `codex-cli 0.155.0-alpha.16.4`. An independently installed official CLI is preferable for a standalone BRN deployment. No installer or auto-download mechanism is introduced here.

Launch `codex app-server` over private stdio, complete initialization before declaring readiness, and supervise only the child BRN launched. The harness reports missing executable, launch failure, protocol timeout and unexpected exit without forwarding server stderr or raw error payloads. Shutdown closes stdin, waits for bounded graceful exit, then kills/reaps if needed. An interrupted transport does not prove a turn was cancelled. There is no automatic retry or replacement conversation.

Keep credentials and authoritative conversation history under Codex ownership. Do not package, inspect, copy or migrate the user's Codex auth cache. BRN's trial state file contains only conversation metadata. The same Codex home/store and compatible executable must remain available when resuming.

## Documentation and terms checked

- [Official App Server documentation](https://learn.chatgpt.com/docs/app-server): supported stdio protocol, initialization, stored thread resume and managed authentication.
- [Codex installation/build instructions](https://github.com/openai/codex/blob/main/docs/install.md): standalone installation and source-build route.
- [Official App Server README](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md): local client integration and protocol details.
- [Codex LICENSE](https://github.com/openai/codex/blob/main/LICENSE) and [NOTICE](https://github.com/openai/codex/blob/main/NOTICE): the source repository is Apache-2.0; redistribution requires compliance with its license and applicable notices. The NOTICE includes Ratatui attribution. Apache-2.0 does not grant trademark rights.
- [OpenAI Terms of Use](https://openai.com/policies/terms-of-use/) and [Service Terms](https://openai.com/policies/service-terms/): service/software terms and open-source provisions are distinct. They do not establish blanket permission to redistribute the ChatGPT app or its embedded components.

A future standalone CLI bundle built from a pinned official source/release is a plausible route under the applicable open-source licenses. This is a technical trial decision, not a completed redistribution audit. Before shipping, identify the exact artifact's provenance, inventory transitive licenses/notices, include required texts and change notices, and recheck the terms applicable to that artifact. The repository's top-level license alone does not clear every component in an arbitrary binary.

## Still unverified

- A BRN `.app` bundle, sidecar resource layout, installation and updates on a clean Mac.
- Signing both app and sidecar, hardened runtime and entitlements, notarization, Gatekeeper and quarantine behavior.
- macOS App Sandbox compatibility, Keychain access under BRN's identity, login UI integration, and app lifecycle events such as crash, sleep/wake or reboot.
- Graceful process-tree cleanup after forcible termination of BRN itself; the probe supervises its directly launched child during normal/error unwinding.
- Long-term protocol compatibility. Dynamic tools are experimental; pin and revalidate the executable and generated protocol schema before product integration.

No signing credentials, release account settings, or installed applications were changed for this work.
