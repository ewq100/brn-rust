# Exact efc survivor disposition

The raw efc automatic verdicts stay **MISSED** in mutation-supplement-per-id.json and supp-new-guards-outcomes.json. The following qualification is source analysis, not a retroactive catch or an executed mutant result.

Both actual emitted diffs target threads_context_budget.rs:144:52, replacing `boundaries.len() - 1` with respectively `boundaries.len() + 1` and `boundaries.len() / 1`. The boundary vector contains all character starts followed by text.len(). This analysis applies to these exact initial-high diffs, not the interrupted division at line150 or any other timeout/unexecuted ID.

The `+1` change is a real gap. For text `õ`, deterministic predicate `str::is_empty` rejects the full text and accepts the empty prefix. boundaries=[0,2], low=0; mutated high=3 gives first mid=2, which indexes beyond the vector. Correct code high=1 probes index1, rejects it, and returns None. The concise new budget assertion exercises this meaningful no-whole-UTF-8-character-fits boundary. A new-pin catch is claimed only after an actual targeted recheck; the old efc MISSED is unchanged.

The `/1` change gives high=n instead of n-1, where n is the vector length. This is qualified redundant for the current production callbacks:

1. Empty input returns before the search. For nonempty input reaching the search, n>=2, the full-text precheck is false and the empty-prefix precheck is true.
2. The callbacks in read_note, bounded_record_output, source_metadata and exact-version read_history deterministically replace the current text/range fields before computing the exact same structured-output estimate. No callback accumulates state between probes. On proper prefixes, text's encoded length strictly increases; end/next_start integer widths cannot shrink; complete/truncated flags are fixed. Thus fitting is monotone over proper prefixes. Full-text special metadata can change, but full text is already known not to fit.
3. Let T<=n-2 be the last fitting index. low never exceeds T. While mutated high=n, low<=n-2, so the upper midpoint cannot exceed n-1. A full-text probe at n-1 rejects and sets high=n-2; the index n is never accessed. Other probes preserve low<=T<=high. Once high<n this is ordinary binary search. Both initial bounds converge to T, so output and refusal are unchanged.

This is not a promise for arbitrary stateful/nonmonotone FnMut callers; none is present in production. The new helper assertion also has a deterministic monotone predicate. The exact `/1` variant can therefore be excluded from the tiny final campaign as qualified redundant under these concrete preconditions. It remains a raw efc MISSED, never relabelled caught/equivalent in the original outcomes. Six efc timeouts and three no-verdict IDs remain separate unresolved campaign outcomes.
