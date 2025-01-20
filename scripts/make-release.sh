#!/usr/bin/env bash
# Usage: scripts/make-release.sh [patch|minor|major]
# Default bump type is "patch" if not specified

set -euo pipefail

# 1. Figure out the bump type
BUMP_TYPE="${1:-patch}" # one of: patch, minor, major

# 2. Get the current version from Cargo.toml
CURRENT_VERSION="$(cargo pkgid | cut -d# -f2 | cut -d: -f2)"
echo "Current Cargo version: $CURRENT_VERSION"

# Quick format check (X.Y.Z)
if ! [[ "$CURRENT_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Invalid version format in Cargo.toml ($CURRENT_VERSION). Expected X.Y.Z"
    exit 1
fi

# Split out version parts
IFS='.' read -r MAJOR MINOR PATCH <<<"$CURRENT_VERSION"

# 3. Increment accordingly
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

# 4. Generate/Update CHANGELOG using cargo-cliff
#    Make sure cargo-cliff is installed (cargo install cargo-cliff)
git cliff --tag "v${NEW_VERSION}" --output CHANGELOG.md

# 5. Update Cargo.toml
sed -i.bak "s/^version *= *\"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" Cargo.toml
rm -f Cargo.toml.bak

# 6. Update Cargo.lock (so that if your package references itself, it's updated)
cargo update -p yek

# 7. Commit changes
git add Cargo.toml Cargo.lock CHANGELOG.md
if git diff --cached --quiet; then
    echo "No changes to commit. Exiting."
    exit 0
fi

git commit -m "release: v${NEW_VERSION}"

# 8. Tag the commit (annotated)
git tag -a "v${NEW_VERSION}" -m "release: v${NEW_VERSION}"

echo
echo "Local release commit and tag v${NEW_VERSION} created."
echo "Review your changes, then push if desired:"
echo "  git push origin HEAD"
echo "  git push origin v${NEW_VERSION}"
echo
echo "Done."
