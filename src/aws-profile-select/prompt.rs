use aws_profile_select::Profile;
use dialoguer::{theme::ColorfulTheme, Select};
use std::collections::HashMap;

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

/// If no profiles have environments, a flat single-level list is shown instead.
pub fn run_interactive(profiles: &[Profile], current_aws_profile: &str) -> Option<String> {
    let has_environments = profiles.iter().any(|p| p.environment.is_some());

    if has_environments {
        // Build environment -> profiles mapping
        let mut env_map: HashMap<String, Vec<String>> = HashMap::new();
        for profile in profiles {
            let env_key = profile
                .environment
                .clone()
                .unwrap_or_else(|| "other".to_string());
            env_map
                .entry(env_key)
                .or_default()
                .push(profile.name.clone());
        }

        let mut environments: Vec<String> = env_map.keys().cloned().collect();
        environments.sort();

        // Multi-level selection: environment first, then profile.
        // ESC at profile level goes back to environment; ESC at environment level exits.
        'outer: loop {
            match select_environment(&environments) {
                None => return None, // ESC at environment level: exit
                Some(env_idx) => {
                    let env = &environments[env_idx];
                    let env_profiles = env_map.get(env).expect(
                        "environment key must exist in env_map as it was derived from its keys",
                    );
                    match select_profile(env_profiles, current_aws_profile) {
                        None => continue 'outer, // ESC at profile level: back to environment
                        Some(profile_idx) => return Some(env_profiles[profile_idx].clone()),
                    }
                }
            }
        }
    } else {
        // No environment fields present: single-level selection (original behaviour).
        let profile_names: Vec<String> = profiles.iter().map(|p| p.name.clone()).collect();
        match select_profile(&profile_names, current_aws_profile) {
            None => None,
            Some(idx) => Some(profile_names[idx].clone()),
        }
    }
}
