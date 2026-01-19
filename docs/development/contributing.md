# Contributing to CCGO

Thank you for your interest in contributing to CCGO! This guide will help you get started.

## Ways to Contribute

- **Report Bugs**: File issues with detailed reproduction steps
- **Suggest Features**: Propose new features or improvements
- **Write Documentation**: Improve docs, add examples, fix typos
- **Submit Code**: Fix bugs, implement features, improve performance
- **Share Feedback**: Tell us about your experience using CCGO

## Getting Started

### 1. Fork and Clone

```bash
# Fork the repository on GitHub
# Then clone your fork
git clone https://github.com/YOUR_USERNAME/ccgo.git
cd ccgo
```

### 2. Set Up Development Environment

```bash
# Install Python dependencies
cd ccgo
pip3 install -e ".[dev]"

# Or install Rust version
cd ccgo-rs
cargo build
```

### 3. Create a Branch

```bash
git checkout -b feature/my-new-feature
# or
git checkout -b fix/issue-123
```

## Development Workflow

### For Python CLI (`/ccgo/`)

```bash
cd ccgo

# Install in editable mode
pip3 install -e .

# Run tests
pytest tests/

# Run linters
flake8 .
black .
mypy .

# Test CLI command
ccgo --version
```

### For Rust CLI (`/ccgo-rs/`)

```bash
cd ccgo-rs

# Build
cargo build

# Run tests
cargo test

# Run linters
cargo clippy
cargo fmt

# Install locally for testing
cargo install --path .
```

### For Gradle Plugins (`/ccgo-gradle-plugins/`)

```bash
cd ccgo-gradle-plugins

# Build plugins
./gradlew build

# Publish to Maven Local for testing
./gradlew publishToMavenLocal

# Test in a project
# Add mavenLocal() to pluginManagement.repositories
```

### For Templates (`/ccgo-template/`)

```bash
# Test template generation
copier copy ccgo-template/ test-output/ --vcs-ref HEAD --trust

# Test in existing project
cd existing-project
copier update --vcs-ref HEAD
```

## Code Style

### Python
- Follow [PEP 8](https://www.python.org/dev/peps/pep-0008/)
- Use [Black](https://black.readthedocs.io/) for formatting
- Use type hints where possible
- Write docstrings for public APIs

### Rust
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Write doc comments for public items

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Examples:
```
feat(build): add support for RISC-V architecture
fix(android): resolve AAR packaging issue with native libs
docs(readme): update installation instructions
```

## Testing

### Write Tests

- Add unit tests for new functionality
- Add integration tests for user-facing features
- Ensure tests pass on all supported platforms
- Aim for >80% code coverage

### Run Tests

```bash
# Python
pytest tests/ -v

# Rust
cargo test

# Integration tests
cd ccgo-now/ccgonow
ccgo build android --arch arm64-v8a
ccgo test
```

## Documentation

### Update Documentation

- Add docstrings/doc comments for new APIs
- Update README.md if adding user-facing features
- Update relevant docs in `/docs/` directory
- Add examples for complex features

### Build Documentation Locally

```bash
# Install MkDocs
pip install -r docs/requirements.txt

# Serve documentation locally
mkdocs serve

# Open http://localhost:8000 in your browser
```

## Pull Request Process

### 1. Prepare Your PR

- Ensure all tests pass
- Update documentation
- Add entry to CHANGELOG.md (if applicable)
- Rebase on latest main branch

```bash
git fetch upstream
git rebase upstream/main
```

### 2. Submit PR

- Push your branch to your fork
- Open a Pull Request on GitHub
- Fill out the PR template completely
- Link related issues using #issue_number

### 3. Code Review

- Address reviewer feedback
- Keep commits clean and atomic
- Be responsive to questions/suggestions
- CI checks must pass

### 4. Merge

- Squash commits if requested
- Maintainer will merge when ready
- Delete your branch after merge

## Issue Guidelines

### Reporting Bugs

Include:
- CCGO version (`ccgo --version`)
- Operating system and version
- Steps to reproduce
- Expected vs actual behavior
- Error messages and logs
- Minimal reproduction case if possible

### Requesting Features

Include:
- Clear description of the feature
- Use cases and benefits
- Proposed API/interface (if applicable)
- Alternatives you've considered

## Community Guidelines

- Be respectful and inclusive
- Follow our [Code of Conduct](https://github.com/zhlinh/ccgo/blob/main/CODE_OF_CONDUCT.md)
- Help others in discussions
- Give constructive feedback
- Celebrate contributions

## Getting Help

- Check existing [documentation](https://ccgo.readthedocs.io)
- Search [existing issues](https://github.com/zhlinh/ccgo/issues)
- Ask in [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions)
- Join our community chat (coming soon)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to CCGO! 🎉
