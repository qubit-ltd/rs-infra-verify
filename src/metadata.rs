// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Cargo metadata used to configure package-scoped verification suites.

use std::path::Path;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use serde_json::Value;

/// Configuration for running one Cargo package under Miri.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct MiriPackage {
    /// Cargo package name passed to `--package`.
    pub(crate) name: String,
    /// Optional Cargo test target and name filters.
    pub(crate) test_args: Vec<String>,
}

/// Loads Miri-enabled packages from the project's Cargo workspace metadata.
///
/// # Parameters
///
/// * `project` - Workspace root whose Cargo metadata is queried.
///
/// # Returns
///
/// Miri-enabled workspace packages and their optional test-selection arguments.
///
/// # Errors
///
/// Returns an error when Cargo metadata cannot be loaded or a package's Miri
/// configuration has an invalid type.
pub(crate) fn miri_packages(project: &Path) -> Result<Vec<MiriPackage>> {
    let metadata = workspace_metadata(project)?;
    miri_packages_from_metadata(&metadata)
}

/// Extracts Miri-enabled workspace packages from Cargo metadata.
///
/// # Parameters
///
/// * `metadata` - JSON value returned by `cargo metadata`.
///
/// # Returns
///
/// Miri-enabled workspace packages and their optional test-selection arguments.
///
/// # Errors
///
/// Returns an error when workspace package metadata or Miri configuration is
/// malformed.
fn miri_packages_from_metadata(metadata: &Value) -> Result<Vec<MiriPackage>> {
    let workspace_members = metadata
        .get("workspace_members")
        .and_then(Value::as_array)
        .context("cargo metadata has no workspace_members array")?;
    let packages = metadata
        .get("packages")
        .and_then(Value::as_array)
        .context("cargo metadata has no packages array")?;
    let mut miri_packages = Vec::new();
    for package in packages {
        let package_id = package
            .get("id")
            .and_then(Value::as_str)
            .context("cargo metadata package has no string id")?;
        if !workspace_members
            .iter()
            .any(|member| member.as_str() == Some(package_id))
        {
            continue;
        }
        let package_metadata = package.get("metadata").and_then(Value::as_object);
        let infra_metadata = package_metadata
            .and_then(|metadata| metadata.get("rs-infra"))
            .or_else(|| package_metadata.and_then(|metadata| metadata.get("rs-ci")));
        let Some(infra_metadata) = infra_metadata else {
            continue;
        };
        let Some(miri) = infra_metadata.get("miri") else {
            continue;
        };
        match miri.as_bool() {
            Some(true) => {}
            Some(false) => continue,
            None => bail!("package {package_id}: miri must be a boolean"),
        }
        let name = package
            .get("name")
            .and_then(Value::as_str)
            .context("Miri-enabled Cargo package has no string name")?
            .to_owned();
        let test_args = parse_miri_test_args(infra_metadata, package_id)?;
        miri_packages.push(MiriPackage { name, test_args });
    }
    Ok(miri_packages)
}

/// Parses one package's optional Miri test-selection arguments.
///
/// # Parameters
///
/// * `metadata` - Package metadata containing the optional setting.
/// * `package_id` - Cargo package identifier used in configuration errors.
///
/// # Returns
///
/// The configured string arguments, or an empty vector when absent.
///
/// # Errors
///
/// Returns an error when `miri-test-args` is not an array of strings.
fn parse_miri_test_args(metadata: &Value, package_id: &str) -> Result<Vec<String>> {
    let Some(arguments) = metadata.get("miri-test-args") else {
        return Ok(Vec::new());
    };
    arguments
        .as_array()
        .with_context(|| format!("package {package_id}: miri-test-args must be an array"))?
        .iter()
        .map(|argument| {
            argument.as_str().map(str::to_owned).with_context(|| {
                format!("package {package_id}: miri-test-args entries must be strings")
            })
        })
        .collect()
}

/// Queries Cargo workspace metadata.
///
/// # Parameters
///
/// * `project` - Working directory for the Cargo metadata subprocess.
///
/// # Returns
///
/// Cargo's JSON metadata including workspace members and their dependencies.
///
/// # Errors
///
/// Returns a contextual process or JSON error if Cargo metadata cannot load.
/// This runs a blocking subprocess and may update Cargo metadata caches.
pub(crate) fn workspace_metadata(project: &Path) -> Result<Value> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(project)
        .output()
        .context("failed to start cargo metadata")?;
    if !output.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata: Value =
        serde_json::from_slice(&output.stdout).context("cargo metadata returned invalid JSON")?;
    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::miri_packages_from_metadata;

    #[test]
    fn test_miri_packages_preserve_each_package_test_arguments() {
        let metadata = json!({
            "workspace_members": ["demo 0.1.0 (path+file:///workspace/demo)"],
            "packages": [
                {
                    "id": "demo 0.1.0 (path+file:///workspace/demo)",
                    "name": "demo",
                    "metadata": {
                        "rs-infra": {
                            "miri": true,
                            "miri-test-args": ["--test", "tests", "module::critical_case"]
                        }
                    }
                },
                {
                    "id": "transitive 0.1.0 (path+file:///workspace/transitive)",
                    "name": "transitive",
                    "metadata": {"rs-infra": {"miri": true}}
                }
            ]
        });

        let packages = miri_packages_from_metadata(&metadata)
            .expect("configured workspace packages should parse");

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "demo");
        assert_eq!(
            packages[0].test_args,
            ["--test", "tests", "module::critical_case"]
        );
    }

    #[test]
    fn test_miri_packages_reject_invalid_test_arguments() {
        let metadata = json!({
            "workspace_members": ["demo-id"],
            "packages": [{
                "id": "demo-id",
                "name": "demo",
                "metadata": {
                    "rs-infra": {"miri": true, "miri-test-args": ["--test", 7]}
                }
            }]
        });

        let error = miri_packages_from_metadata(&metadata)
            .expect_err("non-string test arguments must be rejected");

        assert!(
            error
                .to_string()
                .contains("miri-test-args entries must be strings")
        );
    }
}
