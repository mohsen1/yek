#!/usr/bin/env bash
set -euo pipefail

REPO_OWNER="bodo-run"
REPO_NAME="yek"

# Determine a sensible default install directory
# We'll check for a directory in PATH that is writable.
# If none is found, we fall back to "$HOME/.local/bin".
fallback_dir="$HOME/.local/bin"

# Split PATH on ":" into an array
IFS=':' read -ra path_entries <<<"$PATH"
install_candidates=("/usr/local/bin" "${path_entries[@]}")
install_dir=""

for dir in "${install_candidates[@]}"; do
    # Skip empty paths
    [ -z "$dir" ] && continue

    # Check if directory is writable
    if [ -d "$dir" ] && [ -w "$dir" ]; then
        install_dir="$dir"
        break
    fi
done

# If we didn't find a writable dir in PATH, fallback to $HOME/.local/bin
if [ -z "$install_dir" ]; then
    install_dir="$fallback_dir"
fi

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
