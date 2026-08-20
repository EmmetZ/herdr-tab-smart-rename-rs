# Changelog

## [0.1.1] - 2026-08-20

### Added

- Add the `configure-ai` plugin action to initialize and open the private
  `provider.env` file.
- Configure API key, base URL, model, timeout, and reasoning effort in
  `provider.env`.
- Support `low`, `medium`, and `high` reasoning effort values for compatible
  providers.

### Changed

- Optimize release binaries for size with LTO, a single codegen unit, symbol
  stripping, and abort-on-panic.

### Security

- Create and maintain private provider configuration permissions (`0700` for
  its directory and `0600` for the file).
- Reject non-regular and symbolic-link provider configuration paths.

## [0.1.0] - 2026-08-20

### Added

- Rust implementation of the Herdr smart tab rename plugin.
- GitHub Actions release workflow for Linux and macOS prebuilt binaries.
- Herdr install script that downloads and verifies release binaries.
