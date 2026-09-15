# rs-infra-verify

[![Rust CI](https://github.com/qubit-ltd/rs-infra-verify/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-infra-verify/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-infra-verify/coverage-badge.json)](https://qubit-ltd.github.io/rs-infra-verify/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-infra-verify.svg?color=blue)](https://crates.io/crates/qubit-infra-verify)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Provide focused, reusable Cargo verification commands for Rust projects.

## Installation

```bash
cargo install --git https://github.com/qubit-ltd/rs-infra-verify.git --tag v0.1.0 qubit-infra-verify
```

## Quick Start

From a Rust project root:

```bash
cargo run --manifest-path /path/to/rs-infra-verify/Cargo.toml -- --help
```

The project's `.infra` configuration remains the source of truth; this tool does not copy project configuration into the tool repository.

## Verification suites

Run one suite with `run --suite <name>`, or use `run --suite all` to execute every suite. Available suites are lock, build, test, doc, package, readme, clippy, feature-matrix, cross, platform, miri, address-sanitizer, loom, fuzz, and audit.

The `package` suite runs `cargo package --package <name> --allow-dirty`
for each publishable workspace member, including Cargo's package build verification.
Members with `publish = false` or `publish = []` are skipped. Local path dependencies
receive package-specific crates.io patches so unpublished sibling versions can be
verified locally. This checks the package with those local dependencies; it does
not prove that the sibling versions have been published to a registry.

The `readme` suite checks each member's declared README (or `README.md`) and
`README.zh_CN.md`. In these files, dependency declarations for any workspace
package must use its exact current major.minor version: package version `1.2.3`
requires `demo = "1.2"` or `demo = { version = "1.2" }`. Patch versions, ranges,
and declarations without a version fail with file and line diagnostics. Unrelated
dependencies are ignored; missing files or no matching declarations are reported
as skips. Inherited workspace versions and custom README paths are supported.

```bash
rs-infra-verify --project /path/to/project run --suite package
rs-infra-verify --project /path/to/project run --suite readme
```

Optional suites use the legacy rs-ci project inputs: `.rs-ci-cargo-matrix.json` enables feature-matrix; `Cross.toml` or `.rs-ci-cross.toml` enables cross; `.rs-ci-platform.toml` enables platform; package metadata enables miri and AddressSanitizer; a direct Loom dependency enables loom; and `fuzz/Cargo.toml` must contain `cargo-fuzz = true` to enable fuzz.

An unconfigured optional suite prints an explicit `skipped: not configured` message. Once configured, missing executables or failed commands are errors; configuration is never silently ignored.

An opted-in package can set `miri-test-args` in `[package.metadata.rs-infra]` to pass Cargo test-target or name filters to Miri. This keeps expensive checks focused on selected safety-sensitive tests.

### Nightly, sanitizer, fuzz, and Loom checks

`RS_INFRA_NIGHTLY_TOOLCHAIN` selects the toolchain for Miri, AddressSanitizer,
and fuzz (default: `nightly`). Set it to `nightly-2026-06-05` to reproduce the
legacy pinned toolchain. Install that toolchain and its `miri`/`rust-src`
components, and install the required cargo-fuzz version in your workflow.

AddressSanitizer selects only workspace packages declaring
`sanitizers = ["address"]` in `[package.metadata.rs-infra]` or the legacy
`[package.metadata.rs-ci]` namespace; modern metadata takes precedence.
Empty lists opt out; malformed, duplicate, or unsupported entries fail.
Each selected package runs with `-Zbuild-std`, all features, and its native target:
Linux x86_64, macOS x86_64, or macOS aarch64. Unsupported hosts explicitly skip.
The tool appends `-Zsanitizer=address` to both `RUSTFLAGS` and `RUSTDOCFLAGS`,
retaining existing flags, and propagates package failures.

Fuzz discovers targets with `cargo +<toolchain> fuzz list`, then builds and
runs each target with `cargo +<toolchain> fuzz run`. Configure positive integer
limits with `RS_INFRA_FUZZ_SECONDS_PER_TARGET` (default `10`) and
`RS_INFRA_FUZZ_MAX_LEN` (default `4096`). For projects previously using larger
inputs, set `RS_INFRA_FUZZ_MAX_LEN=16384`. Each run receives libFuzzer's
`-max_total_time`, `-max_len`, and a separate `-artifact_prefix` pointing to
`fuzz/artifacts/<target>/`. Crash files are retained on failure; the workflow
must upload that directory. Cargo-fuzz may also update its corpus and build
output. Empty target lists, discovery errors, invalid limits, and target failures
are errors, rather than successful discovery-only checks.

Loom selects workspace members whose Cargo metadata declares a direct `loom`
dependency (including dev, optional, or renamed dependencies). It sets
`RUSTFLAGS=--cfg loom`, lists tests with `--release --all-features loom -- --list`,
and fails if any selected package has zero models. It then executes each
package's matching tests in release mode with all features. A workspace without
Loom dependencies explicitly skips; comments or transitive dependencies do not
opt in a package. Cargo's configured project toolchain is used for Loom.

```bash
RS_INFRA_NIGHTLY_TOOLCHAIN=nightly-2026-06-05 rs-infra-verify run --suite miri
RS_INFRA_NIGHTLY_TOOLCHAIN=nightly-2026-06-05 rs-infra-verify run --suite address-sanitizer
RS_INFRA_NIGHTLY_TOOLCHAIN=nightly-2026-06-05 RS_INFRA_FUZZ_MAX_LEN=16384 rs-infra-verify run --suite fuzz
rs-infra-verify run --suite loom
```

## Learn More

See the command help and source tests for the supported interface. Switch to [中文文档](README.zh_CN.md).

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./ci-check.sh

# Check code coverage
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public API documentation and tests current, and run `./align-ci.sh` to format code and `./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-infra-verify](https://github.com/qubit-ltd/rs-infra-verify)
