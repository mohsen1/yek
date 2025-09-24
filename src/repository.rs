use crate::models::{InputConfig, RepositoryInfo};
use anyhow::{anyhow, Result};
use git2;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::SystemTime,
};

/// Trait for file system operations
pub trait FileSystem {
    /// Check if a path exists
    fn path_exists(&self, path: &Path) -> bool;

    /// Check if a path is a file
    fn is_file(&self, path: &Path) -> bool;

    /// Check if a path is a directory
    fn is_directory(&self, path: &Path) -> bool;

    /// Read file contents as bytes
    fn read_file(&self, path: &Path) -> Result<Vec<u8>>;

    /// Read directory entries
    fn read_directory(&self, path: &Path) -> Result<Vec<PathBuf>>;

    /// Get file metadata
    fn get_file_metadata(&self, path: &Path) -> Result<FileMetadata>;

    /// Check if path is a symlink
    fn is_symlink(&self, path: &Path) -> bool;

    /// Resolve symlink safely (preventing infinite loops)
    fn resolve_symlink(&self, path: &Path) -> Result<PathBuf>;
}

/// Trait for Git operations
pub trait GitOperations {
    /// Check if a path is a git repository
    fn is_git_repository(&self, path: &Path) -> bool;

    /// Get commit times for files in the repository
    fn get_file_commit_times(&self, max_commits: usize) -> Result<HashMap<String, u64>>;

    /// Get repository root path
    fn get_repository_root(&self) -> Result<PathBuf>;
}

/// Real file system implementation
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        fs::read(path).map_err(|e| anyhow!("Failed to read file '{}': {}", path.display(), e))
    }

    fn read_directory(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            entries.push(entry.path());
        }
        Ok(entries)
    }

    fn get_file_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let metadata = fs::metadata(path)?;
        let modified = metadata.modified()?;
        let size = metadata.len();

        Ok(FileMetadata {
            size,
            modified,
            is_file: metadata.is_file(),
            is_directory: metadata.is_dir(),
            is_symlink: metadata.is_symlink(),
        })
    }

    fn is_symlink(&self, path: &Path) -> bool {
        fs::symlink_metadata(path)
            .map(|m| m.is_symlink())
            .unwrap_or(false)
    }

    fn resolve_symlink(&self, path: &Path) -> Result<PathBuf> {
        // Prevent infinite loops by tracking visited paths
        let mut visited = std::collections::HashSet::new();
        let mut current = path.to_path_buf();

        for _ in 0..100 {
            // Reasonable limit to prevent infinite loops
            if !self.is_symlink(&current) {
                break;
            }

            if !visited.insert(current.clone()) {
                return Err(anyhow!("Symlink loop detected at '{}'", current.display()));
            }

            current = fs::read_link(&current)?;
        }

        Ok(current)
    }
}

/// Real Git operations implementation
pub struct RealGitOperations {
    repository: git2::Repository,
    repo_path: PathBuf,
}

impl RealGitOperations {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let repository = git2::Repository::open(repo_path).map_err(|e| {
            anyhow!(
                "Failed to open git repository at '{}': {}",
                repo_path.display(),
                e
            )
        })?;

        Ok(Self {
            repository,
            repo_path: repo_path.to_path_buf(),
        })
    }
}

impl GitOperations for RealGitOperations {
    fn is_git_repository(&self, _path: &Path) -> bool {
        true // We already verified this when creating the instance
    }

    fn get_file_commit_times(&self, max_commits: usize) -> Result<HashMap<String, u64>> {
        let mut revwalk = self
            .repository
            .revwalk()
            .map_err(|e| anyhow!("Failed to create revision walker: {}", e))?;

        revwalk
            .push_head()
            .map_err(|e| anyhow!("Failed to push HEAD to revision walker: {}", e))?;

        revwalk
            .set_sorting(git2::Sort::TIME)
            .map_err(|e| anyhow!("Failed to set sorting for revision walker: {}", e))?;

        let mut commit_times = HashMap::new();

        for (commits_processed, oid_result) in revwalk.enumerate() {
            if commits_processed >= max_commits {
                break;
            }

            let oid = oid_result.map_err(|e| anyhow!("Error during revision walk: {}", e))?;

            let commit = self
                .repository
                .find_commit(oid)
                .map_err(|e| anyhow!("Failed to find commit for OID {:?}: {}", oid, e))?;

            let tree = commit
                .tree()
                .map_err(|e| anyhow!("Failed to get tree for commit {:?}: {}", oid, e))?;

            let time = commit.time().seconds() as u64;

            // Walk the tree to get file paths
            tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
                if let Some(name) = entry.name() {
                    if entry.kind() == Some(git2::ObjectType::Blob) {
                        let full_path = format!("{}{}", root, name);
                        commit_times.entry(full_path).or_insert(time);
                    }
                }
                git2::TreeWalkResult::Ok
            })
            .map_err(|e| anyhow!("Failed to walk commit tree: {}", e))?;
        }

        Ok(commit_times)
    }

    fn get_repository_root(&self) -> Result<PathBuf> {
        Ok(self.repo_path.clone())
    }
}

/// File metadata structure
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: SystemTime,
    pub is_file: bool,
    pub is_directory: bool,
    pub is_symlink: bool,
}

/// Repository factory for creating repository instances
pub struct RepositoryFactory {
    file_system: Box<dyn FileSystem + Send + Sync>,
    git_cache: OnceLock<HashMap<PathBuf, Arc<dyn GitOperations + Send + Sync>>>,
}

impl Default for RepositoryFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl RepositoryFactory {
    pub fn new() -> Self {
        Self {
            file_system: Box::new(RealFileSystem),
            git_cache: OnceLock::new(),
        }
    }

    pub fn with_file_system(file_system: Box<dyn FileSystem + Send + Sync>) -> Self {
        Self {
            file_system,
            git_cache: OnceLock::new(),
        }
    }

    /// Create repository info for a given path
    pub fn create_repository_info(
        &self,
        root_path: &Path,
        config: &InputConfig,
    ) -> Result<RepositoryInfo> {
        let resolved_path = if self.file_system.is_symlink(root_path) {
            self.file_system.resolve_symlink(root_path)?
        } else {
            root_path.to_path_buf()
        };

        let is_git_repo = self.is_git_repository(&resolved_path);
        let mut repo_info = RepositoryInfo::new(resolved_path, is_git_repo);

        if is_git_repo {
            if let Some(git_ops) = self.get_git_operations(&repo_info.root_path)? {
                let commit_times = git_ops.get_file_commit_times(config.max_git_depth as usize)?;
                repo_info.commit_times = commit_times;
            }
        }

        Ok(repo_info)
    }

    /// Check if a path is a git repository
    fn is_git_repository(&self, path: &Path) -> bool {
        // Walk up the directory tree to find a .git folder
        let mut current = path.to_path_buf();
        while current.components().count() > 0 {
            if current.join(".git").exists() {
                return true;
            }
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
        false
    }

    /// Get cached git operations for a repository
    fn get_git_operations(&self, repo_path: &Path) -> Result<Option<Arc<dyn GitOperations>>> {
        // Try to get from cache first
        if let Some(cached) = self
            .git_cache
            .get()
            .and_then(|cache| cache.get(repo_path).cloned())
        {
            return Ok(Some(cached));
        }

        // Create new git operations instance
        if let Ok(git_ops) = RealGitOperations::new(repo_path) {
            // Cache it for future use
            if let Some(_cache) = self.git_cache.get() {
                // Note: In a real implementation, you'd need a mutable cache
                // This is a simplified version
            }
            Ok(Some(Arc::new(git_ops)))
        } else {
            Ok(None)
        }
    }
}

/// Global repository factory instance
static REPOSITORY_FACTORY: OnceLock<RepositoryFactory> = OnceLock::new();

/// Get the global repository factory
pub fn get_repository_factory() -> &'static RepositoryFactory {
    REPOSITORY_FACTORY.get_or_init(RepositoryFactory::new)
}

/// Convenience functions for common operations
pub mod convenience {
    use super::*;

    /// Read file content safely with UTF-8 validation
    pub fn read_file_content_safe(path: &Path, fs: &dyn FileSystem) -> Result<String> {
        let bytes = fs.read_file(path)?;
        String::from_utf8(bytes)
            .map_err(|e| anyhow!("File '{}' contains invalid UTF-8: {}", path.display(), e))
    }

    /// Check if file should be ignored based on patterns
    pub fn should_ignore_file(path: &Path, patterns: &[glob::Pattern]) -> bool {
        let path_str = path.to_string_lossy();
        patterns.iter().any(|pattern| pattern.matches(&path_str))
    }

    /// Get relative path from base directory
    pub fn get_relative_path(full_path: &Path, base_path: &Path) -> Result<PathBuf> {
        full_path
            .strip_prefix(base_path)
            .map(|p| p.to_path_buf())
            .map_err(|e| {
                anyhow!(
                    "Path '{}' is not relative to '{}': {}",
                    full_path.display(),
                    base_path.display(),
                    e
                )
            })
    }
}
