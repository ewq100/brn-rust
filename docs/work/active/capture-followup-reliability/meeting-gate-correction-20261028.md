# Offline meeting qualification gate correction

The reported Mac job `brn-live-qualification-001` remains **FAILED**: its local
adapter incorrectly required a pending-only Action and rejected a saved Suggested
Action allowed by the fixture. The report says the first-source action, protection
and lineage checks passed; the actual local adapter and raw snapshots are not
available here and were not replayed. One Run was consumed, fourteen cumulative;
four planned Runs remain unrun. Input tokens were 75,459 including 47,616 cached;
output was 921; cost was unavailable. The reported model was `gpt-6.1-sol`,
medium effort, helpers off.

The committed offline prefix gate reuses the fixture oracles and checks baseline
Actions and receipt after-images. It permits fixture-conforming saved Suggested
Actions and pending proposals, requires exact observed source lineage and retains
protected-state checks. The full two-source structural gate remains unchanged.
Offline synthetic tests do not establish a live case pass. This correction does
not reopen the failed job or authorize a retry, new Runs, or Mac access.
