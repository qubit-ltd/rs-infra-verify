// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Discovers and executes release-mode Loom models per workspace package.

use std::path::Path;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;

use crate::metadata::workspace_metadata;

/// Finds workspace members with a direct dependency named `loom` in Cargo
/// metadata.
///
/// # Parameters
///
/// * `project` - Workspace root used for Cargo metadata discovery.
///
/// # Returns
///
/// Selected member names, including dev/optional/renamed Loom dependencies.
/// Transitive dependencies and manifest comments do not opt in a package.
///
/// # Errors
///
/// Returns contextual Cargo subprocess or malformed metadata errors.
pub(crate) fn packages(project: &Path) -> Result<Vec<String>> {
    let metadata = workspace_metadata(project)?;
    let members = metadata["workspace_members"]
        .as_array()
        .context("missing workspace members")?;
    let all_packages = metadata["packages"].as_array().context("missing packages")?;
    let mut selected = Vec::new();
    for package in all_packages {
        if !members.contains(&package["id"]) {
            continue;
        }
        let dependencies = package["dependencies"].as_array().context("missing dependencies")?;
        if dependencies
            .iter()
            .any(|dependency| dependency["name"].as_str() == Some("loom"))
        {
            selected.push(package["name"].as_str().context("missing package name")?.to_owned());
        }
    }
    Ok(selected)
}

/// Lists and executes each selected package's release-mode tests matching
/// `loom`.
///
/// # Parameters
///
/// * `project` - Workspace root whose models are discovered and executed.
///
/// # Returns
///
/// Success when all opted-in packages have at least one discovered model and
/// every model passes. An unconfigured workspace is handled by the suite
/// planner.
///
/// # Errors
///
/// Returns discovery, zero-model, or execution errors naming the package.
/// Cargo builds release artifacts; both invocations use `RUSTFLAGS=--cfg loom`.
pub(crate) fn verify(project: &Path) -> Result<()> {
    for name in packages(project)? {
        let mut discovery = cargo(project, &name);
        let output = discovery
            .args(["loom", "--", "--list"])
            .output()
            .with_context(|| format!("failed to discover Loom models for {name}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            bail!("Loom model discovery failed for {name}: {stderr}");
        }
        let count = stdout
            .lines()
            .filter(|line| line.trim_end().ends_with(": test"))
            .count();
        if count == 0 {
            bail!("no Loom model tests were discovered for {name}; model test names must contain 'loom'");
        }
        print!("{stdout}");
        println!("Running {count} Loom model test(s) for {name}");
        let status = cargo(project, &name)
            .args(["--verbose", "loom"])
            .status()
            .with_context(|| format!("failed to run Loom models for {name}"))?;
        if !status.success() {
            bail!("Loom model checks failed for package {name}");
        }
    }
    println!("Loom model checks passed.");
    Ok(())
}

/// Constructs the common Cargo command for one package without executing it.
///
/// # Parameters
///
/// * `project` - Working directory for Cargo.
/// * `name` - Workspace package selected by metadata.
///
/// # Returns
///
/// A release/all-features test command with the legacy Loom configuration flag.
fn cargo(project: &Path, name: &str) -> Command {
    let mut command = Command::new("cargo");
    command.current_dir(project).env("RUSTFLAGS", "--cfg loom").args([
        "test",
        "--locked",
        "--package",
        name,
        "--release",
        "--all-features",
    ]);
    command
}
