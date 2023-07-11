use super::utils;
use anyhow::Result;
use pathdiff::diff_paths;
use std::io::{self, BufRead};
use std::{fs::File, path::PathBuf};
use walkdir::{DirEntry, WalkDir};

// Pack is the base struct holding the Package information.
#[derive(Debug)]
pub struct Pack {
    pub name: String,
    pub interpreter: String,
    pub entrypoint: String,
    pub deps: Vec<String>,
}
impl Pack {
    pub fn builder() -> PackBuilder {
        PackBuilder::default()
    }
}

#[derive(Default)]
pub struct PackBuilder {
    name: Option<String>,
    interpreter: Option<String>,
    entrypoint: Option<String>,
    deps: Option<Vec<String>>,
}

impl PackBuilder {
    pub fn new(project_root: PathBuf) -> Result<Self> {
        let mut builder = PackBuilder::default();
        builder.name = detect_name(&project_root);
        let executable_files = get_executable_files(&project_root)?;
        // Only handle the case where there is exactly one executable found.
        if executable_files.len() == 1 {
            builder.interpreter = Some(executable_files[0].1.clone());
            let entrypoint = executable_files[0].0.to_path_buf();
            let entrypoint = diff_paths(&entrypoint, &project_root).unwrap_or_default();
            builder.entrypoint = Some(entrypoint.to_str().unwrap_or_default().to_string());
        }
        Ok(builder)
    }

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

    pub fn deps(mut self, deps: Vec<String>) -> Self {
        self.deps = Some(deps);
        self
    }

    pub fn build(self) -> Result<Pack> {
        Ok(Pack {
            name: self.name.unwrap_or_default(),
            interpreter: self.interpreter.unwrap_or_default(),
            entrypoint: self.entrypoint.unwrap_or_default(),
            deps: self.deps.unwrap_or_default(),
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
        let pack = PackBuilder::new(PathBuf::from(project_root))
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(pack.name, "sandbox");
        assert_eq!(pack.interpreter, "/usr/bin/env python");
        assert_eq!(pack.entrypoint, "main.py");
    }
}
