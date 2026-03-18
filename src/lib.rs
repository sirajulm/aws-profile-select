use config::{Config, FileFormat, Source, Value};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::path::PathBuf;

pub struct Profile {
    pub name: String,
    pub environment: Option<String>,
    pub sso_session: Option<String>,
    pub sso_start_url: Option<String>,
}

impl Profile {
    /// Returns `true` when the profile is configured for AWS SSO and therefore
    /// requires `aws sso login` before use.  Two configuration styles are
    /// recognised:
    ///
    /// * **Modern (token-provider)** – the profile contains an `sso_session`
    ///   key that references a named `[sso-session …]` block.
    /// * **Legacy (URL-based)** – the profile contains `sso_start_url`
    ///   directly, without a separate `[sso-session …]` block.
    pub fn is_sso(&self) -> bool {
        self.sso_session.is_some() || self.sso_start_url.is_some()
    }
}

pub fn get_env(env_key: &str) -> String {
    env::var(env_key).unwrap_or_default()
}

/// Resolves the path to the AWS config file.
///
/// Checks `AWS_CONFIG_FILE` first; falls back to `~/.aws/config` using the
/// `dirs` crate for cross-platform home directory resolution.
pub fn resolve_config_path() -> Result<PathBuf, Box<dyn Error>> {
    match env::var("AWS_CONFIG_FILE") {
        Ok(path) if !path.is_empty() => Ok(PathBuf::from(path)),
        _ => {
            let home = dirs::home_dir()
                .ok_or("Could not determine home directory. Set AWS_CONFIG_FILE or HOME.")?;
            Ok(home.join(".aws").join("config"))
        }
    }
}

pub fn parse_profiles(aws_config_path: &str) -> Result<Vec<Profile>, Box<dyn Error>> {
    let config = Config::builder()
        .add_source(config::File::new(aws_config_path, FileFormat::Ini))
        .build()?;

    let map: HashMap<String, Value> = config.collect()?;

    let mut profiles: Vec<Profile> = map
        .into_iter()
        .filter(|(key, _)| !key.contains("sso-session"))
        .map(|(key, value)| {
            let name = key.replace("profile ", "");
            let (environment, sso_session, sso_start_url) = value
                .into_table()
                .ok()
                .map(|table| {
                    let environment = table
                        .get("environment")
                        .and_then(|v| v.clone().into_string().ok());
                    let sso_session = table
                        .get("sso_session")
                        .and_then(|v| v.clone().into_string().ok());
                    let sso_start_url = table
                        .get("sso_start_url")
                        .and_then(|v| v.clone().into_string().ok());
                    (environment, sso_session, sso_start_url)
                })
                .unwrap_or((None, None, None));
            Profile {
                name,
                environment,
                sso_session,
                sso_start_url,
            }
        })
        .collect();

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}

/*
 * Unit tests
 */
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // -----------------------------------------------------------------------
    // get_env
    // -----------------------------------------------------------------------

    #[test]
    #[serial]
    fn get_env_returns_value() {
        env::set_var("AWS_PS_TEST_VAR", "hello");
        assert_eq!(get_env("AWS_PS_TEST_VAR"), "hello");
        env::remove_var("AWS_PS_TEST_VAR");
    }

    #[test]
    #[serial]
    fn get_env_returns_empty_when_missing() {
        env::remove_var("AWS_PS_TEST_MISSING");
        assert_eq!(get_env("AWS_PS_TEST_MISSING"), "");
    }

    #[test]
    #[serial]
    fn get_env_returns_empty_when_value_is_empty() {
        env::set_var("AWS_PS_TEST_EMPTY", "");
        assert_eq!(get_env("AWS_PS_TEST_EMPTY"), "");
        env::remove_var("AWS_PS_TEST_EMPTY");
    }

    // -----------------------------------------------------------------------
    // resolve_config_path
    // -----------------------------------------------------------------------

    #[test]
    #[serial]
    fn resolve_config_path_uses_aws_config_file_when_set() {
        env::set_var("AWS_CONFIG_FILE", "/custom/path/config");
        let path = resolve_config_path().unwrap();
        assert_eq!(path, PathBuf::from("/custom/path/config"));
        env::remove_var("AWS_CONFIG_FILE");
    }

    #[test]
    #[serial]
    fn resolve_config_path_ignores_empty_aws_config_file() {
        env::set_var("AWS_CONFIG_FILE", "");
        let path = resolve_config_path().unwrap();
        assert!(
            path.ends_with(".aws/config"),
            "expected fallback to ~/.aws/config, got: {:?}",
            path
        );
        env::remove_var("AWS_CONFIG_FILE");
    }

    #[test]
    #[serial]
    fn resolve_config_path_falls_back_to_home_aws_config() {
        env::remove_var("AWS_CONFIG_FILE");
        let path = resolve_config_path().unwrap();
        let home = dirs::home_dir().expect("home dir should resolve in test");
        assert_eq!(path, home.join(".aws").join("config"));
    }
}
