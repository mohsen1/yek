#!/usr/bin/env bash
set -euo pipefail

REPO_OWNER="prooflinedev"
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
RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest")

# Prints the download URL of the latest release's asset named $1, if any.
asset_url() {
    printf '%s\n' "${RELEASE_JSON}" | grep '"browser_download_url"' | grep -F "/$1\"" | cut -d '"' -f 4 || true
}

LATEST_URL=$(asset_url "${ASSET_NAME}")
SUMS_URL=$(asset_url "SHA256SUMS")

if [ -z "${LATEST_URL}" ]; then
    echo "Failed to find a release asset named ${ASSET_NAME} in the latest release."
    echo "Check that your OS/ARCH is built or consider building from source."
    exit 1
fi

if [ -z "${SUMS_URL}" ]; then
    echo "The latest release has no SHA256SUMS file, so the download cannot be verified."
    exit 1
fi

WORK_DIR=$(mktemp -d)
trap 'rm -rf "${WORK_DIR}"' EXIT

echo "Downloading from: ${LATEST_URL}"
curl -fsSL -o "${WORK_DIR}/${ASSET_NAME}" "${LATEST_URL}"
curl -fsSL -o "${WORK_DIR}/SHA256SUMS" "${SUMS_URL}"

echo "Verifying the SHA-256 checksum..."
EXPECTED_SUM=$(awk -v name="${ASSET_NAME}" '$2 == name || $2 == "*" name { print $1; exit }' "${WORK_DIR}/SHA256SUMS")
if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL_SUM=$(sha256sum "${WORK_DIR}/${ASSET_NAME}" | cut -d ' ' -f 1)
elif command -v shasum >/dev/null 2>&1; then
    ACTUAL_SUM=$(shasum -a 256 "${WORK_DIR}/${ASSET_NAME}" | cut -d ' ' -f 1)
else
    echo "Neither sha256sum nor shasum is installed, so the download cannot be verified."
    exit 1
fi

if [ -z "${EXPECTED_SUM}" ]; then
    echo "SHA256SUMS lists no checksum for ${ASSET_NAME}."
    exit 1
fi
if [ "${EXPECTED_SUM}" != "${ACTUAL_SUM}" ]; then
    echo "Checksum mismatch for ${ASSET_NAME}: expected ${EXPECTED_SUM}, got ${ACTUAL_SUM}."
    exit 1
fi

echo "Extracting archive..."
tar xzf "${WORK_DIR}/${ASSET_NAME}" -C "${WORK_DIR}"

# The archive contains a folder named yek-${TARGET} holding the binary.
echo "Moving binary to ${install_dir}..."
mv "${WORK_DIR}/yek-${TARGET}/yek" "${install_dir}/yek"

echo "Making the binary executable..."
chmod +x "${install_dir}/yek"

echo "Installation complete!"

# Check if install_dir is in PATH
if ! echo "$PATH" | tr ':' '\n' | grep -Fx "$install_dir" >/dev/null; then
    echo "NOTE: $install_dir is not in your PATH. Add it by running:"
    echo "  export PATH=\"\$PATH:$install_dir\""
fi

echo "Now you can run: yek --help"
