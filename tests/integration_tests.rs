use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage: envyr"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("generate"))
        .stdout(predicate::str::contains("run"))
        .stdout(predicate::str::contains("alias"));
}

#[test]
fn test_version_command() {
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("--version");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("envyr"));
}

#[test]
fn test_generate_command_python_project() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a simple Python project
    fs::write(temp_dir.path().join("main.py"), r#"
def main():
    print("Hello World")

if __name__ == "__main__":
    main()
"#).unwrap();
    
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("generate")
       .arg(temp_dir.path());
    
    cmd.assert()
        .success();
    
    // Check that .envyr directory was created
    let envyr_dir = temp_dir.path().join(".envyr");
    assert!(envyr_dir.exists());
    
    // Check that meta.json was created
    let meta_file = envyr_dir.join("meta.json");
    assert!(meta_file.exists());
    
    // Check that Dockerfile was created
    let dockerfile = envyr_dir.join("Dockerfile");
    assert!(dockerfile.exists());
    
    // Check that .dockerignore was created
    let dockerignore = temp_dir.path().join(".dockerignore");
    assert!(dockerignore.exists());
    
    // Verify meta.json content
    let meta_content = fs::read_to_string(meta_file).unwrap();
    assert!(meta_content.contains("Python"));
    assert!(meta_content.contains("main.py"));
}

#[test]
fn test_generate_command_node_project() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a simple Node.js project
    fs::write(temp_dir.path().join("package.json"), r#"
{
    "name": "test-node",
    "main": "index.js",
    "version": "1.0.0"
}
"#).unwrap();
    
    fs::write(temp_dir.path().join("index.js"), "console.log('Hello World');").unwrap();
    
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("generate")
       .arg(temp_dir.path());
    
    cmd.assert()
        .success();
    
    // Check that meta.json was created and contains Node info
    let meta_file = temp_dir.path().join(".envyr").join("meta.json");
    assert!(meta_file.exists());
    
    let meta_content = fs::read_to_string(meta_file).unwrap();
    assert!(meta_content.contains("Node"));
    assert!(meta_content.contains("index.js"));
}

#[test]
fn test_generate_command_shell_project() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a shell script
    fs::write(temp_dir.path().join("script.sh"), r#"#!/bin/bash
echo "Hello World"
"#).unwrap();
    
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("generate")
       .arg(temp_dir.path());
    
    cmd.assert()
        .success();
    
    // Check that meta.json was created
    let meta_file = temp_dir.path().join(".envyr").join("meta.json");
    assert!(meta_file.exists());
    
    let meta_content = fs::read_to_string(meta_file).unwrap();
    assert!(meta_content.contains("script.sh"));
    assert!(meta_content.contains("/bin/bash"));
}

#[test]
fn test_generate_command_nonexistent_directory() {
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("generate")
       .arg("/nonexistent/directory");
    
    cmd.assert()
        .failure();
}

#[test]
fn test_alias_list_empty() {
    let temp_dir = TempDir::new().unwrap();
    
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("alias")
       .arg("list")
       .env("HOME", temp_dir.path());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No aliases found"));
}

#[test]
fn test_run_help() {
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("run")
       .arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Run the package"))
        .stdout(predicate::str::contains("--timeout"))
        .stdout(predicate::str::contains("--interactive"))
        .stdout(predicate::str::contains("--autogen"));
}

#[test]
fn test_run_nonexistent_directory() {
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("run")
       .arg("/nonexistent/directory");
    
    cmd.assert()
        .failure();
}

#[test]
fn test_generate_overwrites_existing() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a simple Python project
    fs::write(temp_dir.path().join("main.py"), "print('Hello')").unwrap();
    
    // Run generate first time
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("generate")
       .arg(temp_dir.path());
    
    cmd.assert()
        .success();
    
    let meta_file = temp_dir.path().join(".envyr").join("meta.json");
    let _first_content = fs::read_to_string(&meta_file).unwrap();
    
    // Modify the project
    fs::write(temp_dir.path().join("app.py"), r#"
if __name__ == "__main__":
    print("New main")
"#).unwrap();
    
    // Run generate second time - should overwrite
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("generate")
       .arg(temp_dir.path());
    
    cmd.assert()
        .success();
    
    let _second_content = fs::read_to_string(&meta_file).unwrap();
    
    // Content should potentially be different due to new entrypoint detection
    // This tests that the command runs successfully when overwriting
    assert!(meta_file.exists());
}

#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("invalid-command");
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_verbose_flag() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.py"), "print('Hello')").unwrap();
    
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("-v")
       .arg("generate")
       .arg(temp_dir.path());
    
    cmd.assert()
        .success();
    // With verbose flag, we expect debug output but hard to test exact content
    // The main thing is that it should still succeed
}

#[test]
fn test_generate_with_overrides() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("script.py"), "print('Hello')").unwrap();
    
    let mut cmd = Command::cargo_bin("envyr").unwrap();
    cmd.arg("generate")
       .arg(temp_dir.path())
       .arg("--name")
       .arg("custom-name")
       .arg("--interpreter")
       .arg("/usr/bin/python3")
       .arg("--entrypoint")
       .arg("script.py");
    
    cmd.assert()
        .success();
    
    let meta_file = temp_dir.path().join(".envyr").join("meta.json");
    let meta_content = fs::read_to_string(meta_file).unwrap();
    
    assert!(meta_content.contains("custom-name"));
    assert!(meta_content.contains("/usr/bin/python3"));
    assert!(meta_content.contains("script.py"));
}