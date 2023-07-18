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
    let status = std::process::Command::new("git")
        .arg("pull")
        .current_dir(path)
        .output()?;
    if !status.status.success() {
        return Err(anyhow!(
            "Failed to pull git repository: {:?}",
            String::from_utf8(status.stderr),
        ));
    };
    Ok(())
}

fn fetch_tags(path: &Path) -> Result<()> {
    debug!("Fetching tags for: {:?}", path);
    let status = std::process::Command::new("git")
        .arg("fetch")
        .arg("--tags")
        .current_dir(path)
        .output()?;
    if !status.status.success() {
        return Err(anyhow!(
            "Failed to fetch tags: {:?}",
            String::from_utf8(status.stderr),
        ));
    };
    Ok(())
}

fn checkout_version(path: &Path, version: &str) -> Result<()> {
    if version != "latest" {
        debug!("Checking out version: {}", version);
        let status = std::process::Command::new("git")
            .arg("checkout")
            .arg(version)
            .current_dir(path)
            .output()?;
        if !status.status.success() {
            return Err(anyhow!(
                "Failed to checkout version: {:?}",
                String::from_utf8(status.stderr),
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

    let status = std::process::Command::new("git")
        .arg("clone")
        .arg(url)
        .arg(path)
        .output()?;
    if !status.status.success() {
        return Err(anyhow!(
            "Failed to clone git repository: {:?}",
            String::from_utf8(status.stderr),
        ));
    };

    Ok(())
}

fn swap_back_to_latest(path: &Path) -> Result<()> {
    debug!("Swapping back to main/master branch");
    let out = std::process::Command::new("git")
        .arg("checkout")
        .arg("main")
        .current_dir(path)
        .output()?;
    if !out.status.success() {
        // Try master
        let out = std::process::Command::new("git")
            .arg("checkout")
            .arg("master")
            .current_dir(path)
            .output()?;
        if !out.status.success() {
            return Err(anyhow!(
                "Failed to swap back to main/master branch: {:?}",
                String::from_utf8(out.stderr),
            ));
        }
    }
    Ok(())
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
    let provider = url.split(':').next().unwrap().to_string();
    let provider = provider.split('@').last().unwrap_or(&provider).to_string();
    Ok(provider)
}

fn get_org_name(url: &str) -> Result<String> {
    let url = url.strip_suffix(".git").unwrap_or(url);
    let name = url.split('/').nth_back(1).unwrap().to_string();
    let name = name.split(':').last().unwrap().to_string();
    Ok(name)
}

// Get project name for git repository
fn get_project_name(url: &str) -> Result<String> {
    let url = url.strip_suffix(".git").unwrap_or(url);
    let name = url.split('/').last().unwrap().to_string();
    Ok(name)
}

// Some Tests for the git functions
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_unwrapping() {
        let url = "git@github.com:envy-lang/envy.git";
        let name = get_project_name(url).unwrap();
        assert_eq!(name, "envy");
        let org = get_org_name(url).unwrap();
        assert_eq!(org, "envy-lang");
        let provider = get_git_provider(url).unwrap();
        assert_eq!(provider, "github.com");

        let full_path = get_storage_path(url).unwrap();
        assert_eq!(full_path, PathBuf::from("github.com/envy-lang/envy"));
    }
}
