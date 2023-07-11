// Purpose: Utility functions for the project.

use std::{
    fs::File,
    io::{self, BufRead},
    path::PathBuf,
};

use anyhow::Result;

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
