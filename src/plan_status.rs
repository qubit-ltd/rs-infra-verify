// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Status values produced while planning verification suites.

/// Describes whether a verification suite can run for a project.
///
/// # Examples
///
/// ```
/// use qubit_infra_verify::PlanStatus;
///
/// let status = PlanStatus::Skipped("not configured".to_owned());
/// assert!(matches!(status, PlanStatus::Skipped(reason) if reason == "not configured"));
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlanStatus {
    /// The suite has the configuration required to run.
    Ready,
    /// The suite is unavailable and includes the reason it was skipped.
    Skipped(String),
}
