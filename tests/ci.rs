use assert_cmd::assert::OutputAssertExt;
use std::process::Command;

#[test]
fn clippy() {
    Command::new("cargo")
        .args([
            "+nightly",
            "clippy",
            "--all-targets",
            "--offline",
            "--",
            "--deny=warnings",
        ])
        .assert()
        .success();
}

#[test]
fn dylint() {
    Command::new("cargo")
        .args(["dylint", "--all", "--", "--all-targets"])
        .env("DYLINT_RUSTFLAGS", "--deny=warnings")
        .assert()
        .success();
}

#[test]
fn elaborate_disallowed_methods() {
    elaborate::disallowed_methods()
        .args(["--all-features", "--all-targets"])
        .env("RUSTUP_TOOLCHAIN", "nightly")
        .assert()
        .success();
}
