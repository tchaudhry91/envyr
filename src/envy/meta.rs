use std::path::Path;

use super::docker;
use super::package::Pack;
use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

pub struct Generator {
    pub pack: Pack,
}

impl Generator {
    pub fn new(pack: Pack) -> Self {
        Self { pack }
    }

    pub fn generate_meta_dir(&self, project_root: &Path) -> Result<()> {
        let meta_dir = project_root.join(".envy");
        if !meta_dir.exists() {
            std::fs::create_dir(&meta_dir)?;
        }
        Ok(())
    }

    pub fn generate_docker(&self, project_root: &Path) -> Result<()> {
        let dockerfile = docker::generate_dockerfile(&self.pack, project_root)?;
        let dockerignore = docker::generate_docker_ignore(&self.pack)?;
        let dockerfile_path = project_root.join(".envy").join("Dockerfile");
        let dockerignore_path = project_root.join(".dockerignore");
        std::fs::write(dockerfile_path, dockerfile)?;
        std::fs::write(dockerignore_path, dockerignore)?;
        Ok(())
    }

    pub fn generate(&self, project_root: &Path) -> Result<()> {
        self.generate_meta_dir(project_root)?;
        // Write the json file to the meta dir
        self.pack.save(project_root)?;

        // Generate the dockerfile
        self.generate_docker(project_root)?;
        Ok(())
    }
}

#[derive(Debug, Default, Clone, ValueEnum, Serialize, Deserialize)]
pub enum Executors {
    #[default]
    Docker,
    Nix,
    Native,
}
