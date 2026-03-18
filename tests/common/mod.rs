use std::env;
use std::fs;
use std::path::PathBuf;

/// Write `content` to a file inside the system temp directory and return its
/// path. Using unique file names avoids collisions when tests run in parallel.
pub fn write_temp_config(file_name: &str, content: &str) -> PathBuf {
    let path = env::temp_dir().join(file_name);
    fs::write(&path, content).expect("failed to write temp config file");
    path
}
