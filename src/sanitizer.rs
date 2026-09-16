// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Package-scoped AddressSanitizer checks on supported host platforms.

use std::env;
use std::path::Path;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;

use crate::metadata::workspace_metadata;
use crate::nightly::toolchain;

/// Resolves workspace packages opting into AddressSanitizer.
///
/// # Parameters
///
/// * `project` - Workspace root used for the Cargo metadata subprocess.
///
/// # Returns
///
/// Names of packages declaring `sanitizers = ["address"]` under modern
/// `package.metadata.rs-infra` or legacy `package.metadata.rs-ci`
/// configuration. Modern configuration takes precedence when both namespaces
/// exist.
///
/// # Errors
///
/// Returns contextual Cargo metadata or configuration errors, including
/// non-array settings, non-string entries, duplicate or unsupported sanitizers.
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
        let name = package["name"].as_str().context("missing package name")?;
        let metadata = &package["metadata"];
        let config = metadata.get("rs-infra").or_else(|| metadata.get("rs-ci"));
        let Some(config) = config else { continue };
        let config = config
            .as_object()
            .with_context(|| format!("package {name}: sanitizer metadata must be an object"))?;
        let Some(sanitizers) = config.get("sanitizers") else {
            continue;
        };
        let sanitizers = sanitizers
            .as_array()
            .with_context(|| format!("package {name}: sanitizers must be an array"))?;
        let mut enabled = false;
        for sanitizer in sanitizers {
            let sanitizer = sanitizer
                .as_str()
                .with_context(|| format!("package {name}: sanitizers entries must be strings"))?;
            if sanitizer != "address" {
                bail!("package {name}: sanitizers contains unsupported sanitizer {sanitizer}");
            }
            if enabled {
                bail!("package {name}: sanitizers contains a duplicate entry");
            }
            enabled = true;
        }
        if enabled {
            selected.push(name.to_owned());
        }
    }
    Ok(selected)
}

/// Runs instrumented tests for each opted-in package on the host target.
///
/// # Parameters
///
/// * `project` - Cargo workspace root containing sanitizer metadata.
///
/// # Returns
///
/// Success when selected package tests pass; unsupported hosts explicitly skip.
///
/// # Errors
///
/// Returns toolchain, metadata, or subprocess errors. Cargo builds the standard
/// library and selected tests, writing build output and potentially lockfiles.
pub(crate) fn verify(project: &Path) -> Result<()> {
    let selected = packages(project)?;
    let Some(target) = host_target(env::consts::OS, env::consts::ARCH) else {
        println!(
            "AddressSanitizer is not supported on {} {}; skipping.",
            env::consts::OS,
            env::consts::ARCH
        );
        return Ok(());
    };
    let nightly = toolchain()?;
    let rustflags = instrumented_flags(&env::var("RUSTFLAGS").unwrap_or_default());
    let rustdocflags = instrumented_flags(&env::var("RUSTDOCFLAGS").unwrap_or_default());
    for name in selected {
        println!("Running AddressSanitizer for package {name} on {target}");
        let status = Command::new("cargo")
            .args([
                &nightly,
                "test",
                "--locked",
                "-Zbuild-std",
                "--target",
                target,
                "--all-features",
                "--package",
                &name,
            ])
            .env("RUSTFLAGS", &rustflags)
            .env("RUSTDOCFLAGS", &rustdocflags)
            .current_dir(project)
            .status()
            .with_context(|| format!("failed to start AddressSanitizer for package {name}"))?;
        if !status.success() {
            bail!("AddressSanitizer failed for package {name} on {target}");
        }
    }
    Ok(())
}

/// Maps runtime OS and architecture names to legacy-supported native targets.
/// Returns `None` for unsupported combinations rather than using a Linux
/// target.
fn host_target(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        _ => None,
    }
}

/// Appends address instrumentation to existing compiler or rustdoc flags.
/// Returns flags retaining the caller's options with address instrumentation
/// last.
fn instrumented_flags(current: &str) -> String {
    if current.is_empty() {
        "-Zsanitizer=address".to_owned()
    } else {
        format!("{current} -Zsanitizer=address")
    }
}

#[cfg(test)]
mod tests {
    use super::host_target;

    #[test]
    fn test_host_target_matches_supported_legacy_platforms() {
        assert_eq!(host_target("linux", "x86_64"), Some("x86_64-unknown-linux-gnu"));
        assert_eq!(host_target("macos", "aarch64"), Some("aarch64-apple-darwin"));
        assert_eq!(host_target("macos", "x86_64"), Some("x86_64-apple-darwin"));
        assert_eq!(host_target("windows", "x86_64"), None);
        assert_eq!(host_target("linux", "aarch64"), None);
    }
}
