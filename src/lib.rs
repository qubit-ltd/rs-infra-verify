// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Cargo project verification primitives.

mod plan;
mod plan_entry;
mod plan_status;
mod runner;
mod suite;

pub use crate::plan::plan;
pub use crate::plan_entry::PlanEntry;
pub use crate::plan_status::PlanStatus;
pub use crate::runner::lock_check;
pub use crate::runner::lock_sync;
pub use crate::runner::run_suite;
pub use crate::suite::Suite;

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::plan::plan;
    use crate::plan_status::PlanStatus;
    use crate::suite::Suite;

    #[test]
    fn suite_enum_is_available_to_callers() {
        assert!(matches!(Suite::Test, Suite::Test));
    }

    #[test]
    fn every_legacy_capability_has_a_stable_suite_command() {
        assert_eq!(Suite::all().len(), 14);
        assert_eq!(
            Suite::Clippy.command(),
            &["clippy", "--workspace", "--all-targets", "--all-features"]
        );
        assert_eq!(
            Suite::Cross.command(),
            &["cross", "test", "--workspace", "--all-features"]
        );
        assert_eq!(Suite::Audit.command(), &["audit"]);
    }

    #[test]
    fn unconfigured_optional_capabilities_are_explicitly_skipped() {
        let project = tempdir().expect("temp project");
        fs::write(
            project.path().join("Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\n",
        )
        .expect("manifest");

        let plan = plan(project.path(), None).expect("plan");
        let miri = plan
            .iter()
            .find(|entry| entry.suite == Suite::Miri)
            .expect("miri entry");
        assert!(matches!(miri.status, PlanStatus::Skipped(_)));
        assert!(miri.message().contains("not configured"));
    }

    #[test]
    fn configured_capability_is_not_silently_skipped() {
        let project = tempdir().expect("temp project");
        fs::write(project.path().join("Cargo.toml"), "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\n\n[package.metadata.rs-ci]\nmiri=true\n").expect("manifest");

        let plan = plan(project.path(), Some(Suite::Miri)).expect("plan");
        assert!(matches!(plan[0].status, PlanStatus::Ready));
        assert!(plan[0].command.iter().any(|arg| *arg == "miri"));
    }
}
