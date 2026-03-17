use std::collections::HashMap;
use std::error::Error;
use std::process::Command;
use aws_profile_select::{get_env, parse_profiles};
use dialoguer::{Select, theme::ColorfulTheme};

fn select_environment(environments: &[String]) -> Option<usize> {
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select the environment")
        .items(environments)
        .default(0)
        .max_length(10)
        .interact_opt()
        .ok()
        .flatten()
}

fn select_profile(profiles: &[String], default: &str) -> Option<usize> {
    let default_idx = profiles.iter().position(|p| p == default).unwrap_or(0);

    Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select the AWS Profile to switch")
        .items(profiles)
        .default(default_idx)
        .max_length(10)
        .interact_opt()
        .ok()
        .flatten()
}

fn main() -> Result<(), Box<dyn Error>> {
    const AWS_PROFILE: &str = "AWS_PROFILE";
    const HOME: &str = "HOME";

    let home_path = get_env(HOME);
    let current_aws_profile = get_env(AWS_PROFILE);

    let aws_config_file_path = format!("{home_path}/.aws/config");

    let profiles = parse_profiles(&aws_config_file_path)?;

    let has_environments = profiles.iter().any(|p| p.environment.is_some());

    let chosen_profile = if has_environments {
        // Build environment -> profiles mapping
        let mut env_map: HashMap<String, Vec<String>> = HashMap::new();
        for profile in &profiles {
            let env_key = profile
                .environment
                .clone()
                .unwrap_or_else(|| "other".to_string());
            env_map.entry(env_key).or_default().push(profile.name.clone());
        }

        let mut environments: Vec<String> = env_map.keys().cloned().collect();
        environments.sort();

        // Multi-level selection: environment first, then profile.
        // ESC at profile level goes back to environment; ESC at environment level exits.
        'outer: loop {
            match select_environment(&environments) {
                None => return Ok(()), // ESC at environment level: exit
                Some(env_idx) => {
                    let env = &environments[env_idx];
                    let env_profiles = env_map
                        .get(env)
                        .expect("environment key must exist in env_map as it was derived from its keys");
                    match select_profile(env_profiles, &current_aws_profile) {
                        None => continue 'outer, // ESC at profile level: back to environment
                        Some(profile_idx) => break env_profiles[profile_idx].clone(),
                    }
                }
            }
        }
    } else {
        // No environment fields present: single-level selection (original behaviour).
        let profile_names: Vec<String> = profiles.iter().map(|p| p.name.clone()).collect();
        match select_profile(&profile_names, &current_aws_profile) {
            None => return Ok(()),
            Some(idx) => profile_names[idx].clone(),
        }
    };

    let profile_uses_sso = profiles
        .iter()
        .find(|p| p.name == chosen_profile)
        .map(|p| p.is_sso())
        .unwrap_or(false);

    if profile_uses_sso {
        // Treat any execution failure (e.g. aws not in PATH) as an invalid
        // session so that the subsequent login attempt surfaces the real error.
        let session_valid = Command::new("aws")
            .args(["sts", "get-caller-identity", "--profile", &chosen_profile])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !session_valid {
            let status = Command::new("aws")
                .args(["sso", "login", "--profile", &chosen_profile])
                .status()
                .map_err(|e| format!("Failed to execute 'aws' command. Is it installed and in PATH? Error: {e}"))?;
            if !status.success() {
                return Err(format!(
                    "'aws sso login --profile {}' failed with exit code: {}",
                    chosen_profile,
                    status.code().unwrap_or(-1)
                )
                .into());
            }
        }
    }

    println!("export {}='{}';", AWS_PROFILE, chosen_profile);

    Ok(())
}
