use std::env;
use std::fs;
use std::path::PathBuf;

use aws_profile_select::{get_env, parse_profiles};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Write `content` to a file inside the system temp directory and return its
/// path. Using unique file names avoids collisions when tests run in parallel.
fn write_temp_config(file_name: &str, content: &str) -> PathBuf {
    let path = env::temp_dir().join(file_name);
    fs::write(&path, content).expect("failed to write temp config file");
    path
}

// ---------------------------------------------------------------------------
// get_env
// ---------------------------------------------------------------------------

#[test]
fn get_env_returns_value_of_existing_variable() {
    env::set_var("AWS_PS_TEST_EXISTING_VAR", "hello_world");
    assert_eq!(get_env("AWS_PS_TEST_EXISTING_VAR"), "hello_world");
    env::remove_var("AWS_PS_TEST_EXISTING_VAR");
}

#[test]
fn get_env_returns_empty_string_for_missing_variable() {
    env::remove_var("AWS_PS_TEST_DEFINITELY_MISSING_VAR");
    assert_eq!(get_env("AWS_PS_TEST_DEFINITELY_MISSING_VAR"), "");
}

// ---------------------------------------------------------------------------
// parse_profiles – environment field
// ---------------------------------------------------------------------------

#[test]
fn parse_profiles_reads_environment_field() {
    let content = "\
[profile prod-admin]
region = eu-central-1
environment = production

[profile dev-readonly]
region = eu-central-1
environment = development
";
    let path = write_temp_config("aws_ps_test_with_env.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles.len(), 2);
    // sorted alphabetically: dev-readonly < prod-admin
    assert_eq!(profiles[0].name, "dev-readonly");
    assert_eq!(profiles[0].environment, Some("development".to_string()));
    assert_eq!(profiles[1].name, "prod-admin");
    assert_eq!(profiles[1].environment, Some("production".to_string()));
}

#[test]
fn parse_profiles_environment_is_none_when_field_absent() {
    let content = "\
[profile legacy]
region = us-east-1
";
    let path = write_temp_config("aws_ps_test_no_env.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].name, "legacy");
    assert_eq!(profiles[0].environment, None);
}

#[test]
fn parse_profiles_mixed_env_and_no_env() {
    let content = "\
[profile with-env]
region = eu-central-1
environment = staging

[profile without-env]
region = us-east-1
";
    let path = write_temp_config("aws_ps_test_mixed.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles.len(), 2);
    // 'i' < 'o' at position 4, so "with-env" < "without-env"
    assert_eq!(profiles[0].name, "with-env");
    assert_eq!(profiles[0].environment, Some("staging".to_string()));
    assert_eq!(profiles[1].name, "without-env");
    assert_eq!(profiles[1].environment, None);
}

// ---------------------------------------------------------------------------
// parse_profiles – sso-session filtering
// ---------------------------------------------------------------------------

#[test]
fn parse_profiles_filters_out_sso_session_sections() {
    let content = "\
[profile my-profile]
region = eu-central-1

[sso-session my-sso]
sso_start_url = https://example.awsapps.com/start
sso_region = eu-central-1
";
    let path = write_temp_config("aws_ps_test_sso.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].name, "my-profile");
}

// ---------------------------------------------------------------------------
// parse_profiles – alphabetical ordering
// ---------------------------------------------------------------------------

#[test]
fn parse_profiles_returns_profiles_sorted_alphabetically() {
    let content = "\
[profile zebra]
region = us-east-1

[profile alpha]
region = us-east-1

[profile middle]
region = us-east-1
";
    let path = write_temp_config("aws_ps_test_sorted.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles.len(), 3);
    assert_eq!(profiles[0].name, "alpha");
    assert_eq!(profiles[1].name, "middle");
    assert_eq!(profiles[2].name, "zebra");
}

// ---------------------------------------------------------------------------
// parse_profiles – error handling
// ---------------------------------------------------------------------------

#[test]
fn parse_profiles_returns_error_for_missing_file() {
    let result = parse_profiles("/tmp/aws_ps_test_this_file_does_not_exist_xyz.ini");
    assert!(result.is_err());
}
