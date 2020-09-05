use {
    anyhow::anyhow,
    clap::ArgMatches,
    std::{
        collections::HashMap,
        fmt::{self, Display, Formatter},
        path::PathBuf,
    },
};

pub struct Dependencies<'a> {
    failed_deps: Vec<&'a str>,
}

impl<'a> Display for Dependencies<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.failed_deps.len() {
            0 => {}
            1 => {
                writeln!(
                    f,
                    "Required dependency '{}' not found in $PATH",
                    self.failed_deps[0],
                )?;
            }
            _ => {
                writeln!(f, "These required dependencies were not found in $PATH:")?;
                for dep in self.failed_deps.iter() {
                    writeln!(f, "    {}", dep)?;
                }
            }
        }
        Ok(())
    }
}

impl<'a> Dependencies<'a> {
    fn parse(deps: &'_ [&'a str]) -> HashMap<&'a str, Option<PathBuf>> {
        let mut map = HashMap::new();
        for dep in deps.into_iter() {
            map.insert(*dep, which::which(dep).ok());
        }
        map
    }

    #[cfg(feature = "color")]
    fn print_colored(&self) -> anyhow::Result<()> {
        use {
            std::io::Write,
            termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor},
        };

        let mut stderr = StandardStream::stderr(
            if atty::is(atty::Stream::Stdout) || atty::is(atty::Stream::Stderr) {
                ColorChoice::Auto
            } else {
                ColorChoice::Never
            },
        );
        let mut color_spec = ColorSpec::new();
        stderr.set_color(color_spec.set_fg(Some(Color::Red)).set_bold(true))?;

        match self.failed_deps.len() {
            0 => {}
            1 => {
                write!(&mut stderr, "error: ")?;
                stderr.set_color(color_spec.set_fg(None).set_bold(false))?;
                write!(&mut stderr, "Required dependency ")?;
                stderr.set_color(color_spec.set_fg(Some(Color::Green)).set_bold(true))?;
                write!(&mut stderr, "{}", self.failed_deps[0])?;
                stderr.set_color(color_spec.set_fg(None).set_bold(false))?;
                write!(&mut stderr, " not found in ")?;
                stderr.set_color(color_spec.set_fg(Some(Color::Cyan)).set_bold(true))?;
                writeln!(&mut stderr, "$PATH")?;
            }
            _ => {
                write!(&mut stderr, "error: ")?;
                stderr.set_color(color_spec.set_fg(None).set_bold(false))?;
                write!(
                    &mut stderr,
                    "These required dependencies were not found in "
                )?;
                stderr.set_color(color_spec.set_fg(Some(Color::Cyan)).set_bold(true))?;
                write!(&mut stderr, "$PATH")?;
                stderr.set_color(color_spec.set_fg(None).set_bold(false))?;
                writeln!(&mut stderr, ":")?;
                stderr.set_color(color_spec.set_fg(Some(Color::Green)).set_bold(true))?;
                for dep in self.failed_deps.iter() {
                    writeln!(&mut stderr, "    {}", dep)?;
                }
            }
        }
        Ok(())
    }

    fn print(&self) {
        eprintln!("{}", self);
    }

    pub fn check(matches: &'a ArgMatches) -> Option<anyhow::Result<()>> {
        macro_rules! exit {
            ( $x:expr ) => {
                return Some(if $x == 0 {
                    Ok(())
                } else {
                    Err(anyhow!(
                        "1 or more required dependencies were not found in $PATH"
                    ))
                });
            };
        }

        if let Some(matches) = matches.subcommand_matches("deps") {
            let deps: Vec<&'a str> = matches
                .values_of("DEPENDENCIES")
                .unwrap()
                .collect::<Vec<_>>();
            let results = Self::parse(&deps);

            if matches.is_present("succeded") {
                let mut failed_deps = 0;
                for (_, path) in results {
                    if let Some(path) = path {
                        println!("{}", path.display());
                    } else {
                        failed_deps += 1;
                    }
                }
                exit!(failed_deps);
            }

            if matches.is_present("failed") {
                let mut failed_deps = 0;
                for (dep, path) in results {
                    if path.is_none() {
                        failed_deps += 1;
                        println!("{}", dep);
                    }
                }
                exit!(failed_deps);
            }

            if matches.is_present("all") {
                let mut succeded = HashMap::new();
                let mut failed = Vec::new();
                for (k, v) in results {
                    if let Some(path) = v {
                        succeded.insert(k, path);
                    } else {
                        failed.push(k);
                    }
                }
                let json_val = serde_json::json!({
                    "succeded": succeded,
                    "failed": failed,
                });
                let json_str = if matches.is_present("pretty") {
                    serde_json::to_string_pretty(&json_val).unwrap()
                } else {
                    json_val.to_string()
                };
                println!("{}", json_str);

                exit!(failed.len());
            }

            let mut failed_deps = Vec::new();
            for (k, v) in results {
                if v.is_none() {
                    failed_deps.push(k);
                }
            }
            let len = failed_deps.len();

            let s = Self { failed_deps };
            if cfg!(feature = "color") {
                if let Err(e) = s.print_colored() {
                    return Some(Err(e));
                }
            } else {
                s.print();
            }

            exit!(len);
        } else {
            None
        }
    }
}
