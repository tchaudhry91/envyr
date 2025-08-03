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

use super::package::PType;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[derive(Serialize, Deserialize)]
struct PackDeps {
    deps: Vec<String>,
}

pub fn check_bash_dependencies(script_file: &Path) -> Result<Vec<String>> {
    let output = std::process::Command::new("envyr")
        .arg("run")
        .arg(format!(
            "--fs-map={}:/envyr/app/script.sh",
            script_file.display()
        ))
        .arg("git@github.com:tchaudhry91/detect-pkgs.git")
        .output()?;
    let deps: PackDeps = serde_json::from_slice(output.stdout.as_slice())?;
    Ok(deps.deps)
}

pub fn create_requirements_txt(project_root: &Path) -> Result<()> {
    // Assume pipreqs exists
    let output = std::process::Command::new("envyr")
        .arg("run")
        .arg(format!("--fs-map={}:/envyr/target", project_root.display()))
        .arg("git@github.com:tchaudhry91/pipreqs-wrap.git")
        .arg("--")
        .arg("/envyr/target")
        .output()?;
    if !output.status.success() {
        log::warn!(
            "Failed to create requirements.txt: {}:{}",
            String::from_utf8(output.stdout.clone())?,
            String::from_utf8(output.stderr.clone())?
        );
        return Err(anyhow::anyhow!(
            "Failed to create requirements.txt: {}:{}",
            String::from_utf8(output.stdout)?,
            String::from_utf8(output.stderr)?
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_check_python_main_double_quotes() {
        let code = r#"
def main():
    print("Hello World")

if __name__ == "__main__":
    main()
"#;
        assert!(check_python_main(code).unwrap());
    }

    #[test]
    fn test_check_python_main_single_quotes() {
        let code = r#"
def main():
    print("Hello World")

if __name__ == '__main__':
    main()
"#;
        assert!(check_python_main(code).unwrap());
    }

    #[test]
    fn test_check_python_main_false() {
        let code = r#"
def some_function():
    print("Hello World")

# No main block
"#;
        assert!(!check_python_main(code).unwrap());
    }

    #[test]
    fn test_check_shebang_file_python() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.py");
        fs::write(&file_path, "#!/usr/bin/env python3\nprint('Hello')").unwrap();
        
        let result = check_shebang_file(&file_path).unwrap();
        assert_eq!(result, Some("/usr/bin/env python3".to_string()));
    }

    #[test]
    fn test_check_shebang_file_bash() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.sh");
        fs::write(&file_path, "#!/bin/bash\necho 'Hello'").unwrap();
        
        let result = check_shebang_file(&file_path).unwrap();
        assert_eq!(result, Some("/bin/bash".to_string()));
    }

    #[test]
    fn test_check_shebang_file_none() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "This is just a text file").unwrap();
        
        let result = check_shebang_file(&file_path).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_map_extension_to_ptype() {
        assert_eq!(map_extension_to_ptype("py"), Some(PType::Python));
        assert_eq!(map_extension_to_ptype("sh"), Some(PType::Shell));
        assert_eq!(map_extension_to_ptype("js"), Some(PType::Node));
        assert_eq!(map_extension_to_ptype("ts"), Some(PType::Node));
        assert_eq!(map_extension_to_ptype("txt"), None);
        assert_eq!(map_extension_to_ptype("unknown"), None);
    }

    #[test]
    fn test_check_package_json_exists() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");
        fs::write(&package_json, r#"{"name": "test"}"#).unwrap();
        
        assert!(check_package_json(temp_dir.path()));
    }

    #[test]
    fn test_check_package_json_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!check_package_json(temp_dir.path()));
    }

    #[test]
    fn test_check_requirements_txt_exists() {
        let temp_dir = TempDir::new().unwrap();
        let requirements = temp_dir.path().join("requirements.txt");
        fs::write(&requirements, "requests==2.28.1").unwrap();
        
        assert!(check_requirements_txt(temp_dir.path()));
    }

    #[test]
    fn test_check_requirements_txt_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!check_requirements_txt(temp_dir.path()));
    }

    #[test]
    fn test_detect_main_node_with_valid_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");
        fs::write(&package_json, r#"{"name": "test", "main": "index.js"}"#).unwrap();
        
        let result = detect_main_node(temp_dir.path());
        assert_eq!(result, Some(PathBuf::from("index.js")));
    }

    #[test]
    fn test_detect_main_node_with_no_main_field() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");
        fs::write(&package_json, r#"{"name": "test"}"#).unwrap();
        
        let result = detect_main_node(temp_dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_main_node_with_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");
        fs::write(&package_json, "invalid json").unwrap();
        
        let result = detect_main_node(temp_dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_main_node_no_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let result = detect_main_node(temp_dir.path());
        assert_eq!(result, None);
    }

    #[test]
    fn test_check_python_exec_priority_with_main() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.py");
        fs::write(&file_path, r#"
def main():
    print("Hello")

if __name__ == "__main__":
    main()
"#).unwrap();
        
        let priority = check_python_exec_priority(&file_path).unwrap();
        assert_eq!(priority, PRIORITY_TOP);
    }

    #[test]
    fn test_check_python_exec_priority_without_main() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.py");
        fs::write(&file_path, r#"
def some_function():
    print("Hello")
"#).unwrap();
        
        let priority = check_python_exec_priority(&file_path).unwrap();
        assert_eq!(priority, PRIORITY_UNLIKELY);
    }
}
