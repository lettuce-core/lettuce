#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn assert_script_exists(root: &Path, script: &str) {
    let script_path = root.join(script);

    assert!(
        script_path.exists(),
        "missing script: {}",
        script_path.display()
    );
}

pub fn shell_parse_check(root: &Path, script: &str) {
    assert_script_exists(root, script);
    run(root, "bash", &["-n", script]);
}

pub fn run(root: &Path, cmd: &str, args: &[&str]) {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(root)
        .status()
        .unwrap_or_else(|err| panic!("failed to run `{cmd}`: {err}"));

    assert!(
        status.success(),
        "command failed: {} {}",
        cmd,
        args.join(" ")
    );
}
