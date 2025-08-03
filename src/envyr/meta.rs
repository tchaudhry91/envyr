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
        let meta_dir = project_root.join(".envyr");
        if !meta_dir.exists() {
            std::fs::create_dir(&meta_dir)?;
        }
        Ok(())
    }

    pub fn generate_docker(&self, project_root: &Path) -> Result<()> {
        let dockerfile = docker::generate_dockerfile(&self.pack, project_root)?;
        let dockerignore = docker::generate_docker_ignore(&self.pack)?;
        let dockerfile_path = project_root.join(".envyr").join("Dockerfile");
        let dockerignore_path = project_root.join(".dockerignore");
        std::fs::write(dockerfile_path, dockerfile)?;
        std::fs::write(dockerignore_path, dockerignore)?;
        Ok(())
    }

    pub fn generate_python(&self, project_root: &Path) -> Result<()> {
        if !utils::check_requirements_txt(project_root) {
            // Attempt to generate with pipreqs
            if utils::create_requirements_txt(project_root).is_err() {
                log::warn!("No requirements.txt found. Unable to generate using pipreqs.");
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

pub fn load_aliases(envyr_root: &Path) -> Result<AliasMap> {
    let aliases_f = envyr_root.join("aliases.json");
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

pub fn remove_alias(envyr_root: &Path, name: String) -> Result<()> {
    let mut aliases = load_aliases(envyr_root)?;
    aliases.remove(&name);
    let aliases_f = envyr_root.join("aliases.json");
    let aliases = serde_json::to_string_pretty(&aliases)?;
    std::fs::write(aliases_f, aliases)?;
    Ok(())
}

pub fn store_alias(envyr_root: &Path, name: String, conf: RunConfig) -> Result<()> {
    let mut aliases = load_aliases(envyr_root)?;
    aliases.insert(name, conf);
    let aliases_f = envyr_root.join("aliases.json");
    let aliases = serde_json::to_string_pretty(&aliases)?;
    std::fs::write(aliases_f, aliases)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use std::path::PathBuf;
    use crate::{RunConfig, OverrideOpts};
    use crate::envyr::package::PType;

    fn create_test_pack() -> Pack {
        Pack {
            name: "test-pack".to_string(),
            interpreter: "/usr/bin/env python".to_string(),
            ptype: PType::Python,
            deps: vec!["requests".to_string()],
            entrypoint: PathBuf::from("main.py"),
        }
    }

    fn create_test_run_config() -> RunConfig {
        RunConfig {
            project_root: "/tmp/test".to_string(),
            sub_dir: None,
            executor: Executors::Docker,
            interactive: false,
            network: None,
            refresh: false,
            autogen: false,
            tag: "latest".to_string(),
            fs_map: vec![],
            port_map: vec![],
            env_map: vec![],
            timeout: None,
            overrides: OverrideOpts {
                name: None,
                interpreter: None,
                entrypoint: None,
                ptype: None,
            },
            args: vec![],
        }
    }

    #[test]
    fn test_generator_new() {
        let pack = create_test_pack();
        let generator = Generator::new(pack.clone());
        
        assert_eq!(generator.pack.name, pack.name);
        assert_eq!(generator.pack.interpreter, pack.interpreter);
        assert_eq!(generator.pack.entrypoint, pack.entrypoint);
    }

    #[test]
    fn test_generator_generate_meta_dir() {
        let temp_dir = TempDir::new().unwrap();
        let pack = create_test_pack();
        let generator = Generator::new(pack);
        
        generator.generate_meta_dir(temp_dir.path()).unwrap();
        
        let meta_dir = temp_dir.path().join(".envyr");
        assert!(meta_dir.exists());
        assert!(meta_dir.is_dir());
    }

    #[test]
    fn test_generator_generate_meta_dir_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let meta_dir = temp_dir.path().join(".envyr");
        fs::create_dir(&meta_dir).unwrap();
        
        let pack = create_test_pack();
        let generator = Generator::new(pack);
        
        // Should not fail if directory already exists
        generator.generate_meta_dir(temp_dir.path()).unwrap();
        
        assert!(meta_dir.exists());
        assert!(meta_dir.is_dir());
    }

    #[test]
    fn test_generator_generate_docker() {
        let temp_dir = TempDir::new().unwrap();
        let meta_dir = temp_dir.path().join(".envyr");
        fs::create_dir(&meta_dir).unwrap();
        
        let pack = create_test_pack();
        let generator = Generator::new(pack);
        
        generator.generate_docker(temp_dir.path()).unwrap();
        
        let dockerfile_path = meta_dir.join("Dockerfile");
        let dockerignore_path = temp_dir.path().join(".dockerignore");
        
        assert!(dockerfile_path.exists());
        assert!(dockerignore_path.exists());
        
        let dockerfile_content = fs::read_to_string(dockerfile_path).unwrap();
        assert!(dockerfile_content.contains("FROM python:3.11-alpine"));
        
        let dockerignore_content = fs::read_to_string(dockerignore_path).unwrap();
        assert!(dockerignore_content.contains("*.pyc"));
    }

    #[test]
    fn test_generator_generate_all() {
        let temp_dir = TempDir::new().unwrap();
        let pack = create_test_pack();
        let generator = Generator::new(pack.clone());
        
        generator.generate(temp_dir.path()).unwrap();
        
        // Check that meta directory was created
        let meta_dir = temp_dir.path().join(".envyr");
        assert!(meta_dir.exists());
        
        // Check that meta.json was created
        let meta_file = meta_dir.join("meta.json");
        assert!(meta_file.exists());
        
        // Check that docker files were created
        let dockerfile = meta_dir.join("Dockerfile");
        let dockerignore = temp_dir.path().join(".dockerignore");
        assert!(dockerfile.exists());
        assert!(dockerignore.exists());
        
        // Verify meta.json content
        let loaded_pack = Pack::load(temp_dir.path()).unwrap();
        assert_eq!(loaded_pack.name, pack.name);
        assert_eq!(loaded_pack.interpreter, pack.interpreter);
    }

    #[test]
    fn test_load_aliases_empty() {
        let temp_dir = TempDir::new().unwrap();
        
        let aliases = load_aliases(temp_dir.path()).unwrap();
        assert!(aliases.is_empty());
    }

    #[test]
    fn test_load_aliases_existing() {
        let temp_dir = TempDir::new().unwrap();
        let aliases_file = temp_dir.path().join("aliases.json");
        
        // Create a test aliases file
        let test_config = create_test_run_config();
        let mut test_aliases = std::collections::HashMap::new();
        test_aliases.insert("test-alias".to_string(), test_config);
        
        let aliases_json = serde_json::to_string_pretty(&test_aliases).unwrap();
        fs::write(&aliases_file, aliases_json).unwrap();
        
        let loaded_aliases = load_aliases(temp_dir.path()).unwrap();
        assert_eq!(loaded_aliases.len(), 1);
        assert!(loaded_aliases.contains_key("test-alias"));
    }

    #[test]
    fn test_load_aliases_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let aliases_file = temp_dir.path().join("aliases.json");
        
        // Create invalid JSON
        fs::write(&aliases_file, "invalid json").unwrap();
        
        let result = load_aliases(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_store_alias() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_run_config();
        
        store_alias(temp_dir.path(), "my-alias".to_string(), config.clone()).unwrap();
        
        let aliases = load_aliases(temp_dir.path()).unwrap();
        assert_eq!(aliases.len(), 1);
        assert!(aliases.contains_key("my-alias"));
        
        let stored_config = aliases.get("my-alias").unwrap();
        assert_eq!(stored_config.project_root, config.project_root);
        assert_eq!(stored_config.autogen, config.autogen);
    }

    #[test]
    fn test_store_multiple_aliases() {
        let temp_dir = TempDir::new().unwrap();
        let config1 = create_test_run_config();
        let mut config2 = create_test_run_config();
        config2.project_root = "/tmp/other".to_string();
        
        store_alias(temp_dir.path(), "alias1".to_string(), config1).unwrap();
        store_alias(temp_dir.path(), "alias2".to_string(), config2).unwrap();
        
        let aliases = load_aliases(temp_dir.path()).unwrap();
        assert_eq!(aliases.len(), 2);
        assert!(aliases.contains_key("alias1"));
        assert!(aliases.contains_key("alias2"));
    }

    #[test]
    fn test_store_alias_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let config1 = create_test_run_config();
        let mut config2 = create_test_run_config();
        config2.interactive = true;
        
        // Store first config
        store_alias(temp_dir.path(), "test-alias".to_string(), config1).unwrap();
        
        // Overwrite with second config
        store_alias(temp_dir.path(), "test-alias".to_string(), config2.clone()).unwrap();
        
        let aliases = load_aliases(temp_dir.path()).unwrap();
        assert_eq!(aliases.len(), 1);
        
        let stored_config = aliases.get("test-alias").unwrap();
        assert_eq!(stored_config.interactive, config2.interactive);
    }
}
