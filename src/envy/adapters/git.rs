// This adapter allows using git respositories as a source for scripts.

use super::fetcher::Fetcher;
use anyhow::{anyhow, Result};
use std::path::PathBuf;

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
    fn fetch(&self, url: &str) -> Result<PathBuf> {
        let path = self.storage_dir_root.clone().join(get_name(url)?);
        // Pull instead of clone if the repo already exists
        if path.exists() {
            let status = std::process::Command::new("git")
                .arg("pull")
                .current_dir(&path)
                .output()?;
            if !status.status.success() {
                return Err(anyhow!(
                    "Failed to pull git repository: {:?}",
                    status.stderr
                ));
            };
            Ok(path)
        } else {
            let status = std::process::Command::new("git")
                .arg("clone")
                .arg(url)
                .arg(&path)
                .output()?;
            if !status.status.success() {
                return Err(anyhow!(
                    "Failed to clone git repository: {:?}",
                    status.stderr
                ));
            };
            Ok(path)
        }
    }
}

// Get project name for git repository
fn get_name(url: &str) -> Result<String> {
    let url = url.strip_suffix(".git").unwrap_or(url);
    let name = url.split('/').last().unwrap().to_string();
    Ok(name)
}
