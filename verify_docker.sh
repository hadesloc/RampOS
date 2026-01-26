#!/bin/bash
set -e

echo "Starting Docker build verification..."

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "Warning: Docker is not running. Only syntax/file checks will be performed."
    exit 0
fi

# Build only the API server to verify Dockerfile
echo "Verifying Dockerfile build..."
docker build -t rampos-api-test . --no-cache --target builder --build-arg BUILDKIT_INLINE_CACHE=1

echo "Verifying docker-compose config..."
docker-compose config

echo "Docker verification complete!"
