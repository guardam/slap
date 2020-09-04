mod app_wrapper;
mod config_checker;
mod ident_type;
mod shell;

pub use shell::Shell;

use {
    crate::app_wrapper::AppWrapper,
    anyhow::{bail, Context},
    clap::{App, AppSettings, Arg, ArgMatches, SubCommand, YamlLoader},
    std::{
        convert::TryFrom,
        io::{self, Read},
        str,
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
        ])
        .arg(
            Arg::with_name("SHELL")
                .help("The target shell")
                .index(1)
                .required(true)
                .possible_values(&Shell::SHELLS),
        )
        .subcommand(
            SubCommand::with_name("completions")
                .about("Output a completions script for the specified shell"),
        )
        .subcommand(
            SubCommand::with_name("parse")
                .about("Check the passed arguments and output code intended to be evaluated by your shell")
                .arg(
                    Arg::with_name("VAR_PREFIX")
                        .help("The prefix to use for the exported variables")
                        .index(1),
                )
                .arg(
                    Arg::with_name("EXTERNAL_ARGS")
                        .help("Arguments to parse using the YAML config passed to STDIN")
                        .index(2)
                        .raw(true)
                        .allow_hyphen_values(true)
                        .multiple(true),
                ),
        )
        .get_matches()
}

fn run() -> anyhow::Result<()> {
    let matches = this_cli();
    let shell = Shell::try_from(matches.value_of("SHELL").unwrap()).unwrap();

    // Clap doesn't let us redirect --help and --version to stderr so we have to do it manually.
    // This block of code parses the subcommands into a Vec<AppWrapper> and removes from the
    // YamlLoader the subcommands parts so we can add them manually later to the `external_app`
    // clap::App.
    let stdin = {
        let mut stdin = String::new();
        io::stdin().read_to_string(&mut stdin)?;
        if stdin.is_empty() {
            bail!("Received an empty string from STDIN. Check that the YAML config file exists")
        }
        stdin
    };
    let mut yaml_loader = YamlLoader::load_from_str(&stdin)?[0].clone();
    let yaml_config = yaml_loader
        .clone()
        .into_hash()
        .context("Invalid YAML config")?;
    config_checker::required(&yaml_config)?;
    config_checker::banned(&yaml_config)?;
    let external_app_subcommands = {
        let subcommands_loader = YamlLoader::load_from_str("subcommands").ok();
        let subcommands_key = subcommands_loader.as_ref().map(|x| &x[0]).unwrap();
        if let Some(subcommands) = yaml_config.get(subcommands_key) {
            let external_app_subcommands = {
                let external_app_subcommands = subcommands
                    .as_vec()
                    .context("Subcommands object must be an array of maps")?
                    .into_iter()
                    .map(SubCommand::from_yaml)
                    .map(|x| Ok::<_, anyhow::Error>(AppWrapper::new(x, |app| app)?));
                let mut xs = Vec::new();
                for subcmd in external_app_subcommands {
                    xs.push(subcmd?);
                }
                xs
            };

            yaml_loader = {
                let mut yaml_config = yaml_config.clone();
                yaml_config.remove_entry(subcommands_key);
                let new_yaml_content = {
                    let mut buffer = String::new();
                    let mut emitter = yaml_rust::YamlEmitter::new(&mut buffer);
                    let yaml_hash = Yaml::Hash(yaml_config);
                    emitter.dump(&yaml_hash).unwrap();
                    buffer
                };
                YamlLoader::load_from_str(&new_yaml_content)?[0].clone()
            };

            external_app_subcommands
        } else {
            Vec::new()
        }
    };

    let external_app_help_subcmd = AppWrapper::new(SubCommand::with_name("help"), |app: App| {
        app.arg(Arg::with_name("SUBCMD").required(false))
            .about("Prints this message or the help of the given subcommand(s)")
    })?;
    let external_app = App::from(&yaml_loader);
    let name = external_app.get_name().to_owned();
    let mut external_app = {
        let external_app = external_app.bin_name(&name);
        let external_app_help_subcmd_app = external_app_help_subcmd.app.clone();
        let external_app_subcommands_clone = external_app_subcommands.clone();
        AppWrapper::new(external_app, move |app: App| {
            app.subcommand(external_app_help_subcmd_app)
                .subcommands(external_app_subcommands_clone.into_iter().map(|x| x.app))
        })?
    };

    // FIXME: Fix ZSH not generating the code for completion.
    if matches.subcommand_matches("completions").is_some() {
        let completions_script = external_app.completions_script(&name, &shell)?;
        println!("{}", completions_script);
        return Ok(());
    }

    if let Some(matches) = matches.subcommand_matches("parse") {
        let mut external_args = matches
            .values_of("EXTERNAL_ARGS")
            .map(|x| x.collect::<Vec<_>>())
            .unwrap_or_default();
        let var_prefix = matches.value_of("VAR_PREFIX").map(|x| x.to_owned());

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
                    eprintln!("{}", external_app_help_subcmd.help_msg);
                    return Ok(());
                }
                if subcmd_matches.is_present("version") {
                    eprintln!("{}", external_app_help_subcmd.version_msg);
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

    Ok(())
}

fn main() -> anyhow::Result<()> {
    run()
}
