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
    fn test_validate_config_valid() {
        let config = FullYekConfig {
            input_dirs: vec![".".to_string()],
            max_size: "10MB".to_string(),
            tokens: "".to_string(),
            debug: false,
            output_dir: "output".to_string(),
            ignore_patterns: vec!["*.log".to_string()],
            priority_rules: vec![PriorityRule {
                pattern: ".*".to_string(),
                score: 10,
            }],
            binary_extensions: vec!["bin".to_string()],
            stream: false,
            token_mode: false,
            output_file_full_path: "output/file.txt".to_string(),
            git_boost_max: 100,
        };

        let errors = validate_config(&config);
        assert!(errors.is_empty(), "Expected no validation errors");
    }

    #[test]
    fn test_validate_config_invalid_max_size() {
        let config = FullYekConfig {
            input_dirs: vec![".".to_string()],
            max_size: "0".to_string(), // Invalid
            tokens: "".to_string(),
            debug: false,
            output_dir: "output".to_string(),
            ignore_patterns: vec![],
            priority_rules: vec![],
            binary_extensions: vec![],
            stream: false,
            token_mode: false,
            output_file_full_path: "output/file.txt".to_string(),
            git_boost_max: 100,
        };

        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "max_size");
    }

    #[test]
    fn test_validate_config_invalid_priority_rule_score() {
        let config = FullYekConfig {
            input_dirs: vec![],
            max_size: "10MB".to_string(),
            tokens: "".to_string(),
            debug: false,
            output_dir: "/tmp/yek".to_string(),
            ignore_patterns: vec![],
            priority_rules: vec![PriorityRule {
                pattern: "foo".to_string(),
                score: 1001,
            }],
            binary_extensions: vec![],
            stream: false,
            token_mode: false,
            output_file_full_path: "/tmp/yek/output.txt".to_string(),
            git_boost_max: 100,
        };

        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "priority_rules");
        assert!(errors[0]
            .message
            .contains("Priority score 1001 must be between 0 and 1000"));
    }

    #[test]
    fn test_validate_config_invalid_priority_rule_pattern() {
        let config = FullYekConfig {
            input_dirs: vec![],
            max_size: "10MB".to_string(),
            tokens: "".to_string(),
            debug: false,
            output_dir: "/tmp/yek".to_string(),
            ignore_patterns: vec![],
            priority_rules: vec![PriorityRule {
                pattern: "[".to_string(), // Invalid regex
                score: 100,
            }],
            binary_extensions: vec![],
            stream: false,
            token_mode: false,
            output_file_full_path: "/tmp/yek/output.txt".to_string(),
            git_boost_max: 100,
        };

        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "priority_rules");
        assert!(errors[0].message.contains("Invalid regex pattern"));
    }

    #[test]
    fn test_validate_config_invalid_ignore_pattern() {
        let config = FullYekConfig {
            input_dirs: vec![],
            max_size: "10MB".to_string(),
            tokens: "".to_string(),
            debug: false,
            output_dir: "/tmp/yek".to_string(),
            ignore_patterns: vec!["[".to_string()], // Invalid regex
            priority_rules: vec![],
            binary_extensions: vec![],
            stream: false,
            token_mode: false,
            output_file_full_path: "/tmp/yek/output.txt".to_string(),
            git_boost_max: 100,
        };

        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "ignore_patterns");
        assert!(errors[0].message.contains("Invalid pattern"));
    }
}
