# Build Caching with ccache/sccache

## Overview

CCGO supports **compiler caching** through ccache and sccache to dramatically speed up C++ compilation by **30-50%** (or more) by caching compilation artifacts.

## Benefits

- ✅ **Faster Rebuilds** - Subsequent builds reuse cached compilation results
- ✅ **Automatic Detection** - Auto-detects and uses available cache tools
- ✅ **Zero Configuration** - Works out of the box when cache tool is installed
- ✅ **Shared Cache** - Cache is shared across all CCGO projects
- ✅ **CI/CD Friendly** - Speeds up continuous integration builds significantly

## Quick Start

### Install a Cache Tool

**Option 1: sccache (Recommended)**
```bash
# macOS
brew install sccache

# Linux
cargo install sccache

# Arch Linux
sudo pacman -S sccache

# Debian/Ubuntu
sudo apt install sccache
```

**Option 2: ccache**
```bash
# macOS
brew install ccache

# Linux
sudo apt install ccache  # Debian/Ubuntu
sudo yum install ccache  # CentOS/RHEL
sudo pacman -S ccache    # Arch Linux
```

### Build with Caching

Cache is **enabled by default** with `--cache auto`:

```bash
# Auto-detect and use available cache (default)
ccgo build linux

# Explicitly specify cache tool
ccgo build linux --cache sccache
ccgo build linux --cache ccache

# Disable caching
ccgo build linux --cache none
```

## Cache Tools Comparison

| Feature | ccache | sccache |
|---------|--------|---------|
| Language | C | Rust |
| Speed | Fast | Faster |
| Cloud Storage | No | Yes (S3, Redis, Memcached) |
| Distribution | Stable, Mature | Modern, Actively Developed |
| Platform Support | All platforms | All platforms |
| Memory Usage | Low | Medium |
| Recommendation | Good choice | Better choice |

**CCGO Preference Order**: sccache > ccache > none

## Usage

### Command-Line Options

```bash
# Auto-detect (default) - tries sccache first, then ccache
ccgo build <platform> --cache auto

# Force specific cache tool
ccgo build <platform> --cache sccache
ccgo build <platform> --cache ccache

# Disable caching
ccgo build <platform> --cache none
ccgo build <platform> --cache off
ccgo build <platform> --cache disabled
```

### Build Output

When caching is enabled, you'll see:

```bash
$ ccgo build linux

Building CCGO Library for linux...
   🚀 Using sccache for compilation caching

Configuring CMake...
-- Build files generated successfully
```

### Cache Statistics

**ccache**:
```bash
# Show cache statistics
ccache -s

# Zero statistics (reset counters)
ccache -z

# Clear cache
ccache -C
```

**sccache**:
```bash
# Show cache statistics
sccache --show-stats

# Zero statistics (reset counters)
sccache --zero-stats

# Stop server (clears in-memory cache)
sccache --stop-server
```

## Performance Impact

### First Build (Cold Cache)

```
Time: 100% (baseline)
- No cached artifacts
- Full compilation required
```

### Second Build (Warm Cache)

```
Time: 20-50% of first build (50-80% faster)
- Cached artifacts reused
- Only changed files recompiled
```

### Example Metrics

| Project Size | First Build | Cached Build | Speedup |
|-------------|-------------|--------------|---------|
| Small (5-10 files) | 10s | 3s | 3.3x |
| Medium (50-100 files) | 60s | 15s | 4x |
| Large (500+ files) | 300s | 60s | 5x |

**Note**: Speedup increases with project size and build frequency.

## How It Works

### CMake Integration

CCGO automatically configures CMake with compiler launcher variables:

```cmake
# Injected by CCGO when cache is enabled
CMAKE_C_COMPILER_LAUNCHER=/path/to/sccache
CMAKE_CXX_COMPILER_LAUNCHER=/path/to/sccache
```

This wraps your C/C++ compiler (gcc, clang, msvc) with the cache tool.

### Cache Key

Compilation artifacts are cached based on:
- Source file content
- Compiler flags
- Header dependencies
- Preprocessor definitions
- Compiler version

If any of these change, the cache is invalidated and recompilation occurs.

### Cache Location

**ccache**:
- Default: `~/.ccache/` (Linux/macOS), `%LOCALAPPDATA%\ccache` (Windows)
- Configure: `export CCACHE_DIR=/custom/path`

**sccache**:
- Default: `~/.cache/sccache/` (Linux), `~/Library/Caches/Mozilla.sccache/` (macOS)
- Configure: `export SCCACHE_DIR=/custom/path`

## Configuration

### Environment Variables

**ccache**:
```bash
# Set cache directory
export CCACHE_DIR=/path/to/cache

# Set max cache size
export CCACHE_MAXSIZE=5G

# Enable compression
export CCACHE_COMPRESS=true

# Set compression level (1-9)
export CCACHE_COMPRESSLEVEL=6
```

**sccache**:
```bash
# Set cache directory
export SCCACHE_DIR=/path/to/cache

# Set max cache size
export SCCACHE_CACHE_SIZE="5G"

# Use Redis for distributed caching
export SCCACHE_REDIS=redis://localhost:6379

# Use AWS S3 for distributed caching
export SCCACHE_BUCKET=my-sccache-bucket
export SCCACHE_REGION=us-west-2
```

### CI/CD Configuration

**GitHub Actions**:
```yaml
name: Build with Cache

on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      # Install sccache
      - name: Install sccache
        run: |
          wget https://github.com/mozilla/sccache/releases/download/v0.5.4/sccache-v0.5.4-x86_64-unknown-linux-musl.tar.gz
          tar xzf sccache-v0.5.4-x86_64-unknown-linux-musl.tar.gz
          sudo mv sccache-v0.5.4-x86_64-unknown-linux-musl/sccache /usr/local/bin/

      # Cache sccache directory
      - name: Cache sccache
        uses: actions/cache@v3
        with:
          path: ~/.cache/sccache
          key: ${{ runner.os }}-sccache-${{ hashFiles('**/CCGO.toml') }}
          restore-keys: |
            ${{ runner.os }}-sccache-

      # Build with caching
      - name: Build
        run: ccgo build linux

      # Show cache statistics
      - name: Show cache stats
        run: sccache --show-stats
```

**GitLab CI**:
```yaml
build:
  image: ubuntu:latest
  cache:
    key: $CI_COMMIT_REF_SLUG
    paths:
      - .cache/sccache
  before_script:
    - apt-get update && apt-get install -y sccache
    - export SCCACHE_DIR=$PWD/.cache/sccache
  script:
    - ccgo build linux
    - sccache --show-stats
```

## Troubleshooting

### Cache Not Working

**Check if cache tool is installed**:
```bash
which sccache
which ccache
```

**Check if cache is detected**:
```bash
ccgo build linux --verbose
# Should show "Using sccache for compilation caching"
```

**Verify CMake configuration**:
```bash
# Check CMake build log for COMPILER_LAUNCHER
grep COMPILER_LAUNCHER cmake_build/release/linux/CMakeCache.txt
```

### Cache Miss Rate High

**Possible causes**:
- Different compiler flags between builds
- Timestamp-based dependencies (use hash-based)
- Header files changing frequently
- Cache size too small (increase with `CCACHE_MAXSIZE` or `SCCACHE_CACHE_SIZE`)

**Solutions**:
```bash
# Increase cache size
export CCACHE_MAXSIZE=10G
export SCCACHE_CACHE_SIZE="10G"

# Enable compression to fit more in cache
export CCACHE_COMPRESS=true
```

### Permission Errors

**Fix cache directory permissions**:
```bash
# ccache
chmod -R u+w ~/.ccache

# sccache
chmod -R u+w ~/.cache/sccache
```

### Cache Corruption

**Clear and rebuild cache**:
```bash
# ccache
ccache -C  # Clear cache
ccgo build linux

# sccache
sccache --stop-server  # Stop server (clears cache)
rm -rf ~/.cache/sccache  # Remove cache directory
ccgo build linux
```

## Best Practices

### DO

✅ **Use sccache** - Faster and more feature-rich than ccache
✅ **Enable in CI/CD** - Speeds up pipeline builds significantly
✅ **Monitor cache size** - Set appropriate limits to avoid disk space issues
✅ **Share cache** - Use distributed cache (Redis/S3) for team builds
✅ **Keep cache warm** - Regular builds maintain cache effectiveness

### DON'T

❌ **Don't mix debug/release** - They have different cache keys
❌ **Don't commit cache** - Cache directory should be in `.gitignore`
❌ **Don't use tiny cache** - Set at least 5GB for medium projects
❌ **Don't disable in dev** - Caching speeds up development builds

## Advanced Usage

### Distributed Caching

**sccache with Redis**:
```bash
# Start Redis server
docker run -d -p 6379:6379 redis

# Configure sccache
export SCCACHE_REDIS=redis://localhost:6379

# Build (cache shared across team)
ccgo build linux
```

**sccache with AWS S3**:
```bash
# Configure AWS credentials
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...

# Configure S3 bucket
export SCCACHE_BUCKET=my-team-cache
export SCCACHE_REGION=us-west-2

# Build (cache shared in S3)
ccgo build linux
```

### Custom Cache Configuration

Create `.ccgo/cache_config.toml`:
```toml
[cache]
# Auto, ccache, sccache, or none
tool = "auto"

# Maximum cache size
max_size = "10G"

# Enable compression
compress = true

# Distributed cache URL (sccache only)
redis_url = "redis://localhost:6379"
```

**Note**: This feature is planned for a future release.

## See Also

- [Build Performance Optimization](build-optimization.md)
- [Incremental Builds](incremental-builds.md)
- [CMake Configuration](../reference/cmake-configuration.md)

## Changelog

### v3.0.11 (2026-01-21)

- ✅ Implemented ccache/sccache integration
- ✅ Auto-detection of available cache tools
- ✅ Command-line `--cache` option
- ✅ Automatic CMake compiler launcher configuration
- ✅ Support for all platforms (Linux, macOS, Windows, iOS, Android, OHOS)

---

*Compiler caching can reduce build times by 30-50% or more, making iterative development much faster.*
