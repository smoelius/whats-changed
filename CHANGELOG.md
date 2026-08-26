# Changelog

## 1.0.0

- BREAKING: Always check `workspace.dependencies` followed by `dependencies` ([5d798d2](https://github.com/smoelius/whats-changed/commit/5d798d2a2391b80688f681a049afaf9932fbbeb6))
- BREAKING: Change output format to markdown ([f07472a](https://github.com/smoelius/whats-changed/commit/f07472a48b060898b34769a23bc248585b91ed21))
- Don't print a section heading when every dependency comparison errors ([06f5ea9](https://github.com/smoelius/whats-changed/commit/06f5ea9056dbcd548d85dccb16c8973c5a6dde6c))
- Warn and skip package dependency comparisons when a manifest has no resolvable package name ([7fbd480](https://github.com/smoelius/whats-changed/commit/7fbd48036726f990f53ac981e781369b5316b5f6), [af56e6a](https://github.com/smoelius/whats-changed/commit/af56e6a45637a4a62e24143f0b006234cde6a3d4), and [14532b8](https://github.com/smoelius/whats-changed/commit/14532b80480b9b494e48426fdf2b268b66af369c))
- FEATURE: Report dependencies of manifests deleted since the previous revision ([ede2483](https://github.com/smoelius/whats-changed/commit/ede2483187ac0c8fa3f32f6177a2e801d55ecb7f) and [5aa484a](https://github.com/smoelius/whats-changed/commit/5aa484a5b73347e29ef109cfa92e6eed0f6ee4e5))
- FEATURE: Detect renamed Cargo.toml files instead of reporting false removals ([abf7065](https://github.com/smoelius/whats-changed/commit/abf70658828ae1a4a43922b0e872d51eb09fb5c6))
- Don't report git/path/workspace-inherited dependencies as removed ([49b8e70](https://github.com/smoelius/whats-changed/commit/49b8e70edc04411a12d19891a029ea3d8329b763))
- Don't abort the entire scan on a failed git show for a removed manifest ([c2a1349](https://github.com/smoelius/whats-changed/commit/c2a13499b1f5510ff45364c9a6ccfd69de175e74))
- Update the README to document deleted and renamed manifests and dependency filtering ([30d4e3a](https://github.com/smoelius/whats-changed/commit/30d4e3a1044ec78fd7cfd6d01cd9a74b8c3efdab))
- Treat an unstaged deletion of a Cargo.toml as removed, not a crash ([28caebc](https://github.com/smoelius/whats-changed/commit/28caebcd1f5e7170519f7cbbc8c7bb2f6d6ee00e))
- Dependency updates
  - `elaborate` upgraded to version 2

## 0.3.0

- FEATURE: Dependency updates in unpublished packages are ignored ([8dfdc17](https://github.com/smoelius/whats-changed/commit/8dfdc173a7790b6c54f13a19fe5078ee1a35d4e8))
- Dependency updates
  - `elaborate` upgraded to version 1

## 0.2.0

- Eliminate double indentation preceding "removed" messages ([c06cf35](https://github.com/smoelius/whats-changed/commit/c06cf35d421844e31a599a1e6d70feb2850844b9))
- Correct typo in main.rs ([f3b6ca3](https://github.com/smoelius/whats-changed/commit/f3b6ca3d056b1b2cd445e82eec05141feb885ac3))
- Document intentional design decisions ([a78a6c2](https://github.com/smoelius/whats-changed/commit/a78a6c2e14db1e5eb6a92a952546a1853e10aa04))
- Fix a bug that caused packages to be incorrectly reported as removed ([c4998b3](https://github.com/smoelius/whats-changed/commit/c4998b378c6183ae3f4eb484635d0e33750dd936))
- FEATURE: If no previous revision is specified, use most recent tag ([576b4c9](https://github.com/smoelius/whats-changed/commit/576b4c96262f213b53ba279b9a3c930bc36ee0cc))
- Dependency updates
  - `tempfile` removed
  - `toml` upgraded to version 1.0
  - `walkdir` removed

## 0.1.0

- Initial release
