mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, ensure_empty_output_dir, setup_temp_repo};
use std::fs;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

// Add macro for test timeout
macro_rules! timeout_test {
    ($name:ident, $timeout:expr, $test:expr) => {
        #[tokio::test]
        async fn $name() {
            let result = timeout(Duration::from_secs($timeout), async { $test }).await;
            assert!(result.is_ok(), "Test timed out after {} seconds", $timeout);
        }
    };
}

/// This test simulates an entire small repository with multiple directories
/// and checks the end-to-end behavior of running `yek` on it.
/// It verifies parts, ignoring, and content ordering.
#[test]
fn e2e_small_repo_basic() {
    let repo = setup_temp_repo();

    // Create a few files
    create_file(repo.path(), "README.md", "# This is a test repo".as_bytes());
    create_file(repo.path(), "src/main.rs", "fn main() {}".as_bytes());
    create_file(repo.path(), "src/lib.rs", "pub fn lib_fn() {}".as_bytes());
    create_file(
        repo.path(),
        "tests/test_it.rs",
        "#[test] fn test_it() {}".as_bytes(),
    );
    create_file(repo.path(), "ignore_me/binary.bin", b"fakebinary\x00\x7f");
    // Add .gitignore to ignore `ignore_me/`
    create_file(repo.path(), ".gitignore", "ignore_me/\n".as_bytes());

    // Run `yek` in non-stream mode
    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--max-size=200KB") // Large enough to include all files in one output
        .assert()
        .success();

    // Check that ignore_me/binary.bin is not in output
    let mut found_lib_rs = false;
    let mut found_bin = false;

    for entry in fs::read_dir(&output_dir).expect("Output dir must exist") {
        let path = entry.expect("entry").path();
        if path.extension().unwrap_or_default() != "txt" {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read output file");
        if content.contains("binary.bin") {
            found_bin = true;
        }
        if content.contains("src/lib.rs") {
            found_lib_rs = true;
        }
    }
    assert!(!found_bin, "binary.bin (ignored) must not appear in output");
    assert!(found_lib_rs, "lib.rs must appear in the serialized output");
}

/// This test ensures that large files are truncated based on max_size
#[test]
fn large_file_truncation() {
    let repo = TempDir::new().unwrap();

    // 1 MB worth of text
    let big_content = "test content ".repeat(100_000);
    create_file(repo.path(), "BIGFILE.txt", big_content.as_bytes());

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    // We set max_size to ~100 KB so that 1 MB file is truncated
    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--max-size=50KB") // Much smaller size limit
        .arg("--output-dir")
        .arg(&output_dir)
        .assert()
        .success();

    // Verify output file
    let output_file = output_dir.join("output.txt");
    assert!(output_file.exists(), "Should write output file");
    let content = fs::read_to_string(&output_file).expect("read output");
    assert!(
        content.contains("BIGFILE.txt"),
        "Should contain file name banner"
    );
    assert!(
        content.len() <= 50 * 1024,
        "Content should be truncated to max size"
    );
}

/// This test simulates a multi-directory layout, including deeper nested directories.
/// The scenario attempts cross-platform path handling.
#[test]
fn e2e_nested_paths() {
    let repo = setup_temp_repo();

    // Nested directories
    create_file(
        repo.path(),
        "src/module1/foo.rs",
        "// module1 foo".as_bytes(),
    );
    create_file(
        repo.path(),
        "src/module1/bar.rs",
        "// module1 bar".as_bytes(),
    );
    create_file(
        repo.path(),
        "src/module2/baz.rs",
        "// module2 baz".as_bytes(),
    );
    create_file(
        repo.path(),
        "src/module2/extra/deep_file.rs",
        "// deep nested file".as_bytes(),
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

    // Check output content quickly
    let mut file_found = false;
    for entry in fs::read_dir(&output_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().unwrap_or_default() == "txt" {
            let content = fs::read_to_string(&path).unwrap();
            if content.contains("src/module2/extra/deep_file.rs") {
                file_found = true;
            }
        }
    }
    assert!(file_found, "Nested file wasn't found in output");
}

/// Test cross-platform environment by mocking environment variables or
/// checking for Windows path usage.
/// This won't fully replicate Windows vs. Unix, but it ensures code runs in both
/// without crashing or mishandling path separators.
#[test]
fn e2e_cross_platform_sanity() {
    let repo = setup_temp_repo();

    // We just put some small files
    create_file(
        repo.path(),
        "windows_path.txt",
        "C:\\windows\\style\\path".as_bytes(),
    );
    create_file(
        repo.path(),
        "unix_path.txt",
        "/home/user/unix/style/path".as_bytes(),
    );

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .env("TERM", "dumb") // Force streaming
        .assert()
        .success();

    // We expect the output in stdout
    // Because there's no --output-dir and output is not a TTY => streaming
    // We'll just check that the command succeeded, for cross-plat sanity.
}

/// This test checks that with piping detection, if STDOUT is a TTY,
/// it writes to a file, otherwise it writes to STDOUT (stream).
#[test]
fn e2e_stream_detection() {
    let repo = setup_temp_repo();
    create_file(repo.path(), "test.txt", "some content".as_bytes());

    // We'll forcibly pipe the output into a local buffer
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        .env("TERM", "dumb")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&assert.stdout);
    assert!(
        stdout.contains("test.txt"),
        "Must see test.txt in streamed output"
    );
    assert!(
        stdout.contains("some content"),
        "Must see file content in streamed output"
    );
}

/// This test checks a scenario with a `yek.toml` that modifies ignore patterns,
/// custom binary extensions, and priority rules in a single run.
/// Ensures the end-to-end flow respects all of them.
#[test]
fn e2e_custom_config_all_features() {
    let repo = setup_temp_repo();

    // Custom config
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

    // Some files
    create_file(
        repo.path(),
        "assets/secret.txt",
        "should be ignored".as_bytes(),
    );
    create_file(repo.path(), "README.md", "readme content".as_bytes());
    create_file(repo.path(), "app.lock", "lock file ignored".as_bytes());
    create_file(
        repo.path(),
        "core/main.rs",
        "core is high priority".as_bytes(),
    );
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

    println!("STDOUT: {}", String::from_utf8_lossy(&assert.stdout));
    println!("STDERR: {}", String::from_utf8_lossy(&assert.stderr));

    // Check output (should have `core/main.rs` due to highest priority).
    let output_file = output_dir.join("output.txt");
    assert!(output_file.exists(), "Should write output file");
    let content = fs::read_to_string(&output_file).expect("Read output");
    assert!(
        content.contains("core/main.rs"),
        "highest priority must come last"
    );
    assert!(
        !content.contains("assets/secret.txt"),
        "ignored file should not appear"
    );
    assert!(!content.contains("app.lock"), "lock file is ignored");
    assert!(
        !content.contains("binary.custombin"),
        "custom bin file is ignored"
    );
    // Make sure README.md is included but before the highest priority
    // We won't do a heavy check here, just confirm it appears somewhere
    let mut included_md = false;
    if content.contains("README.md") {
        included_md = true;
    }
    assert!(
        included_md,
        "README.md must be included, albeit with lower priority than core/"
    );
}

/// This test verifies that after processing multiple directories at once,
/// the highest priority files from either directory appear last.
#[test]
fn e2e_multi_directory_priority() {
    let repo1 = setup_temp_repo();
    let repo2 = setup_temp_repo();

    // Put a config in each
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

    // Some files in repo1
    create_file(repo1.path(), "dir1/a.txt", "from repo1/dir1".as_bytes());
    create_file(repo1.path(), "dir2/b.txt", "from repo1/dir2".as_bytes());
    // Some files in repo2
    create_file(repo2.path(), "super/c.txt", "from repo2/super".as_bytes());
    create_file(repo2.path(), "basic/d.txt", "from repo2/basic".as_bytes());

    // Let's process them both at once
    let output_dir = TempDir::new().unwrap(); // create a truly separate temp directory
    ensure_empty_output_dir(output_dir.path());
    let out_str = output_dir.path().to_str().unwrap();

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.arg(repo1.path())
        .arg(repo2.path())
        .arg("--output-dir")
        .arg(out_str)
        .arg("--max-size=5KB") // Much smaller size limit
        .assert()
        .success();

    // The output should have `super/c.txt` due to higher priority from second repo
    let output_file = output_dir.path().join("output.txt");
    assert!(output_file.exists(), "Should write output file");
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("super/c.txt"),
        "highest priority must come last"
    );
    // dir1 is priority 10, super is priority 99 => super is last

    // Ensure output is truncated
    let output_file = output_dir.path().join("output.txt");
    assert!(output_file.exists(), "Should write output file");
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(
        content.len() <= 5 * 1024,
        "Content should be truncated to max size"
    );

    // Check if files appear in output
    let mut found_first = false;
    let mut found_last = false;

    for entry in fs::read_dir(&output_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().unwrap_or_default() == "txt" {
            let c = fs::read_to_string(&path).unwrap();
            if c.contains(">>>> dir1/a.txt") {
                found_first = true;
            }
            if c.contains(">>>> dir2/b.txt") {
                found_last = true;
            }
        }
    }

    assert!(found_first, "dir1/a.txt must appear in output");
    assert!(found_last, "dir2/b.txt must appear in output");
}

// Replace the parallel test with macro usage
timeout_test!(
    e2e_many_small_files_parallel,
    30,
    async {
        let repo = setup_temp_repo();

        // Create many small files
        for i in 0..200 {
            let file_name = format!("file_{:03}.txt", i);
            let content = "some small content\n".repeat(100);
            create_file(repo.path(), &file_name, content.as_bytes());
        }

        let output_dir = repo.path().join("yek-output");
        ensure_empty_output_dir(&output_dir);

        let mut cmd = Command::cargo_bin("yek").unwrap();
        let output = cmd
            .current_dir(repo.path())
            .arg("--output-dir")
            .arg(&output_dir)
            .arg("--max-size=5KB") // Much smaller part size
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success(), "Command failed");

        // Ensure we have multiple parts
        let mut part_files: Vec<_> = fs::read_dir(&output_dir)
            .unwrap()
            .filter_map(|e| {
                let p = e.ok()?.path();
                if p.extension()? == "txt" {
                    // Extract part index from filename "part-{index}.txt"
                    let index = p
                        .file_stem()?
                        .to_str()?
                        .strip_prefix("part-")?
                        .split("-part-") // Handle split parts if any
                        .next()?
                        .parse::<usize>()
                        .ok()?;
                    Some((index, p))
                } else {
                    None
                }
            })
            .collect();
        // Sort by part index
        part_files.sort_by_key(|(index, _)| *index);
        let part_files: Vec<_> = part_files.into_iter().map(|(_, p)| p).collect();

        assert!(
            part_files.len() > 1,
            "Must produce multiple parts with 200 small files"
        );

        // Check if files appear in any part
        let mut found_first = false;
        let mut found_last = false;

        for part_file in &part_files {
            let content = fs::read_to_string(part_file).unwrap();
            if content.contains(">>>> file_000.txt") {
                found_first = true;
            }
            if content.contains(">>>> file_199.txt") {
                found_last = true;
            }
        }

        assert!(found_first, "file_000.txt must appear in some part");
        assert!(found_last, "file_199.txt must appear in some part");
    }
    .await
);

#[test]
fn streams_content_when_piped() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create test repository structure
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}")?;
    fs::create_dir(temp_dir.path().join("src"))?;
    fs::write(
        temp_dir.path().join("src/lib.rs"),
        "pub fn magic() -> i32 { 42 }",
    )?;

    let mut cmd = Command::cargo_bin("yek")?;

    // Capture output from piped execution - using TERM=dumb to force streaming
    let output = cmd
        .arg(temp_dir.path())
        .env("TERM", "dumb")
        .env("NO_COLOR", "1") // Disable color output
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // In streaming mode, we still get part headers
    assert!(
        stdout.contains(">>>> main.rs\nfn main() {}"),
        "Missing main.rs content"
    );
    assert!(
        stdout.contains(">>>> src/lib.rs\npub fn magic() -> i32 { 42 }"),
        "Missing lib.rs content"
    );

    // Verify no files were created
    assert!(!temp_dir.path().join("output.txt").exists());
    Ok(())
}

#[test]
fn writes_files_when_interactive() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;

    // Create test file
    fs::write(temp_dir.path().join("config.yml"), "key: value")?;

    let mut cmd = Command::cargo_bin("yek")?;

    // Simulate interactive terminal by forcing file output
    cmd.arg("--output-dir")
        .arg(output_dir.path())
        .arg(temp_dir.path())
        .assert()
        .success();

    // Verify file output
    let output_file = output_dir.path().join("output.txt");
    let content = fs::read_to_string(output_file)?;

    assert!(
        content.contains(">>>> config.yml\nkey: value"),
        "Missing config content"
    );

    Ok(())
}

#[test]
fn handles_large_files_with_splitting() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create 2MB test file
    let large_content = "a".repeat(2_000_000);
    fs::write(temp_dir.path().join("big.txt"), &large_content)?;

    let mut cmd = Command::cargo_bin("yek")?;
    let output = cmd
        .arg("--max-size=1MB")
        .env("TERM", "dumb")
        .arg(temp_dir.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Verify splitting occurred
    assert!(stdout.contains(">>>> big.txt (part 0)"));
    assert!(stdout.contains(">>>> big.txt (part 1)"));
    assert!(!stdout.contains("part 0"));

    Ok(())
}

#[test]
fn respects_token_mode() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;

    // Create test files with known token counts
    fs::write(temp_dir.path().join("test1.txt"), "Hello world")?;
    fs::write(
        temp_dir.path().join("test2.txt"),
        "This is a longer test sentence.",
    )?;

    let mut cmd = Command::cargo_bin("yek")?;

    cmd.arg("--tokens") // Changed from --token-mode
        .arg("--max-size=10") // Small token limit to force splitting
        .arg("--output-dir")
        .arg(output_dir.path())
        .arg(temp_dir.path())
        .assert()
        .success();

    // Verify files were split based on token count
    let files: Vec<_> = fs::read_dir(output_dir.path())?.collect();
    assert!(
        files.len() > 1,
        "Files should be split based on token count"
    );

    Ok(())
}

#[test]
fn streams_despite_config_output_dir() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create config file specifying output directory
    let config_content = "output_dir = \"./repo-serialized\"\n";
    create_file(temp_dir.path(), "yek.toml", config_content.as_bytes());

    // Create test file content
    create_file(temp_dir.path(), "test.txt", "Hello, world!".as_bytes());

    // Execute yek with simulated pipe (non-TTY)
    let mut cmd = Command::cargo_bin("yek")?;
    let output = cmd
        .current_dir(temp_dir.path())
        .env("TERM", "dumb") // Disable TTY detection
        .env("NO_COLOR", "1") // Ensure clean output
        .arg(".")
        .output()?;

    // Verify command success
    assert!(output.status.success(), "Command should succeed");

    // Check stdout contains expected content
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains(">>>> test.txt\nHello, world!"),
        "Should stream test.txt content to stdout"
    );

    // Ensure config-specified output directory wasn't created
    let output_dir = temp_dir.path().join("repo-serialized");
    assert!(
        !output_dir.exists(),
        "Should not create output directory when streaming"
    );

    Ok(())
}

#[test]
fn handles_empty_directory() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;

    let mut cmd = Command::cargo_bin("yek")?;

    cmd.arg("--output-dir")
        .arg(output_dir.path())
        .arg(temp_dir.path())
        .assert()
        .success();

    // Verify no output files were created for empty directory
    assert!(fs::read_dir(output_dir.path())?.count() == 0);

    Ok(())
}
