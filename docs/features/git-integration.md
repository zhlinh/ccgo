# Git Integration

Complete guide to Git workflow integration in CCGO projects, including automated operations, hooks, and best practices.

## Overview

CCGO provides seamless Git integration for version control workflows:

- **Automated Tagging** - Create version tags from CCGO.toml
- **Git Hooks** - Pre-commit validation and automation
- **Submodule Management** - Handle C++ dependencies as submodules
- **CI/CD Integration** - Automated builds on git events
- **Branch Workflows** - Support for GitFlow and trunk-based development
- **Commit Metadata** - Inject git info into builds

## Git Workflow Patterns

### Trunk-Based Development

```bash
# Main branch is always deployable
main ─────●─────●─────●─────●─────●──→
           \     \     \     \
            \─●   \─●   \─●   \─●  short-lived
              |     |     |     |   feature branches
              merge merge merge merge
```

**Workflow:**
```bash
# 1. Create short-lived feature branch
git checkout -b feature/quick-fix

# 2. Make changes and commit
ccgo build --all
git commit -am "feat: add new API endpoint"

# 3. Merge to main (after CI passes)
git checkout main
git merge feature/quick-fix

# 4. Tag release
ccgo tag v1.2.3 --message "Release 1.2.3"
```

### GitFlow

```bash
main ────●─────────────●─────────────●──→  production releases
          \           /               /
           \         /               /
develop ────●───●───●───●───●───●───●──→  integration branch
             \  |  /     \     /
              \ | /       \   /
feature-a      ●●●         \ /
feature-b                   ●●●
```

**Workflow:**
```bash
# 1. Create feature branch from develop
git checkout develop
git checkout -b feature/new-module

# 2. Develop feature
ccgo build --all
ccgo test

# 3. Merge back to develop
git checkout develop
git merge feature/new-module

# 4. Create release branch
git checkout -b release/1.2.0

# 5. Finalize release
# Update CCGO.toml: version = "1.2.0"
ccgo build --all --release
ccgo test --all

# 6. Merge to main and develop
git checkout main
git merge release/1.2.0
ccgo tag v1.2.0 --message "Release 1.2.0"

git checkout develop
git merge release/1.2.0
```

## Automated Tagging

### Using ccgo tag

```bash
# Create tag from CCGO.toml version
ccgo tag

# Create tag with custom version
ccgo tag v2.0.0

# Create annotated tag with message
ccgo tag --annotate --message "Release 2.0.0 with new features"

# Push tag to remote
ccgo tag --push

# Force update existing tag
ccgo tag --force
```

### Tag Naming Convention

CCGO follows semantic versioning for tags:

```bash
# Release tags
v1.0.0          # Major release
v1.2.3          # Minor/patch release
v2.0.0-rc.1     # Release candidate
v1.5.0-beta.2   # Beta release

# Internal tags
build-20240115  # CI build tag
dev-john-123    # Development tag
```

### Automated Tag Creation in CI

```yaml
# GitHub Actions
name: Tag Release
on:
  push:
    branches: [main]

jobs:
  tag:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install CCGO
        run: pip install ccgo

      - name: Create tag if version changed
        run: |
          # Extract version from CCGO.toml
          VERSION=$(grep '^version = ' CCGO.toml | cut -d'"' -f2)

          # Check if tag exists
          if ! git rev-parse "v$VERSION" >/dev/null 2>&1; then
            ccgo tag "v$VERSION" --message "Release $VERSION" --push
          fi
```

## Git Hooks

### Pre-Commit Hook

Validates code before commit:

```bash
# .git/hooks/pre-commit
#!/bin/bash

# Format check
echo "Running code formatting check..."
if ! ccgo check --format; then
    echo "Error: Code formatting issues found"
    echo "Run: ccgo format --fix"
    exit 1
fi

# Build check
echo "Running build check..."
if ! ccgo build --quick-check; then
    echo "Error: Build check failed"
    exit 1
fi

# License headers
echo "Checking license headers..."
if ! ccgo check --license; then
    echo "Error: Missing license headers"
    exit 1
fi

echo "Pre-commit checks passed!"
```

### Pre-Push Hook

Runs tests before pushing:

```bash
# .git/hooks/pre-push
#!/bin/bash

# Run unit tests
echo "Running unit tests..."
if ! ccgo test; then
    echo "Error: Tests failed"
    exit 1
fi

# Check for TODO/FIXME in changed files
echo "Checking for unresolved TODOs..."
changed_files=$(git diff --name-only @{u}..HEAD)
if echo "$changed_files" | xargs grep -n "FIXME\|TODO" 2>/dev/null; then
    read -p "Found TODOs/FIXMEs. Continue push? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo "Pre-push checks passed!"
```

### Commit-Msg Hook

Enforces conventional commits:

```bash
# .git/hooks/commit-msg
#!/bin/bash

commit_msg=$(cat "$1")

# Check conventional commit format
if ! echo "$commit_msg" | grep -qE "^(feat|fix|docs|style|refactor|test|chore|perf|ci|build|revert)(\(.+\))?: .+"; then
    echo "Error: Commit message must follow Conventional Commits format"
    echo ""
    echo "Format: <type>(<scope>): <subject>"
    echo ""
    echo "Types: feat, fix, docs, style, refactor, test, chore, perf, ci, build, revert"
    echo ""
    echo "Example: feat(android): add new build configuration"
    exit 1
fi

echo "Commit message format validated!"
```

### Installing Hooks

```bash
# Make hooks executable
chmod +x .git/hooks/pre-commit
chmod +x .git/hooks/pre-push
chmod +x .git/hooks/commit-msg

# Or use CCGO to install hooks
ccgo install-hooks
```

## Submodule Management

### Adding C++ Dependencies as Submodules

```bash
# Add third-party library as submodule
git submodule add https://github.com/openssl/openssl.git third_party/openssl

# Initialize and update submodules
git submodule update --init --recursive

# Update submodules to latest
git submodule update --remote --recursive
```

### CCGO.toml with Submodules

```toml
[dependencies]
# Git submodule dependency
openssl = { path = "third_party/openssl", type = "submodule" }

# Specific version via git tag
boost = {
    git = "https://github.com/boostorg/boost.git",
    tag = "boost-1.80.0",
    type = "submodule"
}

# Branch tracking
nlohmann_json = {
    git = "https://github.com/nlohmann/json.git",
    branch = "develop",
    type = "submodule"
}
```

### Submodule Workflow

```bash
# Clone project with submodules
git clone --recurse-submodules https://github.com/user/myproject.git

# Update existing clone
git submodule update --init --recursive

# Update specific submodule
cd third_party/openssl
git checkout v3.0.0
cd ../..
git add third_party/openssl
git commit -m "chore: update openssl to v3.0.0"

# Remove submodule
git submodule deinit third_party/oldlib
git rm third_party/oldlib
```

## Commit Metadata Injection

### Build-Time Git Info

CCGO automatically injects git metadata into builds:

```cpp
// Auto-generated in include/<project>/version.h
#define MYLIB_GIT_SHA "8f3a2b1c"
#define MYLIB_GIT_BRANCH "main"
#define MYLIB_GIT_TAG "v1.2.3"
#define MYLIB_GIT_DIRTY 0  // 1 if uncommitted changes

namespace mylib {
    const char* get_git_sha();
    const char* get_git_branch();
    const char* get_git_tag();
    bool is_git_dirty();
}
```

### Using Git Info in Code

```cpp
#include "mylib/version.h"
#include <iostream>

void print_build_info() {
    std::cout << "Version: " << mylib::get_version() << "\n";
    std::cout << "Git SHA: " << mylib::get_git_sha() << "\n";
    std::cout << "Git Branch: " << mylib::get_git_branch() << "\n";
    std::cout << "Git Tag: " << mylib::get_git_tag() << "\n";

    if (mylib::is_git_dirty()) {
        std::cout << "Warning: Built from dirty working tree\n";
    }
}
```

### Disabling Git Injection

```bash
# Build without git metadata
ccgo build android --no-git-inject

# Or configure in CCGO.toml
[version]
inject_git_metadata = false
```

## Branch Protection

### GitHub Branch Protection Rules

```yaml
# .github/branch-protection.yml
main:
  required_status_checks:
    strict: true
    contexts:
      - "build-android"
      - "build-ios"
      - "test-all"
      - "lint"

  required_pull_request_reviews:
    required_approving_review_count: 2
    dismiss_stale_reviews: true

  enforce_admins: false
  restrictions: null

develop:
  required_status_checks:
    strict: false
    contexts:
      - "build-android"
      - "test-all"

  required_pull_request_reviews:
    required_approving_review_count: 1
```

### Pre-Receive Hook (Server-Side)

```bash
# Prevent direct pushes to main
#!/bin/bash

while read oldrev newrev refname; do
    if [ "$refname" = "refs/heads/main" ]; then
        echo "Error: Direct pushes to main are not allowed"
        echo "Please create a pull request instead"
        exit 1
    fi
done
```

## CI/CD Integration

### GitHub Actions

```yaml
name: CI Build
on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]
  create:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
        platform: [android, ios]

    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive
          fetch-depth: 0  # Full history for git describe

      - name: Install CCGO
        run: pip install ccgo

      - name: Build
        run: ccgo build ${{ matrix.platform }} --release

      - name: Test
        run: ccgo test ${{ matrix.platform }}

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: ${{ matrix.platform }}-build
          path: target/${{ matrix.platform }}/
```

### GitLab CI

```yaml
stages:
  - build
  - test
  - deploy

variables:
  GIT_SUBMODULE_STRATEGY: recursive

build:android:
  stage: build
  tags: [android]
  script:
    - ccgo build android --release
  artifacts:
    paths:
      - target/android/
    expire_in: 1 week

build:ios:
  stage: build
  tags: [macos]
  script:
    - ccgo build ios --release
  artifacts:
    paths:
      - target/ios/
    expire_in: 1 week

test:all:
  stage: test
  script:
    - ccgo test --all
  dependencies:
    - build:android
    - build:ios

deploy:release:
  stage: deploy
  only:
    - tags
  script:
    - ccgo publish --all --registry official
```

### Jenkins Pipeline

```groovy
pipeline {
    agent any

    environment {
        CCGO_VERSION = sh(script: "grep '^version' CCGO.toml | cut -d'\"' -f2", returnStdout: true).trim()
    }

    stages {
        stage('Checkout') {
            steps {
                checkout scm
                sh 'git submodule update --init --recursive'
            }
        }

        stage('Build') {
            parallel {
                stage('Android') {
                    steps {
                        sh 'ccgo build android --release'
                    }
                }
                stage('iOS') {
                    steps {
                        sh 'ccgo build ios --release'
                    }
                }
            }
        }

        stage('Test') {
            steps {
                sh 'ccgo test --all'
            }
        }

        stage('Tag') {
            when {
                branch 'main'
            }
            steps {
                sh "ccgo tag v${CCGO_VERSION} --message 'Release ${CCGO_VERSION}' --push"
            }
        }
    }
}
```

## Monorepo Management

### Git Subtree for Shared Libraries

```bash
# Add shared library as subtree
git subtree add --prefix=libs/common \
    https://github.com/company/common-lib.git main --squash

# Update subtree
git subtree pull --prefix=libs/common \
    https://github.com/company/common-lib.git main --squash

# Push changes back to subtree
git subtree push --prefix=libs/common \
    https://github.com/company/common-lib.git feature-branch
```

### Sparse Checkout

```bash
# Clone only specific directories
git clone --filter=blob:none --sparse https://github.com/user/monorepo.git
cd monorepo
git sparse-checkout set libs/mylib docs/mylib
```

## Git LFS for Binary Assets

### Setup

```bash
# Install Git LFS
git lfs install

# Track binary files
git lfs track "*.so"
git lfs track "*.dylib"
git lfs track "*.dll"
git lfs track "*.a"
git lfs track "assets/*.png"

# Commit .gitattributes
git add .gitattributes
git commit -m "chore: add Git LFS tracking"
```

### .gitattributes

```
# Native libraries
*.so filter=lfs diff=lfs merge=lfs -text
*.dylib filter=lfs diff=lfs merge=lfs -text
*.dll filter=lfs diff=lfs merge=lfs -text
*.a filter=lfs diff=lfs merge=lfs -text

# Assets
*.png filter=lfs diff=lfs merge=lfs -text
*.jpg filter=lfs diff=lfs merge=lfs -text
*.mp4 filter=lfs diff=lfs merge=lfs -text
```

## Best Practices

### 1. Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```bash
# Good commit messages
git commit -m "feat(android): add ARMv7 support"
git commit -m "fix(ios): resolve memory leak in network module"
git commit -m "docs: update installation guide"
git commit -m "chore: bump version to 1.2.3"
git commit -m "refactor(core): simplify error handling"
git commit -m "test: add unit tests for calculator module"

# Breaking change
git commit -m "feat(api)!: change function signature

BREAKING CHANGE: Calculator.add() now returns Result<int>"
```

### 2. Branch Naming

```bash
# Feature branches
feature/user-authentication
feature/android-arm64-support

# Bug fix branches
fix/memory-leak-ios
fix/crash-on-startup

# Release branches
release/1.2.0
release/2.0.0-rc.1

# Hotfix branches
hotfix/1.2.1
hotfix/security-patch
```

### 3. Pull Request Workflow

```bash
# 1. Create feature branch
git checkout -b feature/new-api

# 2. Make changes and commit frequently
git commit -am "wip: implement new API"
git commit -am "feat: complete new API implementation"
git commit -am "test: add API tests"

# 3. Rebase on main before PR
git fetch origin
git rebase origin/main

# 4. Push and create PR
git push origin feature/new-api

# 5. Address review comments
git commit -am "fix: address review comments"
git push origin feature/new-api

# 6. Squash commits before merge (optional)
git rebase -i origin/main
```

### 4. Signing Commits

```bash
# Generate GPG key
gpg --gen-key

# Configure git to use GPG key
git config --global user.signingkey YOUR_KEY_ID
git config --global commit.gpgsign true

# Sign commits
git commit -S -m "feat: add signed commit"

# Verify signature
git log --show-signature
```

### 5. Git Aliases

```bash
# Add useful aliases to ~/.gitconfig
[alias]
    co = checkout
    br = branch
    ci = commit
    st = status
    unstage = reset HEAD --
    last = log -1 HEAD
    visual = log --graph --oneline --decorate --all

    # CCGO-specific aliases
    ccgo-build = "!f() { ccgo build $1 --release; }; f"
    ccgo-tag = "!f() { ccgo tag --push; }; f"
```

## Troubleshooting

### Submodule Issues

**Problem:**
```
fatal: No url found for submodule path 'third_party/lib'
```

**Solution:**
```bash
# Re-initialize submodules
git submodule deinit -f third_party/lib
rm -rf .git/modules/third_party/lib
git submodule update --init --recursive
```

### Detached HEAD in Submodule

**Problem:**
```
You are in 'detached HEAD' state in submodule
```

**Solution:**
```bash
cd third_party/lib
git checkout main  # or appropriate branch
cd ../..
git add third_party/lib
git commit -m "chore: fix submodule detached HEAD"
```

### Large Repository Size

**Problem:**
Repository is too large due to binary files in history

**Solution:**
```bash
# Use git filter-repo (safer than filter-branch)
pip install git-filter-repo

# Remove large files from history
git filter-repo --path-glob '*.so' --invert-paths
git filter-repo --path-glob '*.a' --invert-paths

# Migrate to Git LFS
git lfs migrate import --include="*.so,*.a,*.dylib,*.dll"
```

## Resources

### Tools

- [Git](https://git-scm.com/)
- [Git LFS](https://git-lfs.github.com/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [GitFlow](https://nvie.com/posts/a-successful-git-branching-model/)

### CCGO Documentation

- [CLI Reference](../reference/cli.md)
- [Version Management](version-management.md)
- [CI/CD Setup](../development/contributing.md)
- [Publishing Guide](publishing.md)

### Community

- [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions)
- [Issue Tracker](https://github.com/zhlinh/ccgo/issues)

## Next Steps

- [Version Management](version-management.md)
- [Publishing Guide](publishing.md)
- [CI/CD Setup](../development/contributing.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
