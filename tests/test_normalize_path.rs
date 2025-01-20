use std::path::PathBuf;
use yek::normalize_path;

#[test]
fn test_normalize_path() {
    let base = PathBuf::from("/base/path");
    let path = PathBuf::from("/base/path/foo/bar.txt");
    assert_eq!(normalize_path(&path, &base), "foo/bar.txt");

    // Test with path not under base
    let other_path = PathBuf::from("/other/path/baz.txt");
    assert_eq!(normalize_path(&other_path, &base), "/other/path/baz.txt");

    // Test with relative paths
    let rel_base = PathBuf::from("base");
    let rel_path = PathBuf::from("base/foo/bar.txt");
    assert_eq!(normalize_path(&rel_path, &rel_base), "foo/bar.txt");

    // Test with current directory
    let current = PathBuf::from(".");
    assert_eq!(normalize_path(&current, &current), ".");

    // Test with Windows-style absolute path
    #[cfg(target_family = "windows")]
    {
        let win_path = PathBuf::from("C:\\other\\path\\baz.txt");
        assert_eq!(normalize_path(&win_path, &base), "C:/other/path/baz.txt");

        let win_unc = PathBuf::from("\\\\server\\share\\file.txt");
        assert_eq!(normalize_path(&win_unc, &base), "//server/share/file.txt");

        // Test with forward slashes in UNC path
        let win_unc_fwd = PathBuf::from("//server/share/file.txt");
        assert_eq!(
            normalize_path(&win_unc_fwd, &base),
            "//server/share/file.txt"
        );

        // Test with mixed slashes in UNC path
        let win_unc_mixed = PathBuf::from("\\/server\\share/file.txt");
        assert_eq!(
            normalize_path(&win_unc_mixed, &base),
            "//server/share/file.txt"
        );
    }
}
