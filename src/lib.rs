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
    pub duration: Option<String>,
    pub readonly: Option<bool>,
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

    /// Returns the profile name with optional annotations.
    ///
    /// Examples:
    /// - `"dev"` — no annotations
    /// - `"dev (8h)"` — duration only
    /// - `"dev (8h) 🔥"` — writable with duration
    /// - `"prod (4h) 👀"` — readonly with duration
    /// - `"prod 👀"` — readonly, no duration
    pub fn display_name(&self) -> String {
        let mut display = self.name.clone();
        if let Some(ref dur) = self.duration {
            display.push_str(&format!(" ({dur})"));
        }
        match self.readonly {
            Some(true) => display.push_str(" 👀"),
            Some(false) => display.push_str(" 🔥"),
            None => {}
        }
        display
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
            let (environment, sso_session, sso_start_url, duration, readonly) = value
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
                    let duration = table
                        .get("duration")
                        .and_then(|v| v.clone().into_string().ok());
                    let readonly = table
                        .get("readonly")
                        .and_then(|v| v.clone().into_string().ok())
                        .map(|v| v == "true");
                    (environment, sso_session, sso_start_url, duration, readonly)
                })
                .unwrap_or((None, None, None, None, None));
            Profile {
                name,
                environment,
                sso_session,
                sso_start_url,
                duration,
                readonly,
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

    // -----------------------------------------------------------------------
    // Helper
    // -----------------------------------------------------------------------

    fn make_profile(
        name: &str,
        sso_session: Option<&str>,
        sso_start_url: Option<&str>,
        duration: Option<&str>,
        readonly: Option<bool>,
    ) -> Profile {
        Profile {
            name: name.to_string(),
            environment: None,
            sso_session: sso_session.map(|s| s.to_string()),
            sso_start_url: sso_start_url.map(|s| s.to_string()),
            duration: duration.map(|s| s.to_string()),
            readonly,
        }
    }

    // -----------------------------------------------------------------------
    // display_name — no annotations
    // -----------------------------------------------------------------------

    #[test]
    fn display_name_plain_profile() {
        let p = make_profile("dev", None, None, None, None);
        assert_eq!(p.display_name(), "dev");
    }

    // -----------------------------------------------------------------------
    // display_name — duration only
    // -----------------------------------------------------------------------

    #[test]
    fn display_name_with_duration() {
        let p = make_profile("dev", None, None, Some("8h"), None);
        assert_eq!(p.display_name(), "dev (8h)");
    }

    #[test]
    fn display_name_with_duration_and_writable() {
        let p = make_profile("dev", None, None, Some("8h"), Some(false));
        assert_eq!(p.display_name(), "dev (8h) 🔥");
    }

    // -----------------------------------------------------------------------
    // display_name — readonly only
    // -----------------------------------------------------------------------

    #[test]
    fn display_name_with_readonly() {
        let p = make_profile("prod", None, None, None, Some(true));
        assert_eq!(p.display_name(), "prod 👀");
    }

    #[test]
    fn display_name_with_writable() {
        let p = make_profile("prod", None, None, None, Some(false));
        assert_eq!(p.display_name(), "prod 🔥");
    }

    #[test]
    fn display_name_readonly_absent() {
        let p = make_profile("prod", None, None, None, None);
        assert_eq!(p.display_name(), "prod");
    }

    // -----------------------------------------------------------------------
    // display_name — both annotations
    // -----------------------------------------------------------------------

    #[test]
    fn display_name_with_duration_and_readonly() {
        let p = make_profile("staging", None, None, Some("8h"), Some(true));
        assert_eq!(p.display_name(), "staging (8h) 👀");
    }

    #[test]
    fn display_name_with_short_duration_and_readonly() {
        let p = make_profile("staging", None, None, Some("1h"), Some(true));
        assert_eq!(p.display_name(), "staging (1h) 👀");
    }

    // -----------------------------------------------------------------------
    // display_name — annotations are independent of sso fields
    // -----------------------------------------------------------------------

    #[test]
    fn display_name_with_sso_and_duration() {
        let p = make_profile("sso-dev", Some("my-sso"), None, Some("4h"), Some(false));
        assert_eq!(p.display_name(), "sso-dev (4h) 🔥");
    }

    #[test]
    fn display_name_with_sso_and_duration_no_readonly() {
        let p = make_profile("sso-dev", Some("my-sso"), None, Some("4h"), None);
        assert_eq!(p.display_name(), "sso-dev (4h)");
    }

    #[test]
    fn display_name_with_sso_and_readonly() {
        let p = make_profile("sso-prod", Some("corp"), None, None, Some(true));
        assert_eq!(p.display_name(), "sso-prod 👀");
    }

    #[test]
    fn display_name_with_sso_duration_and_readonly() {
        let p = make_profile("sso-prod", Some("corp"), None, Some("2h"), Some(true));
        assert_eq!(p.display_name(), "sso-prod (2h) 👀");
    }

    // -----------------------------------------------------------------------
    // is_sso — basic cases
    // -----------------------------------------------------------------------

    #[test]
    fn is_sso_false_when_no_sso_fields() {
        let p = make_profile("dev", None, None, None, None);
        assert!(!p.is_sso());
    }

    #[test]
    fn is_sso_true_with_sso_session() {
        let p = make_profile("sso-dev", Some("my-sso"), None, None, None);
        assert!(p.is_sso());
    }

    #[test]
    fn is_sso_true_with_sso_start_url() {
        let p = make_profile(
            "sso-legacy",
            None,
            Some("https://example.awsapps.com/start"),
            None,
            None,
        );
        assert!(p.is_sso());
    }

    #[test]
    fn is_sso_true_with_both_sso_fields() {
        let p = make_profile(
            "sso-both",
            Some("corp"),
            Some("https://corp.awsapps.com/start"),
            None,
            None,
        );
        assert!(p.is_sso());
    }

    // -----------------------------------------------------------------------
    // is_sso — independent of duration and readonly
    // -----------------------------------------------------------------------

    #[test]
    fn is_sso_false_with_duration_and_readonly_but_no_sso() {
        let p = make_profile("dev", None, None, Some("8h"), Some(true));
        assert!(!p.is_sso());
    }

    #[test]
    fn is_sso_true_with_sso_session_and_annotations() {
        let p = make_profile("sso-dev", Some("my-sso"), None, Some("4h"), Some(true));
        assert!(p.is_sso());
    }
}
