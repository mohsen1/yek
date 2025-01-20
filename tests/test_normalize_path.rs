use std::path::PathBuf;
use yek::normalize_path;

#[test]
fn test_normalize_path() {
    let base = PathBuf::from("/base/path");
    let path = PathBuf::from("/base/path/foo/bar.txt");
    assert_eq!(normalize_path(&base, &path), "foo/bar.txt");

    // Test with path not under base
    let other_path = PathBuf::from("/other/path/baz.txt");
    assert_eq!(normalize_path(&base, &other_path), "/other/path/baz.txt");

    // Test with relative paths
    let rel_base = PathBuf::from("base");
    let rel_path = PathBuf::from("base/foo/bar.txt");
    assert_eq!(normalize_path(&rel_base, &rel_path), "foo/bar.txt");

    // Test with current directory
    let current = PathBuf::from(".");
    assert_eq!(normalize_path(&base, &current), ".");

    // Test with Windows-style absolute path
    #[cfg(target_family = "windows")]
    {
        let win_path = PathBuf::from("C:\\other\\path\\baz.txt");
        assert_eq!(normalize_path(&base, &win_path), "/C:/other/path/baz.txt");

        let win_unc = PathBuf::from("\\\\server\\share\\file.txt");
        assert_eq!(normalize_path(&base, &win_unc), "//server/share/file.txt");

        // Test with forward slashes in UNC path
        let win_unc_fwd = PathBuf::from("//server/share/file.txt");
        assert_eq!(
            normalize_path(&base, &win_unc_fwd),
            "//server/share/file.txt"
        );

        // Test with mixed slashes in UNC path
        let win_unc_mixed = PathBuf::from("\\/server\\share/file.txt");
        assert_eq!(
            normalize_path(&base, &win_unc_mixed),
            "//server/share/file.txt"
        );
    }
}
