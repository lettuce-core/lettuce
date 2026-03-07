mod common;

#[test]
fn local_shell_scripts_parse() {
    let root = common::repo_root();

    for script in [
        "tests/build_iso.sh",
        "tests/run_qemu.sh",
        "tests/dev_qemu.sh",
    ] {
        common::shell_parse_check(&root, script);
    }
}

#[test]
#[ignore = "requires host OS build tools (grub/xorriso/mtools)"]
fn build_iso_script_runs() {
    let root = common::repo_root();
    common::run(&root, "bash", &["tests/build_iso.sh", "bios"]);
}
