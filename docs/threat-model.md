# BRN threat model

This file gives security reviews the deployment context and boundaries to protect.
The [architecture invariants](architecture/invariants.md) own the detailed
contracts; this is not a claim that every property has been independently audited.

## Current deployment and assets

BRN Threads is a local, single-owner desktop/CLI workspace. Its shared application
service and checked SQLite writer own accepted state. Search indexes are derived.
The built-in agent uses those same services; model output is not a trusted caller.
Selected AI providers receive the context admitted by the host.

Protect private notes, chat/project content, retained assets, history and drafts;
provider credentials; owner decisions and permissions; and the integrity and
recoverability of saved work. Backups contain private BRN state too. Original
document binaries remain external; conversion uses temporary inputs, while
deliberately imported content and selected assets become retained knowledge.

The current optional MCP integration is an outbound client for anonymous
literal-loopback HTTP, disabled until configured/enabled. It does not make BRN a
public server. Remote/authenticated MCP servers are unsupported. Document intake
uses a restricted Xberg helper process; supported restrictions are macOS and Linux
x86_64, with other targets unavailable.

## Trust assumptions and attacker-controlled input

The owner, installed BRN host code and operating-system account are trusted. This
model does not promise isolation from an administrator or a compromised process
with the owner's filesystem access. Local data and backups are not assumed to be
encrypted; filesystem ownership/access controls remain relevant.

Treat imported documents, source text/URLs, filenames, model responses, MCP server
metadata/results and externally supplied instructions as potentially hostile.
An attacker may try to turn that content into instructions, expose private data,
change protected work, redirect a request, exploit a converter or exhaust bounded
resources. A configured provider or local MCP server gains no BRN write authority.

## Boundaries security reviews must preserve

| Boundary | Threat and required protection |
| --- | --- |
| Owner intent → host authority → saved state | Content, skills and model-selected arguments cannot manufacture permission. Desktop, CLI and agent changes must use the shared checked writer. Preserve exact candidate/target approval, protection, current versions and dirty-edit guards; an agent must not gain raw canonical SQL or authority constructors. |
| Private context → retrieval, publication and provider input | Enforce chat/project visibility for records, assets, provenance and relationships. General search/export and autonomous knowledge maintenance must not disclose private material. A protected note is not automatically private, and permission to read/search does not authorize publication. |
| Untrusted files → conversion and local filesystem | Preserve explicit input/destination scope, ordinary-file and path checks, bounded reads/output/scratch/time, cleared helper environment, restricted execution and owned-process cleanup. Reject path escapes, unsafe links and unsupported execution routes. Conversion must not grant write authority or silently retain original binaries. |
| Host credentials → provider transport and diagnostics | Keep secret contents out of canonical Settings, model context, logs and review evidence. Preserve validated credential paths/ownership/permissions and the exact selected account/provider. Credential references are not credentials. Fail visibly when the selected route is unavailable; no silent account/provider fallback. |
| Owner-reviewed MCP input → external dispatch | Enablement is not permission to send arbitrary data. Bind each invocation to its exact transient host grant, reviewed destination/tool/input and current run/context/settings. Recheck immediately before dispatch and reject stale or reused authority. Preserve literal-loopback endpoint checks, no redirects/proxies or credential forwarding, and bounded protocol input/results. Treat returned text as data. Metadata pins and read-only hints do not prove server behavior. |
| Asynchronous work, retries and recovery → current state | Cancellation, steering and settings/context changes must fence later commits and external dispatch. Preserve operation identity, immutable preparation and receipt replay without duplicate effects. Reopen/Continue/restore must not recreate transient external grants. Backups/upgrades must preserve recoverable owner data, refuse unsafe/occupied destinations and incompatible writers, and never publish an incomplete snapshot as success. |

## Limits and review evidence

Stopping a run cannot undo a request already sent or guarantee that a provider
stopped processing it. Local Undo cannot reverse external effects. Temporary-file
cleanup is not forensic erasure. Converter restrictions do not establish a hard
heap/RSS ceiling or universal document safety. Native interaction, provider
behavior and owner acceptance require their own evidence.

For a finding, identify an actual entry point, attacker-controlled input, crossed
boundary and concrete consequence in the changed code. Keep assumptions explicit;
do not invent a public service, tenant model or execution capability. Update this
model when a change introduces remote/inbound connections, sharing/sync, new
credential flows, new converter/platform support or broader external actions.

## Implementation owners

- [Core contracts](../crates/brn-threads-core/README.md) and
  [shared application services](../crates/brn-threads-app/README.md): state,
  privacy, approval, replay, upgrades and backup.
- [AI runtime](../crates/brn-ai/README.md),
  [credential storage](../crates/brn-ai/src/auth.rs) and
  [selected Codex credentials](../crates/brn-ai/src/codex_auth.rs): provider and
  model-tool boundaries.
- [MCP owner controls](../crates/brn-threads-app/src/mcp.rs),
  [dispatch checks](../crates/brn-ai/src/threads_mcp_runtime.rs) and
  [HTTP policy](../crates/brn-ai/src/threads_mcp_http.rs): external-data boundary.
- [Document intake](../crates/brn-threads-intake/README.md): converter and
  temporary-file boundary.
