use super::utils;
use anyhow::Result;
use std::io::{self, BufRead};
use std::{fs::File, path::PathBuf};
use walkdir::{DirEntry, WalkDir};

// Pack is the base struct holding the Package information.
pub struct Pack {
    pub name: String,
    pub interpretter: String,
    pub entrypoints: Vec<String>,
    pub deps: Vec<String>,
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn get_executable_files(project_root: PathBuf) -> Result<Vec<PathBuf>> {
    let mut executable_files: Vec<PathBuf> = vec![];

    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    // Check for a basic shebang first
                    let shebang_file =
                        utils::check_shebang_file(entry.path().to_path_buf()).unwrap_or(None);
                    if shebang_file.is_some() {
                        executable_files.push(entry.path().to_path_buf());
                        continue;
                    }

                    // Check if the file has a .py extension
                    // and if it has a python main.
                    if entry.path().extension().unwrap_or_default() == "py" {
                        let python_main =
                            utils::check_python_main(entry.path().to_path_buf()).unwrap_or(false);
                        if python_main {
                            executable_files.push(entry.path().to_path_buf());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_executable_files() {
        let shebang_files =
            get_executable_files(PathBuf::from("/home/tchaudhry/Workspace/cassini")).unwrap();
        println!("{:?}", shebang_files);
    }
}
