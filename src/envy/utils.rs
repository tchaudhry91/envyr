// Purpose: Utility functions for the project.

use std::{
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

pub const PRIORITY_TOP: u8 = 0;
pub const PRIORITY_LIKELY: u8 = 1;
pub const PRIORITY_UNLIKELY: u8 = 2;
pub const PRIORITY_LAST: u8 = 3;

use anyhow::Result;
use serde_json::Value;

use super::package::PType;

// Checks if the file contains a python main.
pub fn check_python_main(code: &str) -> Result<bool> {
    if code.contains("if __name__ == \"__main__\":") || code.contains("if __name__ == '__main__':")
    {
        return Ok(true);
    }
    Ok(false)
}

pub fn check_python_exec_priority(f: &PathBuf) -> Result<u8> {
    let code = std::fs::read_to_string(f)?;
    let main_defined = check_python_main(&code)?;
    if main_defined {
        return Ok(PRIORITY_TOP);
    }
    Ok(PRIORITY_UNLIKELY)
}

// Returns the interpretter of the file if a shebang is found on top.
pub fn check_shebang_file(file: &PathBuf) -> Result<Option<String>> {
    let file = File::open(file)?;
    let mut reader = io::BufReader::new(file);
    let mut line = vec![];
    _ = reader.read_until(b'\n', &mut line)?;
    let line = String::from_utf8(line)?.trim().to_string();
    if line.starts_with("#!") {
        return Ok(Some(
            line.strip_prefix("#!").unwrap_or_default().to_string(),
        ));
    }
    Ok(None)
}

pub fn map_extension_to_ptype(ext: &str) -> Option<PType> {
    match ext {
        "py" => Some(PType::Python),
        "sh" => Some(PType::Shell),
        "js" => Some(PType::Node),
        "ts" => Some(PType::Node),
        _ => None,
    }
}

pub fn check_package_json(project_root: &Path) -> bool {
    let package_json = project_root.join("package.json");
    if package_json.exists() {
        return true;
    }
    false
}

pub fn check_requirements_txt(project_root: &Path) -> bool {
    let requirements_txt = project_root.join("requirements.txt");
    if requirements_txt.exists() {
        return true;
    }
    false
}

pub fn detect_main_node(project_root: &Path) -> Option<PathBuf> {
    if !check_package_json(project_root) {
        return None;
    }
    let package_json = std::fs::read_to_string(project_root.join("package.json"));
    match package_json {
        Ok(package_json) => {
            let v = serde_json::from_str(&package_json);
            if v.is_err() {
                return None;
            }
            let v: Value = v.unwrap();
            let main = v["main"].as_str()?;
            Some(PathBuf::from(main))
        }
        Err(_) => None,
    }
}
