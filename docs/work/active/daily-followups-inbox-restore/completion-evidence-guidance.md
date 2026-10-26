# Completion evidence guidance correction

Owner approval: `Sentinel_edc8704882d08191b468d38e52715ac2`. Cloud work starts
from draft PR229 head `63568bb3c3548399fd19d436dc964c1d8e5b42f7`, based on
merged main `b270ad2fb0e67e10a86f68d513a37bf309760f97`. Its Mac fixes are
preserved. The [Mac qualification](mac-live-model-qualification.md) records nine
attempted live Runs and a failed/partial result; raw Mac receipts were not
independently inspected in cloud. No cloud live calls or Mac execution occurred.

## Change and boundary

The base guide and `prepare_changes` data description now distinguish
Action completion evidence from source provenance. Without supported actual
completion, `Action.evidence` stays null. Source grounding remains in Links and
exact versioned inputs. Explicit owner confirmation or an observed result can
support completion; a draft can satisfy an internal drafting Action but does not
prove external delivery. Evidence does not grant authority to mark Done.

The instruction bundle advances21→22. The prior21 summary-cache invalidation
witness and CLI identity assertion advance with it. Canonical record shapes,
immutable serialization, argument normalization, host authority and stored model
outputs are unchanged. No Source ID is stripped or converted to null by code.

Existing intake, maintenance and reply skills were inspected and agree with this
contract; no duplicate workflow or broader skill rewrite was needed. Independent
review tightened the distinction between internal drafting and external obligations.

## Verification record

The new actual-runtime preamble/tool-schema regression failed on the original
guidance: missing “Action.evidence is completion evidence only.” The original
red output is retained. One intermediate build was stopped before test execution
to incorporate the review wording correction; it contributes no passing evidence.

The existing evaluator gains a negative control: a pending Action with a Source ID
in completion evidence still fails even when its Source Link and exact versioned
input exist. Null completion evidence passes with provenance intact. Production
evaluator logic and fixtures are unchanged. Guidance/helper-path tests28 and v2
tests99 passed (127 total); synthetic failure-reporting output is expected test
coverage, not a live provider result.

At code commit `79422f8fc78918b243e64bc3835552c59e05e068`, the new runtime
regression and prior21 summary-cache witness passed. The full cloud capability
suite **failed: 351 passed, 156 failed, 1 ignored**. Independent failure review
found125 explicit unsafe-credential diagnostics, one direct admission assertion,
and30 downstream failures consistent with admission refusal but not individually
root-caused. Synthetic credential ancestors encounter sandbox-mounted `.git`
directories (`/tmp/.git` and `/workspace/.git`); the production guard refuses them.
This is not a full local pass. No guidance-specific failure was found.

Strict all-target `brn-ai`/`brn` Clippy with `brn-ai/capability-spike` and warnings
denied passed, as did formatting, Markdown links and generated overview checks.
The compiled test binary contains the final reviewed external-obligation wording
and excludes the superseded sentence. The cancelled compiler child was stopped;
no cancelled build contributes passing evidence. [Selected red/green and failure
receipts](evidence/completion-guidance-checks.txt) retain the observed limits.
The existing hosted core job runs full capability fixtures and CLI process tests;
exact-head hosted results remain pending at publication.
Existing cloud synthetic credential-fixture restrictions are not worked around;
normal hosted CI owns CLI process qualification. No repeated live admission or
picker-portal attempt is made.

## Live handoff

The [pinned Mac continuation](mac-completion-evidence-continuation.md) uses code commit
`79422f8fc78918b243e64bc3835552c59e05e068` and must first test a fresh duplicate meeting case,
then only if it passes run the remaining conflict, grounded QA/maintenance and
precise-edit cases. Maximum8 new Runs/30minutes/12 tool rounds, preserving prior9
separately. Strict completion-evidence assertions remain unchanged; stop on any
failure, without repeating passed cases or changing models. Guidance checks do
not establish live semantic success. Mac native acceptance remains separate.

This follow-up may update draft PR229 after a fresh branch-drift check. It has no
merge, release, calculator, credential or security-change authority.
