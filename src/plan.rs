// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Verification planning and project configuration detection.

use std::path::Path;

use anyhow::Context;
use anyhow::Result;

use crate::Suite;
use crate::plan_entry::PlanEntry;
use crate::plan_status::PlanStatus;

/// Plans one selected suite or every supported suite for a project.
///
/// # Parameters
///
/// * `project` - Project directory whose Cargo manifest and optional
///   configuration files are inspected.
/// * `selected` - One suite to plan, or `None` to plan every suite.
///
/// # Returns
///
/// A plan entry for each requested suite.
///
/// # Errors
///
/// Returns an error when the project's `Cargo.toml` cannot be read. Optional
/// suites are represented as [`PlanStatus::Skipped`] when their configuration
/// is absent.
pub fn plan(project: &Path, selected: Option<Suite>) -> Result<Vec<PlanEntry>> {
    let suites = selected.map_or_else(|| Suite::all().to_vec(), |suite| vec![suite]);
    suites
        .into_iter()
        .map(|suite| {
            let status = if is_configured(project, suite)? {
                PlanStatus::Ready
            } else {
                PlanStatus::Skipped(format!("not configured in {}", project.display()))
            };
            Ok(PlanEntry {
                suite,
                command: suite
                    .command()
                    .iter()
                    .map(|arg| (*arg).to_owned())
                    .collect(),
                status,
            })
        })
        .collect()
}

/// Determines whether the project has the configuration required by a suite.
///
/// # Parameters
///
/// * `project` - Project directory to inspect.
/// * `suite` - Suite whose configuration is checked.
///
/// # Returns
///
/// `true` for configured suites and `false` for unavailable optional suites.
///
/// # Errors
///
/// Returns an error when the project's `Cargo.toml` cannot be read.
fn is_configured(project: &Path, suite: Suite) -> Result<bool> {
    let manifest = project.join("Cargo.toml");
    let manifest_text = std::fs::read_to_string(&manifest)
        .with_context(|| format!("failed to read {}", manifest.display()))?;
    Ok(match suite {
        Suite::Lock
        | Suite::Build
        | Suite::Test
        | Suite::Doc
        | Suite::Package
        | Suite::Clippy
        | Suite::Audit => true,
        Suite::FeatureMatrix => {
            project.join(".infra/ci/cargo-matrix.json").is_file()
                || project.join(".rs-ci-cargo-matrix.json").is_file()
        }
        Suite::Cross => {
            project.join(".infra/ci/cross.toml").is_file()
                || project.join("Cross.toml").is_file()
                || project.join(".rs-ci-cross.toml").is_file()
        }
        Suite::Platform => {
            project.join(".infra/ci/platform.toml").is_file()
                || project.join(".rs-ci-platform.toml").is_file()
        }
        Suite::Miri => has_metadata_flag(&manifest_text, "miri", "true"),
        Suite::AddressSanitizer => {
            manifest_text.contains("sanitizers") && manifest_text.contains("address")
        }
        Suite::Loom => manifest_text.contains("loom"),
        Suite::Fuzz => {
            project.join("fuzz/Cargo.toml").is_file()
                && std::fs::read_to_string(project.join("fuzz/Cargo.toml"))
                    .map(|text| text.contains("cargo-fuzz") && text.contains("true"))
                    .unwrap_or(false)
        }
    })
}

/// Checks a manifest line for a metadata key/value pair.
///
/// # Parameters
///
/// * `manifest` - Manifest text to inspect.
/// * `key` - Metadata key to find.
/// * `value` - Required metadata value.
///
/// # Returns
///
/// `true` when a matching metadata assignment is present.
fn has_metadata_flag(manifest: &str, key: &str, value: &str) -> bool {
    manifest.lines().any(|line| {
        let compact: String = line
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect();
        compact == format!("{key}={value}") || compact.ends_with(&format!(".{key}={value}"))
    })
}
