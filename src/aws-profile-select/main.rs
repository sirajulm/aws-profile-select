use std::collections::HashMap;
use std::env;
use std::error::Error;
use config::{Config, FileFormat, Source, Value};
use dialoguer::{Select, theme::ColorfulTheme};

fn get_env(env_key: &str) -> String {
    match env::var_os(env_key) {
        Some(os_str) => os_str.into_string().unwrap_or_else(|os_string| {
            panic!("Failed to convert OsString to String: {:?}", os_string)
        }),
        None => String::new(),
    }
}

struct Profile {
    name: String,
    environment: Option<String>,
}

fn parse_profiles(aws_config_path: &str) -> Result<Vec<Profile>, Box<dyn Error>> {
    let config = Config::builder()
        .add_source(config::File::new(aws_config_path, FileFormat::Ini))
        .build()?;

    let map: HashMap<String, Value> = config.collect()?;

    let mut profiles: Vec<Profile> = map
        .into_iter()
        .filter(|(key, _)| !key.contains("sso-session"))
        .map(|(key, value)| {
            let name = key.replace("profile ", "");
            let environment = value
                .into_table()
                .ok()
                .and_then(|table| table.get("environment").cloned())
                .and_then(|v| v.into_string().ok());
            Profile { name, environment }
        })
        .collect();

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}

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

    println!("export {}='{}';", AWS_PROFILE, chosen_profile);

    Ok(())
}
