#!/bin/bash

# Script to push ccgo-builder Docker images to GitHub Container Registry
# Usage: ./push_to_ghcr.sh

set -e

echo "========================================"
echo "推送 CCGO Builder 镜像到 GHCR"
echo "========================================"
echo ""

# Check if GITHUB_TOKEN is set
if [ -z "$GITHUB_TOKEN" ]; then
  echo "❌ 错误: GITHUB_TOKEN 环境变量未设置"
  echo "请设置 GITHUB_TOKEN 后再运行此脚本"
  exit 1
fi

echo "✅ GITHUB_TOKEN 已设置"
echo ""

# Login to GHCR
echo "正在登录到 GitHub Container Registry..."
echo "$GITHUB_TOKEN" | docker login ghcr.io -u zhlinh --password-stdin

if [ $? -ne 0 ]; then
  echo "❌ 登录失败"
  exit 1
fi

echo "✅ 登录成功"
echo ""

# Push images
echo "========================================"
echo "开始推送镜像..."
echo "========================================"
echo ""

# 1. Push Linux image (smallest, fastest)
echo "1/4 正在推送 Linux 镜像 (467MB)..."
docker push ghcr.io/zhlinh/ccgo-builder-linux:latest
echo "✅ Linux 镜像推送完成"
echo ""

# 2. Push Windows image
echo "2/4 正在推送 Windows 镜像 (1.4GB)..."
docker push ghcr.io/zhlinh/ccgo-builder-windows:latest
echo "✅ Windows 镜像推送完成"
echo ""

# 3. Push Android image
echo "3/4 正在推送 Android 镜像 (3.41GB)..."
docker push ghcr.io/zhlinh/ccgo-builder-android:latest
echo "✅ Android 镜像推送完成"
echo ""

# 4. Push Apple image (largest, slowest)
echo "4/4 正在推送 Apple 镜像 (4.61GB)..."
docker push ghcr.io/zhlinh/ccgo-builder-apple:latest
echo "✅ Apple 镜像推送完成"
echo ""

echo "========================================"
echo "✅ 所有镜像推送完成！"
echo "========================================"
echo ""
echo "镜像地址:"
echo "  - ghcr.io/zhlinh/ccgo-builder-linux:latest"
echo "  - ghcr.io/zhlinh/ccgo-builder-windows:latest"
echo "  - ghcr.io/zhlinh/ccgo-builder-android:latest"
echo "  - ghcr.io/zhlinh/ccgo-builder-apple:latest"
echo ""
echo "你可以通过以下命令拉取镜像:"
echo "  docker pull ghcr.io/zhlinh/ccgo-builder-linux:latest"
