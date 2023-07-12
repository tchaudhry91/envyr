use super::utils;
use anyhow::Result;
use clap::ValueEnum;
use pathdiff::diff_paths;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Default, Clone, ValueEnum, Serialize, Deserialize)]
pub enum PType {
    Python,
    Node,
    Shell,
    #[default]
    Other,
}

// Pack is the base struct holding the Package information.
#[derive(Debug, Serialize, Deserialize)]
pub struct Pack {
    pub name: String,
    pub interpreter: String,
    pub ptype: PType,
    pub entrypoint: PathBuf,
}
impl Pack {
    pub fn builder(project_root: PathBuf) -> Result<PackBuilder> {
        let builder = analyse_project(&project_root)?;
        Ok(builder)
    }
}

#[derive(Default)]
pub struct PackBuilder {
    name: Option<String>,
    interpreter: Option<String>,
    entrypoint: Option<PathBuf>,
    executables: Vec<(PathBuf, String)>,
    ptype: PType,
}

impl PackBuilder {
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn interpreter(mut self, interpreter: String) -> Self {
        self.interpreter = Some(interpreter);
        self
    }

    pub fn entrypoint(mut self, entrypoint: PathBuf) -> Self {
        self.entrypoint = Some(entrypoint);
        self
    }

    pub fn ptype(mut self, ptype: PType) -> Self {
        self.ptype = ptype;
        self
    }

    pub fn build(mut self) -> Result<Pack> {
        // Check values
        if self.name.is_none() {
            return Err(anyhow::anyhow!(
                "Could not detect project name. Please specify it manually."
            ));
        }
        if self.entrypoint.is_none() {
            if self.executables.len() < 1 {
                return Err(anyhow::anyhow!(
                    "Could not detect project entrypoint. Please specify it manually."
                ));
            } else if self.executables.len() > 1 {
                return Err(anyhow::anyhow!(
                    "Multiple entrypoints detected! {:?}. Please choose one manually.",
                    self.executables
                ));
            } else {
                self.entrypoint = Some(self.executables[0].0.clone());
                self.interpreter = Some(self.executables[0].1.clone());
            }
        }
        if self.interpreter.is_none() {
            return Err(anyhow::anyhow!(
                "Could not detect project interpreter. Please specify it manually."
            ));
        }
        Ok(Pack {
            name: self.name.unwrap_or_default(),
            interpreter: self.interpreter.unwrap_or_default(),
            entrypoint: self.entrypoint.unwrap_or_default(),
            ptype: self.ptype,
        })
    }
}

fn detect_name(project_root: &PathBuf) -> Option<String> {
    let name = project_root.file_name()?.to_str()?;
    Some(name.to_string())
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}
fn detect_ptype(project_root: &PathBuf) -> Option<PType> {
    // Check package.json
    if utils::check_package_json(project_root) {
        return Some(PType::Node);
    }
    // Check requirements.txt
    if utils::check_requirements_txt(project_root) {
        return Some(PType::Python);
    }
    None
}

fn analyse_project(project_root: &PathBuf) -> Result<PackBuilder> {
    let mut builder = PackBuilder::default();
    // Detect Name
    builder.name = detect_name(&project_root);

    // See if the project type can be ascertained
    if let Some(ptype) = detect_ptype(&project_root) {
        builder.ptype = ptype;
    }

    // Walk the project directory
    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    // Do a series of checks
                    // 1. Check a possible entrypoint
                    if let Some((f, interpreter)) = detect_possible_entrypoint(&entry) {
                        let relative_path = diff_paths(&f, &project_root).expect(
                            "Path Diff Error, this should not happen while walking the dir.",
                        );
                        builder.executables.push((relative_path, interpreter));
                    }
                    // 2. Check the file extensions and update ptype if necessary
                    if let Some(ptype) = detect_ptype_from_extension(&entry) {
                        builder.ptype = ptype;
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error walking project directory: {:?}", e));
            }
        }
    }

    Ok(builder)
}

fn detect_ptype_from_extension(entry: &DirEntry) -> Option<PType> {
    let extension = entry.path().extension()?.to_str()?;
    utils::map_extension_to_ptype(extension)
}

fn detect_possible_entrypoint(entry: &DirEntry) -> Option<(PathBuf, String)> {
    if let Some(interpreter) =
        utils::check_shebang_file(&entry.path().to_path_buf()).unwrap_or(None)
    {
        return Some((entry.path().to_path_buf(), interpreter));
    }

    // Check if the file has a .py extension
    // and if it has a python main.
    if entry.path().extension().unwrap_or_default() == "py" {
        if utils::check_python_main(&entry.path().to_path_buf()).unwrap_or(false) {
            return Some((
                entry.path().to_path_buf(),
                "/usr/bin/env python".to_string(),
            ));
        }
    }
    None
}

// This is a test that only works locally for now.
// It is a development convenience. Remove later
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_package() {
        let project_root = PathBuf::from("/home/tchaudhr/Workspace/sandbox");
        let pack = Pack::builder(project_root).unwrap().build().unwrap();
        assert_eq!(pack.name, "sandbox");
        assert_eq!(pack.interpreter, "/usr/bin/env python");
        assert_eq!(pack.entrypoint, "main.py");
    }
}
