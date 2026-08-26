# whats-changed

Show Rust dependencies that were upgraded or removed

Example output:

```markdown
## Package: `swc_ecma_parser`

- `swc_core` upgraded to version 55.0
- `toml_edit` upgraded to version 0.24
- `tree-sitter` upgraded to version 0.26
```

## How to run

Run `whats-changed` in the root of a Git repository. You may pass a revision, `PREVIOUS`, to compare against. If you omit it, the tool uses the most recent tag.

```sh
whats-changed [PREVIOUS]
```

## How it works

`whats-changed` does essentially the following:

1. Use `git ls-files` to find Cargo.toml files in the working tree, and `git ls-tree -r --name-only PREVIOUS` to find those that existed in `PREVIOUS`.
2. Use `git diff --name-status -M PREVIOUS` to detect renamed Cargo.toml files, so a renamed package is compared against its own previous content rather than as two unrelated files.
3. Read each manifest's previous version with `git show PREVIOUS:path/to/Cargo.toml`. A manifest present only in the working tree (and not a rename target) is skipped with a warning; a manifest present only in `PREVIOUS` (and not a rename source) is treated as a deleted package, and its dependencies are reported as removed.
4. Skip packages whose manifest specifies `publish = false`.
5. Read each manifest's `[workspace.dependencies]` and `[dependencies]` tables, in that order. A `[dependencies]` table with no resolvable `[package].name` triggers a warning and is skipped; `[workspace.dependencies]`, if present, is still compared.
6. For each dependency in those tables from `PREVIOUS`:
   - If it does not appear in the current table, report that it was removed — except for Git, path, and workspace-inherited dependencies, which are never reported as removed.
   - Otherwise, compute the minimum version satisfying its current version requirement; report an upgrade if that version does not satisfy the previous requirement.

Notes:

- `[dev-dependencies]` and `[build-dependencies]` are intentionally ignored.
- Git dependencies, path dependencies, and dependencies inherited from a workspace are intentionally excluded from version comparisons, including from being reported as removed.
- Newly added dependencies are intentionally not reported; only upgrades and removals are.
- If every dependency in a table fails to compare, a warning is printed for each failure, but no section heading is emitted for that table.

## Known problems

- Cargo.toml renames are detected using git's similarity heuristic (`git diff -M`); a rename combined with substantial content changes may not be recognized as a rename.
- `whats-changed` does not handle all possible version requirements, e.g., requirements with multiple comparators.
