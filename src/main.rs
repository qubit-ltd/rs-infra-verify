// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Command-line interface for `rs-infra-verify`.

use std::path::PathBuf;

use anyhow::Result;
use anyhow::anyhow;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use qubit_infra_verify::Suite;
use qubit_infra_verify::lock_check;
use qubit_infra_verify::lock_sync;
use qubit_infra_verify::run_suite;

/// Command-line options for the verification tool.
#[derive(Debug, Parser)]
#[command(name = "rs-infra-verify")]
struct Cli {
    /// Project directory to verify.
    #[arg(long, default_value = ".")]
    project: PathBuf,
    /// Verification command to execute.
    #[command(subcommand)]
    command: Command,
}

/// Top-level verification commands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Check or regenerate the lockfile.
    Lock {
        /// Lockfile operation.
        #[command(subcommand)]
        command: LockCommand,
    },
    /// Run one verification suite or all suites.
    Run {
        /// Suite to execute.
        #[arg(long, value_enum)]
        suite: SuiteArg,
    },
}

/// Lockfile operations.
#[derive(Debug, Subcommand)]
enum LockCommand {
    /// Check that the lockfile is current.
    Check,
    /// Regenerate and check the lockfile.
    Sync,
}

/// Command-line names for supported verification suites.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum SuiteArg {
    /// Run every configured suite.
    All,
    /// Check the lockfile.
    Lock,
    /// Build all targets.
    Build,
    /// Run all tests.
    Test,
    /// Build documentation.
    Doc,
    /// Build and verify each publishable workspace package.
    Package,
    /// Check workspace README dependency versions.
    Readme,
    /// Run Clippy.
    Clippy,
    /// Check the feature matrix.
    FeatureMatrix,
    /// Run cross-platform tests.
    Cross,
    /// Run platform tests.
    Platform,
    /// Run Miri.
    Miri,
    /// Run AddressSanitizer.
    AddressSanitizer,
    /// Run Loom.
    Loom,
    /// Build and smoke-test every fuzz target.
    Fuzz,
    /// Run cargo-audit.
    Audit,
}

/// Parses command-line arguments and runs the selected operation.
///
/// # Returns
///
/// Returns successfully when the selected operation completes.
///
/// # Errors
///
/// Returns an error when the project path cannot be canonicalized or the
/// selected library operation fails.
fn main() -> Result<()> {
    let cli = Cli::parse();
    let operation = operation_name(&cli.command);
    execute(cli)
        .map(|()| {
            println!("{operation} completed successfully.");
        })
        .map_err(|error| anyhow!("{operation} failed: {error:#}"))
}

/// Executes a parsed command without adding user-facing completion messages.
fn execute(cli: Cli) -> Result<()> {
    let project = std::fs::canonicalize(cli.project)?;
    match cli.command {
        Command::Lock {
            command: LockCommand::Check,
        } => lock_check(&project),
        Command::Lock {
            command: LockCommand::Sync,
        } => lock_sync(&project),
        Command::Run { suite } => {
            if matches!(suite, SuiteArg::All) {
                for suite in Suite::all() {
                    run_suite(&project, *suite)?;
                }
                Ok(())
            } else {
                run_suite(&project, suite.into())
            }
        }
    }
}

/// Returns the operation label used in the final status message.
fn operation_name(command: &Command) -> String {
    match command {
        Command::Lock {
            command: LockCommand::Check,
        } => "lock check".to_owned(),
        Command::Lock {
            command: LockCommand::Sync,
        } => "lock sync".to_owned(),
        Command::Run { suite } => format!("run --suite {}", suite_name(*suite)),
    }
}

/// Returns the command-line spelling of a suite.
fn suite_name(suite: SuiteArg) -> &'static str {
    match suite {
        SuiteArg::All => "all",
        SuiteArg::Lock => "lock",
        SuiteArg::Build => "build",
        SuiteArg::Test => "test",
        SuiteArg::Doc => "doc",
        SuiteArg::Package => "package",
        SuiteArg::Readme => "readme",
        SuiteArg::Clippy => "clippy",
        SuiteArg::FeatureMatrix => "feature-matrix",
        SuiteArg::Cross => "cross",
        SuiteArg::Platform => "platform",
        SuiteArg::Miri => "miri",
        SuiteArg::AddressSanitizer => "address-sanitizer",
        SuiteArg::Loom => "loom",
        SuiteArg::Fuzz => "fuzz",
        SuiteArg::Audit => "audit",
    }
}

/// Converts a command-line suite name into the library suite type.
impl From<SuiteArg> for Suite {
    /// Converts a command-line suite into its library representation.
    ///
    /// # Returns
    ///
    /// The library suite corresponding to the command-line value.
    fn from(suite: SuiteArg) -> Self {
        match suite {
            SuiteArg::All => unreachable!("all is handled before conversion"),
            SuiteArg::Lock => Self::Lock,
            SuiteArg::Build => Self::Build,
            SuiteArg::Test => Self::Test,
            SuiteArg::Doc => Self::Doc,
            SuiteArg::Package => Self::Package,
            SuiteArg::Readme => Self::Readme,
            SuiteArg::Clippy => Self::Clippy,
            SuiteArg::FeatureMatrix => Self::FeatureMatrix,
            SuiteArg::Cross => Self::Cross,
            SuiteArg::Platform => Self::Platform,
            SuiteArg::Miri => Self::Miri,
            SuiteArg::AddressSanitizer => Self::AddressSanitizer,
            SuiteArg::Loom => Self::Loom,
            SuiteArg::Fuzz => Self::Fuzz,
            SuiteArg::Audit => Self::Audit,
        }
    }
}
