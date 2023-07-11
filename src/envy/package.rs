use super::utils;
use anyhow::Result;
use clap::ValueEnum;
use pathdiff::diff_paths;
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Default, Clone, ValueEnum)]
pub enum PType {
    Python,
    Node,
    Shell,
    #[default]
    Other,
}

// Pack is the base struct holding the Package information.
#[derive(Debug)]
pub struct Pack {
    pub name: String,
    pub interpreter: String,
    pub ptype: PType,
    pub entrypoint: String,
}
impl Pack {
    pub fn builder(project_root: PathBuf) -> Result<PackBuilder> {
        let mut builder = PackBuilder::default();
        builder.name = detect_name(&project_root);

        // Try to detect project type.

        // Move on to executable detection.
        builder.executables = get_executable_files(&project_root)?;
        // Only handle the case where there is exactly one executable found.
        if builder.executables.len() == 1 {
            builder.interpreter = Some(builder.executables[0].1.clone());
            let entrypoint = builder.executables[0].0.to_path_buf();
            let entrypoint = diff_paths(&entrypoint, &project_root).unwrap_or_default();
            builder.entrypoint = Some(entrypoint.to_str().unwrap_or_default().to_string());
        }
        Ok(builder)
    }
}

#[derive(Default)]
pub struct PackBuilder {
    name: Option<String>,
    interpreter: Option<String>,
    entrypoint: Option<String>,
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

    pub fn entrypoint(mut self, entrypoint: String) -> Self {
        self.entrypoint = Some(entrypoint);
        self
    }

    pub fn ptype(mut self, ptype: PType) -> Self {
        self.ptype = ptype;
        self
    }

    pub fn build(self) -> Result<Pack> {
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
            } else {
                return Err(anyhow::anyhow!(
                    "Multiple entrypoints detected! {:?}. Please choose one manually.",
                    self.executables
                ));
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

fn get_executable_files(project_root: &PathBuf) -> Result<Vec<(PathBuf, String)>> {
    let mut executable_files: Vec<(PathBuf, String)> = vec![];

    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    // Check for a basic shebang first
                    let shebang_file =
                        utils::check_shebang_file(&entry.path().to_path_buf()).unwrap_or(None);
                    if let Some(shebang_file) = shebang_file {
                        executable_files.push((entry.path().to_path_buf(), shebang_file));
                        continue;
                    }

                    // Check if the file has a .py extension
                    // and if it has a python main.
                    if entry.path().extension().unwrap_or_default() == "py" {
                        let python_main =
                            utils::check_python_main(&entry.path().to_path_buf()).unwrap_or(false);
                        if python_main {
                            executable_files.push((
                                entry.path().to_path_buf(),
                                "/usr/bin/env python".to_string(),
                            ));
                            continue;
                        }
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error walking project directory: {:?}", e));
            }
        }
    }

    Ok(executable_files)
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
