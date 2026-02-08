// This adapter allows using git respositories as a source for scripts.

use super::fetcher::Fetcher;
use anyhow::{anyhow, Result};
use log::debug;
use std::path::{Path, PathBuf};

pub struct GitFetcher {
    storage_dir_root: PathBuf,
}

impl GitFetcher {
    pub fn new(storage_dir_root: PathBuf) -> Result<Self> {
        if !storage_dir_root.exists() {
            std::fs::create_dir_all(&storage_dir_root)?;
        }
        Ok(Self { storage_dir_root })
    }
}

impl Fetcher for GitFetcher {
    fn fetch(&self, url: &str, version: &str, refresh: bool) -> Result<PathBuf> {
        let path = self.storage_dir_root.clone().join(get_storage_path(url)?);
        // Pull instead of clone if the repo already exists
        if path.exists() {
            debug!("Clone already exists: {:?}", path);
            swap_back_to_latest(&path)?;
            if refresh {
                pull_repo(&path)?;
                fetch_tags(&path)?;
            }
            checkout_version(&path, version)?;
        } else {
            clone_repo(url, &path)?;
            fetch_tags(&path)?;
            checkout_version(&path, version)?;
        }
        Ok(path)
    }
}

fn pull_repo(path: &Path) -> Result<()> {
    let output = std::process::Command::new("git")
        .arg("pull")
        .current_dir(path)
        .output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "Failed to pull git repository: {}",
            String::from_utf8_lossy(&output.stderr),
        ));
    };
    Ok(())
}

fn fetch_tags(path: &Path) -> Result<()> {
    debug!("Fetching tags for: {:?}", path);
    let output = std::process::Command::new("git")
        .arg("fetch")
        .arg("--tags")
        .current_dir(path)
        .output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "Failed to fetch tags: {}",
            String::from_utf8_lossy(&output.stderr),
        ));
    };
    Ok(())
}

fn checkout_version(path: &Path, version: &str) -> Result<()> {
    if version != "latest" {
        debug!("Checking out version: {}", version);
        let output = std::process::Command::new("git")
            .arg("checkout")
            .arg(version)
            .current_dir(path)
            .output()?;
        if !output.status.success() {
            return Err(anyhow!(
                "Failed to checkout version '{}': {}",
                version,
                String::from_utf8_lossy(&output.stderr),
            ));
        };
    }
    Ok(())
}

fn clone_repo(url: &str, path: &Path) -> Result<()> {
    // Create basedir if it doesn't exist
    //
    debug!("Cloning git repository: {:?}", path);
    let base_dir = path.parent();
    match base_dir {
        Some(dir) => {
            if !dir.exists() {
                std::fs::create_dir_all(dir)?;
            }
        }
        None => {
            return Err(anyhow!("Failed to get parent directory of {:?}", path));
        }
    }

    let output = std::process::Command::new("git")
        .arg("clone")
        .arg(url)
        .arg(path)
        .output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "Failed to clone git repository: {}",
            String::from_utf8_lossy(&output.stderr),
        ));
    };

    Ok(())
}

fn swap_back_to_latest(path: &Path) -> Result<()> {
    debug!("Swapping back to default branch");

    // Try to detect the default branch from remote HEAD
    if let Ok(output) = std::process::Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
        .current_dir(path)
        .output()
    {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let branch = branch.strip_prefix("origin/").unwrap_or(&branch);
            let checkout = std::process::Command::new("git")
                .args(["checkout", branch])
                .current_dir(path)
                .output()?;
            if checkout.status.success() {
                return Ok(());
            }
        }
    }

    // Fallback: try main, then master
    let out = std::process::Command::new("git")
        .args(["checkout", "main"])
        .current_dir(path)
        .output()?;
    if out.status.success() {
        return Ok(());
    }

    let out = std::process::Command::new("git")
        .args(["checkout", "master"])
        .current_dir(path)
        .output()?;
    if out.status.success() {
        return Ok(());
    }

    Err(anyhow!(
        "Failed to checkout default branch: {}",
        String::from_utf8_lossy(&out.stderr),
    ))
}

fn get_storage_path(url: &str) -> Result<PathBuf> {
    let path = PathBuf::from("");
    let path = path
        .join(get_git_provider(url)?)
        .join(get_org_name(url)?)
        .join(get_project_name(url)?);
    Ok(path)
}

fn get_git_provider(url: &str) -> Result<String> {
    let url = url.strip_suffix(".git").unwrap_or(url);
    // Handle HTTPS/HTTP URLs: https://github.com/org/repo
    if let Some(rest) = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")) {
        let hostname = rest
            .split('/')
            .next()
            .ok_or_else(|| anyhow!("Failed to parse hostname from URL: {}", url))?;
        return Ok(hostname.to_string());
    }
    // Handle SSH-style URLs: git@github.com:org/repo
    let provider = url
        .split(':')
        .next()
        .ok_or_else(|| anyhow!("Failed to parse git provider from URL: {}", url))?;
    let provider = provider.split('@').last().unwrap_or(provider);
    Ok(provider.to_string())
}

fn get_org_name(url: &str) -> Result<String> {
    let url = url.strip_suffix(".git").unwrap_or(url);
    let name = url
        .split('/')
        .nth_back(1)
        .ok_or_else(|| anyhow!("Failed to parse org name from URL: {}", url))?
        .to_string();
    let name = name
        .split(':')
        .last()
        .ok_or_else(|| anyhow!("Failed to parse org name from URL: {}", url))?
        .to_string();
    Ok(name)
}

// Get project name for git repository
fn get_project_name(url: &str) -> Result<String> {
    let url = url.strip_suffix(".git").unwrap_or(url);
    let name = url
        .split('/')
        .last()
        .ok_or_else(|| anyhow!("Failed to parse project name from URL: {}", url))?
        .to_string();
    Ok(name)
}

// Some Tests for the git functions
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_unwrapping() {
        let url = "git@github.com:envyr-lang/envyr.git";
        let name = get_project_name(url).unwrap();
        assert_eq!(name, "envyr");
        let org = get_org_name(url).unwrap();
        assert_eq!(org, "envyr-lang");
        let provider = get_git_provider(url).unwrap();
        assert_eq!(provider, "github.com");

        let full_path = get_storage_path(url).unwrap();
        assert_eq!(full_path, PathBuf::from("github.com/envyr-lang/envyr"));
    }

    #[test]
    fn test_get_org_name_no_slash() {
        let result = get_org_name("no-slash-here");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_project_name_without_git_suffix() {
        let url = "git@github.com:org/my-project";
        let name = get_project_name(url).unwrap();
        assert_eq!(name, "my-project");
    }

    #[test]
    fn test_get_git_provider_https_style() {
        let url = "https://github.com/org/repo.git";
        let provider = get_git_provider(url).unwrap();
        assert_eq!(provider, "github.com");
    }

    #[test]
    fn test_get_storage_path_https() {
        let url = "https://github.com/envyr-lang/envyr.git";
        let full_path = get_storage_path(url).unwrap();
        assert_eq!(full_path, PathBuf::from("github.com/envyr-lang/envyr"));
    }
}
