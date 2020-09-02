use {crate::shell::Shell, lazy_static::lazy_static, regex::Regex};

lazy_static! {
    static ref IDENT_CANNOT_START_WITH_NUM_RE: Regex =
        Regex::new("^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
    static ref IDENT_CAN_START_WITH_NUM_RE: Regex = Regex::new("^[a-zA-Z0-9_]+$").unwrap();
    static ref IDENT_ANY_RE: Regex = Regex::new("^.+$").unwrap();
}

pub enum IdentType {
    Head,
    Tail,
}

impl IdentType {
    // Regex for validating the (head or tail) identifier.
    pub fn re(&self, shell: &Shell) -> &'static Regex {
        match self {
            Self::Head => match shell {
                Shell::Bash | Shell::Zsh => &*IDENT_CANNOT_START_WITH_NUM_RE,
                Shell::Elvish | Shell::Fish => &*IDENT_CAN_START_WITH_NUM_RE,
                Shell::PowerShell => &*IDENT_ANY_RE,
            },
            Self::Tail => match shell {
                Shell::Bash | Shell::Zsh | Shell::Elvish | Shell::Fish => {
                    &*IDENT_CAN_START_WITH_NUM_RE
                }
                Shell::PowerShell => &*IDENT_ANY_RE,
            },
        }
    }
}
