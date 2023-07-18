use std::path::Path;

use super::package::Pack;
use super::{docker, utils};
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

    pub fn generate_python(&self, project_root: &Path) -> Result<()> {
        if !utils::check_requirements_txt(project_root) {
            // Attempt to generate with pipreqs
            if utils::create_requirements_txt(project_root).is_err() {
                log::warn!("No requirements.txt found. Unable to generate using pipreqs. You may need to install pipreqs separately.");
            }
        }
        Ok(())
    }

    pub fn generate(&self, project_root: &Path) -> Result<()> {
        self.generate_meta_dir(project_root)?;
        // Write the json file to the meta dir
        self.pack.save(project_root)?;

        // Generate language specific stuff
        if matches!(self.pack.ptype, super::package::PType::Python) {
            self.generate_python(project_root)?;
        }

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
