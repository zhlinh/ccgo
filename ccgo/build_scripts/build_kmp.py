#!/usr/bin/env python3
# -*- coding: utf-8 -*-
#
# build_kmp.py
# ccgo
#
# Copyright 2024 zhlinh and ccgo Project Authors. All rights reserved.
# Use of this source code is governed by a MIT-style
# license that can be found at
#
# https://opensource.org/license/MIT
#
# The above copyright notice and this permission
# notice shall be included in all copies or
# substantial portions of the Software.

"""
Kotlin Multiplatform Library build script.

This script builds the KMP library for all supported platforms:
- Android (JNI)
- iOS (cinterop)
- macOS (cinterop)
- Linux (cinterop)
- Desktop/Windows (JVM)

Usage:
    python3 build_kmp.py <mode>

    mode: 1 = build library (default)
          2 = publish to Maven local
          3 = publish to Maven remote

Requirements:
- Gradle 7.0+ (included via gradlew)
- Android SDK (for Android target)
- Xcode (for iOS/macOS targets, macOS only)
- Java 17+
"""

import os
import sys
import subprocess
import argparse
import platform
import shutil
import zipfile
from pathlib import Path

# Get the script directory (where this build script is located)
SCRIPT_DIR = Path(__file__).parent.absolute()

# Get project directory (current working directory when ccgo build kmp is called)
try:
    PROJECT_DIR = Path(os.getcwd()).absolute()
except (OSError, FileNotFoundError) as e:
    PROJECT_DIR = Path(os.environ.get("PWD", ".")).absolute()

# KMP directory is in project/kmp
KMP_DIR = PROJECT_DIR / "kmp"

# Import build_utils for project configuration
# build_utils reads PROJECT_NAME from CCGO.toml in the current working directory
try:
    from build_utils import (
        PROJECT_NAME,
        PROJECT_NAME_LOWER,
        get_archive_version_info,
        get_target_subdir,
        print_zip_tree,
        generate_build_info,
    )
    _HAS_BUILD_UTILS = True
except ImportError:
    # Fallback if build_utils not available
    _HAS_BUILD_UTILS = False
    PROJECT_NAME = None
    PROJECT_NAME_LOWER = None
    get_archive_version_info = None
    get_target_subdir = None
    print_zip_tree = None
    generate_build_info = None


def get_project_name_from_toml():
    """Read project name from CCGO.toml file."""
    try:
        import tomllib
    except ImportError:
        try:
            import tomli as tomllib
        except ImportError:
            return None, None

    ccgo_toml = PROJECT_DIR / "CCGO.toml"
    if not ccgo_toml.exists():
        return None, None

    try:
        with open(ccgo_toml, "rb") as f:
            data = tomllib.load(f)
        project_name_lower = data.get("project", {}).get("name", "sdk")
        project_name = project_name_lower.upper()
        return project_name, project_name_lower
    except Exception:
        return None, None


# Get project name - prefer build_utils, fallback to direct TOML reading
if not _HAS_BUILD_UTILS or PROJECT_NAME is None:
    _name, _name_lower = get_project_name_from_toml()
    if _name:
        PROJECT_NAME = _name
        PROJECT_NAME_LOWER = _name_lower
    else:
        PROJECT_NAME = "SDK"
        PROJECT_NAME_LOWER = "sdk"


def run_command(cmd, cwd=None, env=None):
    """Execute a shell command and return the result"""
    print(f"\n{'='*80}")
    print(f"Executing: {' '.join(cmd)}")
    print(f"Working directory: {cwd or os.getcwd()}")
    print(f"{'='*80}\n")

    result = subprocess.run(
        cmd, cwd=cwd or os.getcwd(), env=env, capture_output=False, text=True
    )

    if result.returncode != 0:
        print(f"\n{'!'*80}")
        print(f"ERROR: Command failed with exit code {result.returncode}")
        print(f"{'!'*80}\n")
        sys.exit(result.returncode)

    return result


def build_native_libraries():
    """
    Build native libraries for KMP.

    This function automatically builds native libraries for the current platform:
    - Android (always built)
    - iOS (on macOS)
    - macOS (on macOS)
    - Linux (on Linux)
    - Windows (on Windows)

    Uses 'ccgo build <platform> --native-only' to build only native libraries.
    """
    print("\n" + "=" * 80)
    print("Building Native Libraries for KMP")
    print("=" * 80 + "\n")

    system = platform.system()

    # Determine which platforms to build
    platforms_to_build = []

    # Always build Android (cross-platform)
    platforms_to_build.append("android")

    if system == "Darwin":
        # On macOS, build iOS and macOS
        platforms_to_build.extend(["ios", "macos"])
    elif system == "Linux":
        # On Linux, build Linux
        platforms_to_build.append("linux")
    elif system == "Windows":
        # On Windows, build Windows
        platforms_to_build.append("windows")

    # Build each platform using ccgo build <platform> --native-only --no-archive
    # The --no-archive flag prevents overwriting platform ZIPs when building as part of ccgo build all
    for platform_name in platforms_to_build:
        print(f"\n--- Building {platform_name} native libraries ---\n")

        try:
            # Use ccgo command to build native libraries
            # --no-archive skips ZIP creation to avoid overwriting existing platform archives
            cmd = ["ccgo", "build", platform_name, "--native-only", "--no-archive"]

            print(f"Executing: {' '.join(cmd)}\n")

            result = subprocess.run(
                cmd, cwd=PROJECT_DIR, capture_output=False, text=True
            )

            if result.returncode != 0:
                print(
                    f"\n⚠️  WARNING: {platform_name} build failed with exit code {result.returncode}"
                )
                print(f"   KMP may not work correctly on {platform_name}.\n")
                # Don't exit, continue with other platforms
            else:
                print(f"\n✅ {platform_name} native libraries built successfully.\n")

        except Exception as e:
            print(f"\n⚠️  WARNING: Failed to build {platform_name}: {e}")
            print(f"   KMP may not work correctly on {platform_name}.\n")

    print("\n" + "=" * 80)
    print("Native Libraries Build Complete")
    print("=" * 80 + "\n")


def build_kmp_library():
    """Build the KMP library for all platforms (release variant only)"""
    print("\n" + "=" * 80)
    print("Building Kotlin Multiplatform Library")
    print("Build variant: RELEASE")
    print("=" * 80 + "\n")

    # Check if kmp directory exists
    if not KMP_DIR.exists():
        print(f"ERROR: KMP directory not found: {KMP_DIR}")
        print("Please ensure your project has the KMP module configured")
        sys.exit(1)

    # Build native libraries first
    build_native_libraries()

    # Make gradlew executable
    gradlew = KMP_DIR / "gradlew"
    if gradlew.exists():
        os.chmod(gradlew, 0o755)

    # Build all targets
    print("\n--- Building all KMP targets ---\n")

    # Build release variant only
    tasks = [
        "assembleRelease",  # Android release
        "desktopJar",  # Desktop JVM target
    ]

    # Detect platform and add platform-specific targets
    system = platform.system()

    if system == "Darwin":  # macOS
        tasks.extend(
            [
                "iosArm64MainKlibrary",
                "iosX64MainKlibrary",
                "iosSimulatorArm64MainKlibrary",
                "macosArm64MainKlibrary",
                "macosX64MainKlibrary",
            ]
        )
    elif system == "Linux":
        tasks.extend(
            [
                "linuxX64MainKlibrary",
                "linuxArm64MainKlibrary",
            ]
        )

    # Run build
    cmd = [str(gradlew), "clean"] + tasks
    run_command(cmd, cwd=KMP_DIR)

    print("\n" + "=" * 80)
    print("✅ KMP Library built successfully!")
    print("=" * 80 + "\n")

    # Show output locations
    print("\nBuild outputs:")
    print(f"  - Android AAR: {KMP_DIR}/build/outputs/aar/")
    print(f"  - Desktop JAR: {KMP_DIR}/build/libs/")

    # List actual files in key directories
    # Check Android AAR
    aar_dir = KMP_DIR / "build" / "outputs" / "aar"
    if aar_dir.exists():
        aar_files = list(aar_dir.glob("*.aar"))
        if aar_files:
            print(f"\n  Android AAR files:")
            for f in aar_files:
                print(f"    - {f.name}")

    # Check Desktop JAR
    jar_dir = KMP_DIR / "build" / "libs"
    if jar_dir.exists():
        jar_files = list(jar_dir.glob("*.jar"))
        if jar_files:
            print(f"\n  Desktop JAR files:")
            for f in jar_files:
                print(f"    - {f.name}")

    # Check for klib files
    if system == "Darwin":
        classes_dir = KMP_DIR / "build" / "classes" / "kotlin"
        if classes_dir.exists():
            klib_files = list(classes_dir.glob("**/*.klib"))
            if klib_files:
                print(f"\n  Native klib files (iOS/macOS):")
                for f in klib_files:
                    print(f"    - {f.relative_to(classes_dir)}")

        # Check cinterop outputs
        cinterop_dirs = (
            list(classes_dir.glob("*/main/cinterop")) if classes_dir.exists() else []
        )
        if cinterop_dirs:
            print(f"\n  Cinterop outputs:")
            for d in cinterop_dirs:
                print(f"    - {d.relative_to(classes_dir)}")

    elif system == "Linux":
        classes_dir = KMP_DIR / "build" / "classes" / "kotlin"
        if classes_dir.exists():
            klib_files = list(classes_dir.glob("**/*.klib"))
            if klib_files:
                print(f"\n  Native klib files (Linux):")
                for f in klib_files:
                    print(f"    - {f.relative_to(classes_dir)}")

    print(f"\n  Tip: For publishable artifacts, use:")
    print(f"  ccgo publish kmp  # Publish to Maven")
    print()

    # Create unified ZIP archive in target/{debug|release}/kmp directory
    print("\n" + "=" * 80)
    print("Creating Unified ZIP Archive")
    print("=" * 80 + "\n")

    # Determine target subdirectory based on build mode
    target_subdir = get_target_subdir() if get_target_subdir else "debug"
    target_kmp_dir = PROJECT_DIR / "target" / target_subdir / "kmp"

    # Clean and create target/{debug|release}/kmp directory
    if target_kmp_dir.exists():
        print(f"Cleaning existing target/{target_subdir}/kmp directory...")
        shutil.rmtree(target_kmp_dir)

    target_kmp_dir.mkdir(parents=True, exist_ok=True)

    # Get version info for ZIP naming
    version_str = "1.0.0"
    if get_archive_version_info:
        try:
            _, _, full_version = get_archive_version_info(str(PROJECT_DIR))
            version_str = full_version
        except Exception:
            pass

    project_upper = PROJECT_NAME.upper()

    # Create single unified ZIP containing all platforms
    # Naming convention: {PROJECT}_KMP_SDK-{version}.zip
    zip_name = f"{project_upper}_KMP_SDK-{version_str}.zip"
    zip_path = target_kmp_dir / zip_name
    files_added = 0

    with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as zf:
        # Add Android AAR files to lib/kmp/android/
        if aar_dir.exists():
            aar_files = list(aar_dir.glob("*.aar"))
            for aar_file in aar_files:
                zf.write(aar_file, f"lib/kmp/android/{aar_file.name}")
                files_added += 1
                print(f"  + lib/kmp/android/{aar_file.name}")

        # Add Desktop JAR files to lib/kmp/desktop/
        if jar_dir.exists():
            jar_files = list(jar_dir.glob("*.jar"))
            for jar_file in jar_files:
                zf.write(jar_file, f"lib/kmp/desktop/{jar_file.name}")
                files_added += 1
                print(f"  + lib/kmp/desktop/{jar_file.name}")

        # Add Native klib files (iOS/macOS/Linux) to lib/kmp/native/
        classes_dir = KMP_DIR / "build" / "classes" / "kotlin"
        if classes_dir.exists():
            for klib_dir in classes_dir.glob("*/main"):
                if klib_dir.is_dir():
                    platform_name = klib_dir.parent.name  # e.g., iosArm64, macosX64, etc.

                    # Add klib directory
                    if (klib_dir / "klib").exists():
                        klib_path = klib_dir / "klib"
                        for root, dirs, files in os.walk(klib_path):
                            for file in files:
                                file_path = Path(root) / file
                                arcname = f"lib/kmp/native/{platform_name}/klib/{file_path.relative_to(klib_path)}"
                                zf.write(file_path, arcname)
                                files_added += 1
                        print(f"  + lib/kmp/native/{platform_name}/klib/")

                    # Add cinterop directory
                    if (klib_dir / "cinterop").exists():
                        cinterop_path = klib_dir / "cinterop"
                        for root, dirs, files in os.walk(cinterop_path):
                            for file in files:
                                file_path = Path(root) / file
                                arcname = f"lib/kmp/native/{platform_name}/cinterop/{file_path.relative_to(cinterop_path)}"
                                zf.write(file_path, arcname)
                                files_added += 1
                        print(f"  + lib/kmp/native/{platform_name}/cinterop/")

        # Generate and add build_info.json to meta/kmp/ directory
        if generate_build_info:
            import json
            build_info = generate_build_info(
                project_name=PROJECT_NAME_LOWER or PROJECT_NAME.lower(),
                target_platform="kmp",
                version=version_str,
                link_type="both",
                extra_info={"build_system": "gradle"}
            )
            build_info_json = json.dumps(build_info, indent=2)
            zf.writestr("meta/kmp/build_info.json", build_info_json)
            files_added += 1
            print(f"  + meta/kmp/build_info.json")

            # Also write build_info.json to target/kmp/
            build_info_path = target_kmp_dir / "build_info.json"
            with open(build_info_path, 'w') as f:
                f.write(build_info_json)

    if files_added > 0:
        # Add archive_info.json to meta/kmp/ directory inside ZIP
        try:
            from build_utils import _add_archive_info_to_zip
            _add_archive_info_to_zip(str(zip_path), "kmp")
        except ImportError:
            pass

        size_mb = zip_path.stat().st_size / (1024 * 1024)
        print(f"\n" + "=" * 60)
        print(f"Build artifacts in target/{target_subdir}/kmp/:")
        print("-" * 60)
        print(f"  {zip_name} ({size_mb:.2f} MB)")

        # Print ZIP tree structure and generate archive_info.json to target directory
        if print_zip_tree:
            print_zip_tree(str(zip_path), indent="    ", generate_info_file=True)

        print("-" * 60)
        print(f"\nKMP artifacts archived in: {zip_path}")
    else:
        print("⚠️  No files were added to the ZIP archive")
        # Remove empty ZIP
        if zip_path.exists():
            zip_path.unlink()

    print()


def publish_to_maven_local():
    """Publish the KMP library to local Maven repository"""
    print("\n" + "=" * 80)
    print("Publishing KMP to Maven Local")
    print("=" * 80 + "\n")

    gradlew = KMP_DIR / "gradlew"

    # Use ccgoPublishToMavenLocal task from ccgo-gradle-plugins
    # --no-configuration-cache avoids Android SDK cache issues
    cmd = [str(gradlew), "ccgoPublishToMavenLocal", "--no-configuration-cache"]
    run_command(cmd, cwd=KMP_DIR)

    # Success message is printed by the Gradle plugin


def _check_maven_credentials():
    """
    Check if Maven credentials are configured.

    Credential sources (priority from high to low):
    1. Environment variables (MAVEN_CENTRAL_USERNAME, MAVEN_CUSTOM_URLS, etc.)
    2. CCGO.toml (publish.maven.*)
    3. Project-level gradle.properties
    4. User-level ~/.gradle/gradle.properties

    Returns:
        Tuple of (has_credentials, source_description)
    """
    # 1. Check environment variables
    env_central = os.environ.get('MAVEN_CENTRAL_USERNAME') and os.environ.get('MAVEN_CENTRAL_PASSWORD')
    env_custom = os.environ.get('MAVEN_CUSTOM_URLS') and os.environ.get('MAVEN_CUSTOM_USERNAMES')
    if env_central or env_custom:
        return True, "environment variables"

    # 2. Check CCGO.toml
    ccgo_toml = PROJECT_DIR / "CCGO.toml"
    if ccgo_toml.exists():
        try:
            import tomllib
        except ImportError:
            try:
                import tomli as tomllib
            except ImportError:
                tomllib = None

        if tomllib:
            with open(ccgo_toml, "rb") as f:
                config = tomllib.load(f)
                maven_config = config.get('publish', {}).get('maven', {})
                if maven_config.get('central_username') and maven_config.get('central_password'):
                    return True, "CCGO.toml"
                if maven_config.get('custom_urls') and maven_config.get('custom_usernames'):
                    return True, "CCGO.toml"

    # 3. Check project-level gradle.properties (KMP dir or project dir)
    for props_file in [KMP_DIR / "gradle.properties", PROJECT_DIR / "gradle.properties"]:
        if props_file.exists():
            with open(props_file, "r") as f:
                content = f.read()
                if "mavenCentralUsername" in content and "mavenCentralPassword" in content:
                    return True, f"project gradle.properties ({props_file})"
                if "mavenCustomUrls" in content and "mavenCustomUsernames" in content:
                    return True, f"project gradle.properties ({props_file})"

    # 4. Check user-level ~/.gradle/gradle.properties
    user_gradle_props = Path.home() / ".gradle" / "gradle.properties"
    if user_gradle_props.exists():
        with open(user_gradle_props, "r") as f:
            content = f.read()
            if "mavenCentralUsername" in content and "mavenCentralPassword" in content:
                return True, "~/.gradle/gradle.properties"
            if "mavenCustomUrls" in content and "mavenCustomUsernames" in content:
                return True, "~/.gradle/gradle.properties"

    return False, ""


def publish_to_maven_central():
    """Publish the KMP library to Maven Central (Sonatype OSSRH)"""
    print("\n" + "=" * 80)
    print("Publishing KMP to Maven Central")
    print("=" * 80 + "\n")

    gradlew = KMP_DIR / "gradlew"

    # Use ccgoPublishToMavenCentral task from ccgo-gradle-plugins
    # --no-configuration-cache avoids Android SDK cache issues
    cmd = [str(gradlew), "ccgoPublishToMavenCentral", "--no-configuration-cache"]
    run_command(cmd, cwd=KMP_DIR)

    # Success message is printed by the Gradle plugin


def publish_to_maven_custom():
    """Publish the KMP library to custom Maven repository"""
    print("\n" + "=" * 80)
    print("Publishing KMP to Custom Maven Repository")
    print("=" * 80 + "\n")

    gradlew = KMP_DIR / "gradlew"

    # Use ccgoPublishToMavenCustom task from ccgo-gradle-plugins
    # --no-configuration-cache avoids Android SDK cache issues
    cmd = [str(gradlew), "ccgoPublishToMavenCustom", "--no-configuration-cache"]
    run_command(cmd, cwd=KMP_DIR)

    # Success message is printed by the Gradle plugin


def main(publish_local=False, publish_central=False, publish_custom=False):
    """
    Main entry point for building and publishing KMP library.

    Args:
        publish_local: If True, publish to Maven local repository
        publish_central: If True, publish to Maven Central (Sonatype OSSRH)
        publish_custom: If True, publish to custom Maven repository
    """
    if publish_local:
        publish_to_maven_local()
    elif publish_central:
        publish_to_maven_central()
    elif publish_custom:
        publish_to_maven_custom()
    else:
        build_kmp_library()


# Command-line interface for KMP builds
#
# Usage:
#   python build_kmp.py                   # Build KMP library (release)
#   python build_kmp.py --publish-local   # Publish to Maven local
#   python build_kmp.py --publish-central # Publish to Maven Central
#   python build_kmp.py --publish-custom  # Publish to custom Maven repository
if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Build Kotlin Multiplatform Library (release variant only)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--publish-local",
        action="store_true",
        help="Publish to Maven local repository (~/.m2/repository/)",
    )
    parser.add_argument(
        "--publish-central",
        action="store_true",
        help="Publish to Maven Central (Sonatype OSSRH)",
    )
    parser.add_argument(
        "--publish-custom",
        action="store_true",
        help="Publish to custom Maven repository",
    )

    args = parser.parse_args()
    main(
        publish_local=args.publish_local,
        publish_central=args.publish_central,
        publish_custom=args.publish_custom
    )
