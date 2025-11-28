"""
Maven publisher for Android/JVM artifacts.

Handles the actual publishing process to Maven repositories.
"""

import os
import subprocess
import tempfile
from pathlib import Path
from typing import Optional, Dict, Any


class MavenPublisher:
    """Handle publishing artifacts to Maven repositories."""

    def __init__(self, config: 'MavenConfig', project_dir: str, verbose: bool = False):
        """
        Initialize Maven publisher.

        Args:
            config: MavenConfig instance
            project_dir: Root directory of the project
            verbose: Enable verbose output
        """
        self.config = config
        self.project_dir = project_dir
        self.verbose = verbose

        # Determine Android directory
        self.android_dir = os.path.join(project_dir, "android")

    def prepare_gradle_files(self) -> bool:
        """
        Prepare Gradle configuration files for publishing.

        Returns:
            True if preparation successful
        """
        try:
            # Generate gradle.properties
            gradle_props_path = os.path.join(self.android_dir, "gradle.properties")
            gradle_props_backup = None

            # Backup existing gradle.properties if it exists
            if os.path.exists(gradle_props_path):
                gradle_props_backup = gradle_props_path + ".backup"
                with open(gradle_props_path, 'r') as f:
                    original_content = f.read()
                with open(gradle_props_backup, 'w') as f:
                    f.write(original_content)

                if self.verbose:
                    print(f"Backed up existing gradle.properties to {gradle_props_backup}")

            # Write new gradle.properties
            gradle_props_content = self.config.generate_gradle_properties()
            with open(gradle_props_path, 'w') as f:
                f.write(gradle_props_content)

            if self.verbose:
                print(f"Generated gradle.properties at {gradle_props_path}")

            # Generate local.properties if needed
            local_props_path = os.path.join(self.android_dir, "local.properties")
            if not os.path.exists(local_props_path):
                local_props_content = self.config.generate_local_properties()
                if local_props_content:
                    with open(local_props_path, 'w') as f:
                        f.write(local_props_content)
                    if self.verbose:
                        print(f"Generated local.properties at {local_props_path}")

            return True

        except Exception as e:
            print(f"Error preparing Gradle files: {e}")
            return False

    def publish(self) -> bool:
        """
        Publish the artifact to Maven repository.

        Returns:
            True if publishing successful
        """
        # Validate configuration
        is_valid, error_msg = self.config.validate()
        if not is_valid:
            print(f"Configuration validation failed: {error_msg}")
            return False

        # Check if android directory exists
        if not os.path.isdir(self.android_dir):
            print(f"Error: Android directory not found at {self.android_dir}")
            print("Please run this command from the project root directory")
            return False

        # Prepare Gradle configuration files
        if not self.prepare_gradle_files():
            return False

        # Get the appropriate Gradle task
        gradle_task = self.config.get_gradle_task()

        # Build the Gradle command
        gradlew = "./gradlew" if os.name != 'nt' else "gradlew.bat"
        gradle_cmd = [gradlew, gradle_task, "--no-daemon"]

        if self.verbose:
            gradle_cmd.append("--info")

        # Add specific options for different repository types
        if self.config.repo_type == 'central':
            # For Maven Central, might need to close and release staging repo
            # This is typically handled by the gradle-maven-publish-plugin
            pass

        print(f"\nPublishing to {self.config.repo_type} repository...")
        print(self.config.get_config_summary())
        print()

        # Execute Gradle command
        try:
            result = subprocess.run(
                gradle_cmd,
                cwd=self.android_dir,
                capture_output=not self.verbose,
                text=True,
                check=False
            )

            if result.returncode == 0:
                print(f"✓ Successfully published to {self.config.repo_type} repository")

                if self.config.repo_type == 'local':
                    local_path = os.path.expanduser(
                        f"~/.m2/repository/{self.config.group_id.replace('.', '/')}"
                        f"/{self.config.artifact_id}/{self.config.version}"
                    )
                    print(f"  Artifacts available at: {local_path}")
                elif self.config.repo_type == 'central':
                    print(f"  Group ID: {self.config.group_id}")
                    print(f"  Artifact ID: {self.config.artifact_id}")
                    print(f"  Version: {self.config.version}")
                    print("\n  Note: For Maven Central, you may need to:")
                    print("  1. Log in to https://s01.oss.sonatype.org/")
                    print("  2. Close and release the staging repository")
                    print("  3. Wait for synchronization to Central (usually 10-30 minutes)")
                else:
                    print(f"  Published to: {self.config.repo_url}")
                    print(f"  Coordinates: {self.config.group_id}:{self.config.artifact_id}:{self.config.version}")

                return True
            else:
                print(f"✗ Publishing failed with exit code: {result.returncode}")
                if not self.verbose and result.stderr:
                    print(f"Error output:\n{result.stderr}")
                return False

        except FileNotFoundError:
            print(f"Error: Gradle wrapper not found at {self.android_dir}/{gradlew}")
            print("Please ensure the Android module is properly configured")
            return False
        except Exception as e:
            print(f"Error during publishing: {e}")
            return False

    def restore_gradle_files(self):
        """Restore original Gradle configuration files if backed up."""
        gradle_props_path = os.path.join(self.android_dir, "gradle.properties")
        gradle_props_backup = gradle_props_path + ".backup"

        if os.path.exists(gradle_props_backup):
            try:
                with open(gradle_props_backup, 'r') as f:
                    backup_content = f.read()
                with open(gradle_props_path, 'w') as f:
                    f.write(backup_content)
                os.remove(gradle_props_backup)

                if self.verbose:
                    print(f"Restored original gradle.properties")
            except Exception as e:
                print(f"Warning: Failed to restore gradle.properties: {e}")

    def verify_publication(self) -> bool:
        """
        Verify that the artifact was published successfully.

        Returns:
            True if verification successful
        """
        if self.config.repo_type == 'local':
            # Check if files exist in local Maven repository
            local_path = os.path.expanduser(
                f"~/.m2/repository/{self.config.group_id.replace('.', '/')}"
                f"/{self.config.artifact_id}/{self.config.version}"
            )

            expected_files = [
                f"{self.config.artifact_id}-{self.config.version}.aar",
                f"{self.config.artifact_id}-{self.config.version}.pom"
            ]

            all_exist = True
            for filename in expected_files:
                file_path = os.path.join(local_path, filename)
                if os.path.exists(file_path):
                    if self.verbose:
                        print(f"  ✓ Found: {filename}")
                else:
                    print(f"  ✗ Missing: {filename}")
                    all_exist = False

            return all_exist

        elif self.config.repo_type == 'central':
            # For Maven Central, we can check the staging repository
            print("  Note: Verification for Maven Central requires manual checking")
            print("  Visit: https://s01.oss.sonatype.org/")
            return True

        else:
            # For custom repositories, would need to make HTTP request to check
            print("  Note: Verification for custom repositories not implemented")
            return True


def publish_android(project_dir: str, config: Dict[str, Any],
                   repo_type: Optional[str] = None,
                   verbose: bool = False) -> bool:
    """
    Convenience function to publish Android artifacts.

    Args:
        project_dir: Root directory of the project
        config: Configuration dictionary from CCGO.toml
        repo_type: Override repository type (local/central/custom)
        verbose: Enable verbose output

    Returns:
        True if publishing successful
    """
    from .config import MavenConfig

    # Create Maven configuration
    maven_config = MavenConfig(config, platform='android')

    # Override repository type if specified
    if repo_type:
        maven_config.repo_type = repo_type

    # Create publisher
    publisher = MavenPublisher(maven_config, project_dir, verbose)

    try:
        # Publish
        success = publisher.publish()

        if success and verbose:
            # Verify publication
            publisher.verify_publication()

        return success

    finally:
        # Always try to restore original files
        publisher.restore_gradle_files()