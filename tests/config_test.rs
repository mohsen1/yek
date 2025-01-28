#[cfg(test)]
mod config_tests {
    use yek::config::{validate_config, FullYekConfig};
    use yek::priority::PriorityRule;

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

        let result = validate_config(&config);
        assert!(result.is_ok(), "Expected no validation errors");
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

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_size"));
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

        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("priority_rules"));
        assert!(err.contains("Priority score 1001 must be between 0 and 1000"));
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

        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("priority_rules"));
        assert!(err.contains("Invalid regex pattern"));
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

        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("ignore_patterns"));
        assert!(err.contains("Invalid pattern"));
    }
}
