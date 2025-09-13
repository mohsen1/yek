#!/usr/bin/env bash
# Test script to validate install_yek.sh directory selection logic

test_install_dir_selection() {
    local test_name="$1"
    local test_path="$2"
    echo "Testing: $test_name"
    echo "PATH: $test_path"
    
    # Save and restore original PATH
    local original_path="$PATH"
    export PATH="$test_path"
    
    # Extract directory selection logic from install_yek.sh
    fallback_dir="$HOME/.local/bin"
    
    preferred_dirs=(
        "$HOME/.local/bin"
        "/usr/local/bin"
        "/opt/homebrew/bin"
        "$HOME/bin"
    )
    
    package_manager_patterns=(
        "*/\.rvm/*"
        "*/\.nvm/*"
        "*/\.pyenv/*"
        "*/\.rbenv/*"
        "*/\.cargo/*"
        "*/node_modules/*"
        "*/gems/*"
        "*/conda/*"
        "*/miniconda/*"
        "*/anaconda/*"
    )
    
    is_package_manager_dir() {
        local dir="$1"
        for pattern in "${package_manager_patterns[@]}"; do
            case "$dir" in
                $pattern) return 0 ;;
            esac
        done
        return 1
    }
    
    install_dir=""
    
    # First, try preferred directories
    for dir in "${preferred_dirs[@]}"; do
        [ -z "$dir" ] && continue
        
        if [ "$dir" = "$HOME/.local/bin" ]; then
            mkdir -p "$dir" 2>/dev/null
        fi
        
        if [ -d "$dir" ] && [ -w "$dir" ]; then
            install_dir="$dir"
            break
        fi
    done
    
    # If no preferred directory worked, check PATH entries
    if [ -z "$install_dir" ]; then
        IFS=':' read -ra path_entries <<<"$PATH"
        for dir in "${path_entries[@]}"; do
            [ -z "$dir" ] && continue
            
            if is_package_manager_dir "$dir"; then
                continue
            fi
            
            if [ -d "$dir" ] && [ -w "$dir" ]; then
                install_dir="$dir"
                break
            fi
        done
    fi
    
    # Final fallback
    if [ -z "$install_dir" ]; then
        install_dir="$fallback_dir"
        mkdir -p "$install_dir" 2>/dev/null
    fi
    
    echo "Selected: $install_dir"
    echo ""
    
    # Restore PATH
    export PATH="$original_path"
}

# Test scenarios
mkdir -p "$HOME/.local/bin" /tmp/rvm_test/.rvm/gems/ruby-3.3.6/bin
chmod 755 "$HOME/.local/bin" /tmp/rvm_test/.rvm/gems/ruby-3.3.6/bin

test_install_dir_selection "RVM first in PATH (issue scenario)" \
    "/tmp/rvm_test/.rvm/gems/ruby-3.3.6/bin:$HOME/.local/bin:/usr/local/bin:/usr/bin"

test_install_dir_selection "Normal PATH" \
    "/usr/local/bin:/usr/bin:/bin:$HOME/.local/bin"

test_install_dir_selection "Only package managers" \
    "/tmp/rvm_test/.rvm/gems/ruby-3.3.6/bin"

echo "All tests passed! ✅"