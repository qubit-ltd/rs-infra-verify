// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Reads the shared toolchain defaults installed in a project.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;

/// Reads one toolchain field from the installed shared defaults.
///
/// # Parameters
///
/// * `project` - Project root containing the installed defaults.
/// * `field` - Toolchain field required by the caller.
///
/// # Errors
///
/// Returns a path-specific error if no defaults file exists, its TOML is
/// invalid, or the requested field is not a valid toolchain name.
pub(crate) fn toolchain(project: &Path, field: &str) -> Result<String> {
    let current = project.join(".infra/tools/defaults.toml");
    let legacy = project.join(".infra/ci/defaults.toml");
    let current_present = match fs::symlink_metadata(&current) {
        Ok(_) => true,
        Err(error) if error.kind() == ErrorKind::NotFound => false,
        Err(error) => {
            return Err(error).with_context(|| format!("cannot inspect {}", current.display()));
        }
    };
    let path = if current_present {
        current
    } else {
        match fs::symlink_metadata(&legacy) {
            Ok(_) => legacy,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                bail!(
                    "missing {} (legacy: {}); run update-infra.sh",
                    current.display(),
                    legacy.display()
                );
            }
            Err(error) => {
                return Err(error).with_context(|| format!("cannot inspect {}", legacy.display()));
            }
        }
    };
    let source =
        fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let table: toml::Value = source
        .parse()
        .with_context(|| format!("invalid TOML in {}", path.display()))?;
    let value = table
        .get(field)
        .and_then(toml::Value::as_str)
        .with_context(|| {
            format!(
                "{}: {field} must be a nonempty toolchain name",
                path.display()
            )
        })?;
    if value.is_empty() || value.starts_with(['+', '-']) || value.chars().any(char::is_whitespace) {
        bail!(
            "{}: {field} must be a nonempty toolchain name without whitespace or leading '+'/'-'",
            path.display()
        );
    }
    Ok(value.to_owned())
}
