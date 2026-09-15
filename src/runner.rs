// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Command execution for verification suites and lockfiles.

use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;

use crate::PlanStatus;
use crate::Suite;
use crate::metadata::miri_packages;
use crate::plan;

/// Checks that a project's lockfile is current.
///
/// # Parameters
///
/// * `project` - Project directory whose lockfile is checked.
///
/// # Errors
///
/// Returns an error when Cargo cannot start or reports a missing or stale
/// lockfile.
///
/// This function executes Cargo and prints a success message on completion.
pub fn lock_check(project: &Path) -> Result<()> {
    run_quiet(
        project,
        &["metadata", "--no-deps", "--locked", "--format-version", "1"],
    )
    .context("Cargo.lock is missing or stale; run rs-infra-verify lock sync")?;
    println!("Cargo.lock is current.");
    Ok(())
}

/// Regenerates and then checks a project's lockfile.
///
/// # Parameters
///
/// * `project` - Project directory whose lockfile is regenerated.
///
/// # Errors
///
/// Returns an error when Cargo cannot regenerate or validate the lockfile.
///
/// This function writes the project's lockfile and invokes Cargo subprocesses.
pub fn lock_sync(project: &Path) -> Result<()> {
    run(project, &["generate-lockfile"])?;
    lock_check(project)
}

/// Executes a configured verification suite for a project.
///
/// # Parameters
///
/// * `project` - Project directory in which the suite runs.
/// * `suite` - Suite to plan and execute.
///
/// # Errors
///
/// Returns an error when project configuration cannot be read, a required
/// program cannot start, or the suite exits unsuccessfully.
///
/// This function executes external verification commands in `project`.
pub fn run_suite(project: &Path, suite: Suite) -> Result<()> {
    let entry = plan(project, Some(suite))?.remove(0);
    if let PlanStatus::Skipped(_) = entry.status {
        println!("{}", entry.message());
        return Ok(());
    }
    if suite == Suite::Package {
        return crate::package::verify(project);
    }
    if suite == Suite::Readme {
        return crate::readme::verify(project);
    }
    if suite == Suite::Miri {
        return run_miri(project, &entry.command);
    }
    let (program, args): (&str, Vec<String>) = match suite {
        Suite::Cross => (entry.command[0].as_str(), entry.command[1..].to_vec()),
        Suite::AddressSanitizer => (
            "cargo",
            [vec!["+nightly".to_owned()], entry.command.clone()].concat(),
        ),
        _ => ("cargo", entry.command.clone()),
    };
    run_program(project, program, &args)
}

/// Runs each configured workspace package under Miri.
///
/// # Parameters
///
/// * `project` - Workspace root whose Miri-enabled packages are tested.
/// * `command` - Base Cargo Miri command planned for the suite.
///
/// # Errors
///
/// Returns an error when metadata is invalid, Miri cannot start, it selects no
/// tests, or a selected package fails.
fn run_miri(project: &Path, command: &[String]) -> Result<()> {
    let miriflags = miri_flags(&std::env::var("MIRIFLAGS").unwrap_or_default());
    for package in miri_packages(project)? {
        let mut arguments = command.to_vec();
        arguments.extend(["--package".to_owned(), package.name.clone()]);
        arguments.extend(package.test_args);
        let output = Command::new("cargo")
            .args(&arguments)
            .current_dir(project)
            .env("PROPTEST_DISABLE_FAILURE_PERSISTENCE", "1")
            .env("PROPTEST_CASES", "8")
            .env("MIRIFLAGS", &miriflags)
            .output()
            .with_context(|| format!("failed to start cargo {}", arguments.join(" ")))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        print!("{stdout}");
        eprint!("{stderr}");
        if !output.status.success() {
            bail!("Miri failed for package {}", package.name);
        }
        if !output_reports_tests(&format!("{stdout}\n{stderr}")) {
            bail!("Miri selected no tests for package {}", package.name);
        }
    }
    Ok(())
}

/// Adds Miri's filesystem allowance while retaining caller-provided flags.
///
/// # Returns
///
/// The existing `MIRIFLAGS` followed by `-Zmiri-disable-isolation` unless that
/// flag is already present.
fn miri_flags(current: &str) -> String {
    if current
        .split_whitespace()
        .any(|flag| flag == "-Zmiri-disable-isolation")
    {
        current.to_owned()
    } else if current.is_empty() {
        "-Zmiri-disable-isolation".to_owned()
    } else {
        format!("{current} -Zmiri-disable-isolation")
    }
}

/// Checks whether captured test output reports at least one selected test.
///
/// # Parameters
///
/// * `output` - One stream of Cargo or test-harness output.
///
/// # Returns
///
/// `true` when a test harness reports a positive test count.
fn output_reports_tests(output: &str) -> bool {
    output.lines().any(|line| {
        let Some(count) = line.strip_prefix("running ") else {
            return false;
        };
        let Some((count, unit)) = count.split_once(' ') else {
            return false;
        };
        matches!(unit, "test" | "tests") && count.parse::<u32>().is_ok_and(|count| count > 0)
    })
}

/// Runs a program in the target project and reports a non-zero exit status.
///
/// # Parameters
///
/// * `project` - Working directory for the child process.
/// * `program` - Program executable to start.
/// * `args` - Arguments passed to the program.
///
/// # Errors
///
/// Returns an error when the process cannot start or exits unsuccessfully.
fn run_program(project: &Path, program: &str, args: &[String]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .current_dir(project)
        .status()
        .with_context(|| format!("failed to start {program} {}", args.join(" ")))?;
    if !status.success() {
        bail!("{program} {} failed", args.join(" "));
    }
    Ok(())
}

/// Runs Cargo with the supplied arguments and forwards its output.
///
/// # Parameters
///
/// * `project` - Working directory for Cargo.
/// * `args` - Cargo arguments.
///
/// # Errors
///
/// Returns an error when Cargo cannot start or exits unsuccessfully.
fn run(project: &Path, args: &[&str]) -> Result<()> {
    run_program(
        project,
        "cargo",
        &args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>(),
    )
}

/// Runs Cargo while suppressing successful output.
///
/// # Parameters
///
/// * `project` - Working directory for Cargo.
/// * `args` - Cargo arguments.
///
/// # Errors
///
/// Returns an error when Cargo cannot start or exits unsuccessfully. Failure
/// output is included in the error message.
fn run_quiet(project: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("cargo")
        .args(args)
        .current_dir(project)
        .stdout(Stdio::null())
        .output()
        .with_context(|| format!("failed to start cargo {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "cargo {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::miri_flags;
    use super::output_reports_tests;

    #[test]
    fn test_miri_flags_allow_filesystem_and_preserve_existing_values() {
        assert_eq!(
            miri_flags("-Zmiri-backtrace=full"),
            "-Zmiri-backtrace=full -Zmiri-disable-isolation"
        );
        assert_eq!(
            miri_flags("-Zmiri-disable-isolation"),
            "-Zmiri-disable-isolation"
        );
    }

    #[test]
    fn test_miri_output_requires_a_positive_test_count() {
        assert!(output_reports_tests("running 1 test\ntest sample ... ok"));
        assert!(!output_reports_tests("running 0 tests\n"));
    }
}
