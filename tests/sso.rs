//! SSO session handling, run against a stub `aws` executable on `PATH`.
//!
//! The stub records every invocation and fails `sts` and/or `sso login`
//! depending on the scenario, so the tests never touch real AWS.
#![cfg(unix)]

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CONFIG: &str = "\
[sso-session corp]
sso_start_url = https://example.awsapps.com/start
sso_region = eu-central-1

[profile sso-dev]
sso_session = corp
sso_account_id = 111111111111
sso_role_name = Dev

[profile chained]
role_arn = arn:aws:iam::222222222222:role/x
source_profile = sso-dev

[profile static]
region = eu-central-1
";

/// A temp directory holding the stub `aws`, its call log, and the config.
struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    /// `sts_ok`: whether `aws sts get-caller-identity` succeeds.
    /// `login_ok`: whether `aws sso login` succeeds.
    fn new(name: &str, sts_ok: bool, login_ok: bool) -> Self {
        let dir = env::temp_dir().join(format!("aws_ps_sso_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create sandbox dir");

        let stub = format!(
            "#!/bin/sh\n\
             echo \"$*\" >> \"{log}\"\n\
             case \"$1\" in\n\
               sts) exit {sts} ;;\n\
               sso) exit {login} ;;\n\
             esac\n\
             exit 0\n",
            log = dir.join("calls").display(),
            sts = if sts_ok { 0 } else { 255 },
            login = if login_ok { 0 } else { 1 },
        );
        let aws = dir.join("aws");
        fs::write(&aws, stub).expect("write stub aws");
        fs::set_permissions(&aws, fs::Permissions::from_mode(0o755)).expect("chmod stub aws");
        fs::write(dir.join("config"), CONFIG).expect("write config");

        Sandbox { dir }
    }

    fn run(&self, profile: &str) -> Output {
        let path = format!(
            "{}:{}",
            self.dir.display(),
            env::var("PATH").unwrap_or_default()
        );
        Command::new(env!("CARGO_BIN_EXE_aws-profile-select"))
            .args(["--profile", profile])
            .env("PATH", path)
            .env("AWS_CONFIG_FILE", self.dir.join("config"))
            .output()
            .expect("failed to run binary")
    }

    fn calls(&self) -> Vec<String> {
        read_lines(&self.dir.join("calls"))
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn read_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .map(|s| s.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

#[test]
fn valid_session_does_not_log_in() {
    let sb = Sandbox::new("valid", true, true);
    let out = sb.run("sso-dev");

    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("AWS_PROFILE='sso-dev'"));
    assert_eq!(
        sb.calls(),
        ["sts get-caller-identity --profile sso-dev --no-cli-pager"]
    );
}

#[test]
fn expired_session_logs_in_once() {
    let sb = Sandbox::new("expired", false, true);
    let out = sb.run("sso-dev");

    assert!(out.status.success());
    assert_eq!(
        sb.calls(),
        [
            "sts get-caller-identity --profile sso-dev --no-cli-pager",
            "sso login --profile sso-dev",
        ]
    );
}

#[test]
fn assume_role_profile_logs_in_with_sso_source() {
    let sb = Sandbox::new("chained", false, true);
    let out = sb.run("chained");

    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("AWS_PROFILE='chained'"));
    assert_eq!(
        sb.calls(),
        [
            "sts get-caller-identity --profile sso-dev --no-cli-pager",
            "sso login --profile sso-dev",
        ]
    );
}

#[test]
fn failed_login_reports_sso_profile() {
    let sb = Sandbox::new("login_fails", false, false);
    let out = sb.run("chained");

    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("aws sso login --profile sso-dev"));
}

#[test]
fn non_sso_profile_skips_aws() {
    let sb = Sandbox::new("static", false, false);
    let out = sb.run("static");

    assert!(out.status.success());
    assert!(sb.calls().is_empty());
}
