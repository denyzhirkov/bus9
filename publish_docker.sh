#!/bin/bash

# Script to build and publish Bus9 Docker image to Docker Hub
# Usage: ./publish_docker.sh

set -e

# Configuration - CHANGE THIS TO YOUR DOCKER HUB USERNAME
DOCKER_USERNAME="denyzhirkov"

# Read version from version.json
if [ ! -f "version.json" ]; then
    echo "Error: version.json not found"
    exit 1
fi

VERSION=$(grep -o '"version": "[^"]*"' version.json | cut -d'"' -f4)

if [ -z "$VERSION" ]; then
    echo "Error: Could not read version from version.json"
    exit 1
fi

echo "=========================================="
echo "Building Bus9 Docker Image"
echo "Version: $VERSION"
echo "Docker Hub: ${DOCKER_USERNAME}/bus9"
echo "=========================================="
echo ""

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is not installed or not in PATH"
    exit 1
fi

# Check if logged in to Docker Hub
if ! docker info | grep -q "Username"; then
    echo "Warning: Not logged in to Docker Hub"
    echo "Please run: docker login"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo "Building image with tags:"
echo "  - ${DOCKER_USERNAME}/bus9:${VERSION}"
echo "  - ${DOCKER_USERNAME}/bus9:latest"
echo ""

# Build image
docker build -t ${DOCKER_USERNAME}/bus9:${VERSION} -t ${DOCKER_USERNAME}/bus9:latest .

if [ $? -ne 0 ]; then
    echo "Error: Docker build failed"
    exit 1
fi

echo ""
echo "Build successful!"
echo ""

# Ask for confirmation before pushing
read -p "Push images to Docker Hub? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Build completed. Images are ready locally."
    echo "To push manually, run:"
    echo "  docker push ${DOCKER_USERNAME}/bus9:${VERSION}"
    echo "  docker push ${DOCKER_USERNAME}/bus9:latest"
    exit 0
fi

echo ""
echo "Pushing images to Docker Hub..."
echo ""

# Push version tag
echo "Pushing ${DOCKER_USERNAME}/bus9:${VERSION}..."
docker push ${DOCKER_USERNAME}/bus9:${VERSION}

if [ $? -ne 0 ]; then
    echo "Error: Failed to push ${DOCKER_USERNAME}/bus9:${VERSION}"
    exit 1
fi

# Push latest tag
echo "Pushing ${DOCKER_USERNAME}/bus9:latest..."
docker push ${DOCKER_USERNAME}/bus9:latest

if [ $? -ne 0 ]; then
    echo "Error: Failed to push ${DOCKER_USERNAME}/bus9:latest"
    exit 1
fi

echo ""
echo "=========================================="
echo "Successfully published to Docker Hub!"
echo "=========================================="
echo ""
echo "Images:"
echo "  - ${DOCKER_USERNAME}/bus9:${VERSION}"
echo "  - ${DOCKER_USERNAME}/bus9:latest"
echo ""
echo "Test with:"
echo "  docker run -p 8080:8080 ${DOCKER_USERNAME}/bus9:${VERSION}"
echo ""
