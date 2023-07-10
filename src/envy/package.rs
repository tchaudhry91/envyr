use super::installable::Installable;
use anyhow;
use std::io::{self, BufRead};
use std::{
    fs::{read, File},
    path::PathBuf,
};
use walkdir::{DirEntry, WalkDir};

type Error = anyhow::Error;

// Pack is the base struct holding the Package information.
struct Pack {
    name: String,
    interpretter: String,
    entrypoints: Vec<String>,
    installables: Vec<Box<dyn Installable>>,
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn get_shebang_files(project_root: PathBuf) -> Result<Vec<PathBuf>, Error> {
    let mut shebang_files: Vec<PathBuf> = vec![];

    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    let shebang_file =
                        check_shebang_file(entry.path().to_path_buf()).unwrap_or(None);
                    if shebang_file.is_some() {
                        shebang_files.push(entry.path().to_path_buf());
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error reading file: {:?}", e));
            }
        }
    }

    Ok(shebang_files)
}

fn check_shebang_file(file: PathBuf) -> Result<Option<String>, Error> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_shebang_files() {
        let shebang_files = get_shebang_files(PathBuf::from("/home/tchaudhry/Temp")).unwrap();
        println!("{:?}", shebang_files);
    }
}
