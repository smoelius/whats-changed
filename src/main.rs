use anyhow::{Result, bail, ensure};
use semver::{BuildMetadata, Comparator, Op, Version, VersionReq};
use std::{
    convert::identity, env::args, fs::read_to_string, path::Path, process::Command, sync::LazyLock,
};
use tempfile::tempdir;
use walkdir::WalkDir;

fn main() -> Result<()> {
    let args = args().collect::<Vec<_>>();
    let [_, prev_rev] = args.as_slice() else {
        bail!("expect one argument: previous revision");
    };
    let tempdir = tempdir()?;
    let tempdir_path = tempdir.path();
    clone_to(tempdir_path)?;
    checkout(tempdir_path, prev_rev)?;
    compare_repo_to_curr(tempdir_path)?;
    Ok(())
}

fn clone_to(dir: &Path) -> Result<()> {
    let mut command = Command::new("git");
    command.args([
        "clone",
        ".",
        "--config=advice.detachedHead=false",
        "--quiet",
    ]);
    command.arg(dir);
    let status = command.status()?;
    ensure!(status.success(), "command failed: {command:?}");
    Ok(())
}

fn checkout(dir: &Path, rev: &str) -> Result<()> {
    let mut command = Command::new("git");
    command.args(["checkout", "--quiet", rev]);
    command.current_dir(dir);
    let status = command.status()?;
    ensure!(status.success(), "command failed: {command:?}");
    Ok(())
}

fn compare_repo_to_curr(tempdir_path: &Path) -> Result<()> {
    for result in WalkDir::new(".") {
        let dir_entry = result?;
        if dir_entry.file_name() != "Cargo.toml" {
            continue;
        }
        let path_curr = dir_entry.path();
        let relative_path = path_curr.strip_prefix(".").unwrap();
        let path_prev = tempdir_path.join(relative_path);
        if !path_prev.try_exists()? {
            eprintln!(
                "`{}` does not exists in previous revision",
                relative_path.display()
            );
            continue;
        }
        compare_manifests_at_paths(relative_path, &path_prev, path_curr)?;
    }
    Ok(())
}

fn compare_manifests_at_paths(
    relative_path: &Path,
    path_prev: &Path,
    path_curr: &Path,
) -> Result<()> {
    let manifest_prev = read_manifest(path_prev)?;
    let manifest_curr = read_manifest(path_curr)?;
    compare_manifests(relative_path, &manifest_prev, &manifest_curr);
    Ok(())
}

fn read_manifest(manifest_path: impl AsRef<Path>) -> Result<toml::Table> {
    let contents = read_to_string(manifest_path)?;
    contents.parse::<toml::Table>().map_err(Into::into)
}

fn compare_manifests(
    relative_path: &Path,
    manifest_prev: &toml::Table,
    manifest_curr: &toml::Table,
) {
    let deps_prev = get_deps_table(manifest_prev);
    let deps_curr = get_deps_table(manifest_curr);
    compare_deps_tables(relative_path, deps_prev, deps_curr);
}

fn get_deps_table(manifest: &toml::Table) -> &toml::Table {
    static EMPTY: LazyLock<toml::Table> = LazyLock::new(toml::Table::default);
    if let Some(deps) = manifest
        .get("dependencies")
        .and_then(|value| value.as_table())
    {
        deps
    } else if let Some(deps) = manifest
        .get("workspace")
        .and_then(|value| value.as_table())
        .and_then(|table| table.get("dependencies"))
        .and_then(|value| value.as_table())
    {
        deps
    } else {
        // smoelius: Manifest has no `dependencies` table.
        &EMPTY
    }
}

fn compare_deps_tables(relative_path: &Path, deps_prev: &toml::Table, deps_curr: &toml::Table) {
    let mut path_printed = false;
    let mut iter_curr = deps_curr.iter().peekable();
    for (name_prev, value_prev) in deps_prev {
        let result = (|| {
            if iter_curr
                .peek()
                .is_some_and(|&(name_curr, _)| name_prev != name_curr)
            {
                return Ok(Some(format!("    `{name_prev}` removed")));
            }
            let Some((_, value_curr)) = iter_curr.next() else {
                return Ok(Some(format!("    `{name_prev}` removed")));
            };
            compare_deps(name_prev, value_prev, value_curr)
        })();
        match result {
            Ok(None) => {}
            Ok(Some(msg)) => {
                maybe_print_path(&mut path_printed, relative_path);
                println!("    {msg}");
            }
            Err(err) => {
                maybe_print_path(&mut path_printed, relative_path);
                eprintln!("failed to compare `{name_prev}`: {err}");
            }
        }
    }
}

fn compare_deps(
    name: &str,
    value_prev: &toml::Value,
    value_curr: &toml::Value,
) -> Result<Option<String>> {
    let Some(req_prev) = get_req_from_value(value_prev)? else {
        return Ok(None);
    };
    let Some(req_curr) = get_req_from_value(value_curr)? else {
        return Ok(None);
    };
    let minimum_version = minimum_version_for_req(&req_curr)?;
    if req_prev.matches(&minimum_version) {
        Ok(None)
    } else {
        let req_with_op = req_curr.to_string();
        let index_of_first_digit = req_with_op
            .as_bytes()
            .iter()
            .position(u8::is_ascii_digit)
            .unwrap();
        Ok(Some(format!(
            "`{name}` upgraded to version {}",
            &req_with_op[index_of_first_digit..]
        )))
    }
}

fn get_req_from_value(value: &toml::Value) -> Result<Option<VersionReq>> {
    // smoelius: Skip git dependencies.
    if value
        .as_table()
        .and_then(|table| table.get("git"))
        .is_some()
    {
        return Ok(None);
    }
    // smoelius: Skip path dependencies.
    if value
        .as_table()
        .and_then(|table| table.get("path"))
        .is_some()
    {
        return Ok(None);
    }
    // smoelius: Skip dependencies inherited from a workspace.
    if value
        .as_table()
        .and_then(|table| table.get("workspace"))
        .and_then(toml::Value::as_bool)
        .is_some_and(identity)
    {
        return Ok(None);
    }
    let req = if let Some(req) = value.as_str() {
        req
    } else if let Some(req) = value
        .as_table()
        .and_then(|table| table.get("version"))
        .and_then(|value| value.as_str())
    {
        req
    } else {
        bail!("failed to get version requirement");
    };
    let req = req.parse::<VersionReq>()?;
    Ok(Some(req))
}

fn maybe_print_path(printed: &mut bool, path: &Path) {
    if *printed {
        return;
    }
    println!("{}", path.display());
    *printed = true;
}

fn minimum_version_for_req(req: &VersionReq) -> Result<Version> {
    let VersionReq { comparators } = req;
    let [comparator] = comparators.as_slice() else {
        bail!("unexpected number of comparators: {}", comparators.len());
    };
    let Comparator {
        op,
        major,
        minor,
        patch,
        pre,
    } = comparator;
    match op {
        Op::Caret | Op::Exact => {
            let minor = minor.unwrap_or(0);
            let patch = patch.unwrap_or(0);
            Ok(Version {
                major: *major,
                minor,
                patch,
                pre: pre.clone(),
                build: BuildMetadata::default(),
            })
        }
        _ => bail!("unexpected operator: {op:?}"),
    }
}
