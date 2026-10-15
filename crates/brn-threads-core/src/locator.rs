//! One spelling for a local file supplied as a source. The same file can
//! arrive as `/tmp/x` and `/private/tmp/x`, or through a symlinked folder;
//! Source matching compares canonical spellings so it stays one Source.

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
}
