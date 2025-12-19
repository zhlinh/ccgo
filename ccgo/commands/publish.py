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

# region setup path
SCRIPT_PATH = os.path.split(os.path.realpath(__file__))[0]
PROJECT_ROOT_PATH = os.path.dirname(SCRIPT_PATH)
sys.path.append(SCRIPT_PATH)
sys.path.append(PROJECT_ROOT_PATH)
PACKAGE_NAME = os.path.basename(SCRIPT_PATH)
# endregion
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


class Publish(CliCommand):
    def description(self) -> str:
        return """
        This is a subcommand to publish the library or documentation.

        Examples:
            # Common arguments (all targets)
            ccgo publish <target> [--registry local|official|private] [--skip-build] [-y]

            # Android/KMP: publish to Maven
            ccgo publish android --registry local           # Publish to Maven Local (~/.m2/repository/)
            ccgo publish android --registry official        # Publish to Maven Central
            ccgo publish android --registry local --skip-build  # Use existing AAR
            ccgo publish kmp --registry local               # Publish KMP to Maven Local
            ccgo publish kmp --registry official            # Publish KMP to Maven Central

            # OHOS: publish to OHPM
            ccgo publish ohos --registry local              # Publish to local OHPM registry
            ccgo publish ohos --registry official           # Publish to official OHPM registry
            ccgo publish ohos --registry private --url URL  # Publish to private registry

            # Conan: publish to Conan remote
            ccgo publish conan --registry local             # Publish to local cache
            ccgo publish conan --registry official          # Publish to first configured remote
            ccgo publish conan --registry private --remote-name myremote --url URL  # Private
            ccgo publish conan --registry local --skip-build  # Export recipe only

            # Apple: publish to CocoaPods and/or SPM
            ccgo publish apple --manager cocoapods          # Generate podspec and xcframework.zip
            ccgo publish apple --manager cocoapods --push   # Upload to GitHub releases
            ccgo publish apple --manager cocoapods --registry private --remote-name myspecs
            ccgo publish apple --manager spm --push         # Generate Package.swift and push tag
            ccgo publish apple --manager all --push         # Publish to both

            # Documentation: publish to GitHub Pages
            ccgo publish doc --doc-branch gh-pages --doc-open
        """

    def get_target_list(self) -> list:
        return [
            "android",
            "ohos",
            "kmp",
            "conan",
            "apple",
            "doc",
        ]

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo publish",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )
        parser.add_argument(
            "target",
            metavar=f"{self.get_target_list()}",
            type=str,
            choices=self.get_target_list(),
        )

        # ========== Common Options (all targets) ==========
        parser.add_argument(
            "--registry",
            type=str,
            choices=["local", "official", "private"],
            default=None,
            help="Registry type: local, official, or private",
        )
        parser.add_argument(
            "--remote-name",
            type=str,
            default=None,
            help="Remote repository name (Conan remote or CocoaPods spec repo)",
        )
        parser.add_argument(
            "--url",
            type=str,
            default=None,
            help="Custom registry URL (required for private if remote doesn't exist)",
        )
        parser.add_argument(
            "--skip-build",
            action="store_true",
            help="Skip build step, use existing artifacts (also: Conan export-only mode)",
        )
        parser.add_argument(
            "-y", "--yes",
            action="store_true",
            help="Skip confirmation prompts",
        )

        # ========== Apple-specific Options ==========
        parser.add_argument(
            "--manager",
            type=str,
            choices=["cocoapods", "spm", "all"],
            default=None,
            help="Package manager for apple target: cocoapods, spm, or all",
        )
        parser.add_argument(
            "--push",
            action="store_true",
            default=False,
            help="Push to remote (git tag for SPM, GitHub release for CocoaPods)",
        )
        parser.add_argument(
            "--platform",
            type=str,
            default=None,
            help="Apple platforms to publish: ios,macos,tvos,watchos",
        )
        parser.add_argument(
            "--allow-warnings",
            action="store_true",
            default=True,
            help="Allow warnings during CocoaPods lint (default: true)",
        )

        # ========== Conan-specific Options ==========
        parser.add_argument(
            "--profile",
            type=str,
            default="default",
            help="Conan profile to use (default: default)",
        )
        parser.add_argument(
            "--link-type",
            type=str,
            choices=["static", "shared", "both"],
            default="both",
            help="Library type to build for Conan: static, shared, or both (default: both)",
        )

        # ========== Doc-specific Options ==========
        parser.add_argument(
            "--doc-branch",
            type=str,
            default=None,
            help="GitHub Pages branch name (default: from publish.pages_branch in CCGO.toml)",
        )
        parser.add_argument(
            "--doc-force",
            action="store_true",
            help="Force push to remote repository",
        )
        parser.add_argument(
            "--doc-open",
            action="store_true",
            help="Open documentation in browser after publishing",
        )

        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        print("Publishing library project, with configuration...")
        print(vars(args))

        if args.target == "android":
            # Android: publish to Maven repository
            # Change to android directory to run gradlew
            android_dir = os.path.join(os.getcwd(), "android")
            if not os.path.isdir(android_dir):
                print(f"\nError: android directory not found at {android_dir}")
                print("Please run this command from the project root directory")
                sys.exit(1)

            # Determine Maven repository target
            if args.registry:
                # Use command line argument
                maven_target = args.registry
            else:
                # Ask user which Maven repository to publish to
                print("\nMaven Publish Options:")
                print("  1 - Publish to Local (~/.m2/repository/)")
                print("  2 - Publish to Official (Maven Central, requires credentials)")
                print("  3 - Publish to Private (custom repository, requires configuration)")
                choice = input("\nSelect option [1]: ").strip() or "1"

                if choice == "1":
                    maven_target = "local"
                elif choice == "2":
                    maven_target = "official"
                elif choice == "3":
                    maven_target = "private"
                else:
                    print("Invalid option. Please select 1, 2, or 3.")
                    sys.exit(1)

            # Map target to Gradle task (ccgo-prefixed tasks to avoid conflicts)
            maven_task_map = {
                "local": "ccgoPublishToMavenLocal",
                "official": "ccgoPublishToMavenCentral",
                "private": "ccgoPublishToMavenCustom",
            }
            gradle_task = maven_task_map[maven_target]

            # Build gradle command
            skip_build_flag = "-x buildAAR" if args.skip_build else ""
            cmd = f"cd '{android_dir}' && ./gradlew {gradle_task} {skip_build_flag} --no-daemon"
            print(f"\nRunning: ./gradlew {gradle_task} {skip_build_flag}".strip())
            print("-" * 60)
            sys.stdout.flush()
            # Use subprocess.run with real-time output (stdout=None passes through to terminal)
            import subprocess
            result = subprocess.run(cmd, shell=True)
            if result.returncode != 0:
                print(f"\nPublish failed with exit code: {result.returncode}")
                sys.exit(result.returncode)
            print("\nPublish completed successfully!")
        elif args.target == "ohos":
            # OHOS: build HAR and publish to OHPM using OhpmPublisher
            self.publish_ohos(context, args)
        elif args.target == "kmp":
            # KMP: publish to Maven repository (local or remote)
            # Get current working directory (project directory)
            try:
                project_dir = os.getcwd()
            except (OSError, FileNotFoundError) as e:
                # If current directory was deleted, try to use PWD environment variable
                project_dir = os.environ.get('PWD')
                if not project_dir or not os.path.exists(project_dir):
                    print(f"ERROR: Current working directory no longer exists: {e}")
                    print("Please navigate to your project directory and try again.")
                    sys.exit(1)
                # Try to change to the saved directory
                try:
                    os.chdir(project_dir)
                except (OSError, FileNotFoundError):
                    print(f"ERROR: Cannot access project directory: {project_dir}")
                    sys.exit(1)

            # Check if CCGO.toml exists in one of the subdirectories
            config_path = None
            for subdir in os.listdir(project_dir):
                subdir_path = os.path.join(project_dir, subdir)
                if not os.path.isdir(subdir_path):
                    continue
                potential_config = os.path.join(subdir_path, "CCGO.toml")
                if os.path.isfile(potential_config):
                    config_path = potential_config
                    project_subdir = subdir_path
                    break

            # If not found in subdirectory, check current directory
            if not config_path:
                if os.path.isfile(os.path.join(project_dir, "CCGO.toml")):
                    config_path = os.path.join(project_dir, "CCGO.toml")
                    project_subdir = project_dir
                else:
                    print("\n❌ ERROR: CCGO.toml not found in project directory")
                    print("Please ensure you are in a CCGO project directory")
                    sys.exit(1)

            # Use build_kmp.py script from ccgo build_scripts directory
            build_scripts_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "build_scripts")
            build_kmp_script = os.path.join(build_scripts_dir, "build_kmp.py")

            if not os.path.isfile(build_kmp_script):
                print(f"ERROR: build_kmp.py not found at {build_kmp_script}")
                print("Please ensure ccgo is properly installed")
                sys.exit(1)

            # Determine Maven repository target
            if args.registry:
                maven_target = args.registry
            else:
                # Ask user which Maven repository to publish to
                print("\nMaven Publish Options:")
                print("  1 - Publish to Local (~/.m2/repository/)")
                print("  2 - Publish to Official (Maven Central, requires credentials)")
                print("  3 - Publish to Private (custom repository, requires configuration)")
                choice = input("\nSelect option [1]: ").strip() or "1"

                if choice == "1":
                    maven_target = "local"
                elif choice == "2":
                    maven_target = "official"
                elif choice == "3":
                    maven_target = "private"
                else:
                    print("Invalid option. Please select 1, 2, or 3.")
                    sys.exit(1)

            # Map target to publish flag
            maven_flag_map = {
                "local": "--publish-local",
                "official": "--publish-central",  # Maven Central (Sonatype OSSRH)
                "private": "--publish-custom",    # Custom Maven repository
            }
            publish_flag = maven_flag_map[maven_target]

            cmd = f"cd '{project_subdir}' && python3 '{build_kmp_script}' {publish_flag}"
            print(f"\nExecuting: {cmd}")

            err_code = os.system(cmd)
            if err_code != 0:
                sys.exit(err_code)

            print("\nSuccessfully published KMP library!")
        elif args.target == "conan":
            # Conan: publish to local cache or remote repository
            self.publish_conan(context, args)
        elif args.target == "doc":
            # Doc: publish documentation to GitHub Pages
            self.publish_doc(context, args)
        elif args.target == "apple":
            # Apple: publish to CocoaPods and/or SPM
            self.publish_apple(context, args)
        else:
            print(f"\nPublishing not yet supported for {args.target}")
            print("Currently supported: android, ohos, kmp, conan, apple, doc")
            sys.exit(1)

    def publish_conan(self, context: CliContext, args: CliNameSpace):
        """Publish to Conan local cache or remote repository"""
        print("\n=== Publishing to Conan ===")

        # Get current working directory (project directory)
        try:
            project_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            project_dir = os.environ.get('PWD')
            if not project_dir or not os.path.exists(project_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                print("Please navigate to your project directory and try again.")
                sys.exit(1)
            try:
                os.chdir(project_dir)
            except (OSError, FileNotFoundError):
                print(f"ERROR: Cannot access project directory: {project_dir}")
                sys.exit(1)

        # Find project subdirectory with CCGO.toml
        config_path = None
        project_subdir = project_dir
        for subdir in os.listdir(project_dir):
            potential_config = os.path.join(project_dir, subdir, "CCGO.toml")
            if os.path.isfile(potential_config):
                config_path = potential_config
                project_subdir = os.path.join(project_dir, subdir)
                break

        if not config_path:
            if os.path.isfile(os.path.join(project_dir, "CCGO.toml")):
                config_path = os.path.join(project_dir, "CCGO.toml")
                project_subdir = project_dir
            else:
                print("ERROR: CCGO.toml not found in project directory")
                sys.exit(1)

        # Get publish_conan.py script path
        build_scripts_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "build_scripts")
        publish_conan_script = os.path.join(build_scripts_dir, "publish_conan.py")

        if not os.path.isfile(publish_conan_script):
            print(f"ERROR: publish_conan.py not found at {publish_conan_script}")
            sys.exit(1)

        # Get list of configured remotes
        import subprocess
        remotes = []
        readonly_remotes = ['conancenter']  # Read-only remotes that can't be uploaded to
        try:
            result = subprocess.run(
                ["conan", "remote", "list"],
                capture_output=True,
                text=True,
                check=False,
                timeout=10
            )
            if result.returncode == 0 and result.stdout.strip():
                # Parse remote names from output (format: "name: url")
                for line in result.stdout.strip().split('\n'):
                    if ':' in line:
                        remote_name = line.split(':')[0].strip()
                        if remote_name and not remote_name.startswith('#'):
                            # Skip read-only remotes
                            if remote_name.lower() not in readonly_remotes:
                                remotes.append(remote_name)
        except Exception:
            pass

        # Determine publish target
        conan_mode = "local"
        conan_remote = None

        # Handle --registry option
        if args.registry:
            if args.registry == "local":
                conan_mode = "local"
            elif args.registry == "official":
                if remotes:
                    conan_mode = "remote"
                    conan_remote = remotes[0]
                else:
                    print("ERROR: No writable remotes configured")
                    print("\nTo add a remote:")
                    print("  conan remote add <name> <url>")
                    print("\nExample:")
                    print("  conan remote add artifactory https://mycompany.jfrog.io/artifactory/api/conan/conan-local")
                    sys.exit(1)
            elif args.registry == "private":
                if not args.remote_name:
                    print("ERROR: --remote-name is required for private remote")
                    sys.exit(1)
                # Check if remote exists
                remote_exists = args.remote_name in remotes
                if not remote_exists and not args.url:
                    print(f"ERROR: Remote '{args.remote_name}' not found. Provide --url to add it.")
                    sys.exit(1)
                # Add or update the private remote if URL provided
                if args.url:
                    print(f"Configuring private remote: {args.remote_name} -> {args.url}")
                    try:
                        # Try to add, if exists will fail
                        add_result = subprocess.run(
                            ["conan", "remote", "add", args.remote_name, args.url],
                            capture_output=True,
                            text=True,
                            check=False,
                            timeout=10
                        )
                        if add_result.returncode != 0:
                            # Remote might exist, try to update URL
                            subprocess.run(
                                ["conan", "remote", "update", args.remote_name, "--url", args.url],
                                capture_output=True,
                                text=True,
                                check=False,
                                timeout=10
                            )
                    except Exception as e:
                        print(f"Warning: Failed to configure remote: {e}")
                conan_mode = "remote"
                conan_remote = args.remote_name
        else:
            # Interactive prompt
            print("\nConan Publish Options:")
            print("  1 - Publish to Local Cache (for testing)")
            if remotes:
                print(f"  2 - Publish to Remote: {remotes[0]}")
            else:
                print("  2 - Publish to Remote (no writable remotes configured)")
            print("  3 - Publish to Private Remote (requires URL)")

            choice = input("\nSelect option [1]: ").strip() or "1"

            if choice == "1":
                conan_mode = "local"
            elif choice == "2":
                if remotes:
                    conan_mode = "remote"
                    conan_remote = remotes[0]
                else:
                    print("\nNo writable remotes configured. To add a remote:")
                    print("  conan remote add <name> <url>")
                    print("\nExample:")
                    print("  conan remote add artifactory https://mycompany.jfrog.io/artifactory/api/conan/conan-local")
                    sys.exit(1)
            elif choice == "3":
                conan_name = input("Enter remote name: ").strip()
                if not conan_name:
                    print("ERROR: Remote name is required")
                    sys.exit(1)
                conan_url = input("Enter remote URL: ").strip()
                if not conan_url:
                    print("ERROR: Remote URL is required")
                    sys.exit(1)
                # Add or update the private remote
                print(f"Configuring private remote: {conan_name} -> {conan_url}")
                try:
                    add_result = subprocess.run(
                        ["conan", "remote", "add", conan_name, conan_url],
                        capture_output=True,
                        text=True,
                        check=False,
                        timeout=10
                    )
                    if add_result.returncode != 0:
                        subprocess.run(
                            ["conan", "remote", "update", conan_name, "--url", conan_url],
                            capture_output=True,
                            text=True,
                            check=False,
                            timeout=10
                        )
                except Exception as e:
                    print(f"Warning: Failed to configure remote: {e}")
                conan_mode = "remote"
                conan_remote = conan_name
            else:
                print("Invalid option. Please select 1, 2, or 3.")
                sys.exit(1)

        # Build command arguments
        cmd_args = ["python3", f"'{publish_conan_script}'"]

        # Set mode and remote
        if conan_mode == "remote" and conan_remote:
            cmd_args.extend(["--mode", "remote", "--remote", conan_remote])
        else:
            cmd_args.extend(["--mode", "local"])

        # Add profile
        if args.profile and args.profile != "default":
            cmd_args.extend(["--profile", args.profile])

        # Add export-only flag (--skip-build means export only for Conan)
        if args.skip_build:
            cmd_args.append("--export-only")

        # Add link-type
        if args.link_type:
            cmd_args.extend(["--link-type", args.link_type])

        # Add confirmation skip
        if args.yes:
            cmd_args.append("-y")

        cmd = f"cd '{project_subdir}' && {' '.join(cmd_args)}"
        print(f"Executing: {cmd}")

        err_code = os.system(cmd)
        if err_code != 0:
            sys.exit(err_code)

    def publish_doc(self, context: CliContext, args: CliNameSpace):
        """Publish documentation to GitHub Pages"""
        import subprocess
        import shutil
        import datetime

        print("\n📚 Publishing documentation to GitHub Pages...\n")

        # Get current working directory (project root directory)
        try:
            project_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            project_dir = os.environ.get('PWD')
            if not project_dir or not os.path.exists(project_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                print("Please navigate to your project directory and try again.")
                sys.exit(1)
            try:
                os.chdir(project_dir)
            except (OSError, FileNotFoundError):
                print(f"ERROR: Cannot access project directory: {project_dir}")
                sys.exit(1)

        # Check if CCGO.toml exists
        ccgo_toml_path = os.path.join(project_dir, "CCGO.toml")
        if not os.path.isfile(ccgo_toml_path):
            print("❌ ERROR: CCGO.toml not found in project root directory")
            print(f"Expected location: {ccgo_toml_path}")
            print("\nPlease run this command from the project root directory.")
            sys.exit(1)

        # Load CCGO.toml to get configuration
        print(f"Loading configuration from: {ccgo_toml_path}")
        config = {}
        try:
            # Import tomllib (Python 3.11+) or use fallback
            try:
                import tomllib
            except ImportError:
                try:
                    import tomli as tomllib
                except ImportError:
                    print("ERROR: tomllib not available. Please install tomli for Python < 3.11")
                    print("Run: pip install tomli")
                    sys.exit(1)

            with open(ccgo_toml_path, 'rb') as f:
                toml_data = tomllib.load(f)

            # Convert TOML structure to match expected format
            project_relative_path = toml_data.get('project', {}).get('name', '')
            pages_branch_name = args.doc_branch or toml_data.get('publish', {}).get('pages_branch', 'gh-pages')
        except Exception as e:
            print(f"ERROR: Failed to load CCGO.toml: {e}")
            sys.exit(1)

        if not project_relative_path:
            print("ERROR: project.name not found in CCGO.toml")
            sys.exit(1)

        print(f"Project relative path: {project_relative_path}")
        print(f"Pages branch: {pages_branch_name}")
        print()

        # Get python command
        python_cmd = "python" if sys.platform.startswith("win") else "python3"

        # Step 1: Build documentation using build.py
        print("[Step 1/4] Building documentation...")
        build_cmd = [python_cmd, "build.py", "CI_BUILD_DOCS"]
        try:
            subprocess.run(build_cmd, check=True, cwd=project_dir)
        except subprocess.CalledProcessError as e:
            print(f"\nERROR: Documentation build failed: {e}")
            sys.exit(1)
        print("✓ Documentation built successfully\n")

        # Step 2: Copy HTML files to root
        print("[Step 2/4] Copying HTML files to root directory...")
        html_dir = os.path.join(project_dir, project_relative_path, "docs", "_html")

        if not os.path.isdir(html_dir):
            print(f"ERROR: HTML output directory not found: {html_dir}")
            sys.exit(1)

        files_list = os.listdir(html_dir)
        for item in files_list:
            src = os.path.join(html_dir, item)
            dst = os.path.join(project_dir, item)
            if os.path.isdir(src):
                if os.path.exists(dst):
                    shutil.rmtree(dst)
                shutil.copytree(src, dst)
            else:
                shutil.copy2(src, dst)
        print(f"✓ Copied {len(files_list)} items to root directory\n")

        # Step 3: Git operations
        print("[Step 3/4] Managing Git branches...")

        # Get current branch
        try:
            last_branch = subprocess.check_output(
                ["git", "symbolic-ref", "--short", "-q", "HEAD"],
                cwd=project_dir
            ).decode().strip()
            print(f"Current branch: {last_branch}")
        except subprocess.CalledProcessError:
            print("ERROR: Failed to get current branch")
            sys.exit(1)

        # Delete old pages branch if exists (ignore error if not exists)
        try:
            subprocess.run(
                ["git", "branch", "-D", pages_branch_name],
                cwd=project_dir,
                check=True,
                capture_output=True
            )
            print(f"Deleted old '{pages_branch_name}' branch")
        except subprocess.CalledProcessError:
            pass  # Branch doesn't exist, that's fine

        # Create new pages branch
        try:
            subprocess.run(
                ["git", "checkout", "-b", pages_branch_name],
                cwd=project_dir,
                check=True
            )
            print(f"Created new '{pages_branch_name}' branch")
        except subprocess.CalledProcessError as e:
            print(f"\nERROR: Failed to create branch '{pages_branch_name}': {e}")
            sys.exit(1)

        # Commit changes
        now_date = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        commit_message = f"ci({pages_branch_name}): release {pages_branch_name} on {now_date}"

        try:
            subprocess.run(["git", "add", "--all"], cwd=project_dir, check=True)
            subprocess.run(
                ["git", "commit", "-a", "-m", commit_message],
                cwd=project_dir,
                check=True
            )
            print(f"✓ Committed changes: {commit_message}\n")
        except subprocess.CalledProcessError as e:
            print(f"\nERROR: Failed to commit changes: {e}")
            # Try to checkout back to original branch
            subprocess.run(["git", "checkout", last_branch], cwd=project_dir)
            sys.exit(1)

        # Step 4: Push to remote
        print("[Step 4/4] Pushing to remote repository...")
        push_cmd = ["git", "push", "--set-upstream", "origin", pages_branch_name]
        if args.doc_force:
            push_cmd.append("-f")
            print("Using force push (-f)")

        try:
            subprocess.run(push_cmd, cwd=project_dir, check=True)
            print(f"✓ Successfully pushed to origin/{pages_branch_name}\n")
        except subprocess.CalledProcessError as e:
            print(f"\nERROR: Failed to push to remote: {e}")
            print("You may need to use --force flag if the branch already exists remotely.")
            # Checkout back to original branch
            subprocess.run(["git", "checkout", last_branch], cwd=project_dir)
            sys.exit(1)

        # Checkout back to original branch
        try:
            subprocess.run(["git", "checkout", last_branch], cwd=project_dir, check=True)
            print(f"Switched back to '{last_branch}' branch")
        except subprocess.CalledProcessError as e:
            print(f"\nWARNING: Failed to switch back to '{last_branch}': {e}")

        # Success message
        print("\n" + "="*60)
        print("🎉 Documentation published successfully!")
        print("="*60)
        print(f"\nBranch: {pages_branch_name}")

        # Get repository URL from git config
        try:
            repo_url = subprocess.check_output(
                ["git", "config", "--get", "remote.origin.url"],
                cwd=project_dir
            ).decode().strip()

            # Convert SSH URL to HTTPS for display
            if repo_url.startswith("git@github.com:"):
                repo_url = repo_url.replace("git@github.com:", "https://github.com/")
            if repo_url.endswith(".git"):
                repo_url = repo_url[:-4]

            print(f"Repository: {repo_url}")
            print(f"\nYour documentation should be available at:")
            print(f"  {repo_url}/tree/{pages_branch_name}")

            # If using gh-pages, show the GitHub Pages URL
            if pages_branch_name == "gh-pages" and "github.com" in repo_url:
                # Extract username/repo from URL
                parts = repo_url.split("/")
                if len(parts) >= 2:
                    username = parts[-2]
                    repo_name = parts[-1]
                    pages_url = f"https://{username}.github.io/{repo_name}/"
                    print(f"\nGitHub Pages URL (once enabled):")
                    print(f"  {pages_url}")

                    if args.doc_open:
                        import webbrowser
                        print(f"\nOpening {pages_url} in browser...")
                        webbrowser.open(pages_url)
        except:
            pass  # Ignore errors in URL extraction

        print()

    def publish_ohos(self, context: CliContext, args: CliNameSpace):
        """Publish to OHPM (OpenHarmony Package Manager)"""
        print("\n=== Publishing to OHPM ===")

        # Get current working directory (project directory)
        try:
            project_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            project_dir = os.environ.get('PWD')
            if not project_dir or not os.path.exists(project_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                print("Please navigate to your project directory and try again.")
                sys.exit(1)
            try:
                os.chdir(project_dir)
            except (OSError, FileNotFoundError):
                print(f"ERROR: Cannot access project directory: {project_dir}")
                sys.exit(1)

        # Find CCGO.toml configuration
        config_path = None
        project_subdir = project_dir
        for subdir in os.listdir(project_dir):
            subdir_path = os.path.join(project_dir, subdir)
            if not os.path.isdir(subdir_path):
                continue
            potential_config = os.path.join(subdir_path, "CCGO.toml")
            if os.path.isfile(potential_config):
                config_path = potential_config
                project_subdir = subdir_path
                break

        if not config_path:
            if os.path.isfile(os.path.join(project_dir, "CCGO.toml")):
                config_path = os.path.join(project_dir, "CCGO.toml")
                project_subdir = project_dir
            else:
                print("ERROR: CCGO.toml not found in project directory")
                sys.exit(1)

        # Load CCGO.toml configuration
        print(f"Loading configuration from: {config_path}")
        try:
            try:
                import tomllib
            except ImportError:
                try:
                    import tomli as tomllib
                except ImportError:
                    print("ERROR: tomllib not available. Please install tomli for Python < 3.11")
                    print("Run: pip install tomli")
                    sys.exit(1)

            with open(config_path, 'rb') as f:
                toml_config = tomllib.load(f)
        except Exception as e:
            print(f"ERROR: Failed to load CCGO.toml: {e}")
            sys.exit(1)

        # Import OHPM publishing utilities
        try:
            from ccgo.utils.ohpm.config import OhpmConfig
            from ccgo.utils.ohpm.publisher import OhpmPublisher
        except ImportError:
            from utils.ohpm.config import OhpmConfig
            from utils.ohpm.publisher import OhpmPublisher

        # Determine OHPM registry target
        if args.registry:
            # Use command line argument
            ohpm_target = args.registry
        else:
            # Ask user which registry to publish to
            print("\nOHPM Publish Options:")
            print("  1 - Publish to Local Registry (for testing)")
            print("  2 - Publish to Official OHPM Registry (ohpm.openharmony.cn)")
            print("  3 - Publish to Private Registry (requires URL)")
            choice = input("Select option (1, 2, or 3): ").strip()

            if choice == "1":
                ohpm_target = "local"
            elif choice == "2":
                ohpm_target = "official"
            elif choice == "3":
                ohpm_target = "private"
            else:
                print("Invalid option. Please select 1, 2, or 3.")
                sys.exit(1)

        # Create configuration
        config = OhpmConfig(toml_config)

        # Override registry type from command line
        config.registry_type = ohpm_target

        # Override registry URL for private registry
        if ohpm_target == "private" and args.url:
            config.registry_url = args.url

        # Validate configuration
        is_valid, error_msg = config.validate()
        if not is_valid:
            print(f"ERROR: Invalid configuration: {error_msg}")
            sys.exit(1)

        # Print configuration summary
        print("\nConfiguration:")
        print(config.get_config_summary())
        print()

        # Create publisher and publish
        publisher = OhpmPublisher(config, project_subdir, verbose=True)

        try:
            success = publisher.publish()
            if success:
                print("\n" + "=" * 60)
                print("OHPM publishing completed successfully!")
                print("=" * 60)
            else:
                print("\n" + "=" * 60)
                print("OHPM publishing failed")
                print("=" * 60)
                sys.exit(1)
        finally:
            publisher.restore_package_files()

    def publish_apple(self, context: CliContext, args: CliNameSpace):
        """Publish to Apple platforms (CocoaPods and/or SPM)"""
        print("\n=== Publishing to Apple Platforms ===")

        # Get current working directory (project directory)
        try:
            project_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            project_dir = os.environ.get('PWD')
            if not project_dir or not os.path.exists(project_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                print("Please navigate to your project directory and try again.")
                sys.exit(1)
            try:
                os.chdir(project_dir)
            except (OSError, FileNotFoundError):
                print(f"ERROR: Cannot access project directory: {project_dir}")
                sys.exit(1)

        # Find CCGO.toml configuration
        config_path = None
        project_subdir = project_dir
        for subdir in os.listdir(project_dir):
            subdir_path = os.path.join(project_dir, subdir)
            if not os.path.isdir(subdir_path):
                continue
            potential_config = os.path.join(subdir_path, "CCGO.toml")
            if os.path.isfile(potential_config):
                config_path = potential_config
                project_subdir = subdir_path
                break

        if not config_path:
            if os.path.isfile(os.path.join(project_dir, "CCGO.toml")):
                config_path = os.path.join(project_dir, "CCGO.toml")
                project_subdir = project_dir
            else:
                print("ERROR: CCGO.toml not found in project directory")
                sys.exit(1)

        # Load CCGO.toml configuration
        print(f"Loading configuration from: {config_path}")
        try:
            try:
                import tomllib
            except ImportError:
                try:
                    import tomli as tomllib
                except ImportError:
                    print("ERROR: tomllib not available. Please install tomli for Python < 3.11")
                    print("Run: pip install tomli")
                    sys.exit(1)

            with open(config_path, 'rb') as f:
                toml_config = tomllib.load(f)
        except Exception as e:
            print(f"ERROR: Failed to load CCGO.toml: {e}")
            sys.exit(1)

        # Import apple publishing utilities
        try:
            from ccgo.utils.apple.config import ApplePublishConfig
            from ccgo.utils.apple.cocoapods import CocoaPodsPublisher
            from ccgo.utils.apple.spm import SPMPublisher
        except ImportError:
            from utils.apple.config import ApplePublishConfig
            from utils.apple.cocoapods import CocoaPodsPublisher
            from utils.apple.spm import SPMPublisher

        # Override platform configuration from command line
        if args.platform:
            if 'publish' not in toml_config:
                toml_config['publish'] = {}
            if 'apple' not in toml_config['publish']:
                toml_config['publish']['apple'] = {}
            toml_config['publish']['apple']['platforms'] = args.platform.split(',')

        # Override CocoaPods repo from command line (--registry private --remote-name)
        if args.registry == 'private' and args.remote_name:
            if 'publish' not in toml_config:
                toml_config['publish'] = {}
            if 'apple' not in toml_config['publish']:
                toml_config['publish']['apple'] = {}
            if 'cocoapods' not in toml_config['publish']['apple']:
                toml_config['publish']['apple']['cocoapods'] = {}
            toml_config['publish']['apple']['cocoapods']['repo'] = args.remote_name

        # Create configuration
        config = ApplePublishConfig(toml_config, project_subdir)

        # Validate configuration
        is_valid, errors = config.validate()
        if not is_valid:
            print("ERROR: Invalid configuration:")
            for error in errors:
                print(f"  - {error}")
            sys.exit(1)

        # Print configuration summary
        print("\nConfiguration:")
        print(config.get_config_summary())
        print()

        # Determine what to publish based on --manager argument
        publish_cocoapods = False
        publish_spm = False

        if args.manager:
            if args.manager == "cocoapods":
                publish_cocoapods = True
            elif args.manager == "spm":
                publish_spm = True
            elif args.manager == "all":
                publish_cocoapods = True
                publish_spm = True
        else:
            # If not specified, ask user
            print("Apple Publish Options:")
            print("  1 - Publish to CocoaPods")
            print("  2 - Publish to Swift Package Manager (SPM)")
            print("  3 - Publish to both")
            choice = input("Select option (1, 2, or 3): ").strip()

            if choice == "1":
                publish_cocoapods = True
            elif choice == "2":
                publish_spm = True
            elif choice == "3":
                publish_cocoapods = True
                publish_spm = True
            else:
                print("Invalid option. Please select 1, 2, or 3.")
                sys.exit(1)

        success = True
        publish_skipped = False  # Track if publishing was skipped (files generated but not published)

        # Publish to CocoaPods
        if publish_cocoapods and config.cocoapods.enabled:
            print("\n[CocoaPods] Starting publish process...")
            publisher = CocoaPodsPublisher(config)

            # Create CocoaPods-compatible xcframework zip from SDK build output
            # Do this first since users may want to generate files without publishing
            print("[CocoaPods] Creating xcframework.zip from SDK build...")
            cocoapods_zip = publisher.create_cocoapods_zip()
            if cocoapods_zip:
                print(f"[CocoaPods] Created: {cocoapods_zip}")

                # Upload to GitHub releases if requested (--push for CocoaPods means upload)
                if args.push:
                    print("\n[GitHub] Uploading to GitHub releases...")
                    upload_ok, upload_msg = publisher.upload_to_github_release(cocoapods_zip)
                    if upload_ok:
                        print(f"[GitHub] {upload_msg}")
                    else:
                        print(f"[GitHub] Upload failed: {upload_msg}")
                        print("[CocoaPods] Continuing without upload...")
                else:
                    print(f"[CocoaPods] Upload this file to GitHub releases before publishing to trunk")
                    print(f"[CocoaPods] Or use --push flag to auto-upload via gh CLI")
            else:
                print("[CocoaPods] Warning: Could not create xcframework.zip (SDK zip not found)")

            # Generate podspec
            print("[CocoaPods] Generating podspec...")
            podspec_path = publisher.generate_podspec()
            print(f"[CocoaPods] Generated: {podspec_path}")

            # Validate prerequisites for publishing
            prereq_ok, prereq_msg = publisher.validate_prerequisites()
            if not prereq_ok:
                print(f"[CocoaPods] Warning: {prereq_msg}")
                print("[CocoaPods] Podspec and xcframework.zip generated, but cannot publish to trunk")
                publish_skipped = True  # Files generated but publishing skipped
            else:
                # Lint podspec
                print("[CocoaPods] Validating podspec...")
                lint_ok, lint_msg = publisher.lint_podspec(allow_warnings=args.allow_warnings)
                if not lint_ok:
                    print(f"[CocoaPods] Validation failed: {lint_msg}")
                    success = False
                else:
                    print("[CocoaPods] Validation passed")

                    # Publish
                    print(f"[CocoaPods] Publishing to {config.cocoapods.repo}...")
                    pub_ok, pub_msg = publisher.publish(allow_warnings=args.allow_warnings)
                    if pub_ok:
                        print(f"[CocoaPods] Successfully published!")
                        print(pub_msg)
                    else:
                        print(f"[CocoaPods] Publish failed: {pub_msg}")
                        success = False

        elif publish_cocoapods and not config.cocoapods.enabled:
            print("[CocoaPods] Skipped (disabled in configuration)")

        # Publish to SPM
        if publish_spm and config.spm.enabled:
            print("\n[SPM] Starting publish process...")
            publisher = SPMPublisher(config)

            # Validate prerequisites
            prereq_ok, prereq_msg = publisher.validate_prerequisites()
            if not prereq_ok:
                print(f"ERROR: {prereq_msg}")
                success = False
            else:
                # Generate Package.swift
                print("[SPM] Generating Package.swift...")
                package_path = publisher.generate_package_swift()
                print(f"[SPM] Generated: {package_path}")

                # Publish (create git tag)
                print(f"[SPM] Creating git tag for version {config.version}...")
                pub_ok, pub_msg = publisher.publish(push_tag=args.push)
                if pub_ok:
                    print(f"[SPM] Successfully published!")
                    print(pub_msg)

                    # Show package URL
                    package_url = publisher.get_package_url()
                    if package_url:
                        print(f"\n[SPM] To add this package to your project:")
                        print(f"      URL: {package_url}")
                        print(f"      Version: {config.version}")
                else:
                    print(f"[SPM] Publish failed: {pub_msg}")
                    success = False

        elif publish_spm and not config.spm.enabled:
            print("[SPM] Skipped (disabled in configuration)")

        # Final status
        if success and not publish_skipped:
            print("\n" + "=" * 60)
            print("Apple publishing completed successfully!")
            print("=" * 60)
        elif success and publish_skipped:
            print("\n" + "=" * 60)
            print("Files generated successfully, but publishing was skipped")
            print("Please fix the warnings above and run again to publish")
            print("=" * 60)
        else:
            print("\n" + "=" * 60)
            print("Apple publishing completed with errors")
            print("=" * 60)
            sys.exit(1)
