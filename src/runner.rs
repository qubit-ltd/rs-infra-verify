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
