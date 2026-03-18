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

/**
 * Unit tests for the CLI.
 */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_contains_eval_and_defines_asp() {
        let output = shell_init(&Shell::Bash);
        assert!(output.contains("eval"), "bash init should use eval");
        assert!(output.contains("asp()"), "bash init should define asp()");
    }

    #[test]
    fn zsh_matches_bash() {
        assert_eq!(
            shell_init(&Shell::Zsh),
            shell_init(&Shell::Bash),
            "zsh and bash wrappers should be identical"
        );
    }

    #[test]
    fn fish_uses_function_and_source() {
        let output = shell_init(&Shell::Fish);
        assert!(output.contains("source"), "fish init should pipe to source");
        assert!(
            output.contains("function asp"),
            "fish init should define function asp"
        );
    }

    #[test]
    fn fish_differs_from_bash() {
        assert_ne!(
            shell_init(&Shell::Fish),
            shell_init(&Shell::Bash),
            "fish and bash wrappers should differ"
        );
    }
}
