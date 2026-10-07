# P1 Office/MIME adoption evaluation — 2026-10-07

## Authority and baseline

The owner selected P1 and authorized isolated evaluation, a conditional P2 implementation specification, checks, independent review, a commit and one reviewable PR. Production dependency adoption, P2 implementation, other PR merges, release, private data and the separate live-model campaign are not selected. The [owner amendment](../../../product/BRN_PRODUCT_VISION.md#owner-amendment--2026-10-07) and [existing plan](plan.md) govern scope. Historical PR85 finalization-only restrictions describe that completed session; this later selection authorizes the experiments here.

Fresh remote inspection found main at `3d59cdd4d6379b526fed19f6a339cfd2dce2dead`, the confirmed PR85 merge. Open PRs were 84 (H4), 82 (H1 integration record), and 80 (H5). The registered main checkout is at `18f3891` with unrelated untracked `.DS_Store` files; all existing worktrees and changes were preserved. This task uses isolated branch `codex/p1-office-mime-evaluation` in `/Users/evokessler/repos/brn-p1-office-mime`, starting at verified remote main.

Read-only status checks across all registered worktrees found the previously recorded dirty `brn-removal-boundary@4a6446f` patch (Store/workflow/native changes plus untracked original-operation files); other pre-existing linked worktrees were clean. Nothing was reset, rebased, cleaned or merged. PR84 remains `0f3e277`, PR80 `be59a7a`, PR82 `8018755`; their historical experiment/status edits overlap planning records but are not dependencies or selected PR updates.

Resource record: requested lead Sol 6.1 High retained; native context settings unchanged, effective numerical context/credit cost not observable. One explicitly configured Sol 6.1 High evaluator owns only the new standalone experiment directory; the lead owns documentation and integration. A fresh independent Sol 6.1 High reviewer will assess the complete result. Normal allowance is at most two active helpers including review, with no delegated agents. Stop/reassess at a bounded adoption recommendation or a specific evidence-backed blocker; do not broaden into all-Office fidelity or the runtime-model campaign.

## Bounded result and recommendation

Recommend **conditional adoption of BetterOffice DOCX 0.3.0 plus mail-parser 0.11.8 for the proposed P2 EML/DOCX slice**, with one small evidence adapter, versioned extraction snapshots and the existing proposal/apply/recovery system. This is a recommendation for the owner's next decision, not an installed production dependency or accepted design. PPTX informs the common evidence envelope but is not added to P2 production scope. No concrete remaining gap justified another library comparison in this pass.

The [standalone experiment](../../../../experiments/architecture-reassessment/p1-office-mime/README.md) retains fixtures, code, manifests, raw outputs, measurements, readable adapted views and rendered originals. Its new producer-generated corpus uses python-docx, python-pptx and a valid meaningful PNG showing 120 → 72 litres/day. It is distinct from the earlier tiny handcrafted probes and invalid CID sentinel. Synthetic producer coverage is still not a representative private/real-world corpus qualification.

Demonstrated:

- Useful DOCX paragraphs, ordered table cells (including repeated `Pending`), header/footer constraints, exact image bytes and two separate picture occurrences/titles. Reflowed views show the actual table and each image at its document location, with evidence locators.
- Useful PPTX slide text, presenter notes and pictures; picture titles are scoped to their containing picture and exclude the titled non-picture dependency shape. The second slide's native chart is **not** rendered by the adapter: the adapted view visibly names the omission, while the original preview shows two inspection slots on 12 October and zero on 13 October. Those values cannot be quoted as adapter-extracted chart evidence.
- Real MIME text/HTML, encoded subject, original Date/timezone, actual message ID, valid CID PNG and byte-identical DOCX children. Separate email/attachment node identities preserve parent associations even when the same attachment bytes occur in two emails. The plural case retains an unsupported XLSX as unprocessed and preserves both occurrences of the same DOCX PNG. MIME subtree/CID ambiguity, every encoding and arbitrary malformed mail remain unqualified.
- Four hostile packages are rejected for traversal, DTD, expanded-byte and member-count limits. Native sandbox activation before input access denies synthetic vault/credential/repository reads and connection to a live loopback listener. Cancellation terminates the synthetic parent/child/grandchild process group. These witnesses exercise real OS behavior, not just parser return values.

The first pre-main `sandbox-exec` approach aborted before parsing. A broad-file-read diagnostic was rejected by automatic approval review and never executed. The successful safer alternative starts the trusted runtime with an empty environment and activates the same deny-default native sandbox before any fixture/converter access; it grants no general home/private-file access. Failed launch metrics are retained separately and are not counted as successful converter timings. Neither the legacy launch recipe nor a broad-read profile is a production recommendation.

## Measured costs and remaining conditions

See experiment `evidence/costs.json`, `measurements.json`, enabled graph files and source/output artifacts for exact scope. On Apple Silicon, Rust 1.98.1:

| Measurement | Result and limit |
| --- | --- |
| Enabled normal registry graph | 53 packages on arm64; lock has 56 registry packages. Ten exact name/version pairs differ from the root lock (eight new names). Includes DOCX, PPTX and MIME experiment together; not a measured P2-only application delta. |
| Source archives | 6,082,560 bytes for enabled packages; 2,479,002 bytes incremental exact-version archives versus root lock. No Python/models required by the native helper; fixture/QA tooling is separate. |
| Helper installation/full replacement | Stripped helper 4,297,440 bytes; 3,924,400 bytes above a stripped empty helper. Compressed whole-helper archive 1,881,572 bytes. This excludes full BRN linking/bundle, signing and renderer deployment. |
| Build | Clean target with dependency/source cache already present: 35.805 seconds, child peak RSS 1,684,504,576 bytes. Warm unchanged build: 0.095 seconds. Download time and cold filesystem-cache effects are not controlled. |
| Runtime | First DOCX process 28.291 ms; warm processes 25.697–25.782 ms. PPTX 10.135–10.759 ms; EML/DOCX 26.294–30.972 ms. Peak observed parser-process RSS at most 47,087,616 bytes in these final runs. Three starts per benign fixture; uncontrolled OS caches, not throughput/corpus benchmarks. Earlier measurements during concurrent compilation are retained separately. |
| Adapter scope | Approximately 400 Rust source lines for the complete probe including guards/MIME/native restriction checks, plus host supervision, fixture/oracle/QA tooling. The experiment documents exact final counts. This is not a claim that production integration requires only 400 lines. |

The small adapter delivers useful content/associations beyond the historical strict custom profile without a second full OOXML parser. That supports the direction; it does **not** prove lower lifetime maintenance cost. Migration, historical reader retention, snapshot/evidence binding, production supervision and useful CLI/native review are real remaining integration costs. Full application size/build delta, clean-machine install/update/rollback, signed distribution, renderer packaging and lifetime maintenance remain unmeasured. Production manifests/lockfiles/code are unchanged.

Working limits are 8 MiB input, 16 MiB expanded package bytes, 512 members, XML depth 128 / 200,000 events per part, three CPU seconds, five wall seconds, 64 file descriptors and 32 MiB per output file. No hard RSS ceiling is established; measured RSS and finite input/event/output bounds are the evidence. These are experimental settings, not frozen product limits. P2 must validate aggregate decoded/image/output budgets, complete image decoding, protocol trust, packaged restriction activation, cancellation/crash/shutdown and omission reviewability in the actual workflow.

Known Office gaps remain explicit: native chart/SmartArt/complex-drawing preview, layout/spatial fidelity, revisions and unrepresented containers are not qualified. Do not mark such extraction complete or convert a missing central visual into an inferred fact. Keep meaningful original inspection and partial status; PDF/layout selection remains later. No BRN live model, native product GUI, actual proposal/revision/approval/history integration or owner acceptance is demonstrated by P1. The [conditional P2 specification](p2-conditional-spec.md) defines those separate acceptance gates and the replacement/retirement inventory.

## Gate record

- Defined: owner-selected P1 question and conditional P2 planning scope.
- Evaluation: useful bounded parser/adapter/containment result demonstrated; earlier tiny probes reused as historical evidence. Production adoption remains conditional on owner selection and P2 gates.
- Conditional P2 specification: prepared from actual production paths, conditional on adoption/selection.
- Independent review: completed fresh Sol 6.1 High read-only full-candidate review with **no actionable findings**. The reviewer verified the [review inventory](p1-review-inventory.json) SHA256 `115933fadd1ce1e4a88e265276103430e55e8e0f3bb84accf4969a5bb8085e3b` (88 files / 6,196,155 bytes) unchanged at start/end, fixture/raw-output/release hashes and enabled graph/archive calculations; independently passed the oracle in a temporary copy, format and strict offline all-targets Clippy; visually inspected all eight original/adapted page PNGs; checked actual native activation ordering, scoped failure/negative-access/cancellation claims and conditional P2 authority/recovery boundaries. Initial specification review was also clean; it did not substitute for this full pass.
- Verification: lead strict offline experiment all-targets Clippy and format passed; Python syntax passed; frozen oracle passed in a disposable copy; measured release SHA matched; staged EML bytes matched disk exactly. Final current Markdown gate passed 46 files / 539 links / 0 failures, explicit experiment README outgoing gate passed 21 links / 0 failures, diff checks passed and Git attributes correctly suppress raw-wire diffs/preserve EML bytes. Remote main was re-fetched before publication and remains `3d59cdd`. No unchanged workspace/native/live tests are relabelled as new evidence.
- Post-review changes are only gate/status/handoff records, the durable inventory copy and experiment README/Git diff presentation (`*.wire.json`/PDF diff suppression). Parser, fixtures, results, measurements and P2 specification are unchanged. No reviewer findings were deferred or left unresolved.
- Candidate publication: one committed owner-review PR carries the result; exact candidate SHA/hosted-check state belongs to that PR. No merge is authorized.
- Adoption/implementation acceptance and merge: not authorized by this task.

## Durable next action

The owner can accept/decline the bounded BetterOffice DOCX + dedicated MIME direction and select/amend the conditional P2 implementation scope. Until that decision, keep production dependencies/code unchanged. The P2 record identifies the custom new-import parser/singleton coupling to replace, shared protections/effect families to retain and legacy readers needed only for old records. Carry its two product stories, packaged-helper/reviewability and native/live/owner gates forward. The separate runtime comparison remains authorized for its own campaign but is not started here.

Observed friction worth an optional later retrospective: pre-main macOS helper startup, keeping concurrent-build measurements separate from runtime timing, and avoiding giant raw-wire diffs. No retrospective or architecture-improvement task was started automatically.
