use anyhow::{Result, bail, ensure};
use elaborate::std::{fs::read_to_string_wc, path::PathContext, process::CommandContext};
use semver::{BuildMetadata, Comparator, Op, Version, VersionReq};
use std::{
    collections::HashMap, convert::identity, env::args, path::Path, process::Command,
    sync::LazyLock,
};

#[derive(Clone, Copy)]
enum DependencyOwner<'a> {
    Workspace,
    Package(&'a str),
}

struct Section {
    heading: String,
    messages: Vec<String>,
}

impl Section {
    fn new(owner: DependencyOwner<'_>, messages: Vec<String>) -> Self {
        let heading = match owner {
            DependencyOwner::Workspace => String::from("Workspace"),
            DependencyOwner::Package(name) => format!("Package: `{name}`"),
        };
        Self { heading, messages }
    }
}

impl std::fmt::Display for Section {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "## {}", self.heading)?;
        if let Some((first, rest)) = self.messages.split_first() {
            write!(formatter, "\n\n- {first}")?;
            for message in rest {
                write!(formatter, "\n- {message}")?;
            }
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = args().collect::<Vec<_>>();
    let prev_rev = match args.as_slice() {
        [_, prev_rev] => prev_rev.clone(),
        [_] => {
            let tag = most_recent_tag()?;
            eprintln!("No revision specified; using most recent tag: {tag}");
            tag
        }
        _ => bail!("expect at most one argument: previous revision"),
    };
    compare_repo_to_curr(&prev_rev)?;
    Ok(())
}

fn most_recent_tag() -> Result<String> {
    let mut command = Command::new("git");
    command.args(["describe", "--tags", "--abbrev=0"]);
    let output = command.output_wc()?;
    ensure!(
        output.status.success(),
        "no previous tag found; specify a revision explicitly"
    );
    let stdout = std::str::from_utf8(&output.stdout)?;
    let tag = stdout.trim().to_string();
    Ok(tag)
}

fn compare_repo_to_curr(prev_rev: &str) -> Result<()> {
    let mut command = Command::new("git");
    command.args(["ls-files"]);
    let output = command.output_wc()?;
    ensure!(output.status.success(), "command failed: {command:?}");
    let curr_paths = cargo_toml_paths(&output.stdout)?;

    let renames = collect_cargo_toml_renames(prev_rev)?;

    let mut sections = Vec::new();
    for path_curr_str in &curr_paths {
        let path_curr = Path::new(path_curr_str);
        let manifest_curr = read_manifest(path_curr)?;
        if get_publish(&manifest_curr).is_some_and(|publish| !publish) {
            continue;
        }
        let path_prev_str = renames.get(path_curr_str).unwrap_or(path_curr_str);
        let warning = format!(
            "`{}` does not exist in previous revision",
            path_curr.display()
        );
        let Some(manifest_prev) = read_manifest_at_rev(prev_rev, path_prev_str, &warning)? else {
            continue;
        };
        let manifest_sections =
            compare_manifests(path_prev_str, &manifest_prev, path_curr_str, &manifest_curr);
        sections.extend(manifest_sections);
    }

    let mut command = Command::new("git");
    command.args(["ls-tree", "-r", "--name-only", prev_rev]);
    let output = command.output_wc()?;
    ensure!(output.status.success(), "command failed: {command:?}");
    let prev_paths = cargo_toml_paths(&output.stdout)?;

    for path_prev_str in &prev_paths {
        if curr_paths.contains(path_prev_str) || renames.values().any(|old| old == path_prev_str) {
            continue;
        }
        let warning = format!("failed to read `{path_prev_str}` from previous revision");
        let Some(manifest_prev) = read_manifest_at_rev(prev_rev, path_prev_str, &warning)? else {
            continue;
        };
        if get_publish(&manifest_prev).is_some_and(|publish| !publish) {
            continue;
        }
        let manifest_sections = compare_manifests(
            path_prev_str,
            &manifest_prev,
            path_prev_str,
            &toml::Table::new(),
        );
        sections.extend(manifest_sections);
    }

    if !sections.is_empty() {
        let markdown = sections
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n\n");
        println!("{markdown}");
    }
    Ok(())
}

fn cargo_toml_paths(output: &[u8]) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    for line in output.split(|&byte| byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        let path_str = std::str::from_utf8(line)?;
        if Path::new(path_str).file_name_wc()? != "Cargo.toml" {
            continue;
        }
        paths.push(path_str.to_string());
    }
    Ok(paths)
}

fn collect_cargo_toml_renames(prev_rev: &str) -> Result<HashMap<String, String>> {
    let mut command = Command::new("git");
    command.args(["diff", "--name-status", "-M", "--diff-filter=R", prev_rev]);
    let output = command.output_wc()?;
    ensure!(output.status.success(), "command failed: {command:?}");
    let mut renames = HashMap::new();
    for line in output.stdout.split(|&byte| byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        let line_str = std::str::from_utf8(line)?;
        let mut fields = line_str.split('\t');
        let (Some(_status), Some(path_prev_str), Some(path_curr_str)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if Path::new(path_prev_str).file_name_wc()? == "Cargo.toml"
            && Path::new(path_curr_str).file_name_wc()? == "Cargo.toml"
        {
            renames.insert(path_curr_str.to_string(), path_prev_str.to_string());
        }
    }
    Ok(renames)
}

fn read_manifest(manifest_path: impl AsRef<Path>) -> Result<toml::Table> {
    let contents = read_to_string_wc(manifest_path)?;
    contents.parse::<toml::Table>().map_err(Into::into)
}

fn get_publish(manifest: &toml::Table) -> Option<bool> {
    manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("publish"))
        .and_then(toml::Value::as_bool)
}

fn read_manifest_at_rev(prev_rev: &str, path: &str, warning: &str) -> Result<Option<toml::Table>> {
    let mut command = Command::new("git");
    command.args(["show", &format!("{prev_rev}:{path}")]);
    let output = command.output_wc()?;
    if !output.status.success() {
        eprintln!("{warning}");
        return Ok(None);
    }
    let contents = std::str::from_utf8(&output.stdout)?;
    let manifest = contents.parse::<toml::Table>()?;
    Ok(Some(manifest))
}

fn compare_manifests(
    path_prev: &str,
    manifest_prev: &toml::Table,
    path_curr: &str,
    manifest_curr: &toml::Table,
) -> Vec<Section> {
    let mut sections = Vec::new();
    push_section(
        &mut sections,
        get_workspace_deps_table(manifest_prev),
        get_workspace_deps_table(manifest_curr),
    );
    push_section(
        &mut sections,
        get_package_deps_table(path_prev, manifest_prev),
        get_package_deps_table(path_curr, manifest_curr),
    );
    sections
}

fn push_section(
    sections: &mut Vec<Section>,
    deps_prev: Option<(&toml::Table, DependencyOwner<'_>)>,
    deps_curr: Option<(&toml::Table, DependencyOwner<'_>)>,
) {
    static EMPTY: LazyLock<toml::Table> = LazyLock::new(toml::Table::default);
    let Some(messages) = compare_deps_tables(
        deps_prev.map_or(&EMPTY, |(deps, _)| deps),
        deps_curr.map_or(&EMPTY, |(deps, _)| deps),
    ) else {
        return;
    };
    let owner = deps_curr
        .map(|(_, owner)| owner)
        .or_else(|| deps_prev.map(|(_, owner)| owner))
        .unwrap();
    sections.push(Section::new(owner, messages));
}

fn get_workspace_deps_table(manifest: &toml::Table) -> Option<(&toml::Table, DependencyOwner<'_>)> {
    let deps = manifest
        .get("workspace")
        .and_then(|value| value.as_table())
        .and_then(|table| table.get("dependencies"))
        .and_then(|value| value.as_table())?;
    Some((deps, DependencyOwner::Workspace))
}

fn get_package_deps_table<'a>(
    path: &str,
    manifest: &'a toml::Table,
) -> Option<(&'a toml::Table, DependencyOwner<'a>)> {
    let deps = manifest
        .get("dependencies")
        .and_then(|value| value.as_table())?;
    let Some(name) = get_package_name(manifest) else {
        eprintln!("`{path}` has a dependencies table but no package name");
        return None;
    };
    Some((deps, DependencyOwner::Package(name)))
}

fn get_package_name(manifest: &toml::Table) -> Option<&str> {
    manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("name"))
        .and_then(toml::Value::as_str)
}

fn compare_deps_tables(deps_prev: &toml::Table, deps_curr: &toml::Table) -> Option<Vec<String>> {
    let mut messages = Vec::new();
    for (name_prev, value_prev) in deps_prev {
        let result = (|| {
            let Some(value_curr) = deps_curr.get(name_prev) else {
                // Don't report a git, path, or workspace-inherited dependency as removed; such
                // dependencies are ignored when present too.
                let Some(_) = get_req_from_value(value_prev)? else {
                    return Ok(None);
                };
                return Ok(Some(format!("`{name_prev}` removed")));
            };
            compare_deps(name_prev, value_prev, value_curr)
        })();
        match result {
            Ok(None) => {}
            Ok(Some(msg)) => {
                messages.push(msg);
            }
            Err(err) => {
                eprintln!("failed to compare `{name_prev}`: {err}");
            }
        }
    }
    (!messages.is_empty()).then_some(messages)
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
