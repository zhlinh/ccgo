# CCGO Documentation

This directory contains the source files for CCGO documentation, built with [MkDocs](https://www.mkdocs.org/) and [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/).

## Documentation Structure

```
docs/
├── index.md                 # Home page (English)
├── index.zh.md              # Home page (Chinese)
├── getting-started/         # Getting started guides
│   ├── installation.md
│   ├── installation.zh.md
│   ├── quickstart.md
│   ├── quickstart.zh.md
│   ├── configuration.md
│   └── project-structure.md
├── platforms/               # Platform-specific guides
│   ├── index.md
│   ├── android.md
│   ├── ios.md
│   ├── macos.md
│   ├── windows.md
│   ├── linux.md
│   ├── ohos.md
│   └── kmp.md
├── features/                # Feature documentation
│   ├── build-system.md
│   ├── dependency-management.md
│   ├── publishing.md
│   ├── docker-builds.md
│   ├── version-management.md
│   └── git-integration.md
├── reference/               # Reference documentation
│   ├── cli.md
│   ├── ccgo-toml.md
│   ├── cmake.md
│   └── gradle-plugins.md
├── development/             # Development guides
│   ├── contributing.md
│   ├── contributing.zh.md
│   ├── roadmap.md
│   ├── roadmap.zh.md
│   ├── changelog.md
│   └── architecture.md
└── requirements.txt         # Python dependencies
```

## Building Documentation Locally

### Prerequisites

```bash
# Install Python 3.8+
python3 --version

# Install dependencies
pip3 install -r docs/requirements.txt
```

### Serve Documentation

```bash
# From project root
mkdocs serve

# Open http://127.0.0.1:8000 in your browser
```

### Build Static Site

```bash
# Build to site/ directory
mkdocs build

# Build with strict mode (fail on warnings)
mkdocs build --strict
```

## Multi-language Support

Documentation supports English and Chinese:

- English files: `filename.md`
- Chinese files: `filename.zh.md`

The language switcher appears in the site header.

### Adding a New Language

1. Update `mkdocs.yml`:
   ```yaml
   plugins:
     - i18n:
         languages:
           - locale: fr
             name: Français
             build: true
   ```

2. Create translated files with `.fr.md` suffix

3. Add translations to `nav_translations` section

## Writing Documentation

### Style Guide

- Use clear, concise language
- Include code examples for complex concepts
- Add command output where helpful
- Use admonitions for important notes
- Cross-link related documentation

### Code Blocks

Use fenced code blocks with language specifiers:

\`\`\`bash
ccgo build android --arch arm64-v8a
\`\`\`

\`\`\`toml
[package]
name = "mylib"
version = "1.0.0"
\`\`\`

### Admonitions

```markdown
!!! note
    This is a note.

!!! warning
    This is a warning.

!!! tip
    This is a tip.
```

### Tabbed Content

```markdown
=== "Linux"
    Linux-specific content

=== "macOS"
    macOS-specific content

=== "Windows"
    Windows-specific content
```

## Publishing

### ReadTheDocs

Documentation is automatically built and published to ReadTheDocs on every push to main branch.

- Site: https://ccgo.readthedocs.io
- Admin: https://readthedocs.org/projects/ccgo/

### Manual Deployment

```bash
# Build and deploy to GitHub Pages
mkdocs gh-deploy
```

## Contributing

When contributing documentation:

1. Follow the existing structure and style
2. Test locally with `mkdocs serve`
3. Check for broken links with `mkdocs build --strict`
4. Update both English and Chinese versions
5. Submit a pull request

See [Contributing Guide](development/contributing.md) for details.

## Troubleshooting

### Build Errors

```bash
# Clear build cache
rm -rf site/

# Rebuild
mkdocs build --strict
```

### Live Reload Not Working

```bash
# Try different port
mkdocs serve --dev-addr 127.0.0.1:8001
```

### Missing Dependencies

```bash
# Reinstall dependencies
pip3 install -r docs/requirements.txt --upgrade
```

## Resources

- [MkDocs Documentation](https://www.mkdocs.org/)
- [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/)
- [Python Markdown Extensions](https://facelessuser.github.io/pymdown-extensions/)
- [mkdocs-static-i18n Plugin](https://github.com/ultrabug/mkdocs-static-i18n)
