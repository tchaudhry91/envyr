mod envy;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Args)]
struct GlobalOpts {
    // Project Root
    #[arg(long, short, global = true, default_value_os_t = PathBuf::from("."))]
    project_root: PathBuf,
}

#[derive(Debug, Args)]
struct GenerateOpts {
    #[arg(long, short)]
    name: Option<String>,

    #[arg(long, short)]
    interpreter: Option<String>,

    #[arg(long, short)]
    entrypoint: Option<PathBuf>,

    #[arg(long = "type", short = 't', value_enum)]
    ptype: Option<envy::package::PType>,
}

#[derive(Debug, Subcommand)]
enum Command {
    // Generate the meta.json file. This will overwrite if re-run.
    #[clap(
        name = "generate",
        about = "Generate the associated meta files. Overwrites if re-run."
    )]
    Generate {
        #[clap(flatten)]
        args: GenerateOpts,
    },

    #[clap(name = "run", about = "Run the package with the given executor.")]
    Run {
        #[clap(long, short, value_enum, default_value_t = envy::meta::Executors::Docker)]
        executor: envy::meta::Executors,
        #[clap(raw = true)]
        args: Vec<String>,
    },
}

#[derive(Parser)]
#[command(name = "envy")]
#[command(author = "Tanmay Chaudhry <tanmay.chaudhry@gmail.com")]
#[command(about="A tool to automagically create 'executable' packages for your scripts.", long_about=None)]
#[command(version = "0.1.0")]
pub struct App {
    #[clap(flatten)]
    args: GlobalOpts,

    #[clap(subcommand)]
    command: Command,
}

fn main() -> Result<()> {
    let app = App::parse();
    let args = app.args;
    let canon_path = std::fs::canonicalize(args.project_root)?;

    match app.command {
        Command::Generate { args } => {
            let mut pack_builder = envy::package::Pack::builder(&canon_path)?;

            // Overwrite global opts if needed
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
            let generator = envy::meta::Generator::new(pack);
            generator.generate(&canon_path)?;
        }
        Command::Run { executor, args } => match executor {
            envy::meta::Executors::Docker => {
                envy::docker::run(&canon_path, args)?;
            }
            envy::meta::Executors::Nix => todo!(),
            envy::meta::Executors::Native => todo!(),
        },
    }

    Ok(())
}
