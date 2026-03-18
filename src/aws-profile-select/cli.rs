use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    /// Output the shell wrapper function for the given shell.
    /// Add `eval "$(aws-profile-select --init zsh)"` to your shell rc file.
    #[arg(long, value_name = "SHELL")]
    pub init: Option<Shell>,

    /// Skip interactive selection and set the given profile directly.
    /// Useful for scripts and automation.
    #[arg(short, long, value_name = "NAME")]
    pub profile: Option<String>,
}

pub fn shell_init(shell: &Shell) -> &'static str {
    match shell {
        Shell::Bash | Shell::Zsh => {
            r#"asp() {
  eval "$(command aws-profile-select "$@")"
}"#
        }
        Shell::Fish => {
            r#"function asp
  command aws-profile-select $argv | source
end"#
        }
    }
}
