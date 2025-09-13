use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use yek::tree::{clean_path_components, generate_tree};

#[cfg(test)]
mod tree_tests {
    use super::*;

    fn create_test_structure(base_dir: &Path) -> std::io::Result<()> {
        // Create nested directory structure
        fs::create_dir_all(base_dir.join("src"))?;
        fs::create_dir_all(base_dir.join("tests"))?;
        fs::create_dir_all(base_dir.join("docs/guides"))?;

        // Create files
        fs::write(base_dir.join("config.py"), "# Config file\n")?;
        fs::write(base_dir.join("Cargo.toml"), "[package]\nname = \"test\"\n")?;
        fs::write(base_dir.join("src/main.rs"), "fn main() {}\n")?;
        fs::write(base_dir.join("src/lib.rs"), "// Library code\n")?;
        fs::write(base_dir.join("tests/test.rs"), "#[test]\nfn test() {}\n")?;
        fs::write(base_dir.join("docs/api.py"), "# API Documentation\n")?;
        fs::write(base_dir.join("docs/guides/setup.py"), "# Setup Guide\n")?;

        Ok(())
    }

    #[test]
    fn test_tree_header_basic() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(temp_dir.path()).unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-header")
            .arg("--max-size")
            .arg("1KB")
            .arg(temp_dir.path());

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Directory structure:"))
            .stdout(predicate::str::contains("├── src/"))
            .stdout(predicate::str::contains("│   ├── lib.rs"))
            .stdout(predicate::str::contains("│   └── main.rs"))
            .stdout(predicate::str::contains("├── tests/"))
            .stdout(predicate::str::contains("├── Cargo.toml"))
            .stdout(predicate::str::contains("└── config.py"))
            .stdout(predicate::str::contains(">>>> "));
    }

    #[test]
    fn test_tree_only_mode() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(temp_dir.path()).unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-only").arg(temp_dir.path());

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Directory structure:"))
            .stdout(predicate::str::contains("├── docs/"))
            .stdout(predicate::str::contains("│   ├── guides/"))
            .stdout(predicate::str::contains("│   │   └── setup.py"))
            .stdout(predicate::str::contains("│   └── api.py"))
            .stdout(predicate::str::contains("├── src/"))
            .stdout(predicate::str::contains(">>>> ").not())
            .stdout(predicate::str::contains("fn main()").not());
    }

    #[test]
    fn test_tree_header_short_flag() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "content").unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("-t").arg(temp_dir.path());

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Directory structure:"))
            .stdout(predicate::str::contains("└── test.rs"));
    }

    #[test]
    fn test_tree_mutual_exclusivity() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "content").unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-header")
            .arg("--tree-only")
            .arg(temp_dir.path());

        cmd.assert().failure().stderr(predicate::str::contains(
            "tree_header and tree_only cannot both be enabled",
        ));
    }

    #[test]
    fn test_tree_with_single_file() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("single.rs"), "// Single file\n").unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-header")
            .arg(temp_dir.path().join("single.rs"));

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Directory structure:"))
            .stdout(predicate::str::contains("└── single.rs"))
            .stdout(predicate::str::contains(">>>> single.rs"))
            .stdout(predicate::str::contains("// Single file"));
    }

    #[test]
    fn test_tree_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("empty")).unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-only").arg(temp_dir.path());

        let output = cmd.assert().success();
        let stdout = std::str::from_utf8(&output.get_output().stdout).unwrap();

        // For empty directories, tree-only should produce empty content
        // Since this runs in streaming mode (no files to process), it should be empty or just whitespace
        assert!(
            stdout.trim().is_empty(),
            "Expected empty output for empty directory, got: '{}'",
            stdout
        );
    }

    #[test]
    fn test_tree_with_ignored_patterns() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(temp_dir.path()).unwrap();

        // Create additional files that should be ignored
        fs::create_dir_all(temp_dir.path().join("node_modules")).unwrap();
        fs::write(temp_dir.path().join("node_modules/package.json"), "{}").unwrap();
        fs::write(temp_dir.path().join("Cargo.lock"), "lock file").unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-only").arg(temp_dir.path());

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Directory structure:"))
            .stdout(predicate::str::contains("node_modules").not())
            .stdout(predicate::str::contains("Cargo.lock").not());
    }

    #[test]
    fn test_tree_header_with_json_output() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "content").unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-header").arg("--json").arg(temp_dir.path());

        cmd.assert().failure().stderr(predicate::str::contains(
            "JSON output not supported with tree header mode",
        ));
    }

    #[test]
    fn test_tree_only_with_json_output() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "content").unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-only").arg("--json").arg(temp_dir.path());

        cmd.assert().failure().stderr(predicate::str::contains(
            "JSON output not supported in tree-only mode",
        ));
    }

    #[test]
    fn test_tree_header_with_token_mode() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("small.rs"), "small content").unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-header")
            .arg("--tokens")
            .arg("100")
            .arg(temp_dir.path());

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Directory structure:"))
            .stdout(predicate::str::contains("└── small.rs"));
    }

    #[test]
    fn test_tree_respects_max_size() {
        let temp_dir = TempDir::new().unwrap();
        let large_content = "x".repeat(2000);
        fs::write(temp_dir.path().join("large.rs"), &large_content).unwrap();
        fs::write(temp_dir.path().join("small.rs"), "small").unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-header")
            .arg("--max-size")
            .arg("1KB")
            .arg(temp_dir.path());

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Directory structure:"))
            .stdout(predicate::str::contains("├── ").or(predicate::str::contains("└── ")));
    }

    #[test]
    fn test_tree_header_cli_flag() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.py"), "content").unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-header")
            .arg("--max-size")
            .arg("1KB")
            .arg(temp_dir.path());

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Directory structure:"))
            .stdout(predicate::str::contains("test.py"))
            .stdout(predicate::str::contains(">>>> test.py"));
    }

    #[test]
    fn test_tree_directory_sorting() {
        let temp_dir = TempDir::new().unwrap();

        // Create files and directories in non-alphabetical order
        fs::write(temp_dir.path().join("zebra.rs"), "content").unwrap();
        fs::create_dir_all(temp_dir.path().join("alpha")).unwrap();
        fs::write(temp_dir.path().join("alpha/file.rs"), "content").unwrap();
        fs::write(temp_dir.path().join("beta.rs"), "content").unwrap();
        fs::create_dir_all(temp_dir.path().join("gamma")).unwrap();
        fs::write(temp_dir.path().join("gamma/file.rs"), "content").unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-only").arg(temp_dir.path());

        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8(output).unwrap();

        // Directories should come before files, both sorted alphabetically
        let alpha_pos = output_str.find("alpha/").unwrap();
        let gamma_pos = output_str.find("gamma/").unwrap();
        let beta_pos = output_str.find("beta.rs").unwrap();
        let zebra_pos = output_str.find("zebra.rs").unwrap();

        // Directories first (alpha, gamma), then files (beta, zebra)
        assert!(alpha_pos < gamma_pos);
        assert!(gamma_pos < beta_pos);
        assert!(beta_pos < zebra_pos);
    }

    #[test]
    fn test_tree_with_custom_template() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "hello world").unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-header")
            .arg("--output-template")
            .arg("==== FILE_PATH ====\\nFILE_CONTENT\\n")
            .arg(temp_dir.path());

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Directory structure:"))
            .stdout(predicate::str::contains("└── test.rs"))
            .stdout(predicate::str::contains("==== test.rs ===="))
            .stdout(predicate::str::contains("hello world"));
    }

    #[test]
    fn test_tree_critical_fixes_comprehensive() {
        let temp_dir = TempDir::new().unwrap();

        // Create a complex structure that tests all critical fixes:
        // 1. Path normalization and component filtering
        // 2. Duplicate file handling
        // 3. File vs directory conflicts
        // 4. Proper sorting and tree structure

        // Create nested directories
        fs::create_dir_all(temp_dir.path().join("src").join("utils")).unwrap();
        fs::create_dir_all(temp_dir.path().join("config")).unwrap();
        fs::create_dir_all(temp_dir.path().join("tests")).unwrap();

        // Create files that test duplicate handling
        fs::write(temp_dir.path().join("src").join("main.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("src").join("lib.rs"), "// Library").unwrap();
        fs::write(
            temp_dir.path().join("src").join("utils").join("helper.rs"),
            "// Helper",
        )
        .unwrap();

        // Create file vs directory conflict scenario
        fs::write(temp_dir.path().join("config").join("app.toml"), "[app]").unwrap();
        fs::write(temp_dir.path().join("config.json"), "{}").unwrap(); // config as both file and dir

        // Create files with various extensions for sorting test
        fs::write(temp_dir.path().join("README.md"), "# Project").unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(
            temp_dir.path().join("tests").join("integration.rs"),
            "#[test]",
        )
        .unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-only").arg(temp_dir.path());

        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8(output).unwrap();

        // Test 1: Proper directory structure with correct sorting (directories first)
        assert!(output_str.contains("├── config/"));
        assert!(output_str.contains("├── src/"));
        assert!(output_str.contains("├── tests/"));

        // Test 2: Files come after directories, sorted alphabetically
        assert!(output_str.contains("├── Cargo.toml"));
        assert!(output_str.contains("└── config.json"));

        // Test 3: Nested structure is properly rendered
        assert!(output_str.contains("│   ├── utils/"));
        assert!(output_str.contains("│   │   └── helper.rs"));
        assert!(output_str.contains("│   ├── lib.rs"));
        assert!(output_str.contains("│   └── main.rs"));

        // Test 4: File vs directory conflict resolved (config/ directory and config.json file coexist)
        let config_dir_count = output_str.matches("config/").count();
        let config_file_count = output_str.matches("config.json").count();
        assert_eq!(
            config_dir_count, 1,
            "Should have exactly one config/ directory"
        );
        assert_eq!(
            config_file_count, 1,
            "Should have exactly one config.json file"
        );

        // Test 5: No problematic path components (like Windows drive prefixes) appear
        assert!(!output_str.contains("C:"));
        assert!(!output_str.contains("D:"));
        assert!(!output_str.contains("./"));
        assert!(!output_str.contains("../"));

        // Test 6: Proper Unicode tree characters are used
        assert!(output_str.contains("├──"));
        assert!(output_str.contains("└──"));
        assert!(output_str.contains("│"));

        // Test 7: Directory structure header is present in tree-only mode
        assert!(output_str.contains("Directory structure:"));

        // Test 8: All expected files are present and accounted for
        assert!(output_str.contains("main.rs"));
        assert!(output_str.contains("lib.rs"));
        assert!(output_str.contains("helper.rs"));
        assert!(output_str.contains("app.toml"));
        assert!(output_str.contains("integration.rs"));
        assert!(output_str.contains("Cargo.toml"));
    }

    #[test]
    fn test_tree_windows_path_handling() {
        let temp_dir = TempDir::new().unwrap();

        // Create a nested structure that would trigger Windows path issues
        fs::create_dir_all(temp_dir.path().join("repo").join("src")).unwrap();
        fs::write(
            temp_dir.path().join("repo").join("src").join("lib.rs"),
            "// lib content",
        )
        .unwrap();
        fs::write(temp_dir.path().join("repo").join("Cargo.toml"), "[package]").unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.arg("--tree-only").arg(temp_dir.path());

        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8(output).unwrap();

        // Should not contain drive prefixes (C:, D:, etc.) that could appear on Windows
        assert!(!output_str.contains("C:"));
        assert!(!output_str.contains("D:"));
        assert!(!output_str.contains("E:"));

        // Should contain proper nested structure
        assert!(output_str.contains("repo/"));
        assert!(output_str.contains("├── src/") || output_str.contains("└── src/"));
        assert!(output_str.contains("lib.rs"));
        assert!(output_str.contains("Cargo.toml"));
    }

    #[test]
    fn test_generate_tree_empty() {
        let paths = vec![];
        let result = generate_tree(&paths);
        assert_eq!(result, "");
    }

    #[test]
    fn test_generate_tree_single_file() {
        let paths = vec![PathBuf::from("README.md")];
        let result = generate_tree(&paths);
        assert!(result.contains("Directory structure:"));
        assert!(result.contains("└── README.md"));
    }

    #[test]
    fn test_generate_tree_nested_structure() {
        let paths = vec![
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/main.rs"),
            PathBuf::from("Cargo.toml"),
            PathBuf::from("README.md"),
        ];
        let result = generate_tree(&paths);

        assert!(result.contains("Directory structure:"));
        assert!(result.contains("├── src/"));
        assert!(result.contains("│   ├── lib.rs"));
        assert!(result.contains("│   └── main.rs"));
        assert!(result.contains("├── Cargo.toml"));
        assert!(result.contains("└── README.md"));
    }

    #[test]
    fn test_generate_tree_directories_before_files() {
        let paths = vec![PathBuf::from("file.txt"), PathBuf::from("dir/nested.rs")];
        let result = generate_tree(&paths);

        // Directories should come before files
        let dir_pos = result.find("├── dir/").unwrap_or(0);
        let file_pos = result.find("└── file.txt").unwrap_or(0);
        assert!(dir_pos < file_pos);
    }

    #[test]
    fn test_final_component_always_treated_as_file() {
        // Test that final components are always treated as files, regardless of extension
        let paths = vec![
            PathBuf::from("Makefile"),      // No extension
            PathBuf::from("Dockerfile"),    // No extension
            PathBuf::from("src/mod"),       // No extension in subdirectory
            PathBuf::from("config.toml"),   // With extension
            PathBuf::from("scripts/build"), // No extension, could look like directory
        ];
        let result = generate_tree(&paths);

        // All final components should be files (no trailing slash)
        // Directories come first, then files alphabetically
        assert!(result.contains("├── scripts/"));
        assert!(result.contains("│   └── build")); // build should be a file, not build/
        assert!(result.contains("├── src/"));
        assert!(result.contains("│   └── mod")); // mod should be a file, not mod/
        assert!(result.contains("├── Dockerfile"));
        assert!(result.contains("├── Makefile"));
        assert!(result.contains("└── config.toml")); // Last file uses └──

        // Verify no final components have trailing slashes (which would indicate directories)
        assert!(!result.contains("Dockerfile/"));
        assert!(!result.contains("Makefile/"));
        assert!(!result.contains("config.toml/"));
        assert!(!result.contains("build/"));
        assert!(!result.contains("mod/"));
    }

    #[test]
    fn test_windows_path_component_filtering() {
        // Test the clean_path_components function directly
        // On Unix systems, we can't easily create Windows-style paths,
        // so we test the filtering logic with relative paths that have
        // problematic components like ".." and "."

        let path = Path::new("./src/../src/lib.rs");
        let components = clean_path_components(&path);

        // Should filter out "." and keep ".." and normal components
        assert_eq!(components, vec!["src", "..", "src", "lib.rs"]);

        // Test with a simple path
        let path = Path::new("repo/src/lib.rs");
        let components = clean_path_components(&path);
        assert_eq!(components, vec!["repo", "src", "lib.rs"]);
    }

    #[test]
    fn test_path_normalization_in_tree() {
        // Test that paths with current directory components are handled correctly
        let paths = vec![PathBuf::from("./src/lib.rs"), PathBuf::from("src/main.rs")];
        let result = generate_tree(&paths);

        // Should contain proper structure without "./"
        assert!(result.contains("└── src/"));
        assert!(result.contains("    ├── lib.rs"));
        assert!(result.contains("    └── main.rs"));
        // Should not contain "./" in the output
        assert!(!result.contains("./"));
    }

    #[test]
    fn test_duplicate_file_paths() {
        // Test that duplicate file paths are handled correctly
        // The same file path added twice should still result in a single entry
        let paths = vec![
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/lib.rs"), // Duplicate
            PathBuf::from("src/main.rs"),
        ];
        let result = generate_tree(&paths);

        // Should only show lib.rs once
        let lib_rs_count = result.matches("lib.rs").count();
        assert_eq!(
            lib_rs_count, 1,
            "lib.rs should appear only once, got: {}",
            result
        );

        // Should still show both files
        assert!(result.contains("├── lib.rs"));
        assert!(result.contains("└── main.rs"));
    }

    #[test]
    fn test_file_vs_directory_conflict() {
        // Test when the same path is used as both intermediate directory and final file
        // This tests the fix for issue where a file could be marked as directory
        // In this case, the directory usage takes precedence to maintain tree consistency
        let paths = vec![
            PathBuf::from("config/settings.json"), // config as directory
            PathBuf::from("config"), // config as file - should be absorbed into directory
            PathBuf::from("readme.txt"), // another file for comparison
        ];
        let result = generate_tree(&paths);

        // config should be treated as a directory containing settings.json
        // The standalone "config" file is absorbed into the directory structure
        // because directory usage takes precedence when there are children
        assert!(result.contains("├── config/"));
        assert!(result.contains("│   └── settings.json"));
        assert!(result.contains("└── readme.txt"));

        // Should not show config as both file and directory
        let config_lines: Vec<&str> = result
            .lines()
            .filter(|line| line.contains("config"))
            .collect();
        assert_eq!(
            config_lines.len(),
            1,
            "Config should appear only once as directory"
        );
    }

    #[test]
    fn test_empty_directory_becomes_file() {
        // Test that an empty directory entry can be converted to a file
        let paths = vec![
            PathBuf::from("item"), // item as file
        ];
        let result = generate_tree(&paths);

        // item should be treated as a file (no trailing slash)
        assert!(result.contains("└── item"));
        assert!(!result.contains("item/"));
    }

    #[test]
    fn test_processing_order_independence() {
        // Test that the result is the same regardless of processing order
        let paths1 = vec![
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/main.rs"),
            PathBuf::from("src"),
        ];
        let paths2 = vec![
            PathBuf::from("src"),
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/main.rs"),
        ];

        let result1 = generate_tree(&paths1);
        let result2 = generate_tree(&paths2);

        // Both should produce the same tree structure
        // src should be a directory containing lib.rs and main.rs
        assert!(result1.contains("src/"));
        assert!(result1.contains("lib.rs"));
        assert!(result1.contains("main.rs"));

        assert!(result2.contains("src/"));
        assert!(result2.contains("lib.rs"));
        assert!(result2.contains("main.rs"));

        // The essential structure should be the same (ignoring exact formatting)
        let result1_lines: Vec<&str> = result1.lines().filter(|l| !l.trim().is_empty()).collect();
        let result2_lines: Vec<&str> = result2.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(result1_lines.len(), result2_lines.len());
    }
}
