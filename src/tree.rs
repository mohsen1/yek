use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

/// Generate a directory tree from a list of file paths
pub fn generate_tree(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return String::new();
    }

    // Pre-allocate string with estimated capacity
    let total_path_len: usize = paths.iter().map(|p| p.to_string_lossy().len()).sum();
    let mut output = String::with_capacity(total_path_len + paths.len() * 8);

    // Build a tree structure from the paths
    let mut tree = TreeNode::new();

    // Add all paths to the tree
    for path in paths {
        add_path_to_tree(&mut tree, path);
    }

    // Generate the tree output
    output.push_str("Directory structure:\n");
    render_tree(&tree, &mut output, "", true);
    output.push('\n'); // Add blank line after tree

    output
}

#[derive(Debug)]
struct TreeNode {
    name: String,
    children: HashMap<String, TreeNode>,
    is_file: bool,
}

impl TreeNode {
    fn new() -> Self {
        TreeNode {
            name: String::new(),
            children: HashMap::new(),
            is_file: false,
        }
    }

    fn new_with_name(name: String, is_file: bool) -> Self {
        TreeNode {
            name,
            children: HashMap::new(),
            is_file,
        }
    }
}

/// Filter out Windows drive prefixes and root directory components to get logical path components.
/// This ensures that paths like "C:\repo\src\lib.rs" become ["repo", "src", "lib.rs"]
/// instead of ["C:", "\", "repo", "src", "lib.rs"].
///
/// Note: This function is public for testing purposes only.
pub fn clean_path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Prefix(_) | Component::RootDir => None,
            Component::CurDir => None, // Skip "." components
            Component::ParentDir => Some("..".to_string()), // Keep ".." components
            Component::Normal(os_str) => Some(os_str.to_string_lossy().to_string()),
        })
        .collect()
}

/// Add a path to the tree structure.
///
/// This function processes file paths by treating:
/// - All intermediate components as directories
/// - The final component as a file (unless explicitly marked as directory)
///
/// This approach avoids filesystem checks with `Path::is_file()` which can fail
/// for relative paths or non-existent files. When processing a list of file paths
/// from a file processor, the final component should always be treated as a file.
///
/// # Arguments
/// * `root` - The root tree node to add the path to
/// * `path` - The path to add to the tree
/// * `final_is_file` - Whether to treat the final component as a file (default: true)
///
/// # Future Enhancement
/// For explicit directory support, this function could be extended to accept
/// an additional parameter or use a separate function that marks directories explicitly.
fn add_path_to_tree(root: &mut TreeNode, path: &Path) {
    add_path_to_tree_with_type(root, path, true)
}

/// Internal function to add a path to the tree with explicit control over final component type.
///
/// # Arguments
/// * `root` - The root tree node to add the path to
/// * `path` - The path to add to the tree
/// * `final_is_file` - Whether to treat the final component as a file
fn add_path_to_tree_with_type(root: &mut TreeNode, path: &Path, final_is_file: bool) {
    let components = clean_path_components(path);
    if components.is_empty() {
        return;
    }

    let mut current = root;

    // Process all components, treating intermediate ones as directories
    for (i, name) in components.iter().enumerate() {
        let is_last = i == components.len() - 1;

        if is_last {
            // Handle the final component
            match current.children.get_mut(name) {
                Some(existing_entry) => {
                    // Entry already exists - handle conflicts
                    if existing_entry.is_file && !final_is_file {
                        // Existing file, trying to make it a directory
                        // Directory wins if it will contain children
                        existing_entry.is_file = false;
                    } else if !existing_entry.is_file && final_is_file {
                        // Existing directory, trying to make it a file
                        // Keep as directory if it has children, otherwise make it a file
                        if existing_entry.children.is_empty() {
                            existing_entry.is_file = true;
                        }
                        // If it has children, directory wins and we ignore the file
                    }
                    // If both are files or both are directories, no change needed
                }
                None => {
                    // Create new entry
                    current.children.insert(
                        current.children.insert(
                        name.clone(),
                        TreeNode::new_with_name(name.clone(), final_is_file),
                    );
                        TreeNode::new_with_name(name.clone(), final_is_file),
                    );
                }
            }
        } else {
            // Intermediate component - must be a directory
            let entry = current
                .children
                .entry(name.clone())
                .or_insert_with(|| TreeNode::new_with_name(name.clone(), false));

            // If this was previously marked as a file, convert to directory since we need to traverse it
            if entry.is_file {
                entry.is_file = false;
            }
            current = entry;
        }
    }
}

fn render_child(
    child: &TreeNode,
    output: &mut String,
    current_prefix: &str,
    is_last: bool,
    is_root: bool,
) {
    // Add current prefix (empty for root)
    if !is_root {
        output.push_str(current_prefix);
    }

    // Add tree symbols
    let child_prefix = if is_last { "└── " } else { "├── " };
    output.push_str(child_prefix);
    output.push_str(&child.name);

    // Add '/' for directories
    if !child.is_file {
        output.push('/');
    }
    output.push('\n');

    // Calculate next prefix for children
    let next_prefix = if is_root {
        // For root children, use simple prefix
        if is_last { "    " } else { "│   " }.to_string()
    } else {
        // For non-root children, extend current prefix
        let mut next = String::with_capacity(current_prefix.len() + 4);
        next.push_str(current_prefix);
        next.push_str(if is_last { "    " } else { "│   " });
        next
    };

    // Recursively render this child's children
    render_tree(child, output, &next_prefix, false);
}

fn render_tree(node: &TreeNode, output: &mut String, prefix: &str, is_root: bool) {
    // Sort children: directories first, then files, both alphabetically
    let mut children: Vec<_> = node.children.values().collect();
    children.sort_by(|a, b| {
        // Directories before files
        match (a.is_file, b.is_file) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    // Render each child using the helper function
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        render_child(child, output, prefix, is_last, is_root);
    }
}
