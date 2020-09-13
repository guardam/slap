mod app_wrapper;
mod config_checker;
mod dependencies;
mod ident_type;
mod shell;

pub use {dependencies::Dependencies, shell::Shell};

use {
    crate::app_wrapper::AppWrapper,
    anyhow::{bail, Context},
    clap::{App, AppSettings, Arg, ArgMatches, SubCommand, YamlLoader},
    std::{
        convert::TryFrom,
        env,
        io::{self, Read},
        path::Path,
        process, str,
    },
    yaml_rust::Yaml,
};

fn this_cli() -> ArgMatches<'static> {
    App::new("slap")
        .version(clap::crate_version!())
        .author(clap::crate_authors!("\n"))
        .about(clap::crate_description!())
        .settings(&[
            AppSettings::ArgRequiredElseHelp,
            AppSettings::SubcommandRequiredElseHelp,
            AppSettings::ColorAuto,
        ])
        .global_settings(&[
            AppSettings::ColoredHelp,
        ])
        .subcommand(
            SubCommand::with_name("completions")
                .about("Output a completions script for the specified shell")
                .arg(
                    Arg::with_name("SHELL")
                        .help("The target shell")
                        .index(1)
                        .required(true)
                        .possible_values(&Shell::SHELLS),
                )
        )
        .subcommand(
            SubCommand::with_name("parse")
                .about("Check the passed arguments and output code intended to be evaluated by your shell")
                .arg(
                    Arg::with_name("SHELL")
                        .help("The target shell")
                        .index(1)
                        .required(true)
                        .possible_values(&Shell::SHELLS),
                )
                .arg(
                    Arg::with_name("VAR_PREFIX")
                        .help("The prefix to use for the exported variables")
                        .index(2),
                )
                .arg(
                    Arg::with_name("EXTERNAL_ARGS")
                        .help("Arguments to parse using the YAML config passed to STDIN")
                        .index(3)
                        .raw(true)
                        .allow_hyphen_values(true)
                        .multiple(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("deps")
                .about("Check that your sh script dependencies are present in $PATH")
                .arg(
                    Arg::with_name("DEPENDENCIES")
                        .help("Your sh script dependencies")
                        .index(1)
                        .multiple(true)
                        .required(true)
                )
                .arg(
                    Arg::with_name("failed")
                        .help("Lists every dependency not found in $PATH")
                        .long("failed")
                        .short("f")
                        .conflicts_with_all(&["succeded", "all"])
                )
                .arg(
                    Arg::with_name("succeded")
                        .help("Lists the absolute path of every dependency found in $PATH")
                        .long("succeded")
                        .short("s")
                        .conflicts_with_all(&["failed", "all"])
                )
                .arg(
                    Arg::with_name("all")
                        .help("Outputs a JSON containing succeded and failed dependencies (can easily be parsed using jq)")
                        .long("all")
                        .short("a")
                        .conflicts_with_all(&["failed", "succeded"])
                )
                .arg(
                    Arg::with_name("pretty")
                        .help("Pretty print the JSON output")
                        .long("pretty")
                        .short("p")
                        .requires("all")
                ),
        )
        .subcommand(
            SubCommand::with_name("path")
                .about("Gives you the absolute path given the relative path of a script")
                .arg(
                    Arg::with_name("SCRIPT_RELATIVE_PATH")
                        .help("Relative path of your script. For example in bash: `slap path \"${BASH_SOURCE[0]}\"`, in fish: `slap path (status -f)`, in zsh: `slap path \"${(%):-%N}\"`")
                        .index(1)
                        .required(true)
                )
                .arg(
                    Arg::with_name("dir_only")
                        .long("dir-only")
                        .short("d")
                        .help("Gives you the absolute path of the script without including the script name")
                )
        )
        .get_matches()
}

fn path_subcmd(matches: &ArgMatches) -> anyhow::Result<()> {
    let relativep = matches.value_of("SCRIPT_RELATIVE_PATH").unwrap();
    let relativep = Path::new(relativep);
    let script_name = relativep
        .file_name()
        .with_context(|| format!("Can't get file name of path '{}'", relativep.display()))?;

    let dirname = relativep
        .parent()
        .context("Can't get parent path of root (/)")?;
    env::set_current_dir(dirname)
        .with_context(|| format!("Failed to cd in '{}'", dirname.display()))?;
    let mut current_dir = env::current_dir()?;
    if !matches.is_present("dir_only") {
        current_dir = current_dir.join(script_name);
    }

    println!("{}", current_dir.display());

    Ok(())
}

// FIXME: Fix ZSH not generating the code for completion.
fn autocompletions_subcmd(
    matches: &ArgMatches,
    external_app: &mut AppWrapper,
    name: &str,
) -> anyhow::Result<()> {
    let shell = Shell::try_from(matches.value_of("SHELL").unwrap()).unwrap();
    let completions_script = external_app.completions_script(name, &shell)?;
    println!("{}", completions_script);
    Ok(())
}

fn parse_subcmd(
    matches: &ArgMatches,
    name: &str,
    external_app: AppWrapper,
    external_app_subcommands: &[AppWrapper],
    help_msg: &str,
    version_msg: &str,
) -> anyhow::Result<()> {
    let shell = Shell::try_from(matches.value_of("SHELL").unwrap()).unwrap();
    let mut external_args = matches
        .values_of("EXTERNAL_ARGS")
        .map(|x| x.collect::<Vec<_>>())
        .unwrap_or_default();
    let var_prefix = matches.value_of("VAR_PREFIX");

    external_args.insert(0, &name);
    let external_matches = external_app.app.get_matches_from(external_args);

    // We can't output help or version messages to stdout. Only to stderr.
    // The only thing that we can output to stdout is the code that the user will eval.
    if let Some(ref subcmd) = external_matches.subcommand {
        let subcmd_matches = &subcmd.matches;
        let subcmd_name = &subcmd.name;

        macro_rules! handle_subcmd {
            ( $x:ident, $y:ident ) => {
                let subcmd = external_app_subcommands
                    .into_iter()
                    .find(|x| x.app.get_name() == $y)
                    .unwrap();
                eprintln!("{}", subcmd.$x);
                return Ok(());
            };
        }

        if subcmd_name == "help" {
            if subcmd_matches.is_present("help") {
                eprintln!("{}", help_msg);
                return Ok(());
            }
            if subcmd_matches.is_present("version") {
                eprintln!("{}", version_msg);
                return Ok(());
            }
            match subcmd_matches.value_of("SUBCMD") {
                Some(help_subcmd) => {
                    handle_subcmd!(help_msg, help_subcmd);
                }
                None => {
                    eprintln!("{}", external_app.help_msg);
                    return Ok(());
                }
            }
        }
        if subcmd_matches.is_present("help") {
            handle_subcmd!(help_msg, subcmd_name);
        }
        if subcmd_matches.is_present("version") {
            handle_subcmd!(version_msg, subcmd_name);
        }
    } else {
        if external_matches.is_present("help") {
            eprintln!("{}", external_app.help_msg);
            return Ok(());
        }
        if external_matches.is_present("version") {
            eprintln!("{}", external_app.version_msg);
            return Ok(());
        }
    }

    let code = shell.parse(external_matches, var_prefix)?;
    println!("{}", code);

    return Ok(());
}

fn main() -> anyhow::Result<()> {
    let matches = this_cli();

    match Dependencies::check(&matches) {
        Some(Ok(())) => return Ok(()),
        Some(Err(_)) => process::exit(1),
        None => {}
    }

    if let Some(matches) = matches.subcommand_matches("path") {
        return path_subcmd(matches);
    }

    let stdin = {
        let mut stdin = String::new();
        io::stdin().read_to_string(&mut stdin)?;
        if stdin.is_empty() {
            bail!("Received an empty string from STDIN. Check that the YAML config file exists")
        }
        stdin
    };

    let yaml_loader = {
        let mut yaml_loader = YamlLoader::load_from_str(&stdin)?;
        yaml_loader.remove(0)
    };
    let mut yaml_config = yaml_loader.into_hash().context("Invalid YAML config")?;
    config_checker::required(&yaml_config)?;
    config_checker::banned(&yaml_config)?;

    let subcommands_key = YamlLoader::load_from_str("subcommands")
        .ok()
        .map(|mut x| x.remove(0))
        .unwrap();

    let yaml_loader = {
        yaml_config.remove_entry(&subcommands_key);
        let new_yaml_content = {
            let mut buffer = String::new();
            let mut emitter = yaml_rust::YamlEmitter::new(&mut buffer);
            let yaml_hash = Yaml::Hash(yaml_config);
            emitter.dump(&yaml_hash).unwrap();
            buffer
        };
        let mut loader = YamlLoader::load_from_str(&new_yaml_content)?;
        loader.remove(0)
    };

    // Clap doesn't let us redirect --help and --version to stderr so we have to do it manually.
    // This block of code parses the subcommands into a Vec<AppWrapper> and removes from the
    // YamlLoader the subcommands parts so we can add them manually later to the `external_app`
    // clap::App.
    let external_app_subcommands = {
        let yaml_config = yaml_loader.as_hash().unwrap();
        if yaml_config.contains_key(&subcommands_key) {
            let subcommands = yaml_config.get(&subcommands_key).unwrap();
            let external_app_subcommands = subcommands
                .as_vec()
                .context("Subcommands object must be an array of maps")?
                .into_iter()
                .map(SubCommand::from_yaml)
                .map(|x| AppWrapper::new(x, |app| app));
            let mut xs = Vec::new();
            for subcmd in external_app_subcommands {
                xs.push(subcmd?);
            }
            xs
        } else {
            Default::default()
        }
    };

    let external_app_help_subcmd = AppWrapper::new(SubCommand::with_name("help"), |app: App| {
        app.arg(Arg::with_name("SUBCMD").required(false))
            .about("Prints this message or the help of the given subcommand(s)")
    })?;
    let external_app = App::from(&yaml_loader);
    let name = external_app.get_name().to_owned();
    let mut external_app = AppWrapper::new(external_app.bin_name(&name), {
        let subcommand = external_app_help_subcmd.app;
        let subcommands = external_app_subcommands.clone().into_iter().map(|x| x.app);
        move |app: App| app.subcommand(subcommand).subcommands(subcommands)
    })?;

    if matches.subcommand_matches("completions").is_some() {
        return autocompletions_subcmd(&matches, &mut external_app, &name);
    }

    if let Some(matches) = matches.subcommand_matches("parse") {
        return parse_subcmd(
            matches,
            &name,
            external_app,
            &external_app_subcommands,
            &external_app_help_subcmd.help_msg,
            &external_app_help_subcmd.version_msg,
        );
    }

    Ok(())
}
