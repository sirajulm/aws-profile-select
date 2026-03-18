use aws_profile_select::Profile;
use std::error::Error;
use std::process::{Command, Stdio};

/// If the chosen profile is configured for SSO, validates the current session
/// and triggers `aws sso login` when necessary.
///
/// Non-SSO profiles are left untouched.
pub fn handle_sso_login(profiles: &[Profile], chosen_profile: &str) -> Result<(), Box<dyn Error>> {
    let profile_uses_sso = profiles
        .iter()
        .find(|p| p.name == chosen_profile)
        .map(|p| p.is_sso())
        .unwrap_or(false);

    if profile_uses_sso {
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
                    format!(
                        "Failed to execute 'aws' command. Is it installed and in PATH? Error: {e}"
                    )
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
    }

    Ok(())
}
