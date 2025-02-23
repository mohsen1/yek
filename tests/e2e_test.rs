#[cfg(test)]
mod e2e_tests {
    use assert_cmd::Command;
    use predicates::prelude::*;
    use std::fs;

    use tempfile::tempdir;

    #[test]
    fn test_empty_dir() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        Command::cargo_bin("yek")?
            .arg(temp_dir.path())
            .assert()
            .success();
        Ok(())
    }

    #[test]
    fn test_single_file() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("test.txt"), "Test content")?;

        let mut cmd = Command::cargo_bin("yek")?;
        cmd.arg(temp_dir.path()).assert().success();
        Ok(())
    }

    #[test]
    fn test_multiple_files() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("test1.txt"), "Test content 1")?;
        fs::write(temp_dir.path().join("test2.txt"), "Test content 2")?;

        Command::cargo_bin("yek")?
            .arg(temp_dir.path())
            .assert()
            .success();
        Ok(())
    }

    #[test]
    fn test_ignore_patterns() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("test.log"), "Log content")?;
        fs::write(temp_dir.path().join("test.txt"), "Test content")?;

        Command::cargo_bin("yek")?
            .arg(temp_dir.path())
            .arg("--ignore-patterns")
            .arg("*.log")
            .assert()
            .success();
        Ok(())
    }

    #[test]
    fn test_priority_rules() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        // Create the 'src' directory
        fs::create_dir(temp_dir.path().join("src"))?;
        fs::write(temp_dir.path().join("src/test.rs"), "Test content")?;
        fs::write(temp_dir.path().join("test.txt"), "Test content")?;

        let config_content = r#"
            input_paths = ["."]
            [[priority_rules]]
            pattern = "src/.*\\.rs"
            score = 100
        "#;
        fs::write(temp_dir.path().join("yek.toml"), config_content)?;

        let mut cmd = Command::cargo_bin("yek")?;
        cmd.arg("--config-file")
            .arg(temp_dir.path().join("yek.toml"))
            .assert()
            .success();
        Ok(())
    }

    #[test]
    fn test_binary_files() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("image.jpg"), [0xFF, 0xD8, 0xFF])?; // Mock JPEG header

        let mut cmd = Command::cargo_bin("yek")?;
        cmd.arg(temp_dir.path()).assert().success();
        Ok(())
    }

    #[test]
    fn test_output_dir() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let output_dir = temp_dir.path().join("output");

        // Create a simple Rust file
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}")?;

        // Run Yek with output_dir
        let mut cmd = Command::cargo_bin("yek")?;
        let output = cmd
            .current_dir(temp_dir.path())
            .env("TERM", "xterm") // Ensure terminal mode
            .env("FORCE_TTY", "1")
            .arg(temp_dir.path())
            .arg("--output-dir")
            .arg(&output_dir)
            .arg("--debug")
            .output()?;

        assert!(output.status.success());

        // Ensure output dir is printed in stdout
        let stdout = String::from_utf8(output.stdout)?;
        assert!(
            stdout.contains(&output_dir.display().to_string()),
            "Expected output directory `{}` to be printed in stdout, but it was {}",
            output_dir.display(),
            stdout
        );

        // Ensure the directory exists
        assert!(
            output_dir.exists(),
            "Expected output directory `{}` to exist, but it does not",
            output_dir.display()
        );

        // Ensure at least one output file exists inside the directory
        let output_files = fs::read_dir(&output_dir)?
            .filter_map(|e| match e {
                Ok(entry) => Some(entry),
                Err(err) => {
                    eprintln!("Warning: Failed to read directory entry: {}", err);
                    None
                }
            })
            .collect::<Vec<_>>();

        assert!(
            !output_files.is_empty(),
            "Expected output directory `{}` to contain files, but it is empty",
            output_dir.display()
        );

        Ok(())
    }

    #[test]
    fn test_max_size() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("test.txt"), "Test content")?;

        Command::cargo_bin("yek")?
            .arg(temp_dir.path())
            .arg("--max-size")
            .arg("1KB")
            .assert()
            .success();
        Ok(())
    }

    #[test]
    fn test_tokens_mode() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("test.txt"), "Test content")?;

        let mut cmd = Command::cargo_bin("yek")?;
        cmd.arg(temp_dir.path())
            .arg("--tokens")
            .arg("100")
            .assert()
            .success();
        Ok(())
    }

    #[test]
    fn test_git_integration() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        // Initialize a Git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()?;

        fs::write(temp_dir.path().join("test.txt"), "Test content")?;
        std::process::Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(temp_dir.path())
            .output()?;
        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp_dir.path())
            .output()?;

        Command::cargo_bin("yek")?
            .arg(temp_dir.path())
            .assert()
            .success();
        Ok(())
    }

    #[test]
    fn test_multiple_input_dirs() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir1 = tempdir()?;
        let temp_dir2 = tempdir()?;
        fs::write(temp_dir1.path().join("test1.txt"), "Test content 1")?;
        fs::write(temp_dir2.path().join("test2.txt"), "Test content 2")?;

        Command::cargo_bin("yek")?
            .arg(temp_dir1.path())
            .arg(temp_dir2.path())
            .assert()
            .success();
        Ok(())
    }

    #[test]
    fn test_glob_pattern() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("test.txt"), "Test content")?;

        let output = Command::cargo_bin("yek")?
            .current_dir(temp_dir.path())
            .arg("*.txt")
            .output()?;
        let stdout = String::from_utf8(output.stdout)?;
        assert!(output.status.success());
        assert!(stdout.contains("Test content"));
        Ok(())
    }

    #[test]
    fn test_mix_of_files_and_dirs() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("test.txt"), "Test content")?;
        fs::write(temp_dir.path().join("test2.txt"), "Test content 2")?;
        let dir = temp_dir.path().join("dir");
        fs::create_dir(&dir)?;
        fs::write(dir.join("test3"), "Test content 3")?;

        Command::cargo_bin("yek")?
            .current_dir(temp_dir.path())
            .arg("*.txt")
            .assert()
            .success();

        let output = Command::cargo_bin("yek")?
            .current_dir(temp_dir.path())
            .arg("*.txt")
            .output()?;
        let stdout = String::from_utf8(output.stdout)?;
        assert!(stdout.contains("Test content"));
        assert!(stdout.contains("Test content 2"));
        assert!(!stdout.contains("Test content 3"));
        Ok(())
    }

    #[test]
    fn test_mix_of_files_and_dirs_with_glob_pattern() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("test.txt"), "Test content")?;
        fs::write(temp_dir.path().join("test2.txt"), "Test content 2")?;
        fs::write(temp_dir.path().join("code.rs"), "use std::fs;")?;
        let dir = temp_dir.path().join("dir");
        fs::create_dir(&dir)?;
        fs::write(dir.join("test4"), "Test content 4")?;

        Command::cargo_bin("yek")?
            .current_dir(temp_dir.path())
            .args(["*.txt", "code.rs"])
            .assert()
            .success();

        let output = Command::cargo_bin("yek")?
            .current_dir(temp_dir.path())
            .args(["*.txt", "code.rs"])
            .output()?;
        let stdout = String::from_utf8(output.stdout)?;
        assert!(stdout.contains("Test content"));
        assert!(stdout.contains("Test content 2"));
        assert!(!stdout.contains("Test content 4"));
        assert!(stdout.contains("use std::fs;"));
        Ok(())
    }

    #[test]
    fn test_config_file() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let config_content = r#"
            max_size = "1KB"
            input_paths = ["."]
        "#;
        fs::write(temp_dir.path().join("yek.toml"), config_content)?;

        let mut cmd = Command::cargo_bin("yek")?;
        cmd.arg("--config-file")
            .arg(temp_dir.path().join("yek.toml"))
            .assert()
            .success();
        Ok(())
    }

    #[test]
    fn test_streaming_mode() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("test.rs"), "Test content")?;

        let mut cmd = Command::cargo_bin("yek")?;
        cmd.arg(temp_dir.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("Test content"));
        Ok(())
    }

    #[test]
    fn test_gitignore_respected() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join(".gitignore"), "*.log")?;
        fs::write(temp_dir.path().join("test.log"), "Log content")?;
        fs::write(temp_dir.path().join("test.txt"), "Test content")?;

        Command::cargo_bin("yek")?
            .arg(temp_dir.path())
            .assert()
            .success();

        Ok(())
    }

    #[test]
    fn test_hidden_files_included() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join(".hidden.txt"), "Hidden content")?;

        Command::cargo_bin("yek")?
            .arg(temp_dir.path())
            .assert()
            .success();
        Ok(())
    }

    #[test]
    fn test_binary_file_extension_config() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("data.bin"), [0, 1, 2, 3])?;

        let config_content = r#"
            input_paths = ["."]
            binary_extensions = ["bin"]
        "#;
        fs::write(temp_dir.path().join("yek.toml"), config_content)?;

        Command::cargo_bin("yek")?
            .arg("--config-file")
            .arg(temp_dir.path().join("yek.toml"))
            .assert()
            .success();
        Ok(())
    }

    #[test]
    fn test_git_boost_config() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let config_content = r#"
            input_paths = ["."]
            git_boost_max = 50
        "#;
        fs::write(temp_dir.path().join("yek.toml"), config_content)?;

        // Initialize a Git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()?;

        fs::write(temp_dir.path().join("file.txt"), "content")?;
        std::process::Command::new("git")
            .args(["add", "file.txt"])
            .current_dir(temp_dir.path())
            .output()?;
        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp_dir.path())
            .output()?;

        let mut cmd = Command::cargo_bin("yek")?;
        cmd.arg("--config-file")
            .arg(temp_dir.path().join("yek.toml"))
            .assert()
            .success();
        Ok(())
    }

    #[test]
    fn test_default_ignore_license_no_config() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("LICENSE"), "License content")?;

        let mut cmd = Command::cargo_bin("yek")?;
        let output = cmd.arg(temp_dir.path()).output()?;

        // Assert that the command was successful
        assert!(output.status.success());

        // Convert stdout bytes to a string
        let stdout = String::from_utf8(output.stdout)?;

        // Assert that the output does not contain "License content"
        assert!(
            !stdout.contains("License content"),
            "Output should not contain 'License content'"
        );

        Ok(())
    }

    #[test]
    fn test_default_ignore_license_empty_config() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("LICENSE"), "License content")?;
        fs::write(
            temp_dir.path().join("yek.yaml"),
            "ignore_patterns: []\n", // Empty ignore_patterns
        )?;

        let mut cmd = Command::cargo_bin("yek")?;
        let output = cmd
            .arg("--config-file")
            .arg(temp_dir.path().join("yek.yaml"))
            .arg(temp_dir.path())
            .output()?;

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout)?;
        assert!(
            !stdout.contains("License content"),
            "Output should not contain 'License content' even with empty config"
        );

        Ok(())
    }

    #[test]
    fn test_gitignore_allowlist() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("LICENSE"), "License content")?;
        fs::write(temp_dir.path().join(".gitignore"), "!LICENSE\n")?;

        let mut cmd = Command::cargo_bin("yek")?;
        let output = cmd.arg(temp_dir.path()).output()?;

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout)?;

        assert!(
            stdout.contains("License content"),
            "Output should contain 'License content' because .gitignore allowlists it"
        );

        Ok(())
    }

    #[test]
    fn test_windows_path_normalization() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        fs::write(temp_dir.path().join("LICENSE"), "License content")?;
        // TODO:
        // Use a path with mixed slashes to simulate potential Windows issues
        // let windows_path = format!(
        //     "{}\\LICENSE",
        //     temp_dir.path().to_string_lossy().replace("/", "\\")
        // );

        let mut cmd = Command::cargo_bin("yek")?;
        let output = cmd.arg(temp_dir.path()).output()?;

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout)?;

        assert!(
            !stdout.contains("License content"),
            "Output should not contain 'License content' even with Windows-style paths"
        );

        Ok(())
    }
}
