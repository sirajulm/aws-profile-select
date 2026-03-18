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

/// Profiles without an environment are placed under the `"other"` key.
fn build_env_map(profiles: &[Profile]) -> HashMap<String, Vec<String>> {
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
    env_map
}

/// Returns `true` when any profile in the slice carries an `environment` field.
fn has_environments(profiles: &[Profile]) -> bool {
    profiles.iter().any(|p| p.environment.is_some())
}

/// Presents an interactive menu to choose an AWS profile.
/// If no profiles have environments, a flat single-level list is shown instead.
pub fn run_interactive(profiles: &[Profile], current_aws_profile: &str) -> Option<String> {
    if has_environments(profiles) {
        let env_map = build_env_map(profiles);

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
        select_profile(&profile_names, current_aws_profile).map(|idx| profile_names[idx].clone())
    }
}

// ---------------------------------------------------------------------------
// Unit tests — pure logic only, no dialoguer interaction
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to build a `Profile` with minimal boilerplate.
    fn profile(name: &str, environment: Option<&str>) -> Profile {
        Profile {
            name: name.to_string(),
            environment: environment.map(|s| s.to_string()),
            sso_session: None,
            sso_start_url: None,
        }
    }

    // -----------------------------------------------------------------------
    // has_environments
    // -----------------------------------------------------------------------

    #[test]
    fn has_environments_false_when_empty() {
        assert!(!has_environments(&[]));
    }

    #[test]
    fn has_environments_false_when_none_have_env() {
        let profiles = vec![profile("a", None), profile("b", None)];
        assert!(!has_environments(&profiles));
    }

    #[test]
    fn has_environments_true_when_at_least_one_has_env() {
        let profiles = vec![profile("a", None), profile("b", Some("prod"))];
        assert!(has_environments(&profiles));
    }

    #[test]
    fn has_environments_true_when_all_have_env() {
        let profiles = vec![profile("a", Some("dev")), profile("b", Some("prod"))];
        assert!(has_environments(&profiles));
    }

    // -----------------------------------------------------------------------
    // build_env_map — grouping
    // -----------------------------------------------------------------------

    #[test]
    fn build_env_map_empty_input() {
        let map = build_env_map(&[]);
        assert!(map.is_empty());
    }

    #[test]
    fn build_env_map_groups_by_environment() {
        let profiles = vec![
            profile("prod-admin", Some("production")),
            profile("prod-readonly", Some("production")),
            profile("dev-admin", Some("development")),
        ];
        let map = build_env_map(&profiles);

        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get("production").unwrap(),
            &vec!["prod-admin".to_string(), "prod-readonly".to_string()]
        );
        assert_eq!(
            map.get("development").unwrap(),
            &vec!["dev-admin".to_string()]
        );
    }

    #[test]
    fn build_env_map_profiles_without_env_go_to_other() {
        let profiles = vec![profile("has-env", Some("staging")), profile("no-env", None)];
        let map = build_env_map(&profiles);

        assert_eq!(map.len(), 2);
        assert_eq!(map.get("staging").unwrap(), &vec!["has-env".to_string()]);
        assert_eq!(map.get("other").unwrap(), &vec!["no-env".to_string()]);
    }

    #[test]
    fn build_env_map_all_without_env_go_to_other() {
        let profiles = vec![profile("a", None), profile("b", None)];
        let map = build_env_map(&profiles);

        assert_eq!(map.len(), 1);
        assert_eq!(
            map.get("other").unwrap(),
            &vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn build_env_map_preserves_insertion_order_within_group() {
        let profiles = vec![
            profile("zebra", Some("env")),
            profile("alpha", Some("env")),
            profile("middle", Some("env")),
        ];
        let map = build_env_map(&profiles);

        // Order within a group should match the input slice order, not sorted.
        assert_eq!(
            map.get("env").unwrap(),
            &vec![
                "zebra".to_string(),
                "alpha".to_string(),
                "middle".to_string()
            ]
        );
    }

    #[test]
    fn build_env_map_single_profile_single_env() {
        let profiles = vec![profile("only", Some("lone"))];
        let map = build_env_map(&profiles);

        assert_eq!(map.len(), 1);
        assert_eq!(map.get("lone").unwrap(), &vec!["only".to_string()]);
    }

    #[test]
    fn build_env_map_many_environments() {
        let profiles = vec![
            profile("a", Some("env1")),
            profile("b", Some("env2")),
            profile("c", Some("env3")),
            profile("d", Some("env4")),
        ];
        let map = build_env_map(&profiles);

        assert_eq!(map.len(), 4);
        for (env, name) in [("env1", "a"), ("env2", "b"), ("env3", "c"), ("env4", "d")] {
            assert_eq!(map.get(env).unwrap(), &vec![name.to_string()]);
        }
    }
}
