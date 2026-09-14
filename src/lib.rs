//! Cargo project verification primitives.

use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use anyhow::{Context, Result, bail};

pub use crate::suite::Suite;

mod suite {
    #[derive(Debug, Clone, Copy)]
    pub enum Suite {
        Build,
        Test,
        Doc,
        Package,
    }
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
    let args: Vec<&str> = match suite {
        Suite::Build => vec!["build", "--workspace", "--all-targets", "--all-features"],
        Suite::Test => vec!["test", "--workspace", "--all-targets", "--all-features"],
        Suite::Doc => vec!["doc", "--workspace", "--all-features", "--no-deps"],
        Suite::Package => vec!["package", "--workspace", "--allow-dirty"],
    };
    run(project, &args)
}

fn run(project: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("cargo")
        .args(args)
        .current_dir(project)
        .status()
        .with_context(|| format!("failed to start cargo {}", args.join(" ")))?;
    if !status.success() {
        bail!("cargo {} failed", args.join(" "));
    }
    Ok(())
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

    #[test]
    fn suite_enum_is_available_to_callers() {
        assert!(matches!(Suite::Test, Suite::Test));
    }
}
