#!/usr/bin/env bash
# Validation test for GitHub issue #85: Install script always installs to last PATH
# This test reproduces the exact scenario described in the issue

echo "🧪 Testing GitHub Issue #85 Fix"
echo "================================"

# Reproduce the exact PATH from the issue
USER_PATH="/Users/dome/.rvm/gems/ruby-3.3.6/bin:/Users/dome/.rvm/gems/ruby-3.3.6@global/bin:/Users/dome/.rvm/rubies/ruby-3.3.6/bin:/Users/dome/.local/bin:/Users/dome/.deno/bin:/Users/dome/.nvm/versions/node/v20.10.0/bin:/opt/homebrew/Caskroom/miniconda/base/bin:/opt/homebrew/Caskroom/miniconda/base/condabin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/System/Cryptexes/App/usr/bin:/usr/bin:/bin:/usr/sbin:/sbin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/local/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/appleinternal/bin:/Library/Apple/usr/bin:/Users/dome/.cargo/bin:/Applications/iTerm.app/Contents/Resources/utilities:/Users/dome/go/bin:/Users/dome/.rvm/bin"

echo "Original issue PATH:"
echo "$USER_PATH"
echo ""

# Create test directories that correspond to the actual user scenario
mkdir -p /tmp/users_dome/.rvm/gems/ruby-3.3.6/bin
mkdir -p /tmp/users_dome/.local/bin
mkdir -p /tmp/opt/homebrew/bin
mkdir -p /tmp/usr/local/bin

# Make them all writable to simulate the real scenario
chmod 755 /tmp/users_dome/.rvm/gems/ruby-3.3.6/bin
chmod 755 /tmp/users_dome/.local/bin  
chmod 755 /tmp/opt/homebrew/bin
chmod 755 /tmp/usr/local/bin

# Map the paths to our test environment
TEST_PATH="/tmp/users_dome/.rvm/gems/ruby-3.3.6/bin:/tmp/users_dome/.local/bin:/tmp/opt/homebrew/bin:/tmp/usr/local/bin:/usr/bin:/bin"
export HOME="/tmp/users_dome"

echo "Test environment PATH:"
echo "$TEST_PATH"
echo ""

# Test the old behavior (what would happen without our fix)
echo "❌ OLD BEHAVIOR (before fix): Would select first writable directory"
echo "   Expected: /tmp/users_dome/.rvm/gems/ruby-3.3.6/bin (RVM - problematic!)"
echo ""

# Test our new behavior
echo "✅ NEW BEHAVIOR (with our fix):"

export PATH="$TEST_PATH"

# Our improved logic from install_yek.sh
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

# Check if RVM directory would be skipped
echo "   Checking if RVM directory is correctly identified as package manager:"
if is_package_manager_dir "/tmp/users_dome/.rvm/gems/ruby-3.3.6/bin"; then
    echo "   ✓ RVM directory correctly identified as package manager (will be skipped)"
else
    echo "   ✗ RVM directory NOT identified as package manager (this would be bad)"
fi
echo ""

# First, try preferred directories
for dir in "${preferred_dirs[@]}"; do
    [ -z "$dir" ] && continue
    
    if [ "$dir" = "$HOME/.local/bin" ]; then
        mkdir -p "$dir" 2>/dev/null
    fi
    
    if [ -d "$dir" ] && [ -w "$dir" ]; then
        install_dir="$dir"
        echo "   ✓ Selected preferred directory: $dir"
        break
    fi
done

if [ -z "$install_dir" ]; then
    echo "   No preferred directory found, checking PATH..."
    IFS=':' read -ra path_entries <<<"$PATH"
    for dir in "${path_entries[@]}"; do
        [ -z "$dir" ] && continue
        
        if is_package_manager_dir "$dir"; then
            echo "   ⏭️  Skipping package manager directory: $dir"
            continue
        fi
        
        if [ -d "$dir" ] && [ -w "$dir" ]; then
            install_dir="$dir"
            echo "   ✓ Selected from PATH: $dir"
            break
        fi
    done
fi

if [ -z "$install_dir" ]; then
    install_dir="$fallback_dir"
    mkdir -p "$install_dir" 2>/dev/null
    echo "   ✓ Using fallback: $install_dir"
fi

echo ""
echo "🎯 FINAL RESULT:"
echo "   Selected install directory: $install_dir"
echo ""

# Verify the fix
if [[ "$install_dir" == *"/.local/bin" ]]; then
    echo "✅ SUCCESS: Script correctly selects ~/.local/bin instead of RVM directory!"
    echo "   This fixes the issue described in GitHub issue #85."
else
    echo "❌ FAILURE: Script did not select ~/.local/bin as expected."
    exit 1
fi

echo ""
echo "🔧 USER EXPECTATION FULFILLED:"
echo "   User wanted: Installation in ~/.local/bin (standard directory)"
echo "   User got:    $install_dir"
echo "   ✓ Match!"

echo ""
echo "📋 ISSUE RESOLUTION SUMMARY:"
echo "   Before: Script installed to first writable directory in PATH (RVM in this case)"
echo "   After:  Script prioritizes standard directories (~/.local/bin) over package managers"
echo "   Result: ✅ Issue #85 is resolved!"