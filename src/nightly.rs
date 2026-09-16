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

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;

/// Returns the configured Cargo `+toolchain` argument, defaulting to
/// `+nightly`.
///
/// # Errors
///
/// Returns an error if `RS_INFRA_NIGHTLY_TOOLCHAIN` is non-Unicode, empty,
/// contains whitespace, or starts with an option/toolchain prefix.
pub(crate) fn toolchain() -> Result<String> {
    let selected = match env::var("RS_INFRA_NIGHTLY_TOOLCHAIN") {
        Ok(value) => value,
        Err(VarError::NotPresent) => "nightly".to_owned(),
        Err(error) => return Err(error).context("invalid RS_INFRA_NIGHTLY_TOOLCHAIN"),
    };
    if selected.is_empty() || selected.starts_with(['+', '-']) || selected.chars().any(char::is_whitespace) {
        bail!("RS_INFRA_NIGHTLY_TOOLCHAIN must be a nonempty toolchain name without a leading '+' or '-'");
    }
    Ok(format!("+{selected}"))
}
