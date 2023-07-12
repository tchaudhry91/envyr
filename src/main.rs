mod envy;

use anyhow::Result;
use clap::Parser;
use serde_json;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "envy")]
#[command(author = "Tanmay Chaudhry <tanmay.chaudhry@gmail.com")]
#[command(version = "0.1.0")]
#[command(about="A tool to automagically create 'executable' packages for your scripts.", long_about=None)]
struct Args {
    // Project Root
    #[arg(long, short, default_value_os_t = PathBuf::from("."))]
    project_root: PathBuf,

    #[arg(long, short)]
    name: Option<String>,

    #[arg(long, short)]
    interpreter: Option<String>,

    #[arg(long, short)]
    entrypoint: Option<PathBuf>,

    #[arg(long = "type", short = 't', value_enum)]
    ptype: Option<envy::package::PType>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let canon_path = std::fs::canonicalize(&args.project_root)?;
    let mut pack_builder = envy::package::Pack::builder(canon_path)?;

    // Overwrite params if needed
    if let Some(name) = args.name {
        pack_builder = pack_builder.name(name);
    }

    if let Some(interpreter) = args.interpreter {
        pack_builder = pack_builder.interpreter(interpreter);
    }

    if let Some(entrypoint) = args.entrypoint {
        pack_builder = pack_builder.entrypoint(entrypoint);
    }

    if let Some(ptype) = args.ptype {
        pack_builder = pack_builder.ptype(ptype);
    }

    let pack = pack_builder.build()?;

    println!("{}", serde_json::to_string_pretty(&pack)?);
    Ok(())
}
