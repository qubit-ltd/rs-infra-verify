// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Output;

use tempfile::tempdir;

/// Writes a minimal package for real, offline Cargo verification.
fn package(root: &Path, name: &str, extra: &str, source: &str) {
    fs::create_dir_all(root.join("src")).expect("source directory");
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname='{name}'\nversion='1.2.3'\nedition='2024'\n{extra}\n"),
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), source).expect("source");
}

/// Invokes the public CLI without network access.
fn verify(root: &Path, suite: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rs-infra-verify"))
        .args([
            "--project",
            root.to_str().expect("path"),
            "run",
            "--suite",
            suite,
        ])
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .expect("CLI")
}

#[test]
fn test_package_rejects_source_that_only_listing_would_accept() {
    let root = tempdir().expect("fixture");
    package(
        root.path(),
        "broken-fixture",
        "",
        "compile_error!(\"package must compile\");",
    );
    let result = verify(root.path(), "package");
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("package must compile"),
        "{result:?}"
    );
}

#[test]
fn test_package_builds_publishable_members_with_local_dependencies() {
    let root = tempdir().expect("fixture");
    fs::write(
        root.path().join("Cargo.toml"),
        "[workspace]\nmembers=['app','sibling','private']\nresolver='3'\n",
    )
    .expect("workspace");
    package(
        &root.path().join("sibling"),
        "unpublished-local-fixture",
        "publish=false",
        "pub fn value() -> u8 { 1 }",
    );
    package(
        &root.path().join("private"),
        "private-fixture",
        "publish=[]",
        "compile_error!(\"must skip private\");",
    );
    package(
        &root.path().join("app"),
        "app-fixture",
        "[dependencies]\nunpublished-local-fixture={path='../sibling',version='1.2'}",
        "pub fn value() -> u8 { unpublished_local_fixture::value() }",
    );
    let result = verify(root.path(), "package");
    assert!(result.status.success(), "{result:?}");
    assert!(
        root.path()
            .join("target/package/app-fixture-1.2.3.crate")
            .is_file()
    );
}

#[test]
fn test_readme_checks_all_workspace_names_and_both_languages() {
    let root = tempdir().expect("fixture");
    package(root.path(), "demo", "", "");
    fs::write(
        root.path().join("README.md"),
        "demo = \"1.2\" # correct\nother = \"9\"\n",
    )
    .expect("README");
    fs::write(
        root.path().join("README.zh_CN.md"),
        "demo = { version = \"1.1\", features = [] }\ndemo = { path = \".\" }\n",
    )
    .expect("Chinese README");
    let result = verify(root.path(), "readme");
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(!result.status.success());
    assert!(
        error.contains("README.zh_CN.md:1") && error.contains("README.zh_CN.md:2"),
        "{error}"
    );
    fs::write(
        root.path().join("README.zh_CN.md"),
        "demo = { version = \"1.2\" }\n",
    )
    .expect("correct README");
    assert!(verify(root.path(), "readme").status.success());
}

#[test]
fn test_package_skips_workspace_without_publishable_members() {
    let root = tempdir().expect("fixture");
    package(
        root.path(),
        "private-fixture",
        "publish=false",
        "compile_error!(\"skip\");",
    );
    let result = verify(root.path(), "package");
    assert!(result.status.success(), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stdout).contains("No publishable workspace packages"));
}

#[test]
fn test_readme_uses_inherited_versions_custom_paths_and_cross_member_snippets() {
    let root = tempdir().expect("fixture");
    fs::write(root.path().join("Cargo.toml"), "[workspace]\nmembers=['first','second']\nresolver='3'\n[workspace.package]\nversion='2.4.9-beta.1'\n").expect("workspace");
    package(
        &root.path().join("first"),
        "first",
        "readme='../GUIDE.md'",
        "",
    );
    package(&root.path().join("second"), "second", "", "");
    let path = root.path().join("second/Cargo.toml");
    fs::write(
        &path,
        fs::read_to_string(&path)
            .expect("manifest")
            .replace("version='1.2.3'", "version.workspace=true"),
    )
    .expect("inherited version");
    fs::write(
        root.path().join("GUIDE.md"),
        "first = \"1.2\"\nsecond = { version = \"2.4\" } # inherited\n",
    )
    .expect("custom README");
    assert!(verify(root.path(), "readme").status.success());
    for invalid in ["2.4.9", "^2.4", "2.3", "2.4.9-beta.1"] {
        fs::write(
            root.path().join("GUIDE.md"),
            format!("second = \"{invalid}\"\n"),
        )
        .expect("incorrect README");
        let result = verify(root.path(), "readme");
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr)
                .contains("GUIDE.md:1: expected \"2.4\" for second")
        );
    }
}

#[test]
fn test_readme_skips_absent_files_and_unrelated_declarations() {
    let root = tempdir().expect("fixture");
    package(root.path(), "demo", "", "");
    let absent = verify(root.path(), "readme");
    assert!(absent.status.success());
    assert!(String::from_utf8_lossy(&absent.stdout).contains("No README files"));
    fs::write(
        root.path().join("README.md"),
        "demo-extra = \"9\"\n# demo = \"9\"\n",
    )
    .expect("README");
    let unrelated = verify(root.path(), "readme");
    assert!(unrelated.status.success());
    assert!(
        String::from_utf8_lossy(&unrelated.stdout).contains("No README dependency declarations")
    );
}

#[test]
fn test_readme_reports_invalid_utf8() {
    let root = tempdir().expect("fixture");
    package(root.path(), "demo", "", "");
    fs::write(root.path().join("README.md"), [255]).expect("invalid README");
    let result = verify(root.path(), "readme");
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("cannot read"));
}
