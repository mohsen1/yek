#!/usr/bin/env bash

# Set UTF-8 locale globally for all functions
export LC_ALL=en_US.UTF-8
export LANG=en_US.UTF-8

# Initialize git repo with proper UTF-8 handling
git_init() {
    local repo_dir=$1
    git -c core.quotepath=false init "$repo_dir"
}

# Configure git user settings
git_config_user() {
    local repo_dir=$1
    local name=$2
    local email=$3
    git -c core.quotepath=false -C "$repo_dir" config user.name "$name"
    git -c core.quotepath=false -C "$repo_dir" config user.email "$email"
}

# Add files to git
git_add() {
    local repo_dir=$1
    local files=${2:-.} # Default to all files if not specified
    git -c core.quotepath=false -C "$repo_dir" add "$files"
}

# Commit changes with message
git_commit() {
    local repo_dir=$1
    local message=$2
    git -c core.quotepath=false -C "$repo_dir" commit -m "$message"
}

# Show git log with proper UTF-8 handling
git_log() {
    local repo_dir=$1
    git -c core.quotepath=false \
        -C "$repo_dir" log \
        --format=%ct \
        --name-only \
        --no-merges \
        --no-renames \
        -- . | tr -cd '[:print:]\n' | iconv -f utf-8 -t utf-8 -c
}

# Create directory with proper error handling
create_dir() {
    local dir=$1
    mkdir -p "$dir" || {
        echo "Failed to create directory: $dir" >&2
        return 1
    }
}

# Write content to file with proper escaping
write_file() {
    local file=$1
    local content=$2
    # Create parent directory if it doesn't exist
    mkdir -p "$(dirname "$file")"
    # Use printf to handle special characters properly
    printf "%s" "$content" >"$file" || {
        echo "Failed to write to file: $file" >&2
        return 1
    }
}

# Setup a temporary repo with git initialized
setup_temp_repo() {
    local repo_dir=$1

    # Initialize git repo
    git_init "$repo_dir"

    # Configure git user
    git_config_user "$repo_dir" "test" "test@test"
}

# Create a file in the repo with proper UTF-8 handling
create_repo_file() {
    local repo_dir=$1
    local file_path=$2
    local content=$3

    # Create parent directories if needed
    local full_path="$repo_dir/$file_path"
    local parent_dir
    parent_dir=$(dirname "$full_path")
    create_dir "$parent_dir"

    # Write file content
    write_file "$full_path" "$content"
}

# Helper function to setup a git repo with test files
setup_git_repo() {
    local repo_dir=$1
    echo "Setting up git repo in $repo_dir"

    # Initialize git repo
    git_init "$repo_dir"

    # Configure git user
    git_config_user "$repo_dir" "test" "test@test"

    # Create test directories
    create_dir "$repo_dir/src"
    create_dir "$repo_dir/docs"

    # Create test files
    write_file "$repo_dir/src/main.rs" 'fn main() { println!("Hello"); }'
    write_file "$repo_dir/docs/README.md" "# Documentation\nThis is a test."

    # Add and commit files
    git_add "$repo_dir"
    git_commit "$repo_dir" 'Initial commit'
}

# Helper function to show git log for debugging
show_git_log() {
    local repo_dir=$1
    git_log "$repo_dir"
}

# Helper function that combines setup and log
setup_and_log() {
    local repo_dir=$1
    setup_git_repo "$repo_dir" >/dev/null 2>&1
    show_git_log "$repo_dir"
}

# Main script
case "$1" in
"setup_git_repo")
    setup_git_repo "$2"
    ;;
"show_git_log")
    show_git_log "$2"
    ;;
"setup_and_log")
    setup_and_log "$2"
    ;;
"setup_temp_repo")
    setup_temp_repo "$2"
    ;;
"create_repo_file")
    create_repo_file "$2" "$3" "$4"
    ;;
*)
    echo "Usage: $0 {setup_git_repo|show_git_log|setup_and_log|setup_temp_repo|create_repo_file} <repo_dir> [file_path] [content]"
    exit 1
    ;;
esac
