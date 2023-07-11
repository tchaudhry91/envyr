// Purpose: Utility functions for the project.

use std::{
    fs::File,
    io::{self, BufRead},
    path::PathBuf,
};

use anyhow::Result;

use super::package::PType;

// Checks if the file contains a python main.
pub fn check_python_main(f: &PathBuf) -> Result<bool> {
    let code = std::fs::read_to_string(f)?;
    if code.contains("if __name__ == \"__main__\":") {
        return Ok(true);
    }
    return Ok(false);
}

// Returns the interpretter of the file if a shebang is found on top.
pub fn check_shebang_file(file: &PathBuf) -> Result<Option<String>> {
    let file = File::open(file)?;
    let mut reader = io::BufReader::new(file);
    let mut line = vec![];
    _ = reader.read_until(b'\n', &mut line)?;
    let line = String::from_utf8(line)?.trim().to_string();
    if line.starts_with("#!") {
        return Ok(Some(line[2..].to_string()));
    }
    return Ok(None);
}

pub fn map_extension_to_ptype(ext: &str) -> Option<PType> {
    return match ext {
        "py" => Some(PType::Python),
        "sh" => Some(PType::Shell),
        "js" => Some(PType::Node),
        "ts" => Some(PType::Node),
        _ => None,
    };
}

pub fn check_package_json(project_root: &PathBuf) -> Result<bool> {
    let package_json = project_root.join("package.json");
    if package_json.exists() {
        return Ok(true);
    }
    return Ok(false);
}

pub fn check_requirements_txt(project_root: &PathBuf) -> Result<bool> {
    let requirements_txt = project_root.join("requirements.txt");
    if requirements_txt.exists() {
        return Ok(true);
    }
    return Ok(false);
}
