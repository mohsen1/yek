#[cfg(test)]
mod config_tests {
    use yek::config::{parse_size_input, validate_config, FullYekConfig};
    use yek::priority::PriorityRule;

    #[test]
    fn test_parse_size_input_bytes() {
        assert_eq!(parse_size_input("1024", false).unwrap(), 1024);
        assert_eq!(parse_size_input("1KB", false).unwrap(), 1024);
        assert_eq!(parse_size_input("1MB", false).unwrap(), 1024 * 1024);
        assert_eq!(parse_size_input("1GB", false).unwrap(), 1024 * 1024 * 1024);
        assert!(parse_size_input("invalid", false).is_err());
    }

    #[test]
    fn test_parse_size_input_tokens() {
        assert_eq!(parse_size_input("100", true).unwrap(), 100);
        assert_eq!(parse_size_input("1K", true).unwrap(), 1000);
        assert_eq!(parse_size_input("10K", true).unwrap(), 10000);
        assert!(parse_size_input("1MB", true).is_err());
    }

    #[test]
    fn test_parse_size_input_invalid_tokens() {
        assert!(parse_size_input("-1", true).is_err());
        assert!(parse_size_input("abc", true).is_err());
        assert!(parse_size_input("1.5K", true).is_err());
        assert!(parse_size_input("1M", true).is_err());
    }

    #[test]
    fn test_parse_size_input_edge_cases() {
        assert!(parse_size_input("", false).is_err());
        assert!(parse_size_input(" ", false).is_err());
        assert!(parse_size_input("\t", false).is_err());
        assert!(parse_size_input("1.5MB", false).is_err());
    }

    #[test]
    fn test_validate_config() {
        let valid_config = FullYekConfig {
            input_dirs: vec!["/test/dir".to_string()],
            max_size: "10MB".to_string(),
            tokens: "".to_string(),
            debug: false,
            output_dir: std::env::temp_dir().to_string_lossy().to_string(),
            ignore_patterns: vec!["*.log".to_string()],
            priority_rules: vec![PriorityRule {
                pattern: "src/.*\\.rs".to_string(),
                score: 100,
            }],
            binary_extensions: vec!["bin".to_string()],
            stream: false,
            token_mode: false,
            output_file_full_path: "output.txt".to_string(),
            git_boost_max: 100,
        };

        let errors = validate_config(&valid_config);
        assert!(errors.is_empty());

        let invalid_config = FullYekConfig {
            input_dirs: vec!["/test/dir".to_string()],
            max_size: "0".to_string(), // Invalid
            tokens: "".to_string(),
            debug: false,
            output_dir: "/nonexistent/dir".to_string(),
            ignore_patterns: vec!["[invalid regex".to_string()], // Invalid
            priority_rules: vec![PriorityRule {
                pattern: "[invalid regex".to_string(), // Invalid
                score: 2000,                           // Invalid
            }],
            binary_extensions: vec!["bin".to_string()],
            stream: false,
            token_mode: false,
            output_file_full_path: "output.txt".to_string(),
            git_boost_max: 100,
        };

        let errors = validate_config(&invalid_config);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.field == "max_size"));
        assert!(errors.iter().any(|e| e.field == "priority_rules"));
        assert!(errors.iter().any(|e| e.field == "ignore_patterns"));
    }
}
