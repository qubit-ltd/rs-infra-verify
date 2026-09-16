// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Builds publishable Cargo packages, retaining local dependency patches.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use serde_json::to_string;

use crate::metadata::workspace_metadata;

/// Builds and verifies each publishable member using Cargo.
///
/// # Parameters
///
/// * `project` - Cargo workspace directory.
///
/// # Returns
///
/// Success when every publishable member builds, or no members are publishable.
///
/// # Errors
///
/// Returns contextual metadata, process, or package verification errors.
/// Cargo writes package archives, build output, and potentially lockfiles.
pub(crate) fn verify(project: &Path) -> Result<()> {
    let metadata = workspace_metadata(project)?;
    let members = metadata["workspace_members"]
        .as_array()
        .context("missing workspace members")?;
    let packages = metadata["packages"].as_array().context("missing packages")?;
    let mut checked = 0;
    for package in packages {
        if !members.contains(&package["id"]) || package["publish"].as_array().is_some_and(Vec::is_empty) {
            continue;
        }
        let name = package["name"].as_str().context("missing package name")?;
        let mut patches = BTreeMap::new();
        for dependency in package["dependencies"].as_array().context("missing dependencies")? {
            if let Some(path) = dependency["path"].as_str()
                && Path::new(path).is_dir()
            {
                let name = dependency["name"].as_str().context("missing dependency name")?;
                patches.entry(name).or_insert(path);
            }
        }
        let mut command = Command::new("cargo");
        command.current_dir(project);
        for (name, path) in patches {
            // JSON string quoting is also valid TOML basic-string quoting here.
            command.args([
                "--config",
                &format!("patch.crates-io.{}.path={}", to_string(name)?, to_string(path)?),
            ]);
        }
        let status = command
            .args(["package", "--package", name, "--allow-dirty"])
            .status()
            .with_context(|| format!("failed to start Cargo package for {name}"))?;
        if !status.success() {
            bail!("Cargo package verification failed for package {name}");
        }
        checked += 1;
    }
    if checked == 0 {
        println!("No publishable workspace packages found; skipping Cargo package verification.");
    } else {
        println!("Cargo package verification passed for {checked} workspace package(s).");
    }
    Ok(())
}
