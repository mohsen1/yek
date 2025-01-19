#!/usr/bin/env bash
# Usage: scripts/make-release.sh [patch|minor|major]
# Default bump type is "patch" if not specified

set -euo pipefail

# -- Step 0: Determine bump type --
BUMP_TYPE="${1:-patch}" # default to 'patch' if not set: patch|minor|major

# -- Step 1: Read current version from Cargo.toml --
CURRENT_VERSION="$(cargo pkgid | cut -d# -f2 | cut -d: -f2)"
echo "Current Cargo version: $CURRENT_VERSION"

# Validate version format
if ! echo "$CURRENT_VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "Error: Invalid version format in Cargo.toml. Expected format: X.Y.Z"
    exit 1
fi

# Split into semver parts and validate
IFS='.' read -r MAJOR MINOR PATCH <<<"$CURRENT_VERSION"
if ! [[ "$MAJOR" =~ ^[0-9]+$ ]] || ! [[ "$MINOR" =~ ^[0-9]+$ ]] || ! [[ "$PATCH" =~ ^[0-9]+$ ]]; then
    echo "Error: Version components must be valid numbers"
    exit 1
fi

# -- Step 2: Bump version accordingly --
case "$BUMP_TYPE" in
major)
    MAJOR=$((MAJOR + 1))
    MINOR=0
    PATCH=0
    ;;
minor)
    MINOR=$((MINOR + 1))
    PATCH=0
    ;;
patch)
    PATCH=$((PATCH + 1))
    ;;
*)
    echo "Unknown bump type: $BUMP_TYPE"
    exit 1
    ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
echo "Bumping version to: $NEW_VERSION"

# -- Step 3: Update CHANGELOG.md --
RELEASE_DATE=$(date +%Y-%m-%d)
TEMP_CHANGELOG=$(mktemp)

# Get all commits since last tag
COMMITS=$(git log --pretty=format:"- %s" $(git describe --tags --abbrev=0 2>/dev/null || git rev-list --max-parents=0 HEAD)..HEAD | grep -v "^- release:")

# Create new changelog entry
{
    echo "# Changelog"
    echo
    echo "## [${NEW_VERSION}] - ${RELEASE_DATE}"
    echo
    if [[ "$BUMP_TYPE" == "major" ]]; then
        echo "### Breaking Changes"
        echo
        echo "$COMMITS" | grep -i "^- breaking:" || true
        echo
    fi
    echo "### Features"
    echo
    echo "$COMMITS" | grep -i "^- feat:" || true
    echo
    echo "### Bug Fixes"
    echo
    echo "$COMMITS" | grep -i "^- fix:" || true
    echo
    echo "### Other Changes"
    echo
    echo "$COMMITS" | grep -v "^- breaking:\|^- feat:\|^- fix:" || true
    echo
} >"$TEMP_CHANGELOG"

# Append existing changelog if it exists
if [ -f CHANGELOG.md ]; then
    # Skip the first line if it's "# Changelog"
    if grep -q "^# Changelog" CHANGELOG.md; then
        tail -n +2 CHANGELOG.md >>"$TEMP_CHANGELOG"
    else
        cat CHANGELOG.md >>"$TEMP_CHANGELOG"
    fi
fi

mv "$TEMP_CHANGELOG" CHANGELOG.md

# -- Step 4: Update Cargo.toml version --
sed -i.bak "s/^version *= *\"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" Cargo.toml
rm -f Cargo.toml.bak

# -- Step 5: Update version in Formula/yek.rb --
sed -i.bak "s/^  version \".*\"/  version \"${NEW_VERSION}\"/" Formula/yek.rb
rm -f Formula/yek.rb.bak

# -- Step 6: Build artifacts and compute SHA --
make build-artifacts

# -- Step 7: Update SHA256 hash in Formula for current platform --
CURRENT_PLATFORM=$(rustc -vV | grep host: | cut -d' ' -f2)
TARBALL_NAME="yek-${CURRENT_PLATFORM}.tar.gz"
SHA256_VALUE="$(shasum -a 256 "${TARBALL_NAME}" | awk '{print $1}')"
echo "Computed SHA256 for $TARBALL_NAME: $SHA256_VALUE"
sed -i.bak "/url \".*yek-${CURRENT_PLATFORM}.tar.gz\"/{n;s/sha256 \".*\"/sha256 \"${SHA256_VALUE}\"/;}" Formula/yek.rb
rm -f Formula/yek.rb.bak

# -- Step 8: Stage all changes --
git add Cargo.toml Cargo.lock Formula/yek.rb CHANGELOG.md .gitignore

# -- Step 9: Amend the last commit with version bump changes --
if git log -1 --pretty=%B | grep -q "^release: v"; then
    # If the last commit is already a release commit, amend it
    git commit --amend --no-edit
else
    # Create a new release commit
    git commit -m "release: v${NEW_VERSION}"
fi

# -- Step 10: Tag and push --
git tag -f "v${NEW_VERSION}" # -f in case we're amending and the tag exists
git push -f origin HEAD      # Push current branch, not necessarily main
git push -f origin "v${NEW_VERSION}"

echo "Done. Pushed tag v${NEW_VERSION}."
echo "CI should trigger on that tag to build & publish a release."
