mod common;

#[test]
fn workflow_shell_scripts_parse() {
    let root = common::repo_root();

    for script in ["tests/workflow/kernel_check.sh", "tests/workflow/boot_bios_check.sh"] {
        common::shell_parse_check(&root, script);
    }
}

#[test]
#[ignore = "requires rust target setup on host"]
fn kernel_check_script_runs() {
    let root = common::repo_root();
    common::run(&root, "bash", &["tests/workflow/kernel_check.sh"]);
}

#[test]
#[ignore = "requires qemu plus OS image build dependencies"]
fn bios_boot_check_script_runs() {
    let root = common::repo_root();
    common::run(&root, "bash", &["tests/workflow/boot_bios_check.sh"]);
}
