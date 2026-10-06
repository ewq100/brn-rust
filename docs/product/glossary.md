# BRN product language

Compact definitions of settled product concepts. [Product Vision](BRN_PRODUCT_VISION.md) owns intended behavior; [architecture](../architecture/overview.md) and [vault format](../architecture/vault-format.md) own boundaries and current representation. Implementation gaps belong in status/tasks, not these definitions.

## Knowledge and evidence

**Original content:** what arrived in BRN, including wording and meaningful visuals. An original intake file is the retained incoming artifact; a Source is its preserved, inspectable evidence. [Authority](BRN_PRODUCT_VISION.md#41-source-evidence).

**Source:** evidence of what was received or actually communicated, distinct from BRN's interpretation and curated knowledge. Approval of its preservation does not make its claims current truth. Avoid using “current source” to mean current approved knowledge. [Authority](BRN_PRODUCT_VISION.md#4-knowledge-model).

**Asset:** an ordinary file containing meaningful source material, such as an extracted image, linked from a note. A description is interpretation, not a replacement for the asset. [Authority](BRN_PRODUCT_VISION.md#72-visual-material).

**AI interpretation:** a tentative semantic account of evidence, including uncertainty, distinct from original wording/assets. Approval of an annotation preserves that distinction. [Authority](BRN_PRODUCT_VISION.md#72-visual-material).

**Current knowledge:** the user's approved present understanding, separate from Source evidence and superseded information. A current label describes saved classification, not proof that a claim is true. [Authority](BRN_PRODUCT_VISION.md#91-current-truth).

**History:** retained superseded or completed information used as labeled evidence, examples or precedent. It normally does not govern current-state answers. [Authority](BRN_PRODUCT_VISION.md#43-historical--archived-knowledge).

**Retrieval scope:** the explicit choice of Current, Source, History or All evidence; Source and History can overlap. Scope changes what may be retrieved, not what is approved as truth. [Current contract](../architecture/vault-format.md#history-and-retrieval-scopes).

## Review and work

**Proposal:** an exact, reviewable candidate for durable changes; it has no application authority on its own. **Approval:** the owner's decision on that exact proposal version and bound evidence. **Application:** carrying out that approved change with checked effects. [Authority](../architecture/invariants.md#frozen-target-guarantees).

**Recovery:** reconciling an interrupted or uncertain operation from its retained evidence. **Undo:** an explicit recent inverse of an applied operation where supported, preserving later work and refusing conflicts. Recovery is not another approval and Undo is not a guessed rollback. [Authority](BRN_PRODUCT_VISION.md#33-version-history-and-recovery).

**Action:** retained operational work with lifecycle, responsibility and relationships; new related follow-up work is a new Action. **Finding:** a tentative review issue with evidence; closing it changes review state, not knowledge or Action completion. [Action authority](BRN_PRODUCT_VISION.md#12-actions), [Finding contract](../../crates/brn-workflow/README.md#tentative-review-findings).

## Removal and lifecycle

**Archive:** retain material while taking it out of ordinary active/current use. Knowledge History and session Archive are distinct uses of this idea. **Trash:** retain removed material for recoverable restoration. Neither means permanent erasure. [Authority](BRN_PRODUCT_VISION.md#35-deletion-and-trash).

**Delete:** the explicit removal operation for the named object; permanent deletion is a separate explicit action for important knowledge. Session Delete must warn about likely uncaptured outcomes and preserve captured durable knowledge. [Authority](BRN_PRODUCT_VISION.md#193-session-lifecycle).

**Restore:** return retained material or state in a named context: session restoration, Trash restoration, original-copy restoration or interrupted-operation repair. Always name that context; do not infer one from another. [Recovery contract](../../crates/brn-workflow/README.md#explicit-interrupted-operation-repair).
