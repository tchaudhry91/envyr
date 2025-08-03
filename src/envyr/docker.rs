use std::env;
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use handlebars::Handlebars;
use log::debug;
use log::log_enabled;
use serde::Deserialize;
use serde::Serialize;
use subprocess::{Popen, PopenConfig};

use super::templates::{DOCKER_IGNORE, TEMPLATE_DOCKERFILE};

use super::package::{PType, Pack};
use super::utils;

pub fn check_docker() -> Result<()> {
    let mut p = Popen::create(
        &["docker", "ps"],
        PopenConfig {
            stdout: subprocess::Redirection::Pipe,
            stderr: subprocess::Redirection::Pipe,
            ..Default::default()
        },
    )?;
    p.wait_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
}

pub fn check_podman() -> Result<()> {
    let mut p = Popen::create(
        &["podman", "ps"],
        PopenConfig {
            stdout: subprocess::Redirection::Pipe,
            stderr: subprocess::Redirection::Pipe,
            ..Default::default()
        },
    )?;
    p.wait_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
}

pub fn get_docker_executor() -> Result<String> {
    if check_docker().is_ok() {
        return Ok("docker".to_string());
    } else if check_podman().is_ok() {
        return Ok("podman".to_string());
    }
    Err(anyhow::anyhow!("Docker or Podman not found."))
}

pub fn run(
    project_root: &Path,
    force_rebuild: bool,
    interactive: bool,
    network: Option<String>,
    tag: String,
    fs_map: Vec<String>,
    port_map: Vec<String>,
    env_map: Vec<String>,
    timeout: Option<u32>,
    args: Vec<String>,
    start: Instant,
) -> Result<()> {
    let executor = get_docker_executor()?;

    // Check if the image already exists
    let mut image = get_image_name(project_root, tag.clone())?;

    if force_rebuild || !check_image_existence(&image)? {
        // rebuild
        debug!("Building image: {}", image);
        image = build_local(project_root, tag)?;
    }

    let mut interactive_mode = "";
    if interactive {
        interactive_mode = "-it";
    }

    let mut network_name: String = "".to_string();
    if network.is_some() {
        network_name = format!("--network={}", network.unwrap())
    }

    let command = format!(
        "{} run {} {} {} {} {} --rm {} {}",
        executor,
        interactive_mode,
        network_name,
        get_port_map_str(port_map),
        get_fs_map_str(fs_map),
        get_env_map_str(env_map),
        image,
        args.join(" ")
    );
    debug!("Running command: {}", command);
    debug!("Time Elapsed in Setup: {:?}", start.elapsed());
    let mut p = Popen::create(
        command.split_whitespace().collect::<Vec<&str>>().as_slice(),
        PopenConfig::default(),
    )?;
    
    let status = if let Some(timeout_secs) = timeout {
        debug!("Running with timeout: {} seconds", timeout_secs);
        match p.wait_timeout(std::time::Duration::from_secs(timeout_secs as u64))? {
            Some(status) => status,
            None => {
                debug!("Container execution timed out after {} seconds", timeout_secs);
                p.terminate()?;
                return Err(anyhow::anyhow!("Container execution timed out after {} seconds", timeout_secs));
            }
        }
    } else {
        p.wait()?
    };
    if !status.success() {
        return Err(anyhow::anyhow!("Non-zero exit code"));
    }
    Ok(())
}

fn get_env_map_str(env_map: Vec<String>) -> String {
    if env_map.is_empty() {
        return "".to_string();
    }
    let env_map = env_map
        .iter()
        .map(|x| {
            if x.contains('=') {
                x.to_string()
            } else {
                let val = env::var(x).unwrap_or("".to_string());
                format!("{}={}", x, val)
            }
        })
        .collect::<Vec<String>>();

    let env_map_string = String::from("-e");
    format!("{} {}", env_map_string, env_map.join(" -e "))
}

fn get_port_map_str(port_map: Vec<String>) -> String {
    if port_map.is_empty() {
        return "".to_string();
    }
    let port_map_string = String::from("-p");
    format!("{} {}", port_map_string, port_map.join(" -p "))
}

fn get_fs_map_str(fs_map: Vec<String>) -> String {
    if fs_map.is_empty() {
        return "".to_string();
    }
    let fs_map_string = String::from("-v");
    format!("{} {}", fs_map_string, fs_map.join(" -v "))
}

fn get_image_name(project_root: &Path, tag: String) -> Result<String> {
    let mut name_str = String::from(project_root.to_str().unwrap());
    name_str = name_str.replace(['/', '.'], "-");
    Ok(format!(
        "envyr{}:{}",
        name_str.to_lowercase(),
        tag.to_lowercase()
    ))
}

fn check_image_existence(image: &str) -> Result<bool> {
    let executor = get_docker_executor()?;
    let cmd = std::process::Command::new(executor)
        .arg("images")
        .arg("-q")
        .arg("--filter")
        .arg(format!("reference={}", image))
        .output()?;
    let status = cmd.status;
    let stdout = String::from_utf8(cmd.stdout)?;

    if status.success() && !stdout.is_empty() {
        return Ok(true);
    }
    Ok(false)
}

fn build_local(project_root: &Path, tag: String) -> Result<String> {
    let executor = get_docker_executor()?;

    let image = get_image_name(project_root, tag)?;

    let dockerfile_path = project_root.join(".envyr").join("Dockerfile");
    debug!("Building local docker image: {}", image);
    let mut popen_conf = PopenConfig {
        stdout: subprocess::Redirection::Pipe,
        stderr: subprocess::Redirection::Pipe,
        ..Default::default()
    };
    if log_enabled!(log::Level::Debug) {
        // This prints all logs
        popen_conf = PopenConfig::default();
    }
    let mut p = Popen::create(
        &[
            executor.as_str(),
            "build",
            "-t",
            image.as_str(),
            "-f",
            dockerfile_path.to_str().unwrap(),
            project_root.to_str().unwrap(),
        ],
        popen_conf,
    )?;
    let status = p.wait_timeout(std::time::Duration::from_secs(300))?;

    match status {
        Some(s) => {
            if s.success() {
                Ok(image)
            } else {
                Err(anyhow::anyhow!("Failed to build docker image."))
            }
        }
        None => Err(anyhow::anyhow!("Failed to build docker image.")),
    }
}

pub fn generate_dockerfile(pack: &Pack, project_root: &Path) -> Result<String> {
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
    let interpreter = pack.interpreter.trim_start_matches("/usr/bin/env ");

    let mut d = Data {
        interpreter: interpreter.to_string(),
        entrypoint: pack.entrypoint.to_str().unwrap().to_string(),
        os_deps: pack.deps.clone(),
        ptype: pack.ptype.clone(),
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

pub fn generate_docker_ignore(pack: &Pack) -> Result<String> {
    let mut handlebars = Handlebars::new();
    let source = DOCKER_IGNORE;
    handlebars.register_template_string("dockerignore", source)?;

    #[derive(Default, Serialize, Deserialize)]
    struct Data {
        ptype: PType,
    }

    let d = Data {
        ptype: pack.ptype.clone(),
    };

    Ok(handlebars.render("dockerignore", &d)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_docker_volumes_map() {
        let input = vec!["/root:/root".to_string()];
        assert_eq!(super::get_fs_map_str(input), "-v /root:/root");

        let input = vec!["/root:/root".to_string(), ".app:/app".to_string()];
        assert_eq!(super::get_fs_map_str(input), "-v /root:/root -v .app:/app");
    }

    #[test]
    fn test_get_fs_map_str_empty() {
        let input = vec![];
        assert_eq!(get_fs_map_str(input), "");
    }

    #[test]
    fn test_get_fs_map_str_single() {
        let input = vec!["/host:/container".to_string()];
        assert_eq!(get_fs_map_str(input), "-v /host:/container");
    }

    #[test]
    fn test_get_port_map_str_empty() {
        let input = vec![];
        assert_eq!(get_port_map_str(input), "");
    }

    #[test]
    fn test_get_port_map_str_single() {
        let input = vec!["8080:80".to_string()];
        assert_eq!(get_port_map_str(input), "-p 8080:80");
    }

    #[test]
    fn test_get_port_map_str_multiple() {
        let input = vec!["8080:80".to_string(), "3000:3000".to_string()];
        assert_eq!(get_port_map_str(input), "-p 8080:80 -p 3000:3000");
    }

    #[test]
    fn test_get_env_map_str_empty() {
        let input = vec![];
        assert_eq!(get_env_map_str(input), "");
    }

    #[test]
    fn test_get_env_map_str_key_value() {
        let input = vec!["KEY=value".to_string()];
        assert_eq!(get_env_map_str(input), "-e KEY=value");
    }

    #[test]
    fn test_get_env_map_str_multiple() {
        let input = vec!["KEY1=value1".to_string(), "KEY2=value2".to_string()];
        assert_eq!(get_env_map_str(input), "-e KEY1=value1 -e KEY2=value2");
    }

    #[test]
    fn test_get_env_map_str_passthrough() {
        // Set an environment variable for testing
        std::env::set_var("TEST_VAR", "test_value");
        
        let input = vec!["TEST_VAR".to_string()];
        assert_eq!(get_env_map_str(input), "-e TEST_VAR=test_value");
        
        // Clean up
        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_get_env_map_str_missing_var() {
        // Ensure the variable doesn't exist
        std::env::remove_var("NONEXISTENT_VAR");
        
        let input = vec!["NONEXISTENT_VAR".to_string()];
        assert_eq!(get_env_map_str(input), "-e NONEXISTENT_VAR=");
    }

    #[test]
    fn test_get_image_name() {
        let path = std::path::Path::new("/tmp/test-project");
        let tag = "latest".to_string();
        
        let result = get_image_name(path, tag).unwrap();
        assert_eq!(result, "envyr-tmp-test-project:latest");
    }

    #[test]
    fn test_get_image_name_with_special_chars() {
        let path = std::path::Path::new("/home/user/my.project/with-dots.and.slashes");
        let tag = "v1.0".to_string();
        
        let result = get_image_name(path, tag).unwrap();
        assert_eq!(result, "envyr-home-user-my-project-with-dots-and-slashes:v1.0");
    }

    #[test]
    fn test_get_image_name_case_handling() {
        let path = std::path::Path::new("/tmp/TestProject");
        let tag = "Latest".to_string();
        
        let result = get_image_name(path, tag).unwrap();
        assert_eq!(result, "envyr-tmp-testproject:latest");
    }

    #[test]
    fn test_generate_dockerfile_python() {
        let temp_dir = TempDir::new().unwrap();
        
        let pack = Pack {
            name: "test-python".to_string(),
            interpreter: "/usr/bin/env python".to_string(),
            ptype: PType::Python,
            deps: vec!["curl".to_string()],
            entrypoint: PathBuf::from("main.py"),
        };
        
        // Create requirements.txt to trigger type_reqs
        fs::write(temp_dir.path().join("requirements.txt"), "requests==2.28.1").unwrap();
        
        let dockerfile = generate_dockerfile(&pack, temp_dir.path()).unwrap();
        
        assert!(dockerfile.contains("FROM python:3.11-alpine"));
        assert!(dockerfile.contains("RUN apk add --no-cache  curl "));
        assert!(dockerfile.contains("ADD ./requirements.txt"));
        assert!(dockerfile.contains("RUN pip install"));
        assert!(dockerfile.contains("ENTRYPOINT [\"python\", \"main.py\"]"));
    }

    #[test]
    fn test_generate_dockerfile_node() {
        let temp_dir = TempDir::new().unwrap();
        
        let pack = Pack {
            name: "test-node".to_string(),
            interpreter: "/usr/bin/env node".to_string(),
            ptype: PType::Node,
            deps: vec!["git".to_string()],
            entrypoint: PathBuf::from("index.js"),
        };
        
        // Create package.json to trigger type_reqs
        fs::write(temp_dir.path().join("package.json"), r#"{"name": "test"}"#).unwrap();
        
        let dockerfile = generate_dockerfile(&pack, temp_dir.path()).unwrap();
        
        assert!(dockerfile.contains("FROM node:alpine"));
        assert!(dockerfile.contains("RUN apk add --no-cache  git "));
        assert!(dockerfile.contains("ADD ./package.json"));
        assert!(dockerfile.contains("RUN npm install"));
        assert!(dockerfile.contains("ENTRYPOINT [\"node\", \"index.js\"]"));
    }

    #[test]
    fn test_generate_dockerfile_shell() {
        let temp_dir = TempDir::new().unwrap();
        
        let pack = Pack {
            name: "test-shell".to_string(),
            interpreter: "/bin/bash".to_string(),
            ptype: PType::Shell,
            deps: vec!["wget".to_string()],
            entrypoint: PathBuf::from("script.sh"),
        };
        
        let dockerfile = generate_dockerfile(&pack, temp_dir.path()).unwrap();
        
        assert!(dockerfile.contains("FROM alpine"));
        assert!(dockerfile.contains("RUN apk add --no-cache  wget "));
        assert!(dockerfile.contains("ENTRYPOINT [\"/bin/bash\", \"script.sh\"]"));
    }

    #[test]
    fn test_generate_dockerfile_other() {
        let temp_dir = TempDir::new().unwrap();
        
        let pack = Pack {
            name: "test-other".to_string(),
            interpreter: "/usr/bin/custom".to_string(),
            ptype: PType::Other,
            deps: vec![],
            entrypoint: PathBuf::from("app"),
        };
        
        let dockerfile = generate_dockerfile(&pack, temp_dir.path()).unwrap();
        
        assert!(dockerfile.contains("FROM alpine"));
        assert!(dockerfile.contains("ENTRYPOINT [\"/usr/bin/custom\", \"app\"]"));
    }

    #[test]
    fn test_generate_docker_ignore_python() {
        let pack = Pack {
            name: "test".to_string(),
            interpreter: "/usr/bin/env python".to_string(),
            ptype: PType::Python,
            deps: vec![],
            entrypoint: PathBuf::from("main.py"),
        };
        
        let dockerignore = generate_docker_ignore(&pack).unwrap();
        
        // The current template is generic, not language-specific
        assert!(dockerignore.contains("**/.git"));
        assert!(dockerignore.contains("*.pyc"));
        assert!(dockerignore.contains("**/node_modules"));
    }

    #[test]
    fn test_generate_docker_ignore_node() {
        let pack = Pack {
            name: "test".to_string(),
            interpreter: "/usr/bin/env node".to_string(),
            ptype: PType::Node,
            deps: vec![],
            entrypoint: PathBuf::from("index.js"),
        };
        
        let dockerignore = generate_docker_ignore(&pack).unwrap();
        
        // The current template is generic, not language-specific
        assert!(dockerignore.contains("**/node_modules"));
        assert!(dockerignore.contains("**/.git"));
        assert!(dockerignore.contains("*.pyc"));
    }
}
