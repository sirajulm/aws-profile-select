mod cli;
mod prompt;
mod sso;

use aws_profile_select::{get_env, parse_profiles, resolve_config_path};
use clap::Parser;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = cli::Cli::parse();

    // --init: print the shell wrapper and exit
    if let Some(shell) = &cli.init {
        println!("{}", cli::shell_init(shell));
        return Ok(());
    }

    let aws_config_file_path = resolve_config_path()?;
    let aws_config_file_str = aws_config_file_path.to_string_lossy();
    let profiles = parse_profiles(&aws_config_file_str)?;

    if profiles.is_empty() {
        return Err(format!(
            "No AWS profiles found in {}.\n\
             Add profiles to your AWS config file or set AWS_CONFIG_FILE to point to a valid config.\n\
             See: https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-files.html",
            aws_config_file_str
        )
        .into());
    }

    let current_aws_profile = get_env("AWS_PROFILE");

    // Determine the chosen profile: either from --profile or interactively
    let chosen_profile = if let Some(name) = cli.profile {
        // Validate that the requested profile exists
        if !profiles.iter().any(|p| p.name == name) {
            let available: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
            return Err(format!(
                "Profile '{}' not found in {}.\nAvailable profiles: {}",
                name,
                aws_config_file_str,
                available.join(", ")
            )
            .into());
        }
        name
    } else {
        match prompt::run_interactive(&profiles, &current_aws_profile) {
            Some(name) => name,
            None => return Ok(()), // user pressed ESC
        }
    };

    sso::handle_sso_login(&profiles, &chosen_profile)?;

    println!("export AWS_PROFILE='{}';", chosen_profile);

    Ok(())
}
