use anyhow::{Result, bail, ensure};
use elaborate::std::{fs::read_to_string_wc, path::PathContext, process::CommandContext};
use semver::{BuildMetadata, Comparator, Op, Version, VersionReq};
use std::{convert::identity, env::args, path::Path, process::Command, sync::LazyLock};

fn main() -> Result<()> {
    let args = args().collect::<Vec<_>>();
    let [_, prev_rev] = args.as_slice() else {
        bail!("expect one argument: previous revision");
    };
    compare_repo_to_curr(prev_rev)?;
    Ok(())
}

fn compare_repo_to_curr(prev_rev: &str) -> Result<()> {
    let mut command = Command::new("git");
    command.args(["ls-files"]);
    let output = command.output_wc()?;
    ensure!(output.status.success(), "command failed: {command:?}");
    for line in output.stdout.split(|&byte| byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        let path_curr_str = std::str::from_utf8(line)?;
        let path_curr = Path::new(path_curr_str);
        if path_curr.file_name_wc()? != "Cargo.toml" {
            continue;
        }
        let mut command = Command::new("git");
        command.args(["show", &format!("{prev_rev}:{path_curr_str}")]);
        let output = command.output_wc()?;
        if !output.status.success() {
            eprintln!(
                "`{}` does not exist in previous revision",
                path_curr.display()
            );
            continue;
        }
        let contents_prev = std::str::from_utf8(&output.stdout)?;
        let manifest_prev = contents_prev.parse::<toml::Table>()?;
        let manifest_curr = read_manifest(path_curr)?;
        compare_manifests(path_curr, &manifest_prev, &manifest_curr);
    }
    Ok(())
}

fn read_manifest(manifest_path: impl AsRef<Path>) -> Result<toml::Table> {
    let contents = read_to_string_wc(manifest_path)?;
    contents.parse::<toml::Table>().map_err(Into::into)
}

fn compare_manifests(path_curr: &Path, manifest_prev: &toml::Table, manifest_curr: &toml::Table) {
    let deps_prev = get_deps_table(manifest_prev);
    let deps_curr = get_deps_table(manifest_curr);
    compare_deps_tables(path_curr, deps_prev, deps_curr);
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

fn compare_deps_tables(path_curr: &Path, deps_prev: &toml::Table, deps_curr: &toml::Table) {
    let mut path_printed = false;
    for (name_prev, value_prev) in deps_prev {
        let result = (|| {
            let Some(value_curr) = deps_curr.get(name_prev) else {
                return Ok(Some(format!("`{name_prev}` removed")));
            };
            compare_deps(name_prev, value_prev, value_curr)
        })();
        match result {
            Ok(None) => {}
            Ok(Some(msg)) => {
                maybe_print_path(&mut path_printed, path_curr);
                println!("    {msg}");
            }
            Err(err) => {
                maybe_print_path(&mut path_printed, path_curr);
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
