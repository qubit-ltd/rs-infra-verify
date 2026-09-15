// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Legacy-compatible cargo-fuzz execution modes.

use std::env;
use std::env::VarError;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;

/// The operation selected by `RS_INFRA_FUZZ_MODE`.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum FuzzMode {
    /// Build and run bounded smoke tests.
    Smoke,
    /// Build every target without executing it.
    BuildOnly,
    /// Skip without invoking Cargo or requiring nightly.
    Disabled,
}

impl FuzzMode {
    /// Reads `RS_INFRA_FUZZ_MODE`, defaulting unset/empty values to smoke.
    ///
    /// # Returns
    ///
    /// The selected mode, matching the legacy shell default semantics.
    ///
    /// # Errors
    ///
    /// Returns a contextual error for non-Unicode or unsupported values.
    pub(crate) fn from_env() -> Result<Self> {
        let value = match env::var("RS_INFRA_FUZZ_MODE") {
            Ok(value) => value,
            Err(VarError::NotPresent) => return Ok(Self::Smoke),
            Err(error) => return Err(error).context("invalid RS_INFRA_FUZZ_MODE"),
        };
        match value.as_str() {
            "" | "smoke" => Ok(Self::Smoke),
            "build-only" => Ok(Self::BuildOnly),
            "disabled" => Ok(Self::Disabled),
            _ => bail!("RS_INFRA_FUZZ_MODE must be smoke, build-only, or disabled; got {value:?}"),
        }
    }
}
