//! One spelling for a local file supplied as a source. The same file can
//! arrive as `/tmp/x` and `/private/tmp/x`, or through a symlinked folder;
//! Source matching compares canonical spellings so it stays one Source.
use crate::{Record, RecordData, Source};
use std::collections::BTreeSet;

/// The real path of the existing local regular file an absolute `locator`
/// names, on Unix. Every other locator is returned unchanged: URLs,
/// `synthetic://` and other schemes, relative, missing, non-UTF-8 and
/// directory paths.
pub fn canonical_locator(locator: &str) -> String {
    if cfg!(unix)
        && locator.starts_with('/')
        && let Ok(real) = std::fs::canonicalize(locator)
        && real.is_file()
        && let Some(real) = real.to_str()
    {
        return real.to_owned();
    }
    locator.to_owned()
}

/// Whether a `stored` Source locator names the same source as `canonical`,
/// a [`canonical_locator`] result. A Source saved under an older spelling of
/// the same file still matches.
pub fn same_locator(stored: &str, canonical: &str) -> bool {
    stored == canonical
        || canonical.starts_with('/')
            && stored.starts_with('/')
            && canonical_locator(stored) == canonical
}

/// The Sources a host recorded: supplied to a Run, or the source of an import
/// or a figure. A model may name other Sources. Only these are resolved on
/// the filesystem when an older spelling of a path is looked for, so a path
/// only a model named is never touched. Empty for a non-path locator.
fn older_spelling_candidates<'a>(records: &'a [Record], canonical: &str) -> BTreeSet<&'a str> {
    let mut ids = BTreeSet::new();
    if !canonical.starts_with('/') {
        return ids;
    }
    for record in records {
        match &record.data {
            RecordData::Run(run) => ids.extend(run.sources.iter().map(String::as_str)),
            RecordData::Note(note) => ids.extend(note.import.as_ref().map(|i| i.source.as_str())),
            RecordData::Asset(asset) => ids.extend(asset.source.as_deref()),
            _ => {}
        }
    }
    ids
}

/// The Source that `scope` admits for the file at `canonical`, a
/// [`canonical_locator`] result: the exact spelling first, then a
/// host-recorded Source saved under an older spelling of the same file.
pub fn find_source<'a>(
    records: &'a [Record],
    canonical: &str,
    scope: impl Fn(&Source) -> bool,
) -> Option<&'a Record> {
    let admitted = |record: &Record, same: fn(&str, &str) -> bool| {
        matches!(&record.data, RecordData::Source(source)
            if scope(source) && same(&source.locator, canonical))
    };
    records
        .iter()
        .find(|record| admitted(record, |stored, canonical| stored == canonical))
        .or_else(|| {
            let candidates = older_spelling_candidates(records, canonical);
            records.iter().find(|record| {
                candidates.contains(record.id.as_str()) && admitted(record, same_locator)
            })
        })
}

/// The IDs of every Source for the file at `canonical`, in any scope: exact
/// spellings and host-recorded older spellings.
pub fn matching_sources<'a>(records: &'a [Record], canonical: &str) -> BTreeSet<&'a str> {
    let candidates = older_spelling_candidates(records, canonical);
    records
        .iter()
        .filter(|record| {
            matches!(&record.data, RecordData::Source(source)
                if source.locator == canonical
                    || candidates.contains(record.id.as_str()) && same_locator(&source.locator, canonical))
        })
        .map(|record| record.id.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> tempfile::TempDir {
        tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
    }

    #[test]
    fn non_file_locators_are_unchanged() {
        let root = root();
        let folder = root.path().to_str().unwrap();
        for locator in [
            "https://example.invalid/mail.txt",
            "synthetic://mari-crane-email",
            "outlook:message/1",
            "relative/mail.txt",
            "/missing/brn-locator-test/mail.txt",
            folder,
        ] {
            assert_eq!(canonical_locator(locator), locator);
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_file_through_a_symlinked_folder_maps_to_its_real_path() {
        let root = root();
        let real = root.path().join("real");
        std::fs::create_dir(&real).unwrap();
        std::fs::write(real.join("mail.txt"), "synthetic").unwrap();
        std::os::unix::fs::symlink(&real, root.path().join("alias")).unwrap();
        let alias = root.path().join("alias/mail.txt");
        let real = real.join("mail.txt");
        assert_eq!(
            canonical_locator(alias.to_str().unwrap()),
            real.to_str().unwrap()
        );
        assert_eq!(
            canonical_locator(real.to_str().unwrap()),
            real.to_str().unwrap()
        );
        // A folder is not a file source and keeps its spelling.
        let folder = root.path().join("alias");
        assert_eq!(
            canonical_locator(folder.to_str().unwrap()),
            folder.to_str().unwrap()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn tmp_maps_to_private_tmp() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let real = dir.path().join("mail.txt");
        std::fs::write(&real, "synthetic").unwrap();
        let real = real.to_str().unwrap();
        let tmp = real.replacen("/private/tmp/", "/tmp/", 1);
        assert_eq!(canonical_locator(&tmp), real);
    }

    #[cfg(unix)]
    #[test]
    fn an_older_spelling_matches_and_another_file_with_the_same_name_does_not() {
        let root = root();
        let (a, b) = (root.path().join("a"), root.path().join("b"));
        for dir in [&a, &b] {
            std::fs::create_dir(dir).unwrap();
            std::fs::write(dir.join("mail.txt"), "synthetic").unwrap();
        }
        std::os::unix::fs::symlink(&a, root.path().join("alias")).unwrap();
        let alias = root.path().join("alias/mail.txt");
        let canonical = canonical_locator(a.join("mail.txt").to_str().unwrap());
        assert!(same_locator(alias.to_str().unwrap(), &canonical));
        assert!(same_locator(&canonical, &canonical));
        assert!(!same_locator(
            b.join("mail.txt").to_str().unwrap(),
            &canonical
        ));
        assert!(!same_locator("synthetic://mail.txt", &canonical));
        assert!(same_locator("synthetic://mail", "synthetic://mail"));
    }

    fn record(id: &str, data: RecordData) -> Record {
        Record {
            id: id.into(),
            version: 1,
            archived: false,
            data,
        }
    }

    fn source(id: &str, locator: &str) -> Record {
        record(
            id,
            RecordData::Source(Source {
                chat: None,
                locator: locator.into(),
                label: locator.into(),
                observed_at: "unix:1".into(),
                external_version: None,
                outcome: String::new(),
                gaps: vec![],
                report: None,
            }),
        )
    }

    #[cfg(unix)]
    #[test]
    fn only_a_host_recorded_source_matches_under_an_older_spelling() {
        let root = root();
        let real = root.path().join("real");
        std::fs::create_dir(&real).unwrap();
        std::fs::write(real.join("mail.txt"), "synthetic").unwrap();
        std::os::unix::fs::symlink(&real, root.path().join("alias")).unwrap();
        let alias = root.path().join("alias/mail.txt");
        let (alias, real) = (
            alias.to_str().unwrap(),
            real.join("mail.txt").to_str().unwrap().to_owned(),
        );
        // A Source only a model named, under the alias spelling.
        let mut records = vec![source("model-named", alias)];
        assert!(find_source(&records, &real, |_| true).is_none());
        assert!(matching_sources(&records, &real).is_empty());
        // The same Source once an import names it.
        let mut note = crate::Note::working("Mail", "synthetic");
        note.import = Some(crate::ImportCoverage {
            source: "model-named".into(),
            full_note: true,
            gaps: vec![],
            report: None,
        });
        records.push(record("import", RecordData::Note(note)));
        assert_eq!(
            find_source(&records, &real, |_| true).map(|r| r.id.as_str()),
            Some("model-named")
        );
        assert!(find_source(&records, &real, |s| s.chat.is_some()).is_none());
        // An exact spelling wins and needs no host record.
        records.insert(0, source("exact", &real));
        assert_eq!(
            find_source(&records[..1], &real, |_| true).map(|r| r.id.as_str()),
            Some("exact")
        );
        records.rotate_left(1);
        assert_eq!(
            find_source(&records, &real, |_| true).map(|r| r.id.as_str()),
            Some("exact")
        );
        assert_eq!(
            matching_sources(&records, &real),
            BTreeSet::from(["exact", "model-named"])
        );
    }
}
