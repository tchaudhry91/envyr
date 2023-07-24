use std::path::Path;

use crate::RunConfig;

use super::package::Pack;
use super::{docker, utils};
use anyhow::Result;
use clap::ValueEnum;
use log::debug;
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

pub type AliasMap = std::collections::HashMap<String, RunConfig>;

pub fn load_aliases(envy_root: &Path) -> Result<AliasMap> {
    let aliases_f = envy_root.join("aliases.json");
    // If the file doesn't exist, create it
    if !aliases_f.exists() {
        let aliases = serde_json::to_string_pretty(&AliasMap::new())?;
        std::fs::write(aliases_f.clone(), aliases)?;
        debug!("Created new aliases file at {}", aliases_f.display());
    }

    // Read the json from the file
    let aliases = std::fs::read_to_string(aliases_f)?;
    debug!("Loaded aliases..");
    // Deserialize the json into a map
    let aliases: AliasMap = serde_json::from_str(&aliases)?;
    Ok(aliases)
}

pub fn store_alias(envy_root: &Path, name: String, conf: RunConfig) -> Result<()> {
    let mut aliases = load_aliases(envy_root)?;
    aliases.insert(name, conf);
    let aliases_f = envy_root.join("aliases.json");
    let aliases = serde_json::to_string_pretty(&aliases)?;
    std::fs::write(aliases_f, aliases)?;
    Ok(())
}
