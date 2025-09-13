#[cfg(test)]
mod stdin_tests {
    use assert_cmd::prelude::*;
    use std::fs;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use tempfile::tempdir;

    #[test]
    fn test_stdin_input_paths() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let file1_path = temp_dir.path().join("test1.txt");
        let file2_path = temp_dir.path().join("test2.txt");

        fs::write(&file1_path, "Test content 1")?;
        fs::write(&file2_path, "Test content 2")?;

        let mut cmd = Command::cargo_bin("yek")?;
        cmd.current_dir(temp_dir.path());
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());

        let mut child = cmd.spawn()?;

        if let Some(stdin) = child.stdin.as_mut() {
            writeln!(stdin, "test1.txt")?;
            writeln!(stdin, "test2.txt")?;
        }

        let output = child.wait_with_output()?;
        assert!(output.status.success());

        let stdout = String::from_utf8(output.stdout)?;
        assert!(
            stdout.contains("Test content 1"),
            "Should contain content from test1.txt"
        );
        assert!(
            stdout.contains("Test content 2"),
            "Should contain content from test2.txt"
        );

        Ok(())
    }

    #[test]
    fn test_stdin_empty_lines_filtered() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Test content")?;

        let mut cmd = Command::cargo_bin("yek")?;
        cmd.current_dir(temp_dir.path());
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());

        let mut child = cmd.spawn()?;

        if let Some(stdin) = child.stdin.as_mut() {
            writeln!(stdin, "test.txt")?;
            writeln!(stdin, "")?; // empty line
            writeln!(stdin, "   ")?; // whitespace only
            writeln!(stdin, "")?; // another empty line
        }

        let output = child.wait_with_output()?;
        assert!(output.status.success());

        let stdout = String::from_utf8(output.stdout)?;
        assert!(
            stdout.contains("Test content"),
            "Should contain content from test.txt"
        );

        // Count the number of file headers (">>>> filename" patterns)
        let file_count = stdout.matches(">>>> ").count();
        assert_eq!(
            file_count, 1,
            "Should only process one file despite empty lines"
        );

        Ok(())
    }

    #[test]
    fn test_stdin_nonexistent_files_handled() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;

        let mut cmd = Command::cargo_bin("yek")?;
        cmd.current_dir(temp_dir.path());
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());

        let mut child = cmd.spawn()?;

        if let Some(stdin) = child.stdin.as_mut() {
            writeln!(stdin, "nonexistent1.txt")?;
            writeln!(stdin, "nonexistent2.txt")?;
        }

        let output = child.wait_with_output()?;
        assert!(output.status.success());

        let stdout = String::from_utf8(output.stdout)?;
        // Should be empty or minimal since files don't exist
        assert!(
            stdout.trim().is_empty() || stdout.len() < 10,
            "Should have minimal output for nonexistent files"
        );

        Ok(())
    }

    #[test]
    fn test_stdin_empty_defaults_to_current_dir() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Test content")?;

        let mut cmd = Command::cargo_bin("yek")?;
        cmd.current_dir(temp_dir.path());
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());

        let mut child = cmd.spawn()?;

        // Send empty stdin
        if let Some(stdin) = child.stdin.as_mut() {
            // Just close stdin without writing anything
            let _ = stdin;
        }

        let output = child.wait_with_output()?;
        assert!(output.status.success());

        let stdout = String::from_utf8(output.stdout)?;
        assert!(
            stdout.contains("Test content"),
            "Should contain content from current directory scan"
        );

        Ok(())
    }

    #[test]
    fn test_explicit_args_override_stdin() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let file1_path = temp_dir.path().join("explicit.txt");
        let file2_path = temp_dir.path().join("stdin.txt");

        fs::write(&file1_path, "Explicit content")?;
        fs::write(&file2_path, "Stdin content")?;

        let mut cmd = Command::cargo_bin("yek")?;
        cmd.current_dir(temp_dir.path());
        cmd.arg("explicit.txt"); // Explicit argument
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());

        let mut child = cmd.spawn()?;

        if let Some(stdin) = child.stdin.as_mut() {
            writeln!(stdin, "stdin.txt")?; // This should be ignored
        }

        let output = child.wait_with_output()?;
        assert!(output.status.success());

        let stdout = String::from_utf8(output.stdout)?;
        assert!(
            stdout.contains("Explicit content"),
            "Should contain content from explicit argument"
        );
        assert!(
            !stdout.contains("Stdin content"),
            "Should NOT contain content from stdin when explicit args provided"
        );

        Ok(())
    }
}
