# Publishing CCGO to PyPI

This guide explains how to publish the `ccgo` package to PyPI.

## Prerequisites

### 1. PyPI Account
- Create an account at [PyPI](https://pypi.org/) for production releases
- Create an account at [TestPyPI](https://test.pypi.org/) for testing (optional but recommended)

### 2. API Tokens
Generate API tokens for uploading packages:

**For PyPI:**
1. Go to https://pypi.org/manage/account/token/
2. Create a new API token with "Entire account" scope
3. Save the token securely (it will only be shown once)

**For TestPyPI:**
1. Go to https://test.pypi.org/manage/account/token/
2. Create a new API token with "Entire account" scope
3. Save the token securely

### 3. Configure PyPI Credentials

Create or edit `~/.pypirc`:

```ini
[distutils]
index-servers =
    pypi
    testpypi

[pypi]
username = __token__
password = pypi-<your-pypi-token>

[testpypi]
repository = https://test.pypi.org/legacy/
username = __token__
password = pypi-<your-testpypi-token>
```

**Important:** Replace `<your-pypi-token>` and `<your-testpypi-token>` with your actual tokens.

Set proper permissions:
```bash
chmod 600 ~/.pypirc
```

### 4. Install Required Tools

The publish script will automatically install required dependencies, but you can install them manually:

```bash
pip3 install build twine
```

## Development Installation

For local development and testing, install ccgo in editable mode:

```bash
# Using the publishing script
python3 publish_to_pypi.py --dev

# Or manually
pip3 install -e .
```

This allows you to test changes immediately without reinstalling.

## Publishing Workflow

### Quick Start

```bash
# Recommended: Using the publishing script
python3 publish_to_pypi.py --check    # Build and verify
python3 publish_to_pypi.py --test     # Test on TestPyPI
python3 publish_to_pypi.py            # Publish to PyPI

# Advanced: Using standard tools directly
python3 -m build && python3 -m twine upload dist/*
```

### Detailed Steps

#### Step 1: Update Version

Edit `pyproject.toml` (recommended) or `setup.py` and update the version number:

**Using pyproject.toml (recommended):**
```toml
[project]
name = "ccgo"
version = "2.1.1"  # Update this
```

**Or using setup.py (legacy):**
```python
setup(
    name="ccgo",
    version="2.1.1",  # Update this
    ...
)
```

Follow [Semantic Versioning](https://semver.org/):
- **Major** (X.0.0): Incompatible API changes
- **Minor** (x.Y.0): New functionality, backwards compatible
- **Patch** (x.y.Z): Bug fixes, backwards compatible

#### Step 2: Update Changelog

Document changes in your CHANGELOG.md or release notes.

#### Step 3: Commit Changes

```bash
git add setup.py CHANGELOG.md
git commit -m "Bump version to 2.1.1"
```

#### Step 4: Build and Check Package

```bash
python3 publish_to_pypi.py --check
```

This will:
- Clean old build artifacts
- Check dependencies
- Build source distribution (`.tar.gz`)
- Build wheel distribution (`.whl`)
- Validate package with `twine check`

#### Step 5: Test on TestPyPI (Recommended)

```bash
python3 publish_to_pypi.py --test
```

After upload, test installation:
```bash
pip3 install --index-url https://test.pypi.org/simple/ ccgo==2.1.1
```

#### Step 6: Publish to PyPI

```bash
python3 publish_to_pypi.py
```

The script will:
1. Check git status for uncommitted changes
2. Clean build artifacts
3. Check dependencies
4. Build distributions
5. Validate with twine
6. Ask for confirmation before uploading
7. Upload to PyPI

#### Step 7: Create Git Tag

After successful upload:

```bash
git tag -a v2.1.1 -m "Release version 2.1.1"
git push origin v2.1.1
```

#### Step 8: Create GitHub Release

1. Go to https://github.com/zhlinh/ccgo/releases/new
2. Select the tag you just created
3. Add release notes
4. Publish release

## Publishing Tools Usage

### Python Script (publish_to_pypi.py)

Comprehensive publishing script for all tasks:

```bash
# Development and installation
python3 publish_to_pypi.py --dev         # Install in development mode (editable)
python3 publish_to_pypi.py --install     # Install package locally
python3 publish_to_pypi.py --uninstall   # Uninstall package

# Publishing workflow
python3 publish_to_pypi.py --check       # Build and check package
python3 publish_to_pypi.py --test        # Upload to TestPyPI
python3 publish_to_pypi.py               # Publish to PyPI (production)
python3 publish_to_pypi.py --clean       # Clean build artifacts

# Advanced options
python3 publish_to_pypi.py --skip-git-check  # Skip git status check
```

### Using Standard Python Tools Directly

For those who prefer standard Python packaging tools:

```bash
# Build package
python3 -m build

# Check package
python3 -m twine check dist/*

# Upload to TestPyPI
python3 -m twine upload --repository testpypi dist/*

# Upload to PyPI
python3 -m twine upload dist/*

# Clean (manual)
rm -rf build/ dist/ *.egg-info/
```

## Troubleshooting

### Authentication Errors

If you get authentication errors:
1. Check that `~/.pypirc` exists and has correct permissions (`chmod 600 ~/.pypirc`)
2. Verify your API tokens are correct
3. Ensure tokens have proper scope (entire account or specific project)

### Package Already Exists

PyPI does not allow re-uploading the same version. If you need to make changes:
1. Increment the version number in `setup.py`
2. Rebuild and re-upload

### Build Errors

If the build fails:
1. Ensure all required files are present (setup.py, README.md, etc.)
2. Check that `setup.py` has correct syntax
3. Verify all dependencies are properly listed in `install_requires`

### Upload Timeout

If upload times out:
1. Check your internet connection
2. Try again later (PyPI might be experiencing issues)
3. Try uploading to TestPyPI first to verify connectivity

## Best Practices

1. **Always test on TestPyPI first** before publishing to production PyPI
2. **Use semantic versioning** for version numbers
3. **Keep a CHANGELOG** to document changes between versions
4. **Create git tags** for each release
5. **Test installation** from PyPI after publishing
6. **Never commit** PyPI credentials to version control
7. **Use API tokens** instead of passwords
8. **Review package contents** before uploading (check with `make check`)

## Verification

After publishing, verify your package:

```bash
# Check package page
open https://pypi.org/project/ccgo/

# Install from PyPI
pip3 install ccgo

# Test basic functionality
ccgo --help
```

## References

- [PyPI Documentation](https://packaging.python.org/en/latest/tutorials/packaging-projects/)
- [Twine Documentation](https://twine.readthedocs.io/)
- [Semantic Versioning](https://semver.org/)
- [Python Packaging User Guide](https://packaging.python.org/)
