#[cfg(test)]
mod category_tests {
    use yek::category::{categorize_file, FileCategory, CategoryWeights};
    use yek::priority::{get_file_priority_with_category, PriorityRule};

    #[test]
    fn test_categorize_source_files() {
        assert_eq!(categorize_file("src/main.rs"), FileCategory::Source);
        assert_eq!(categorize_file("lib/utils.py"), FileCategory::Source);
        assert_eq!(categorize_file("app/component.js"), FileCategory::Source);
        assert_eq!(categorize_file("main.go"), FileCategory::Source);
        assert_eq!(categorize_file("index.html"), FileCategory::Source);
        assert_eq!(categorize_file("style.css"), FileCategory::Source);
        assert_eq!(categorize_file("script.ts"), FileCategory::Source);
        assert_eq!(categorize_file("component.jsx"), FileCategory::Source);
    }

    #[test]
    fn test_categorize_test_files() {
        assert_eq!(categorize_file("tests/test_main.py"), FileCategory::Test);
        assert_eq!(categorize_file("test/utils_test.go"), FileCategory::Test);
        assert_eq!(categorize_file("src/component.test.js"), FileCategory::Test);
        assert_eq!(categorize_file("__tests__/unit.js"), FileCategory::Test);
        assert_eq!(categorize_file("spec/feature_spec.rb"), FileCategory::Test);
        assert_eq!(categorize_file("e2e/integration.test.ts"), FileCategory::Test);
        assert_eq!(categorize_file("test_utils.py"), FileCategory::Test);
        assert_eq!(categorize_file("utils_spec.rb"), FileCategory::Test);
        assert_eq!(categorize_file("MyComponentTest.java"), FileCategory::Test);
    }

    #[test]
    fn test_categorize_configuration_files() {
        assert_eq!(categorize_file("package.json"), FileCategory::Configuration);
        assert_eq!(categorize_file("Cargo.toml"), FileCategory::Configuration);
        assert_eq!(categorize_file("docker-compose.yml"), FileCategory::Configuration);
        assert_eq!(categorize_file(".eslintrc.json"), FileCategory::Configuration);
        assert_eq!(categorize_file("config/database.yml"), FileCategory::Configuration);
        assert_eq!(categorize_file("Makefile"), FileCategory::Configuration);
        assert_eq!(categorize_file(".gitignore"), FileCategory::Configuration);
        assert_eq!(categorize_file("webpack.config.js"), FileCategory::Configuration);
        assert_eq!(categorize_file("tsconfig.json"), FileCategory::Configuration);
        assert_eq!(categorize_file(".prettierrc"), FileCategory::Configuration);
        assert_eq!(categorize_file("requirements.txt"), FileCategory::Configuration);
        assert_eq!(categorize_file("poetry.toml"), FileCategory::Configuration);
    }

    #[test]
    fn test_categorize_documentation_files() {
        assert_eq!(categorize_file("README.md"), FileCategory::Documentation);
        assert_eq!(categorize_file("docs/guide.rst"), FileCategory::Documentation);
        assert_eq!(categorize_file("CHANGELOG.txt"), FileCategory::Documentation);
        assert_eq!(categorize_file("LICENSE"), FileCategory::Documentation);
        assert_eq!(categorize_file("manual/install.md"), FileCategory::Documentation);
        assert_eq!(categorize_file("CONTRIBUTING.md"), FileCategory::Documentation);
        assert_eq!(categorize_file("AUTHORS"), FileCategory::Documentation);
        assert_eq!(categorize_file("guide/quickstart.md"), FileCategory::Documentation);
    }

    #[test]
    fn test_categorize_other_files() {
        assert_eq!(categorize_file("random.unknown"), FileCategory::Other);
        assert_eq!(categorize_file("data.bin"), FileCategory::Other);
        assert_eq!(categorize_file("image.png"), FileCategory::Other);
        assert_eq!(categorize_file("video.mp4"), FileCategory::Other);
        assert_eq!(categorize_file("archive.zip"), FileCategory::Other);
    }

    #[test]
    fn test_category_priority_offsets() {
        assert_eq!(FileCategory::Configuration.default_priority_offset(), 5);
        assert_eq!(FileCategory::Test.default_priority_offset(), 10);
        assert_eq!(FileCategory::Documentation.default_priority_offset(), 15);
        assert_eq!(FileCategory::Source.default_priority_offset(), 20);
        assert_eq!(FileCategory::Other.default_priority_offset(), 1);
    }

    #[test]
    fn test_category_weights_default() {
        let weights = CategoryWeights::default();
        assert_eq!(weights.get_offset(FileCategory::Source), 20);
        assert_eq!(weights.get_offset(FileCategory::Test), 10);
        assert_eq!(weights.get_offset(FileCategory::Configuration), 5);
        assert_eq!(weights.get_offset(FileCategory::Documentation), 15);
        assert_eq!(weights.get_offset(FileCategory::Other), 1);
    }

    #[test]
    fn test_category_weights_custom() {
        let custom_weights = CategoryWeights {
            source: 100,
            test: 50,
            configuration: 25,
            documentation: 10,
            other: 5,
        };
        assert_eq!(custom_weights.get_offset(FileCategory::Source), 100);
        assert_eq!(custom_weights.get_offset(FileCategory::Test), 50);
        assert_eq!(custom_weights.get_offset(FileCategory::Configuration), 25);
        assert_eq!(custom_weights.get_offset(FileCategory::Documentation), 10);
        assert_eq!(custom_weights.get_offset(FileCategory::Other), 5);
    }

    #[test]
    fn test_priority_calculation_with_category() {
        let rules = vec![
            PriorityRule {
                pattern: "src/.*".to_string(),
                score: 100,
            },
            PriorityRule {
                pattern: ".*\\.rs".to_string(),
                score: 50,
            },
        ];

        let weights = CategoryWeights::default();

        // Test source file with rule matches
        let (priority, category) = get_file_priority_with_category("src/main.rs", &rules, &weights);
        assert_eq!(category, FileCategory::Source);
        // Rule priority: 100 (src/*) + 50 (*.rs) = 150
        // Category offset: 20 (source)
        // Total: 170
        assert_eq!(priority, 170);

        // Test test file with rule matches
        let (priority, category) = get_file_priority_with_category("tests/main.rs", &rules, &weights);
        assert_eq!(category, FileCategory::Test);
        // Rule priority: 50 (*.rs) = 50
        // Category offset: 10 (test)
        // Total: 60
        assert_eq!(priority, 60);

        // Test config file with no rule matches
        let (priority, category) = get_file_priority_with_category("package.json", &rules, &weights);
        assert_eq!(category, FileCategory::Configuration);
        // Rule priority: 0 (no matches)
        // Category offset: 5 (configuration)
        // Total: 5
        assert_eq!(priority, 5);
    }

    #[test]
    fn test_edge_case_categorization() {
        // Files that could be ambiguous should follow specific rules
        
        // JavaScript test files
        assert_eq!(categorize_file("component.test.js"), FileCategory::Test);
        assert_eq!(categorize_file("utils.spec.ts"), FileCategory::Test);
        
        // Configuration files that might look like source
        assert_eq!(categorize_file("webpack.config.js"), FileCategory::Configuration);
        assert_eq!(categorize_file("rollup.config.js"), FileCategory::Configuration);
        
        // README files in various formats
        assert_eq!(categorize_file("README"), FileCategory::Documentation);
        assert_eq!(categorize_file("readme.txt"), FileCategory::Documentation);
        assert_eq!(categorize_file("README.rst"), FileCategory::Documentation);
        
        // Files in test directories should be test even if they don't have test extensions
        assert_eq!(categorize_file("tests/helper.js"), FileCategory::Test);
        assert_eq!(categorize_file("__tests__/setup.ts"), FileCategory::Test);
        
        // Files in config directories should be configuration
        assert_eq!(categorize_file("config/app.js"), FileCategory::Configuration);
        assert_eq!(categorize_file(".config/settings.txt"), FileCategory::Configuration);
    }

    #[test]
    fn test_path_normalization() {
        // Test with different path separators (should work on all platforms)
        assert_eq!(categorize_file("src\\main.rs"), FileCategory::Source);
        assert_eq!(categorize_file("tests\\unit\\test.py"), FileCategory::Test);
        assert_eq!(categorize_file("config\\database.yml"), FileCategory::Configuration);
        assert_eq!(categorize_file("docs\\guide\\install.md"), FileCategory::Documentation);
    }

    #[test]
    fn test_category_name_strings() {
        assert_eq!(FileCategory::Source.name(), "source");
        assert_eq!(FileCategory::Test.name(), "test");
        assert_eq!(FileCategory::Configuration.name(), "configuration");
        assert_eq!(FileCategory::Documentation.name(), "documentation");
        assert_eq!(FileCategory::Other.name(), "other");
    }

    #[test]
    fn test_priority_with_custom_weights() {
        let rules = vec![
            PriorityRule {
                pattern: ".*\\.rs".to_string(),
                score: 50,
            },
        ];

        let custom_weights = CategoryWeights {
            source: 200,
            test: 100,
            configuration: 25,
            documentation: 10,
            other: 5,
        };

        // Source file should get high priority due to custom weights
        let (priority, category) = get_file_priority_with_category("main.rs", &rules, &custom_weights);
        assert_eq!(category, FileCategory::Source);
        assert_eq!(priority, 250); // 50 (rule) + 200 (custom source weight)

        // Test file should get medium priority
        let (priority, category) = get_file_priority_with_category("test_main.rs", &rules, &custom_weights);
        assert_eq!(category, FileCategory::Test);
        assert_eq!(priority, 150); // 50 (rule) + 100 (custom test weight)
    }
}