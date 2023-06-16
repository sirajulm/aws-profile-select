use std::{env};
use std::error::Error;
use config::{Config, Source, FileFormat};
use std::fmt;
use dialoguer::{Select, theme::ColorfulTheme};

#[derive(Debug)]
struct MyError {
    message: String,
}

impl Error for MyError {}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MyError: {}", self.message)
    }
}

fn get_env(env_key: &str) -> String {
    let profile =  env::var_os(env_key);

    let env_value  = match profile {
        Some(os_str) => os_str.into_string().unwrap_or_else(|os_string| {
            panic!("Failed to convert OsString to String: {:?}", os_string)
        }),
        None => String::new(),
    };

    env_value
}


fn select_aws_profile(mut list: Vec<String>, default: String) -> String {
    list.sort();

    let current_profile_index: usize = match list.clone().into_iter().position(|p| p == default) {
        Some(value) => {value}
        None => {0}
    };

    let chosen_result : Result<usize, std::io::Error> = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select the AWS Profile to switch")
        .items(&list)
        .default(current_profile_index)
        .max_length(10)
        .interact();
    
    match chosen_result {
        Ok(value) => list[value].clone(),
        Err(_) => default,
    }
}

fn main() -> Result<(), Box<dyn Error>> {

    const AWS_PROFILE: &str = "AWS_PROFILE";
    const  HOME: &str = "HOME";
    
    let home_path = get_env(HOME);
    let current_aws_profile = get_env(AWS_PROFILE);

    let aws_config_file_path = format!("{home_path}/.aws/config");

    let config = Config::builder()
    .add_source(config::File::new(&aws_config_file_path, FileFormat::Ini))
    .build()?;


    let list: Vec<String> = config
        .collect()?
        .keys()
        .filter(|key| !key.contains("sso-session"))
        .map(|key| key.replace("profile ", ""))
        .collect();

    let chosen_profile = select_aws_profile(list, current_aws_profile);

    println!("export {}='{}';", AWS_PROFILE, chosen_profile);

    Ok(())
}
