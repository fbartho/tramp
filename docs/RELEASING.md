# Releasing Tramp

This document describes the release process for tramp.

## Versioning

Tramp follows [Semantic Versioning](https://semver.org/):

- **MAJOR** version for incompatible API/CLI changes
- **MINOR** version for new functionality in a backward compatible manner
- **PATCH** version for backward compatible bug fixes

### Version Locations

The version is defined in `Cargo.toml`:

```toml
[package]
version = "0.1.0"
```

## Release Process

### 1. Update Version

Update the version in `Cargo.toml`:

```bash
# Edit Cargo.toml and change version = "X.Y.Z"
```

### 2. Commit and Tag

```bash
git add Cargo.toml
git commit -m "Release vX.Y.Z"
git tag -a vX.Y.Z -m "Release vX.Y.Z"
```

### 3. Push

```bash
git push origin main
git push origin vX.Y.Z
```

### 4. Automated Release

When a tag matching `v*` is pushed, GitHub Actions will automatically:

1. **Build binaries** for all supported platforms:
   - `x86_64-unknown-linux-gnu` (Linux x86_64)
   - `x86_64-apple-darwin` (macOS Intel)
   - `aarch64-apple-darwin` (macOS Apple Silicon)
   - `x86_64-pc-windows-msvc` (Windows x86_64)

2. **Create a GitHub Release** with:
   - All platform binaries attached
   - Auto-generated release notes from commits

3. **Publish to crates.io** (requires `CARGO_REGISTRY_TOKEN` secret)

## Pre-release Checklist

Before creating a release:

- [ ] All tests pass: `cargo test`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Code is formatted: `cargo fmt --check`
- [ ] README is up to date
- [ ] CHANGELOG updated (if maintained)

## Required Secrets

The following GitHub repository secrets are required:

- `CARGO_REGISTRY_TOKEN`: API token for publishing to crates.io
  - Generate at: https://crates.io/settings/tokens

## Manual Release (if needed)

To publish manually without the workflow:

```bash
# Build release binaries
cargo build --release

# Publish to crates.io
cargo publish
```

## Platform Support

| Platform | Target | Notes |
|----------|--------|-------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Primary Linux target |
| macOS Intel | `x86_64-apple-darwin` | Intel Macs |
| macOS ARM | `aarch64-apple-darwin` | Apple Silicon (M1/M2/M3) |
| Windows x86_64 | `x86_64-pc-windows-msvc` | Windows with MSVC toolchain |
