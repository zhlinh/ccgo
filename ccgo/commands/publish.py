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


class Publish(CliCommand):
    def description(self) -> str:
        return """
        This is a subcommand to publish the library or documentation.

        Examples:
            ccgo publish android              # Publish to Maven repository
            ccgo publish ohos                 # Publish to OHPM repository
            ccgo publish kmp                  # Publish KMP library
            ccgo publish doc                  # Publish documentation to GitHub Pages
            ccgo publish doc --branch main    # Publish to specific branch
        """

    def get_target_list(self) -> list:
        return [
            "android",
            "ohos",
            "kmp",
            "doc",
            "ios",
            "windows",
            "linux",
            "macos",
            "tests",
            "benches",
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
        # Arguments for doc publishing
        parser.add_argument(
            "--branch",
            type=str,
            default=None,
            help="GitHub Pages branch name (default: from publish.pages_branch in CCGO.toml)",
        )
        parser.add_argument(
            "--force",
            action="store_true",
            help="Force push to remote repository (used with 'doc' target)",
        )
        parser.add_argument(
            "--open",
            action="store_true",
            help="Open documentation in browser after publishing (used with 'doc' target)",
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

            cmd = f"cd '{android_dir}' && ./gradlew publishMainPublicationToMavenRepository --no-daemon --info"
            err_code, err_msg = exec_command(cmd)
            if err_code != 0:
                print("\nEnd with error:")
                print(err_msg)
                sys.exit(err_code)
        elif args.target == "ohos":
            # OHOS: build HAR and publish to OHPM
            # Step 1: Build HAR file
            print("\n[Step 1/3] Building HAR file...")
            # Change to ohos directory to run hvigorw
            ohos_dir = os.path.join(os.getcwd(), "ohos")
            if not os.path.isdir(ohos_dir):
                print(f"\nError: ohos directory not found at {ohos_dir}")
                print("Please run this command from the project root directory")
                sys.exit(1)

            build_cmd = f"cd '{ohos_dir}' && hvigorw assembleHar --mode module -p product=default --no-daemon --no-parallel --info"
            err_code, err_msg = exec_command(build_cmd)
            if err_code != 0:
                print("\nBuild HAR failed with error:")
                print(err_msg)
                sys.exit(err_code)

            # Step 2: Copy HAR file to bin directory
            print("\n[Step 2/3] Copying HAR file to bin directory...")
            import glob
            import shutil

            # Find the HAR file in ohos build output
            har_search_path = os.path.join(
                ohos_dir, "main_ohos_sdk", "build", "default", "outputs", "default"
            )
            if not os.path.exists(har_search_path):
                print(f"\nError: HAR output directory not found: {har_search_path}")
                sys.exit(1)

            har_files_in_build = glob.glob(os.path.join(har_search_path, "*.har"))
            if not har_files_in_build:
                print(f"\nError: No HAR file found in {har_search_path}")
                sys.exit(1)

            # Create bin directory if not exists
            bin_dir = "bin"
            os.makedirs(bin_dir, exist_ok=True)

            # Copy HAR file to bin directory
            source_har = har_files_in_build[0]
            target_har = os.path.join(bin_dir, os.path.basename(source_har))
            shutil.copy2(source_har, target_har)
            print(f"Copied HAR file to {target_har}")

            # Step 3: Publish HAR file to OHPM
            print("\n[Step 3/3] Publishing HAR to OHPM...")
            print(f"Publishing {target_har}...")
            publish_cmd = f'ohpm publish "{target_har}"'
            err_code, err_msg = exec_command(publish_cmd)
            if err_code != 0:
                print("\nPublish to OHPM failed with error:")
                print(err_msg)
                sys.exit(err_code)

            print("\nSuccessfully published to OHPM!")
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

            # Use build_kmp.py script from project directory
            build_kmp_script = os.path.join(project_subdir, "build_kmp.py")

            if not os.path.isfile(build_kmp_script):
                print(f"ERROR: build_kmp.py not found at {build_kmp_script}")
                print("Please ensure your project has the KMP module configured")
                sys.exit(1)

            # Ask user whether to publish to local or remote Maven
            print("\nKMP Publish Options:")
            print("  1 - Publish to Maven Local (~/.m2/repository/)")
            print("  2 - Publish to Maven Remote (requires credentials)")
            choice = input("Select option (1 or 2): ").strip()

            if choice == "1":
                publish_flag = "--publish-local"
            elif choice == "2":
                publish_flag = "--publish-remote"
            else:
                print("Invalid option. Please select 1 or 2.")
                sys.exit(1)

            cmd = f"cd '{project_subdir}' && python3 '{build_kmp_script}' {publish_flag}"
            print(f"\nExecuting: {cmd}")

            err_code = os.system(cmd)
            if err_code != 0:
                sys.exit(err_code)

            print("\nSuccessfully published KMP library!")
        elif args.target == "doc":
            # Doc: publish documentation to GitHub Pages
            self.publish_doc(context, args)
        else:
            print(f"\nPublishing not yet supported for {args.target}")
            print("Currently supported: android, ohos, kmp, doc")
            sys.exit(1)

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
            pages_branch_name = args.branch or toml_data.get('publish', {}).get('pages_branch', 'gh-pages')
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
        if args.force:
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

                    if args.open:
                        import webbrowser
                        print(f"\nOpening {pages_url} in browser...")
                        webbrowser.open(pages_url)
        except:
            pass  # Ignore errors in URL extraction

        print()
