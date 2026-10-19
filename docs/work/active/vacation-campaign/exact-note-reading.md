# Exact Note reading guidance

One owner-authorized synthetic Codex Luna-low request omitted the `:0` suffix
from a supplied Note ID, read no exact record, then reported the Note inaccessible.
A separate explicit exact-ID/read_note diagnostic succeeded. These observations
and the bounded account-test policy are preserved in [PR174](https://github.com/ewq100/brn-rust/pull/174).

This candidate clarifies only existing JSON-schema descriptions: full opaque IDs
copied verbatim, observed versions, exact UTF-8/heading/continuation offsets and
the existing required bounded count and section arguments. Types, required fields,
limits, dispatch, exact ID matching and access remain unchanged. Packaged bundle17
bytes are unchanged; the source and executable hashes distinguish this metadata.
No formal ID grammar, shortened-ID repair or model fallback is introduced.

[Independent source review](evidence/exact-note-reading-source-review.json) approved
corrected full candidate9229085. Initial wording omitted an advertised heading start;
the final wording includes it. [Qualification](evidence/exact-note-reading-qualification.json)
records runtime522/0/2, strict all-target lint, format and ordinary CLI success.
The initial519/3/2 failure was caused by omitted helper paths; the same binary's
six affected tests and the full suite pass with unchanged qualified helpers.
Original logs and the source/command/binary bindings are retained losslessly.

The final natural-request probe used the same Luna low and the original prompt
structure with a fresh synthetic Note. It read the full supplied ID, returned the
correct codeword/total and preserved the Note with no candidate. The current
three-attempt account session is closed. One post-change success does not establish
statistical model quality. Actual native AI/usability and hosted/main checks remain
separate gates. The trace's truncation flag has a separately recorded presence-vs-
value diagnostic defect; it does not establish read coverage.
