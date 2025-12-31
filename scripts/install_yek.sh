#!/usr/bin/env bash
set -euo pipefail

REPO_OWNER="mohsen1"
REPO_NAME="yek"

# Determine a sensible default install directory
# We'll check preferred directories first, then fall back to PATH entries,
# avoiding package manager-specific directories when possible.
fallback_dir="$HOME/.local/bin"

# Define preferred directories in order of preference
preferred_dirs=(
    "$HOME/.local/bin"
    "/usr/local/bin"
    "/opt/homebrew/bin"
    "$HOME/bin"
)

# Package manager directories to avoid unless they're in preferred list
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

# Function to check if a path matches package manager patterns
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
    # Skip empty paths
    [ -z "$dir" ] && continue
    
    # Check if directory is writable (create if needed for ~/.local/bin)
    if [ "$dir" = "$HOME/.local/bin" ]; then
        mkdir -p "$dir" 2>/dev/null
    fi
    
    if [ -d "$dir" ] && [ -w "$dir" ]; then
        install_dir="$dir"
        break
    fi
done

# If no preferred directory worked, check PATH entries (excluding package managers)
if [ -z "$install_dir" ]; then
    IFS=':' read -ra path_entries <<<"$PATH"
    for dir in "${path_entries[@]}"; do
        # Skip empty paths
        [ -z "$dir" ] && continue
        
        # Skip package manager directories
        if is_package_manager_dir "$dir"; then
            continue
        fi
        
        # Check if directory is writable
        if [ -d "$dir" ] && [ -w "$dir" ]; then
            install_dir="$dir"
            break
        fi
    done
fi

# Final fallback to ~/.local/bin (create if needed)
if [ -z "$install_dir" ]; then
    install_dir="$fallback_dir"
    mkdir -p "$install_dir" 2>/dev/null
fi

# Ensure the final install directory exists
mkdir -p "$install_dir"

echo "Selected install directory: $install_dir"

# Detect OS and ARCH to choose the correct artifact
OS=$(uname -s)
ARCH=$(uname -m)

case "${OS}_${ARCH}" in
Linux_x86_64)
    # Check glibc version
    GLIBC_VERSION=$(ldd --version 2>&1 | head -n1 | grep -oP 'GLIBC \K[\d.]+' || echo "")
    if [ -z "$GLIBC_VERSION" ] || [ "$(printf '%s\n' "2.31" "$GLIBC_VERSION" | sort -V | head -n1)" = "$GLIBC_VERSION" ]; then
        TARGET="x86_64-unknown-linux-musl"
    else
        TARGET="x86_64-unknown-linux-gnu"
    fi
    ;;
Linux_aarch64)
    # Check glibc version for ARM64
    GLIBC_VERSION=$(ldd --version 2>&1 | head -n1 | grep -oP 'GLIBC \K[\d.]+' || echo "")
    if [ -z "$GLIBC_VERSION" ] || [ "$(printf '%s\n' "2.31" "$GLIBC_VERSION" | sort -V | head -n1)" = "$GLIBC_VERSION" ]; then
        TARGET="aarch64-unknown-linux-musl"
    else
        TARGET="aarch64-unknown-linux-gnu"
    fi
    ;;
Darwin_x86_64)
    TARGET="x86_64-apple-darwin"
    ;;
Darwin_arm64)
    TARGET="aarch64-apple-darwin"
    ;;
*)
    echo "Unsupported OS/ARCH combo: ${OS} ${ARCH}"
    echo "Please check the project's releases for a compatible artifact or build from source."
    exit 1
    ;;
esac

ASSET_NAME="yek-${TARGET}.tar.gz"
echo "OS/ARCH => ${TARGET}"
echo "Asset name => ${ASSET_NAME}"

echo "Fetching latest release info from GitHub..."
LATEST_URL=$(
    curl -s "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" |
        grep "browser_download_url" |
        grep "${ASSET_NAME}" |
        cut -d '"' -f 4
)

if [ -z "${LATEST_URL}" ]; then
    echo "Failed to find a release asset named ${ASSET_NAME} in the latest release."
    echo "Check that your OS/ARCH is built or consider building from source."
    exit 1
fi

echo "Downloading from: ${LATEST_URL}"
curl -L -o "${ASSET_NAME}" "${LATEST_URL}"

echo "Extracting archive..."
tar xzf "${ASSET_NAME}"

# The tar will contain a folder named something like: yek-${TARGET}/yek
echo "Moving binary to ${install_dir}..."
mv "yek-${TARGET}/yek" "${install_dir}/yek"

echo "Making the binary executable..."
chmod +x "${install_dir}/yek"

# Cleanup
rm -rf "yek-${TARGET}" "${ASSET_NAME}"

echo "Installation complete!"

# Check if install_dir is in PATH
if ! echo "$PATH" | tr ':' '\n' | grep -Fx "$install_dir" >/dev/null; then
    echo "NOTE: $install_dir is not in your PATH. Add it by running:"
    echo "  export PATH=\"\$PATH:$install_dir\""
fi

echo "Now you can run: yek --help"
