use assert_cmd::{assert::OutputAssertExt, cargo::cargo_bin_cmd};
use elaborate::std::fs::{
    DirEntryContext, copy_wc, create_dir_all_wc, read_dir_wc, read_to_string_wc, remove_file_wc,
    write_wc,
};
use std::{path::Path, process::Command};
use tempfile::tempdir;

#[test]
fn cases() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cases_dir = Path::new(manifest_dir).join("cases");

    let mut entries: Vec<_> = read_dir_wc(&cases_dir)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        run_case(&entry.path());
    }
}

fn run_case(case_dir: &Path) {
    assert!(
        case_dir.join("description.txt").exists(),
        "{}: missing description.txt",
        case_dir.display()
    );

    let tempdir = tempdir().unwrap();
    let repo_dir = tempdir.path();

    init_repo(repo_dir);

    copy_wc(case_dir.join("before.toml"), repo_dir.join("Cargo.toml")).unwrap();

    let before_extra_dir = case_dir.join("before_extra");
    if before_extra_dir.exists() {
        copy_dir(&before_extra_dir, repo_dir);
    }

    let prev_rev = commit_all(repo_dir);
    let prev_rev = prev_rev.as_str();

    let tag_path = case_dir.join("tag.txt");
    if tag_path.exists() {
        let tag = read_to_string_wc(&tag_path).unwrap();
        let tag = tag.trim();
        git(&["tag", tag]).current_dir(repo_dir).assert().success();
    }

    copy_wc(case_dir.join("after.toml"), repo_dir.join("Cargo.toml")).unwrap();

    // Remove any `before_extra` files so that they are absent from the current working tree,
    // unless `after_extra` re-creates them (e.g., at a renamed path).
    if before_extra_dir.exists() {
        remove_mirrored(&before_extra_dir, repo_dir);
    }

    let extra_dir = case_dir.join("extra");
    if extra_dir.exists() {
        let repo_extra_dir = repo_dir.join("extra");
        create_dir_all_wc(&repo_extra_dir).unwrap();
        copy_dir(&extra_dir, &repo_extra_dir);
    }

    let after_extra_dir = case_dir.join("after_extra");
    if after_extra_dir.exists() {
        copy_dir(&after_extra_dir, repo_dir);
    }

    // Stage additions and removals so that `git ls-files` reflects the current working tree.
    git(&["add", "-A"]).current_dir(repo_dir).assert().success();

    let expected_status: i32 = read_to_string_wc(case_dir.join("status.txt"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let expected_stdout = read_to_string_wc(case_dir.join("stdout.txt")).unwrap();
    let expected_stderr = read_to_string_wc(case_dir.join("stderr.txt")).unwrap();

    let args: Vec<String> = if case_dir.join("no_args.txt").exists() {
        vec![]
    } else {
        vec![prev_rev.to_string()]
    };

    let mut cmd = cargo_bin_cmd!("whats-changed");
    cmd.current_dir(repo_dir);
    for arg in &args {
        cmd.arg(arg);
    }
    cmd.assert()
        .code(expected_status)
        .stdout(expected_stdout)
        .stderr(expected_stderr);
}

// `git ls-files` lists a tracked file even after it has been deleted from disk without staging
// the deletion, which previously made `whats-changed` fail trying to read the missing file
// instead of treating it as a removed package. This can't be expressed as a case under `cases/`,
// since `run_case` always stages every change (`git add -A`) before running the binary.
#[test]
fn unstaged_deletion_is_reported_as_removed() {
    let tempdir = tempdir().unwrap();
    let repo_dir = tempdir.path();

    init_repo(repo_dir);

    write_wc(
        repo_dir.join("Cargo.toml"),
        "[package]\nname = \"test-package\"\n\n[dependencies]\nfoo = \"1.0\"\n",
    )
    .unwrap();

    let prev_rev = commit_all(repo_dir);

    // Delete the file from disk without staging the deletion (no `git rm`/`git add`).
    remove_file_wc(repo_dir.join("Cargo.toml")).unwrap();

    let mut cmd = cargo_bin_cmd!("whats-changed");
    cmd.current_dir(repo_dir);
    cmd.arg(&prev_rev);
    cmd.assert()
        .success()
        .stdout("## Package: `test-package`\n\n- `foo` removed\n")
        .stderr("");
}

fn init_repo(repo_dir: &Path) {
    git(&["init"]).current_dir(repo_dir).assert().success();

    git(&["config", "user.email", "test@test.com"])
        .current_dir(repo_dir)
        .assert()
        .success();

    git(&["config", "user.name", "Test"])
        .current_dir(repo_dir)
        .assert()
        .success();
}

fn copy_dir(src: &Path, dst: &Path) {
    for entry in read_dir_wc(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type_wc().unwrap().is_dir() {
            create_dir_all_wc(&dst_path).unwrap();
            copy_dir(&src_path, &dst_path);
        } else {
            copy_wc(&src_path, &dst_path).unwrap();
        }
    }
}

fn commit_all(repo_dir: &Path) -> String {
    git(&["add", "-A"]).current_dir(repo_dir).assert().success();

    git(&["commit", "-m", "init"])
        .current_dir(repo_dir)
        .assert()
        .success();

    let rev_parse = git(&["rev-parse", "HEAD"])
        .current_dir(repo_dir)
        .assert()
        .success();
    let prev_rev = String::from_utf8(rev_parse.get_output().stdout.clone()).unwrap();
    prev_rev.trim().to_string()
}

fn git(args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command.args(args);
    command
}

fn remove_mirrored(src: &Path, dst: &Path) {
    for entry in read_dir_wc(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type_wc().unwrap().is_dir() {
            remove_mirrored(&src_path, &dst_path);
        } else {
            remove_file_wc(&dst_path).unwrap();
        }
    }
}
