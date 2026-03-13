use std::collections::HashMap;
use std::env;
use std::error::Error;
use config::{Config, FileFormat, Source, Value};

pub struct Profile {
    pub name: String,
    pub environment: Option<String>,
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
