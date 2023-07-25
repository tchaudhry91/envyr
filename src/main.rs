mod envy;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use envy::adapters::fetcher;
use log::debug;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::envy::meta;

#[derive(Debug, Args)]
struct GlobalOpts {
    #[arg(
        long,
        short,
        help = "relative sub-directory to the project_root, useful if you're working with monorepos."
    )]
    sub_dir: Option<String>,

    #[arg(
        long,
        short,
        default_value = "latest",
        help = "The tag of the package to run. Accepts git tags/commits. Defaults to latest."
    )]
    tag: Option<String>,

    #[clap(
        long,
        default_value_t = false,
        help = "refresh code cache before running."
    )]
    refresh: bool,
}

#[derive(Debug, Args, Serialize, Deserialize, Clone)]
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
enum AliasSubcommand {
    #[clap(name = "list", about = "List all aliases.")]
    List,

    #[clap(name = "delete", about = "Delete an existing alias.")]
    Delete {
        #[clap(help = "The name of the alias to delete.")]
        name: String,
    },
}

#[derive(Debug, Subcommand)]
enum Command {
    // Generate the meta.json file. This will overwrite if re-run.
    #[clap(
        name = "generate",
        about = "Generate the associated meta files. Overwrites if re-run."
    )]
    Generate {
        #[clap(help = "The location to the project. Accepts, local filesystem path/git repos.")]
        project_root: String,

        #[clap(flatten)]
        global_opts: GlobalOpts,

        #[clap(flatten)]
        args: OverrideOpts,
    },

    #[clap(name = "alias", about = "Subcommands for aliases.")]
    Alias {
        #[clap(subcommand)]
        subcmd: AliasSubcommand,
    },

    #[clap(name = "run", about = "Run the package with the given executor.")]
    Run {
        #[clap(help = "The location to the project. Accepts, local filesystem path/git repos.")]
        project_root: String,

        #[clap(flatten)]
        global_opts: GlobalOpts,

        #[clap(
            long,
            help = "Upon successful completion, record this run command as an alias. To allow usage of `envy run <alias>` in the future."
        )]
        alias: Option<String>,

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

        #[clap(long, num_args = 0.., help="Environment variables to pass through, leave value empty to pass through the value from the current environment. Format: 'key=value' or 'key' (passwthrough). Allows multiples.")]
        env_map: Vec<String>,

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
    #[clap(subcommand)]
    command: Command,

    #[arg(
        long,
        short,
        help = "Emit Envy logs to stdout. Useful for debugging. But may spoil pipes.",
        default_value_t = false
    )]
    verbose: bool,
}

fn setup_logging(verbose: bool) -> Result<()> {
    let mut log_level = log::LevelFilter::Error;
    if verbose {
        log_level = log::LevelFilter::Debug;
    }

    simplelog::TermLogger::init(
        log_level,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;
    Ok(())
}

fn get_alias_config(envy_root: PathBuf, alias: String) -> Option<RunConfig> {
    let aliases = meta::load_aliases(&envy_root);
    if aliases.is_err() {
        debug!("No aliases found.");
        return None;
    }
    let aliases = aliases.unwrap();
    aliases.get(&alias).cloned()
}

fn fetch(
    envy_root: PathBuf,
    project_root: &str,
    tag: &str,
    refresh: bool,
    subdir: Option<String>,
) -> Result<PathBuf> {
    let p_fetcher = fetcher::get_fetcher(project_root, envy_root)?;
    let mut path = p_fetcher.fetch(project_root, tag, refresh)?;
    if let Some(subdir) = subdir {
        path = path.join(subdir);
    }
    let path = std::fs::canonicalize(path)?;
    Ok(path)
}

fn main() -> Result<()> {
    let app = App::parse();

    // TODO: Make this configurable later
    let homedir = home::home_dir().unwrap();
    let envy_root = homedir.join(".envy");

    setup_logging(app.verbose)?;

    match app.command {
        Command::Generate {
            args,
            project_root,
            global_opts,
        } => {
            let path = fetch(
                envy_root,
                &project_root,
                global_opts.tag.unwrap_or("latest".to_string()).as_str(),
                global_opts.refresh,
                global_opts.sub_dir,
            )?;
            debug!("Running Generator with args: {:?}", args);
            generate(path, args)?;
        }
        Command::Run {
            project_root,
            global_opts,
            executor,
            overrides,
            autogen,
            args,
            fs_map,
            env_map,
            port_map,
            alias,
        } => {
            debug!(
                "Running {:?} executor with autogen={}, fs_map:{:?}, port_map:{:?}, overrides:{:?} and args: {:?}",
                executor, autogen, fs_map, port_map, overrides, args
            );
            if let Some(mut config) = get_alias_config(envy_root.clone(), project_root.clone()) {
                debug!("Found alias config: {:?}", config);
                if !args.is_empty() {
                    config.args = args;
                }
                run(&envy_root, config)?;
                return Ok(()); // Early return if alias is found
            };
            let tag = global_opts.tag.unwrap_or("latest".to_string());
            let config = RunConfig {
                project_root,
                executor,
                refresh: global_opts.refresh,
                autogen,
                tag,
                fs_map,
                port_map,
                sub_dir: global_opts.sub_dir,
                env_map,
                overrides,
                args,
            };
            run(&envy_root, config.clone())?;
            if let Some(alias) = alias {
                meta::store_alias(&envy_root, alias, config)?;
            }
        }
        Command::Alias { subcmd } => match subcmd {
            AliasSubcommand::List => {
                let aliases = meta::load_aliases(&envy_root)?;
                if aliases.is_empty() {
                    println!("No aliases found.");
                    return Ok(());
                }
                for (alias, config) in aliases {
                    println!("{}: {:?}", alias, config.project_root);
                }
            }
            AliasSubcommand::Delete { name } => {
                meta::remove_alias(&envy_root, name)?;
            }
        },
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    project_root: String,
    sub_dir: Option<String>,
    executor: envy::meta::Executors,
    refresh: bool,
    autogen: bool,
    tag: String,
    fs_map: Vec<String>,
    port_map: Vec<String>,
    env_map: Vec<String>,
    overrides: OverrideOpts,
    args: Vec<String>,
}

fn run(envy_root: &Path, config: RunConfig) -> Result<()> {
    let canon_path = fetch(
        envy_root.to_path_buf(),
        &config.project_root,
        config.tag.as_str(),
        config.refresh,
        config.sub_dir,
    )?;
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
                config.env_map,
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
