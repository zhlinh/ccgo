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

    # Build each platform using ccgo build <platform> --native-only
    for platform_name in platforms_to_build:
        print(f"\n--- Building {platform_name} native libraries ---\n")

        try:
            # Use ccgo command to build native libraries
            cmd = ["ccgo", "build", platform_name, "--native-only"]

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
    """Build the KMP library for all platforms"""
    print("\n" + "=" * 80)
    print("Building Kotlin Multiplatform Library")
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

    tasks = [
        "assembleDebug",  # Android debug
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


def publish_to_maven_local():
    """Publish the KMP library to local Maven repository"""
    print("\n" + "=" * 80)
    print("Publishing KMP to Maven Local")
    print("=" * 80 + "\n")

    gradlew = KMP_DIR / "gradlew"

    cmd = [str(gradlew), "publishToMavenLocal"]
    run_command(cmd, cwd=KMP_DIR)

    print("\n" + "=" * 80)
    print("Published to Maven Local successfully!")
    print("=" * 80 + "\n")

    print("\nMaven Local artifacts can be found at:")
    print(f"  ~/.m2/repository/")
    print()


def publish_to_maven_remote():
    """Publish the KMP library to remote Maven repository"""
    print("\n" + "=" * 80)
    print("Publishing KMP to Maven Remote")
    print("=" * 80 + "\n")

    gradlew = KMP_DIR / "gradlew"

    # Check if Maven credentials are configured
    gradle_props = KMP_DIR / "gradle.properties"
    has_credentials = False

    if gradle_props.exists():
        with open(gradle_props, "r") as f:
            content = f.read()
            has_credentials = (
                "maven.username" in content and "maven.password" in content
            )

    if not has_credentials:
        print("⚠️  WARNING: Maven credentials not found in gradle.properties")
        print("\nPlease add the following to kmp/gradle.properties:")
        print("  maven.username=your-username")
        print("  maven.password=your-password")
        print("\nOr configure them in ~/.gradle/gradle.properties\n")

        response = input("Continue anyway? (y/N): ")
        if response.lower() != "y":
            print("Aborted.")
            sys.exit(0)

    cmd = [str(gradlew), "publish"]
    run_command(cmd, cwd=KMP_DIR)

    print("\n" + "=" * 80)
    print("Published to Maven Remote successfully!")
    print("=" * 80 + "\n")


def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(description="Build Kotlin Multiplatform Library")
    parser.add_argument(
        "mode",
        nargs="?",
        type=int,
        default=1,
        choices=[1, 2, 3],
        help="Build mode: 1=build (default), 2=publish to Maven local, 3=publish to Maven remote",
    )

    args = parser.parse_args()

    if args.mode == 1:
        build_kmp_library()
    elif args.mode == 2:
        publish_to_maven_local()
    elif args.mode == 3:
        publish_to_maven_remote()


if __name__ == "__main__":
    main()
