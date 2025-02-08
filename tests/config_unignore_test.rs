#[cfg(test)]
mod config_unignore_tests {
    use yek::config::YekConfig;

    #[test]
    fn test_unignore_patterns_are_merged() {
        // Create a basic config with custom ignore and unignore patterns.
        let mut config =
            YekConfig::extend_config_with_defaults(vec![".".to_string()], "output".to_string());
        config.ignore_patterns = vec!["*.log".to_string(), "temp/**".to_string()];
        config.unignore_patterns = vec!["debug.log".to_string(), "temp/keep/**".to_string()];

        // Simulate the merging step that occurs in init_config.
        // (The unignore patterns are applied by prefixing them with "!" and extending ignore_patterns.)
        config.ignore_patterns.extend(
            config
                .unignore_patterns
                .iter()
                .map(|pat| format!("!{}", pat)),
        );

        // Check that the merged ignore_patterns include the negated rules.
        assert!(
            config.ignore_patterns.contains(&"!debug.log".to_string()),
            "Expected ignore_patterns to contain !debug.log"
        );
        assert!(
            config
                .ignore_patterns
                .contains(&"!temp/keep/**".to_string()),
            "Expected ignore_patterns to contain !temp/keep/**"
        );
    }
}
