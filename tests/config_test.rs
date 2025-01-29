#[cfg(test)]
mod config_tests {
    use yek::config::{validate_config, FullYekConfig};
    use yek::priority::PriorityRule;

    #[test]
    fn test_validate_config_valid() {
        let mut config =
            FullYekConfig::extend_config_with_defaults(vec![".".to_string()], "output".to_string());
        config.ignore_patterns = vec!["*.log".to_string()];
        config.priority_rules = vec![PriorityRule {
            pattern: ".*".to_string(),
            score: 10,
        }];
        config.binary_extensions = vec!["bin".to_string()];

        let result = validate_config(&config);
        assert!(result.is_ok(), "Expected no validation errors");
    }

    #[test]
    fn test_validate_config_invalid_max_size() {
        let mut config =
            FullYekConfig::extend_config_with_defaults(vec![".".to_string()], "output".to_string());
        config.max_size = "0".to_string(); // Invalid

        let result = validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_size"));
    }

    #[test]
    fn test_validate_config_invalid_priority_rule_score() {
        let mut config = FullYekConfig::extend_config_with_defaults(vec![], "/tmp/yek".to_string());
        config.priority_rules = vec![PriorityRule {
            pattern: "foo".to_string(),
            score: 1001,
        }];

        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("priority_rules"));
        assert!(err.contains("Priority score 1001 must be between 0 and 1000"));
    }

    #[test]
    fn test_validate_config_invalid_priority_rule_pattern() {
        let mut config = FullYekConfig::extend_config_with_defaults(vec![], "/tmp/yek".to_string());
        config.priority_rules = vec![PriorityRule {
            pattern: "[".to_string(), // Invalid regex
            score: 100,
        }];

        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("priority_rules"));
        assert!(err.contains("Invalid pattern"));
    }

    #[test]
    fn test_validate_config_invalid_ignore_pattern() {
        let mut config = FullYekConfig::extend_config_with_defaults(vec![], "/tmp/yek".to_string());
        config.ignore_patterns = vec!["[".to_string()]; // Invalid regex

        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("ignore_patterns"));
        assert!(err.contains("Invalid pattern"));
    }
}
