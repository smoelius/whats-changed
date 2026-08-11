# whats-changed

Show Rust dependencies that were upgraded or removed

Example output:

```
backends/Cargo.toml
    `swc_core` upgraded to version 55.0
    `toml_edit` upgraded to version 0.24
    `tree-sitter` upgraded to version 0.26
```

## How to run

Run `whats-changed` in the root of a Git repository. You may pass a revision, `PREVIOUS`, to compare against. If you omit it, the tool uses the most recent tag.

```sh
whats-changed [PREVIOUS]
```

## How it works

`whats-changed` does essentially the following:

1. Use `git ls-files` to find tracked Cargo.toml files in the current repository.
2. Read each manifest from the working tree and its previous version with `git show PREVIOUS:path/to/Cargo.toml`.
3. Skip packages whose current manifest specifies `publish = false` and manifests that do not exist in `PREVIOUS`.
4. Read the manifest's `[workspace.dependencies]` and `[dependencies]` tables, in that order.
5. For each dependency in those tables from `PREVIOUS`:
   - If it does not appear in the current table, report that it was removed.
   - Otherwise, compute the minimum version satisfying its current version requirement. If that version does not satisfy the previous requirement, report that the dependency was upgraded.

Notes:

- `[dev-dependencies]` and `[build-dependencies]` are intentionally ignored.
- Git dependencies, path dependencies, and dependencies inherited from a workspace are intentionally excluded from version comparisons.
- Newly added dependencies are intentionally not reported; only upgrades and removals are.

## Known problems

- If Cargo.toml files were moved or directories were renamed, `whats-changed` may not work correctly.
- `whats-changed` does not handle all possible version requirements, e.g., requirements with multiple comparators.
