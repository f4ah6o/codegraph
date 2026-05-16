# Changelog

All notable changes to CodeGraph are documented here. Each entry also ships as
a [GitHub Release](https://github.com/f4ah6o/codegraph/releases) tagged
`vX.Y.Z`, which is where most people will look.

This project follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2026.5.3] - 2026-05-16

### Changed
- Reissue the `cgz` release from the merged `origin/main` branch so the
  published crate and release tag are based on the repository's release branch.

## [2026.5.2] - 2026-05-16

### Added
- Agent workflow guidance now explains how to use `cgz` safely during
  read-only exploration, when initialization should be treated as an explicit
  workspace-changing step, and why final validation still belongs to the target
  repository's normal checks.

### Changed
- `cgz context` now returns useful matches for natural-language task queries by
  extracting code-like terms such as identifiers and file names before falling
  back to retry guidance.
- `cgz affected --json` now includes per-file debug details explaining how test
  candidates were selected.

### Fixed
- MoonBit source changes now conservatively report same-package test files such
  as `*_test.mbt`, `*_wbtest.mbt`, and `*.mbt.md` when import-dependent graph
  edges are not enough.

## [2026.5.1] - 2026-05-16

### Changed
- The root README now documents the Rust `cgz` crate, its Cargo commands, and
  the currently implemented CLI surface.
- The original upstream README is preserved as `README.org.md`.

## [0.7.6] - 2026-05-13

### Fixed
- `codegraph` CLI failing with `zsh: permission denied: codegraph` after a fresh
  global install. The published 0.7.5 tarball shipped `dist/bin/codegraph.js`
  without the executable bit, so the shell refused to run it through the npm
  symlink. The build now `chmod +x`'s the binary before packing.

  Already on 0.7.5? Either upgrade to 0.7.6, or unblock yourself in place:
  ```bash
  chmod +x "$(npm root -g)/@colbymchenry/codegraph/dist/bin/codegraph.js"
  ```

[2026.5.3]: https://github.com/f4ah6o/codegraph/releases/tag/v2026.5.3
[2026.5.2]: https://github.com/f4ah6o/codegraph/releases/tag/v2026.5.2
[2026.5.1]: https://github.com/f4ah6o/codegraph/releases/tag/v2026.5.1
[0.7.6]: https://github.com/f4ah6o/codegraph/releases/tag/v0.7.6
