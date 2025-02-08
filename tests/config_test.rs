use std::fs;
use std::fs::File;
use std::io::Write;
use yek::defaults::{BINARY_FILE_EXTENSIONS, DEFAULT_IGNORE_PATTERNS, DEFAULT_OUTPUT_TEMPLATE};

use yek::config::YekConfig;
use yek::priority::PriorityRule;

#[test]
fn test_validate_config_valid() {
    let mut config =
        YekConfig::extend_config_with_defaults(vec![".".to_string()], "output".to_string());
    config.ignore_patterns = vec!["*.log".to_string()];
    config.priority_rules = vec![PriorityRule {
        pattern: ".*".to_string(),
        score: 10,
    }];
    config.binary_extensions = vec!["bin".to_string()];

    let result = config.validate();
    assert!(result.is_ok(), "Expected no validation errors");
}

#[test]
fn test_validate_config_invalid_max_size() {
    let mut config =
        YekConfig::extend_config_with_defaults(vec![".".to_string()], "output".to_string());
    config.max_size = "0".to_string(); // Invalid

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("max_size"));
}

#[test]
fn test_validate_config_invalid_priority_rule_score() {
    let mut config = YekConfig::extend_config_with_defaults(vec![], "/tmp/yek".to_string());
    config.priority_rules = vec![PriorityRule {
        pattern: "foo".to_string(),
        score: 1001,
    }];

    let result = config.validate();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("priority_rules"));
    assert!(err.contains("Priority score 1001 must be between 0 and 1000"));
}

#[test]
fn test_validate_config_invalid_priority_rule_pattern() {
    let mut config = YekConfig::extend_config_with_defaults(vec![], "/tmp/yek".to_string());
    config.priority_rules = vec![PriorityRule {
        pattern: "[".to_string(), // Invalid regex
        score: 100,
    }];

    let result = config.validate();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("priority_rules"));
    assert!(err.contains("Invalid pattern"));
}

#[test]
fn test_validate_config_invalid_ignore_pattern() {
    let mut config = YekConfig::extend_config_with_defaults(vec![], "/tmp/yek".to_string());
    config.ignore_patterns = vec!["[".to_string()]; // Invalid regex

    let result = config.validate();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("ignore_patterns"));
    assert!(err.contains("Invalid pattern"));
}

#[test]
fn test_validate_invalid_output_template() {
    let mut cfg = YekConfig::default();
    cfg.output_template = ">>>> FILE_PATH\n".to_string(); // Missing FILE_CONTENT
    let result = cfg.validate();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "output_template: must contain FILE_PATH and FILE_CONTENT"
    );

    cfg.output_template = ">>>> FILE_CONTENT\n".to_string(); // Missing FILE_PATH
    let result = cfg.validate();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "output_template: must contain FILE_PATH and FILE_CONTENT"
    );
}

#[test]
fn test_validate_max_size_zero() {
    let mut cfg = YekConfig::default();
    cfg.max_size = "0".to_string();
    let result = cfg.validate();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "max_size: cannot be 0");
}

#[test]
fn test_validate_invalid_tokens() {
    let mut cfg = YekConfig::default();
    cfg.token_mode = true;

    cfg.tokens = "0".to_string();
    let result = cfg.validate();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "tokens: cannot be 0");

    cfg.tokens = "-100".to_string();
    let result = cfg.validate();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("tokens: Invalid token size:"));

    cfg.tokens = "abc".to_string();
    let result = cfg.validate();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("tokens: Invalid token size:"));
}

#[test]
fn test_validate_invalid_ignore_patterns() {
    let mut cfg = YekConfig::default();
    cfg.ignore_patterns.push("**/*".to_string()); // Valid pattern
    let result = cfg.validate();
    assert!(result.is_ok());

    cfg.ignore_patterns.push("**[[".to_string()); // Invalid pattern
    let result = cfg.validate();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    println!("Actual error message: {}", err);
    assert!(err.contains("ignore_patterns: Invalid pattern"));
}

#[test]
fn test_validate_invalid_priority_rules() {
    // Test 1: Valid priority rule
    let mut cfg = YekConfig::default();
    cfg.priority_rules.push(PriorityRule {
        pattern: "*.rs".to_string(),
        score: 500,
    });
    let result = cfg.validate();
    assert!(result.is_ok());

    // Test 2: Invalid score
    let mut cfg = YekConfig::default();
    cfg.priority_rules.push(PriorityRule {
        pattern: "*.rs".to_string(),
        score: -10,
    });
    let result = cfg.validate();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    println!("Actual error message: {}", err);
    assert!(err.contains("Priority score -10 must be between 0 and 1000"));

    // Test 3: Invalid pattern
    let mut cfg = YekConfig::default();
    cfg.priority_rules.push(PriorityRule {
        pattern: "[[[".to_string(),
        score: 500,
    });
    let result = cfg.validate();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    println!("Actual error message: {}", err);
    assert!(err.contains("priority_rules: Invalid pattern '[[[':"));
}

#[test]
fn test_ensure_output_dir_output_dir_is_file() {
    // Create a temp file
    let temp_dir = std::env::temp_dir();
    let temp_file_path = temp_dir.join("yek_test_temp_file");
    let mut temp_file = File::create(&temp_file_path).unwrap();
    writeln!(temp_file, "test").unwrap();

    let temp_file_path_str = temp_file_path.to_string_lossy().to_string();

    let mut cfg = YekConfig::default();
    cfg.output_dir = Some(temp_file_path_str.clone());
    cfg.stream = false;

    let result = cfg.ensure_output_dir();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        format!(
            "output_dir: '{}' exists but is not a directory",
            temp_file_path_str
        )
    );

    // Clean up
    std::fs::remove_file(&temp_file_path).unwrap();
}

#[test]
fn test_ensure_output_dir_valid_output_dir() {
    // Create a temp directory
    let temp_dir = std::env::temp_dir().join("yek_test_output_dir");
    let temp_dir_str = temp_dir.to_string_lossy().to_string();

    // Ensure it doesn't exist first
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    let mut cfg = YekConfig::default();
    cfg.output_dir = Some(temp_dir_str.clone());
    cfg.stream = false;

    let result = cfg.ensure_output_dir();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), temp_dir_str);

    // Check that the directory was created
    assert!(temp_dir.is_dir());

    // Clean up
    fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_ensure_output_dir_output_dir_none() {
    let mut cfg = YekConfig::default();
    cfg.output_dir = None;
    cfg.stream = false;

    let result = cfg.ensure_output_dir();
    assert!(result.is_ok());

    let output_dir = result.unwrap();
    // Output dir should be in temp dir
    assert!(output_dir.contains("yek-output"));
}

#[test]
fn test_ensure_output_dir_streaming() {
    let cfg = YekConfig {
        stream: true,
        ..Default::default()
    };

    let result = cfg.ensure_output_dir();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), String::new());
}

#[test]
fn test_get_checksum_consistency() {
    let temp_dir = std::env::temp_dir().join("yek_test_checksum_dir");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir(&temp_dir).unwrap();

    let file_path = temp_dir.join("test_file.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "Hello, world!").unwrap();

    // Get checksum
    let input_dirs = vec![temp_dir.to_string_lossy().to_string()];
    let checksum1 = YekConfig::get_checksum(&input_dirs);

    // Wait a bit and get checksum again
    std::thread::sleep(std::time::Duration::from_millis(100));
    let checksum2 = YekConfig::get_checksum(&input_dirs);

    assert_eq!(checksum1, checksum2);

    // Modify the file and get checksum again
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "Modified content").unwrap();

    // Ensure the modification time is updated
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.write_at(b" ", 0).unwrap();
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        file.seek_write(b" ", 0).unwrap();
    }

    let checksum3 = YekConfig::get_checksum(&input_dirs);
    assert_ne!(checksum1, checksum3);

    // Clean up
    drop(file); // Ensure file is closed before removal
    fs::remove_dir_all(&temp_dir).unwrap_or_else(|e| eprintln!("Failed to remove temp dir: {}", e));
}

// New tests added

#[test]
fn test_extend_config_with_defaults() {
    let input_dirs = vec!["src".to_string(), "tests".to_string()];
    let output_dir = "output".to_string();
    let cfg = YekConfig::extend_config_with_defaults(input_dirs.clone(), output_dir.clone());

    assert_eq!(cfg.input_dirs, input_dirs);
    assert_eq!(cfg.output_dir.unwrap(), output_dir);

    // Check other fields are default
    assert!(!cfg.version);
    assert_eq!(cfg.max_size, "10MB");
    assert_eq!(cfg.tokens, String::new());
    assert!(!cfg.json);
    assert!(!cfg.debug);
    assert_eq!(cfg.output_template, DEFAULT_OUTPUT_TEMPLATE.to_string());
    assert_eq!(cfg.ignore_patterns, Vec::<String>::new());
    assert_eq!(cfg.unignore_patterns, Vec::<String>::new());
    assert_eq!(cfg.priority_rules, Vec::<PriorityRule>::new());
    assert_eq!(
        cfg.binary_extensions,
        BINARY_FILE_EXTENSIONS
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(cfg.git_boost_max, Some(100));
    assert!(!cfg.stream);
    assert!(!cfg.token_mode);
    assert_eq!(cfg.output_file_full_path, None);
    assert_eq!(cfg.max_git_depth, 100);
}

#[test]
fn test_validate_valid_config() {
    let mut cfg = YekConfig::default();
    cfg.output_template = ">>>> FILE_PATH\nFILE_CONTENT".to_string();
    cfg.max_size = "5MB".to_string();
    cfg.tokens = String::new();
    cfg.token_mode = false;
    cfg.ignore_patterns.push("**/*.tmp".to_string());
    cfg.unignore_patterns.push("**/important.tmp".to_string());

    // Valid priority rule
    cfg.priority_rules.push(PriorityRule {
        pattern: "*.rs".to_string(),
        score: 500,
    });

    // Valid binary extensions
    cfg.binary_extensions.push("bin".to_string());

    // Valid git_boost_max
    cfg.git_boost_max = Some(500);

    // Valid max_git_depth
    cfg.max_git_depth = 200;

    // Validate should pass
    let result = cfg.validate();
    assert!(result.is_ok());
}

#[test]
fn test_merge_binary_extensions() {
    let mut cfg = YekConfig::default();
    cfg.binary_extensions = vec!["custom_ext".to_string(), "exe".to_string()];

    // Simulate the merging behavior in init_config()
    let mut merged_bins = BINARY_FILE_EXTENSIONS
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    merged_bins.append(&mut cfg.binary_extensions.clone());
    cfg.binary_extensions = merged_bins
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Check that binary_extensions contains both default and user-provided extensions, without duplicates
    let mut expected_extensions = BINARY_FILE_EXTENSIONS
        .iter()
        .map(|s| s.to_string())
        .collect::<std::collections::HashSet<_>>();
    expected_extensions.insert("custom_ext".to_string());
    expected_extensions.insert("exe".to_string());

    let extensions_set: std::collections::HashSet<_> = cfg.binary_extensions.into_iter().collect();
    assert_eq!(extensions_set, expected_extensions);
}

#[test]
fn test_merge_ignore_patterns() {
    let mut cfg = YekConfig::default();
    cfg.ignore_patterns = vec!["**/*.log".to_string(), "**/*.tmp".to_string()];
    cfg.unignore_patterns = vec!["**/important.log".to_string()];

    // Simulate the merging behavior in init_config()
    let mut ignore = DEFAULT_IGNORE_PATTERNS
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    ignore.extend(cfg.ignore_patterns.clone());
    cfg.ignore_patterns = ignore;

    // Apply unignore patterns
    cfg.ignore_patterns
        .extend(cfg.unignore_patterns.iter().map(|pat| format!("!{}", pat)));

    // Expected ignore patterns
    let mut expected_patterns = DEFAULT_IGNORE_PATTERNS
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    expected_patterns.extend(vec!["**/*.log".to_string(), "**/*.tmp".to_string()]);
    expected_patterns.push("!**/important.log".to_string());

    assert_eq!(cfg.ignore_patterns, expected_patterns);
}

#[test]
fn test_input_dirs_default() {
    let mut cfg = YekConfig::default();

    // Simulate init_config() behavior
    if cfg.input_dirs.is_empty() {
        cfg.input_dirs.push(".".to_string());
    }

    assert_eq!(cfg.input_dirs, vec![".".to_string()]);
}

#[test]
fn test_get_checksum_empty_dirs() {
    let input_dirs: Vec<String> = vec![];
    let checksum = YekConfig::get_checksum(&input_dirs);
    // Checksum should be computed even if input_dirs is empty
    assert!(!checksum.is_empty());

    // Now test with non-existent directory
    let input_dirs = vec!["non_existent_dir".to_string()];
    let checksum = YekConfig::get_checksum(&input_dirs);
    // Again, checksum should be computed
    assert!(!checksum.is_empty());
}

#[test]
fn test_get_checksum_empty_directory() {
    // Create a temporary empty directory
    let temp_dir = std::env::temp_dir().join("yek_test_empty_dir");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir(&temp_dir).unwrap();

    let input_dirs = vec![temp_dir.to_string_lossy().to_string()];
    let checksum = YekConfig::get_checksum(&input_dirs);
    // Checksum should be computed
    assert!(!checksum.is_empty());

    // Clean up
    fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_validate_invalid_max_size_format() {
    let mut cfg = YekConfig::default();
    cfg.max_size = "invalid_size".to_string();

    let result = cfg.validate();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("max_size: Invalid size format"));
}

#[test]
fn test_validate_valid_tokens() {
    let mut cfg = YekConfig::default();
    cfg.token_mode = true;
    cfg.tokens = "1000".to_string();

    let result = cfg.validate();
    assert!(result.is_ok());

    // Test with tokens ending with 'k'
    cfg.tokens = "2k".to_string();
    let result = cfg.validate();
    assert!(result.is_ok());
}
