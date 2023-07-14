use super::git::GitFetcher;
use anyhow::Result;
use std::path::PathBuf;

// Fetcher abstracts over the different ways to source a project.
pub trait Fetcher {
    fn fetch(&self, url: &str, refresh: bool) -> Result<PathBuf>;
}

struct NoopFetcher {}

impl Fetcher for NoopFetcher {
    fn fetch(&self, url: &str, _refresh: bool) -> Result<PathBuf> {
        Ok(PathBuf::from(url))
    }
}

pub fn get_fetcher(url: &str, storage_dir: PathBuf) -> Result<Box<dyn Fetcher>> {
    if url.starts_with("git") {
        return Ok(Box::new(GitFetcher::new(storage_dir)?));
    }
    Ok(Box::new(NoopFetcher {}))
}
