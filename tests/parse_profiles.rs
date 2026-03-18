mod common;

use aws_profile_select::parse_profiles;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn assert_profile_names(content: &str, file_name: &str, expected: &[&str]) {
    let path = common::write_temp_config(file_name, content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();
    let names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, expected);
}

// ---------------------------------------------------------------------------
// environment field
// ---------------------------------------------------------------------------

#[test]
fn reads_environment_field() {
    let content = "\
[profile prod-admin]
region = eu-central-1
environment = production

[profile dev-readonly]
region = eu-central-1
environment = development
";
    let path = common::write_temp_config("aws_ps_env.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].name, "dev-readonly");
    assert_eq!(profiles[0].environment, Some("development".to_string()));
    assert_eq!(profiles[1].name, "prod-admin");
    assert_eq!(profiles[1].environment, Some("production".to_string()));
}

#[test]
fn mixed_env_and_no_env() {
    let content = "\
[profile with-env]
region = eu-central-1
environment = staging

[profile without-env]
region = us-east-1
";
    let path = common::write_temp_config("aws_ps_mixed_env.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].environment, Some("staging".to_string()));
    assert_eq!(profiles[1].environment, None);
}

// ---------------------------------------------------------------------------
// sso-session filtering
// ---------------------------------------------------------------------------

#[test]
fn filters_out_sso_session_sections() {
    let content = "\
[profile my-profile]
region = eu-central-1

[sso-session my-sso]
sso_start_url = https://example.awsapps.com/start
sso_region = eu-central-1
";
    assert_profile_names(content, "aws_ps_sso_filter.ini", &["my-profile"]);
}

// ---------------------------------------------------------------------------
// sso detection (modern sso_session + legacy sso_start_url)
// ---------------------------------------------------------------------------

#[test]
fn modern_sso_profile_detected() {
    let content = "\
[profile sso-dev]
sso_session = my-sso
sso_account_id = 123456789012
sso_role_name = DevAccess
region = eu-central-1
";
    let path = common::write_temp_config("aws_ps_sso_modern.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].sso_session, Some("my-sso".to_string()));
    assert_eq!(profiles[0].sso_start_url, None);
    assert!(profiles[0].is_sso());
}

#[test]
fn legacy_sso_profile_detected() {
    let content = "\
[profile sso-legacy]
sso_start_url = https://my-org.awsapps.com/start
sso_account_id = 123456789012
sso_role_name = ReadOnly
sso_region = eu-central-1
region = eu-central-1
";
    let path = common::write_temp_config("aws_ps_sso_legacy.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].sso_session, None);
    assert_eq!(
        profiles[0].sso_start_url,
        Some("https://my-org.awsapps.com/start".to_string())
    );
    assert!(profiles[0].is_sso());
}

#[test]
fn iam_profile_is_not_sso() {
    let content = "\
[profile iam-only]
region = us-east-1
";
    let path = common::write_temp_config("aws_ps_iam.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert!(!profiles[0].is_sso());
    assert_eq!(profiles[0].sso_session, None);
    assert_eq!(profiles[0].sso_start_url, None);
    assert_eq!(profiles[0].environment, None);
}

#[test]
fn mixed_sso_and_non_sso() {
    let content = "\
[profile sso-prod]
sso_session = corp-sso
sso_account_id = 111111111111
sso_role_name = ProdAdmin
region = us-east-1

[profile iam-legacy]
region = eu-west-1
";
    let path = common::write_temp_config("aws_ps_mixed_sso.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert!(!profiles[0].is_sso()); // iam-legacy
    assert!(profiles[1].is_sso()); // sso-prod
}

// ---------------------------------------------------------------------------
// alphabetical ordering
// ---------------------------------------------------------------------------

#[test]
fn returns_profiles_sorted_alphabetically() {
    let content = "\
[profile zebra]
region = us-east-1

[profile alpha]
region = us-east-1

[profile middle]
region = us-east-1
";
    assert_profile_names(content, "aws_ps_sorted.ini", &["alpha", "middle", "zebra"]);
}

// ---------------------------------------------------------------------------
// [default] section (no "profile" prefix)
// ---------------------------------------------------------------------------

#[test]
fn default_section_parsed_without_profile_prefix() {
    let content = "\
[default]
region = us-east-1
";
    assert_profile_names(content, "aws_ps_default_bare.ini", &["default"]);
}

#[test]
fn default_section_mixed_with_named_profiles() {
    let content = "\
[default]
region = us-east-1

[profile dev]
region = eu-central-1

[profile prod]
region = eu-west-1
";
    assert_profile_names(
        content,
        "aws_ps_default_mixed.ini",
        &["default", "dev", "prod"],
    );
}

#[test]
fn default_section_with_sso_and_environment() {
    let content = "\
[default]
sso_session = my-sso
sso_account_id = 123456789012
sso_role_name = AdminAccess
region = us-east-1
environment = production
";
    let path = common::write_temp_config("aws_ps_default_full.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].name, "default");
    assert_eq!(profiles[0].sso_session, Some("my-sso".to_string()));
    assert_eq!(profiles[0].environment, Some("production".to_string()));
    assert!(profiles[0].is_sso());
}

// ---------------------------------------------------------------------------
// empty / minimal files
// ---------------------------------------------------------------------------

#[test]
fn empty_file_returns_empty_vec() {
    let path = common::write_temp_config("aws_ps_empty.ini", "");
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();
    assert!(profiles.is_empty());
}

#[test]
fn file_with_only_comments_returns_empty_vec() {
    let content = "\
# This is a comment
; This is also a comment
";
    let path = common::write_temp_config("aws_ps_comments_only.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();
    assert!(profiles.is_empty());
}

// ---------------------------------------------------------------------------
// error handling
// ---------------------------------------------------------------------------

#[test]
fn returns_error_for_missing_file() {
    let result = parse_profiles("/tmp/aws_ps_this_file_does_not_exist.ini");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// all fields present
// ---------------------------------------------------------------------------

#[test]
fn profile_with_all_fields() {
    let content = "\
[profile full]
region = us-west-2
output = json
sso_session = corp
sso_account_id = 999999999999
sso_role_name = FullAdmin
sso_start_url = https://corp.awsapps.com/start
environment = production
";
    let path = common::write_temp_config("aws_ps_full.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].name, "full");
    assert_eq!(profiles[0].environment, Some("production".to_string()));
    assert_eq!(profiles[0].sso_session, Some("corp".to_string()));
    assert_eq!(
        profiles[0].sso_start_url,
        Some("https://corp.awsapps.com/start".to_string())
    );
    assert!(profiles[0].is_sso());
}
