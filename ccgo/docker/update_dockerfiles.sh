#!/bin/bash
#
# Script to update all Dockerfiles to install ccgo dependencies
#

set -e

DOCKER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "========================================"
echo "更新 Dockerfiles - 添加 ccgo 依赖"
echo "========================================"
echo ""

# Function to add dependencies after tomli installation
add_ccgo_deps() {
    local file=$1
    echo "更新 $file ..."

    # Check if already updated
    if grep -q "copier>=9.2.0" "$file"; then
        echo "  ✓ 已经包含 ccgo 依赖，跳过"
        return
    fi

    # Replace single tomli install with full dependency list
    sed -i.bak 's/RUN pip3 install --no-cache-dir tomli/# Install Python dependencies for CCGO\n# tomli: TOML parsing for Python < 3.11\n# copier: Template engine (required by ccgo)\nRUN pip3 install --no-cache-dir \\\n    tomli \\\n    copier>=9.2.0 \\\n    copier-templates-extensions>=0.3.0/' "$file"

    rm -f "${file}.bak"
    echo "  ✓ 已添加 ccgo 依赖"
}

# Update each Dockerfile
for dockerfile in Dockerfile.linux Dockerfile.windows Dockerfile.apple Dockerfile.android; do
    if [ -f "$DOCKER_DIR/$dockerfile" ]; then
        add_ccgo_deps "$DOCKER_DIR/$dockerfile"
    else
        echo "⚠ $dockerfile 不存在"
    fi
    echo ""
done

echo "========================================"
echo "✓ 所有 Dockerfiles 更新完成"
echo "========================================"
