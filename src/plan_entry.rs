// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Entries describing planned verification suites.

use crate::Suite;
use crate::plan_status::PlanStatus;

/// A planned suite, its command arguments, and its availability.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use qubit_infra_verify::PlanStatus;
/// use qubit_infra_verify::Suite;
/// use qubit_infra_verify::plan;
///
/// let entries = plan(Path::new("."), Some(Suite::Test)).unwrap();
/// assert!(matches!(entries[0].status, PlanStatus::Ready));
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanEntry {
    /// The suite being planned.
    pub suite: Suite,
    /// Owned command and arguments associated with the suite.
    pub command: Vec<String>,
    /// Whether the suite is ready or was skipped.
    pub status: PlanStatus,
}

impl PlanEntry {
    /// Returns a human-readable status message for this entry.
    ///
    /// # Returns
    ///
    /// A newly allocated message that names the suite and describes whether
    /// it is enabled or why it was skipped.
    #[must_use]
    pub fn message(&self) -> String {
        match &self.status {
            PlanStatus::Ready => format!("{} is enabled", self.suite.name()),
            PlanStatus::Skipped(reason) => format!("{}: skipped: {reason}", self.suite.name()),
        }
    }
}
