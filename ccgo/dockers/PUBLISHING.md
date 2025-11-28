# Publishing Docker Images to Docker Hub

This document explains how to build and publish CCGO Docker images to Docker Hub, making them available for users to download instantly instead of building locally.

## 🎯 Benefits of Prebuilt Images

| Aspect | Prebuilt Image (Docker Hub) | Local Build |
|--------|----------------------------|-------------|
| **Download Time** | 2-10 minutes | N/A |
| **Build Time** | 0 (already built) | 5-30 minutes |
| **Total Time** | **2-10 minutes** | **5-30 minutes** |
| **Network Usage** | Download compressed image | Download packages during build |
| **User Experience** | ✅ Fast, consistent | ⚠️ Slow first time |

**Speed improvement**: 3-20x faster for users!

## 📋 Prerequisites

### 1. Docker Hub Account

- Create a free account at [hub.docker.com](https://hub.docker.com)
- Choose a username (e.g., `myusername`)
- Your images will be: `myusername/ccgo-builder-linux:latest`

### 2. Docker Hub Token

Create a personal access token for secure publishing:

1. Login to Docker Hub
2. Go to Account Settings → Security → Access Tokens
3. Click "New Access Token"
4. Name: `CCGO Builder`
5. Permissions: Read & Write
6. Copy the token (you'll need it later)

### 3. Local Setup

```bash
# Login to Docker Hub
docker login

# Verify login
docker info | grep Username
```

## 🚀 Publishing Methods

### Method 1: Manual Publishing (Quick Start)

Use the provided script to build and push images manually:

```bash
# Navigate to dockers directory
cd ccgo/ccgo/dockers

# Publish all platforms
./publish_images.sh <your-docker-hub-username>

# Publish specific platforms
./publish_images.sh <your-docker-hub-username> linux windows
./publish_images.sh <your-docker-hub-username> apple android
```

**Example:**

```bash
# If your Docker Hub username is "jsmith"
./publish_images.sh jsmith linux windows

# Output:
# ✓ Building linux image...
# ✓ Pushing to jsmith/ccgo-builder-linux:latest
# ✓ Building windows image...
# ✓ Pushing to jsmith/ccgo-builder-windows:latest
```

**Time estimate:**
- Linux: ~5-8 minutes
- Windows: ~8-12 minutes
- Apple: ~15-25 minutes
- Android: ~20-30 minutes
- **Total (all platforms): ~50-75 minutes**

### Method 2: Automated Publishing (Recommended)

Use GitHub Actions to automatically build and publish images.

#### Setup GitHub Secrets

1. Go to your GitHub repository
2. Settings → Secrets and variables → Actions
3. Add two secrets:
   - `DOCKER_HUB_USERNAME`: Your Docker Hub username
   - `DOCKER_HUB_TOKEN`: Your Docker Hub access token

#### Configure Workflow

Edit `.github/workflows/publish-docker-images.yml`:

```yaml
env:
  DOCKER_HUB_USERNAME: your-username  # Change this
```

#### Trigger Builds

**Option A: Manual trigger**
1. Go to Actions tab on GitHub
2. Select "Build and Publish Docker Images"
3. Click "Run workflow"
4. Choose platforms (or select "all")
5. Click "Run workflow"

**Option B: Automatic triggers**
- **Weekly**: Runs every Sunday at 2am UTC (keeps images updated)
- **On Dockerfile changes**: Automatically rebuilds when Dockerfiles are modified
- **On release**: Tag a new release to trigger builds

## 📝 Update CCGO to Use Your Images

After publishing, update `ccgo/dockers/build_docker.py`:

```python
# Change this line:
DOCKER_HUB_REPO = "ccgo"

# To your Docker Hub username:
DOCKER_HUB_REPO = "your-username"
```

Now users will automatically pull images from your Docker Hub repository!

## 🔍 Verify Published Images

### Check on Docker Hub

Visit: `https://hub.docker.com/u/your-username`

You should see:
- `your-username/ccgo-builder-linux`
- `your-username/ccgo-builder-windows`
- `your-username/ccgo-builder-apple`
- `your-username/ccgo-builder-android`

### Test Pull Locally

```bash
# Pull an image
docker pull your-username/ccgo-builder-linux:latest

# Verify it works
docker run --rm your-username/ccgo-builder-linux:latest "gcc --version"
```

### Test with CCGO

```bash
# Remove local image
docker rmi ccgo-builder-linux

# Build a project (will pull from Docker Hub)
ccgo build linux --docker

# Should show:
# ✓ Pulling prebuilt image from Docker Hub...
# ✓ Successfully pulled your-username/ccgo-builder-linux:latest
```

## 📊 Image Sizes

Expected sizes for compressed (download) vs uncompressed (disk):

| Platform | Compressed (Download) | Uncompressed (Disk) |
|----------|----------------------|---------------------|
| Linux | ~300MB | ~800MB |
| Windows | ~450MB | ~1.2GB |
| Apple | ~900MB | ~2.5GB |
| Android | ~1.3GB | ~3.5GB |
| **Total** | **~3GB** | **~8GB** |

## 🔄 Update Schedule

Recommended update frequency:

- **Monthly**: Update all images (toolchain updates, security patches)
- **On Dockerfile changes**: Automatic via GitHub Actions
- **On user request**: If users report issues with current images

## 🛠️ Troubleshooting

### Build fails on GitHub Actions

**Problem**: Build timeout or out of memory

**Solution**:
```yaml
# In .github/workflows/publish-docker-images.yml
# Add timeout and resource settings
jobs:
  build-and-push:
    timeout-minutes: 120  # Increase timeout
```

### Image too large

**Problem**: Docker Hub free tier has 1 repository limit

**Solution**:
1. Upgrade to Docker Hub Pro ($5/month for unlimited repositories)
2. Or use GitHub Container Registry (ghcr.io) - free for public images

**Alternative: GitHub Container Registry**

Change in `build_docker.py`:
```python
# Use GitHub Container Registry instead
DOCKER_HUB_REPO = "ghcr.io/your-username"
```

Update workflow to push to ghcr.io instead of Docker Hub.

### Users getting old images

**Problem**: Users have cached old images

**Solution**: Tell users to update:
```bash
# Remove old images
docker rmi ccgo-builder-*

# Pull fresh images
ccgo build <platform> --docker
```

## 📚 Additional Resources

### Docker Hub Documentation
- [Docker Hub Quickstart](https://docs.docker.com/docker-hub/)
- [Automated Builds](https://docs.docker.com/docker-hub/builds/)
- [Access Tokens](https://docs.docker.com/docker-hub/access-tokens/)

### GitHub Actions Documentation
- [Docker Build-Push Action](https://github.com/docker/build-push-action)
- [Docker Login Action](https://github.com/docker/login-action)
- [GitHub Container Registry](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry)

### Best Practices
- Use multi-stage builds to reduce image size
- Tag images with versions (not just `latest`)
- Document any breaking changes
- Keep images updated with security patches

## 💡 Pro Tips

1. **Use build cache**: GitHub Actions caches layers to speed up rebuilds
2. **Parallel builds**: Build multiple platforms in parallel
3. **Scheduled updates**: Keep images fresh with weekly builds
4. **Version tags**: Tag images with version numbers for stability
5. **Security scanning**: Enable Docker Hub security scanning

## 🎊 Summary

With prebuilt images:
- ✅ Users get started **3-20x faster**
- ✅ Consistent images across all users
- ✅ Reduced bandwidth usage
- ✅ Better CI/CD performance
- ✅ Professional developer experience

Publish your images today and make CCGO even more awesome! 🚀
