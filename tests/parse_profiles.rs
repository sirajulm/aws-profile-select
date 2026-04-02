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
duration = 8h
readonly = true
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
    assert_eq!(profiles[0].duration, Some("8h".to_string()));
    assert_eq!(profiles[0].readonly, Some(true));
    assert_eq!(profiles[0].display_name(), "full (8h) 👀");
}

// ---------------------------------------------------------------------------
// duration field
// ---------------------------------------------------------------------------

#[test]
fn reads_duration_field() {
    let content = "\
[profile short-lived]
region = us-east-1
duration = 1h

[profile long-lived]
region = us-east-1
duration = 12h
";
    let path = common::write_temp_config("aws_ps_duration.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].name, "long-lived");
    assert_eq!(profiles[0].duration, Some("12h".to_string()));
    assert_eq!(profiles[1].name, "short-lived");
    assert_eq!(profiles[1].duration, Some("1h".to_string()));
}

#[test]
fn duration_defaults_to_none() {
    let content = "\
[profile no-duration]
region = us-east-1
";
    let path = common::write_temp_config("aws_ps_no_duration.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].duration, None);
}

// ---------------------------------------------------------------------------
// readonly field
// ---------------------------------------------------------------------------

#[test]
fn reads_readonly_true() {
    let content = "\
[profile ro-profile]
region = us-east-1
readonly = true
";
    let path = common::write_temp_config("aws_ps_readonly_true.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].readonly, Some(true));
}

#[test]
fn reads_readonly_false() {
    let content = "\
[profile rw-profile]
region = us-east-1
readonly = false
";
    let path = common::write_temp_config("aws_ps_readonly_false.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].readonly, Some(false));
}

#[test]
fn readonly_defaults_to_none() {
    let content = "\
[profile no-readonly]
region = us-east-1
";
    let path = common::write_temp_config("aws_ps_no_readonly.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].readonly, None);
}

// ---------------------------------------------------------------------------
// display_name — integration tests
// ---------------------------------------------------------------------------

#[test]
fn display_name_no_annotations() {
    let content = "\
[profile plain]
region = us-east-1
";
    let path = common::write_temp_config("aws_ps_display_plain.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].display_name(), "plain");
}

#[test]
fn display_name_duration_only() {
    let content = "\
[profile timed]
region = us-east-1
duration = 8h
";
    let path = common::write_temp_config("aws_ps_display_duration.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].display_name(), "timed (8h)");
}

#[test]
fn display_name_readonly_only() {
    let content = "\
[profile locked]
region = us-east-1
readonly = true
";
    let path = common::write_temp_config("aws_ps_display_readonly.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].display_name(), "locked 👀");
}

#[test]
fn display_name_both_annotations() {
    let content = "\
[profile annotated]
region = us-east-1
duration = 4h
readonly = true
";
    let path = common::write_temp_config("aws_ps_display_both.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    assert_eq!(profiles[0].display_name(), "annotated (4h) 👀");
}

#[test]
fn mixed_annotated_and_plain_profiles() {
    let content = "\
[profile admin]
region = us-east-1
duration = 8h

[profile reader]
region = us-east-1
readonly = true

[profile basic]
region = us-east-1
";
    let path = common::write_temp_config("aws_ps_display_mixed.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();

    // sorted alphabetically: admin, basic, reader
    assert_eq!(profiles[0].display_name(), "admin (8h)");
    assert_eq!(profiles[1].display_name(), "basic");
    assert_eq!(profiles[2].display_name(), "reader 👀");
}

// ---------------------------------------------------------------------------
// source_profile / assume-role profiles
// ---------------------------------------------------------------------------
#[test]
fn reads_source_profile_field() {
    let content = "\
[profile app.admin]
sso_session = corp-sso
sso_account_id = 111111111111
sso_role_name = AdminAccess
region = us-east-1
 
[profile mongodb-prod]
role_arn = arn:aws:iam::222222222222:role/app-role
source_profile = app.admin
region = us-east-1
";
    let path = common::write_temp_config("aws_ps_assume_role.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();
    // sorted: app.admin, mongodb-prod
    assert_eq!(profiles[0].name, "app.admin");
    assert_eq!(profiles[0].source_profile, None);
    assert!(profiles[0].is_sso());
    assert_eq!(profiles[1].name, "mongodb-prod");
    assert_eq!(profiles[1].source_profile, Some("app.admin".to_string()));
    assert!(!profiles[1].is_sso());
}

#[test]
fn source_profile_defaults_to_none() {
    let content = "\
[profile no-source]
region = us-east-1
";
    let path = common::write_temp_config("aws_ps_no_source_profile.ini", content);
    let profiles = parse_profiles(path.to_str().unwrap()).unwrap();
    assert_eq!(profiles[0].source_profile, None);
}
