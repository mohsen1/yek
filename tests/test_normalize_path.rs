use std::path::Path;
use yek::normalize_path_with_root;

#[test]
fn test_normalize_with_base() {
    let base = Path::new("/base/dir");
    let path = base.join("foo/bar.txt");
    let other_path = Path::new("/other/path/baz.txt");

    assert_eq!(normalize_path_with_root(&path, base), "foo/bar.txt");
    assert_eq!(
        normalize_path_with_root(other_path, base),
        "/other/path/baz.txt"
    );
}

#[test]
fn test_normalize_relative_paths() {
    let rel_base = Path::new("some/relative/dir");
    let rel_path = rel_base.join("foo/bar.txt");

    assert_eq!(normalize_path_with_root(&rel_path, rel_base), "foo/bar.txt");
}

#[test]
fn test_normalize_current_dir() {
    let current = Path::new(".");
    assert_eq!(normalize_path_with_root(current, current), ".");
}
