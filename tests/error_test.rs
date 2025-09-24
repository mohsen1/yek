use std::path::PathBuf;
use tempfile::TempDir;
use yek::error::{safe_ops, ErrorContext, ErrorReporter, YekError};

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_yek_error_display_file_system() {
        let error = YekError::FileSystem {
            operation: "read".to_string(),
            path: PathBuf::from("/test/path"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
        };
        let display = format!("{}", error);
        assert!(display.contains("File system error during 'read' on '/test/path'"));
        assert!(display.contains("File not found"));
    }

    #[test]
    fn test_yek_error_display_git() {
        let error = YekError::Git {
            operation: "commit".to_string(),
            repository: PathBuf::from("/repo"),
            source: git2::Error::from_str("Invalid commit"),
        };
        let display = format!("{}", error);
        assert!(display.contains("Git error during 'commit' in repository '/repo'"));
    }

    #[test]
    fn test_yek_error_display_configuration() {
        let error = YekError::Configuration {
            field: "max_size".to_string(),
            value: "invalid".to_string(),
            reason: "Invalid format".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains(
            "Configuration error for field 'max_size' with value 'invalid': Invalid format"
        ));
    }

    #[test]
    fn test_yek_error_display_processing() {
        let error = YekError::Processing {
            stage: "tokenization".to_string(),
            file: Some(PathBuf::from("test.txt")),
            reason: "Invalid encoding".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains(
            "Processing error in stage 'tokenization' for file 'test.txt': Invalid encoding"
        ));
    }

    #[test]
    fn test_yek_error_display_processing_no_file() {
        let error = YekError::Processing {
            stage: "parsing".to_string(),
            file: None,
            reason: "Syntax error".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Processing error in stage 'parsing': Syntax error"));
    }

    #[test]
    fn test_yek_error_display_memory() {
        let error = YekError::Memory {
            operation: "file reading".to_string(),
            requested: 1000,
            available: Some(500),
        };
        let display = format!("{}", error);
        assert!(display.contains(
            "Memory error during 'file reading' - requested: 1000 bytes, available: 500 bytes"
        ));
    }

    #[test]
    fn test_yek_error_display_memory_no_available() {
        let error = YekError::Memory {
            operation: "allocation".to_string(),
            requested: 2000,
            available: None,
        };
        let display = format!("{}", error);
        assert!(display.contains("Memory error during 'allocation' - requested: 2000 bytes"));
    }

    #[test]
    fn test_yek_error_display_security() {
        let error = YekError::Security {
            violation: "Path traversal".to_string(),
            path: PathBuf::from("../outside"),
            attempted_by: "user_input".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains(
            "Security violation 'Path traversal' for path '../outside' attempted by: user_input"
        ));
    }

    #[test]
    fn test_yek_error_display_validation() {
        let error = YekError::Validation {
            field: "pattern".to_string(),
            value: "*".to_string(),
            constraint: "must be valid regex".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Validation error for field 'pattern' with value '*': violates constraint 'must be valid regex'"));
    }

    #[test]
    fn test_yek_error_display_tokenization() {
        let error = YekError::Tokenization {
            content_type: "text".to_string(),
            size: 1024,
            reason: "Encoding error".to_string(),
        };
        let display = format!("{}", error);
        assert!(
            display.contains("Tokenization error for text content (size: 1024): Encoding error")
        );
    }

    #[test]
    fn test_yek_error_display_user_input() {
        let error = YekError::UserInput {
            input_type: "path".to_string(),
            value: "/invalid".to_string(),
            suggestion: "Use absolute paths".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Invalid path input '/invalid': Use absolute paths"));
    }

    #[test]
    fn test_error_context_new() {
        let context = ErrorContext::new("test_operation");
        assert_eq!(context.operation, "test_operation");
        assert!(context.file.is_none());
        assert!(context.line.is_none());
        assert!(context.column.is_none());
        assert!(context.additional_info.is_empty());
    }

    #[test]
    fn test_error_context_with_file() {
        let context = ErrorContext::new("test").with_file("/test/file.txt");
        assert_eq!(context.file, Some(PathBuf::from("/test/file.txt")));
    }

    #[test]
    fn test_error_context_with_location() {
        let context = ErrorContext::new("test").with_location(10, 5);
        assert_eq!(context.line, Some(10));
        assert_eq!(context.column, Some(5));
    }

    #[test]
    fn test_error_context_with_info() {
        let context = ErrorContext::new("test")
            .with_info("key1", "value1")
            .with_info("key2", "value2");
        assert_eq!(context.additional_info.len(), 2);
        assert_eq!(
            context.additional_info[0],
            ("key1".to_string(), "value1".to_string())
        );
        assert_eq!(
            context.additional_info[1],
            ("key2".to_string(), "value2".to_string())
        );
    }

    #[test]
    fn test_error_context_default() {
        let context = ErrorContext::default();
        assert_eq!(context.operation, "unknown_operation");
    }

    #[test]
    fn test_error_reporter_user_friendly_message_file_system() {
        let error = YekError::FileSystem {
            operation: "read".to_string(),
            path: PathBuf::from("test.txt"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied"),
        };
        let message = ErrorReporter::user_friendly_message(&error);
        assert_eq!(message, "Failed to read file 'test.txt'");
    }

    #[test]
    fn test_error_reporter_user_friendly_message_git() {
        let error = YekError::Git {
            operation: "push".to_string(),
            repository: PathBuf::from("/repo"),
            source: git2::Error::from_str("Network error"),
        };
        let message = ErrorReporter::user_friendly_message(&error);
        assert_eq!(message, "Git operation 'push' failed in repository '/repo'");
    }

    #[test]
    fn test_error_reporter_user_friendly_message_configuration() {
        let error = YekError::Configuration {
            field: "timeout".to_string(),
            value: "abc".to_string(),
            reason: "Must be a number".to_string(),
        };
        let message = ErrorReporter::user_friendly_message(&error);
        assert_eq!(
            message,
            "Configuration issue with 'timeout': Must be a number"
        );
    }

    #[test]
    fn test_error_reporter_user_friendly_message_processing() {
        let error = YekError::Processing {
            stage: "compilation".to_string(),
            file: Some(PathBuf::from("main.rs")),
            reason: "Syntax error".to_string(),
        };
        let message = ErrorReporter::user_friendly_message(&error);
        assert_eq!(
            message,
            "Processing failed in 'compilation' stage for 'main.rs': Syntax error"
        );
    }

    #[test]
    fn test_error_reporter_user_friendly_message_memory() {
        let error = YekError::Memory {
            operation: "buffer allocation".to_string(),
            requested: 1000000,
            available: None,
        };
        let message = ErrorReporter::user_friendly_message(&error);
        assert_eq!(
            message,
            "Insufficient memory for 'buffer allocation' (requested: 1000000 bytes)"
        );
    }

    #[test]
    fn test_error_reporter_user_friendly_message_security() {
        let error = YekError::Security {
            violation: "Directory traversal".to_string(),
            path: PathBuf::from("../../etc"),
            attempted_by: "input".to_string(),
        };
        let message = ErrorReporter::user_friendly_message(&error);
        assert_eq!(
            message,
            "Security violation 'Directory traversal' for path '../../etc'"
        );
    }

    #[test]
    fn test_error_reporter_user_friendly_message_validation() {
        let error = YekError::Validation {
            field: "email".to_string(),
            value: "invalid".to_string(),
            constraint: "must be valid email format".to_string(),
        };
        let message = ErrorReporter::user_friendly_message(&error);
        assert_eq!(
            message,
            "Validation failed for 'email': violates 'must be valid email format'"
        );
    }

    #[test]
    fn test_error_reporter_user_friendly_message_tokenization() {
        let error = YekError::Tokenization {
            content_type: "binary".to_string(),
            size: 2048,
            reason: "Unsupported format".to_string(),
        };
        let message = ErrorReporter::user_friendly_message(&error);
        assert_eq!(
            message,
            "Failed to process binary content (2048): Unsupported format"
        );
    }

    #[test]
    fn test_error_reporter_user_friendly_message_user_input() {
        let error = YekError::UserInput {
            input_type: "command".to_string(),
            value: "invalid_cmd".to_string(),
            suggestion: "Use 'help' to see available commands".to_string(),
        };
        let message = ErrorReporter::user_friendly_message(&error);
        assert_eq!(
            message,
            "Invalid command: Use 'help' to see available commands"
        );
    }

    #[test]
    fn test_safe_read_file_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.txt");
        let context = ErrorContext::new("test_read");

        let result = safe_ops::safe_read_file(&nonexistent_path, &context, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match &err.error {
            YekError::FileSystem { operation, .. } => assert_eq!(operation, "read"),
            _ => panic!("Expected FileSystem error"),
        }
    }

    #[test]
    fn test_safe_read_file_directory() {
        let temp_dir = TempDir::new().unwrap();
        let context = ErrorContext::new("test_read");

        let result = safe_ops::safe_read_file(temp_dir.path(), &context, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match &err.error {
            YekError::FileSystem { operation, .. } => assert_eq!(operation, "read"),
            _ => panic!("Expected FileSystem error"),
        }
    }

    #[test]
    fn test_safe_read_file_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, b"Hello, world!").unwrap();
        let context = ErrorContext::new("test_read");

        let result = safe_ops::safe_read_file(&file_path, &context, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"Hello, world!");
    }

    #[test]
    fn test_safe_read_file_size_limit() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, b"Hello, world!").unwrap(); // 13 bytes
        let context = ErrorContext::new("test_read");

        let result = safe_ops::safe_read_file(&file_path, &context, Some(10));
        assert!(result.is_err());
        let err = result.unwrap_err();
        match &err.error {
            YekError::Memory {
                operation,
                requested,
                available,
            } => {
                assert_eq!(operation, "file reading");
                assert_eq!(*requested, 13);
                assert_eq!(*available, Some(10));
            }
            _ => panic!("Expected Memory error"),
        }
    }

    #[test]
    fn test_safe_validate_utf8_valid() {
        let bytes = b"Hello, world!";
        let context = ErrorContext::new("test_utf8");

        let result = safe_ops::safe_validate_utf8(bytes, &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");
    }

    #[test]
    fn test_safe_validate_utf8_invalid_with_replacement() {
        // Create invalid UTF-8 bytes
        let bytes = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8 sequence
        let context = ErrorContext::new("test_utf8");

        let result = safe_ops::safe_validate_utf8(&bytes, &context);
        assert!(result.is_ok());
        let content = result.unwrap();
        // Should contain replacement character
        assert!(content.contains('\u{FFFD}'));
    }

    #[test]
    fn test_safe_validate_path_valid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, b"test").unwrap();
        let context = ErrorContext::new("test_path");

        // Use canonicalized paths for comparison
        let canonical_base = std::fs::canonicalize(temp_dir.path()).unwrap();
        let canonical_file = std::fs::canonicalize(&file_path).unwrap();

        let result = safe_ops::safe_validate_path(&canonical_file, &canonical_base, &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_safe_validate_path_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let parent_dir = temp_dir.path().parent().unwrap();
        let traversal_path = parent_dir.join("outside.txt");
        let context = ErrorContext::new("test_path");

        // Create the traversal path outside temp_dir
        std::fs::write(&traversal_path, b"outside").unwrap();

        let canonical_base = std::fs::canonicalize(temp_dir.path()).unwrap();
        let canonical_traversal = std::fs::canonicalize(&traversal_path).unwrap();

        let result = safe_ops::safe_validate_path(&canonical_traversal, &canonical_base, &context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match &err.error {
            YekError::Security { violation, .. } => assert_eq!(violation, "Path traversal attempt"),
            _ => panic!("Expected Security error"),
        }
    }
}
