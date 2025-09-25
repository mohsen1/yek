use serde::{Deserialize, Serialize};
use std::path::Path;

/// File categories for sorting and prioritization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FileCategory {
    /// Main application or library source code
    Source,
    /// Test files and testing related code
    Test,
    /// Configuration files (yaml, toml, json, etc.)
    Configuration,
    /// Documentation files (markdown, rst, docs folder)
    Documentation,
    /// Other files that don't fit into above categories
    #[default]
    Other,
}

impl FileCategory {
    /// Get the default priority offset for this category
    pub fn default_priority_offset(self) -> i32 {
        match self {
            FileCategory::Configuration => 5,
            FileCategory::Test => 10,
            FileCategory::Documentation => 15,
            FileCategory::Source => 20,
            FileCategory::Other => 1,
        }
    }

    /// Get the category name as a string for display/debug purposes
    pub fn name(self) -> &'static str {
        match self {
            FileCategory::Source => "source",
            FileCategory::Test => "test",
            FileCategory::Configuration => "configuration",
            FileCategory::Documentation => "documentation",
            FileCategory::Other => "other",
        }
    }
}

/// Configuration for category-based priority weights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryWeights {
    /// Priority offset for source files
    pub source: i32,
    /// Priority offset for test files
    pub test: i32,
    /// Priority offset for configuration files
    pub configuration: i32,
    /// Priority offset for documentation files
    pub documentation: i32,
    /// Priority offset for other files
    pub other: i32,
}

impl Default for CategoryWeights {
    fn default() -> Self {
        Self {
            source: FileCategory::Source.default_priority_offset(),
            test: FileCategory::Test.default_priority_offset(),
            configuration: FileCategory::Configuration.default_priority_offset(),
            documentation: FileCategory::Documentation.default_priority_offset(),
            other: FileCategory::Other.default_priority_offset(),
        }
    }
}

impl CategoryWeights {
    /// Get the priority offset for a given category
    pub fn get_offset(&self, category: FileCategory) -> i32 {
        match category {
            FileCategory::Source => self.source,
            FileCategory::Test => self.test,
            FileCategory::Configuration => self.configuration,
            FileCategory::Documentation => self.documentation,
            FileCategory::Other => self.other,
        }
    }
}

/// Categorize a file based on its path and extension using heuristics
pub fn categorize_file(file_path: &str) -> FileCategory {
    let path = Path::new(file_path);

    // Get file extension (lowercase)
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase());

    // Get file name (lowercase)
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Convert path to lowercase string for pattern matching
    let path_lower = file_path.to_lowercase();

    // Check for test files first (most specific)
    if is_test_file(&path_lower, &file_name, &extension) {
        return FileCategory::Test;
    }

    // Check for configuration files
    if is_configuration_file(&path_lower, &file_name, &extension) {
        return FileCategory::Configuration;
    }

    // Check for documentation files
    if is_documentation_file(&path_lower, &file_name, &extension) {
        return FileCategory::Documentation;
    }

    // Check for source files
    if is_source_file(&path_lower, &extension) {
        return FileCategory::Source;
    }

    // Default to Other
    FileCategory::Other
}

/// Check if a file is a test file based on path patterns and naming conventions
fn is_test_file(path_lower: &str, file_name: &str, extension: &Option<String>) -> bool {
    // Test directory patterns - check both absolute and relative paths
    let test_directories = [
        "/test/",
        "/tests/",
        "/__test__/",
        "/__tests__/",
        "/spec/",
        "/specs/",
        "\\test\\",
        "\\tests\\",
        "\\__test__\\",
        "\\__tests__\\",
        "\\spec\\",
        "\\specs\\",
        "/e2e/",
        "/integration/",
        "/unit/",
        "\\e2e\\",
        "\\integration\\",
        "\\unit\\",
        // Also check for patterns that start at the beginning of the path
        "test/",
        "tests/",
        "__test__/",
        "__tests__/",
        "spec/",
        "specs/",
        "e2e/",
        "integration/",
        "unit/",
    ];

    // Check if path contains test directories
    for test_dir in &test_directories {
        if path_lower.contains(test_dir) {
            return true;
        }
    }

    // Check for test file naming patterns
    let test_patterns = [
        "_test.", ".test.", "_spec.", ".spec.", "_e2e.", ".e2e.", "test_", "spec_", "e2e_",
    ];

    for pattern in &test_patterns {
        if file_name.contains(pattern) {
            return true;
        }
    }

    // Special cases for specific languages/frameworks
    match extension.as_deref() {
        Some("test") | Some("spec") => return true,
        Some("js") | Some("ts") | Some("jsx") | Some("tsx") => {
            if file_name.ends_with(".test.js")
                || file_name.ends_with(".test.ts")
                || file_name.ends_with(".spec.js")
                || file_name.ends_with(".spec.ts")
                || file_name.ends_with(".test.jsx")
                || file_name.ends_with(".test.tsx")
                || file_name.ends_with(".spec.jsx")
                || file_name.ends_with(".spec.tsx")
            {
                return true;
            }
        }
        Some("py") => {
            if file_name.starts_with("test_") || file_name.ends_with("_test.py") {
                return true;
            }
        }
        Some("rs") => {
            // Rust integration tests
            if path_lower.contains("/tests/")
                || path_lower.contains("\\tests\\")
                || path_lower.starts_with("tests/")
            {
                return true;
            }
        }
        Some("java") => {
            if file_name.ends_with("test.java") || file_name.ends_with("tests.java") {
                return true;
            }
        }
        _ => {}
    }

    false
}

/// Check if a file is a configuration file
fn is_configuration_file(path_lower: &str, file_name: &str, extension: &Option<String>) -> bool {
    // Configuration file extensions
    let config_extensions = [
        "toml",
        "yaml",
        "yml",
        "json",
        "ini",
        "cfg",
        "conf",
        "config",
        "properties",
        "env",
        "rc",
        "lock",
        "sum",
        "mod",
        "makefile",
    ];

    if let Some(ext) = extension {
        if config_extensions.contains(&ext.as_str()) {
            return true;
        }
    }

    // Specific configuration file names
    let config_files = [
        "makefile",
        "dockerfile",
        "containerfile",
        "rakefile",
        "gemfile",
        "podfile",
        "vagrantfile",
        "brewfile",
        "procfile",
        "nixfile",
        "package.json",
        "composer.json",
        "project.json",
        "cargo.toml",
        "pyproject.toml",
        "poetry.toml",
        "setup.cfg",
        "tox.ini",
        "docker-compose.yml",
        "docker-compose.yaml",
        "docker-stack.yml",
        "docker-stack.yaml",
        ".gitignore",
        ".gitattributes",
        ".gitmodules",
        ".dockerignore",
        ".eslintrc",
        ".eslintrc.json",
        ".eslintrc.yml",
        ".eslintrc.yaml",
        ".prettierrc",
        ".babelrc",
        ".editorconfig",
        ".travis.yml",
        ".github",
        "appveyor.yml",
        "azure-pipelines.yml",
        "tsconfig.json",
        "jsconfig.json",
        "webpack.config.js",
        "rollup.config.js",
        "vite.config.js",
        "vite.config.ts",
        "next.config.js",
        "nuxt.config.js",
        "tailwind.config.js",
        "postcss.config.js",
        "requirements.txt",
        "setup.py",
        "setup.cfg",
        "manifest.in",
        "pipfile",
        "build.gradle",
        "pom.xml",
        "build.xml",
        "ivy.xml",
        "cmake",
        "cmakelist.txt",
        "meson.build",
    ];

    for config_name in &config_files {
        if file_name == *config_name {
            return true;
        }
    }

    // Check for dotfiles (usually configuration)
    if file_name.starts_with('.') && !file_name.starts_with("..") {
        // Exclude some common non-config dotfiles
        let non_config_dotfiles = [".git", ".gitkeep", ".keep", ".empty", ".placeholder"];
        if !non_config_dotfiles.contains(&file_name) {
            return true;
        }
    }

    // Configuration directories
    let config_dirs = [
        "/config/",
        "/configs/",
        "/.config/",
        "/configuration/",
        "/settings/",
        "\\config\\",
        "\\configs\\",
        "\\.config\\",
        "\\configuration\\",
        "\\settings\\",
        // Also check for patterns that start at the beginning of the path
        "config/",
        "configs/",
        ".config/",
        "configuration/",
        "settings/",
    ];

    for config_dir in &config_dirs {
        if path_lower.contains(config_dir) {
            return true;
        }
    }

    false
}

/// Check if a file is a documentation file
fn is_documentation_file(path_lower: &str, file_name: &str, extension: &Option<String>) -> bool {
    // Documentation file extensions
    let doc_extensions = ["md", "rst", "txt", "adoc", "asciidoc", "org", "wiki"];

    if let Some(ext) = extension {
        if doc_extensions.contains(&ext.as_str()) {
            return true;
        }
    }

    // Documentation file names
    let doc_files = [
        "readme",
        "readme.txt",
        "readme.md",
        "readme.rst",
        "changelog",
        "changelog.md",
        "changelog.txt",
        "changelog.rst",
        "license",
        "license.txt",
        "license.md",
        "contributing",
        "contributing.md",
        "contributing.txt",
        "authors",
        "authors.md",
        "authors.txt",
        "todo",
        "todo.md",
        "todo.txt",
        "notes",
        "notes.md",
        "notes.txt",
        "manual",
        "manual.md",
        "manual.txt",
        "guide",
        "guide.md",
        "guide.txt",
        "help",
        "help.md",
        "help.txt",
        "faq",
        "faq.md",
        "faq.txt",
        "install",
        "install.md",
        "install.txt",
        "usage",
        "usage.md",
        "usage.txt",
        "quickstart",
        "getting-started",
    ];

    for doc_name in &doc_files {
        if file_name == *doc_name {
            return true;
        }
    }

    // Documentation directories
    let doc_dirs = [
        "/doc/",
        "/docs/",
        "/documentation/",
        "/manual/",
        "/guide/",
        "/guides/",
        "\\doc\\",
        "\\docs\\",
        "\\documentation\\",
        "\\manual\\",
        "\\guide\\",
        "\\guides\\",
        // Also check for patterns that start at the beginning of the path
        "doc/",
        "docs/",
        "documentation/",
        "manual/",
        "guide/",
        "guides/",
    ];

    for doc_dir in &doc_dirs {
        if path_lower.contains(doc_dir) {
            return true;
        }
    }

    false
}

/// Check if a file is a source code file
fn is_source_file(path_lower: &str, extension: &Option<String>) -> bool {
    // Source code extensions
    let source_extensions = [
        // Popular languages
        "rs", "go", "py", "js", "ts", "jsx", "tsx", "java", "kt", "scala", "c", "cpp", "cc", "cxx",
        "c++", "h", "hpp", "hxx", "h++", "cs", "vb", "fs", "fsx", "fsi", "php", "rb", "pl", "pm",
        "r", "m", "mm", "swift", "dart", "lua", "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd",
        "clj", "cljs", "cljc", "ex", "exs", "erl", "hrl", "hs", "lhs", "elm", "ml", "mli", "ocaml",
        // Web technologies
        "html", "htm", "css", "scss", "sass", "less", "vue", "svelte", // Mobile
        "m", "mm", "swift", "kt", "java", "dart", // System/Low-level
        "asm", "s", "nasm", "v", "vhd", "vhdl", // Functional
        "clj", "cljs", "hs", "elm", "ml", "fs", // Other
        "sql", "graphql", "proto", "thrift", "avro",
    ];

    if let Some(ext) = extension {
        if source_extensions.contains(&ext.as_str()) {
            return true;
        }
    }

    // Source directories (less specific than test directories)
    let source_dirs = [
        "/src/",
        "/source/",
        "/lib/",
        "/libs/",
        "/app/",
        "/application/",
        "\\src\\",
        "\\source\\",
        "\\lib\\",
        "\\libs\\",
        "\\app\\",
        "\\application\\",
        // Also check for patterns that start at the beginning of the path
        "src/",
        "source/",
        "lib/",
        "libs/",
        "app/",
        "application/",
    ];

    for source_dir in &source_dirs {
        if path_lower.contains(source_dir) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_source_files() {
        assert_eq!(categorize_file("src/main.rs"), FileCategory::Source);
        assert_eq!(categorize_file("lib/utils.py"), FileCategory::Source);
        assert_eq!(categorize_file("app/component.js"), FileCategory::Source);
        assert_eq!(categorize_file("main.go"), FileCategory::Source);
        assert_eq!(categorize_file("index.html"), FileCategory::Source);
    }

    #[test]
    fn test_categorize_test_files() {
        assert_eq!(categorize_file("tests/test_main.py"), FileCategory::Test);
        assert_eq!(categorize_file("test/utils_test.go"), FileCategory::Test);
        assert_eq!(categorize_file("src/component.test.js"), FileCategory::Test);
        assert_eq!(categorize_file("__tests__/unit.js"), FileCategory::Test);
        assert_eq!(categorize_file("spec/feature_spec.rb"), FileCategory::Test);
        assert_eq!(
            categorize_file("e2e/integration.test.ts"),
            FileCategory::Test
        );
    }

    #[test]
    fn test_categorize_configuration_files() {
        assert_eq!(categorize_file("package.json"), FileCategory::Configuration);
        assert_eq!(categorize_file("Cargo.toml"), FileCategory::Configuration);
        assert_eq!(
            categorize_file("docker-compose.yml"),
            FileCategory::Configuration
        );
        assert_eq!(
            categorize_file(".eslintrc.json"),
            FileCategory::Configuration
        );
        assert_eq!(
            categorize_file("config/database.yml"),
            FileCategory::Configuration
        );
        assert_eq!(categorize_file("Makefile"), FileCategory::Configuration);
        assert_eq!(categorize_file(".gitignore"), FileCategory::Configuration);
    }

    #[test]
    fn test_categorize_documentation_files() {
        assert_eq!(categorize_file("README.md"), FileCategory::Documentation);
        assert_eq!(
            categorize_file("docs/guide.rst"),
            FileCategory::Documentation
        );
        assert_eq!(
            categorize_file("CHANGELOG.txt"),
            FileCategory::Documentation
        );
        assert_eq!(categorize_file("LICENSE"), FileCategory::Documentation);
        assert_eq!(
            categorize_file("manual/install.md"),
            FileCategory::Documentation
        );
    }

    #[test]
    fn test_categorize_other_files() {
        assert_eq!(categorize_file("random.unknown"), FileCategory::Other);
        assert_eq!(categorize_file("data.bin"), FileCategory::Other);
        assert_eq!(categorize_file("image.png"), FileCategory::Other);
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
    fn test_category_weights() {
        let weights = CategoryWeights::default();
        assert_eq!(weights.get_offset(FileCategory::Source), 20);
        assert_eq!(weights.get_offset(FileCategory::Test), 10);

        let custom_weights = CategoryWeights {
            source: 100,
            test: 50,
            configuration: 25,
            documentation: 10,
            other: 5,
        };
        assert_eq!(custom_weights.get_offset(FileCategory::Source), 100);
        assert_eq!(custom_weights.get_offset(FileCategory::Test), 50);
    }
}
