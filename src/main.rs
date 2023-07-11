mod envy;

use anyhow::Result;
use clap::Parser;
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
}

fn main() -> Result<()> {
    let args = Args::parse();
    let pack = envy::package::PackBuilder::new(args.project_root)?.build()?;
    println!("{:?}", pack);
    Ok(())
}
