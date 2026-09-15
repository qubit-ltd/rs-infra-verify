// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Verification suite definitions and command mappings.

/// A verification suite supported by the command-line tool.
///
/// # Examples
///
/// ```
/// use qubit_infra_verify::Suite;
///
/// assert!(Suite::all().contains(&Suite::Test));
/// assert_eq!(Suite::Test.name(), "test");
/// ```
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Suite {
    /// Checks that the project's lockfile is current.
    Lock,
    /// Builds all workspace targets with all features.
    Build,
    /// Runs all workspace tests and targets with all features.
    Test,
    /// Builds documentation for all workspace packages.
    Doc,
    /// Builds and verifies each publishable workspace package.
    Package,
    /// Checks README dependency versions against workspace package versions.
    Readme,
    /// Runs Clippy for all workspace targets and features.
    Clippy,
    /// Checks the configured feature matrix.
    FeatureMatrix,
    /// Runs cross-platform tests through `cross`.
    Cross,
    /// Runs the configured platform test command.
    Platform,
    /// Runs tests under Miri.
    Miri,
    /// Runs tests with AddressSanitizer.
    AddressSanitizer,
    /// Runs tests with the Loom concurrency model checker.
    Loom,
    /// Lists configured fuzz targets.
    Fuzz,
    /// Runs cargo-audit.
    Audit,
}

impl Suite {
    /// Returns all supported suites in command-line display order.
    ///
    /// # Returns
    ///
    /// A static slice containing every supported suite.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Lock,
            Self::Build,
            Self::Test,
            Self::Doc,
            Self::Package,
            Self::Readme,
            Self::Clippy,
            Self::FeatureMatrix,
            Self::Cross,
            Self::Platform,
            Self::Miri,
            Self::AddressSanitizer,
            Self::Loom,
            Self::Fuzz,
            Self::Audit,
        ]
    }

    /// Returns the Cargo or external-program arguments for this suite.
    ///
    /// # Returns
    ///
    /// The base program arguments used to execute this suite. Package arguments
    /// are expanded per publishable member at runtime. README checks run in Rust
    /// and return an empty argument list. Otherwise the first argument is the
    /// Cargo subcommand unless the suite uses an external program.
    #[must_use]
    pub const fn command(self) -> &'static [&'static str] {
        match self {
            Self::Lock => &["metadata", "--no-deps", "--locked", "--format-version", "1"],
            Self::Build => &[
                "build",
                "--locked",
                "--workspace",
                "--all-targets",
                "--all-features",
            ],
            Self::Test => &[
                "test",
                "--locked",
                "--workspace",
                "--all-targets",
                "--all-features",
            ],
            Self::Doc => &[
                "doc",
                "--locked",
                "--workspace",
                "--all-features",
                "--no-deps",
            ],
            Self::Package => &["package", "--allow-dirty"],
            Self::Readme => &[],
            Self::Clippy => &[
                "clippy",
                "--locked",
                "--workspace",
                "--all-targets",
                "--all-features",
            ],
            Self::FeatureMatrix => &["check", "--locked", "--workspace", "--all-features"],
            Self::Cross => &["cross", "test", "--locked", "--workspace", "--all-features"],
            Self::Platform => &["test", "--locked", "--workspace", "--all-features"],
            Self::Miri => &["+nightly", "miri", "test", "--locked", "--all-features"],
            Self::AddressSanitizer => &[
                "test",
                "--locked",
                "-Zbuild-std",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--workspace",
                "--all-features",
            ],
            Self::Loom => &["test", "--locked", "--workspace", "--all-features", "loom"],
            Self::Fuzz => &["fuzz", "list"],
            Self::Audit => &["audit"],
        }
    }

    /// Returns the stable command-line name of this suite.
    ///
    /// # Returns
    ///
    /// The lowercase name accepted by the command-line interface.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Lock => "lock",
            Self::Build => "build",
            Self::Test => "test",
            Self::Doc => "doc",
            Self::Package => "package",
            Self::Readme => "readme",
            Self::Clippy => "clippy",
            Self::FeatureMatrix => "feature-matrix",
            Self::Cross => "cross",
            Self::Platform => "platform",
            Self::Miri => "miri",
            Self::AddressSanitizer => "address-sanitizer",
            Self::Loom => "loom",
            Self::Fuzz => "fuzz",
            Self::Audit => "audit",
        }
    }
}
