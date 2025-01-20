mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};
use std::fs;

/// Helper to run yek in streaming mode (pipe to stdout)
fn run_stream_mode(dir: &std::path::Path) -> String {
    let output = Command::cargo_bin("yek")
        .unwrap()
        .current_dir(dir)
        .env("TERM", "dumb") // Force non-interactive mode
        .env("NO_COLOR", "1") // Disable color output
        .env("CI", "1") // Force CI mode
        .output()
        .expect("Failed to execute command");

    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Helper to run yek in file mode (write to output directory)
fn run_file_mode(dir: &std::path::Path) -> String {
    let output_dir = dir.join("output");
    let _ = Command::cargo_bin("yek")
        .unwrap()
        .current_dir(dir)
        .arg("--output-dir")
        .arg(&output_dir)
        .assert()
        .success();

    // Read all chunk files
    let mut content = String::new();
    let read_dir = fs::read_dir(&output_dir).expect("Failed to read output directory");
    for entry in read_dir {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        content.push_str(
            &fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read file: {}", path.display())),
        );
    }
    content
}

#[test]
fn basic_gitignore_exclusion() {
    let repo = setup_temp_repo();

    // Setup test files
    create_file(repo.path(), ".gitignore", "ignore_me.txt\n");
    create_file(repo.path(), "ignore_me.txt", "should be ignored");
    create_file(repo.path(), "keep_me.txt", "should be kept");

    // Test both modes
    for content in [run_stream_mode(repo.path()), run_file_mode(repo.path())] {
        // Should exclude ignored file
        assert!(
            !content.contains("ignore_me.txt"),
            "Found ignored file in output: {content}"
        );

        // Should include kept file
        assert!(
            content.contains("keep_me.txt"),
            "Missing kept file in output: {content}"
        );
    }
}

#[test]
fn nested_gitignore_in_subdirectory() {
    let repo = setup_temp_repo();

    // Root gitignore
    create_file(repo.path(), ".gitignore", "*.temp\n");

    // Subdirectory with its own gitignore
    let sub_dir = repo.path().join("src");
    fs::create_dir_all(&sub_dir).unwrap();
    create_file(&sub_dir, ".gitignore", "secret.conf\n");
    create_file(&sub_dir, "secret.conf", "password=1234");
    create_file(&sub_dir, "app.rs", "fn main() {}");

    // Another subdir without gitignore
    let other_dir = repo.path().join("config");
    fs::create_dir_all(&other_dir).unwrap();
    create_file(&other_dir, "settings.temp", "key=value");

    for content in [run_stream_mode(repo.path()), run_file_mode(repo.path())] {
        // Should exclude nested gitignore entries
        assert!(
            !content.contains("secret.conf"),
            "Found nested gitignore file: {content}"
        );

        // Should exclude root gitignore pattern
        assert!(
            !content.contains("settings.temp"),
            "Found root gitignore pattern violation: {content}"
        );

        // Should keep valid files
        assert!(
            content.contains("app.rs"),
            "Missing valid source file: {content}"
        );
    }
}

#[test]
fn complex_ignore_patterns() {
    let repo = setup_temp_repo();

    create_file(
        repo.path(),
        ".gitignore",
        "
        # Comment
        *.log
        /build/
        temp/*
        !temp/keep.me
    ",
    );

    // Create test files
    create_file(repo.path(), "error.log", "logs");
    create_file(repo.path(), "build/output.exe", "binary");
    create_file(repo.path(), "temp/junk.tmp", "tmp");
    create_file(repo.path(), "temp/keep.me", "important");
    create_file(repo.path(), "src/main.rs", "fn main() {}");

    for content in [run_stream_mode(repo.path()), run_file_mode(repo.path())] {
        // Excluded patterns
        assert!(
            !content.contains("error.log"),
            "Found *.log file: {content}"
        );
        assert!(
            !content.contains("build/output.exe"),
            "Found build dir file: {content}"
        );
        assert!(
            !content.contains("temp/junk.tmp"),
            "Found temp/* file: {content}"
        );

        // Included exceptions
        assert!(
            content.contains("temp/keep.me"),
            "Missing !temp/keep.me: {content}"
        );
        assert!(
            content.contains("src/main.rs"),
            "Missing source file: {content}"
        );
    }
}

#[test]
fn combined_ignore_rules() {
    let repo = setup_temp_repo();

    // Main config
    create_file(
        repo.path(),
        "yek.toml",
        "
        [ignore_patterns]
        patterns = [\"^exclude/\"]
    ",
    );

    // Gitignore
    create_file(
        repo.path(),
        ".gitignore",
        "
        *.tmp
        /node_modules/
    ",
    );

    // Test files
    create_file(repo.path(), "exclude/secret.txt", "confidential");
    create_file(repo.path(), "test.tmp", "temporary");
    create_file(repo.path(), "node_modules/lib.js", "junk");
    create_file(repo.path(), "src/index.rs", "fn main() {}");

    for content in [run_stream_mode(repo.path()), run_file_mode(repo.path())] {
        // Should exclude both gitignore and config patterns
        assert!(
            !content.contains("exclude/secret.txt"),
            "Found excluded dir: {content}"
        );
        assert!(!content.contains("test.tmp"), "Found *.tmp file: {content}");
        assert!(
            !content.contains("node_modules/lib.js"),
            "Found node_modules: {content}"
        );

        // Should keep valid files
        assert!(
            content.contains("src/index.rs"),
            "Missing source file: {content}"
        );
    }
}

#[test]
fn binary_file_exclusion() {
    let repo = setup_temp_repo();

    // Create files without .gitignore
    create_file(repo.path(), "binary.jpg", "ÿØÿÛ"); // JPEG magic bytes
    create_file(repo.path(), "text.txt", "normal text");
    create_file(repo.path(), "unknown.xyz", "unknown format");

    for content in [run_stream_mode(repo.path()), run_file_mode(repo.path())] {
        // Should exclude known binary format
        assert!(
            !content.contains("binary.jpg"),
            "Found binary.jpg: {content}"
        );

        // Should include text files
        assert!(content.contains("text.txt"), "Missing text.txt: {content}");

        // Should include unknown.xyz since it's text content
        assert!(
            content.contains("unknown.xyz"),
            "Missing unknown.xyz which has text content: {content}"
        );
    }
}
