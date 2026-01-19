#!/bin/bash
# release-tag.sh - Create a release tag, updating Cargo.toml version if needed
#
# Usage: ./tools/release-tag.sh <version>
#   version: Must begin with 'v' (e.g., v0.1.3)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check for required argument
if [ -z "$1" ]; then
    log_error "Version argument required"
    echo "Usage: $0 <version>"
    echo "  version: Must begin with 'v' (e.g., v0.1.3)"
    exit 1
fi

VERSION_TAG="$1"

# Validate version starts with 'v'
if [[ ! "$VERSION_TAG" =~ ^v ]]; then
    log_error "Version must begin with 'v' (e.g., v0.1.3)"
    exit 1
fi

# Strip the 'v' prefix to get semantic version
SEMVER="${VERSION_TAG#v}"

# Validate semver format (basic check)
if [[ ! "$SEMVER" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    log_error "Invalid semver format: $SEMVER"
    echo "Expected format: X.Y.Z or X.Y.Z-prerelease"
    exit 1
fi

log_info "Release version: $VERSION_TAG (semver: $SEMVER)"

# Get current version from Cargo.toml
CARGO_TOML="Cargo.toml"
if [ ! -f "$CARGO_TOML" ]; then
    log_error "Cargo.toml not found in current directory"
    exit 1
fi

CURRENT_VERSION=$(grep '^version = ' "$CARGO_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/')

if [ -z "$CURRENT_VERSION" ]; then
    log_error "Could not parse version from Cargo.toml"
    exit 1
fi

log_info "Current Cargo.toml version: $CURRENT_VERSION"

# Check if versions match
if [ "$CURRENT_VERSION" = "$SEMVER" ]; then
    log_info "Cargo.toml version already matches $SEMVER"
else
    log_warn "Version mismatch: Cargo.toml has $CURRENT_VERSION, updating to $SEMVER"

    # Update Cargo.toml version
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS sed requires empty string for -i
        sed -i '' "s/^version = \"$CURRENT_VERSION\"/version = \"$SEMVER\"/" "$CARGO_TOML"
    else
        # Linux sed
        sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$SEMVER\"/" "$CARGO_TOML"
    fi

    # Verify the update
    NEW_VERSION=$(grep '^version = ' "$CARGO_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/')
    if [ "$NEW_VERSION" != "$SEMVER" ]; then
        log_error "Failed to update Cargo.toml version"
        exit 1
    fi

    log_info "Updated Cargo.toml version to $SEMVER"

    # Stage and commit
    git add "$CARGO_TOML"
    git commit -m "$VERSION_TAG"
    log_info "Committed version bump: Release $VERSION_TAG"
fi

# Check if tag already exists
if git rev-parse "$VERSION_TAG" >/dev/null 2>&1; then
    log_error "Tag $VERSION_TAG already exists"
    echo "To delete and recreate: git tag -d $VERSION_TAG"
    exit 1
fi

# Create the tag
git tag -a "$VERSION_TAG" -m "Release $VERSION_TAG"
log_info "Created tag: $VERSION_TAG"

echo ""
echo "========================================="
echo "Release prepared successfully!"
echo "========================================="
echo ""
echo "Tag: $VERSION_TAG"
echo "Version: $SEMVER"
echo ""
echo "To push the release:"
echo "  git push origin main"
echo "  git push origin $VERSION_TAG"
echo ""
echo "Or push both at once:"
echo "  git push origin main --tags"
