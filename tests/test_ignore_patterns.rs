mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};
use std::fs;

#[test]
fn test_ignore_patterns_basic() {
    let repo = setup_temp_repo();
    create_file(
        repo.path(),
        "yek.toml",
        r#"
ignore_patterns = [
    "^ignore_me/",
    "\\.tmp$"
]
"#
        .as_bytes(),
    );

    create_file(repo.path(), "ignore_me/secret.txt", b"should be ignored");
    create_file(repo.path(), "keep_me/public.txt", b"should be kept");
    create_file(repo.path(), "temp.tmp", b"should be ignored");

    let output_dir = repo.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .assert()
        .success();

    let out_file = output_dir.with_extension("txt");
    let content = fs::read_to_string(&out_file).unwrap();

    assert!(!content.contains(">>>> ignore_me/secret.txt"));
    assert!(content.contains(">>>> keep_me/public.txt"));
    assert!(!content.contains(">>>> temp.tmp"));
}

#[test]
fn test_ignore_patterns_regex() {
    let repo = setup_temp_repo();
    create_file(
        repo.path(),
        "yek.toml",
        r#"
ignore_patterns = [
    "^test_[0-9]+\\.txt$",
    ".*\\.bak$"
]
"#
        .as_bytes(),
    );

    create_file(repo.path(), "test_123.txt", b"should be ignored");
    create_file(repo.path(), "test.txt", b"should be kept");
    create_file(repo.path(), "file.bak", b"should be ignored");

    let output_dir = repo.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .assert()
        .success();

    let out_file = output_dir.with_extension("txt");
    let content = fs::read_to_string(&out_file).unwrap();

    assert!(!content.contains(">>>> test_123.txt"));
    assert!(content.contains(">>>> test.txt"));
    assert!(!content.contains(">>>> file.bak"));
}

#[test]
fn test_ignore_patterns_nested() {
    let repo = setup_temp_repo();
    create_file(
        repo.path(),
        "yek.toml",
        r#"
ignore_patterns = [
    "^src/.*\\.bak$",
    "^test/temp/.*$"
]
"#
        .as_bytes(),
    );

    create_file(
        repo.path().join("src").as_path(),
        "main.rs.bak",
        b"should be ignored",
    );
    create_file(
        repo.path().join("src").as_path(),
        "main.rs",
        b"should be kept",
    );
    create_file(
        repo.path().join("test/temp").as_path(),
        "file.txt",
        b"should be ignored",
    );
    create_file(
        repo.path().join("test").as_path(),
        "test.txt",
        b"should be kept",
    );

    let output_dir = repo.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .assert()
        .success();

    let out_file = output_dir.with_extension("txt");
    let content = fs::read_to_string(&out_file).unwrap();

    assert!(!content.contains(">>>> src/main.rs.bak"));
    assert!(content.contains(">>>> src/main.rs"));
    assert!(!content.contains(">>>> test/temp/file.txt"));
    assert!(content.contains(">>>> test/test.txt"));
}

#[test]
fn test_ignore_patterns_with_gitignore() {
    let repo = setup_temp_repo();
    create_file(
        repo.path(),
        "yek.toml",
        r#"
ignore_patterns = [
    "^custom_ignore/.*$"
]
"#
        .as_bytes(),
    );
    create_file(repo.path(), ".gitignore", b"*.tmp\n");

    create_file(repo.path(), "custom_ignore/file.txt", b"should be ignored");
    create_file(repo.path(), "temp.tmp", b"should be ignored");
    create_file(repo.path(), "keep.txt", b"should be kept");

    let output_dir = repo.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .assert()
        .success();

    let out_file = output_dir.with_extension("txt");
    let content = fs::read_to_string(&out_file).unwrap();

    assert!(!content.contains(">>>> custom_ignore/file.txt"));
    assert!(!content.contains(">>>> temp.tmp"));
    assert!(content.contains(">>>> keep.txt"));
}

#[test]
fn test_ignore_patterns_case_sensitivity() {
    let repo = setup_temp_repo();
    create_file(
        repo.path(),
        "yek.toml",
        r#"
ignore_patterns = [
    "^IGNORE/.*$",
    ".*\\.TMP$"
]
"#
        .as_bytes(),
    );

    create_file(repo.path(), "IGNORE/file.txt", b"should be ignored");
    create_file(repo.path(), "ignore/file.txt", b"should be kept");
    create_file(repo.path(), "test.TMP", b"should be ignored");
    create_file(repo.path(), "test.tmp", b"should be kept");

    let output_dir = repo.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .assert()
        .success();

    let out_file = output_dir.with_extension("txt");
    let content = fs::read_to_string(&out_file).unwrap();

    assert!(!content.contains(">>>> IGNORE/file.txt"));
    assert!(content.contains(">>>> ignore/file.txt"));
    assert!(!content.contains(">>>> test.TMP"));
    assert!(content.contains(">>>> test.tmp"));
}
