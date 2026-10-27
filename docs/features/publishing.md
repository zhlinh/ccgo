# Publishing Management

Complete guide to publishing C++ libraries to various package registries with CCGO.

## Overview

Use `ccgo publish package` to publish a CCGO package to a registry. The previous
`ccgo publish index` spelling remains a compatibility alias; existing `--index-*`
options and index configuration are unchanged.

CCGO provides unified publishing capabilities across multiple platforms and package managers:

- **Android**: Maven Local, Maven Central, Private Maven repositories
- **iOS/macOS/Apple**: CocoaPods, Swift Package Manager (SPM)
- **OpenHarmony**: OHPM (official and private registries)
- **Cross-platform**: Conan (local and remote)
- **KMP**: Kotlin Multiplatform Maven publishing
- **Documentation**: GitHub Pages

Publishing defaults to `--registry local`. Doc uses `--doc-*` and does not read `--registry`.

| Target | `local` | `official` | `private` |
| --- | --- | --- | --- |
| Android / KMP | Maven Local | Maven Central | Configured custom Maven repositories |
| OHOS | Local HAR validation with `ohpm prepublish` | Fixed official OHPM URL | OHPM URL supplied by `--remote` |
| CocoaPods | Validate podspec | CocoaPods Trunk | Specs repository selected by `--remote` |
| SPM | Validate manifest; `--push` creates a local version tag | No official central publishing endpoint; errors | Git remote selected by `--remote`; `--push` publishes a version tag |
| Conan | Export to local cache | Fixed ConanCenter contribution workflow; errors with PR guidance, no direct upload | Conan remote selected by `--remote` |
| Package | Validate input; `--push` updates/commits the local index | Default CCGO index | Configured CCGO registry selected by `--remote` |

SPM and Index require `--push` to publish. Without it, they validate only: no tags, index writes/commits, or index clones/pulls. `local --push` writes locally; remote registries push remotely. The old `--index-push` is a compatibility alias. Apple binary hosting involving SPM also validates only without `--push`; explicit CocoaPods-only artifact-provider uploads retain their existing behavior.

Private destinations use `--remote <URL_OR_NAME>` consistently: Android/KMP and OHOS accept repository URLs; CocoaPods, Conan, SPM and Index accept configured repository names. The old `--url` and `--remote-name` are compatibility aliases for the same option; supplying more than one is rejected. For Android/KMP, `--remote` overrides `CCGO_MAVEN_URL` for Maven publishing; omitting it preserves existing Maven configuration.

For Apple, `--remote` selects the package descriptor destination (Specs name or SPM Git remote). It does not override ZIP storage for `--artifact-provider maven`; use `--maven-registry private` (`CCGO_MAVEN_URL`), `local` (`MAVEN_LOCAL_PATH`), or `official` (signed bundle to Maven Central).

SPM and Index share `--version` and `--tag`: version defaults to `CCGO.toml`, and tag defaults to `v<version>`, matching `ccgo tag`. Use `--version` to override the version or `--tag` to override the tag. A tag alone derives the version by stripping v/V; custom prefixes require an explicit version. `--index-version` and `--index-tag` remain compatibility aliases. Top-level `ccgo --version` still displays the CLI version.

SPM distributes packages directly from their Git repositories and version tags, including private repositories with consumer credentials. An Index is an optional package name/version directory: each selected index aggregates entries, and multiple public/private indexes can coexist. Index publication references an existing source tag, while SPM publication can create one. Their publication responsibilities differ, while options and validation behavior match.

## Quick Start

### Basic Publishing

```bash
# Publish Android library to Maven Local
ccgo publish android --registry local

# Publish to Maven Central
ccgo publish android --registry official

# Publish to private Maven repository
ccgo publish android --registry private --remote https://maven.example.com

# Publish iOS/macOS libraries
ccgo publish apple --manager cocoapods      # CocoaPods
ccgo publish apple --manager spm --registry private --remote origin --push     # Swift Package Manager
ccgo publish apple --manager all --registry private --remote company --push     # Both

# Publish OpenHarmony library
ccgo publish ohos --registry official       # Official OHPM
ccgo publish ohos --registry private --remote https://ohpm.example.com

# Publish Conan package
ccgo publish conan --registry local         # Conan local cache
ccgo publish conan --registry official      # ConanCenter: PR guidance, no direct upload
ccgo publish conan --registry private --remote myrepo

# Publish documentation
ccgo publish doc --doc-branch gh-pages --doc-open
```

## Registry Semantics

`--registry` always selects the **package destination**, never the Maven ZIP host for Apple/Index/Doc artifact uploads. Those use `--artifact-provider maven --maven-registry local|official|private` instead.

| Target | `local` | `official` | `private` | `--remote` |
| --- | --- | --- | --- | --- |
| Android | Local Maven (`MAVEN_LOCAL_PATH` / `~/.m2`) | Maven Central (Central Portal) | Maven URL (`CCGO_MAVEN_URL` or `--remote`) | URL |
| KMP | Local Maven | Maven Central (Central Portal) | Maven URL | URL |
| Apple | CocoaPods lint / SPM wrapper validation | CocoaPods Trunk; SPM has no official upload | Private Specs repo / SPM Git remote | Configured name |
| OHOS | `ohpm prepublish` validation | Official OHPM | Custom OHPM URL | URL |
| Conan | Local Conan create/export | Unsupported (ConanCenter PR) | Conan remote name | Configured name |
| Package / Index | Local index directory | Official CCGO index | Configured index name | Configured name |
| Doc | GitHub Pages (no artifact provider) | Not used | Not used | Not used |

Rules:

- `local` never pushes to a network. It writes a local destination (Maven/index) or validates/exports locally (Apple/OHOS/Conan).
- `official` is the target's official public registry. Where direct CLI upload is unsupported (SPM, Conan), it fails with guidance.
- `private` always uses `--remote`: a URL for Android/KMP/OHOS, a configured remote name for Apple/Index/Conan/SPM.
- `--maven-registry` is independent: `official` uploads a signed bundle to Maven Central, `private` PUTs to `CCGO_MAVEN_URL`, and `local` writes to the local Maven repository. `--registry` does not redirect Maven artifacts to Central.
- `ccgo publish doc` rejects a non-local `--registry` because Doc publishing does not use it; use `--artifact-provider maven --maven-registry ...` for Maven hosting.

### Skip Build

Publish existing artifacts without rebuilding:

```bash
# Use existing AAR
ccgo publish android --registry local --skip-build

# Use existing HAR
ccgo publish ohos --registry official --skip-build
```

## Explicit Publish Paths

Omit `--path` to keep the existing defaults. Relative paths are resolved against the invocation directory. Invalid paths, mismatched file types, and unsupported targets fail before publishing.

Use `--path` for the publish input: a directory or the platform's artifact/descriptor file. Android, OHOS, and KMP also accept SDK ZIPs; Conan and Doc accept ZIPs containing their recipe or MkDocs project.

| Target | Directory input | File input | Default without `--path` |
| --- | --- | --- | --- |
| Android | Artifacts with exactly one AAR | `.aar`, SDK `.zip` containing exactly one AAR | Build `android --release` with ccgo, then upload |
| OHOS | Package containing `oh-package.json5`, or artifacts with exactly one HAR | `.har`, SDK `.zip` containing exactly one HAR | Run OHPM in `ohos/` |
| KMP | Project directory or CCGO KMP SDK directory | CCGO KMP SDK `.zip` | Build `kmp --release` with ccgo, then upload |
| Apple / CocoaPods | Podspec directory; binary directory with `--artifact-provider` | `.podspec`; `.zip` with a provider | Current podspec; release Apple SDK ZIP with provider |
| Apple / SPM | Package directory; binary directory with a provider | `Package.swift`; `.zip` with a provider | Current directory; release Apple SDK ZIP with provider |
| Conan | Recipe directory containing `conanfile.py` | Python recipe, usually `conanfile.py`, or a `.zip` containing it | Existing CCGO project discovery |
| Doc | Project containing `mkdocs.yml` or `mkdocs.yaml` | MkDocs `.yml` / `.yaml` config, or a MkDocs project `.zip` | Current project's MkDocs config |
| Package | Directory containing exactly one SDK ZIP | SDK `.zip` to checksum | Default `target/release/package/` path |

Android/OHOS artifact directories and SDK ZIPs are searched recursively; Index directories are searched recursively for ZIPs. Multiple candidates fail and require an exact file path. Project directories take precedence, identified by `gradlew` for Android and `oh-package.json5` for OHOS. CocoaPods searches only the specified directory's immediate files. Apple file inputs require the corresponding `--manager`; directories used with `--manager all` must contain both a podspec and `Package.swift`.

Explicit artifacts (files, artifact directories, and SDK ZIPs) are reused without rebuilding. Project directories use the platform's existing publishing workflow. ZIPs are extracted temporarily; cleanup removes only directories created by the command, never the original ZIP or input directory.

```bash
ccgo publish android --path ./artifacts/library.aar --registry local
ccgo publish ohos --path ./artifacts/library.har --registry official
ccgo publish apple --manager cocoapods --path ./specs/Library.podspec
ccgo publish kmp --path ./custom-kmp --registry local
ccgo publish android --path ./ANDROID_SDK.zip --registry local
ccgo publish ohos --path ./OHOS_SDK.zip --registry official
ccgo publish apple --manager spm --path ./swift-package
ccgo publish conan --path ./recipes/conanfile.py --registry local
ccgo publish doc --path ./docs-project/mkdocs.yml
ccgo publish doc --path ./docs-project.zip
ccgo publish conan --path ./conan-project.zip --registry local
ccgo publish package --version 1.2.3 --index-repo ./package-index \
  --path ./artifacts/SDK.zip --checksum \
  --archive-url-template 'https://cdn.example.com/{name}-{version}.zip'
```

`ccgo publish android` never invokes Gradle. For prebuilt AAR/SDK ZIP inputs, `--registry local` and `--registry private` upload directly from ccgo — no `android/` project, `gradlew`, or Gradle publish plugin is required. Without `--path`, ccgo runs `ccgo build android --release` first and uploads the resulting AAR directly: the equivalent of building and then passing `--path`. `ccgo build` still invokes Gradle's `assembleProdRelease` task to package the AAR, but the publish step itself never calls Gradle. `--skip-build` without `--path` is rejected because there is no artifact to select; pass the built AAR with `--path`. A `--path` directory is resolved to the nearest ancestor containing `CCGO.toml`, then built with ccgo. `--registry official` (Maven Central) builds the AAR, signs every file, and uploads a bundle to the Central Portal; set `MAVEN_CENTRAL_USERNAME` / `MAVEN_CENTRAL_PASSWORD` (Portal user token) and `SIGNING_IN_MEMORY_KEY` / `SIGNING_IN_MEMORY_KEY_PASSWORD` (or install gpg with a secret key). Coordinates come from project configuration (`[publish.android.maven]`, then `[project].group_id`), not the AAR filename; `channel_desc` and `--stl c++_static` produce the same `-stdembed` artifact id as the plugin. Direct uploads honor `--stl`; `--release` does not transform an existing AAR. Local/private uploads publish the AAR and POM only; the Central bundle also carries minimal sources/javadoc jars. Gradle-derived implicit dependencies are not synthesized, so list any required runtime dependencies under `[publish.android.maven].dependencies`.

CocoaPods runs in the specified podspec's directory. Without an artifact provider, an explicit Index archive requires both `--checksum` and `--archive-url-template`, supports only `--archive-format zip`, and fails on errors reading an explicit archive. Host the ZIP separately at the template URL; this mode does not upload it.

KMP accepts a project directory, or a prebuilt CCGO SDK directory/ZIP containing `lib/kmp/` and `meta/kmp/build_info.json`. A ZIP may have one enclosing directory. `ccgo publish kmp` never invokes Gradle: without `--path` it runs `ccgo build kmp --release`, prepares the produced SDK ZIP, and uploads it; a `--path` project directory is resolved to the nearest `CCGO.toml` root and built with ccgo. `ccgo build` still uses Gradle to compile Kotlin, but the publish step never calls Gradle. `--skip-build` without `--path` is rejected because no SDK can be selected.

```bash
ccgo publish kmp --path ./STDCOMM_KMP_SDK-0.0.5.zip --registry local
ccgo publish kmp --path ./unpacked-sdk --registry private \
  --remote https://maven.example.com/releases
```

SDK versions come from `build_info.json`; its project name must match the current project. Group/base artifact IDs come from `[publish.kmp.maven]`, with the same fallbacks as the plugin (`[publish.android.maven].group_id`, then `[project].group_id`; artifact ID defaults to `<project>-kmp`). An Android-only SDK publishes `<group_id>:<artifact_id>-android:<sdk-version>`. Desktop JARs use `-desktop`; native KLIBs use the lowercase target suffix (for example `-iosarm64`), with cinterop KLIBs attached as classifiers.

Direct uploads use `MAVEN_LOCAL_PATH` / `[publish.maven].local_path` for local repositories, and `--remote` (or `CCGO_MAVEN_URL`) plus `CCGO_MAVEN_USERNAME` / `CCGO_MAVEN_PASSWORD` for private repositories. Files follow the Maven layout (`group/path/artifact/version/artifact-version[-classifier].ext`) and include the POM and SHA-256 checksums. Existing files are replaced, so retrying a failed publish is safe. `--registry official` signs the files and uploads a bundle to the Central Portal, using the same `MAVEN_CENTRAL_USERNAME` / `MAVEN_CENTRAL_PASSWORD` and `SIGNING_IN_MEMORY_KEY` / `SIGNING_IN_MEMORY_KEY_PASSWORD` (or gpg) settings as Android.

These SDKs lack complete POM/Gradle module metadata. SDK mode publishes directly addressable platform modules, without synthesizing a root KMP module or commonMain automatic variant selection. POM dependencies come from configured KMP dependencies (`[publish.kmp.maven].dependencies`); ensure they are complete. The Central bundle also carries minimal sources/javadoc jars. ZIPs are extracted into temporary directories and cleaned up afterwards. Unsafe paths, symlinks, missing metadata, ambiguous main artifacts, and project-name mismatches are rejected.

Doc ZIPs must contain exactly one `mkdocs.yml` or `mkdocs.yaml`; Conan ZIPs must contain exactly one `conanfile.py`. Enclosing directories are allowed: descriptors are searched recursively, with missing or ambiguous inputs rejected. Include all files referenced by relative paths; the archive layout is preserved. Temporary extraction is removed after success or failure, leaving the original ZIP untouched.

For Doc ZIPs, run the command from the Git repository whose documentation you want to deploy. `mkdocs gh-deploy` runs from that checkout while loading the extracted config by absolute path. The ZIP contains a MkDocs project and source documents, not just prebuilt HTML. Conan ZIPs run the existing export/upload workflow from the extracted recipe directory; arbitrary binary ZIPs are not automatically converted into Conan packages.

### Index Artifact Hosting

Index supports `--artifact-provider maven|github`. Pass a ZIP or a directory containing a unique ZIP with `--path`; the default is `target/release/package/<NAME_UPPER>_CCGO_PACKAGE-<version>.zip`. The command uploads the existing archive and records its download URL and SHA-256 automatically. Do not combine a provider with `--archive-url-template`.

```bash
ccgo publish package --registry private --remote company \
  --artifact-provider maven --maven-registry private \
  --artifact-group com.example \
  --path ./STDCOMM_CCGO_PACKAGE-0.0.5.zip --push

# Maven Central: sign and upload a bundle
ccgo publish package --registry private --remote company \
  --artifact-provider maven --maven-registry official \
  --artifact-group com.example \
  --path ./STDCOMM_CCGO_PACKAGE-0.0.5.zip --push
```

`--registry` and `--remote` select the package index. `--maven-registry` selects ZIP storage: `private` (`CCGO_MAVEN_URL`, default), `local` (`MAVEN_LOCAL_PATH`), or `official` (Maven Central via the Central Portal). The group comes from `--artifact-group` or `[publish.index.maven].group_id`; the default artifact ID is `<project>-ccgo`, overridable with `--artifact-id`. For GitHub, use `--artifact-provider github --github-repo company/library`; the release for the selected tag must already exist.

The source Git tag must exist before publishing; Index does not create or push it. You can reuse the same source tag across platforms. Without `--push`, the command only validates, without uploads or index changes. With `--push`, it checks for duplicate indexed versions before uploading. A subsequent failure can leave uploaded artifacts; uploads are not rolled back. Retry the same command with the same artifact to resume the pending publication; confirmed uploads are not repeated. Temporary copies are cleaned up on success or failure; the original ZIP is preserved.

Publish and yank serialize access to a shared index clone. Rejected pushes can merge compatible changes to different versions; conflicting same-version changes fail without overwriting history. Package uploads reuse identical existing objects and refuse different bytes. Maven hosting must support conditional creation (`If-None-Match: *`) or immutable release coordinates for protection against independent concurrent publishers. Pending publication or yank operations block other mutations in their clone; retry the original command after fixing the failure. See [publication recovery](../development/registry-publication-recovery.md) for the protocol, manual recovery boundaries, and verification commands.

### Apple Binary Hosting

Use `--artifact-provider maven|github` to upload an Apple binary directory, a single `.xcframework` directory, or a ZIP via `--path`. Run from a project containing `CCGO.toml` and the existing podspec / `Package.swift` for the selected manager. The default manager is `all`; select `--manager cocoapods` for CocoaPods alone.

`--artifact-provider` selects binary storage. Independently, `--registry local|official|private` selects CocoaPods validation, Trunk, or a private Specs Git repository (`--remote` required). For Apple, `--registry` controls CocoaPods Specs publishing, independently of Maven/GitHub binary storage. SPM local/private select wrapper validation or Git tag publishing; official has no direct upload endpoint. With `--artifact-provider maven`, `--maven-registry local|official|private` selects the ZIP destination.

```bash
ccgo publish apple --manager cocoapods \
  --path ./STDCOMM_APPLE_SDK-0.0.5.zip \
  --artifact-provider maven --maven-registry private \
  --artifact-group com.example \
  --registry private --remote company-specs

ccgo publish apple --manager spm \
  --path ./Library.xcframework \
  --artifact-provider github --github-repo company/library --push
```

`private` Maven storage uses a single `CCGO_MAVEN_URL`, `CCGO_MAVEN_USERNAME`, and `CCGO_MAVEN_PASSWORD` and must support hosted Maven PUT uploads. `local` writes to `MAVEN_LOCAL_PATH` (or `~/.m2/repository`). `official` signs the ZIP/POM and uploads a bundle to the Central Portal. Set groupId with `--artifact-group` or `[publish.apple.maven].group_id`. ArtifactId defaults to `<project>-apple`, overridable with `--artifact-id`; version comes from `CCGO.toml` and must match the podspec. The ZIP, a POM with ZIP packaging, and their checksums are uploaded. Comma-separated repository URLs are not supported in private mode.

GitHub uses authenticated `gh` and requires `--github-repo owner/repo` plus an existing `v<version>` Release. The asset is named `<artifactId>-<version>.zip`; existing assets are not overwritten.

Without `--path`, provider mode selects one SDK ZIP for the current project version from `target/release/ios/` and `target/release/macos/`, excluding `-SYMBOLS.zip`. Use `--platform ios|macos` to narrow this down; missing or ambiguous inputs fail. This mode does not build.

ZIP bytes are preserved; directories are temporarily zipped. Directory symlinks must resolve within the input directory. Temporary files are cleaned on success and failure; original inputs remain intact. Generated manifests remain under `target/ccgo-publish/apple/<version>/` for inspection. Source manifests are unchanged. CocoaPods preserves existing metadata and vendored paths, replacing only HTTP source and SHA-256; ensure those paths match the archive.

SPM currently requires one XCFramework at the ZIP root and an existing manifest containing one remote `binaryTarget` with literal URL/checksum strings. Provider mode validates only without `--push`; with `--push` it uploads the ZIP and generates `Package.swift`. Commit that generated file to the wrapper repository, then use ordinary SPM publishing to push its version tag. This avoids tagging a wrapper that still references the old artifact. Nested SDK archives or multiple XCFrameworks can use CocoaPods; the command does not convert them into SPM archives.

Publishing credentials are not embedded in manifests or propagated to consumers. Private downloads need consumer-side authentication. Explicit provider uploads still occur with `--registry local`, followed by `pod spec lint` against the remote ZIP. Uploads are not transactional: later failures do not roll back previously uploaded artifacts; inspect remote state before retrying.

## Android Publishing (Maven)

### Registry Types

**local**: Maven Local (~/.m2/repository/)
- No authentication required
- Immediate availability
- Perfect for local testing
- Not accessible to others

**official**: Maven Central (sonatype.org)
- Requires Sonatype account
- PGP signing required
- Review process (2-4 hours)
- Globally accessible

**private**: Custom Maven repository
- Company/team repositories
- Authentication required
- Immediate availability
- Team accessible

### Configuration

**gradle.properties:**

```properties
# Maven Central credentials
SONATYPE_USERNAME=your-username
SONATYPE_PASSWORD=your-password

# PGP signing
signing.keyId=12345678
signing.password=your-password
signing.secretKeyRingFile=/path/to/secring.gpg

# Private repository
PRIVATE_MAVEN_URL=https://maven.example.com
PRIVATE_MAVEN_USERNAME=your-username
PRIVATE_MAVEN_PASSWORD=your-password
```

### Publishing Commands

```bash
# Publish to Maven Local for testing
ccgo publish android --registry local

# Publish to Maven Central (production)
ccgo publish android --registry official

# Publish to private Maven
export CCGO_MAVEN_USERNAME=admin
export CCGO_MAVEN_PASSWORD=secret
ccgo publish android --registry private \
    --remote https://maven.example.com
```

### Maven Central Publishing

**Prerequisites:**

1. Create a Central Portal account at https://central.sonatype.com/ and verify your namespace.
2. Generate and register a PGP key.

**Configure credentials and signing:**

```bash
# Central Portal user token (not your login password)
export MAVEN_CENTRAL_USERNAME=your-username
export MAVEN_CENTRAL_PASSWORD=your-password

# ASCII-armored secret key; or install gpg with a secret key
export SIGNING_IN_MEMORY_KEY="$(cat /path/to/secret-key.asc)"
export SIGNING_IN_MEMORY_KEY_PASSWORD=your-key-password
```

**Publishing process:**

```bash
ccgo publish android --registry official
```

ccgo builds the AAR, signs every file, uploads one bundle to the Central Portal, and follows the deployment until it is `PUBLISHED` (AUTOMATIC).

## iOS/macOS Publishing (Apple Platforms)

### Package Managers

**CocoaPods**: Traditional dependency manager
- Podspec-based
- Central repository (CocoaPods Trunk)
- Wide adoption

**Swift Package Manager (SPM)**: Apple's official solution
- Git-based
- No central repository
- Native Xcode integration

### CocoaPods Publishing

**Setup:**

```bash
# Register with CocoaPods Trunk
pod trunk register your@email.com 'Your Name'

# Verify registration (check email)
```

**Publish:**

```bash
# Publish to CocoaPods Trunk (official)
ccgo publish apple --manager cocoapods --registry official

# Publish to private spec repo
ccgo publish apple --manager cocoapods \
    --registry private \
    --remote myspecs \
    --remote https://github.com/company/specs.git
```

**Podspec validation:**

```bash
# Validate before publishing
pod spec lint MyLib.podspec

# Validate with verbose output
pod spec lint MyLib.podspec --verbose
```

### Swift Package Manager Publishing

Local mode validates the manifest with `swift package dump-package`; adding `--push` creates only the local version tag, without contacting a remote. Private mode means a custom Git remote, which may host a public or private repository: select it with `--remote origin` and add `--push` to create/push the project version tag. Configure that remote with `git remote add`. Commit all wrapper files first. With `--push`, a missing tag is created; an existing tag pointing at HEAD is reused, including repeated pushes. A tag pointing at another commit is rejected and never overwritten. Official mode errors because SPM has no central publishing service equivalent to CocoaPods Trunk. This implements Git-based SPM publishing, not the Swift Package Registry HTTP protocol or Swift Package Index registration.

**Setup:**

```bash
# SPM uses Git tags for versions
# Ensure repository is initialized
git init
git add .
git commit -m "Initial commit"
```

**Publish:**

```bash
# Tag and push (SPM publishing)
ccgo publish apple --manager spm --registry private --remote origin --push

# This creates git tag and pushes to remote
# SPM users can then reference your repository
```

**Manual SPM publishing:**

```bash
# Create version tag
git tag 1.0.0
git push origin 1.0.0

# Users add to Package.swift:
# .package(url: "https://github.com/user/repo.git", from: "1.0.0")
```

### Publishing Both

```bash
# Publish to both CocoaPods and SPM
ccgo publish apple --manager all --registry private --remote company --push

# company must be both a configured CocoaPods Specs repository and a Git remote
# Run the managers separately if the names differ
# Commit the SPM wrapper before publishing
```

## OpenHarmony Publishing (OHPM)

### Registry Types

**local**: Run `ohpm prepublish <HAR>` for local content validation. This neither uploads nor installs into a consuming project. Requires an OHPM version supporting `prepublish` and an already built HAR. A project directory must contain exactly one HAR, or select it with `--path`; publishing does not build automatically.


**official**: OpenHarmony Package Manager (ohpm.openharmony.cn)
- Official registry
- Requires account
- Public packages
- Globally accessible

**private**: Custom OHPM registry
- Company/team registries
- Authentication required
- Private packages

### Configuration

**Setup OHPM:**

```bash
# Install OHPM
npm install -g @ohos/hpm-cli

# Login to official registry
ohpm login

# Configure private registry
ohpm config set publish_registry https://ohpm.example.com
```

### Publishing Commands

```bash
# Publish to official OHPM
ccgo publish ohos --registry official

# Publish to private OHPM
ccgo publish ohos --registry private --remote https://ohpm.example.com

# Publish with authentication
ccgo publish ohos --registry private \
    --remote https://ohpm.example.com

# Skip build and use existing HAR
ccgo publish ohos --skip-build
```

### HAR Publishing Process

1. Select an existing HAR from `--path` (file / SDK ZIP / directory), or the default `ohos/` directory
2. Local mode runs `ohpm prepublish` and stops
3. Official mode uploads with `--publish_registry https://ohpm.openharmony.cn/ohpm/`
4. Private mode uploads with `--publish_registry <url>`, using OHPM authentication configuration

**Required metadata:**

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "My OpenHarmony library"
authors = ["Your Name <your@email.com>"]
license = "MIT"
homepage = "https://github.com/user/mylib"
repository = "https://github.com/user/mylib"
```

## Conan Publishing

### Registry Types

**local**: Conan 2 local cache (~/.conan2/)
- No network required
- Immediate availability
- Testing only

**official**: Fixed ConanCenter (`https://center2.conan.io`)
- Never falls back to the first remote or a `--remote` override
- ConanCenter does not accept ordinary CLI uploads; this mode errors with the [recipe PR contribution link](https://github.com/conan-io/conan-center-index)
- Does not create a PR or upload elsewhere automatically

**private**: Custom Conan remote
- Requires `--remote`; never chooses the first remote automatically
- Uploads the exact exported recipe reference, never `*`
- Company repositories
- Authentication required
- Immediate availability

### Configuration

**Setup Conan:**

```bash
# Install Conan
pip install conan

# Add Conan Center
conan remote add conancenter https://center2.conan.io

# Add private remote
conan remote add myrepo https://conan.example.com
conan remote login myrepo username
```

### Publishing Commands

```bash
# Create binaries in the local cache (Debug, both static and shared by default)
ccgo publish conan --registry local

# Show ConanCenter PR guidance (returns an error; no direct upload)
ccgo publish conan --registry official

# Publish to private remote
ccgo publish conan --registry private \
    --remote myrepo

# Skip build, only export recipe
ccgo publish conan --skip-build

# Host profile × build type × linkage matrix; upload after all builds succeed
ccgo publish conan --registry private --remote myrepo \
    --profile linux-x64,linux-arm64 --build-profile default \
    --conan-build-types Debug,Release --link-type both \
    --conan-setting compiler.cppstd=17 --conan-option '*:fPIC=True'
```

Requires Conan 2 and configured profiles. `--release` changes the default build type
to Release. Repeat `--conan-setting` and `--conan-option` for additional settings and
options; control build_type and shared through `--conan-build-types` and
`--link-type`. Uploads select only the exact recipe, package IDs and package
revisions from this matrix, excluding dependencies and unrelated cached packages.
Any failed configuration prevents uploading; local packages remain available for
retry. Authenticate with `conan remote login`.
`--skip-build --registry private --remote myrepo` uploads only the recipe.

### Conan Package Recipe

CCGO generates `conanfile.py`:

```python
from conan import ConanFile
from conan.tools.files import copy

class MylibConan(ConanFile):
    name = "mylib"
    version = "1.0.0"
    description = "My C++ library"
    license = "MIT"
    url = "https://github.com/user/mylib"

    settings = "os", "compiler", "build_type", "arch"
    options = {"shared": [True, False]}
    default_options = {"shared": False}

    def package(self):
        copy(self, "*.h", src=self.source_folder, dst=self.package_folder)
        copy(self, "*.a", src=self.build_folder, dst=self.package_folder)
        copy(self, "*.so", src=self.build_folder, dst=self.package_folder)
```

## Documentation Publishing (GitHub Pages)

### Setup

**Enable GitHub Pages:**

1. Repository Settings → Pages
2. Source: Deploy from branch
3. Branch: `gh-pages` (will be created by CCGO)

### Publishing

```bash
# Generate and publish documentation
ccgo publish doc --doc-branch gh-pages --doc-open

# Force push (overwrites existing docs)
ccgo publish doc --doc-branch gh-pages --doc-force

# Custom commit message
ccgo publish doc --doc-branch gh-pages --doc-message "Update docs for v1.0.0"
```

### Process

1. CCGO generates documentation with Doxygen
2. Converts to HTML
3. Creates `gh-pages` branch (if doesn't exist)
4. Commits documentation
5. Pushes to remote
6. Opens browser to docs URL

### Custom Domain

**Add CNAME file:**

```toml
[doc]
custom_domain = "docs.example.com"
```

CCGO creates `CNAME` file in gh-pages branch.

## Version Management

### Semantic Versioning

CCGO follows semantic versioning (semver):

```
MAJOR.MINOR.PATCH
1.0.0 -> 1.0.1 (bug fix)
1.0.1 -> 1.1.0 (new feature)
1.1.0 -> 2.0.0 (breaking change)
```

### Version in CCGO.toml

```toml
[package]
name = "mylib"
version = "1.2.3"  # Used for all publications
```

### Version Tagging

```bash
# Create version tag
ccgo tag

# Custom tag
ccgo tag v1.2.3 --message "Release version 1.2.3"

# This creates git tag matching CCGO.toml version
```

## Authentication

### Credential Storage

**Environment variables:**

```bash
# Maven Central
export SONATYPE_USERNAME=your-username
export SONATYPE_PASSWORD=your-password

# Private Maven
export PRIVATE_MAVEN_URL=https://maven.example.com
export PRIVATE_MAVEN_USERNAME=admin
export PRIVATE_MAVEN_PASSWORD=secret

# OHPM token
export OHPM_TOKEN=your-token

# Conan
export CONAN_LOGIN_USERNAME=your-username
export CONAN_PASSWORD=your-password
```

**Configuration files:**

```bash
# Gradle: ~/.gradle/gradle.properties
SONATYPE_USERNAME=your-username
SONATYPE_PASSWORD=your-password

# OHPM: ~/.ohpm/auth.json
{
  "registry": {
    "https://ohpm.openharmony.cn": {
      "token": "your-token"
    }
  }
}

# Conan: ~/.conan/remotes.json
{
  "remotes": [
    {
      "name": "myrepo",
      "url": "https://conan.example.com",
      "verify_ssl": true
    }
  ]
}
```

### Security Best Practices

1. **Never commit credentials** to repository
2. **Use environment variables** in CI/CD
3. **Rotate tokens regularly**
4. **Use read-only tokens** where possible
5. **Enable 2FA** on package registries

## CI/CD Integration

### GitHub Actions

```yaml
name: Publish Library

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.10'

      - name: Install CCGO
        run: pip install ccgo

      - name: Publish to Maven Central
        env:
          SONATYPE_USERNAME: ${{ secrets.SONATYPE_USERNAME }}
          SONATYPE_PASSWORD: ${{ secrets.SONATYPE_PASSWORD }}
        run: ccgo publish android --registry official

      - name: Publish to CocoaPods
        env:
          COCOAPODS_TRUNK_TOKEN: ${{ secrets.COCOAPODS_TRUNK_TOKEN }}
        run: ccgo publish apple --manager cocoapods
```

### GitLab CI

```yaml
publish:
  stage: deploy
  only:
    - tags
  script:
    - pip install ccgo
    - ccgo publish android --registry official
    - ccgo publish ohos --registry official
  variables:
    SONATYPE_USERNAME: $SONATYPE_USERNAME
    SONATYPE_PASSWORD: $SONATYPE_PASSWORD
    OHPM_TOKEN: $OHPM_TOKEN
```

### Jenkins

```groovy
pipeline {
    agent any

    stages {
        stage('Publish') {
            when {
                tag '*'
            }
            steps {
                sh 'pip install ccgo'
                withCredentials([
                    usernamePassword(
                        credentialsId: 'maven-central',
                        usernameVariable: 'SONATYPE_USERNAME',
                        passwordVariable: 'SONATYPE_PASSWORD'
                    )
                ]) {
                    sh 'ccgo publish android --registry official'
                }
            }
        }
    }
}
```

## Best Practices

### 1. Test Locally First

Always test with local registries:

```bash
# Test Maven publishing
ccgo publish android --registry local

# Verify installation
# (in consumer project)
implementation 'com.example:mylib:1.0.0'
```

### 2. Version Consistency

Ensure version matches across:
- CCGO.toml
- Git tags
- Package metadata

```bash
# CCGO handles this automatically
ccgo tag        # Creates tag from CCGO.toml
ccgo publish    # Uses CCGO.toml version
```

### 3. Changelog

Maintain CHANGELOG.md:

```markdown
# Changelog

## [1.2.0] - 2024-01-15
### Added
- New feature X
- Support for platform Y

### Fixed
- Bug in module Z

## [1.1.0] - 2024-01-01
...
```

### 4. Publishing Checklist

Before publishing:

- [ ] Update version in CCGO.toml
- [ ] Update CHANGELOG.md
- [ ] Run tests: `ccgo test`
- [ ] Test local build: `ccgo build`
- [ ] Test local publish: `ccgo publish <platform> --registry local`
- [ ] Create git tag: `ccgo tag`
- [ ] Publish: `ccgo publish <platform> --registry official`
- [ ] Verify package is available
- [ ] Update documentation
- [ ] Announce release

### 5. Multi-Platform Publishing

Publish to all platforms:

```bash
#!/bin/bash
# publish-all.sh

VERSION=$(ccgo version)

echo "Publishing version $VERSION to all platforms..."

# Android
ccgo publish android --registry official

# iOS/macOS
ccgo publish apple --manager all --registry private --remote company --push

# OpenHarmony
ccgo publish ohos --registry official

# Conan
ccgo publish conan --registry official

# Documentation
ccgo publish doc --doc-branch gh-pages

echo "Publishing complete!"
```

## Troubleshooting

### Maven Publishing Failed

```
Error: Failed to upload to Maven Central
```

**Solutions:**

1. **Check credentials:**
```bash
echo $SONATYPE_USERNAME
echo $SONATYPE_PASSWORD
```

2. **Verify PGP signing:**
```bash
gpg --list-secret-keys
cat ~/.gradle/gradle.properties | grep signing
```

3. **Check network:**
```bash
curl -I https://s01.oss.sonatype.org/
```

4. **Validate POM:**
```bash
# Check generated POM
cat build/publications/release/pom-default.xml
```

### CocoaPods Push Failed

```
Error: Unable to find a pod with name 'MyLib'
```

**Solutions:**

1. **Verify trunk registration:**
```bash
pod trunk me
```

2. **Validate podspec:**
```bash
pod spec lint MyLib.podspec --verbose
```

3. **Check spec repo:**
```bash
pod repo list
pod repo update
```

### OHPM Publishing Failed

```
Error: Package already exists
```

**Solutions:**

1. **Increment version:**
```toml
[package]
version = "1.0.1"  # Bump version
```

2. **Check existing package:**
```bash
ohpm view mylib
```

3. **Verify authentication:**
```bash
ohpm whoami
```

### Conan Upload Failed

```
Error: Recipe 'mylib/1.0.0' already exists
```

**Solutions:**

1. **Remove existing version:**
```bash
conan remove mylib/1.0.0 -r myrepo
```

2. **Use new version:**
```toml
[package]
version = "1.0.1"
```

3. **Check remote configuration:**
```bash
conan remote list
conan user -r myrepo
```

## See Also

- [Build System](build-system.md)
- [Dependency Management](dependency-management.md)
- [Android Platform](../platforms/android.md)
- [iOS Platform](../platforms/ios.md)
- [OpenHarmony Platform](../platforms/openharmony.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)

## Index Registry Selection

Index `--registry` selects the index Git repository; the package source URL still comes from the project's `origin`.

The version defaults to `[project].version` or `[package].version` in `CCGO.toml`; `--version` is optional. Its existing Git tag defaults to `v<version>`; Index does not create or push source tags. Override the version with `--version` or the tag with `--tag`. With a tag alone, the version is derived by stripping a leading v/V. Supply both flags for custom tag prefixes. Each invocation appends one version; duplicate versions remain errors.

```bash
# Validate the configured version and existing source tag, without updating the index
ccgo publish package --registry local

# Publish to the offline local index, default $CCGO_HOME/registry/publish/local
ccgo publish package --registry local --push

# Dedicated local index directory, separate from the package source repository
ccgo publish package --registry local --index-repo ./package-index --push

# Fixed official CCGO index; pushing requires write access
ccgo publish package --registry official --push

# Configure a private index (or use [registries] in CCGO.toml)
ccgo registry add company git@github.com:company/ccgo-index.git
ccgo publish package --registry private --remote company --push
```

Official mode uses `git@github.com:zhlinh/ccgo-index.git`. Private mode resolves `--remote` from project `[registries]` first, then global `ccgo registry add` configuration. Unknown names fail without falling back. This name identifies a CCGO registry, not a Git remote of the source project. Remote working copies live under `$CCGO_HOME/registry/publish/<name>/` (`$CCGO_HOME` defaults to `~/.ccgo`). Without `--push`, validation does not create/update an index checkout, write commits, or clone/pull. Remote duplicate versions cannot be verified at that stage; actual publication pulls the index and checks duplicates.

The official registry uses the internal name `ccgo-index`. Its SSH URL requires GitHub SSH access, and publishing requires repository write access. A new index needs a root `index.json` matching `IndexMetadata` and an initial commit pushed to its default branch; package files are created on publication. If an existing publishing checkout points to the previous official repository, preserve any local commits and move that checkout aside before retrying; CCGO rejects an origin mismatch instead of silently switching repositories. Existing explicitly saved registry URLs are not rewritten automatically.

`--index-repo` now specifies a local directory only, and `--index-name` only names a local index. Migrate old remote URL usage to `ccgo registry add` plus `--registry private --remote`. `--index-push` remains an alias for `--push`. Local mode never clones/pulls; `local --push` updates and commits the local index without a network push. Pull failures preserve local changes; commit failures stop before pushing.

### Documentation archives in Maven

Without an artifact provider, `ccgo publish doc` continues to deploy to GitHub Pages. Maven mode builds a fresh static site into a temporary directory and archives its contents with `index.html` at the ZIP root. Without `--push`, it only builds and validates; add `--push` to upload. It does not deploy a website or use `--registry` / `--remote`.

```bash
ccgo publish doc --artifact-provider maven --maven-registry local \
  --artifact-group com.example
ccgo publish doc --artifact-provider maven --maven-registry private \
  --artifact-group com.example --push
ccgo publish doc --artifact-provider maven --maven-registry official \
  --artifact-group com.example --push
```

`--maven-registry local` writes to `MAVEN_LOCAL_PATH`; `private` uses `CCGO_MAVEN_URL`, `CCGO_MAVEN_USERNAME` and `CCGO_MAVEN_PASSWORD`; `official` signs the ZIP/POM and uploads a bundle to the Central Portal. The working directory must contain `CCGO.toml`; the version comes from its project configuration unless overridden with `--version`. The default artifactId is `<project>-doc` (`--artifact-id` overrides it). Set the group with `--artifact-group` or `[publish.doc.maven].group_id`.

`--path` accepts a MkDocs project directory, config file or project ZIP containing a unique MkDocs config. Temporary files are cleaned up. This mode rejects `--skip-build`, `--doc-open`, `--doc-force` and the GitHub artifact provider. It uploads ZIP, POM and SHA-256 files; partial uploads are not rolled back. For online browsing, extract and deploy the archive to a static web host.
