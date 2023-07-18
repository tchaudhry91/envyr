mod envy;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use envy::adapters::fetcher;
use log::debug;
use std::path::PathBuf;

#[derive(Debug, Args)]
struct GlobalOpts {
    #[arg(
        long,
        short,
        help = "relative sub-directory to the project_root, useful if you're working with monorepos.",
        global = true
    )]
    sub_dir: Option<String>,

    #[arg(
        long,
        short,
        default_value = "latest",
        help = "The tag of the package to run. Accepts git tags/commits. Defaults to latest."
    )]
    tag: Option<String>,

    #[arg(
        long,
        short,
        help = "Emit Envy logs to stdout. Useful for debugging. But may spoil pipes.",
        global = true,
        default_value_t = false
    )]
    verbose: bool,

    #[clap(
        long,
        default_value_t = false,
        help = "refresh code cache before running."
    )]
    refresh: bool,
}

#[derive(Debug, Args)]
struct OverrideOpts {
    #[arg(long, short)]
    name: Option<String>,

    #[arg(long, short)]
    interpreter: Option<String>,

    #[arg(long, short = 'x')]
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
        args: OverrideOpts,
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

        #[clap(long, num_args = 0.., help ="Mount the given directory as a volume. Format: host_dir:container_dir. Allows multiples. Only applicable on Docker Executor.")]
        fs_map: Vec<String>,

        #[clap(long, num_args = 0.., help ="Map ports to host system, Format host_port:source_port. Allows multiples. Only applicable on Docker Executor.")]
        port_map: Vec<String>,

        #[clap(flatten)]
        overrides: OverrideOpts,

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

    #[clap(
        help = "The location to the project. Accepts, local filesystem path/git repos.",
        index = 1
    )]
    project_root: String,
}

fn main() -> Result<()> {
    let app = App::parse();

    let mut log_level = log::LevelFilter::Error;

    if app.args.verbose {
        log_level = log::LevelFilter::Debug;
    }
    simplelog::TermLogger::init(
        log_level,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;

    debug!("Started Envy: Parsed args: {:?}", app.args);
    let args = app.args;
    let project_root = app.project_root;
    // TODO: Make this configurable later
    let homedir = home::home_dir().unwrap();
    let envy_root = homedir.join(".envy");

    let p_fetcher = fetcher::get_fetcher(project_root.as_str(), envy_root)?;

    let refresh = args.refresh;
    let tag = args.tag.unwrap_or("latest".to_string());
    let mut path = p_fetcher.fetch(project_root.as_str(), tag.as_str(), refresh)?;

    if args.sub_dir.is_some() {
        path = path.join(args.sub_dir.unwrap());
    }

    let canon_path = std::fs::canonicalize(path)?;

    match app.command {
        Command::Generate { args } => {
            debug!("Running Generator with args: {:?}", args);
            generate(canon_path, args)?;
        }
        Command::Run {
            executor,
            overrides,
            autogen,
            args,
            fs_map,
            port_map,
        } => {
            debug!(
                "Running {:?} executor with autogen={}, fs_map:{:?}, port_map:{:?}, overrides:{:?} and args: {:?}",
                executor, autogen, fs_map, port_map, overrides, args
            );
            let config = RunConfig {
                executor,
                refresh,
                autogen,
                tag,
                fs_map,
                port_map,
                overrides,
                args,
            };
            run(canon_path, config)?;
        }
    }

    Ok(())
}

struct RunConfig {
    executor: envy::meta::Executors,
    refresh: bool,
    autogen: bool,
    tag: String,
    fs_map: Vec<String>,
    port_map: Vec<String>,
    overrides: OverrideOpts,
    args: Vec<String>,
}

fn run(canon_path: PathBuf, config: RunConfig) -> Result<()> {
    if config.autogen {
        let pack_builder = envy::package::Pack::builder(&canon_path)?;
        let pack_builder = override_builder_opts(config.overrides, pack_builder);
        let pack = pack_builder.build()?;
        let generator = envy::meta::Generator::new(pack);
        generator.generate(&canon_path)?;
    }
    match config.executor {
        envy::meta::Executors::Docker => {
            envy::docker::run(
                &canon_path,
                config.refresh,
                config.tag,
                config.fs_map,
                config.port_map,
                config.args,
            )?;
        }
        envy::meta::Executors::Nix => todo!(),
        envy::meta::Executors::Native => todo!(),
    }
    Ok(())
}

fn generate(canon_path: PathBuf, args: OverrideOpts) -> Result<()> {
    let pack_builder = envy::package::Pack::builder(&canon_path)?;
    let pack_builder = override_builder_opts(args, pack_builder);
    let pack = pack_builder.build()?;
    let generator = envy::meta::Generator::new(pack);
    generator.generate(&canon_path)?;
    Ok(())
}

fn override_builder_opts(
    args: OverrideOpts,
    mut pack_builder: envy::package::PackBuilder,
) -> envy::package::PackBuilder {
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
    pack_builder
}
