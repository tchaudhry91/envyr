mod envy;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use envy::{adapters::fetcher, meta::Executors};
use std::path::PathBuf;

#[derive(Debug, Args)]
struct GlobalOpts {
    // Project Root
    #[arg(
        long,
        short,
        help = "Project location. Currently Supported git repositories and local directories.",
        global = true,
        default_value = "."
    )]
    project_root: String,

    #[arg(
        long,
        short,
        help = "relative sub-directory to the project_root, useful if you're working with monorepos.",
        global = true
    )]
    sub_dir: Option<String>,
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
        #[clap(
            long,
            default_value_t = false,
            help = "Attempt to automatically generate the package metadata before running. This overwrites existing metadata."
        )]
        autogen: bool,
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
    // TODO: Make this configurable later
    let homedir = home::home_dir().unwrap();
    let storage_root = homedir.join(".envy");
    let p_fetcher = fetcher::get_fetcher(args.project_root.as_str(), storage_root)?;

    let mut path = p_fetcher.fetch(args.project_root.as_str())?;

    if args.sub_dir.is_some() {
        path = path.join(args.sub_dir.unwrap());
    }

    let canon_path = std::fs::canonicalize(path)?;

    match app.command {
        Command::Generate { args } => {
            generate(canon_path, args)?;
        }
        Command::Run {
            executor,
            autogen,
            args,
        } => {
            run(canon_path, executor, autogen, args)?;
        }
    }

    Ok(())
}

fn run(canon_path: PathBuf, executor: Executors, autogen: bool, args: Vec<String>) -> Result<()> {
    if autogen {
        let pack_builder = envy::package::Pack::builder(&canon_path)?;
        let pack = pack_builder.build()?;
        let generator = envy::meta::Generator::new(pack);
        generator.generate(&canon_path)?;
    }
    match executor {
        envy::meta::Executors::Docker => {
            envy::docker::run(&canon_path, args)?;
        }
        envy::meta::Executors::Nix => todo!(),
        envy::meta::Executors::Native => todo!(),
    }
    Ok(())
}

fn generate(canon_path: PathBuf, args: GenerateOpts) -> Result<()> {
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
    Ok(())
}
