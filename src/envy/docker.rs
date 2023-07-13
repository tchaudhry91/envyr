use std::path::Path;

use anyhow::Result;
use handlebars::Handlebars;
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

pub fn run(project_root: &Path, args: Vec<String>) -> Result<()> {
    let executor = get_docker_executor()?;
    let image = build_local(project_root)?;
    let command = format!("{} run -it --rm {} {}", executor, image, args.join(" "));
    let mut p = Popen::create(
        command.split_whitespace().collect::<Vec<&str>>().as_slice(),
        PopenConfig::default(),
    )?;
    p.wait()?;
    Ok(())
}

pub fn build_local(project_root: &Path) -> Result<String> {
    let executor = get_docker_executor()?;
    let pack = super::package::Pack::load(project_root)?;
    let image = format!("envy-{}:latest", pack.name);
    let dockerfile_path = project_root.join(".envy").join("Dockerfile");
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
        PopenConfig {
            stdout: subprocess::Redirection::Pipe,
            stderr: subprocess::Redirection::Pipe,
            ..Default::default()
        },
    )?;
    let status = p.wait_timeout(std::time::Duration::from_secs(300))?;

    match status {
        Some(s) => {
            if s.success() {
                Ok(image)
            } else {
                Err(anyhow::anyhow!("Failed to build image."))
            }
        }
        None => Err(anyhow::anyhow!("Failed to build image.")),
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
