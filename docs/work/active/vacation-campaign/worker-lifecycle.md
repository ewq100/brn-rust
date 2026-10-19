# Worker terminal-admission slice — 19 October 2026

**Later hosted/integration observation:** exact final PR162 head `aebfa12b` passed all required contexts, Ubuntu Native UI and documentation. Each UI platform executed312 widgets,59 state tests, writing44 and host12; optional Windows failed on unchanged Unix authentication. The owner merged PR162 at `bd42c1d9`; this continuation performed no merge. [Final-head receipt](evidence/pr162-worker-ci.json). The earlier pending-CI/main statements below describe the original local checkpoint.

Code checkpoint `c24da6af40f50ba263c5f5ddb88cbf0420f3f7b3`, based on `41423230b539ba6ac6aad2e241af4baf74fbb4d5`; main remains `1b7625a8`. Fixed campaign clock in campaign.json remains authoritative. This is a partial ownership improvement within inventory15, not a completed migration of all Run/worker UI state.

## Concrete failure

Two old disconnect branches in `threads_ui.rs::poll_worker` had different cleanup: an import channel closing without events retained `reference_import`; closing after Progress cleared it. The file picker resets that intent, but real chat drag/drop and clipboard routes call `attach_chat_path`/`convert_file` directly. The next conversion could consume the stale intent and lose its requested chat scope.

Added real-Workspace GPUI regression `b3_reference_disconnect_clears_intent_before_the_next_chat_attachment`: queued Progress+disconnect and zero-event disconnect, preserved Note/Source/Asset/WritingSetup contents plus paused waiting queue, then ordinary chat attachment through the production poller using a synthetic decoder completion. The Progress control passed; unchanged production failed the empty-disconnect arm at the durable Note scope assertion (actualNone, expectedSome(chatB)). Baseline compiled and executed0pass/1fail,310filtered; receipt `vacation-worker-reference-baseline.log`. This is toolkit/service integration, not an OS file-picker/clipboard witness.

## Boundary and retirement

Use the existing private Worker as transport owner: drain its exclusive receiver, admit preterminal events plus only the first Finished/RunFinished/Converted, and distinguish Pending/Finished/Disconnected. A terminal outranks later channel closure; trailing old events cannot change a replacement invocation. Keep idle empty polls quiet. One UI disconnect effect preserves the old Run/visible partial text through the checked service before clearing worker, pending continuation, reference intent and matching context.

This removes the poller's duplicate disconnect branches and its local terminal-admission flag. Workspace/SQLite still own queue claims, exact message/Run/fence acknowledgement, leases and interruption; ContinuePath/recovery policy stays in its existing owner. Converted effects may synchronously start a new Run, so cleanup must target the old invocation before those effects and never clear the replacement afterward. Existing optional kind/thread/run-id fields and scattered preview state remain explicit future ownership work; no new stored state, generic event bus or generation framework is introduced.

Independent Sol source review approved this bounded seam with no remaining findings. It requested that the successor test consume its detached worker terminal before deleting its fixture; that bounded teardown and required trait import are included. Final full desktop execution passed315/0/1; strict all-target Clippy passed. Production native-ui build and repository formatting check passed. The existing ignored CLI/desktop handoff separately passed1/0 on a new explicit purpose=test fixture, built CLI and desktop from current material, with an explicitly missing auth file (no provider access). Local jobs are complete; hosted new-head CI remains pending. The pre-teardown full run also passed315/0/1; its followup lint caught the missing local test trait import, and that original failure is retained rather than presented as passing.

## Earlier pilot CI reconciled

PR162 run39174138977 on exact41423230 passed all four configured required contexts, Ubuntu Native UI, and documentation/tooling. Both Ubuntu and Mac actually executed writing44 + independent-host12, native state55, and component306pass/1ignore/3filtered. The zero-test writing doc target is not counted. Optional Windows default workspace build failed on Unix-only auth imports/constants; those files are unchanged against main. These receipts qualify41423230, not this new worker change or a release candidate. Raw log hashes and links are retained with the final checkpoint.

Fresh Cloud listing still returns401; current environment has enforced unrestricted networking and no exposed quota/goal/wake capability. No remote dispatch or supervisor, no next wake; unattended continuation remains inactive.


## Qualification receipts

All commands use the repository pin/lock and activated cloud tools, with synthetic fixtures outside Git ancestry. Code checkpointc24da6af is the executed production material; later commits update documents/evidence only.

| Command/interface | Result | Retained log |
|---|---|---|
| Full desktop native-test-support bin tests | 315 passed,0 failed,1 existing ignore; includes4 protocol tests and2 new production-poller journeys | `vacation-worker-desktop-final.log` |
| Existing ignored CLI/desktop exact reuse case | 1 passed,315 filtered; fresh offline fixture and no provider contact | `vacation-worker-cli-desktop.log` |
| Desktop all-target strict Clippy | Passed | `vacation-worker-clippy-final.log` |
| Production desktop native-ui build and CLI build | Passed | `vacation-worker-production-build.log`, `vacation-worker-cli-build.log` |
| Repository format check | Passed, explicit exit0 | `vacation-worker-fmt-final.log` |

Raw compressed logs (including baseline/initial lint failure) and uncompressed hashes are in `evidence/logs/manifest.json`. Earlier pilot hosted results are in [CI receipt](evidence/pr162-first-pilot-ci.json); its successful required checks are not transferred to an unrun later head. This slice did not rerun the earlier production X11 Note journey and does not claim new native OS-worker or whole-Run ownership qualification.
