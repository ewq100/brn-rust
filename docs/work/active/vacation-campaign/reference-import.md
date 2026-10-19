# Worker-bound reference-import intent

Source `ed859aa466d0b5fc6d0eae57f6277f5ad245aafc`, integrated at `14d7c43e`,
puts the captured target, purpose and idempotency key in the existing Import
operation. Non-Import workers cannot carry it; ordinary imports explicitly pass
None. The separate UI field, post-start assignment and cleanup branches are retired.
Completion copies the intent before worker retirement; disconnect drops it with
the worker. The writing view derives importing state from that same owner.

File-picker admission/target validation, conversion settings/cancellation, checked
service scope/version/dirty-setup/idempotency and queue/successor policy are unchanged.
The existing unused data argument was replaced by the relevant optional intent,
preserving the private method's arity and actual Workspace authority. No new operation
kind, general owner, dependency or reproduced defect is claimed.

Independent Sol approved both [the frozen plan](evidence/reference-import-approved-plan.md)
and clean14d7c43e without correctness/design findings. Existing consequential effect
tests are migrated into the supported constructor. Removed independent intent-cleared
assertions are redundant with retirement now that no separate UI intent exists;
canonical/cancel/scope/successor-attachment/saved-setup assertions remain.

Six focused filters passed seven test executions. Final desktop321/0/1, strict
all-target lint, format/whitespace and an actual production-only native-ui build
passed. Original incomplete-caller and local-name compile errors remain in the
logs; they were corrected before the passing checks. [Commands](evidence/reference-import-commands.txt),
[worker receipt](evidence/reference-import-worker-receipt.md) and [hash/result receipt](evidence/reference-import-qualification.json)
retain exact source/results, raw/compressed hashes and immutable binary identity.
The lead verified all tracked Rust/manifests and root lock/toolchain against the
qualified source and rechecked the binary digest. Independent Luna receipt audit
confirmed all 13 compressed/raw hashes, commands, source and binary receipts and
the actual reviewed commit patch.

The shared external CLI fixture remains ignored; no live provider/model or external retrieval,
private data or actual OS input was used. Hosted exact-head checks, normal merge/main
verification and native qualification remain separate. PR166 merged from its checked
73780065 head asb4a5deeb. Lead merge2bd890ef adds only that main history, with no
crate/root manifest/lock/toolchain delta from reviewed14d7c43e.

PR167 subsequently merged normally as9116ad78 after all four required exact-head
408 checks passed. [Integration receipt](evidence/pr167-integration.json) tracks
resulting-main run39224642627, currently running. Its crate/manifest/lock/toolchain
tree matches qualifieded859aa4. Native qualification remains separate.
