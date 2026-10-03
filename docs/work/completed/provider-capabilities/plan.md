# Narrow provider capability spikes — roadmap Stage 3

Baseline: clean `main@a5ec4aecd1f536ae66d04cb946d6381f4e174323`, 2026-10-03.
Stages 1/2 are locally integrated and automated verified; native owner acceptance
remains pending. The mission permits offline preparation; actual account/provider
calls require separate owner authorization.

Establish exact selected-model routing, effort, retry, native web/source attribution
and image input through the pinned Rig 0.43.0 subscription routes. Keep the six
production crates and protected authentication; no new provider fallback, workflow
framework or durable-write tool. Use feature-gated narrow probe code and an example
entry point with synthetic inputs. Do not inspect/copy original credentials/data.

Offline acceptance: actual Rig serializers/stream parsers for ChatGPT Responses,
Copilot Chat and Copilot Codex Responses preserve selected models, route-specific
effort, PNG bytes and vision header; hosted web/citations are projected without raw
provider errors/secrets. Bound tool rounds and cancellation; no application retry.
Record transport's separate safe HTTP/2 ProtocolNacks policy instead of claiming
synthetic mocks establish zero wire retries. Validate malformed/partial responses,
compile/test/Clippy, independent read-only review and reproducible probe commands.

Live acceptance (separate scope authorized below): fresh task credential directory, explicit
provider/models, bounded short synthetic probes: low/high effort + read tool and
one image per route; native web on Responses routes with observed hosted call and
verifiable source citation. Maximum eleven logical probes; account/model discovery
and refresh only as needed. Stop on quota/model refusal, without changing selection
or route. Test failure/retry handling offline rather than provoking live quotas.
Record unsupported/unverified separately from supported. Sign-in needs the owner.

Current evidence: ordinary text/read tools and no application retry are synthetic
qualified. Historical Copilot streams/tools/restart/local cancellation succeeded;
ChatGPT completed answers were quota-blocked. Current selection has no effort or
web/image inputs, and text collection discards provider citation metadata. These
facts do not establish actual endpoint capabilities. Later implementation follows
the frozen roadmap once this dependency is qualified as permitted.

## Probe and permission scope

The opt-in `brn-ai` example uses existing protected `Auth` and exact selected
models. Build with `cargo build -p brn-ai --example provider-capabilities
--features capability-spike --locked --offline`. Invocations require explicit
`--live PROVIDER ABSOLUTE_CREDENTIALS_DIR EXACT_MODEL KIND`, with `KIND` one of
`low`, `high`, `image`, `web`. The example cancels and awaits on SIGINT or at 120 seconds.
Low/high must read synthetic `probe.md` once and return `orchard-827`; image
should identify upper red/lower blue. Completion alone does not qualify answers.
Web qualification requires an observed completed hosted search, an official
SQLite citation and independently checked version/date. No web tool is exposed
on the pinned Copilot Chat route.

Proposed live scope, requiring owner authorization: fresh current-user-owned
task folder outside Git; explicit human Connect for ChatGPT and Copilot; model
discovery/refresh for those accounts; ChatGPT `gpt-5.5`, Copilot `gpt-5.5`
Chat and `gpt-5.3-codex` Responses, if available. Three low/high/image probes
per route plus web on the two Responses routes gives at most eleven logical
probes and twenty-two completion requests. Authentication/discovery are separate
requests. Do not substitute unavailable models; stop the affected route on quota
or model refusal. No purchase, model download, existing credential inspection,
private vault or durable knowledge write is included. Sign-in requires the owner.

The pinned production transport is `rig-reqwest::shared()` → reqwest 0.13.5
default policy. Source inspection and the locked feature graph establish HTTP/2
enabled and at most two extra sends for remote GOAWAY `NO_ERROR` or
`REFUSED_STREAM`. There is no application retry, HTTP-status retry, partial-answer
retry or provider/model/account fallback. Scripted tests intercept at the Rig
transport seam and do not exercise wire retries or establish upstream cancellation.

## Offline results — 2026-10-03

Implemented the feature-gated module, synthetic PNG and explicit example against
the baseline. All three actual Rig serializers/parsers preserve the selected
model and route-specific low/high effort across read-tool continuation. Exactly
one synthetic read executes; invalid, repeated, parallel and missing calls fail
visibly. Image bytes/MIME and both Copilot vision headers are checked. Responses
streams and terminal-only snapshots project completed hosted web evidence and
safe deduplicated native citations without appending final text again.

One independent read-only review found two valid defects: cancellation hid
protected credential-finalization failures, and terminal-only hosted calls lost
their observed-search fact. Both were reproduced with failing regressions,
corrected and re-reviewed with no remaining actionable findings. Specific Auth
failures now survive cancellation; only cancelled `Other` maps to interruption.
Final metadata is inspected only for the bounded hosted-call fact and discarded.

Fresh pinned Rust 1.98.1 locked/offline checks on macOS arm64:

- Workspace: **403 passed, 0 failed, 1 ignored** private crash entry point,
  exercised by subprocess recovery checks. macOS file coordination required
  command-sandbox escalation; all data was synthetic temporary fixtures.
- `brn-ai` feature all-target tests: **67 library + 1 example passed**, no failures.
  Seven scripted integration tests cover all three routes and multiple scenarios,
  including partial/parser/status/transport failures, cancellation and no retry.
- Default workspace build, all-target Clippy with warnings denied and format
  passed. Feature example build and all-target Clippy passed. Missing arguments
  and unsupported Copilot Chat web exit 2 before credential-directory creation.
- Changed-document local links/fragments and `git diff --check` passed.

Commands: `cargo test -p brn-ai --features capability-spike --all-targets --locked --offline`;
`TMPDIR=/private/tmp cargo test --workspace --locked --offline`;
`cargo build --workspace --locked --offline`;
`cargo clippy --workspace --all-targets --locked --offline -- -D warnings`;
`cargo build -p brn-ai --example provider-capabilities --features capability-spike --locked --offline`;
`cargo clippy -p brn-ai --features capability-spike --all-targets --locked --offline -- -D warnings`;
`cargo fmt --all -- --check`.

Offline preparation is locally integrated at `7e63041`. The following live
qualification used `main@0b0c8e77e2da53a71c7c2b700269ef6b4a66e346`, followed
by the bounded diagnostic/error-mapping delta described below.

## Authorized live results — 2026-10-03

The owner answered Yes to the complete bounded scope above and completed both
sign-ins in a regular browser. The first Copilot attempt had timed out at 600
seconds after its embedded-browser authorization control remained disabled;
the new Copilot and ChatGPT connections succeeded. Device codes, account names
and credentials are deliberately absent from this record.

All live data used the fresh synthetic task folder
`/private/tmp/brn-provider-qualification.yw2v91wv`, with explicit data, vault and
protected credential paths. Copilot discovery returned both requested models;
ChatGPT returned its maintained `gpt-5.5` option. No original credentials/vault,
model download, purchase, durable knowledge write or release occurred.

| Exact selection / pinned route | Low/high and read tool | Fixed PNG | Hosted web and sources |
| --- | --- | --- | --- |
| ChatGPT `gpt-5.5` / Responses | Both completed; one read each; exact `orchard-827` | Correct upper red/lower blue | Completed hosted search observed; correct official release/version/date and source URLs; native annotations absent |
| Copilot `gpt-5.3-codex` / Responses | Both completed; one read each; exact `orchard-827` | Request completed, but answer reversed the colors; accuracy unqualified | Completed hosted search observed; correct official release/version/date and source URLs; native annotations absent |
| Copilot `gpt-5.5` / Chat Completions | Low failed `Other`; high established HTTP 400 `unsupported_api_for_model` | Skipped after explicit route refusal | No native web seam in pinned Chat route; not invoked |

Both web answers identified SQLite **3.53.4, 2026-07-24**, independently checked
against the [official release chronology](https://sqlite.org/chronology.html).
Returned official URLs included `https://sqlite.org/`,
`https://sqlite.org/chronology.html` and `https://www.sqlite.org/changes.html`.
Native citation metadata is **unqualified**: neither live report contained
annotations, although actual parser/serializer fixtures qualify their projection
when present. The single Codex image answer does not establish reliable vision.
These are narrow observations, not general model-quality or account guarantees.

**10 of 11 logical probes ran, with 14 completion attempts**: two per successful
read-tool probe and one for each other invocation. Authentication/discovery are
separate. There was no application retry or selection/route fallback. Exact wire
send counts and upstream cancellation remain unobserved. The remaining image
probe was skipped under the agreed stop-on-model-refusal rule; there are no more
planned live calls. A new probe round or different Copilot route needs a new
live scope, not reuse of this record as standing account authorization.

The high-effort rejection was investigated with a probe-only diagnostic:
fixed error class, numeric HTTP status and exact allowlisted rejection category.
Bounded transient JSON parsing discards bodies, unknown codes, messages, headers
and identifiers. Production safe error mapping now recognizes the demonstrated
`unsupported_api_for_model` code as `ModelRefused`, explaining that the selected
provider route cannot support the model. It does not change route/model or retry.
All three actual Rig routes regression-check this failure without raw retention.
The earlier low failure's specific cause was not retained and remains unknown.

Final independent read-only review found no actionable defects in the diagnostic
or production error-mapping delta. Fresh pinned Rust 1.98.1 locked/offline checks
on macOS arm64 after the final code change:

- Workspace tests: **404 passed, 0 failed, 1 ignored** private crash entry point
  exercised by subprocess recovery checks.
- Feature AI all-target tests: **68 library + 1 example passed**, no failures.
- Default workspace build/all-target Clippy with warnings denied and format
  passed. Feature example build and all-target Clippy passed.
- Documentation links/fragments and `git diff --check` passed.

Commands are the same offline commands listed above, with workspace tests run
without a TMPDIR override. Native code is unchanged; prior native evidence remains
applicable within its recorded limits. The
spike establishes enough current route evidence to proceed to Proposal Core;
unsupported/unqualified capabilities remain explicit downstream constraints.
It does not enable effort/image/web inputs in ordinary chat, assert native UI
acceptance, qualify a future account or remove the need for attributable source
verification. Runtime model-list `live_qualified` flags remain false; these
observations are task evidence, not an account-wide persisted capability cache.

Reproduction: build the feature example with the commands above. With a newly
authorized bounded scope and fresh human connection, run one explicit invocation
per selection/kind, inspect exact note code or PNG colors, and independently
verify web URLs/version/date. Stop a refused route. Native application interaction
and server cancellation need separate qualification; owner sign-in succeeded.
