#!/bin/bash

# Script to bump version in version.json and Cargo.toml
# Usage: ./bump_version.sh [major|minor|patch]

set -e

if [ $# -eq 0 ]; then
    echo "Usage: $0 [major|minor|patch]"
    echo "  major: 1.0.0 -> 2.0.0"
    echo "  minor: 1.0.0 -> 1.1.0"
    echo "  patch: 1.0.0 -> 1.0.1"
    exit 1
fi

BUMP_TYPE=$1

if [[ ! "$BUMP_TYPE" =~ ^(major|minor|patch)$ ]]; then
    echo "Error: Invalid bump type. Must be one of: major, minor, patch"
    exit 1
fi

# Read current version from version.json
if [ ! -f "version.json" ]; then
    echo "Error: version.json not found"
    exit 1
fi

CURRENT_VERSION=$(grep -o '"version": "[^"]*"' version.json | cut -d'"' -f4)

if [ -z "$CURRENT_VERSION" ]; then
    echo "Error: Could not read version from version.json"
    exit 1
fi

# Parse version components
IFS='.' read -r -a VERSION_PARTS <<< "$CURRENT_VERSION"
MAJOR=${VERSION_PARTS[0]}
MINOR=${VERSION_PARTS[1]}
PATCH=${VERSION_PARTS[2]}

# Validate version format
if [ -z "$MAJOR" ] || [ -z "$MINOR" ] || [ -z "$PATCH" ]; then
    echo "Error: Invalid version format. Expected: X.Y.Z, got: $CURRENT_VERSION"
    exit 1
fi

# Bump version based on type
case $BUMP_TYPE in
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
esac

NEW_VERSION="$MAJOR.$MINOR.$PATCH"

echo "Bumping version: $CURRENT_VERSION -> $NEW_VERSION ($BUMP_TYPE)"

# Update version.json
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    sed -i '' "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" version.json
else
    # Linux
    sed -i "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" version.json
fi

# Update Cargo.toml
if [ -f "Cargo.toml" ]; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        sed -i '' "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
    else
        # Linux
        sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
    fi
    echo "Updated Cargo.toml"
fi

echo "Version bumped successfully to $NEW_VERSION"

# Create git tag
if command -v git &> /dev/null; then
    if git rev-parse --git-dir > /dev/null 2>&1; then
        TAG_NAME="v$NEW_VERSION"
        
        # Check if tag already exists
        if git rev-parse "$TAG_NAME" > /dev/null 2>&1; then
            echo "Warning: Tag $TAG_NAME already exists. Skipping tag creation."
        else
            echo "Creating git tag: $TAG_NAME"
            git tag -a "$TAG_NAME" -m "Release version $NEW_VERSION"
            echo "Tag $TAG_NAME created successfully"
            echo ""
            echo "To push the tag to remote, run:"
            echo "  git push origin $TAG_NAME"
        fi
    else
        echo "Warning: Not a git repository. Skipping tag creation."
    fi
else
    echo "Warning: git not found. Skipping tag creation."
fi
