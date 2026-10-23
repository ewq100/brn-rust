# Independent review: Mac precise-edit live qualification

Verdict: **APPROVED for the final reviewed qualification/evidence candidate**. Final local-check receipts are reconciled; integration gates remain separate. No remaining actionable correctness or scope findings in the inventory below. This review includes one owner-assigned bounded final receipt reconciliation; it is not native UI acceptance or merge authority.

Reviewed on 2026-10-23. Repository `/Users/evokessler/.codex/worktrees/precise-edit-live-mac/brn-rust`, branch `codex/precise-edit-live-mac-20261023`, HEAD/baseline `6c86583d709e13e7fc3c5070008d49c824856d07`. The actual candidate consists of the five-line README addition plus untracked mac-live.md and all files in evidence/mac-live-20261023; no product code, prompts, schema, toolchain or lockfile changes are present. Read AGENTS, owner review packet, independent-review skill/resource policy, task README/mac-live, architecture/invariants, verification/workflow and CLI contracts; inspected baseline precise-edit/trace implementation for the relevant claims. No provider call, scenario execution, build, test, network operation, Git mutation, vault inspection or delegation performed. The only review write is this report.

## Evidence assessment

- One attempt is reserved before launch in the driver, with no automatic retry and no session relaunch allowed. Session admission records 8% remaining, a stricter 5% floor, 180-second CLI timeout and four tool rounds. Current model catalogue supports gpt-6.1-sol/medium; configure plus Settings receipt show that selection. A separate upstream resolved-model field is absent and the document says so. Allowance/login observations are retained lead observations, not independently refreshed in this review.
- The shipping `ask` receipt completes in 10.667 seconds, two tool rounds/three model turns, with no dropped calls: exact target read, then successful prepare_note_edit with null/null offsets and expected/replacement lengths 12/10. No edit-note grant, apply tool call, helper or fallback appears. Fresh synthetic preparation uses test purpose, a new explicit root, existing auth path and no auth copying.
- Both unpaged candidate snapshots are identical, contain exactly one operation and one target write, expected version 1, unchanged archive flag and metadata. Candidate operation, request hash and Apply receipt agree. Baseline source shows null/null means one unique exact quote; trace intentionally retains only lengths. The resulting candidate and expected bytes support this one replacement without claiming raw model strings were recorded.
- Independently wrote literal original/expected/sentinel expectations during inspection rather than obtaining the expected answer from production transitions. Original has 221 UTF-8 bytes and SHA256 `4b4e108c7b8ff47b3fb77ff657ad3e76d193865c324cce629bd64af8dd5a5cc3`; expected has 219 bytes and SHA256 `01d162b02bcf7e9a144cf8b15312a751eb39925e515f704c985017c5d8ef0c4c`; sentinel has 64 bytes and SHA256 `cd3d723982810020b572aa364d7004f935155f45c3a3aef02de07cffbfbfed91`. These match session fixture hashes. Combined fixture+prompt input is 642 bytes, below 8KiB. Packaged guidance is correctly excluded from that small-fixture statement and actual token usage is separately reported.
- Read and compared every saved target/sentinel create/read record, including complete metadata, archive state, ID, version and Markdown. After-ask and inspect reads equal baseline at version 1. Apply receipt and subsequent read equal expected at version 2. Undo receipt and subsequent read equal original at version 3. Every sentinel phase equals its complete version-1 baseline. Driver uses a new subprocess for every CLI command and verifies the frozen binary hash between phases. This supports fresh-process persisted reads, not OS/native interaction.
- Repeated Undo uses the same explicit owner key. Original/replay compensation operation, hash, receipt and timestamp are identical; post-replay full target/sentinel records remain unchanged. CLI README's retry table and saved command contract explicitly define keyed Undo as replay and unkeyed Undo as duplicate. Documentation accurately limits the result to keyed replay.
- Initial list/object script defect and recovery are expressly recorded, including pre-correction script hash. Original failing source/exception transcript is not in the candidate, so the original failure description is a lead record; saved successful ask, unchanged reads and identical later inspected candidate corroborate safe recovery. Inspection/verification phases contain no ask. No evidence of a blind retry or second request in this session.
- Trace usage sums correctly to 19,499 input/246 output/12,544 cached-input tokens; cached input is not added again. Monetary cost is unavailable. The model's broad “no records were changed” prose is explicitly disclaimed as evidence; canonical Note receipts establish the narrower assertion.
- Baseline check receipt binds all four required successes and all three optional Windows failures to the exact baseline head. Saved branch protection names those four requirements. This is dated baseline evidence, not current exact-head or resulting-main CI. Original full CLI contract SHA256 matches the saved reduced contract's full_contract_sha256.

## Finding and disposition

**P2, valid/fixed:** `evidence/mac-live-20261023/commands/delegation.json` Settings before/after snapshots originally retained the private configured auth-file path, bypassing configure's intentional redaction. The lead replaced both values with `EXISTING_AUTH_LOCATION`, added recursive auth_file/credentials_dir path redaction before every saved receipt in scenario.py, and recorded the correction in session.json. Re-inspected final driver/receipts and recursively validated all task JSON auth-file/credentials-dir fields are null or the placeholder. No token/auth contents were observed. This correction changes evidence sanitization only; no second model request or synthetic-store mutation was used.

## Gate qualifications

The final local-checks.json and all referenced retained logs were inspected. All seven declared log SHA256 values match actual files. Ordinary CLI build and strict `cargo clippy -p brn -p brn-ai --all-targets --locked --offline --jobs 1 -- -D warnings` logs end successfully. The retained note-edit test log shows 17 passed, zero failed/ignored, 489 filtered. Markdown log reports 647 files/2931 local links/zero failures; overview log reports current. Workspace formatting has an intentionally empty output log and recorded exit 0. Python compilation and `git diff --check` have recorded exit 0 without raw logs. These are inspected lead-produced receipts, not checks I executed. Initial offline-build failure output was overwritten; local-checks.json explicitly discloses this retention limitation, records the missing locked dependency cause and the locked-fetch correction. It is not presented as a passing initial attempt.

Final mac-live.md now accurately states those local results and retains pending exact-head CI/normal merge/resulting-main CI. Session closure adds `closed_after_one_successful_attempt` and false additional-request authority; its attempt, model, byte hashes, saved operation and phase receipts are unchanged. Scenario, trace/candidate/Apply/Undo/read receipts and README addition match the prior reviewed hashes. The incidental compiled Python cache from the first inventory was removed, with no source change. No further finding arose in this bounded reconciliation.

Exact-head CI, authorized normal merge, resulting-main CI and owner acceptance remain separate unpassed gates. Native Mac controls, focus, visual review, accessibility and picker behavior remain unqualified. One synthetic success does not establish general model reliability.

## Reproducible reviewed inventory

Paths below are repository-relative; SHA256 covers actual bytes including dirty/untracked files. Inventory SHA256 is computed over each `SHA256  path\n` line in the order below. README diff SHA256 covers raw `git diff -- docs/work/active/precise-edit-ui-qualification/README.md` bytes.

Files: 40. Inventory SHA256: `14ba80ec310f49be1ffdd0106da0637de53009a4b8bf08e9ad206e2d469de10d`. README diff SHA256: `cc7d8070ff5abf2a15af2dcb78b023eba56cdc47bb2c4e36c9106b1101ff3fb4`.

```text
bca938b2868089e15bd2768cdf848cc72b69a4291c498be77bdb1cc9293935ed  docs/work/active/precise-edit-ui-qualification/README.md
572b841848e4f7c6b82bcef0907b03a0b0e6cb1fe3a3d87fc43598ec565f4086  docs/work/active/precise-edit-ui-qualification/mac-live.md
ec2f212fb792a972b8d6abf718b9774c863faac92f4bea9b579db415768b2d1d  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/baseline-checks.json
8da2d84652f85f85f76b1e5e2720acd44c6edc14ba000184d56e50809ede4fa7  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/baseline-protection.json
d15e78c7af2a80e481e3c2792a7b2dcd10018c38ecf7f375a983f3d2796bfa71  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/build.log
3c0f78268b4c2ee177254e37afe6132759fd4d1a3ad0962c07d99c1d8b7fed58  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/cli-contract.json
03858f1027f152b27db17afa554782cfb7d11f04fb1f1b45bc334229c8ebb1dd  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/clippy.log
8a2e2094aaf9ab7557d88b85aa15de8e87c11ad2d8f28520cca8cd1e8fe15792  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/after-ask-sentinel.json
a9fd3a8c11de9b12a4a9a20747890a5084b533b02b57589059be7093cc1ba733  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/after-ask-target.json
1d6bba0870c1a83813534d9e7d694c21c21407f19cfc4f6d0cc92f45e6beeefd  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/applied-restarted-sentinel.json
27140a66f8f0bf38b72b53f03f40ae7c5ba2fba6d39e899abfd01d65b900625f  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/applied-restarted-target.json
b9ed6617c61c01b703f6e7da77080c52b989e248f77edbdc6f743c7515e7adec  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/apply.json
acbec90ae6c53f0027b3f7a9182e831da177e555475878b1d4145f349db7f482  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/ask.json
50235978f0a046c882e7c2fe27ce092fd677f0a51819a8f17738cbb56baf6136  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/baseline-sentinel.json
f21f690f8908da19efba6749a2d89cd703a28621b5f5cc319cdfdfefbc350a08  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/baseline-target.json
e02aee5e6ee03b527fe0bbf49b721f04e26986066137bb4dd9359832455f1da9  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/candidates-inspected.json
8b8a905a4f2ff2423d0d8ad18c83947a45a03a87323fead35c54a3c507a06834  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/candidates.json
5b8ab27fdab73a3b70429488da6c2814d5bd832c8601033414fc43c77e1d8037  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/configure.json
915ee34f131000ec938351c200fd2e30a07a19bbe7d48674f14f2a31188b2734  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/delegation.json
7d85053a9191a932dac4e4443e25c1e638dad111d5e92f4ca145145b154fc41b  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/init.json
e963266c7e6620a4a3552f8c4e3bc16f0e959cbcd38a04bca79cc0445f702211  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/inspect-sentinel.json
6fe840a9a08d817eeda34cbddd1fbb3456411260a53337950a2a677b0ebedf9f  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/inspect-target.json
1d6bba0870c1a83813534d9e7d694c21c21407f19cfc4f6d0cc92f45e6beeefd  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/replay-restarted-sentinel.json
67c71658b43d0da2a28b8086408fd358a558d332831d98dc15d2c8ae10cb076d  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/replay-restarted-target.json
c764c768537c18e520b502b1dc4c3830e213d686b948c3c0684c763c97253c30  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/sentinel-create.json
1a9bcfce26d7a871add645f90ed491e7bdf9669daf79b88480515ee77770f3db  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/target-create.json
ee32660ba6610e2ec6dd3419c061cecffd35d13c9ebc44fe1168e1b21a0b4721  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/thread-create.json
63da9604f8ee538be43049cb2e85d36e375c6d33ef719fedcef0a5093578bf29  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/undo-keyed-replay.json
e963266c7e6620a4a3552f8c4e3bc16f0e959cbcd38a04bca79cc0445f702211  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/undo-restarted-sentinel.json
67c71658b43d0da2a28b8086408fd358a558d332831d98dc15d2c8ae10cb076d  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/undo-restarted-target.json
4470d2897474494c6701ae209c00bcacda711a138f6ecc7f479b61d64a8ee5d2  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/commands/undo.json
6e453f6ab8e907fb52514b87759a73aaca5d21f4c7da3b04be2a8db8f86e5ab9  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/fetch.log
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/format.log
d4976a576082e453243ec17f4e59b6cba571578dc756dd2d7dc6d8f199fae70c  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/local-checks.json
7f5b209bdcd10407f17e6856e6046df2029449d14789b638948b8fc6b700c3a4  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/markdown.log
bdd9e9e8a03ccda1fbad69c5b7b1845c31a6ca8216212b32635170a724395e38  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/model-preflight.json
1b870681642da3505e35061edec25a73a15c8646d534944b82c0db97219e652a  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/note-edit-tests.log
60fd2e653564bddb46b82363ae68f36a94ca69a93681f982d070780bc681fe79  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/overview.log
3eebb6c13ddf65ac81806b7dcde214e72bfb3dd4a0eba8141b5dcc389eba4d10  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/scenario.py
b740d6e291065ae19ff3ac22a9c006aee99e5637c9911884302b7d5f5811d368  docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/session.json
```
