# aws-profile-select

Interactive AWS profile selector for the terminal. Switch between AWS profiles with a simple TUI — no more hand-editing `AWS_PROFILE` or remembering profile names.

<p>
  <img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue" alt="MIT/Apache 2.0">
  <a href="https://codecov.io/gh/sirajulm/aws-profile-select">
    <img src="https://codecov.io/gh/sirajulm/aws-profile-select/graph/badge.svg?token=PIJ6BGAYYI" alt="codecov">
  </a>
</p>

## Features

- **Interactive selection** — fuzzy-searchable list of all profiles from your AWS config
- **Environment grouping** — profiles with an `environment` field are grouped by environment for easy navigation
- **SSO support** — automatically detects SSO profiles and triggers `aws sso login` when the session has expired
- **Shell integration** — provides an `asp` shell function that sets `AWS_PROFILE` in your current session
- **Direct profile flag** — skip interactive mode with `--profile <name>` for scripting and automation
- **Profile annotations** — annotate profiles with `duration` and `readonly` fields, displayed as `profile (8h) 🔥` or `profile 👀` in the selector
- **Respects `AWS_CONFIG_FILE`** — works with custom config file locations

## Installation

### Using mise

```sh
mise use -g github:sirajulm/aws-profile-select
```

### From source

```sh
cargo install --path .
```

### Pre-built binaries

Check the [Releases](https://github.com/sirajulm/aws-profile-select/releases) page for pre-built binaries for your platform.

## Shell Setup

`aws-profile-select` needs to set an environment variable (`AWS_PROFILE`) in your **current** shell session. To make this work, add the shell wrapper to your shell's config file.

### Zsh

Add to your `~/.zshrc`:

```sh
eval "$(aws-profile-select --init zsh)"
```

### Bash

Add to your `~/.bashrc`:

```sh
eval "$(aws-profile-select --init bash)"
```

### Fish

Add to your `~/.config/fish/config.fish`:

```fish
aws-profile-select --init fish | source
```

This defines an `asp` function you can use instead of calling the binary directly.

## Usage

### Interactive mode

```sh
asp
```

This opens an interactive selector showing all your AWS profiles. If your profiles have `environment` fields, you'll first pick an environment, then a profile within it.

### Direct profile selection

```sh
asp --profile dev
# or
asp -p dev
```

Skips the interactive menu and sets the given profile directly. Useful in scripts.

### Print shell wrapper

```sh
aws-profile-select --init zsh
```

Outputs the shell wrapper function. This is what you `eval` in your shell config.

### Help & version

```sh
aws-profile-select --help
aws-profile-select --version
```

## AWS Config File

`aws-profile-select` reads profiles from your AWS config file. By default this is `~/.aws/config`, but you can override it with the `AWS_CONFIG_FILE` environment variable.

### Basic example

```ini
[default]
region = us-east-1

[profile dev]
region = us-east-1

[profile prod]
region = eu-west-1
```

### With environments

Add an `environment` field to group profiles in the interactive selector:

```ini
[profile dev-admin]
region = us-east-1
environment = development

[profile prod-admin]
region = eu-west-1
environment = production

[profile prod-readonly]
region = eu-west-1
environment = production
```

### With SSO

SSO profiles are automatically detected. When you select one, `aws-profile-select` checks your session and runs `aws sso login` if needed.

```ini
[sso-session my-sso]
sso_start_url = https://my-org.awsapps.com/start
sso_region = us-east-1

[profile sso-dev]
sso_session = my-sso
sso_account_id = 123456789012
sso_role_name = DevAccess
region = us-east-1
```

### With duration and readonly annotations

Add `duration` and `readonly` fields to display extra context in the interactive selector. These are custom fields used only by `aws-profile-select` — they don't affect AWS CLI behaviour.

```ini
[profile prod-admin]
region = eu-west-1
environment = production
duration = 8h
readonly = false

[profile prod-readonly]
region = eu-west-1
environment = production
duration = 4h
readonly = true

[profile dev-admin]
region = us-east-1
environment = development
duration = 1h
```

In the selector, these render as:

```
prod-admin (8h) 🔥
prod-readonly (4h) 👀
dev-admin (1h)
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `duration` | string | _(none)_ | Session duration hint, e.g. `1h`, `8h`, `30m` — displayed in brackets |
| `readonly` | `true` / `false` | _(none)_ | 👀 when `true`, 🔥 when `false` — no icon when field is absent |

## Development

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.94.0+
- [mise](https://mise.jdx.dev/) (optional, for task running)

### Setup

```sh
git clone https://github.com/sirajulm/aws-profile-select.git
cd aws-profile-select
mise install  # installs toolchain and CLI tools
```

### Common tasks

All tasks are defined in `.mise.toml` and can be run with `mise run <task>`:

| Task | Command | Description |
|------|---------|-------------|
| `build` | `cargo build` | Build the project |
| `release` | `cargo build --release` | Build in release mode |
| `install` | `cargo install --path . --force` | Install binary to `~/.cargo/bin` |
| `test` | `cargo test` | Run all tests |
| `lint` | `cargo clippy -- -D warnings` | Run clippy lints |
| `fmt` | `cargo fmt` | Format code |
| `fmt-check` | `cargo fmt -- --check` | Check formatting |
| `coverage` | `cargo llvm-cov --html` | Generate HTML coverage report |
| `coverage-report` | `cargo llvm-cov --open` | Generate and open coverage report |

### Project structure

```
src/
├── lib.rs                      # Core library (profile parsing, config resolution)
└── aws-profile-select/
    ├── main.rs                 # Binary entrypoint
    ├── cli.rs                  # CLI argument parsing (clap) and shell init
    ├── prompt.rs               # Interactive TUI (dialoguer)
    └── sso.rs                  # SSO session detection and login
tests/
├── common/mod.rs               # Shared test helpers
├── cli.rs                      # Integration tests for the binary
└── parse_profiles.rs           # Integration tests for profile parsing
```

### Configuration files

| File | Purpose |
|------|---------|
| `.mise.toml` | Task runner and tool version management |
| `clippy.toml` | Clippy lint configuration |
| `rustfmt.toml` | Code formatting rules |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
