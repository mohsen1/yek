use std::path::PathBuf;
use tempfile::TempDir;
use yek::models::InputConfig;
use yek::repository::{FileSystem, RealFileSystem, RepositoryFactory};

#[cfg(test)]
mod repository_tests {
    use super::*;

    #[test]
    fn test_real_file_system_path_exists() {
        let fs = RealFileSystem;
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, b"test").unwrap();

        assert!(fs.path_exists(&file_path));
        assert!(!fs.path_exists(&temp_dir.path().join("nonexistent.txt")));
    }

    #[test]
    fn test_real_file_system_is_file() {
        let fs = RealFileSystem;
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, b"test").unwrap();

        assert!(fs.is_file(&file_path));
        assert!(!fs.is_file(temp_dir.path()));
    }

    #[test]
    fn test_real_file_system_is_directory() {
        let fs = RealFileSystem;
        let temp_dir = TempDir::new().unwrap();

        assert!(fs.is_directory(temp_dir.path()));
        assert!(!fs.is_directory(&temp_dir.path().join("nonexistent.txt")));
    }

    #[test]
    fn test_real_file_system_read_file() {
        let fs = RealFileSystem;
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = b"Hello, world!";
        std::fs::write(&file_path, content).unwrap();

        let result = fs.read_file(&file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), content);
    }

    #[test]
    fn test_real_file_system_read_file_nonexistent() {
        let fs = RealFileSystem;
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.txt");

        let result = fs.read_file(&nonexistent_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_real_file_system_read_directory() {
        let fs = RealFileSystem;
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, b"test").unwrap();

        let result = fs.read_directory(temp_dir.path());
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(entries.contains(&file_path));
    }

    #[test]
    fn test_real_file_system_get_file_metadata() {
        let fs = RealFileSystem;
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = b"Hello, world!";
        std::fs::write(&file_path, content).unwrap();

        let result = fs.get_file_metadata(&file_path);
        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.size, content.len() as u64);
        assert!(metadata.is_file);
        assert!(!metadata.is_directory);
    }

    #[test]
    fn test_real_file_system_is_symlink() {
        let fs = RealFileSystem;
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, b"test").unwrap();

        assert!(!fs.is_symlink(&file_path));
    }

    #[test]
    fn test_real_file_system_resolve_symlink() {
        let fs = RealFileSystem;
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let symlink_path = temp_dir.path().join("link.txt");
        std::fs::write(&file_path, b"test").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&file_path, &symlink_path).unwrap();
            let result = fs.resolve_symlink(&symlink_path);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), file_path);
        }
        #[cfg(windows)]
        {
            // On Windows, create a file symlink
            std::os::windows::fs::symlink_file(&file_path, &symlink_path).unwrap();
            let result = fs.resolve_symlink(&symlink_path);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), file_path);
        }
    }

    #[test]
    fn test_repository_factory_new() {
        let _factory = RepositoryFactory::new();
        // Should not panic
    }

    #[test]
    fn test_repository_factory_create_repository_info_non_git() {
        let factory = RepositoryFactory::new();
        let temp_dir = TempDir::new().unwrap();
        let config = InputConfig::default();

        let result = factory.create_repository_info(temp_dir.path(), &config);
        assert!(result.is_ok());
        let repo_info = result.unwrap();
        assert_eq!(repo_info.root_path, temp_dir.path());
        assert!(!repo_info.is_git_repo);
        assert!(repo_info.commit_times.is_empty());
    }

    #[test]
    fn test_repository_factory_create_repository_info_git() {
        let temp_dir = TempDir::new().unwrap();
        // Create .git directory to simulate git repo
        std::fs::create_dir(temp_dir.path().join(".git")).unwrap();

        let factory = RepositoryFactory::new();
        let config = InputConfig::default();

        let result = factory.create_repository_info(temp_dir.path(), &config);
        assert!(result.is_ok());
        let repo_info = result.unwrap();
        assert_eq!(repo_info.root_path, temp_dir.path());
        assert!(repo_info.is_git_repo);
    }

    #[test]
    fn test_convenience_read_file_content_safe() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = "Hello, world!";
        std::fs::write(&file_path, content).unwrap();

        let result =
            yek::repository::convenience::read_file_content_safe(&file_path, &RealFileSystem);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), content);
    }

    #[test]
    fn test_convenience_read_file_content_safe_invalid_utf8() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.bin");
        let content = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8
        std::fs::write(&file_path, &content).unwrap();

        let result =
            yek::repository::convenience::read_file_content_safe(&file_path, &RealFileSystem);
        assert!(result.is_err());
    }

    #[test]
    fn test_convenience_should_ignore_file() {
        use glob::Pattern;
        let patterns = vec![Pattern::new("*.txt").unwrap()];

        assert!(yek::repository::convenience::should_ignore_file(
            &PathBuf::from("test.txt"),
            &patterns
        ));
        assert!(!yek::repository::convenience::should_ignore_file(
            &PathBuf::from("test.rs"),
            &patterns
        ));
    }

    #[test]
    fn test_convenience_get_relative_path() {
        let base = PathBuf::from("/home/user/project");
        let full = PathBuf::from("/home/user/project/src/main.rs");

        let result = yek::repository::convenience::get_relative_path(&full, &base);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("src/main.rs"));
    }

    #[test]
    fn test_convenience_get_relative_path_not_relative() {
        let base = PathBuf::from("/home/user/project");
        let full = PathBuf::from("/other/path/file.txt");

        let result = yek::repository::convenience::get_relative_path(&full, &base);
        assert!(result.is_err());
    }
}
