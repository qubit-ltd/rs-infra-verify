//! Cargo project verification primitives.

use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use anyhow::{Context, Result, bail};

pub use crate::suite::Suite;

mod suite {
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub enum Suite {
        Lock,
        Build,
        Test,
        Doc,
        Package,
        Clippy,
        FeatureMatrix,
        Cross,
        Platform,
        Miri,
        AddressSanitizer,
        Loom,
        Fuzz,
        Audit,
    }

    impl Suite {
        pub const fn all() -> &'static [Self] {
            &[
                Self::Lock,
                Self::Build,
                Self::Test,
                Self::Doc,
                Self::Package,
                Self::Clippy,
                Self::FeatureMatrix,
                Self::Cross,
                Self::Platform,
                Self::Miri,
                Self::AddressSanitizer,
                Self::Loom,
                Self::Fuzz,
                Self::Audit,
            ]
        }

        pub const fn command(self) -> &'static [&'static str] {
            match self {
                Self::Lock => &["metadata", "--no-deps", "--locked", "--format-version", "1"],
                Self::Build => &["build", "--workspace", "--all-targets", "--all-features"],
                Self::Test => &["test", "--workspace", "--all-targets", "--all-features"],
                Self::Doc => &["doc", "--workspace", "--all-features", "--no-deps"],
                Self::Package => &["package", "--workspace", "--allow-dirty"],
                Self::Clippy => &["clippy", "--workspace", "--all-targets", "--all-features"],
                Self::FeatureMatrix => &["check", "--workspace", "--all-features"],
                Self::Cross => &["cross", "test", "--workspace", "--all-features"],
                Self::Platform => &["test", "--workspace", "--all-features"],
                Self::Miri => &["miri", "test", "--workspace", "--all-features"],
                Self::AddressSanitizer => &[
                    "test",
                    "-Zbuild-std",
                    "--target",
                    "x86_64-unknown-linux-gnu",
                    "--workspace",
                    "--all-features",
                ],
                Self::Loom => &["test", "--workspace", "--all-features", "loom"],
                Self::Fuzz => &["fuzz", "list"],
                Self::Audit => &["audit"],
            }
        }

        pub const fn name(self) -> &'static str {
            match self {
                Self::Lock => "lock",
                Self::Build => "build",
                Self::Test => "test",
                Self::Doc => "doc",
                Self::Package => "package",
                Self::Clippy => "clippy",
                Self::FeatureMatrix => "feature-matrix",
                Self::Cross => "cross",
                Self::Platform => "platform",
                Self::Miri => "miri",
                Self::AddressSanitizer => "address-sanitizer",
                Self::Loom => "loom",
                Self::Fuzz => "fuzz",
                Self::Audit => "audit",
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlanStatus {
    Ready,
    Skipped(String),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanEntry {
    pub suite: Suite,
    pub command: Vec<String>,
    pub status: PlanStatus,
}

impl PlanEntry {
    pub fn message(&self) -> String {
        match &self.status {
            PlanStatus::Ready => format!("{} is enabled", self.suite.name()),
            PlanStatus::Skipped(reason) => format!("{}: skipped: {reason}", self.suite.name()),
        }
    }
}

pub fn plan(project: &Path, selected: Option<Suite>) -> Result<Vec<PlanEntry>> {
    let suites = selected.map_or_else(|| Suite::all().to_vec(), |suite| vec![suite]);
    suites
        .into_iter()
        .map(|suite| {
            let configured = is_configured(project, suite)?;
            let status = if configured {
                PlanStatus::Ready
            } else {
                PlanStatus::Skipped(format!("not configured in {}", project.display()))
            };
            Ok(PlanEntry {
                suite,
                command: suite
                    .command()
                    .iter()
                    .map(|arg| (*arg).to_owned())
                    .collect(),
                status,
            })
        })
        .collect()
}

fn is_configured(project: &Path, suite: Suite) -> Result<bool> {
    let manifest = project.join("Cargo.toml");
    let manifest_text = std::fs::read_to_string(&manifest)
        .with_context(|| format!("failed to read {}", manifest.display()))?;
    Ok(match suite {
        Suite::Lock
        | Suite::Build
        | Suite::Test
        | Suite::Doc
        | Suite::Package
        | Suite::Clippy
        | Suite::Audit => true,
        Suite::FeatureMatrix => project.join(".rs-ci-cargo-matrix.json").is_file(),
        Suite::Cross => {
            project.join("Cross.toml").is_file() || project.join(".rs-ci-cross.toml").is_file()
        }
        Suite::Platform => project.join(".rs-ci-platform.toml").is_file(),
        Suite::Miri => has_metadata_flag(&manifest_text, "miri", "true"),
        Suite::AddressSanitizer => {
            manifest_text.contains("sanitizers") && manifest_text.contains("address")
        }
        Suite::Loom => manifest_text.contains("loom"),
        Suite::Fuzz => {
            project.join("fuzz/Cargo.toml").is_file()
                && std::fs::read_to_string(project.join("fuzz/Cargo.toml"))
                    .map(|text| text.contains("cargo-fuzz") && text.contains("true"))
                    .unwrap_or(false)
        }
    })
}

fn has_metadata_flag(manifest: &str, key: &str, value: &str) -> bool {
    manifest.lines().any(|line| {
        let compact: String = line
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect();
        compact == format!("{key}={value}") || compact.ends_with(&format!(".{key}={value}"))
    })
}

pub fn lock_check(project: &Path) -> Result<()> {
    run_quiet(
        project,
        &["metadata", "--no-deps", "--locked", "--format-version", "1"],
    )
    .context("Cargo.lock is missing or stale; run rs-infra-verify lock sync")?;
    println!("Cargo.lock is current.");
    Ok(())
}

pub fn lock_sync(project: &Path) -> Result<()> {
    run(project, &["generate-lockfile"])?;
    lock_check(project)
}

pub fn run_suite(project: &Path, suite: Suite) -> Result<()> {
    let entry = plan(project, Some(suite))?.remove(0);
    if let PlanStatus::Skipped(_) = entry.status {
        println!("{}", entry.message());
        return Ok(());
    }
    let (program, args): (&str, Vec<String>) = match suite {
        Suite::Cross => (entry.command[0].as_str(), entry.command[1..].to_vec()),
        Suite::AddressSanitizer => (
            "cargo",
            [vec!["+nightly".to_owned()], entry.command.clone()].concat(),
        ),
        _ => ("cargo", entry.command.clone()),
    };
    run_program(project, program, &args)
}

fn run_program(project: &Path, program: &str, args: &[String]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .current_dir(project)
        .status()
        .with_context(|| format!("failed to start {program} {}", args.join(" ")))?;
    if !status.success() {
        bail!("{program} {} failed", args.join(" "));
    }
    Ok(())
}

fn run(project: &Path, args: &[&str]) -> Result<()> {
    run_program(
        project,
        "cargo",
        &args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>(),
    )
}

fn run_quiet(project: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("cargo")
        .args(args)
        .current_dir(project)
        .stdout(Stdio::null())
        .output()
        .with_context(|| format!("failed to start cargo {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "cargo {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

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
