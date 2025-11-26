#!/usr/bin/env python3
#
# Copyright 2024 ccgo Project. All rights reserved.
# Use of this source code is governed by a MIT-style
# license that can be found at
#
# https://opensource.org/license/MIT
#
# The above copyright notice and this permission
# notice shall be included in all copies or
# substantial portions of the Software.

"""
Dependency Manager for CCGO

Handles dependency resolution and fetching for CCGO projects, similar to Cargo.
Supports:
- Git dependencies (with branch, tag, rev specifications)
- Path dependencies (local and relative)
- Platform-specific dependencies
"""

import os
import sys
import subprocess
import hashlib
import json
import shutil
from typing import Dict, List, Optional, Any, Tuple
from pathlib import Path


class DependencyError(Exception):
    """Exception raised for dependency-related errors"""
    pass


class Dependency:
    """Represents a single dependency"""

    def __init__(self, name: str, spec: Any):
        """
        Initialize a dependency.

        Args:
            name: Dependency name
            spec: Dependency specification (string, dict, or other)
        """
        self.name = name
        self.spec = spec
        self.dep_type = self._determine_type()
        self.version = None
        self.resolved_path = None

    def _determine_type(self) -> str:
        """Determine the type of dependency based on spec"""
        if isinstance(self.spec, str):
            # Simple version string like "1.0.0"
            self.version = self.spec
            return "version"
        elif isinstance(self.spec, dict):
            if "git" in self.spec:
                return "git"
            elif "path" in self.spec:
                return "path"
            else:
                raise DependencyError(f"Unknown dependency specification for '{self.name}': {self.spec}")
        else:
            raise DependencyError(f"Invalid dependency specification type for '{self.name}': {type(self.spec)}")

    def __repr__(self):
        return f"Dependency(name={self.name}, type={self.dep_type}, spec={self.spec})"


class DependencyManager:
    """Manages dependencies for a CCGO project"""

    def __init__(self, project_dir: str, dependencies_dir: str = None):
        """
        Initialize the dependency manager.

        Args:
            project_dir: Root directory of the project
            dependencies_dir: Directory to store downloaded dependencies (default: project_dir/third_party)
        """
        self.project_dir = Path(project_dir).resolve()
        self.dependencies_dir = Path(dependencies_dir) if dependencies_dir else self.project_dir / "third_party"
        self.dependencies_dir.mkdir(parents=True, exist_ok=True)

        # Lock file for tracking resolved dependencies
        self.lock_file = self.project_dir / "CCGO.lock"
        self.lock_data = self._load_lock_file()

        # Cache directory for git repositories
        self.cache_dir = self.dependencies_dir / ".cache"
        self.cache_dir.mkdir(parents=True, exist_ok=True)

    def _load_lock_file(self) -> Dict:
        """Load the lock file if it exists"""
        if self.lock_file.exists():
            try:
                with open(self.lock_file, 'r') as f:
                    return json.load(f)
            except Exception as e:
                print(f"   ⚠️  Warning: Failed to load CCGO.lock: {e}")
                return {}
        return {}

    def _save_lock_file(self):
        """Save the current lock data to file"""
        try:
            with open(self.lock_file, 'w') as f:
                json.dump(self.lock_data, f, indent=2)
        except Exception as e:
            print(f"   ⚠️  Warning: Failed to save CCGO.lock: {e}")

    def _run_git_command(self, args: List[str], cwd: str = None) -> Tuple[int, str, str]:
        """
        Run a git command.

        Args:
            args: Git command arguments (without 'git' prefix)
            cwd: Working directory

        Returns:
            Tuple of (return_code, stdout, stderr)
        """
        cmd = ["git"] + args
        try:
            result = subprocess.run(
                cmd,
                cwd=cwd,
                capture_output=True,
                text=True,
                timeout=300  # 5 minute timeout
            )
            return result.returncode, result.stdout, result.stderr
        except subprocess.TimeoutExpired:
            raise DependencyError(f"Git command timed out: {' '.join(cmd)}")
        except Exception as e:
            raise DependencyError(f"Failed to run git command: {e}")

    def _get_git_commit_hash(self, repo_path: str, ref: str = "HEAD") -> str:
        """Get the commit hash for a given ref"""
        returncode, stdout, stderr = self._run_git_command(
            ["rev-parse", ref],
            cwd=repo_path
        )
        if returncode != 0:
            raise DependencyError(f"Failed to get commit hash for {ref}: {stderr}")
        return stdout.strip()

    def _clone_or_update_git_repo(self, git_url: str, target_dir: Path) -> str:
        """
        Clone or update a git repository.

        Args:
            git_url: Git repository URL
            target_dir: Target directory for the repository

        Returns:
            Path to the repository
        """
        if target_dir.exists():
            # Repository exists, try to update
            print(f"   📦 Updating git repository: {git_url}")
            returncode, stdout, stderr = self._run_git_command(
                ["fetch", "--all", "--tags"],
                cwd=str(target_dir)
            )
            if returncode != 0:
                print(f"   ⚠️  Warning: Failed to update repository: {stderr}")
        else:
            # Clone repository
            print(f"   📦 Cloning git repository: {git_url}")
            target_dir.parent.mkdir(parents=True, exist_ok=True)
            returncode, stdout, stderr = self._run_git_command(
                ["clone", git_url, str(target_dir)]
            )
            if returncode != 0:
                raise DependencyError(f"Failed to clone repository {git_url}: {stderr}")

        return str(target_dir)

    def _checkout_git_ref(self, repo_path: str, ref: str):
        """Checkout a specific git reference"""
        print(f"   📦 Checking out ref: {ref}")
        returncode, stdout, stderr = self._run_git_command(
            ["checkout", ref],
            cwd=repo_path
        )
        if returncode != 0:
            raise DependencyError(f"Failed to checkout {ref}: {stderr}")

    def resolve_git_dependency(self, dep: Dependency) -> str:
        """
        Resolve a git dependency.

        Args:
            dep: Dependency object with git specification

        Returns:
            Path to the resolved dependency
        """
        git_url = dep.spec["git"]
        branch = dep.spec.get("branch")
        tag = dep.spec.get("tag")
        rev = dep.spec.get("rev")

        # Create a unique directory name based on the git URL
        repo_hash = hashlib.md5(git_url.encode()).hexdigest()[:8]
        cache_repo_dir = self.cache_dir / f"{dep.name}_{repo_hash}"

        # Clone or update the repository
        repo_path = self._clone_or_update_git_repo(git_url, cache_repo_dir)

        # Determine which ref to checkout
        if rev:
            # Specific commit/rev (includes PR refs like refs/pull/493/head)
            checkout_ref = rev
        elif tag:
            # Specific tag
            checkout_ref = f"tags/{tag}"
        elif branch:
            # Specific branch
            checkout_ref = f"origin/{branch}"
        else:
            # Default to main/master branch HEAD
            # Try to detect the default branch
            returncode, stdout, stderr = self._run_git_command(
                ["symbolic-ref", "refs/remotes/origin/HEAD"],
                cwd=repo_path
            )
            if returncode == 0:
                default_branch = stdout.strip().replace("refs/remotes/origin/", "")
                checkout_ref = f"origin/{default_branch}"
            else:
                # Fallback to master or main
                checkout_ref = "origin/main"

        # Checkout the specified ref
        self._checkout_git_ref(repo_path, checkout_ref)

        # Get the commit hash for lock file
        commit_hash = self._get_git_commit_hash(repo_path)

        # Create a symlink or copy to the dependencies directory
        target_dir = self.dependencies_dir / dep.name
        if target_dir.exists():
            if target_dir.is_symlink():
                target_dir.unlink()
            else:
                shutil.rmtree(target_dir)

        # Create symlink (preferred) or copy
        try:
            target_dir.symlink_to(cache_repo_dir, target_is_directory=True)
        except OSError:
            # Symlinks not supported, copy instead
            shutil.copytree(cache_repo_dir, target_dir)

        # Update lock file
        self.lock_data[dep.name] = {
            "type": "git",
            "git": git_url,
            "commit": commit_hash,
            "path": str(target_dir)
        }
        if branch:
            self.lock_data[dep.name]["branch"] = branch
        if tag:
            self.lock_data[dep.name]["tag"] = tag
        if rev:
            self.lock_data[dep.name]["rev"] = rev

        dep.resolved_path = str(target_dir)
        return str(target_dir)

    def resolve_path_dependency(self, dep: Dependency) -> str:
        """
        Resolve a path dependency.

        Args:
            dep: Dependency object with path specification

        Returns:
            Absolute path to the dependency
        """
        path_spec = dep.spec["path"]
        version = dep.spec.get("version")

        # Resolve the path relative to project directory
        dep_path = Path(path_spec)
        if not dep_path.is_absolute():
            dep_path = self.project_dir / dep_path

        dep_path = dep_path.resolve()

        if not dep_path.exists():
            raise DependencyError(f"Path dependency not found: {path_spec} (resolved to {dep_path})")

        # Update lock file
        self.lock_data[dep.name] = {
            "type": "path",
            "path": str(dep_path)
        }
        if version:
            self.lock_data[dep.name]["version"] = version

        dep.resolved_path = str(dep_path)
        return str(dep_path)

    def resolve_dependency(self, dep: Dependency) -> str:
        """
        Resolve a dependency based on its type.

        Args:
            dep: Dependency object

        Returns:
            Path to the resolved dependency
        """
        print(f"   🔍 Resolving dependency: {dep.name} ({dep.dep_type})")

        if dep.dep_type == "git":
            return self.resolve_git_dependency(dep)
        elif dep.dep_type == "path":
            return self.resolve_path_dependency(dep)
        elif dep.dep_type == "version":
            # Version dependencies would require a registry (like crates.io)
            # For now, we don't support this
            raise DependencyError(
                f"Version-based dependencies are not yet supported. "
                f"Please use git or path dependencies for '{dep.name}'"
            )
        else:
            raise DependencyError(f"Unknown dependency type: {dep.dep_type}")

    def resolve_all_dependencies(self, dependencies: Dict[str, Any]) -> Dict[str, str]:
        """
        Resolve all dependencies.

        Args:
            dependencies: Dictionary of dependency name to specification

        Returns:
            Dictionary mapping dependency name to resolved path
        """
        resolved = {}

        for name, spec in dependencies.items():
            dep = Dependency(name, spec)
            try:
                resolved_path = self.resolve_dependency(dep)
                resolved[name] = resolved_path
                print(f"   ✅ Resolved {name} -> {resolved_path}")
            except DependencyError as e:
                print(f"   ❌ Failed to resolve {name}: {e}")
                raise

        # Save lock file
        self._save_lock_file()

        return resolved

    def get_cmake_dependencies_list(self, resolved_deps: Dict[str, str]) -> List[str]:
        """
        Get a list of dependency paths for CMake.

        Args:
            resolved_deps: Dictionary of resolved dependencies

        Returns:
            List of absolute paths to dependencies
        """
        return list(resolved_deps.values())

    def get_cmake_include_dirs(self, resolved_deps: Dict[str, str]) -> List[str]:
        """
        Get include directories for all dependencies.

        Args:
            resolved_deps: Dictionary of resolved dependencies

        Returns:
            List of include directory paths
        """
        include_dirs = []
        for name, path in resolved_deps.items():
            dep_path = Path(path)

            # Common include directory patterns
            potential_includes = [
                dep_path / "include",
                dep_path / "src",
                dep_path,  # Some libraries put headers in root
            ]

            for inc_dir in potential_includes:
                if inc_dir.exists() and inc_dir.is_dir():
                    include_dirs.append(str(inc_dir))
                    break

        return include_dirs


def parse_dependencies_from_toml(toml_data: Dict) -> Tuple[Dict[str, Any], Dict[str, Dict[str, Any]]]:
    """
    Parse dependencies from TOML data.

    Args:
        toml_data: Parsed TOML data

    Returns:
        Tuple of (common_dependencies, platform_specific_dependencies)
    """
    # Parse common dependencies
    common_deps = toml_data.get("dependencies", {})

    # Parse platform-specific dependencies
    platform_deps = {}
    for key in toml_data.keys():
        if key.startswith("target."):
            # Extract platform config like target.'cfg(windows)'
            target_cfg = key.replace("target.", "").strip("'\"")
            target_data = toml_data[key]
            if "dependencies" in target_data:
                platform_deps[target_cfg] = target_data["dependencies"]

    return common_deps, platform_deps


def should_include_platform_dependencies(platform_cfg: str, current_platform: str) -> bool:
    """
    Determine if platform-specific dependencies should be included.

    Args:
        platform_cfg: Platform configuration string (e.g., "cfg(windows)")
        current_platform: Current platform name (e.g., "windows", "linux", "android")

    Returns:
        True if dependencies should be included
    """
    # Simple platform matching
    # This can be extended to support more complex cfg expressions

    platform_cfg = platform_cfg.lower()
    current_platform = current_platform.lower()

    # Direct platform matches
    if current_platform in platform_cfg:
        return True

    # Unix platforms
    if "cfg(unix)" in platform_cfg and current_platform in ["linux", "macos", "ios"]:
        return True

    # Architecture matches
    import platform as py_platform
    arch = py_platform.machine().lower()
    if "x86" in platform_cfg and arch in ["x86_64", "amd64", "i686", "i386"]:
        return True
    if "arm" in platform_cfg and "arm" in arch:
        return True

    return False
