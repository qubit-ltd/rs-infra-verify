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

Run one suite with `run --suite <name>`, or use `run --suite all` to execute every suite. Available suites are lock, build, test, doc, package, clippy, feature-matrix, cross, platform, miri, address-sanitizer, loom, fuzz, and audit.

Optional suites use the legacy rs-ci project inputs: `.rs-ci-cargo-matrix.json` enables feature-matrix; `Cross.toml` or `.rs-ci-cross.toml` enables cross; `.rs-ci-platform.toml` enables platform; package metadata enables miri, AddressSanitizer, and loom; and `fuzz/Cargo.toml` must contain `cargo-fuzz = true` to enable fuzz.

An unconfigured optional suite prints an explicit `skipped: not configured` message. Once configured, missing executables or failed commands are errors; configuration is never silently ignored.

## Learn More

See the command help and source tests for the supported interface. Switch to [中文文档](README.zh_CN.md).

## Testing

```bash
cargo test
cargo test --all-features
./ci-check.sh
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public API documentation and tests current, and run `./align-ci.sh` to format code and `./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-infra-verify](https://github.com/qubit-ltd/rs-infra-verify)
