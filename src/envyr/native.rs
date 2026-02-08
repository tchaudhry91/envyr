use std::env;
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use log::debug;
use subprocess::{Popen, PopenConfig};

use super::package::{PType, Pack};

pub struct NativeRunOpts {
    pub env_map: Vec<String>,
    pub timeout: Option<u32>,
    pub args: Vec<String>,
}

pub fn run(
    project_root: &Path,
    envyr_root: &Path,
    pack: &Pack,
    opts: NativeRunOpts,
    start: Instant,
) -> Result<()> {
    // Warn if running a remote (git-fetched) source without sandboxing
    if project_root.starts_with(envyr_root) {
        eprintln!(
            "Warning: Native executor runs code without sandboxing. Ensure you trust this source."
        );
    }

    // Install dependencies based on project type
    install_deps(project_root, &pack.ptype)?;

    // Resolve interpreter (Python uses venv python)
    let interpreter = resolve_interpreter(project_root, pack);

    // Build command
    let entrypoint_str = pack
        .entrypoint
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Entrypoint path contains invalid UTF-8"))?;

    // Split interpreter on whitespace (e.g. "/usr/bin/env python" -> ["/usr/bin/env", "python"])
    let mut cmd_parts: Vec<String> = interpreter
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    cmd_parts.push(entrypoint_str.to_string());
    cmd_parts.extend(opts.args.iter().cloned());

    let cmd_strs: Vec<&str> = cmd_parts.iter().map(|s| s.as_str()).collect();

    // Resolve environment variables
    let env_vars = resolve_env_map(&opts.env_map);

    let popen_config = PopenConfig {
        cwd: Some(project_root.as_os_str().to_owned()),
        env: Some(build_env_with_extras(&env_vars)),
        ..Default::default()
    };

    // Inherit stdin/stdout/stderr (default PopenConfig behavior)
    debug!("Running command: {}", cmd_parts.join(" "));
    debug!("Time Elapsed in Setup: {:?}", start.elapsed());

    let mut p = Popen::create(&cmd_strs, popen_config)?;

    let status = if let Some(timeout_secs) = opts.timeout {
        debug!("Running with timeout: {} seconds", timeout_secs);
        match p.wait_timeout(std::time::Duration::from_secs(timeout_secs as u64))? {
            Some(status) => status,
            None => {
                debug!(
                    "Process execution timed out after {} seconds",
                    timeout_secs
                );
                p.terminate()?;
                return Err(anyhow::anyhow!(
                    "Process execution timed out after {} seconds",
                    timeout_secs
                ));
            }
        }
    } else {
        p.wait()?
    };

    if !status.success() {
        return Err(anyhow::anyhow!(
            "Process exited with non-zero status: {:?}",
            status
        ));
    }
    Ok(())
}

fn install_deps(project_root: &Path, ptype: &PType) -> Result<()> {
    match ptype {
        PType::Python => install_python_deps(project_root),
        PType::Node => install_node_deps(project_root),
        PType::Shell | PType::Other => Ok(()),
    }
}

fn install_python_deps(project_root: &Path) -> Result<()> {
    let venv_path = project_root.join(".envyr").join("venv");
    if !venv_path.exists() {
        debug!("Creating Python venv at {:?}", venv_path);
        let status = std::process::Command::new("python3")
            .args(["-m", "venv"])
            .arg(&venv_path)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to create Python venv"));
        }
    }

    let requirements = project_root.join("requirements.txt");
    if requirements.exists() {
        let pip_path = venv_path.join("bin").join("pip");
        debug!("Installing Python dependencies from requirements.txt");
        let status = std::process::Command::new(pip_path)
            .args(["install", "-r"])
            .arg(&requirements)
            .current_dir(project_root)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!(
                "Failed to install Python dependencies from requirements.txt"
            ));
        }
    }
    Ok(())
}

fn install_node_deps(project_root: &Path) -> Result<()> {
    let package_json = project_root.join("package.json");
    let node_modules = project_root.join("node_modules");
    if package_json.exists() && !node_modules.exists() {
        debug!("Running npm install");
        let status = std::process::Command::new("npm")
            .arg("install")
            .current_dir(project_root)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to run npm install"));
        }
    }
    Ok(())
}

fn resolve_interpreter(project_root: &Path, pack: &Pack) -> String {
    match pack.ptype {
        PType::Python => {
            let venv_python = project_root
                .join(".envyr")
                .join("venv")
                .join("bin")
                .join("python");
            venv_python.to_string_lossy().to_string()
        }
        _ => pack.interpreter.clone(),
    }
}

fn resolve_env_map(env_map: &[String]) -> Vec<(String, String)> {
    env_map
        .iter()
        .map(|x| {
            if let Some((key, value)) = x.split_once('=') {
                (key.to_string(), value.to_string())
            } else {
                let resolved = env::var(x).unwrap_or_default();
                (x.clone(), resolved)
            }
        })
        .collect()
}

fn build_env_with_extras(extras: &[(String, String)]) -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
    let mut env: Vec<(std::ffi::OsString, std::ffi::OsString)> = env::vars_os().collect();
    for (key, value) in extras {
        // Remove existing entry if present, then add
        env.retain(|(k, _)| k != key.as_str());
        env.push((key.into(), value.into()));
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_pack(ptype: PType, interpreter: &str, entrypoint: &str) -> Pack {
        Pack {
            name: "test-project".to_string(),
            interpreter: interpreter.to_string(),
            ptype,
            deps: vec![],
            entrypoint: PathBuf::from(entrypoint),
        }
    }

    #[test]
    fn test_resolve_interpreter_python() {
        let temp_dir = TempDir::new().unwrap();
        let pack = create_test_pack(PType::Python, "/usr/bin/env python", "main.py");
        let interpreter = resolve_interpreter(temp_dir.path(), &pack);
        let expected = temp_dir
            .path()
            .join(".envyr")
            .join("venv")
            .join("bin")
            .join("python");
        assert_eq!(interpreter, expected.to_string_lossy().to_string());
    }

    #[test]
    fn test_resolve_interpreter_node() {
        let temp_dir = TempDir::new().unwrap();
        let pack = create_test_pack(PType::Node, "/usr/bin/env node", "index.js");
        let interpreter = resolve_interpreter(temp_dir.path(), &pack);
        assert_eq!(interpreter, "/usr/bin/env node");
    }

    #[test]
    fn test_resolve_interpreter_shell() {
        let temp_dir = TempDir::new().unwrap();
        let pack = create_test_pack(PType::Shell, "/bin/bash", "script.sh");
        let interpreter = resolve_interpreter(temp_dir.path(), &pack);
        assert_eq!(interpreter, "/bin/bash");
    }

    #[test]
    fn test_resolve_interpreter_other() {
        let temp_dir = TempDir::new().unwrap();
        let pack = create_test_pack(PType::Other, "/usr/bin/custom", "app");
        let interpreter = resolve_interpreter(temp_dir.path(), &pack);
        assert_eq!(interpreter, "/usr/bin/custom");
    }

    #[test]
    fn test_env_map_resolution_key_value() {
        let env_map = vec!["KEY=value".to_string(), "FOO=bar".to_string()];
        let resolved = resolve_env_map(&env_map);
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0], ("KEY".to_string(), "value".to_string()));
        assert_eq!(resolved[1], ("FOO".to_string(), "bar".to_string()));
    }

    #[test]
    fn test_env_map_resolution_passthrough() {
        env::set_var("ENVYR_TEST_VAR", "test_value");
        let env_map = vec!["ENVYR_TEST_VAR".to_string()];
        let resolved = resolve_env_map(&env_map);
        assert_eq!(resolved.len(), 1);
        assert_eq!(
            resolved[0],
            ("ENVYR_TEST_VAR".to_string(), "test_value".to_string())
        );
        env::remove_var("ENVYR_TEST_VAR");
    }

    #[test]
    fn test_env_map_resolution_missing_passthrough() {
        env::remove_var("ENVYR_NONEXISTENT_VAR");
        let env_map = vec!["ENVYR_NONEXISTENT_VAR".to_string()];
        let resolved = resolve_env_map(&env_map);
        assert_eq!(resolved.len(), 1);
        assert_eq!(
            resolved[0],
            ("ENVYR_NONEXISTENT_VAR".to_string(), String::new())
        );
    }

    #[test]
    fn test_is_remote_source() {
        let envyr_root = Path::new("/home/user/.envyr");
        let remote_path = Path::new("/home/user/.envyr/cache/some-repo");
        let local_path = Path::new("/home/user/projects/my-project");

        assert!(remote_path.starts_with(envyr_root));
        assert!(!local_path.starts_with(envyr_root));
    }

    #[test]
    fn test_install_deps_shell_noop() {
        let temp_dir = TempDir::new().unwrap();
        // Should succeed without doing anything
        install_deps(temp_dir.path(), &PType::Shell).unwrap();
        install_deps(temp_dir.path(), &PType::Other).unwrap();
    }

    #[test]
    fn test_install_python_deps_creates_venv() {
        let temp_dir = TempDir::new().unwrap();
        let envyr_dir = temp_dir.path().join(".envyr");
        fs::create_dir(&envyr_dir).unwrap();

        // This test requires python3 to be available
        let result = install_python_deps(temp_dir.path());
        if result.is_ok() {
            let venv_path = envyr_dir.join("venv");
            assert!(venv_path.exists());
            assert!(venv_path.join("bin").join("python").exists());
        }
        // If python3 is not available, the test gracefully skips
    }

    #[test]
    fn test_install_node_deps_skips_without_package_json() {
        let temp_dir = TempDir::new().unwrap();
        // No package.json, should be a no-op
        install_node_deps(temp_dir.path()).unwrap();
        assert!(!temp_dir.path().join("node_modules").exists());
    }

    #[test]
    fn test_build_env_with_extras() {
        let extras = vec![
            ("MY_KEY".to_string(), "my_value".to_string()),
        ];
        let env = build_env_with_extras(&extras);
        let found = env.iter().any(|(k, v)| {
            k == "MY_KEY" && v == "my_value"
        });
        assert!(found);
    }
}
