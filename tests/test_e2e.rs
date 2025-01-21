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
        .arg("--max-size=200K") // Large enough to include all files in one chunk
        .assert()
        .success();

    // Check that ignore_me/binary.bin is not in any output chunk
    let mut found_lib_rs = false;
    let mut found_bin = false;

    for entry in fs::read_dir(&output_dir).expect("Output dir must exist") {
        let path = entry.expect("entry").path();
        if path.extension().unwrap_or_default() != "txt" {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read chunk file");
        if content.contains("binary.bin") {
            found_bin = true;
        }
        if content.contains("src/lib.rs") {
            found_lib_rs = true;
        }
    }
    assert!(!found_bin, "binary.bin (ignored) must not appear in chunks");
    assert!(found_lib_rs, "lib.rs must appear in the serialized output");
}

/// This test ensures that large single files (bigger than the chunk limit)
/// do indeed get split into multiple chunks on Windows and Unix.
#[test]
fn e2e_large_file_splitting() {
    let repo = TempDir::new().unwrap();

    // 1 MB worth of text
    let big_content = "test content ".repeat(100_000);
    create_file(repo.path(), "BIGFILE.txt", big_content.as_bytes());

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    // We set chunk limit to ~100 KB so that 1 MB file is forced into ~10 parts
    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--max-size=50K") // Much smaller chunk size
        .arg("--output-dir")
        .arg(&output_dir)
        .assert()
        .success();

    // Verify multiple chunk files
    let mut chunk_count = 0;
    println!("Output directory: {:?}", output_dir);
    for entry in fs::read_dir(&output_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().unwrap_or_default() == "txt" {
            chunk_count += 1;
            let content = fs::read_to_string(&path).expect("read chunk");
            // Only print first 100 chars of content
            println!(
                "Chunk {}: {} ...",
                chunk_count,
                &content.chars().take(100).collect::<String>()
            );
            assert!(
                content.contains("BIGFILE.txt") && content.contains("chunk"),
                "Each chunk should show the file name banner"
            );
        }
    }
    assert!(
        chunk_count > 1,
        "Should produce multiple chunks for a large file"
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
        .arg("--max-size=50K")
        .assert()
        .success();

    // Check chunk content quickly
    let mut chunk_found = false;
    for entry in fs::read_dir(&output_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().unwrap_or_default() == "txt" {
            let content = fs::read_to_string(&path).unwrap();
            if content.contains("src/module2/extra/deep_file.rs") {
                chunk_found = true;
            }
        }
    }
    assert!(chunk_found, "Nested file wasn't found in output");
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
        .arg("--max-size=10K")
        .output()
        .expect("Failed to execute command");

    println!("STDOUT: {}", String::from_utf8_lossy(&assert.stdout));
    println!("STDERR: {}", String::from_utf8_lossy(&assert.stderr));

    // Check final chunk (should have `core/main.rs` due to highest priority).
    let entries = fs::read_dir(&output_dir).unwrap();
    let mut chunk_files: Vec<_> = entries
        .filter_map(|e| {
            let p = e.ok()?.path();
            (p.extension()? == "txt").then_some(p)
        })
        .collect();

    chunk_files.sort(); // chunk-0.txt, chunk-1.txt, ...
    let last_chunk = chunk_files.last().expect("Must have at least one chunk");
    let content = fs::read_to_string(last_chunk).expect("Read last chunk");
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
    for file in &chunk_files {
        let c = fs::read_to_string(file).unwrap();
        if c.contains("README.md") {
            included_md = true;
            break;
        }
    }
    assert!(
        included_md,
        "README.md must be included, albeit with lower priority than core/"
    );
}

/// This test verifies that after chunking multiple directories at once,
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
        .arg("--max-size=5K")
        .assert()
        .success();

    // The last chunk should have `super/c.txt` due to higher priority from second repo
    let mut chunk_files: Vec<_> = fs::read_dir(&output_dir)
        .unwrap()
        .filter_map(|e| {
            let p = e.ok()?.path();
            (p.extension()? == "txt").then_some(p)
        })
        .collect();
    chunk_files.sort();

    let last_chunk = chunk_files.last().expect("need at least one chunk");
    let content = fs::read_to_string(last_chunk).unwrap();
    assert!(
        content.contains("super/c.txt"),
        "highest priority must come last"
    );
    // dir1 is priority 10, super is priority 99 => super is last
}

/// This test tries to feed a large number of small files to check if we handle them in parallel
/// without overloading the aggregator or losing order correctness.
#[test]
fn e2e_many_small_files_parallel() {
    let repo = setup_temp_repo();

    // Create many small files
    for i in 0..200 {
        let file_name = format!("file_{:03}.txt", i);
        let content = "some small content\n".repeat(100);
        create_file(repo.path(), &file_name, content.as_bytes());
    }

    // We rely on environment CPU cores for parallel chunk creation
    // Then confirm all files appear in the final output
    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--max-size=5K") // Much smaller chunk size
        .assert()
        .success();

    // Ensure we have multiple chunks
    let mut chunk_files: Vec<_> = fs::read_dir(&output_dir)
        .unwrap()
        .filter_map(|e| {
            let p = e.ok()?.path();
            if p.extension()? == "txt" {
                // Extract chunk index from filename "chunk-{index}.txt"
                let index = p
                    .file_stem()?
                    .to_str()?
                    .strip_prefix("chunk-")?
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
    // Sort by chunk index
    chunk_files.sort_by_key(|(index, _)| *index);
    let chunk_files: Vec<_> = chunk_files.into_iter().map(|(_, p)| p).collect();

    assert!(
        chunk_files.len() > 1,
        "Must produce multiple chunks with 200 small files"
    );

    // Check if files appear in any chunk
    let mut found_first = false;
    let mut found_last = false;

    for chunk_file in &chunk_files {
        let content = fs::read_to_string(chunk_file).unwrap();
        if content.contains(">>>> file_000.txt") {
            found_first = true;
        }
        if content.contains(">>>> file_199.txt") {
            found_last = true;
        }
    }

    assert!(found_first, "file_000.txt must appear in some chunk");
    assert!(found_last, "file_199.txt must appear in some chunk");
}
