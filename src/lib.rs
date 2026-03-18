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
    match env::var_os(env_key) {
        Some(os_str) => os_str.into_string().unwrap_or_else(|os_string| {
            panic!("Failed to convert OsString to String: {:?}", os_string)
        }),
        None => String::new(),
    }
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
