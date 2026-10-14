//! The note versions whose complete Markdown a run has been shown. A proposal
//! replaces a note's whole Markdown, so a write built from a prefix would
//! silently drop the rest; the host refuses it before it is prepared.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct ReadLedger {
    complete: BTreeSet<(String, u64)>,
    ranges: BTreeMap<(String, u64), Vec<(usize, usize)>>,
}

impl ReadLedger {
    /// The whole Markdown of this version was shown at once.
    pub(super) fn complete(&mut self, id: &str, version: u64) {
        self.complete.insert((id.to_owned(), version));
    }

    /// A slice of this version was shown; the version counts as read once the
    /// slices seen cover its whole length.
    pub(super) fn slice(&mut self, id: &str, version: u64, start: usize, end: usize, total: usize) {
        let key = (id.to_owned(), version);
        let spans = self.ranges.entry(key.clone()).or_default();
        spans.push((start, end));
        spans.sort_unstable();
        let mut covered = 0;
        for (from, to) in spans.iter() {
            if *from > covered {
                break;
            }
            covered = covered.max(*to);
        }
        if covered >= total {
            self.complete.insert(key);
        }
    }

    pub(super) fn knows(&self, id: &str, version: u64) -> bool {
        self.complete.contains(&(id.to_owned(), version))
    }

    /// Each version shown, with the bytes covered by slices (`None` when only
    /// whole reads were seen) and whether it was shown completely.
    pub(super) fn shown(&self, limit: usize) -> Vec<(String, u64, Option<usize>, bool)> {
        let keys: BTreeSet<_> = self.complete.iter().chain(self.ranges.keys()).collect();
        keys.into_iter()
            .take(limit)
            .map(|key| {
                let bytes = self.ranges.get(key).map(|spans| {
                    let (mut covered, mut end) = (0, 0);
                    for (from, to) in spans {
                        let from = (*from).max(end);
                        if *to > from {
                            covered += to - from;
                            end = *to;
                        }
                    }
                    covered
                });
                (key.0.clone(), key.1, bytes, self.complete.contains(key))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_count_only_when_they_cover_the_whole_note() {
        let mut ledger = ReadLedger::default();
        ledger.slice("n", 2, 0, 100, 250);
        assert!(!ledger.knows("n", 2));
        ledger.slice("n", 2, 150, 250, 250);
        assert!(!ledger.knows("n", 2), "a gap remains");
        ledger.slice("n", 2, 100, 150, 250);
        assert!(ledger.knows("n", 2));
        assert!(!ledger.knows("n", 3));
        ledger.slice("empty", 1, 0, 0, 0);
        assert!(ledger.knows("empty", 1));
        ledger.complete("whole", 5);
        assert!(ledger.knows("whole", 5) && !ledger.knows("whole", 4));
    }
}
