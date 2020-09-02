use {
    clap::{App, AppSettings, Arg},
    shlap::Shell,
    std::str,
};

#[derive(Clone)]
pub struct AppWrapper<'a, 'b>
where
    'a: 'b,
{
    pub app: App<'a, 'b>,
    pub help_msg: String,
    pub version_msg: String,
}

impl<'a, 'b> AppWrapper<'a, 'b>
where
    'a: 'b,
{
    pub fn new(
        app: App<'a, 'b>,
        modify_app: impl FnOnce(App<'a, 'b>) -> App<'a, 'b>,
    ) -> anyhow::Result<Self> {
        let app = app
            .settings(&[
                AppSettings::DisableHelpFlags,
                AppSettings::DisableVersion,
                AppSettings::DisableHelpSubcommand,
            ])
            .arg(
                Arg::with_name("help")
                    .short("h")
                    .long("help")
                    .help("Prints help information"),
            )
            .arg(
                Arg::with_name("version")
                    .short("V")
                    .long("version")
                    .help("Prints version information"),
            );
        let app = modify_app(app);

        let mut help_msg = Vec::new();
        app.write_help(&mut help_msg)?;
        let help_msg = str::from_utf8(&help_msg)?;
        let mut version_msg = Vec::new();
        app.write_long_version(&mut version_msg)?;
        let version_msg = str::from_utf8(&version_msg)?;

        Ok(Self {
            app,
            help_msg: help_msg.into(),
            version_msg: version_msg.into(),
        })
    }

    // FIXME: Fix ZSH not generating the code for completion.
    pub fn completions_script(&mut self, bin_name: &str, shell: &Shell) -> anyhow::Result<String> {
        let mut completions_script = Vec::new();
        self.app
            .gen_completions_to(bin_name, shell.clone().into(), &mut completions_script);
        Ok(str::from_utf8(&completions_script)?.trim_end().into())
    }
}
