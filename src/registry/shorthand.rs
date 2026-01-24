//! Git URL shorthand syntax support
//!
//! Supports shorthand notations for common Git hosting providers:
//! - `github:user/repo` -> `https://github.com/user/repo.git`
//! - `gitlab:user/repo` -> `https://gitlab.com/user/repo.git`
//! - `bitbucket:user/repo` -> `https://bitbucket.org/user/repo.git`
//! - `gitee:user/repo` -> `https://gitee.com/user/repo.git`
//!
//! Also supports explicit full URLs.

use anyhow::{bail, Result};
use std::fmt;

/// Supported Git hosting providers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitProvider {
    GitHub,
    GitLab,
    Bitbucket,
    Gitee,
    Custom,
}

impl GitProvider {
    /// Get the base URL for this provider
    pub fn base_url(&self) -> &'static str {
        match self {
            GitProvider::GitHub => "https://github.com",
            GitProvider::GitLab => "https://gitlab.com",
            GitProvider::Bitbucket => "https://bitbucket.org",
            GitProvider::Gitee => "https://gitee.com",
            GitProvider::Custom => "",
        }
    }

    /// Get the provider from a shorthand prefix
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix.to_lowercase().as_str() {
            "github" | "gh" => Some(GitProvider::GitHub),
            "gitlab" | "gl" => Some(GitProvider::GitLab),
            "bitbucket" | "bb" => Some(GitProvider::Bitbucket),
            "gitee" => Some(GitProvider::Gitee),
            _ => None,
        }
    }

    /// Detect provider from a full Git URL
    pub fn from_url(url: &str) -> Self {
        let url_lower = url.to_lowercase();
        if url_lower.contains("github.com") {
            GitProvider::GitHub
        } else if url_lower.contains("gitlab.com") {
            GitProvider::GitLab
        } else if url_lower.contains("bitbucket.org") {
            GitProvider::Bitbucket
        } else if url_lower.contains("gitee.com") {
            GitProvider::Gitee
        } else {
            GitProvider::Custom
        }
    }
}

impl fmt::Display for GitProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitProvider::GitHub => write!(f, "github"),
            GitProvider::GitLab => write!(f, "gitlab"),
            GitProvider::Bitbucket => write!(f, "bitbucket"),
            GitProvider::Gitee => write!(f, "gitee"),
            GitProvider::Custom => write!(f, "custom"),
        }
    }
}

/// Parsed shorthand specification
#[derive(Debug, Clone)]
pub struct ShorthandSpec {
    /// Git provider
    pub provider: GitProvider,
    /// Owner/user name
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Optional reference (tag, branch, or commit)
    pub reference: Option<String>,
    /// Expanded full URL
    pub url: String,
}

impl ShorthandSpec {
    /// Get SSH URL for this repository
    pub fn ssh_url(&self) -> String {
        match self.provider {
            GitProvider::GitHub => format!("git@github.com:{}/{}.git", self.owner, self.repo),
            GitProvider::GitLab => format!("git@gitlab.com:{}/{}.git", self.owner, self.repo),
            GitProvider::Bitbucket => {
                format!("git@bitbucket.org:{}/{}.git", self.owner, self.repo)
            }
            GitProvider::Gitee => format!("git@gitee.com:{}/{}.git", self.owner, self.repo),
            GitProvider::Custom => self.url.clone(),
        }
    }
}

/// Expand Git shorthand notation to full URL
///
/// # Supported formats
///
/// - `github:user/repo` -> `https://github.com/user/repo.git`
/// - `github:user/repo@v1.0.0` -> URL with tag reference
/// - `gh:user/repo` -> shorthand for github
/// - `gitlab:user/repo` -> GitLab
/// - `gl:user/repo` -> shorthand for gitlab
/// - `bitbucket:user/repo` -> Bitbucket
/// - `bb:user/repo` -> shorthand for bitbucket
/// - `gitee:user/repo` -> Gitee
/// - Full URLs are returned as-is
///
/// # Examples
///
/// ```
/// use ccgo::registry::expand_git_shorthand;
///
/// let spec = expand_git_shorthand("github:fmtlib/fmt").unwrap();
/// assert_eq!(spec.url, "https://github.com/fmtlib/fmt.git");
///
/// let spec = expand_git_shorthand("gh:nlohmann/json@v3.11.0").unwrap();
/// assert_eq!(spec.url, "https://github.com/nlohmann/json.git");
/// assert_eq!(spec.reference, Some("v3.11.0".to_string()));
/// ```
pub fn expand_git_shorthand(input: &str) -> Result<ShorthandSpec> {
    let input = input.trim();

    // Check if it's already a full URL
    if input.starts_with("https://")
        || input.starts_with("http://")
        || input.starts_with("git@")
        || input.starts_with("ssh://")
    {
        return parse_full_url(input);
    }

    // Check for shorthand format: provider:owner/repo[@ref]
    if let Some(colon_pos) = input.find(':') {
        let prefix = &input[..colon_pos];
        let rest = &input[colon_pos + 1..];

        if let Some(provider) = GitProvider::from_prefix(prefix) {
            return parse_shorthand(provider, rest);
        }
    }

    // If no recognized format, try to parse as owner/repo (assume GitHub)
    if input.contains('/') && !input.contains(':') && !input.contains("//") {
        return parse_shorthand(GitProvider::GitHub, input);
    }

    bail!(
        "Invalid Git shorthand: '{}'\n\
         Expected formats:\n\
         - github:user/repo\n\
         - github:user/repo@tag\n\
         - gh:user/repo (shorthand)\n\
         - user/repo (assumes GitHub)\n\
         - https://github.com/user/repo.git",
        input
    );
}

/// Parse shorthand format: owner/repo[@ref]
fn parse_shorthand(provider: GitProvider, input: &str) -> Result<ShorthandSpec> {
    let (path, reference) = if let Some(at_pos) = input.find('@') {
        let path = &input[..at_pos];
        let reference = &input[at_pos + 1..];
        (path, Some(reference.to_string()))
    } else {
        (input, None)
    };

    // Parse owner/repo
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() != 2 {
        bail!(
            "Invalid repository path: '{}'. Expected 'owner/repo' format.",
            path
        );
    }

    let owner = parts[0].to_string();
    let repo = parts[1].to_string();

    if owner.is_empty() || repo.is_empty() {
        bail!("Owner and repository name cannot be empty");
    }

    // Build full URL
    let url = format!("{}/{}/{}.git", provider.base_url(), owner, repo);

    Ok(ShorthandSpec {
        provider,
        owner,
        repo,
        reference,
        url,
    })
}

/// Parse full Git URL
fn parse_full_url(url: &str) -> Result<ShorthandSpec> {
    let provider = GitProvider::from_url(url);

    // Try to extract owner/repo from URL
    let (owner, repo, reference) = extract_owner_repo_from_url(url)?;

    Ok(ShorthandSpec {
        provider,
        owner,
        repo,
        reference,
        url: url.to_string(),
    })
}

/// Extract owner and repo from a full Git URL
fn extract_owner_repo_from_url(url: &str) -> Result<(String, String, Option<String>)> {
    // Handle SSH URLs: git@github.com:user/repo.git
    if url.starts_with("git@") {
        if let Some(colon_pos) = url.find(':') {
            let path = &url[colon_pos + 1..];
            return parse_path_component(path);
        }
    }

    // Handle HTTPS/HTTP URLs: https://github.com/user/repo.git
    if let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    {
        // Skip the host
        if let Some(slash_pos) = rest.find('/') {
            let path = &rest[slash_pos + 1..];
            return parse_path_component(path);
        }
    }

    // Fallback: return URL as-is
    Ok(("unknown".to_string(), url.to_string(), None))
}

/// Parse path component: user/repo.git or user/repo
fn parse_path_component(path: &str) -> Result<(String, String, Option<String>)> {
    // Remove .git suffix if present
    let path = path.strip_suffix(".git").unwrap_or(path);

    // Split into owner/repo
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 2 {
        let owner = parts[0].to_string();
        let repo = parts[1].to_string();
        Ok((owner, repo, None))
    } else {
        Ok(("unknown".to_string(), path.to_string(), None))
    }
}

/// Convert a ShorthandSpec back to CCGO.toml dependency format
impl ShorthandSpec {
    /// Generate TOML entry for this dependency
    pub fn to_toml_entry(&self, name: &str, version: Option<&str>) -> String {
        let mut entry = format!("[[dependencies]]\nname = \"{}\"", name);

        // Add version
        let version_str = version.unwrap_or("0.0.0");
        entry.push_str(&format!("\nversion = \"{}\"", version_str));

        // Add git URL
        entry.push_str(&format!("\ngit = \"{}\"", self.url));

        // Add reference as branch/tag if present
        if let Some(ref reference) = self.reference {
            // Determine if it looks like a tag (starts with v) or branch
            if reference.starts_with('v') || reference.chars().next().map_or(false, |c| c.is_numeric()) {
                entry.push_str(&format!("\nbranch = \"{}\"", reference));
            } else {
                entry.push_str(&format!("\nbranch = \"{}\"", reference));
            }
        }

        entry.push('\n');
        entry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_shorthand() {
        let spec = expand_git_shorthand("github:fmtlib/fmt").unwrap();
        assert_eq!(spec.provider, GitProvider::GitHub);
        assert_eq!(spec.owner, "fmtlib");
        assert_eq!(spec.repo, "fmt");
        assert_eq!(spec.url, "https://github.com/fmtlib/fmt.git");
        assert!(spec.reference.is_none());
    }

    #[test]
    fn test_github_shorthand_with_ref() {
        let spec = expand_git_shorthand("github:nlohmann/json@v3.11.0").unwrap();
        assert_eq!(spec.owner, "nlohmann");
        assert_eq!(spec.repo, "json");
        assert_eq!(spec.reference, Some("v3.11.0".to_string()));
    }

    #[test]
    fn test_gh_shorthand() {
        let spec = expand_git_shorthand("gh:gabime/spdlog").unwrap();
        assert_eq!(spec.provider, GitProvider::GitHub);
        assert_eq!(spec.url, "https://github.com/gabime/spdlog.git");
    }

    #[test]
    fn test_gitlab_shorthand() {
        let spec = expand_git_shorthand("gitlab:user/repo").unwrap();
        assert_eq!(spec.provider, GitProvider::GitLab);
        assert_eq!(spec.url, "https://gitlab.com/user/repo.git");
    }

    #[test]
    fn test_gl_shorthand() {
        let spec = expand_git_shorthand("gl:user/repo").unwrap();
        assert_eq!(spec.provider, GitProvider::GitLab);
    }

    #[test]
    fn test_bitbucket_shorthand() {
        let spec = expand_git_shorthand("bitbucket:user/repo").unwrap();
        assert_eq!(spec.provider, GitProvider::Bitbucket);
        assert_eq!(spec.url, "https://bitbucket.org/user/repo.git");
    }

    #[test]
    fn test_bb_shorthand() {
        let spec = expand_git_shorthand("bb:user/repo").unwrap();
        assert_eq!(spec.provider, GitProvider::Bitbucket);
    }

    #[test]
    fn test_gitee_shorthand() {
        let spec = expand_git_shorthand("gitee:user/repo").unwrap();
        assert_eq!(spec.provider, GitProvider::Gitee);
        assert_eq!(spec.url, "https://gitee.com/user/repo.git");
    }

    #[test]
    fn test_bare_owner_repo() {
        // Without prefix, assumes GitHub
        let spec = expand_git_shorthand("fmtlib/fmt").unwrap();
        assert_eq!(spec.provider, GitProvider::GitHub);
        assert_eq!(spec.url, "https://github.com/fmtlib/fmt.git");
    }

    #[test]
    fn test_full_https_url() {
        let spec = expand_git_shorthand("https://github.com/fmtlib/fmt.git").unwrap();
        assert_eq!(spec.provider, GitProvider::GitHub);
        assert_eq!(spec.owner, "fmtlib");
        assert_eq!(spec.repo, "fmt");
        assert_eq!(spec.url, "https://github.com/fmtlib/fmt.git");
    }

    #[test]
    fn test_full_ssh_url() {
        let spec = expand_git_shorthand("git@github.com:fmtlib/fmt.git").unwrap();
        assert_eq!(spec.provider, GitProvider::GitHub);
        assert_eq!(spec.owner, "fmtlib");
        assert_eq!(spec.repo, "fmt");
    }

    #[test]
    fn test_ssh_url_generation() {
        let spec = expand_git_shorthand("github:fmtlib/fmt").unwrap();
        assert_eq!(spec.ssh_url(), "git@github.com:fmtlib/fmt.git");
    }

    #[test]
    fn test_invalid_shorthand() {
        assert!(expand_git_shorthand("invalid").is_err());
        assert!(expand_git_shorthand("github:").is_err());
        assert!(expand_git_shorthand("github:/repo").is_err());
    }

    #[test]
    fn test_provider_from_url() {
        assert_eq!(
            GitProvider::from_url("https://github.com/user/repo"),
            GitProvider::GitHub
        );
        assert_eq!(
            GitProvider::from_url("https://gitlab.com/user/repo"),
            GitProvider::GitLab
        );
        assert_eq!(
            GitProvider::from_url("https://bitbucket.org/user/repo"),
            GitProvider::Bitbucket
        );
        assert_eq!(
            GitProvider::from_url("https://example.com/user/repo"),
            GitProvider::Custom
        );
    }

    #[test]
    fn test_toml_entry_generation() {
        let spec = expand_git_shorthand("github:fmtlib/fmt@v10.1.1").unwrap();
        let toml = spec.to_toml_entry("fmt", Some("^10.1"));
        assert!(toml.contains("name = \"fmt\""));
        assert!(toml.contains("version = \"^10.1\""));
        assert!(toml.contains("git = \"https://github.com/fmtlib/fmt.git\""));
        assert!(toml.contains("branch = \"v10.1.1\""));
    }
}
