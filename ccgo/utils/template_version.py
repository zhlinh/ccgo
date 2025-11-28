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

import re
import subprocess
from typing import List, Optional, Tuple


def parse_version(version_str: str) -> Optional[Tuple[int, int, int]]:
    """
    Parse a version string (e.g., '2.1.3', 'v2.1.3') into tuple (major, minor, patch).

    Args:
        version_str: Version string to parse

    Returns:
        Tuple of (major, minor, patch) or None if parsing fails
    """
    # Remove 'v' prefix if present
    version_str = version_str.lstrip('v')

    # Match semantic version pattern
    match = re.match(r'^(\d+)\.(\d+)\.(\d+)', version_str)
    if match:
        return (int(match.group(1)), int(match.group(2)), int(match.group(3)))
    return None


def get_git_tags(repo_url: str) -> List[str]:
    """
    Get all tags from a git repository.

    Args:
        repo_url: Git repository URL

    Returns:
        List of tag names
    """
    try:
        # Use git ls-remote to get tags without cloning
        result = subprocess.run(
            ['git', 'ls-remote', '--tags', repo_url],
            capture_output=True,
            text=True,
            check=True,
            timeout=30
        )

        tags = []
        for line in result.stdout.strip().split('\n'):
            if line and 'refs/tags/' in line:
                # Extract tag name from "hash refs/tags/tagname"
                tag = line.split('refs/tags/')[-1]
                # Skip peeled tags (^{})
                if not tag.endswith('^{}'):
                    tags.append(tag)

        return tags
    except subprocess.CalledProcessError as e:
        print(f"Error fetching tags from {repo_url}: {e}")
        return []
    except subprocess.TimeoutExpired:
        print(f"Timeout fetching tags from {repo_url}")
        return []


def find_matching_version(requested_version: str, repo_url: str) -> Optional[str]:
    """
    Find the best matching version tag based on requested version.

    Version matching rules:
    - 2.0.0 -> Find highest 2.x.x version
    - 2.1.0 -> Find highest 2.1.x version
    - 2.2.3 -> Find exact 2.2.3 version

    Args:
        requested_version: Version pattern to match (e.g., '2.0.0', '2.1.0')
        repo_url: Git repository URL

    Returns:
        Best matching tag name or None if no match found
    """
    # Parse requested version
    req_ver = parse_version(requested_version)
    if not req_ver:
        print(f"Invalid version format: {requested_version}")
        return None

    req_major, req_minor, req_patch = req_ver

    # Get all tags from repository
    all_tags = get_git_tags(repo_url)
    if not all_tags:
        print(f"No tags found in repository: {repo_url}")
        return None

    # Filter and find matching versions
    candidates = []
    for tag in all_tags:
        tag_ver = parse_version(tag)
        if not tag_ver:
            continue

        tag_major, tag_minor, tag_patch = tag_ver

        # Determine match level
        if req_patch == 0 and req_minor == 0:
            # Pattern: X.0.0 -> Match X.y.z (highest)
            if tag_major == req_major:
                candidates.append((tag_ver, tag))
        elif req_patch == 0:
            # Pattern: X.Y.0 -> Match X.Y.z (highest)
            if tag_major == req_major and tag_minor == req_minor:
                candidates.append((tag_ver, tag))
        else:
            # Pattern: X.Y.Z -> Exact match
            if tag_ver == req_ver:
                candidates.append((tag_ver, tag))

    if not candidates:
        print(f"No matching version found for {requested_version}")
        return None

    # Sort by version tuple (descending) and return highest match
    candidates.sort(reverse=True)
    best_match = candidates[0][1]

    return best_match


def resolve_template_version(
    repo_url: str,
    requested_version: Optional[str] = None,
    use_latest: bool = False
) -> Optional[str]:
    """
    Resolve template version to use.

    Args:
        repo_url: Template repository URL
        requested_version: Requested version pattern (e.g., '2.0.0', '2.1.0')
        use_latest: If True, use latest master/main branch

    Returns:
        Git ref to use (tag name, 'HEAD', or None if resolution fails)
    """
    # If use_latest is True, use HEAD (latest master/main)
    if use_latest:
        print(f"Using latest version from {repo_url}")
        return 'HEAD'

    # If no version specified, try to find latest stable tag
    if not requested_version:
        all_tags = get_git_tags(repo_url)
        if not all_tags:
            print("No tags found, falling back to HEAD")
            return 'HEAD'

        # Find highest semantic version tag
        versions = []
        for tag in all_tags:
            ver = parse_version(tag)
            if ver:
                versions.append((ver, tag))

        if not versions:
            print("No semantic version tags found, falling back to HEAD")
            return 'HEAD'

        versions.sort(reverse=True)
        latest_tag = versions[0][1]
        print(f"No version specified, using latest stable version: {latest_tag}")
        return latest_tag

    # Find matching version
    matched_tag = find_matching_version(requested_version, repo_url)
    if matched_tag:
        print(f"Resolved version {requested_version} -> {matched_tag}")
        return matched_tag

    # Fallback: try using requested version as exact tag name
    all_tags = get_git_tags(repo_url)
    if requested_version in all_tags or f'v{requested_version}' in all_tags:
        tag = requested_version if requested_version in all_tags else f'v{requested_version}'
        print(f"Using exact tag: {tag}")
        return tag

    print(f"Failed to resolve version: {requested_version}")
    return None
