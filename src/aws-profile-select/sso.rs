use aws_profile_select::Profile;
use std::error::Error;
use std::process::{Command, Stdio};

/// Returns the profile name to use for `aws sso login`, or `None` if no SSO
/// login is required for `chosen_profile`.
/// Two cases require SSO login:
/// * The chosen profile is itself an SSO profile.
/// * The chosen profile is an assume-role profile whose `source_profile` is
///   an SSO profile — in that case the *source* profile name is returned so
///   that `aws sso login` refreshes the correct session.
fn sso_login_profile<'a>(profiles: &'a [Profile], chosen_profile: &str) -> Option<&'a str> {
    let profile = profiles.iter().find(|p| p.name == chosen_profile)?;

    if profile.is_sso() {
        return Some(&profile.name);
    }

    // Assume-role profile: follow source_profile to find an SSO source.
    if let Some(ref source_name) = profile.source_profile {
        let source = profiles.iter().find(|p| p.name == *source_name)?;
        if source.is_sso() {
            return Some(&source.name);
        }
    }

    None
}

/// If the chosen profile requires SSO (directly or via an assume-role chain),
/// validates the current session and triggers `aws sso login` when necessary.
pub fn handle_sso_login(profiles: &[Profile], chosen_profile: &str) -> Result<(), Box<dyn Error>> {
    let login_profile = match sso_login_profile(profiles, chosen_profile) {
        Some(name) => name,
        None => return Ok(()),
    };

    // Treat any execution failure (e.g. aws not in PATH) as an invalid
    // session so that the subsequent login attempt surfaces the real error.
    let session_valid = Command::new("aws")
        .args(["sso", "login", "--profile", login_profile])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !session_valid {
        let status = Command::new("aws")
            .args(["sso", "login", "--profile", chosen_profile])
            .status()
            .map_err(|e| {
                format!("Failed to execute 'aws' command. Is it installed and in PATH? Error: {e}")
            })?;
        if !status.success() {
            return Err(format!(
                "'aws sso login --profile {}' failed with exit code: {}",
                chosen_profile,
                status.code().unwrap_or(-1)
            )
            .into());
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests — pure logic only, no subprocess execution
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to build a non-SSO `Profile`.
    fn iam_profile(name: &str) -> Profile {
        Profile {
            name: name.to_string(),
            environment: None,
            sso_session: None,
            sso_start_url: None,
            source_profile: None,
            duration: None,
            readonly: None,
        }
    }

    /// Helper to build a modern SSO `Profile` (has `sso_session`).
    fn sso_session_profile(name: &str, session: &str) -> Profile {
        Profile {
            name: name.to_string(),
            environment: None,
            sso_session: Some(session.to_string()),
            sso_start_url: None,
            source_profile: None,
            duration: None,
            readonly: None,
        }
    }

    /// Helper to build a legacy SSO `Profile` (has `sso_start_url` only).
    fn sso_url_profile(name: &str, url: &str) -> Profile {
        Profile {
            name: name.to_string(),
            environment: None,
            sso_session: None,
            sso_start_url: Some(url.to_string()),
            source_profile: None,
            duration: None,
            readonly: None,
        }
    }

    /// Helper to build an assume-role `Profile` with a named source profile.
    fn assume_role_profile(name: &str, source: &str) -> Profile {
        Profile {
            name: name.to_string(),
            environment: None,
            sso_session: None,
            sso_start_url: None,
            source_profile: Some(source.to_string()),
            duration: None,
            readonly: None,
        }
    }

    // -----------------------------------------------------------------------
    // sso_login_profile — non-SSO profiles
    // -----------------------------------------------------------------------

    #[test]
    fn none_for_empty_profile_list() {
        assert!(sso_login_profile(&[], "anything").is_none());
    }

    #[test]
    fn none_for_iam_profile() {
        let profiles = vec![iam_profile("dev")];
        assert!(sso_login_profile(&profiles, "dev").is_none());
    }

    #[test]
    fn none_when_profile_not_found() {
        let profiles = vec![iam_profile("dev")];
        assert!(sso_login_profile(&profiles, "prod").is_none());
    }

    #[test]
    fn none_when_name_is_empty() {
        let profiles = vec![iam_profile("dev")];
        assert!(sso_login_profile(&profiles, "").is_none());
    }

    // -----------------------------------------------------------------------
    // sso_login_profile — direct SSO profiles
    // -----------------------------------------------------------------------

    #[test]
    fn returns_own_name_for_sso_session_profile() {
        let profiles = vec![sso_session_profile("sso-dev", "my-sso")];
        assert_eq!(sso_login_profile(&profiles, "sso-dev"), Some("sso-dev"));
    }

    #[test]
    fn returns_own_name_for_sso_url_profile() {
        let profiles = vec![sso_url_profile(
            "sso-legacy",
            "https://example.awsapps.com/start",
        )];
        assert_eq!(
            sso_login_profile(&profiles, "sso-legacy"),
            Some("sso-legacy")
        );
    }

    #[test]
    fn returns_own_name_when_both_sso_fields_present() {
        let profiles = vec![Profile {
            name: "full-sso".to_string(),
            environment: None,
            sso_session: Some("corp".to_string()),
            sso_start_url: Some("https://corp.awsapps.com/start".to_string()),
            source_profile: None,
            duration: None,
            readonly: None,
        }];
        assert_eq!(sso_login_profile(&profiles, "full-sso"), Some("full-sso"));
    }

    // -----------------------------------------------------------------------
    // sso_login_profile — assume-role profiles backed by SSO
    // -----------------------------------------------------------------------

    #[test]
    fn returns_source_name_for_assume_role_with_sso_source() {
        let profiles = vec![
            sso_session_profile("app.admin", "corp-sso"),
            assume_role_profile("mongodb-prod", "app.admin"),
        ];
        assert_eq!(sso_login_profile(&profiles, "mongodb-prod"),Some("app.admin"));
    }

    #[test]
    fn returns_source_name_for_assume_role_with_legacy_sso_source() {
        let profiles = vec![
            sso_url_profile("base-sso", "https://corp.awsapps.com/start"),
            assume_role_profile("prod-role", "base-sso"),
        ];
        assert_eq!(sso_login_profile(&profiles, "prod-role"),Some("base-sso"));
    }
 
    #[test]
    fn none_for_assume_role_with_iam_source() {
        let profiles = vec![
            iam_profile("app.admin"),
            assume_role_profile("mongodb-prod", "app.admin"),
        ];
        assert!(sso_login_profile(&profiles, "mongodb-prod").is_none());
    }

    #[test]
    fn none_for_assume_role_with_missing_source() {
        let profiles = vec![assume_role_profile("mongodb-prod", "nonexistent")];
        assert!(sso_login_profile(&profiles, "mongodb-prod").is_none());
    }

    // -----------------------------------------------------------------------
    // sso_login_profile — mixed list
    // -----------------------------------------------------------------------
    #[test]
    fn identifies_sso_profile_in_mixed_list() {
        let profiles = vec![
            iam_profile("iam-admin"),
            sso_session_profile("sso-prod", "corp-sso"),
            iam_profile("iam-readonly"),
        ];

        assert!(sso_login_profile(&profiles, "iam-admin").is_none());
        assert_eq!(sso_login_profile(&profiles, "sso-prod"),Some("sso-prod"));
        assert!(sso_login_profile(&profiles, "iam-readonly").is_none());
    }

    #[test]
    fn non_existent_profile_in_mixed_list() {
        let profiles = vec![iam_profile("dev"), sso_session_profile("prod", "corp")];
        assert!(sso_login_profile(&profiles, "staging").is_none());
    }

    // -----------------------------------------------------------------------
    // sso_login_profile — name matching is exact
    // -----------------------------------------------------------------------

    #[test]
    fn does_not_match_substring() {
        let profiles = vec![sso_session_profile("sso-dev", "my-sso")];
        assert!(sso_login_profile(&profiles, "sso").is_none());
        assert!(sso_login_profile(&profiles, "sso-dev-extra").is_none());
        assert!(sso_login_profile(&profiles, "dev").is_none());
    }

    #[test]
    fn matching_is_case_sensitive() {
        let profiles = vec![sso_session_profile("SSO-Dev", "my-sso")];
        assert_eq!(sso_login_profile(&profiles, "SSO-Dev"), Some("SSO-Dev"));
        assert!(sso_login_profile(&profiles, "sso-dev").is_none());
        assert!(sso_login_profile(&profiles, "SSO-DEV").is_none());
    }
}
