use aws_profile_select::Profile;
use std::error::Error;
use std::process::{Command, Stdio};

/// Determines whether the given profile name corresponds to an SSO-configured
/// profile in the provided slice.
fn requires_sso_login(profiles: &[Profile], chosen_profile: &str) -> bool {
    profiles
        .iter()
        .find(|p| p.name == chosen_profile)
        .map(|p| p.is_sso())
        .unwrap_or(false)
}

/// If the chosen profile is configured for SSO, validates the current session
/// and triggers `aws sso login` when necessary.
pub fn handle_sso_login(profiles: &[Profile], chosen_profile: &str) -> Result<(), Box<dyn Error>> {
    if !requires_sso_login(profiles, chosen_profile) {
        return Ok(());
    }

    // Treat any execution failure (e.g. aws not in PATH) as an invalid
    // session so that the subsequent login attempt surfaces the real error.
    let session_valid = Command::new("aws")
        .args(["sts", "get-caller-identity", "--profile", chosen_profile])
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
            duration: None,
            readonly: false,
        }
    }

    /// Helper to build a modern SSO `Profile` (has `sso_session`).
    fn sso_session_profile(name: &str, session: &str) -> Profile {
        Profile {
            name: name.to_string(),
            environment: None,
            sso_session: Some(session.to_string()),
            sso_start_url: None,
            duration: None,
            readonly: false,
        }
    }

    /// Helper to build a legacy SSO `Profile` (has `sso_start_url` only).
    fn sso_url_profile(name: &str, url: &str) -> Profile {
        Profile {
            name: name.to_string(),
            environment: None,
            sso_session: None,
            sso_start_url: Some(url.to_string()),
            duration: None,
            readonly: false,
        }
    }

    // -----------------------------------------------------------------------
    // requires_sso_login — non-SSO profiles
    // -----------------------------------------------------------------------

    #[test]
    fn false_for_empty_profile_list() {
        assert!(!requires_sso_login(&[], "anything"));
    }

    #[test]
    fn false_for_iam_profile() {
        let profiles = vec![iam_profile("dev")];
        assert!(!requires_sso_login(&profiles, "dev"));
    }

    #[test]
    fn false_when_profile_not_found() {
        let profiles = vec![iam_profile("dev")];
        assert!(!requires_sso_login(&profiles, "prod"));
    }

    #[test]
    fn false_when_name_is_empty() {
        let profiles = vec![iam_profile("dev")];
        assert!(!requires_sso_login(&profiles, ""));
    }

    // -----------------------------------------------------------------------
    // requires_sso_login — modern SSO (sso_session)
    // -----------------------------------------------------------------------

    #[test]
    fn true_for_sso_session_profile() {
        let profiles = vec![sso_session_profile("sso-dev", "my-sso")];
        assert!(requires_sso_login(&profiles, "sso-dev"));
    }

    // -----------------------------------------------------------------------
    // requires_sso_login — legacy SSO (sso_start_url)
    // -----------------------------------------------------------------------

    #[test]
    fn true_for_sso_url_profile() {
        let profiles = vec![sso_url_profile(
            "sso-legacy",
            "https://example.awsapps.com/start",
        )];
        assert!(requires_sso_login(&profiles, "sso-legacy"));
    }

    // -----------------------------------------------------------------------
    // requires_sso_login — both fields set
    // -----------------------------------------------------------------------

    #[test]
    fn true_when_both_sso_fields_present() {
        let profiles = vec![Profile {
            name: "full-sso".to_string(),
            environment: None,
            sso_session: Some("corp".to_string()),
            sso_start_url: Some("https://corp.awsapps.com/start".to_string()),
            duration: None,
            readonly: false,
        }];
        assert!(requires_sso_login(&profiles, "full-sso"));
    }

    // -----------------------------------------------------------------------
    // requires_sso_login — mixed list
    // -----------------------------------------------------------------------

    #[test]
    fn identifies_sso_profile_in_mixed_list() {
        let profiles = vec![
            iam_profile("iam-admin"),
            sso_session_profile("sso-prod", "corp-sso"),
            iam_profile("iam-readonly"),
        ];

        assert!(!requires_sso_login(&profiles, "iam-admin"));
        assert!(requires_sso_login(&profiles, "sso-prod"));
        assert!(!requires_sso_login(&profiles, "iam-readonly"));
    }

    #[test]
    fn non_existent_profile_in_mixed_list() {
        let profiles = vec![iam_profile("dev"), sso_session_profile("prod", "corp")];
        assert!(!requires_sso_login(&profiles, "staging"));
    }

    // -----------------------------------------------------------------------
    // requires_sso_login — name matching is exact
    // -----------------------------------------------------------------------

    #[test]
    fn does_not_match_substring() {
        let profiles = vec![sso_session_profile("sso-dev", "my-sso")];
        assert!(!requires_sso_login(&profiles, "sso"));
        assert!(!requires_sso_login(&profiles, "sso-dev-extra"));
        assert!(!requires_sso_login(&profiles, "dev"));
    }

    #[test]
    fn matching_is_case_sensitive() {
        let profiles = vec![sso_session_profile("SSO-Dev", "my-sso")];
        assert!(requires_sso_login(&profiles, "SSO-Dev"));
        assert!(!requires_sso_login(&profiles, "sso-dev"));
        assert!(!requires_sso_login(&profiles, "SSO-DEV"));
    }
}
