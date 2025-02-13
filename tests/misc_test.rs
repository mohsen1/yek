use std::fs;
use std::path::Path;
use tempfile::tempdir;
use yek::is_text_file;

#[cfg(test)]
mod misc_tests {
    use super::*;

    // Test that is_text_file returns an error when the file does not exist.
    #[test]
    fn test_is_text_file_nonexistent() {
        let path = Path::new("this_file_should_not_exist_1234567890.txt");
        let result = is_text_file(path, &[]);
        assert!(result.is_err(), "Expected error for nonexistent file");
    }

    // Additional test: create a temporary file with sample content and ensure is_text_file passes.
    #[test]
    fn test_is_text_file_with_valid_text() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let file_path = temp_dir.path().join("sample.txt");
        fs::write(&file_path, "This is a valid text file.").expect("failed to write file");
        let result = is_text_file(&file_path, &[]);
        assert!(result.is_ok());
        assert!(
            result.unwrap(),
            "Expected a text file to be detected as text"
        );
    }

    // Additional test: create a temporary file with binary content and check that is_text_file returns false.
    #[test]
    fn test_is_text_file_with_binary_content() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let file_path = temp_dir.path().join("binary.dat");
        fs::write(&file_path, [0, 159, 146, 150]).expect("failed to write binary file");
        let result = is_text_file(&file_path, &[]);
        assert!(result.is_ok());
        assert!(
            !result.unwrap(),
            "Expected a binary file to be detected as binary"
        );
    }
}
