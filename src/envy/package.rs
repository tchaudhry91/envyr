use super::{templates::TEMPLATE_DOCKERFILE, utils};
use anyhow::Result;
use clap::ValueEnum;
use handlebars::Handlebars;
use pathdiff::diff_paths;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
    pub deps: Vec<String>,
    pub entrypoint: PathBuf,
}
impl Pack {
    pub fn builder(project_root: &PathBuf) -> Result<PackBuilder> {
        let builder = analyse_project(project_root)?;
        Ok(builder)
    }

    pub fn generate_dockerfile(self, project_root: &Path) -> Result<String> {
        let mut handlebars = Handlebars::new();
        let source = TEMPLATE_DOCKERFILE;
        handlebars.register_template_string("Dockerfile", source)?;

        #[derive(Default, Serialize, Deserialize)]
        struct Data {
            interpreter: String,
            entrypoint: String,
            os_deps: Vec<String>,
            ptype: PType,
            type_reqs: bool,
        }

        // trim env prefix on interpreter
        let interpreter = self.interpreter.trim_start_matches("/usr/bin/env ");

        let mut d = Data {
            interpreter: interpreter.to_string(),
            entrypoint: self.entrypoint.to_str().unwrap().to_string(),
            os_deps: self.deps,
            ptype: self.ptype,
            type_reqs: false,
        };

        // Figure out type specific deps
        match d.ptype {
            PType::Python => {
                d.type_reqs = utils::check_requirements_txt(project_root);
            }
            PType::Node => {
                d.type_reqs = utils::check_package_json(project_root);
            }
            _ => {}
        };

        Ok(handlebars.render("Dockerfile", &d)?)
    }
}

#[derive(Default)]
pub struct PackBuilder {
    project_root: PathBuf,
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
            if self.executables.is_empty() {
                // Try to deduce based on project type
                if let Some(entrypoint) = deduce_entrypoint(self.ptype.clone(), &self.project_root)
                {
                    self.entrypoint = Some(entrypoint);
                } else {
                    return Err(anyhow::anyhow!(
                        "Could not detect project entrypoint. Please specify it manually."
                    ));
                }
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
            // Attempt to deduce from PType.
            if let Some(interpreter) = deduce_interpreter(self.ptype.clone()) {
                self.interpreter = Some(interpreter);
            } else {
                return Err(anyhow::anyhow!(
                    "Could not detect project interpreter. Please specify it manually."
                ));
            }
        }
        Ok(Pack {
            name: self.name.unwrap_or_default(),
            interpreter: self.interpreter.unwrap_or_default(),
            entrypoint: self.entrypoint.unwrap_or_default(),
            ptype: self.ptype,
            deps: vec![],
        })
    }
}

fn detect_name(project_root: &Path) -> Option<String> {
    let name = project_root.file_name()?.to_str()?;
    Some(name.to_string())
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn ignore_dir(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("node_modules") || s.starts_with("__pycache__"))
        .unwrap_or(false)
}

fn deduce_entrypoint(ptype: PType, project_root: &Path) -> Option<PathBuf> {
    match ptype {
        PType::Node => utils::detect_main_node(project_root),
        _ => None,
    }
}

fn deduce_interpreter(ptype: PType) -> Option<String> {
    match ptype {
        PType::Python => Some("/usr/bin/env python".to_string()),
        PType::Node => Some("/usr/bin/env node".to_string()),
        PType::Shell => Some("/bin/sh".to_string()),
        _ => None,
    }
}

fn detect_ptype(project_root: &Path) -> Option<PType> {
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
    let mut builder = PackBuilder {
        name: detect_name(project_root),
        project_root: project_root.clone(),
        ..Default::default()
    };
    // Detect Name

    // See if the project type can be ascertained
    if let Some(ptype) = detect_ptype(project_root) {
        builder.ptype = ptype;
    }

    // Walk the project directory
    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_entry(|e| !(is_hidden(e) || ignore_dir(e)))
    {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    // Do a series of checks
                    // 1. Check a possible entrypoint
                    if let Some((f, interpreter)) = detect_possible_entrypoint(&entry) {
                        let relative_path = diff_paths(&f, project_root).expect(
                            "Path Diff Error, this should not happen while walking the dir.",
                        );
                        builder.executables.push((relative_path, interpreter));
                    }
                    // 2. Check the file extensions and update ptype if necessary
                    // Only do this if the ptype isn't already detected via other methods.
                    if matches!(builder.ptype, PType::Other) {
                        if let Some(ptype) = detect_ptype_from_extension(&entry) {
                            builder.ptype = ptype;
                        }
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
    if entry.path().extension().unwrap_or_default() == "py"
        && utils::check_python_main(&entry.path().to_path_buf()).unwrap_or(false)
    {
        return Some((
            entry.path().to_path_buf(),
            "/usr/bin/env python".to_string(),
        ));
    }
    None
}
