// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Selects the nightly Rust toolchain used by advanced verification suites.

use std::env;
use std::env::VarError;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;

/// Returns the configured Cargo `+toolchain` argument, with a nonempty
/// environment override taking precedence over project defaults.
///
/// # Parameters
///
/// * `project` - Project root containing the installed shared defaults.
///
/// # Errors
///
/// Returns an error if the environment value is non-Unicode, the selected
/// toolchain is invalid, or the project defaults cannot be read.
pub(crate) fn toolchain(project: &Path) -> Result<String> {
    let selected = match env::var("RS_INFRA_NIGHTLY_TOOLCHAIN") {
        Ok(value) if !value.is_empty() => value,
        Ok(_) | Err(VarError::NotPresent) => {
            crate::defaults::toolchain(project, "nightly_toolchain")?
        }
        Err(error) => return Err(error).context("invalid RS_INFRA_NIGHTLY_TOOLCHAIN"),
    };
    if selected.is_empty() || selected.starts_with(['+', '-']) || selected.chars().any(char::is_whitespace) {
        bail!("RS_INFRA_NIGHTLY_TOOLCHAIN must be a nonempty toolchain name without a leading '+' or '-'");
    }
    Ok(format!("+{selected}"))
}
