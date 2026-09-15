// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Validates README dependency snippets for all Cargo workspace members.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use regex::Regex;
use regex::escape;

use crate::metadata::workspace_metadata;

/// Checks existing declared/default and Chinese READMEs.
///
/// # Parameters
///
/// * `project` - Cargo workspace directory whose README files are read.
///
/// # Returns
///
/// Success when declarations use exact workspace major.minor versions, or no
/// README files or matching declarations exist.
///
/// # Errors
///
/// Returns metadata, file/UTF-8, or aggregated path-and-line validation errors.
/// This reads files and invokes Cargo metadata; it never rewrites README files.
pub(crate) fn verify(project: &Path) -> Result<()> {
    let metadata = workspace_metadata(project)?;
    let members = metadata["workspace_members"]
        .as_array()
        .context("missing workspace members")?;
    let packages: Vec<_> = metadata["packages"]
        .as_array()
        .context("missing packages")?
        .iter()
        .filter(|package| members.contains(&package["id"]))
        .collect();
    if packages.is_empty() {
        bail!("Cargo metadata did not report any workspace packages");
    }
    let mut paths = BTreeSet::new();
    for package in &packages {
        let manifest = Path::new(
            package["manifest_path"]
                .as_str()
                .context("missing manifest path")?,
        );
        let root = manifest.parent().context("manifest has no parent")?;
        for path in [
            root.join(package["readme"].as_str().unwrap_or("README.md")),
            root.join("README.zh_CN.md"),
        ] {
            if path.is_file() {
                paths.insert(
                    fs::canonicalize(&path)
                        .with_context(|| format!("cannot resolve {}", path.display()))?,
                );
            }
        }
    }
    let minor = Regex::new(r"^(\d+)\.(\d+)(?:[.\-+]|$)")?;
    let string = Regex::new(r#"^"([^"]+)"\s*$"#)?;
    let inline = Regex::new(r#"\bversion\s*=\s*"([^"]+)""#)?;
    let mut errors = Vec::new();
    let mut checked = 0;
    for path in &paths {
        let content =
            fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
        let display = path.strip_prefix(project).unwrap_or(path).display();
        for package in &packages {
            let name = package["name"].as_str().context("missing package name")?;
            let version = package["version"]
                .as_str()
                .context("missing package version")?;
            let parts = minor
                .captures(version)
                .context("package version does not start with major.minor")?;
            let expected = format!("{}.{}", &parts[1], &parts[2]);
            let declaration = Regex::new(&format!(
                r"^\s*{}\s*=\s*(?P<value>.+?)\s*(?:#.*)?$",
                escape(name)
            ))?;
            for (index, line) in content.lines().enumerate() {
                let Some(found) = declaration.captures(line) else {
                    continue;
                };
                checked += 1;
                let value = found["value"].trim();
                let found_version = string.captures(value).or_else(|| inline.captures(value));
                match found_version {
                    Some(found) if found[1] == expected => {},
                    Some(found) => errors.push(format!("{display}:{}: expected \"{expected}\" for {name}, found \"{}\"", index + 1, &found[1])),
                    None => errors.push(format!("{display}:{}: dependency declaration for {name} must include version = \"{expected}\"", index + 1)),
                }
            }
        }
    }
    if !errors.is_empty() {
        bail!("{}", errors.join("\n"));
    }
    if paths.is_empty() {
        println!("No README files found; skipping README dependency version check.");
    } else if checked == 0 {
        println!(
            "No README dependency declarations found for workspace packages; skipping README dependency version check."
        );
    } else {
        println!(
            "README dependency versions match workspace package versions ({checked} declarations)."
        );
    }
    Ok(())
}
