# CCGO Docker Build Quick Reference

## Universal Cross-Platform Builds

Build **any platform on any OS** using Docker - no platform-specific toolchains needed!

## Quick Start

```bash
# Install Docker Desktop
# Download from: https://www.docker.com/products/docker-desktop

# Navigate to your CCGO project
cd /path/to/your/project

# Build any platform with --docker flag
ccgo build <platform> --docker
```

## Supported Platforms

| Platform | Command | Image Size | Build Time | Output |
|----------|---------|------------|------------|--------|
| **Linux** | `ccgo build linux --docker` | ~800MB | ~5 min | `.a` static libs |
| **Windows** | `ccgo build windows --docker` | ~1.2GB | ~8 min | `.lib` static libs (MinGW) |
| **macOS** | `ccgo build macos --docker` | ~2.5GB | ~15-20 min | `.a`, `.framework` |
| **iOS** | `ccgo build ios --docker` | ~2.5GB | ~15-20 min | `.a`, `.framework` |
| **watchOS** | `ccgo build watchos --docker` | ~2.5GB | ~15-20 min | `.a`, `.framework` |
| **tvOS** | `ccgo build tvos --docker` | ~2.5GB | ~15-20 min | `.a`, `.framework` |
| **Android** | `ccgo build android --docker` | ~3.5GB | ~20-25 min | `.so` native libs |

**Note**: Apple platforms share the same Docker image (`ccgo-builder-apple`)

## Disk Space Requirements

- **First-time setup**: ~8GB (all platform images)
- **Per-image breakdown**:
  - `ccgo-builder-linux`: ~800MB
  - `ccgo-builder-windows`: ~1.2GB
  - `ccgo-builder-apple`: ~2.5GB (shared for macOS/iOS/watchOS/tvOS)
  - `ccgo-builder-android`: ~3.5GB

**Savings vs Native Tools**:
- Xcode: 40GB → Docker: 2.5GB (94% reduction)
- Android Studio: 12GB → Docker: 3.5GB (71% reduction)
- Visual Studio: 8GB → Docker: 1.2GB (85% reduction)
- **Total**: 60GB+ → 8GB (87% reduction)

## ⚡ Prebuilt Images (Instant Setup!)

CCGO automatically downloads prebuilt images from Docker Hub - **no building required**!

### Speed Comparison

| Method | Time | What Happens |
|--------|------|--------------|
| **Prebuilt Image** | **2-10 min** | Downloads compressed image from Docker Hub |
| Local Build | 5-30 min | Builds image from scratch (apt-get, downloads, etc.) |

**You get 3-20x faster setup automatically!** 🚀

### How It Works

```bash
# When you run this command:
ccgo build linux --docker

# CCGO automatically:
# 1. Checks if image exists locally
# 2. If not, pulls prebuilt image from Docker Hub (~2-5 min)
# 3. Starts building your project
# Total time: ~2-5 minutes instead of ~10-15 minutes!
```

No configuration needed - prebuilt images are used automatically!

## Common Use Cases

### Scenario 1: Build iOS app on Windows/Linux

```bash
# No macOS or Xcode needed!
ccgo build ios --docker
```

### Scenario 2: Build Android library on macOS without Android Studio

```bash
# No Android SDK/NDK installation needed!
ccgo build android --docker
```

### Scenario 3: Build Windows library on macOS without VirtualBox

```bash
# No Windows VM or Boot Camp needed!
ccgo build windows --docker
```

### Scenario 4: CI/CD Pipeline (all platforms)

```bash
# Build for all platforms on a single Linux CI runner
ccgo build linux --docker
ccgo build windows --docker
ccgo build macos --docker
ccgo build ios --docker
ccgo build android --docker
```

## Management Commands

### View Docker Images

```bash
# List all CCGO images
docker images | grep ccgo-builder

# Output example:
# ccgo-builder-linux    latest    abc123    800MB
# ccgo-builder-windows  latest    def456    1.2GB
# ccgo-builder-apple    latest    ghi789    2.5GB
# ccgo-builder-android  latest    jkl012    3.5GB
```

### Check Disk Usage

```bash
# View total Docker disk usage
docker system df

# Detailed breakdown
docker system df -v
```

### Clean Up

```bash
# Remove a specific image
docker rmi ccgo-builder-linux

# Remove all CCGO images
docker rmi ccgo-builder-linux ccgo-builder-windows ccgo-builder-apple ccgo-builder-android

# Or use pattern matching
docker rmi $(docker images -q ccgo-builder*)

# Clean up all unused Docker data
docker system prune -a --volumes
```

### Rebuild Images

```bash
# Force rebuild (e.g., after Dockerfile updates)
docker rmi ccgo-builder-linux
ccgo build linux --docker  # Will rebuild image
```

## Performance Tips

1. **First build is slow** (downloads toolchains)
   - Linux: ~5 minutes
   - Windows: ~8 minutes
   - Apple platforms: ~15-20 minutes
   - Android: ~20-25 minutes

2. **Subsequent builds are fast** (uses cached images)
   - Actual project build time only
   - No image rebuild needed

3. **Parallel builds**
   ```bash
   # Build multiple platforms in parallel (separate terminals)
   ccgo build linux --docker &
   ccgo build windows --docker &
   ccgo build android --docker &
   wait
   ```

4. **Docker resource allocation**
   - Docker Desktop → Settings → Resources
   - Increase CPUs: 4+ cores recommended
   - Increase Memory: 8GB+ recommended

## ️ Important Notes

### Windows Builds (MinGW)

- Produces **MinGW-compatible** libraries (`.lib` files)
- Works with GCC/MinGW applications
- **NOT compatible** with MSVC (Visual Studio C++) projects
- For MSVC: Use native Windows build with Visual Studio

### Apple Platform Builds (OSXCross)

- Uses **OSXCross** for cross-compilation
- Can build binaries but **cannot run/test** them
- Best for: Library development, CI/CD pipelines
- Not officially supported by Apple
- Some platform-specific features may not work

### Android Builds

- Builds **native libraries only** (`.so` files)
- Full APK/AAR packaging done separately via Gradle
- Architectures: armeabi-v7a, arm64-v8a, x86_64

## Docker vs Native Builds

| Aspect | Docker Build | Native Build |
|--------|-------------|--------------|
| **Setup Time** | 10-30 min (first time) | Hours (toolchain installation) |
| **Disk Space** | 8GB (all platforms) | 60GB+ (all toolchains) |
| **Cross-platform** | ✅ Yes | ❌ No |
| **Build Speed** | Slower (containerization overhead) | Faster (native execution) |
| **Consistency** | 100% identical across OS | Varies by environment |
| **Maintenance** | Docker updates only | Multiple toolchain updates |
| **Best for** | CI/CD, Release builds | Active development, Debugging |

## 📚 Additional Resources

- **Detailed Docker documentation**: See `ccgo/dockers/README.md`
- **Troubleshooting guide**: See `ccgo/dockers/README.md#troubleshooting`
- **CI/CD integration examples**: See `ccgo/dockers/README.md#cicd-integration`
- **CCGO documentation**: See `CLAUDE.md`

## 💡 Pro Tips

1. **Build only what you need**: Don't pull all images unless necessary
2. **Use Docker BuildKit**: `export DOCKER_BUILDKIT=1` for faster builds
3. **Monitor resource usage**: `docker stats` to see container resource consumption
4. **Volume caching**: Build artifacts are written directly to host filesystem (fast)
5. **CI/CD optimization**: Cache Docker images between builds

## Quick Troubleshooting

| Problem | Solution |
|---------|----------|
| "Docker not found" | Install Docker Desktop |
| "Docker daemon not running" | Start Docker Desktop |
| "No space left on device" | `docker system prune -a` |
| "Build failed in container" | Check logs: `docker ps -a` then `docker logs <container_id>` |
| "Volume mount issues" | Check Docker file sharing settings |
| "Image build timeout" | Increase Docker resource allocation |

---

**Ready to build?** Run `ccgo build <platform> --docker` and enjoy universal cross-platform compilation!
