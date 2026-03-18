mod common;

use std::process::Command;

/// Returns a `Command` pre-configured to run the compiled binary.
/// `cargo test` builds it automatically since it's declared in [[bin]].
fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aws-profile-select"))
}

// ---------------------------------------------------------------------------
// --help / --version
// ---------------------------------------------------------------------------

#[test]
fn help_flag_prints_usage_and_exits_zero() {
    let output = bin().arg("--help").output().expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage"));
    assert!(stdout.contains("--init"));
    assert!(stdout.contains("--profile"));
}

#[test]
fn version_flag_prints_version_and_exits_zero() {
    let output = bin()
        .arg("--version")
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "expected version {} in output, got: {}",
        env!("CARGO_PKG_VERSION"),
        stdout
    );
}

// ---------------------------------------------------------------------------
// --init
// ---------------------------------------------------------------------------

#[test]
fn init_outputs_shell_wrapper_and_exits_zero() {
    // One happy-path check is enough — per-shell output is covered by unit tests in cli.rs
    let output = bin()
        .args(["--init", "zsh"])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "expected shell wrapper output");
}

#[test]
fn init_rejects_unknown_shell() {
    let output = bin()
        .args(["--init", "powershell"])
        .output()
        .expect("failed to run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid value"),
        "expected clap validation error, got: {}",
        stderr
    );
}

#[test]
fn init_does_not_require_config_file() {
    let output = bin()
        .args(["--init", "zsh"])
        .env("AWS_CONFIG_FILE", "/tmp/aws_ps_cli_no_such_file.ini")
        .env("HOME", "/tmp/aws_ps_cli_no_such_home")
        .output()
        .expect("failed to run binary");

    assert!(
        output.status.success(),
        "--init should succeed without a valid config file"
    );
}

// ---------------------------------------------------------------------------
// --profile
// ---------------------------------------------------------------------------

#[test]
fn profile_flag_sets_matching_profile() {
    let content = "\
[profile dev]
region = us-east-1

[profile prod]
region = eu-west-1
";
    let path = common::write_temp_config("aws_ps_cli_profile_match.ini", content);

    let output = bin()
        .args(["--profile", "dev"])
        .env("AWS_CONFIG_FILE", path.to_str().unwrap())
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("export AWS_PROFILE='dev'"),
        "expected export statement, got: {}",
        stdout
    );
}

#[test]
fn profile_short_flag_works() {
    let content = "\
[profile staging]
region = ap-southeast-1
";
    let path = common::write_temp_config("aws_ps_cli_short_profile.ini", content);

    let output = bin()
        .args(["-p", "staging"])
        .env("AWS_CONFIG_FILE", path.to_str().unwrap())
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("export AWS_PROFILE='staging'"));
}

#[test]
fn profile_flag_rejects_nonexistent_profile() {
    let content = "\
[profile dev]
region = us-east-1
";
    let path = common::write_temp_config("aws_ps_cli_profile_missing.ini", content);

    let output = bin()
        .args(["--profile", "nope"])
        .env("AWS_CONFIG_FILE", path.to_str().unwrap())
        .output()
        .expect("failed to run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found"),
        "expected 'not found' in error, got: {}",
        stderr
    );
}

// ---------------------------------------------------------------------------
// unknown flags
// ---------------------------------------------------------------------------

#[test]
fn rejects_unknown_flag() {
    let output = bin().arg("--bogus").output().expect("failed to run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected argument"),
        "expected clap error, got: {}",
        stderr
    );
}
