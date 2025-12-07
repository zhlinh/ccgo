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

import os
import sys
import argparse
import subprocess
import importlib.util
import platform
import time
import multiprocessing
import shutil
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

# setup path
# >>>>>>>>>>>>>>
SCRIPT_PATH = os.path.split(os.path.realpath(__file__))[0]
PROJECT_ROOT_PATH = os.path.dirname(SCRIPT_PATH)
sys.path.append(SCRIPT_PATH)
sys.path.append(PROJECT_ROOT_PATH)
PACKAGE_NAME = os.path.basename(SCRIPT_PATH)
# <<<<<<<<<<<<<
# import this project modules
try:
    from ccgo.utils.context.namespace import CliNameSpace
    from ccgo.utils.context.context import CliContext
    from ccgo.utils.context.command import CliCommand
    from ccgo.utils.cmd.cmd_util import exec_command
except ImportError:
    from utils.context.namespace import CliNameSpace
    from utils.context.context import CliContext
    from utils.context.command import CliCommand
    from utils.cmd.cmd_util import exec_command


class Build(CliCommand):
    def _is_cross_platform_build(self, host_platform: str, target_platform: str) -> bool:
        """
        Check if building target_platform on host_platform requires cross-compilation.

        Args:
            host_platform: Current platform ('Darwin', 'Linux', 'Windows')
            target_platform: Target build platform (e.g., 'linux', 'windows', 'macos', 'ios', etc.)

        Returns:
            True if cross-platform build (requires Docker), False if native build
        """
        # Normalize host platform
        host_map = {
            'Darwin': 'macos',
            'Linux': 'linux',
            'Windows': 'windows'
        }
        host = host_map.get(host_platform, host_platform.lower())

        # Platforms that don't require specific toolchains (can build anywhere)
        universal_targets = ['kmp', 'include']
        if target_platform in universal_targets:
            return False

        # Define native build capabilities for each host platform
        native_builds = {
            'macos': ['macos', 'ios', 'watchos', 'tvos'],  # Requires Xcode
            'linux': ['linux'],  # Requires GCC/Clang
            'windows': ['windows'],  # Requires Visual Studio/MinGW
        }

        # Android and OHOS can be built on any platform with their respective SDKs
        # but we don't auto-enable Docker for them as users likely have SDK installed
        if target_platform in ['android', 'ohos']:
            return False

        # Check if target can be built natively on current host
        return target_platform not in native_builds.get(host, [])

    def description(self) -> str:
        return """Build library for specific platform.

This command builds native libraries and packages them for the target platform.

SUPPORTED PLATFORMS:
    all         Build for all platforms (android, ios, macos, watchos, tvos, windows, linux, ohos, kmp, conan)
    android     Build for Android (AAR package with native .so libraries)
    ios         Build for iOS (static libraries and frameworks)
    watchos     Build for watchOS (static libraries and frameworks)
    tvos        Build for tvOS (static libraries and frameworks)
    macos       Build for macOS (static libraries and frameworks)
    windows     Build for Windows (.lib and .dll libraries)
    linux       Build for Linux (static and shared libraries)
    ohos        Build for OpenHarmony (HAR package with native .so libraries)
    kmp         Build Kotlin Multiplatform library (all supported targets)
    conan       Build Conan package for C/C++ dependency management
    include     Build and package header files only

EXAMPLES:
    # Build all platforms (parallel by default, using CPU count workers)
    ccgo build all

    # Build all platforms with 4 parallel workers
    ccgo build all -j 4

    # Build all platforms sequentially (no parallelism)
    ccgo build all --no-parallel

    # Build Android with default architectures (armeabi-v7a, arm64-v8a, x86_64)
    ccgo build android

    # Build Android with specific architectures
    ccgo build android --arch arm64-v8a,x86_64

    # Build only native libraries without packaging
    ccgo build android --native-only

    # Build iOS with Xcode project generation
    ccgo build ios --ide-project

    # Build watchOS static library and frameworks
    ccgo build watchos

    # Build tvOS static library and frameworks
    ccgo build tvos

    # Build KMP library for all platforms
    ccgo build kmp

    # Build OHOS with specific architectures
    ccgo build ohos --arch arm64-v8a

    # Cross-platform builds using Docker (build any platform on any OS)
    ccgo build linux --docker
    ccgo build windows --docker
    ccgo build macos --docker
    ccgo build ios --docker
    ccgo build watchos --docker
    ccgo build tvos --docker
    ccgo build android --docker

PLATFORM-SPECIFIC OPTIONS:
    Android/OHOS:
        --arch              Comma-separated architectures
                           (armeabi-v7a, arm64-v8a, x86_64)
        --native-only      Build only .so libraries (skip AAR/HAR packaging)

    All platforms (except OHOS/KMP):
        --docker           Build using Docker containers (enables cross-platform builds)
                          Allows building any platform on any OS without local toolchains

    All native platforms:
        --link-type        Library type to build: static, shared, or both (default: both)
        --ide-project      Generate IDE project files for development

    Build all:
        -j, --jobs N       Number of parallel build jobs (default: CPU count)
        --no-parallel      Disable parallel builds (equivalent to --jobs=1)

REQUIREMENTS:
    Native builds (without --docker):
        Android:            ANDROID_HOME, ANDROID_NDK_HOME, JAVA_HOME
        iOS/watchOS/tvOS:   Xcode and command-line tools
        macOS:              Xcode and command-line tools
        OHOS:               OHOS_SDK_HOME or HOS_SDK_HOME
        Windows:            Visual Studio or MinGW
        Linux:              GCC or Clang

    Docker builds (with --docker):
        All platforms:      Docker Desktop installed and running (~8GB disk space)
        """

    def get_target_list(self) -> list:
        return ["all", "android", "ios", "watchos", "tvos", "windows", "linux", "macos", "ohos", "kmp", "conan", "include"]

    def get_build_platforms(self) -> list:
        """Get list of actual build platforms (excluding meta-targets like 'all')"""
        return ["android", "ios", "watchos", "tvos", "windows", "linux", "macos", "ohos", "kmp", "conan"]

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo build",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )
        parser.add_argument(
            "target",
            metavar=f"{self.get_target_list()}",
            type=str,
            choices=self.get_target_list(),
        )
        parser.add_argument(
            "--ide-project",
            action="store",
            help="generate ide project",
        )
        parser.add_argument(
            "--arch",
            action="store",
            default="armeabi-v7a,arm64-v8a,x86_64",
            help="arch like armeabi-v7a,arm64-v8a,x86_64, etc, if choose more than one, use ',' to split them.",
        )
        parser.add_argument(
            "--native-only",
            action="store_true",
            help="only build native libraries (e.g., .so, .framework) without additional packaging",
        )
        parser.add_argument(
            "--docker",
            action="store_true",
            help="build using Docker containers (enables cross-platform builds for Linux/Windows)",
        )
        parser.add_argument(
            "--no-docker",
            action="store_true",
            help="disable automatic Docker mode for cross-platform builds (use native toolchain)",
        )
        parser.add_argument(
            "--docker-dev",
            action="store_true",
            help="use local ccgo source in Docker (for development, requires local ccgo repo)",
        )
        parser.add_argument(
            "--toolchain",
            choices=["msvc", "gnu", "mingw", "auto"],
            default="auto",
            help="Windows toolchain: msvc (Visual Studio), gnu/mingw (MinGW-w64), auto (detect)",
        )
        parser.add_argument(
            "--link-type",
            choices=["static", "shared", "both"],
            default="both",
            help="Library link type: static (only .a/.lib), shared (only .so/.dll/.dylib), both (default)",
        )
        parser.add_argument(
            "-j", "--jobs",
            type=int,
            default=None,
            help="Number of parallel build jobs for 'all' target (default: CPU count). Use 1 to disable parallel builds.",
        )
        parser.add_argument(
            "--no-parallel",
            action="store_true",
            help="Disable parallel builds for 'all' target (equivalent to --jobs=1)",
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def _print_build_time(self, start_time: float):
        """Print the build time in a human-readable format."""
        elapsed = time.time() - start_time
        if elapsed < 60:
            print(f"\n⏱ Build completed in {elapsed:.2f} seconds")
        elif elapsed < 3600:
            minutes = int(elapsed // 60)
            seconds = elapsed % 60
            print(f"\n⏱ Build completed in {minutes} min {seconds:.1f} sec")
        else:
            hours = int(elapsed // 3600)
            minutes = int((elapsed % 3600) // 60)
            seconds = elapsed % 60
            print(f"\n⏱ Build completed in {hours} hr {minutes} min {seconds:.0f} sec")

    def _build_platform_subprocess(self, target_platform: str, args: CliNameSpace, project_subdir: str) -> tuple:
        """
        Build a single platform in a subprocess.
        Returns (target_platform, success, error_message, elapsed_time)
        """
        import subprocess
        import shlex

        platform_start = time.time()

        # Build command line arguments
        cmd_args = ["ccgo", "build", target_platform]

        # Pass through relevant arguments
        if hasattr(args, 'arch') and args.arch:
            cmd_args.extend(["--arch", args.arch])
        if hasattr(args, 'native_only') and args.native_only:
            cmd_args.append("--native-only")
        if hasattr(args, 'docker') and args.docker:
            cmd_args.append("--docker")
        if hasattr(args, 'no_docker') and args.no_docker:
            cmd_args.append("--no-docker")
        if hasattr(args, 'docker_dev') and args.docker_dev:
            cmd_args.append("--docker-dev")
        if hasattr(args, 'toolchain') and args.toolchain and args.toolchain != "auto":
            cmd_args.extend(["--toolchain", args.toolchain])
        if hasattr(args, 'link_type') and args.link_type:
            cmd_args.extend(["--link-type", args.link_type])
        if hasattr(args, 'ide_project') and args.ide_project:
            cmd_args.extend(["--ide-project", args.ide_project])
        # Always pass -j to ensure build scripts use argparse mode (not legacy interactive mode)
        effective_jobs = args.jobs if hasattr(args, 'jobs') and args.jobs else multiprocessing.cpu_count()
        cmd_args.extend(["-j", str(effective_jobs)])

        try:
            result = subprocess.run(
                cmd_args,
                cwd=project_subdir,
                capture_output=True,
                text=True,
                timeout=3600  # 1 hour timeout per platform
            )

            elapsed = time.time() - platform_start

            if result.returncode == 0:
                return (target_platform, True, None, elapsed, result.stdout, result.stderr)
            else:
                error_msg = result.stderr or result.stdout or f"Exit code: {result.returncode}"
                return (target_platform, False, error_msg, elapsed, result.stdout, result.stderr)

        except subprocess.TimeoutExpired:
            elapsed = time.time() - platform_start
            return (target_platform, False, "Build timed out after 1 hour", elapsed, "", "")
        except Exception as e:
            elapsed = time.time() - platform_start
            return (target_platform, False, str(e), elapsed, "", "")

    def _format_elapsed_time(self, elapsed: float) -> str:
        """Format elapsed time in a human-readable format."""
        if elapsed < 60:
            return f"{elapsed:.1f}s"
        elif elapsed < 3600:
            minutes = int(elapsed // 60)
            seconds = elapsed % 60
            return f"{minutes}m {seconds:.0f}s"
        else:
            hours = int(elapsed // 3600)
            minutes = int((elapsed % 3600) // 60)
            return f"{hours}h {minutes}m"

    def exec(self, context: CliContext, args: CliNameSpace):
        # Record start time
        start_time = time.time()

        # Handle 'all' target - build all platforms
        if args.target == "all":
            print("="*80)
            print("Building library for ALL platforms")
            print("="*80)

            # Determine number of parallel jobs
            if args.no_parallel:
                num_jobs = 1
            elif args.jobs is not None:
                num_jobs = max(1, args.jobs)
            else:
                # Default to CPU count
                num_jobs = multiprocessing.cpu_count()

            platforms = self.get_build_platforms()
            total_platforms = len(platforms)
            failed_platforms = []
            successful_platforms = []
            platform_times = {}

            # Get project directory for subprocess builds
            try:
                project_dir = os.getcwd()
            except (OSError, FileNotFoundError) as e:
                project_dir = os.environ.get('PWD')
                if not project_dir or not os.path.exists(project_dir):
                    print(f"ERROR: Current working directory no longer exists: {e}")
                    self._print_build_time(start_time)
                    sys.exit(1)

            # Find CCGO.toml to determine project_subdir
            config_path = None
            project_subdir = project_dir
            for subdir in os.listdir(project_dir):
                potential_config = os.path.join(project_dir, subdir, "CCGO.toml")
                if os.path.isfile(potential_config):
                    config_path = potential_config
                    project_subdir = os.path.join(project_dir, subdir)
                    break
            if not config_path and os.path.isfile(os.path.join(project_dir, "CCGO.toml")):
                config_path = os.path.join(project_dir, "CCGO.toml")
                project_subdir = project_dir

            if not config_path:
                print("❌ ERROR: CCGO.toml not found in project directory")
                self._print_build_time(start_time)
                sys.exit(1)

            print(f"\nWill build {total_platforms} platforms: {', '.join(platforms)}")
            print(f"Parallel jobs: {num_jobs}" + (" (sequential)" if num_jobs == 1 else f" (parallel)"))
            print("="*80)

            if num_jobs == 1:
                # Sequential build (original behavior)
                for index, target_platform in enumerate(platforms, 1):
                    print(f"\n{'='*80}")
                    print(f"Building platform {index}/{total_platforms}: {target_platform.upper()}")
                    print(f"{'='*80}\n")

                    # Create a copy of args with the specific platform
                    platform_args = argparse.Namespace(**vars(args))
                    platform_args.target = target_platform

                    # Build the platform
                    platform_start = time.time()
                    try:
                        # Call exec recursively with the specific platform
                        self.exec(context, platform_args)
                        successful_platforms.append(target_platform)
                        elapsed = time.time() - platform_start
                        platform_times[target_platform] = elapsed
                        print(f"\n✅ {target_platform.upper()} build completed successfully ({self._format_elapsed_time(elapsed)})")
                    except SystemExit as e:
                        elapsed = time.time() - platform_start
                        platform_times[target_platform] = elapsed
                        if e.code != 0:
                            failed_platforms.append(target_platform)
                            print(f"\n❌ {target_platform.upper()} build failed with exit code {e.code} ({self._format_elapsed_time(elapsed)})")
                        else:
                            successful_platforms.append(target_platform)
                            print(f"\n✅ {target_platform.upper()} build completed successfully ({self._format_elapsed_time(elapsed)})")
                    except KeyboardInterrupt:
                        elapsed = time.time() - platform_start
                        platform_times[target_platform] = elapsed
                        print(f"\n⚠️  {target_platform.upper()} build interrupted by user ({self._format_elapsed_time(elapsed)})")
                        failed_platforms.append(target_platform)
                        print(f"\nSkipping {target_platform}, continuing with next platform...")
                        print("(Press Ctrl+C again within 2 seconds to abort all builds)")
                        try:
                            time.sleep(2)
                        except KeyboardInterrupt:
                            print("\n\n🛑 Build aborted by user")
                            break
                    except Exception as e:
                        elapsed = time.time() - platform_start
                        platform_times[target_platform] = elapsed
                        failed_platforms.append(target_platform)
                        print(f"\n❌ {target_platform.upper()} build failed with error: {e} ({self._format_elapsed_time(elapsed)})")
            else:
                # Parallel build using ThreadPoolExecutor
                # Note: Apple platforms (ios, macos, watchos, tvos) share Xcode toolchain
                # and may conflict when built in parallel, so we group them together
                apple_platforms = {'ios', 'macos', 'watchos', 'tvos'}
                apple_to_build = [p for p in platforms if p in apple_platforms]
                other_platforms = [p for p in platforms if p not in apple_platforms]

                print(f"\n🚀 Starting parallel build with {num_jobs} workers...")
                if apple_to_build:
                    print(f"   ℹ Apple platforms ({', '.join(apple_to_build)}) will be built sequentially to avoid Xcode conflicts")
                print()

                def build_apple_sequential():
                    """Build all Apple platforms sequentially and return results."""
                    results = []
                    for plat in apple_to_build:
                        result = self._build_platform_subprocess(plat, args, project_subdir)
                        results.append(result)
                    return results

                # Track in-progress builds (initialized outside try for KeyboardInterrupt handler)
                in_progress = set(platforms)
                completed_count = 0

                try:
                    with ThreadPoolExecutor(max_workers=num_jobs) as executor:
                        # Submit non-Apple platforms individually
                        futures = {}
                        for target_platform in other_platforms:
                            future = executor.submit(
                                self._build_platform_subprocess,
                                target_platform,
                                args,
                                project_subdir
                            )
                            futures[future] = target_platform

                        # Submit all Apple platforms as a single sequential task
                        if apple_to_build:
                            apple_future = executor.submit(build_apple_sequential)
                            futures[apple_future] = "apple_group"

                        # Process completed builds as they finish
                        for future in as_completed(futures):
                            future_id = futures[future]

                            try:
                                if future_id == "apple_group":
                                    # Handle Apple group results
                                    apple_results = future.result()
                                    for platform_name, success, error_msg, elapsed, stdout, stderr in apple_results:
                                        completed_count += 1
                                        platform_times[platform_name] = elapsed
                                        in_progress.discard(platform_name)

                                        if success:
                                            successful_platforms.append(platform_name)
                                            print(f"✅ [{completed_count}/{total_platforms}] {platform_name.upper()} completed ({self._format_elapsed_time(elapsed)})")
                                        else:
                                            failed_platforms.append(platform_name)
                                            print(f"❌ [{completed_count}/{total_platforms}] {platform_name.upper()} failed ({self._format_elapsed_time(elapsed)})")
                                            if error_msg:
                                                error_lines = error_msg.strip().split('\n')
                                                for line in error_lines[:5]:
                                                    print(f"   {line}")
                                                if len(error_lines) > 5:
                                                    print(f"   ... ({len(error_lines) - 5} more lines)")

                                        if in_progress:
                                            print(f"   ⏳ Still building: {', '.join(sorted(in_progress))}")
                                else:
                                    # Handle individual platform result
                                    platform_name, success, error_msg, elapsed, stdout, stderr = future.result()
                                    completed_count += 1
                                    platform_times[platform_name] = elapsed
                                    in_progress.discard(platform_name)

                                    if success:
                                        successful_platforms.append(platform_name)
                                        print(f"✅ [{completed_count}/{total_platforms}] {platform_name.upper()} completed ({self._format_elapsed_time(elapsed)})")
                                    else:
                                        failed_platforms.append(platform_name)
                                        print(f"❌ [{completed_count}/{total_platforms}] {platform_name.upper()} failed ({self._format_elapsed_time(elapsed)})")
                                        if error_msg:
                                            # Print first few lines of error
                                            error_lines = error_msg.strip().split('\n')
                                            for line in error_lines[:5]:
                                                print(f"   {line}")
                                            if len(error_lines) > 5:
                                                print(f"   ... ({len(error_lines) - 5} more lines)")

                                    # Show remaining in-progress builds
                                    if in_progress:
                                        print(f"   ⏳ Still building: {', '.join(sorted(in_progress))}")

                            except Exception as e:
                                if future_id == "apple_group":
                                    # All Apple platforms failed
                                    for plat in apple_to_build:
                                        platform_times[plat] = 0
                                        failed_platforms.append(plat)
                                        in_progress.discard(plat)
                                    print(f"❌ Apple platforms failed with exception: {e}")
                                else:
                                    platform_times[future_id] = 0
                                    failed_platforms.append(future_id)
                                    in_progress.discard(future_id)
                                    print(f"❌ [{completed_count}/{total_platforms}] {future_id.upper()} failed with exception: {e}")
                except KeyboardInterrupt:
                    print("\n\n🛑 Parallel build aborted by user")
                    # Mark remaining platforms as failed
                    for plat in in_progress:
                        if plat not in failed_platforms and plat not in successful_platforms:
                            failed_platforms.append(plat)
                            platform_times[plat] = 0

            # Print summary
            print(f"\n{'='*80}")
            print("BUILD ALL SUMMARY")
            print(f"{'='*80}")
            print(f"\nTotal platforms: {total_platforms}")
            print(f"Successful: {len(successful_platforms)}")
            print(f"Failed: {len(failed_platforms)}")
            if num_jobs > 1:
                print(f"Parallel workers: {num_jobs}")

            if successful_platforms:
                print(f"\n✅ Successful builds:")
                for p in sorted(successful_platforms):
                    elapsed_str = self._format_elapsed_time(platform_times.get(p, 0))
                    print(f"   - {p} ({elapsed_str})")

            if failed_platforms:
                print(f"\n❌ Failed builds:")
                for p in sorted(failed_platforms):
                    elapsed_str = self._format_elapsed_time(platform_times.get(p, 0))
                    print(f"   - {p} ({elapsed_str})")

            self._print_build_time(start_time)

            if failed_platforms:
                sys.exit(1)
            else:
                print(f"\n🎉 All platforms built successfully!")
                sys.exit(0)

        print("Building library, with configuration...")
        print(vars(args))

        # Get current working directory (project directory)
        try:
            project_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            # If current directory was deleted, try to use PWD environment variable
            project_dir = os.environ.get('PWD')
            if not project_dir or not os.path.exists(project_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                print("Please navigate to your project directory and try again.")
                self._print_build_time(start_time)
                sys.exit(1)
            # Try to change to the saved directory
            try:
                os.chdir(project_dir)
            except (OSError, FileNotFoundError):
                print(f"ERROR: Cannot access project directory: {project_dir}")
                self._print_build_time(start_time)
                sys.exit(1)

        # Check if CCGO.toml exists in one of the subdirectories
        config_path = None
        for subdir in os.listdir(project_dir):
            potential_config = os.path.join(project_dir, subdir, "CCGO.toml")
            if os.path.isfile(potential_config):
                config_path = potential_config
                project_subdir = os.path.join(project_dir, subdir)
                break

        # If not found in subdirectory, check current directory
        if not config_path:
            if os.path.isfile(os.path.join(project_dir, "CCGO.toml")):
                config_path = os.path.join(project_dir, "CCGO.toml")
                project_subdir = project_dir
            else:
                print("❌ ERROR: CCGO.toml not found in project directory")
                print("Please create a CCGO.toml file in your project root directory")
                self._print_build_time(start_time)
                sys.exit(1)

        # Auto-enable Docker for cross-platform builds (unless --no-docker is specified)
        if not args.docker and not args.no_docker:
            host_platform = platform.system()  # 'Darwin', 'Linux', or 'Windows'
            if self._is_cross_platform_build(host_platform, args.target):
                args.docker = True
                print(f"\n=== Auto-enabling Docker Mode ===")
                print(f"Detected cross-platform build: {host_platform} → {args.target}")
                print(f"Docker mode automatically enabled for cross-compilation")
                print(f"(Use --no-docker to disable automatic Docker mode)\n")

        # Handle Docker builds for all supported platforms
        supported_docker_platforms = ["linux", "windows", "macos", "ios", "watchos", "tvos", "android"]
        if args.docker:
            if args.target in supported_docker_platforms:
                print(f"\n=== Docker Build for {args.target.capitalize()} ===")
                print("This will build the library using Docker containers")
                print("No local toolchains required - everything runs in Docker")

                # Get Docker build script path
                docker_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "dockers")
                docker_script = os.path.join(docker_dir, "build_docker.py")

                if not os.path.isfile(docker_script):
                    print(f"ERROR: Docker build script not found at {docker_script}")
                    self._print_build_time(start_time)
                    sys.exit(1)

                # Run Docker build
                dev_flag = " --dev" if args.docker_dev else ""
                # Add toolchain option for Windows builds
                toolchain_flag = ""
                if args.target == "windows" and hasattr(args, 'toolchain') and args.toolchain != "auto":
                    toolchain_flag = f" --toolchain={args.toolchain}"
                # Add link-type option
                link_type_flag = ""
                if hasattr(args, 'link_type') and args.link_type:
                    link_type_flag = f" --link-type={args.link_type}"
                cmd = f"python3 '{docker_script}' {args.target} '{project_subdir}'{dev_flag}{toolchain_flag}{link_type_flag}"
                print(f"Executing: {cmd}")

                err_code = os.system(cmd)
                self._print_build_time(start_time)
                sys.exit(err_code)
            else:
                print(f"WARNING: --docker option is not supported for {args.target} builds")
                print(f"Supported Docker platforms: {', '.join(supported_docker_platforms)}")
                print(f"Continuing with native build...")

        # If Android build without --native-only flag, use 3-step build process
        if args.target == "android" and not args.native_only:
            print("\n=== Android Full Build (Native + Gradle + Archive) ===")
            print("This will build native libraries, package AAR, and create archive")

            # Clean target/android directory before build
            target_android_dir = os.path.join(project_subdir, "target", "android")
            if os.path.exists(target_android_dir):
                shutil.rmtree(target_android_dir)
                print(f"Cleaned up old target/android/ directory")

            # Get build script path
            build_script_name = "build_android"
            build_scripts_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "build_scripts")
            build_script_path = os.path.join(build_scripts_dir, f"{build_script_name}.py")

            if not os.path.isfile(build_script_path):
                print(f"ERROR: Build script {build_script_path} not found")
                self._print_build_time(start_time)
                sys.exit(1)

            arch = args.arch if args.arch else "armeabi-v7a,arm64-v8a,x86_64"
            link_type = args.link_type if args.link_type else "both"
            # Always pass -j to ensure build scripts use argparse mode
            effective_jobs = args.jobs if args.jobs else multiprocessing.cpu_count()
            jobs_arg = f" -j {effective_jobs}"
            link_type_arg = f" --link-type {link_type}"

            # Step 1: Build native libraries
            print("\n--- Step 1: Building native libraries ---")
            native_cmd = f"cd '{project_subdir}' && python3 '{build_script_path}' --native-only --arch {arch}{jobs_arg}{link_type_arg}"
            print(f"Executing: {native_cmd}")

            err_code = os.system(native_cmd)
            if err_code != 0:
                print("ERROR: Native library build failed")
                self._print_build_time(start_time)
                sys.exit(err_code)

            # Step 2: Use Gradle to build and copy AAR to target/android/
            print("\n--- Step 2: Building AAR ---")
            gradlew_path = os.path.join(project_subdir, "android", "gradlew")
            if not os.path.isfile(gradlew_path):
                print(f"ERROR: gradlew not found at {gradlew_path}")
                self._print_build_time(start_time)
                sys.exit(1)

            gradle_cmd = f"cd '{project_subdir}/android' && chmod +x gradlew && ./gradlew --no-daemon :buildAAR"
            print(f"Executing: {gradle_cmd}")

            err_code = os.system(gradle_cmd)
            if err_code != 0:
                print("ERROR: AAR build failed")
                self._print_build_time(start_time)
                sys.exit(err_code)

            # Step 3: Create unified archive using Python's archive_android_project()
            print("\n--- Step 3: Creating unified archive ---")
            archive_cmd = f"cd '{project_subdir}' && python3 '{build_script_path}' --archive-only{link_type_arg}"
            print(f"Executing: {archive_cmd}")

            err_code = os.system(archive_cmd)
            self._print_build_time(start_time)
            sys.exit(err_code)

        # If KMP build, use build_scripts/build_kmp.py
        if args.target == "kmp":
            print("\n=== KMP Build (Kotlin Multiplatform Library) ===")
            print("This will build the KMP library for all supported platforms")

            # Get the build script from ccgo build_scripts directory
            build_scripts_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "build_scripts")
            build_kmp_script = os.path.join(build_scripts_dir, "build_kmp.py")

            if not os.path.isfile(build_kmp_script):
                print(f"ERROR: build_kmp.py not found at {build_kmp_script}")
                self._print_build_time(start_time)
                sys.exit(1)

            # Build library (default mode, no flags needed)
            cmd = f"cd '{project_subdir}' && python3 '{build_kmp_script}'"
            print(f"\nProject directory: {project_subdir}")
            print(f"Build script: {build_kmp_script}")
            print(f"Execute command:")
            print(cmd)

            err_code = os.system(cmd)
            self._print_build_time(start_time)
            sys.exit(err_code)

        # If Conan build, create Conan package
        if args.target == "conan":
            print("\n=== Conan Build (C/C++ Package Manager) ===")
            print("This will create a Conan package for the C/C++ library")

            # Get the build script from ccgo build_scripts directory
            build_scripts_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "build_scripts")
            build_conan_script = os.path.join(build_scripts_dir, "build_conan.py")

            if not os.path.isfile(build_conan_script):
                print(f"ERROR: build_conan.py not found at {build_conan_script}")
                self._print_build_time(start_time)
                sys.exit(1)

            # Execute conan build
            cmd = f"cd '{project_subdir}' && python3 '{build_conan_script}'"
            print(f"\nProject directory: {project_subdir}")
            print(f"Build script: {build_conan_script}")
            print(f"Execute command:")
            print(cmd)

            err_code = os.system(cmd)
            self._print_build_time(start_time)
            sys.exit(err_code)

        # If OHOS build without --native-only flag, use Hvigor buildHAR task
        if args.target == "ohos" and not args.native_only:
            print("\n=== OHOS Full Build (Native + Hvigor + Archive) ===")
            print("This will build native libraries, package HAR, and create archive")

            # Get build script path for native build
            build_script_name = "build_ohos"
            build_scripts_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "build_scripts")
            build_script_path = os.path.join(build_scripts_dir, f"{build_script_name}.py")

            if not os.path.isfile(build_script_path):
                print(f"ERROR: Build script {build_script_path} not found")
                self._print_build_time(start_time)
                sys.exit(1)

            arch = args.arch if args.arch else "armeabi-v7a,arm64-v8a,x86_64"
            link_type = args.link_type if args.link_type else "both"
            # Always pass -j to ensure build scripts use argparse mode
            effective_jobs = args.jobs if args.jobs else multiprocessing.cpu_count()
            jobs_arg = f" -j {effective_jobs}"
            link_type_arg = f" --link-type {link_type}"

            # Step 1: Build native libraries
            print("\n--- Step 1: Building native libraries ---")
            native_cmd = f"cd '{project_subdir}' && python3 '{build_script_path}' --native-only --arch {arch}{jobs_arg}{link_type_arg}"
            print(f"Executing: {native_cmd}")

            err_code = os.system(native_cmd)
            if err_code != 0:
                print("ERROR: Native library build failed")
                self._print_build_time(start_time)
                sys.exit(err_code)

            # Step 2: Use Hvigor buildHAR task (packages HAR and copies to target/ohos/)
            print("\n--- Step 2: Building HAR ---")
            hvigor_cmd = f"cd '{project_subdir}/ohos' && hvigorw buildHAR --mode module -p product=default --no-daemon --info"
            print(f"Executing: {hvigor_cmd}")

            err_code = os.system(hvigor_cmd)
            if err_code != 0:
                print("ERROR: HAR build failed")
                self._print_build_time(start_time)
                sys.exit(err_code)

            # Step 3: Create unified archive using Python's archive_ohos_project()
            print("\n--- Step 3: Creating unified archive ---")
            archive_cmd = f"cd '{project_subdir}' && python3 '{build_script_path}' --archive-only{link_type_arg}"
            print(f"Executing: {archive_cmd}")

            err_code = os.system(archive_cmd)
            self._print_build_time(start_time)
            sys.exit(err_code)

        # For native-only builds or other platforms, use Python build scripts
        print("\n=== Native Build (using build scripts) ===")

        # Get the build script module path
        build_script_name = f"build_{args.target}"
        build_scripts_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "build_scripts")
        build_script_path = os.path.join(build_scripts_dir, f"{build_script_name}.py")

        if not os.path.isfile(build_script_path):
            print(f"ERROR: Build script {build_script_path} not found")
            self._print_build_time(start_time)
            sys.exit(1)

        # Prepare command arguments based on target platform
        # Always pass -j to ensure build scripts use argparse mode (not legacy interactive mode)
        effective_jobs = args.jobs if args.jobs else multiprocessing.cpu_count()
        jobs_arg = f" -j {effective_jobs}"
        link_type = args.link_type if args.link_type else "both"
        link_type_arg = f" --link-type {link_type}"

        if args.target == "ohos":
            # OHOS uses new argument-based interface
            # Add --archive to create ZIP package (since Hvigor is not used in native-only mode)
            arch = args.arch if args.arch else "armeabi-v7a,arm64-v8a,x86_64"
            cmd = f"cd '{project_subdir}' && python3 '{build_script_path}' --native-only --archive --arch {arch}{jobs_arg}{link_type_arg}"
        elif args.target in ["ios", "macos", "linux", "watchos", "tvos"]:
            # Apple/Linux platforms use new argument-based interface
            ide_arg = " --ide-project" if args.ide_project else ""
            cmd = f"cd '{project_subdir}' && python3 '{build_script_path}'{jobs_arg}{ide_arg}{link_type_arg}"
        elif args.target == "android":
            # Android uses new argument-based interface
            # Add --archive to create ZIP package (since Gradle is not used in native-only mode)
            arch = args.arch if args.arch else "armeabi-v7a,arm64-v8a,x86_64"
            cmd = f"cd '{project_subdir}' && python3 '{build_script_path}' --native-only --archive --arch {arch}{jobs_arg}{link_type_arg}"
        else:
            # Other platforms (windows, etc.) - try new interface first, fallback to legacy
            ide_arg = " --ide-project" if args.ide_project else ""
            cmd = f"cd '{project_subdir}' && python3 '{build_script_path}'{jobs_arg}{ide_arg}{link_type_arg}"

        print(f"\nProject directory: {project_subdir}")
        print(f"Build script: {build_script_path}")
        print(f"Execute command:")
        print(cmd)

        err_code = os.system(cmd)
        self._print_build_time(start_time)
        sys.exit(err_code)
