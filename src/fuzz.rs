// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Executes cargo-fuzz build or smoke checks, or explicitly skips disabled checks.

use std::env;
use std::env::VarError;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;

use crate::fuzz_mode::FuzzMode;
use crate::nightly::toolchain;

/// Executes the configured fuzz mode for every discovered target in `project`.
///
/// # Parameters
///
/// * `project` - Project root containing the opted-in `fuzz/Cargo.toml`.
///
/// # Returns
///
/// Success when disabled, or when every target builds/runs in the selected mode.
///
/// # Errors
///
/// Returns contextual errors for invalid environment settings, discovery,
/// directory creation, or any target failure. Empty target lists are errors.
/// This runs Cargo subprocesses, updates fuzz corpora/build output, and retains
/// crash artifacts in `fuzz/artifacts/<target>/`, including on failure.
pub(crate) fn verify(project: &Path) -> Result<()> {
    let mode = FuzzMode::from_env()?;
    if mode == FuzzMode::Disabled {
        println!("fuzz: skipped: RS_INFRA_FUZZ_MODE=disabled");
        return Ok(());
    }
    let nightly = toolchain()?;
    let limits = if mode == FuzzMode::Smoke {
        Some((
            positive_setting("RS_INFRA_FUZZ_SECONDS_PER_TARGET", 10)?,
            positive_setting("RS_INFRA_FUZZ_MAX_LEN", 4096)?,
        ))
    } else {
        None
    };
    let output = Command::new("cargo")
        .args([&nightly, "fuzz", "list"])
        .current_dir(project)
        .output()
        .context("failed to start cargo fuzz list")?;
    if !output.status.success() {
        bail!(
            "cargo fuzz list failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let listed =
        String::from_utf8(output.stdout).context("cargo fuzz list returned invalid UTF-8")?;
    let targets: Vec<_> = listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if targets.is_empty() {
        bail!("cargo-fuzz is configured but reported no fuzz targets");
    }
    for target in &targets {
        if !target.starts_with(|c: char| c.is_ascii_alphanumeric())
            || !target
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        {
            bail!("cargo fuzz list returned invalid target name: {target}");
        }
    }
    let project = fs::canonicalize(project).context("cannot resolve fuzz project root")?;
    for target in &targets {
        let Some((seconds, max_len)) = limits else {
            let status = Command::new("cargo")
                .args([&nightly, "fuzz", "build", target])
                .current_dir(&project)
                .status()
                .with_context(|| format!("failed to build fuzz target {target}"))?;
            if !status.success() {
                bail!("fuzz build failed for target {target}");
            }
            continue;
        };
        let artifacts = project.join("fuzz/artifacts").join(target);
        fs::create_dir_all(&artifacts)
            .with_context(|| format!("cannot create {}", artifacts.display()))?;
        println!("Running fuzz target {target} for {seconds}s (max_len={max_len})");
        // cargo fuzz run builds the target before invoking libFuzzer.
        let status = Command::new("cargo")
            .args([&nightly, "fuzz", "run", target, "--"])
            .arg(format!("-max_total_time={seconds}"))
            .arg(format!("-max_len={max_len}"))
            .arg(format!("-artifact_prefix={}/", artifacts.display()))
            .current_dir(&project)
            .status()
            .with_context(|| format!("failed to start fuzz target {target}"))?;
        if !status.success() {
            bail!(
                "fuzz smoke failed for target {target}; artifacts retained in {}",
                artifacts.display()
            );
        }
    }
    let label = if mode == FuzzMode::BuildOnly {
        "build-only"
    } else {
        "smoke"
    };
    println!(
        "Fuzz {label} checks passed for {} target(s).",
        targets.len()
    );
    Ok(())
}

/// Reads a positive integer environment setting, using `default` when absent.
///
/// # Parameters
///
/// * `name` - Environment variable to read.
/// * `default` - Value used when the variable is unset.
///
/// # Returns
///
/// A positive 32-bit integer accepted by libFuzzer's integer flags.
///
/// # Errors
///
/// Returns an error for non-Unicode, malformed, zero, or overflowing values.
fn positive_setting(name: &str, default: u32) -> Result<u32> {
    let value = match env::var(name) {
        Ok(value) => value,
        Err(VarError::NotPresent) => return Ok(default),
        Err(error) => return Err(error).with_context(|| format!("invalid {name}")),
    };
    let parsed = value
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0 && *value <= i32::MAX as u32);
    parsed.with_context(|| {
        format!(
            "{name} must be a positive integer no greater than {}",
            i32::MAX
        )
    })
}
