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
import datetime

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
except ImportError:
    from utils.context.namespace import CliNameSpace
    from utils.context.context import CliContext
    from utils.context.command import CliCommand


class Tag(CliCommand):
    def description(self) -> str:
        return """
        This is a subcommand to create and manage Git tags.

        Tags are used to mark specific points in your repository's history,
        typically for releases.

        Examples:
            ccgo tag                              # Auto-generate tag from CCGO.toml
            ccgo tag v1.0.0                       # Create and push tag v1.0.0
            ccgo tag v1.0.0 --message "Release"   # With custom message
            ccgo tag v1.0.0 --no-push             # Create but don't push
            ccgo tag --delete v1.0.0              # Delete tag
            ccgo tag --list                       # List all tags
        """

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo tag",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )
        parser.add_argument(
            "version",
            nargs="?",
            default=None,
            help="Tag version (e.g., v1.0.0). If not provided, auto-generated from CCGO.toml",
        )
        parser.add_argument(
            "--message",
            "-m",
            type=str,
            default=None,
            help="Custom tag message (default: auto-generated with version info)",
        )
        parser.add_argument(
            "--lightweight",
            action="store_true",
            help="Create lightweight tag instead of annotated tag",
        )
        parser.add_argument(
            "--no-push",
            action="store_true",
            help="Create tag locally without pushing to remote",
        )
        parser.add_argument(
            "--force",
            "-f",
            action="store_true",
            help="Force create tag (replace if exists)",
        )
        parser.add_argument(
            "--delete",
            "-d",
            action="store_true",
            help="Delete specified tag (local and remote)",
        )
        parser.add_argument(
            "--local-only",
            action="store_true",
            help="When deleting, only delete local tag (used with --delete)",
        )
        parser.add_argument(
            "--list",
            "-l",
            action="store_true",
            help="List all tags",
        )
        parser.add_argument(
            "--remote",
            action="store_true",
            help="When listing, show remote tags (used with --list)",
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        # Handle list operation
        if args.list:
            self.list_tags(args)
            return

        # Handle delete operation
        if args.delete:
            if not args.version:
                print("ERROR: Version required for delete operation")
                print("Usage: ccgo tag --delete v1.0.0")
                sys.exit(1)
            self.delete_tag(args)
            return

        # Handle create operation
        self.create_tag(args)

    def get_project_info(self):
        """Get project information from CCGO.toml and git"""
        try:
            project_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            project_dir = os.environ.get('PWD')
            if not project_dir or not os.path.exists(project_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                sys.exit(1)

        # Check if CCGO.toml exists
        ccgo_toml_path = os.path.join(project_dir, "CCGO.toml")
        if not os.path.isfile(ccgo_toml_path):
            print("❌ ERROR: CCGO.toml not found in project root directory")
            print(f"Expected location: {ccgo_toml_path}")
            print("\nPlease run this command from the project root directory.")
            sys.exit(1)

        # Load CCGO.toml
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
            config['CONFIG_PROJECT_VERSION'] = toml_data.get('project', {}).get('version', '1.0.0')
            config['CONFIG_PROJECT_RELATIVE_PATH'] = toml_data.get('project', {}).get('name', 'sdk')
            config['CONFIG_PAGES_BRANCH_NAME'] = toml_data.get('publish', {}).get('pages_branch', 'gh-pages')
        except Exception as e:
            print(f"ERROR: Failed to load CCGO.toml: {e}")
            sys.exit(1)

        # Get git information
        git_info = {}
        try:
            git_info['branch'] = subprocess.check_output(
                ["git", "symbolic-ref", "--short", "-q", "HEAD"],
                cwd=project_dir,
                stderr=subprocess.DEVNULL
            ).decode().strip()
        except subprocess.CalledProcessError:
            git_info['branch'] = "unknown"

        try:
            git_info['version_code'] = subprocess.check_output(
                ["git", "rev-list", "HEAD", "--count"],
                cwd=project_dir,
                stderr=subprocess.DEVNULL
            ).decode().strip()
        except subprocess.CalledProcessError:
            git_info['version_code'] = "0"

        try:
            git_info['revision'] = subprocess.check_output(
                ["git", "rev-parse", "--short", "HEAD"],
                cwd=project_dir,
                stderr=subprocess.DEVNULL
            ).decode().strip()
        except subprocess.CalledProcessError:
            git_info['revision'] = "unknown"

        try:
            timestamp = subprocess.check_output(
                ["git", "log", "-n1", "--format=%at"],
                cwd=project_dir,
                stderr=subprocess.DEVNULL
            ).decode().strip()
            git_info['datetime'] = datetime.datetime.fromtimestamp(int(timestamp)).strftime("%Y-%m-%d %H:%M:%S")
        except:
            git_info['datetime'] = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        return {
            'config': config,
            'git': git_info,
            'project_dir': project_dir
        }

    def generate_tag_message(self, version, info):
        """Generate default tag message"""
        project_name = info['config'].get('CONFIG_PROJECT_NAME', 'Project')

        message = f"{version} Release\n\n"
        message += f"Project: {project_name}\n"
        message += f"VERSION: {version}\n"
        message += f"VERSION_CODE: {info['git']['version_code']}\n"
        message += f"REVISION: {info['git']['revision']}\n"
        message += f"BRANCH: {info['git']['branch']}\n"
        message += f"DATETIME: {info['git']['datetime']}\n"

        return message

    def create_tag(self, args):
        """Create a Git tag"""
        print("\n📌 Creating Git tag...\n")

        # Get project info
        info = self.get_project_info()
        project_dir = info['project_dir']

        # Determine tag version
        if args.version:
            tag_version = args.version
        else:
            # Auto-generate from CCGO.toml
            config_version = info['config'].get('CONFIG_PROJECT_VERSION')
            if not config_version:
                print("ERROR: CONFIG_PROJECT_VERSION not found in CCGO.toml")
                print("Please specify version explicitly: ccgo tag v1.0.0")
                sys.exit(1)
            tag_version = f"v{config_version}" if not config_version.startswith('v') else config_version

        print(f"Tag version: {tag_version}")

        # Generate tag message
        if args.message:
            tag_message = args.message
        else:
            tag_message = self.generate_tag_message(tag_version, info)

        if not args.lightweight:
            print(f"\nTag message:\n{'-'*60}\n{tag_message}\n{'-'*60}\n")

        # Check if tag already exists
        try:
            subprocess.run(
                ["git", "rev-parse", tag_version],
                cwd=project_dir,
                check=True,
                capture_output=True
            )
            # Tag exists
            if not args.force:
                print(f"ERROR: Tag '{tag_version}' already exists")
                print("Use --force to replace it")
                sys.exit(1)
            else:
                print(f"⚠️  Tag '{tag_version}' already exists, will be replaced (--force)")
                # Delete existing tag
                try:
                    subprocess.run(
                        ["git", "tag", "-d", tag_version],
                        cwd=project_dir,
                        check=True,
                        capture_output=True
                    )
                except:
                    pass
        except subprocess.CalledProcessError:
            # Tag doesn't exist, that's fine
            pass

        # Create tag
        try:
            if args.lightweight:
                # Lightweight tag
                cmd = ["git", "tag", tag_version]
                if args.force:
                    cmd.append("-f")
                subprocess.run(cmd, cwd=project_dir, check=True)
                print(f"✓ Created lightweight tag: {tag_version}")
            else:
                # Annotated tag
                cmd = ["git", "tag", "-a", tag_version, "-m", tag_message]
                if args.force:
                    cmd.append("-f")
                subprocess.run(cmd, cwd=project_dir, check=True)
                print(f"✓ Created annotated tag: {tag_version}")
        except subprocess.CalledProcessError as e:
            print(f"\nERROR: Failed to create tag: {e}")
            sys.exit(1)

        # Show tag info
        try:
            result = subprocess.run(
                ["git", "show", tag_version, "--no-patch"],
                cwd=project_dir,
                capture_output=True,
                text=True
            )
            if result.returncode == 0:
                print(f"\nTag info:")
                print(result.stdout)
        except:
            pass

        # Push to remote
        if not args.no_push:
            print(f"Pushing tag to remote...")
            try:
                push_cmd = ["git", "push", "origin", tag_version]
                if args.force:
                    push_cmd.append("-f")
                subprocess.run(push_cmd, cwd=project_dir, check=True)
                print(f"✓ Pushed tag to origin/{tag_version}")
            except subprocess.CalledProcessError as e:
                print(f"\n⚠️  WARNING: Failed to push tag to remote: {e}")
                print(f"Tag created locally. You can push it manually:")
                print(f"  git push origin {tag_version}")
        else:
            print(f"\n📝 Tag created locally (--no-push)")
            print(f"To push it later, run:")
            print(f"  git push origin {tag_version}")

        print("\n" + "="*60)
        print("✅ Tag operation completed successfully!")
        print("="*60 + "\n")

    def delete_tag(self, args):
        """Delete a Git tag"""
        tag_version = args.version

        print(f"\n🗑️  Deleting tag: {tag_version}\n")

        # Get project dir
        try:
            project_dir = os.getcwd()
        except:
            project_dir = os.environ.get('PWD', '.')

        # Delete local tag
        try:
            subprocess.run(
                ["git", "tag", "-d", tag_version],
                cwd=project_dir,
                check=True
            )
            print(f"✓ Deleted local tag: {tag_version}")
        except subprocess.CalledProcessError as e:
            print(f"⚠️  Local tag '{tag_version}' not found or already deleted")

        # Delete remote tag
        if not args.local_only:
            try:
                subprocess.run(
                    ["git", "push", "origin", "--delete", tag_version],
                    cwd=project_dir,
                    check=True
                )
                print(f"✓ Deleted remote tag: origin/{tag_version}")
            except subprocess.CalledProcessError as e:
                print(f"⚠️  Failed to delete remote tag (may not exist)")

        print("\n✅ Tag deletion completed\n")

    def list_tags(self, args):
        """List Git tags"""
        # Get project dir
        try:
            project_dir = os.getcwd()
        except:
            project_dir = os.environ.get('PWD', '.')

        if args.remote:
            print("\n📋 Remote tags:\n")
            try:
                result = subprocess.run(
                    ["git", "ls-remote", "--tags", "origin"],
                    cwd=project_dir,
                    capture_output=True,
                    text=True,
                    check=True
                )
                if result.stdout:
                    lines = result.stdout.strip().split('\n')
                    for line in lines:
                        parts = line.split('\t')
                        if len(parts) >= 2:
                            tag = parts[1].replace('refs/tags/', '')
                            if not tag.endswith('^{}'):  # Skip dereferenced tags
                                print(f"  {tag}")
                else:
                    print("  No remote tags found")
            except subprocess.CalledProcessError as e:
                print(f"ERROR: Failed to list remote tags: {e}")
                sys.exit(1)
        else:
            print("\n📋 Local tags:\n")
            try:
                result = subprocess.run(
                    ["git", "tag", "-l"],
                    cwd=project_dir,
                    capture_output=True,
                    text=True,
                    check=True
                )
                if result.stdout:
                    tags = result.stdout.strip().split('\n')
                    for tag in sorted(tags):
                        if tag:
                            print(f"  {tag}")
                else:
                    print("  No local tags found")
            except subprocess.CalledProcessError as e:
                print(f"ERROR: Failed to list tags: {e}")
                sys.exit(1)

        print()
