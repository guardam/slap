use {
    crate::ident_type::IdentType,
    anyhow::{bail, Context},
    std::convert::TryFrom,
};

#[derive(Clone)]
pub enum Shell {
    Bash,
    Elvish,
    Fish,
    PowerShell,
    Zsh,
}

impl Shell {
    pub const SHELLS: [&'static str; 5] = ["bash", "elvish", "fish", "pwsh", "zsh"];

    fn ident_check<'a>(&self, s: &'a str, ident_type: &IdentType) -> anyhow::Result<&'a str> {
        let re = ident_type.re(self);
        if re.is_match(s) {
            Ok(s)
        } else {
            bail!(
                "`{}` is not a valid identifier, it must conform to this regex: `{}`",
                s,
                re.to_string(),
            )
        }
    }

    fn str_escape(&self, s: &str) -> String {
        let mut s = s.replace(
            '\'',
            match self {
                Self::Fish => "\\'",
                Self::Bash | Self::Elvish | Self::Zsh => r#"'"'"'"#,
                Self::PowerShell => "''",
            },
        );
        s.insert(0, '\'');
        s.push('\'');
        s
    }

    fn array_escape(&self, xs: &[&str]) -> String {
        let mut s = match self {
            Self::Fish => String::new(),
            Self::Bash | Self::Zsh => "(".into(),
            Self::Elvish => "[".into(),
            Self::PowerShell => "@(".into(),
        };
        let len = xs.len();
        for (idx, x) in xs.into_iter().enumerate() {
            s.push_str(&self.str_escape(x));
            if idx < len - 1 {
                if let Self::PowerShell = self {
                    s.push(',');
                }
                s.push(' ');
            }
        }
        match self {
            Self::Bash | Self::PowerShell | Self::Zsh => s.push(')'),
            Self::Elvish => s.push(']'),
            _ => {}
        }
        s
    }

    fn assignment(&self, var_ident: &str, val: &str) -> String {
        match self {
            Self::Fish => format!("set {} {}", var_ident, val),
            Self::Bash | Self::Zsh => format!("{}={}", var_ident, val),
            Self::Elvish => format!("{} = {}", var_ident, val),
            Self::PowerShell => format!(
                "Set-Variable -Name {} -Value {}",
                self.str_escape(var_ident),
                val
            ),
        }
    }

    // NOTE: In the future we could add an option to use associative arrays instead of arrays for
    // elvish and powershell.
    fn parse_(
        &self,
        matches: &clap::ArgMatches,
        var_prefix: Option<&str>,
        // Subcommands are recursive, used to mantain the subcommand prefix for variables.
        subcommands_prefixes: Option<Vec<&str>>,
    ) -> anyhow::Result<String> {
        let vprefix = {
            let mut s = String::new();
            if let Some(vprefix) = var_prefix {
                s = self.ident_check(vprefix, &IdentType::Head)?.into();
            }
            s
        };

        let subcommands_ident = if let Some(ref xs) = subcommands_prefixes {
            format!("{}_", xs.join("_"))
        } else {
            String::new()
        };
        let subcommands_ident = if subcommands_ident.is_empty() {
            subcommands_ident
        } else {
            self.ident_check(&subcommands_ident, &IdentType::Tail)?
                .into()
        };

        let mut buffer = String::new();

        if subcommands_prefixes.is_none() {
            buffer.push_str(
                &self.assignment(&format!("{}success", vprefix), &self.str_escape("true")),
            );
            buffer.push('\n');
        }

        if let Some(ref usage) = matches.usage {
            let clap_usage = self.str_escape(usage);
            buffer.push_str(&self.assignment(
                &format!("{}{}usage", vprefix, subcommands_ident),
                &clap_usage,
            ));
            buffer.push('\n');
        }

        if let Some(ref subcommand) = matches.subcommand {
            let clap_subcommand = self.str_escape(&subcommand.name);
            buffer.push_str(&self.assignment(
                &format!("{}{}subcommand", vprefix, subcommands_ident),
                &clap_subcommand,
            ));
            buffer.push('\n');

            let mut subcommands_prefixes = subcommands_prefixes.unwrap_or_default();
            subcommands_prefixes.push(&subcommand.name);
            buffer.push_str(&self.parse_(
                &subcommand.matches,
                var_prefix,
                Some(subcommands_prefixes),
            )?)
        }

        for (name, arg) in &matches.args {
            let arg_name = self.ident_check(name, &IdentType::Tail)?;

            let clap_occurs = self.str_escape(&arg.occurs.to_string());
            buffer.push_str(&self.assignment(
                &format!("{}{}{}_occurs", vprefix, subcommands_ident, arg_name),
                &clap_occurs,
            ));
            buffer.push('\n');

            let clap_indices = arg
                .indices
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>();
            let clap_indices = clap_indices.iter().map(|x| x.as_str()).collect::<Vec<_>>();
            let clap_indices = self.array_escape(&clap_indices);
            buffer.push_str(&self.assignment(
                &format!("{}{}{}_indices", vprefix, subcommands_ident, arg_name),
                &clap_indices,
            ));
            buffer.push('\n');

            let mut clap_vals = Vec::new();
            for val in &arg.vals {
                clap_vals.push(val.to_str().context("String contains invalid UTF-8 data")?);
            }
            let clap_vals = self.array_escape(&clap_vals);
            buffer.push_str(&self.assignment(
                &format!("{}{}{}_vals", vprefix, subcommands_ident, arg_name),
                &clap_vals,
            ));
            buffer.push('\n');
        }

        Ok(buffer.trim_end().into())
    }

    pub fn parse(
        &self,
        matches: clap::ArgMatches,
        var_prefix: Option<&str>,
    ) -> anyhow::Result<String> {
        self.parse_(&matches, var_prefix, None)
    }
}

impl TryFrom<&str> for Shell {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> anyhow::Result<Self> {
        match s {
            "bash" => Ok(Shell::Bash),
            "elvish" => Ok(Shell::Elvish),
            "fish" => Ok(Shell::Fish),
            "pwsh" => Ok(Shell::PowerShell),
            "zsh" => Ok(Shell::Zsh),
            _ => bail!("Shell must be one of {:?}", Shell::SHELLS),
        }
    }
}

impl<'a> Into<clap::Shell> for &'a Shell {
    fn into(self) -> clap::Shell {
        match *self {
            Shell::Bash => clap::Shell::Bash,
            Shell::Elvish => clap::Shell::Elvish,
            Shell::Fish => clap::Shell::Fish,
            Shell::PowerShell => clap::Shell::PowerShell,
            Shell::Zsh => clap::Shell::Zsh,
        }
    }
}
