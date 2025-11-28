#!/bin/bash
# publish_images.sh
# Script to build and publish CCGO Docker images to Docker Hub
#
# Usage:
#   ./publish_images.sh <docker_hub_username> [platform...]
#
# Examples:
#   ./publish_images.sh myusername                  # Build and push all platforms
#   ./publish_images.sh myusername linux windows    # Build and push specific platforms
#
# Prerequisites:
#   - Docker Desktop installed and running
#   - Docker Hub account
#   - Logged in to Docker Hub: docker login

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if Docker Hub username is provided
if [ -z "$1" ]; then
    print_error "Docker Hub username is required"
    echo "Usage: $0 <docker_hub_username> [platform...]"
    echo ""
    echo "Examples:"
    echo "  $0 myusername                  # Build and push all platforms"
    echo "  $0 myusername linux windows    # Build and push specific platforms"
    exit 1
fi

DOCKER_HUB_USERNAME="$1"
shift

# Available platforms
ALL_PLATFORMS=("linux" "windows" "apple" "android")
SELECTED_PLATFORMS=()

# If specific platforms provided, use them; otherwise use all
if [ $# -eq 0 ]; then
    SELECTED_PLATFORMS=("${ALL_PLATFORMS[@]}")
    print_info "No platforms specified, building all platforms"
else
    SELECTED_PLATFORMS=("$@")
    print_info "Building selected platforms: ${SELECTED_PLATFORMS[*]}"
fi

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    print_error "Docker is not running. Please start Docker Desktop and try again."
    exit 1
fi

print_success "Docker is running"

# Check if logged in to Docker Hub
if ! docker info 2>&1 | grep -q "Username"; then
    print_warning "Not logged in to Docker Hub"
    print_info "Please login with: docker login"
    exit 1
fi

print_success "Logged in to Docker Hub"

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Build and push each platform
TOTAL_PLATFORMS=${#SELECTED_PLATFORMS[@]}
CURRENT=0

print_info "Starting build and push process for ${TOTAL_PLATFORMS} platform(s)"
echo ""

for PLATFORM in "${SELECTED_PLATFORMS[@]}"; do
    CURRENT=$((CURRENT + 1))

    echo ""
    echo "========================================"
    echo " Platform: ${PLATFORM} (${CURRENT}/${TOTAL_PLATFORMS})"
    echo "========================================"
    echo ""

    # Map platform to Dockerfile and image name
    case "${PLATFORM}" in
        linux)
            DOCKERFILE="Dockerfile.linux"
            IMAGE_NAME="ccgo-builder-linux"
            ;;
        windows)
            DOCKERFILE="Dockerfile.windows-mingw"
            IMAGE_NAME="ccgo-builder-windows"
            ;;
        apple|macos|ios|watchos|tvos)
            DOCKERFILE="Dockerfile.apple"
            IMAGE_NAME="ccgo-builder-apple"
            PLATFORM="apple"  # Normalize to "apple"
            ;;
        android)
            DOCKERFILE="Dockerfile.android"
            IMAGE_NAME="ccgo-builder-android"
            ;;
        *)
            print_error "Unknown platform: ${PLATFORM}"
            print_info "Available platforms: ${ALL_PLATFORMS[*]}"
            continue
            ;;
    esac

    REMOTE_IMAGE="${DOCKER_HUB_USERNAME}/${IMAGE_NAME}:latest"

    # Check if Dockerfile exists
    if [ ! -f "${DOCKERFILE}" ]; then
        print_error "Dockerfile not found: ${DOCKERFILE}"
        continue
    fi

    print_info "Building image: ${IMAGE_NAME}"
    print_info "Remote image: ${REMOTE_IMAGE}"
    print_info "Dockerfile: ${DOCKERFILE}"
    echo ""

    # Build the Docker image
    print_info "Building Docker image (this may take 5-30 minutes)..."
    if DOCKER_BUILDKIT=1 docker build \
        -f "${DOCKERFILE}" \
        -t "${IMAGE_NAME}" \
        -t "${REMOTE_IMAGE}" \
        . ; then
        print_success "Successfully built ${IMAGE_NAME}"
    else
        print_error "Failed to build ${IMAGE_NAME}"
        continue
    fi

    # Get image size
    IMAGE_SIZE=$(docker images "${IMAGE_NAME}" --format "{{.Size}}")
    print_info "Image size: ${IMAGE_SIZE}"

    # Push to Docker Hub
    print_info "Pushing to Docker Hub: ${REMOTE_IMAGE}"
    if docker push "${REMOTE_IMAGE}"; then
        print_success "Successfully pushed ${REMOTE_IMAGE}"
    else
        print_error "Failed to push ${REMOTE_IMAGE}"
        continue
    fi

    echo ""
done

echo ""
echo "========================================"
print_success "All done! 🎉"
echo "========================================"
echo ""
print_info "Published images:"
for PLATFORM in "${SELECTED_PLATFORMS[@]}"; do
    case "${PLATFORM}" in
        linux) IMAGE_NAME="ccgo-builder-linux" ;;
        windows) IMAGE_NAME="ccgo-builder-windows" ;;
        apple|macos|ios|watchos|tvos) IMAGE_NAME="ccgo-builder-apple" ;;
        android) IMAGE_NAME="ccgo-builder-android" ;;
    esac
    echo "  - ${DOCKER_HUB_USERNAME}/${IMAGE_NAME}:latest"
done
echo ""
print_info "Users can now pull these images with:"
echo "  docker pull ${DOCKER_HUB_USERNAME}/ccgo-builder-<platform>:latest"
echo ""
print_info "To use in CCGO, update DOCKER_HUB_REPO in build_docker.py:"
echo "  DOCKER_HUB_REPO = \"${DOCKER_HUB_USERNAME}\""
