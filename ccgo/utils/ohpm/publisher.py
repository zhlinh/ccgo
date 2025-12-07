"""
OHPM publisher for OHOS/OpenHarmony artifacts.

Handles the actual publishing process to OHPM registries.
"""

import os
import subprocess
import shutil
import glob
import json
from pathlib import Path
from typing import Optional, Dict, Any


class OhpmPublisher:
    """Handle publishing artifacts to OHPM registries."""

    def __init__(self, config: 'OhpmConfig', project_dir: str, verbose: bool = False):
        """
        Initialize OHPM publisher.

        Args:
            config: OhpmConfig instance
            project_dir: Root directory of the project
            verbose: Enable verbose output
        """
        self.config = config
        self.project_dir = project_dir
        self.verbose = verbose

        # Determine OHOS directory
        self.ohos_dir = os.path.join(project_dir, "ohos")

    def prepare_package_files(self) -> bool:
        """
        Prepare oh-package.json5 file for publishing.

        Returns:
            True if preparation successful
        """
        try:
            # Generate oh-package.json5 if needed
            oh_package_path = os.path.join(self.ohos_dir, "main_ohos_sdk", "oh-package.json5")
            oh_package_backup = None

            # Check if oh-package.json5 exists
            if os.path.exists(oh_package_path):
                # Backup existing file
                oh_package_backup = oh_package_path + ".backup"
                with open(oh_package_path, 'r') as f:
                    original_content = f.read()
                with open(oh_package_backup, 'w') as f:
                    f.write(original_content)

                if self.verbose:
                    print(f"Backed up existing oh-package.json5 to {oh_package_backup}")

                # Update version and other fields in existing file
                try:
                    # Read and parse existing JSON5 (simplified parsing)
                    with open(oh_package_path, 'r') as f:
                        content = f.read()

                    # Update version (simple regex replacement)
                    import re
                    content = re.sub(
                        r'"version"\s*:\s*"[^"]*"',
                        f'"version": "{self.config.version}"',
                        content
                    )

                    # Update package name if organization is specified
                    if self.config.organization:
                        full_name = f"@{self.config.organization}/{self.config.package_name}"
                        content = re.sub(
                            r'"name"\s*:\s*"[^"]*"',
                            f'"name": "{full_name}"',
                            content
                        )

                    # Write updated content
                    with open(oh_package_path, 'w') as f:
                        f.write(content)

                    if self.verbose:
                        print(f"Updated oh-package.json5 with version {self.config.version}")

                except Exception as e:
                    print(f"Warning: Could not update existing oh-package.json5: {e}")
                    # Restore original and generate new one
                    if oh_package_backup:
                        shutil.copy2(oh_package_backup, oh_package_path)
                    # Generate new file
                    package_content = self.config.generate_oh_package_json5()
                    with open(oh_package_path, 'w') as f:
                        f.write(package_content)
            else:
                # Generate new oh-package.json5
                package_content = self.config.generate_oh_package_json5()

                # Ensure directory exists
                os.makedirs(os.path.dirname(oh_package_path), exist_ok=True)

                with open(oh_package_path, 'w') as f:
                    f.write(package_content)

                if self.verbose:
                    print(f"Generated oh-package.json5 at {oh_package_path}")

            return True

        except Exception as e:
            print(f"Error preparing package files: {e}")
            return False

    def build_har(self) -> Optional[str]:
        """
        Build HAR file using hvigorw.

        Returns:
            Path to the built HAR file, or None if build failed
        """
        if not os.path.isdir(self.ohos_dir):
            print(f"Error: OHOS directory not found at {self.ohos_dir}")
            print("Please run this command from the project root directory")
            return None

        print("[Step 1/3] Building HAR file...")

        # Build command
        build_cmd = ["hvigorw", "assembleHar", "--mode", "module",
                    "-p", "product=default", "--no-daemon", "--no-parallel"]

        if self.verbose:
            build_cmd.append("--info")

        try:
            result = subprocess.run(
                build_cmd,
                cwd=self.ohos_dir,
                capture_output=not self.verbose,
                text=True,
                check=False
            )

            if result.returncode != 0:
                print(f"Build HAR failed with exit code: {result.returncode}")
                if not self.verbose and result.stderr:
                    print(f"Error output:\n{result.stderr}")
                return None

            # Find the built HAR file
            har_search_path = os.path.join(
                self.ohos_dir, "main_ohos_sdk", "build", "default", "outputs", "default"
            )

            if not os.path.exists(har_search_path):
                print(f"Error: HAR output directory not found: {har_search_path}")
                return None

            har_files = glob.glob(os.path.join(har_search_path, "*.har"))
            if not har_files:
                print(f"Error: No HAR file found in {har_search_path}")
                return None

            har_file = har_files[0]  # Take the first (should be only one)
            print(f"✓ Built HAR file: {os.path.basename(har_file)}")
            return har_file

        except FileNotFoundError:
            print("Error: hvigorw not found. Please ensure OHOS development environment is set up")
            return None
        except Exception as e:
            print(f"Error during HAR build: {e}")
            return None

    def copy_to_bin(self, har_file: str) -> Optional[str]:
        """
        Copy HAR file to bin directory.

        Args:
            har_file: Path to the HAR file

        Returns:
            Path to the copied HAR file in bin directory
        """
        print("[Step 2/3] Copying HAR file to bin directory...")

        # Create bin directory if it doesn't exist
        bin_dir = os.path.join(self.project_dir, "bin")
        os.makedirs(bin_dir, exist_ok=True)

        # Copy HAR file
        target_har = os.path.join(bin_dir, os.path.basename(har_file))

        try:
            shutil.copy2(har_file, target_har)
            print(f"✓ Copied HAR file to {target_har}")
            return target_har
        except Exception as e:
            print(f"Error copying HAR file: {e}")
            return None

    def publish(self) -> bool:
        """
        Publish the HAR artifact to OHPM registry.

        Returns:
            True if publishing successful
        """
        # Validate configuration
        is_valid, error_msg = self.config.validate()
        if not is_valid:
            print(f"Configuration validation failed: {error_msg}")
            return False

        # Prepare package files
        if not self.prepare_package_files():
            return False

        # Build HAR file
        har_file = self.build_har()
        if not har_file:
            return False

        # Copy to bin directory
        target_har = self.copy_to_bin(har_file)
        if not target_har:
            return False

        # Set up authentication if needed
        print("\n[Step 3/3] Publishing HAR to OHPM...")

        if self.config.credentials.get('token') or (
            self.config.credentials.get('username') and self.config.credentials.get('password')
        ):
            print("Setting up OHPM authentication...")
            if not self.config.setup_ohpm_auth():
                print("Warning: Authentication setup may have failed")

        # Publish using ohpm
        print(f"Publishing {os.path.basename(target_har)}...")
        print(self.config.get_config_summary())
        print()

        # Build ohpm publish command
        publish_cmd = ["ohpm", "publish", target_har]

        # Add additional arguments
        cmd_args = self.config.get_ohpm_command_args()
        if cmd_args:
            publish_cmd.extend(cmd_args)

        if self.verbose:
            print(f"Executing: {' '.join(publish_cmd)}")

        try:
            result = subprocess.run(
                publish_cmd,
                capture_output=not self.verbose,
                text=True,
                check=False
            )

            if result.returncode == 0:
                print(f"✓ Successfully published to {self.config.registry_type} registry")

                if self.config.organization:
                    print(f"  Package: @{self.config.organization}/{self.config.package_name}")
                else:
                    print(f"  Package: {self.config.package_name}")
                print(f"  Version: {self.config.version}")

                if self.config.registry_type == 'official':
                    print(f"\n  View at: https://ohpm.openharmony.cn/package/{self.config.package_name}")
                else:
                    print(f"  Registry: {self.config.get_registry_url()}")

                return True
            else:
                print(f"✗ Publishing failed with exit code: {result.returncode}")
                if not self.verbose and result.stderr:
                    print(f"Error output:\n{result.stderr}")
                return False

        except FileNotFoundError:
            print("Error: ohpm command not found.")
            print("Please install OHPM: npm install -g @ohos/ohpm-cli")
            return False
        except Exception as e:
            print(f"Error during publishing: {e}")
            return False

    def restore_package_files(self):
        """Restore original oh-package.json5 if backed up."""
        oh_package_path = os.path.join(self.ohos_dir, "main_ohos_sdk", "oh-package.json5")
        oh_package_backup = oh_package_path + ".backup"

        if os.path.exists(oh_package_backup):
            try:
                with open(oh_package_backup, 'r') as f:
                    backup_content = f.read()
                with open(oh_package_path, 'w') as f:
                    f.write(backup_content)
                os.remove(oh_package_backup)

                if self.verbose:
                    print(f"Restored original oh-package.json5")
            except Exception as e:
                print(f"Warning: Failed to restore oh-package.json5: {e}")

    def verify_publication(self) -> bool:
        """
        Verify that the package was published successfully.

        Returns:
            True if verification successful
        """
        # Check if package is available in registry
        package_name = self.config.package_name
        if self.config.organization:
            package_name = f"@{self.config.organization}/{package_name}"

        print(f"Verifying publication of {package_name}@{self.config.version}...")

        # Build ohpm view command
        view_cmd = ["ohpm", "view", f"{package_name}@{self.config.version}"]

        if self.config.registry_type != 'official':
            view_cmd.extend(['--registry', self.config.get_registry_url()])

        try:
            result = subprocess.run(
                view_cmd,
                capture_output=True,
                text=True,
                check=False
            )

            if result.returncode == 0:
                print(f"✓ Package {package_name}@{self.config.version} is available in registry")
                if self.verbose:
                    print(result.stdout)
                return True
            else:
                print(f"✗ Package not found or not accessible")
                return False

        except Exception as e:
            print(f"Error during verification: {e}")
            return False


def publish_ohos(project_dir: str, config: Dict[str, Any],
                registry_type: Optional[str] = None,
                verbose: bool = False) -> bool:
    """
    Convenience function to publish OHOS artifacts.

    Args:
        project_dir: Root directory of the project
        config: Configuration dictionary from CCGO.toml
        registry_type: Override registry type (official/private/local)
        verbose: Enable verbose output

    Returns:
        True if publishing successful
    """
    from .config import OhpmConfig

    # Create OHPM configuration
    ohpm_config = OhpmConfig(config)

    # Override registry type if specified
    if registry_type:
        ohpm_config.registry_type = registry_type

    # Create publisher
    publisher = OhpmPublisher(ohpm_config, project_dir, verbose)

    try:
        # Publish
        success = publisher.publish()

        if success and verbose:
            # Verify publication
            publisher.verify_publication()

        return success

    finally:
        # Always try to restore original files
        publisher.restore_package_files()