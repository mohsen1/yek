use std::collections::HashMap;
use std::path::Path;
use tempfile::tempdir;
use yek::config::YekConfig;
use yek::parallel::{normalize_path, process_files_parallel};

#[test]
fn test_normalize_path_unix_style() {
    let input = Path::new("/usr/local/bin");
    let base = Path::new("/"); // Dummy base path
    let expected = "usr/local/bin".to_string();
    assert_eq!(normalize_path(input, base), expected);
}

#[test]
fn test_normalize_path_windows_style() {
    let input = Path::new("C:\\Program Files\\Yek");
    let base = Path::new("C:\\"); // Dummy base for normalization
    let expected = "C:\\Program Files\\Yek".to_string();
    assert_eq!(normalize_path(input, base), expected);
}

#[test]
fn test_process_files_parallel_empty() {
    let temp_dir = tempdir().expect("failed to create temp dir");
    let config = YekConfig::extend_config_with_defaults(
        vec![temp_dir.path().to_string_lossy().to_string()],
        ".".to_string(),
    );
    let boosts: HashMap<String, i32> = HashMap::new();
    let result = process_files_parallel(temp_dir.path(), &config, &boosts)
        .expect("process_files_parallel failed");
    assert_eq!(result.len(), 0);
}

#[test]
fn test_process_files_parallel_with_files() {
    use std::fs;
    let temp_dir = tempdir().expect("failed to create temp dir");
    let file_names = vec!["a.txt", "b.txt", "c.txt"];
    for &file in &file_names {
        let file_path = temp_dir.path().join(file);
        fs::write(file_path, "dummy content").expect("failed to write dummy file");
    }
    let config = YekConfig::extend_config_with_defaults(
        vec![temp_dir.path().to_string_lossy().to_string()],
        ".".to_string(),
    );
    let boosts: HashMap<String, i32> = HashMap::new();
    let base = temp_dir.path();
    let result =
        process_files_parallel(base, &config, &boosts).expect("process_files_parallel failed");
    assert_eq!(result.len(), file_names.len());
    let names: Vec<&str> = result.iter().map(|pf| pf.rel_path.as_str()).collect();
    for file in file_names {
        assert!(names.contains(&file), "Missing file: {}", file);
    }
}
