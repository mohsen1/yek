use std::fs;
use tempfile::tempdir;
use yek::{config::YekConfig, serialize_repo};

#[cfg(test)]
mod line_numbers_tests {
    use super::*;

    #[test]
    fn test_line_numbers_disabled_by_default() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3").unwrap();

        let mut config = YekConfig::default();
        config.input_paths = vec![temp_dir.path().to_string_lossy().to_string()];
        config.line_numbers = false; // Explicitly set to false

        let (output, _) = serialize_repo(&config).unwrap();

        // Should not contain line numbers
        assert!(!output.contains("  1 |"));
        assert!(!output.contains("  2 |"));
        assert!(!output.contains("  3 |"));
        assert!(output.contains("line 1"));
        assert!(output.contains("line 2"));
        assert!(output.contains("line 3"));
    }

    #[test]
    fn test_line_numbers_enabled() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3").unwrap();

        let mut config = YekConfig::default();
        config.input_paths = vec![temp_dir.path().to_string_lossy().to_string()];
        config.line_numbers = true;

        let (output, _) = serialize_repo(&config).unwrap();

        // Should contain line numbers
        assert!(output.contains("  1 | line 1"));
        assert!(output.contains("  2 | line 2"));
        assert!(output.contains("  3 | line 3"));
    }

    #[test]
    fn test_line_numbers_with_json_output() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2").unwrap();

        let mut config = YekConfig::default();
        config.input_paths = vec![temp_dir.path().to_string_lossy().to_string()];
        config.line_numbers = true;
        config.json = true;

        let (output, _) = serialize_repo(&config).unwrap();

        // Should be valid JSON with line numbers
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        let files = json.as_array().unwrap();
        let first_file = &files[0];
        let content = first_file["content"].as_str().unwrap();

        assert!(content.contains("  1 | line 1"));
        assert!(content.contains("  2 | line 2"));
    }

    #[test]
    fn test_line_numbers_single_line() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("single.txt");
        fs::write(&file_path, "single line").unwrap();

        let mut config = YekConfig::default();
        config.input_paths = vec![temp_dir.path().to_string_lossy().to_string()];
        config.line_numbers = true;

        let (output, _) = serialize_repo(&config).unwrap();

        assert!(output.contains("  1 | single line"));
        assert!(!output.contains("  2 |"));
    }

    #[test]
    fn test_line_numbers_empty_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("empty.txt");
        fs::write(&file_path, "").unwrap();

        let mut config = YekConfig::default();
        config.input_paths = vec![temp_dir.path().to_string_lossy().to_string()];
        config.line_numbers = true;

        let (output, _) = serialize_repo(&config).unwrap();

        // Empty file should not have any line numbers
        assert!(!output.contains("  1 |"));
    }

    #[test]
    fn test_line_numbers_with_many_lines() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("many_lines.txt");
        let content = (1..=15)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&file_path, content).unwrap();

        let mut config = YekConfig::default();
        config.input_paths = vec![temp_dir.path().to_string_lossy().to_string()];
        config.line_numbers = true;

        let (output, _) = serialize_repo(&config).unwrap();

        // Check single-digit line numbers are formatted correctly
        assert!(output.contains("  1 | line 1"));
        assert!(output.contains("  9 | line 9"));
        // Check double-digit line numbers are formatted correctly
        assert!(output.contains(" 10 | line 10"));
        assert!(output.contains(" 15 | line 15"));
    }

    #[test]
    fn test_line_numbers_with_custom_template() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2").unwrap();

        let mut config = YekConfig::default();
        config.input_paths = vec![temp_dir.path().to_string_lossy().to_string()];
        config.line_numbers = true;
        config.output_template = "=== FILE_PATH ===\nFILE_CONTENT".to_string();

        let (output, _) = serialize_repo(&config).unwrap();

        // Should contain custom template with line numbers
        assert!(output.contains("=== test.txt ==="));
        assert!(output.contains("  1 | line 1"));
        assert!(output.contains("  2 | line 2"));
    }
}
