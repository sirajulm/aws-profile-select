use std::collections::HashMap;
use std::env;
use std::error::Error;
use config::{Config, FileFormat, Source, Value};

pub struct Profile {
    pub name: String,
    pub environment: Option<String>,
    pub sso_session: Option<String>,
}

pub fn get_env(env_key: &str) -> String {
    match env::var_os(env_key) {
        Some(os_str) => os_str.into_string().unwrap_or_else(|os_string| {
            panic!("Failed to convert OsString to String: {:?}", os_string)
        }),
        None => String::new(),
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
            let (environment, sso_session) = value
                .into_table()
                .ok()
                .map(|table| {
                    let environment = table
                        .get("environment")
                        .and_then(|v| v.clone().into_string().ok());
                    let sso_session = table
                        .get("sso_session")
                        .and_then(|v| v.clone().into_string().ok());
                    (environment, sso_session)
                })
                .unwrap_or((None, None));
            Profile { name, environment, sso_session }
        })
        .collect();

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}
