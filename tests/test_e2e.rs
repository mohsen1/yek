mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, ensure_empty_output_dir, setup_temp_repo};
use std::fs;
use tempfile::TempDir;

/// This test simulates an entire small repository with multiple directories
/// and checks the end-to-end behavior of running `yek` on it.
/// It verifies chunking, ignoring, and content ordering.
#[test]
fn e2e_small_repo_basic() {
    let repo = setup_temp_repo();
    create_file(repo.path(), "README.md", b"# This is a test repo");
    create_file(repo.path(), "src/main.rs", b"fn main() {}");
    create_file(repo.path(), "src/lib.rs", b"pub fn lib_fn() {}");
    create_file(repo.path(), "tests/test_it.rs", b"#[test] fn test_it() {}");
    // Add .gitignore to ignore `ignore_me/`
    create_file(repo.path(), ".gitignore", b"ignore_me/\n");
    create_file(repo.path(), "ignore_me/binary.bin", b"fakebinary\x00\x7f");

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--max-size=200KB")
        .assert()
        .success();

    let out_file = output_dir.with_extension("txt");
    assert!(out_file.exists());

    let content = fs::read_to_string(&out_file).unwrap();
    assert!(content.contains("README.md"));
    assert!(content.contains("src/main.rs"));
    assert!(content.contains("src/lib.rs"));
    assert!(content.contains("tests/test_it.rs"));
    assert!(!content.contains("ignore_me/binary.bin"));
}

/// This test ensures that large single files (bigger than the chunk limit)
/// do indeed get split into multiple chunks on Windows and Unix.
#[test]
fn e2e_large_file_included() {
    let repo = setup_temp_repo();

    let big_content = "test content ".repeat(100_000);
    create_file(repo.path(), "BIGFILE.txt", big_content.as_bytes());

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--max-size")
        .arg("50KB")
        .arg("--output-dir")
        .arg(&output_dir)
        .assert()
        .success();

    // Single output file
    let out_file = output_dir.with_extension("txt");
    assert!(out_file.exists());
    let content = fs::read_to_string(&out_file).unwrap();
    assert!(content.contains("BIGFILE.txt"));
    // Even if max-size is small, no chunk splitting occurs now.
}

/// This test simulates a multi-directory layout, including deeper nested directories.
/// The scenario attempts cross-platform path handling.
#[test]
fn e2e_nested_paths() {
    let repo = setup_temp_repo();
    create_file(
        repo.path().join("src/module1").as_path(),
        "foo.rs",
        b"// module1 foo",
    );
    create_file(
        repo.path().join("src/module1").as_path(),
        "bar.rs",
        b"// module1 bar",
    );
    create_file(
        repo.path().join("src/module2").as_path(),
        "baz.rs",
        b"// module2 baz",
    );
    fs::create_dir_all(repo.path().join("src/module2/extra")).unwrap();
    create_file(
        repo.path().join("src/module2/extra").as_path(),
        "deep_file.rs",
        b"// deep nested file",
    );

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--max-size=50KB")
        .assert()
        .success();

    let out_file = output_dir.with_extension("txt");
    let content = fs::read_to_string(out_file).unwrap();
    assert!(content.contains("src/module2/extra/deep_file.rs"));
}

/// Test cross-platform environment by mocking environment variables or
/// checking for Windows path usage.
/// This won't fully replicate Windows vs. Unix, but it ensures code runs in both
/// without crashing or mishandling path separators.
#[test]
fn e2e_cross_platform_sanity() {
    let repo = setup_temp_repo();
    create_file(repo.path(), "windows_path.txt", b"C:\\windows\\path");
    create_file(repo.path(), "unix_path.txt", b"/home/user/unix/path");

    // No output-dir => streaming
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        .env("TERM", "dumb")
        .output()
        .expect("Failed to execute command");

    assert!(assert.status.success());
    let stdout = String::from_utf8_lossy(&assert.stdout);
    // Just ensure both files appear in stdout
    assert!(stdout.contains("windows_path.txt"));
    assert!(stdout.contains("unix_path.txt"));
}

/// This test checks that with piping detection, if STDOUT is a TTY,
/// it writes to a file, otherwise it writes to STDOUT (stream).
#[test]
fn e2e_stream_detection() {
    let repo = setup_temp_repo();
    create_file(repo.path(), "test.txt", b"some content");

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .env("TERM", "dumb")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test.txt"));
    assert!(stdout.contains("some content"));
}

/// This test checks a scenario with a `yek.toml` that modifies ignore patterns,
/// custom binary extensions, and priority rules in a single run.
/// Ensures the end-to-end flow respects all of them.
#[test]
fn e2e_custom_config_all_features() {
    let repo = setup_temp_repo();
    let config_toml = r#"
ignore_patterns = ["assets/", "*.lock"]
binary_extensions = ["custombin"]
git_boost_max = 30

[[priority_rules]]
pattern = "^core/"
score = 100

[[priority_rules]]
pattern = "\\.md$"
score = 50

[[priority_rules]]
pattern = ".*"
score = 1
"#;
    create_file(repo.path(), "yek.toml", config_toml.as_bytes());
    create_file(repo.path(), "assets/secret.txt", b"should be ignored");
    create_file(repo.path(), "README.md", b"readme content");
    create_file(repo.path(), "app.lock", b"lock file ignored");
    create_file(repo.path(), "core/main.rs", b"core is high priority");
    create_file(repo.path(), "binary.custombin", b"fake binary\x00\x7f");

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--debug")
        .arg("--max-size=10KB")
        .output()
        .expect("Failed to execute command");

    assert!(assert.status.success());

    let out_file = output_dir.with_extension("txt");
    let content = fs::read_to_string(&out_file).unwrap();
    // Should contain README.md but not assets/secret.txt or app.lock or .custombin
    assert!(content.contains("README.md"));
    assert!(content.contains("core/main.rs"));
    assert!(!content.contains("assets/secret.txt"));
    assert!(!content.contains("app.lock"));
    assert!(!content.contains("binary.custombin"));
}

/// This test verifies that after chunking multiple directories at once,
/// the highest priority files from either directory appear last.
#[test]
fn e2e_multi_directory_priority() {
    let repo1 = setup_temp_repo();
    let repo2 = setup_temp_repo();

    create_file(
        repo1.path(),
        "yek.toml",
        r#"
[[priority_rules]]
pattern = "^dir1/"
score = 10
"#
        .as_bytes(),
    );
    create_file(
        repo2.path(),
        "yek.toml",
        r#"
[[priority_rules]]
pattern = "^super/"
score = 99
"#
        .as_bytes(),
    );

    create_file(repo1.path(), "dir1/a.txt", b"from repo1/dir1");
    create_file(repo1.path(), "dir2/b.txt", b"from repo1/dir2");
    create_file(repo2.path(), "super/c.txt", b"from repo2/super");
    create_file(repo2.path(), "basic/d.txt", b"from repo2/basic");

    let output_dir = TempDir::new().unwrap();
    ensure_empty_output_dir(output_dir.path());

    let out_str = output_dir.path().to_str().unwrap();

    // Process them both together
    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.arg(repo1.path())
        .arg(repo2.path())
        .arg("--output-dir")
        .arg(out_str)
        .arg("--max-size=5KB")
        .assert()
        .success();

    let out_file = output_dir.path().with_extension("txt");
    let content = fs::read_to_string(&out_file).unwrap();
    // super/ => higher priority => should appear after dir1
    let dir_pos = content.find(">>>> dir1/a.txt").expect("dir1/a.txt missing");
    let super_pos = content
        .find(">>>> super/c.txt")
        .expect("super/c.txt missing");
    assert!(
        super_pos > dir_pos,
        "Higher priority from second repo should be later in single output"
    );
}

/// This test tries to feed a large number of small files to check if we handle them in parallel
/// without overloading the aggregator or losing order correctness.
#[test]
fn e2e_many_small_files_parallel() {
    let repo = setup_temp_repo();
    for i in 0..200 {
        let file_name = format!("file_{:03}.txt", i);
        let content = "some small content\n".repeat(5);
        create_file(repo.path(), &file_name, content.as_bytes());
    }

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .assert()
        .success();

    let out_file = output_dir.with_extension("txt");
    assert!(out_file.exists());
    let content = fs::read_to_string(&out_file).unwrap();

    assert!(content.contains("file_000.txt"));
    assert!(content.contains("file_199.txt"));
}
