# whats-changed

Show Rust dependencies that were upgraded

Example output:

```
backends/Cargo.toml
    `swc_core` upgraded to version 55.0
    `toml_edit` upgraded to version 0.24
    `tree-sitter` upgraded to version 0.26
```

## How to run

Run `whats-changed` in the root of a Git repository and pass exactly one argument, a revision `PREVIOUS`.

```sh
whats-changed PREVIOUS
```

## How it works

`whats-changed` does the essentially following:

1. Clone the current repository into a temporary directory.
2. Checkout `PREVIOUS`.
3. For each dependency in the `[dependencies]` and `[workspace.dependencies]` sections of each Cargo.toml file in the current directory, compute the minimum version satisfying the dependency's version requirement.
4. If the minimum version does not satisfy the requirement in `PREVIOUS`'s corresponding Cargo.toml file, report that the dependency was upgraded.
5. If the dependency does not appear in `PREVIOUS`'s corresponding Cargo.toml file, report that it was removed.

Notes:

- `[dev-dependencies]` and `[build-dependencies]` are intentionally ignored.
- Newly added dependencies are intentionally not reported; only upgrades and removals are.

## Known problems

- If Cargo.toml files were moved or directories were renamed, `whats-changed` may not work correctly.
- `whats-changed` does not handle all possible version requirements, e.g., requirements with multiple comparators.
