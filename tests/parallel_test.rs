use anyhow::Result;
use normalize_path::NormalizePath;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use yek::config::YekConfig;
use yek::parallel::process_files_parallel;

#[test]
fn test_normalize_path_unix_style() {
    let input = Path::new("/usr/local/bin");
    let base = Path::new("/"); // Dummy base path
    let expected = "usr/local/bin".to_string();
    assert_eq!(
        input
            .strip_prefix(base)
            .unwrap()
            .normalize()
            .to_string_lossy()
            .to_string(),
        expected
    );
}

#[test]
fn test_normalize_path_windows_style() {
    let input = Path::new("C:\\Program Files\\Yek");
    let base = Path::new("C:\\"); // Dummy base for normalization
    let expected = if cfg!(windows) {
        "Program Files\\Yek".to_string()
    } else {
        "C:/Program Files/Yek".to_string()
    };
    let stripped_path = if input.starts_with(base) {
        input.strip_prefix(base).unwrap()
    } else {
        input
    };
    // Normalize the stripped path, then replace backslashes with forward slashes
    let normalized = stripped_path
        .normalize()
        .to_string_lossy()
        .to_string()
        .replace("\\", "/");
    let expected_normalized = expected.replace("\\", "/");
    assert_eq!(normalized, expected_normalized);
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

#[test]
fn test_process_files_parallel_file_read_error() {
    let temp_dir = tempdir().expect("failed to create temp dir");
    let file_path = temp_dir.path().join("unreadable.txt");
    fs::write(&file_path, "content").expect("failed to write file");

    // Make the file unreadable
    let mut permissions = fs::metadata(&file_path).unwrap().permissions();
    permissions.set_mode(0o000); // No permissions
    fs::set_permissions(&file_path, permissions).unwrap();

    let config = YekConfig::extend_config_with_defaults(
        vec![temp_dir.path().to_string_lossy().to_string()],
        ".".to_string(),
    );
    let boosts: HashMap<String, i32> = HashMap::new();
    let result = process_files_parallel(temp_dir.path(), &config, &boosts)
        .expect("process_files_parallel failed");

    // The unreadable file should be skipped, so the result should be empty
    assert_eq!(result.len(), 0);

    // Restore permissions so the directory can be cleaned up
    let mut permissions = fs::metadata(&file_path).unwrap().permissions();
    permissions.set_mode(0o644); // Read permissions
    fs::set_permissions(&file_path, permissions).unwrap();
}

#[test]
fn test_process_files_parallel_walk_error() {
    let temp_dir = tempdir().expect("failed to create temp dir");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("failed to create subdir");

    // Make the subdir unreadable, causing walk error
    let mut permissions = fs::metadata(&subdir).unwrap().permissions();
    permissions.set_mode(0o000);
    fs::set_permissions(&subdir, permissions).unwrap();

    let config = YekConfig::extend_config_with_defaults(
        vec![temp_dir.path().to_string_lossy().to_string()],
        ".".to_string(),
    );
    let boosts: HashMap<String, i32> = HashMap::new();
    let result = process_files_parallel(temp_dir.path(), &config, &boosts);

    // Walk errors are logged and skipped, not propagated as Err
    assert!(result.is_ok()); // Walk errors are logged and skipped, not propagated as Err
    let processed_files = result.unwrap();
    assert_eq!(processed_files.len(), 0); // No files processed due to walk error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_pattern_single_file() -> Result<()> {
        let temp_dir = tempdir()?;
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "Test content")?;

        let glob_pattern = temp_dir.path().join("*.txt").to_string_lossy().to_string();
        let config = YekConfig::default();
        let boost_map = HashMap::new();

        let result = process_files_parallel(&PathBuf::from(&glob_pattern), &config, &boost_map)?;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].rel_path, file_path.to_string_lossy().to_string());

        Ok(())
    }

    #[test]
    fn test_glob_pattern_multiple_files() -> Result<()> {
        let temp_dir = tempdir()?;

        // Create multiple test files
        let files = vec!["test1.txt", "test2.txt", "other.md"];
        for fname in &files {
            let file_path = temp_dir.path().join(fname);
            let mut file = File::create(&file_path)?;
            writeln!(file, "Test content for {}", fname)?;
        }

        let glob_pattern = temp_dir.path().join("*.txt").to_string_lossy().to_string();
        let config = YekConfig::default();
        let boost_map = HashMap::new();

        let result = process_files_parallel(&PathBuf::from(&glob_pattern), &config, &boost_map)?;
        assert_eq!(result.len(), 2); // Should only match .txt files

        let paths: Vec<String> = result.iter().map(|f| f.rel_path.clone()).collect();
        let test1_path = temp_dir.path().join("test1.txt").to_string_lossy().to_string();
        let test2_path = temp_dir.path().join("test2.txt").to_string_lossy().to_string();
        assert!(paths.contains(&test1_path));
        assert!(paths.contains(&test2_path));

        Ok(())
    }

    #[test]
    fn test_glob_pattern_nested_directories() -> Result<()> {
        let temp_dir = tempdir()?;

        // Create nested directory structure
        let nested_dir = temp_dir.path().join("nested");
        fs::create_dir(&nested_dir)?;

        // Create files in both root and nested directory
        let root_file = temp_dir.path().join("root.rs"); // Use .rs to avoid default ignore patterns
        let nested_file = nested_dir.join("nested.rs");
        let other_file = temp_dir.path().join("other.md");

        for (path, content) in [
            (&root_file, "Root content"),
            (&nested_file, "Nested content"),
            (&other_file, "Other content"),
        ] {
            let mut file = File::create(path)?;
            writeln!(file, "{}", content)?;
        }

        let glob_pattern = temp_dir
            .path()
            .join("**/*.rs") // Changed to .rs files
            .to_string_lossy()
            .to_string();
        let config = YekConfig::default();
        let boost_map = HashMap::new();

        let result = process_files_parallel(&PathBuf::from(&glob_pattern), &config, &boost_map)?;
        assert_eq!(result.len(), 2); // Should match both .rs files

        let paths: Vec<String> = result.iter().map(|f| f.rel_path.clone()).collect();
        let root_path = root_file.to_string_lossy().to_string();
        let nested_path = nested_file.to_string_lossy().to_string();
        assert!(paths.contains(&root_path));
        assert!(paths.contains(&nested_path));

        Ok(())
    }

    #[test]
    fn test_glob_pattern_no_matches() -> Result<()> {
        let temp_dir = tempdir()?;
        let glob_pattern = temp_dir.path().join("*.txt").to_string_lossy().to_string();
        let config = YekConfig::default();
        let boost_map = HashMap::new();

        let result = process_files_parallel(&PathBuf::from(&glob_pattern), &config, &boost_map)?;
        assert!(result.is_empty());

        Ok(())
    }
}
