// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CLI subprocess contracts use a Cargo stand-in instead of installing nightly.
#![cfg(unix)]

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;
use tempfile::tempdir;

/// Creates an opted-in project and a deterministic Cargo process stand-in.
fn fixture() -> TempDir {
    let root = tempdir().expect("fixture");
    fs::create_dir(root.path().join("src")).expect("src");
    fs::write(root.path().join("src/lib.rs"), "").expect("source");
    fs::write(root.path().join("Cargo.toml"), "[package]\nname='demo'\nversion='0.1.0'\nedition='2024'\n[package.metadata.rs-infra]\nmiri=true\nsanitizers=['address']\n").expect("manifest");
    fs::create_dir(root.path().join("fuzz")).expect("fuzz");
    fs::write(
        root.path().join("fuzz/Cargo.toml"),
        "[package.metadata]\ncargo-fuzz=true\n",
    )
    .expect("fuzz manifest");
    fs::create_dir(root.path().join("bin")).expect("bin");
    let cargo = root.path().join("bin/cargo");
    fs::write(&cargo, r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> "$COMMAND_LOG"
printf 'RUSTFLAGS=%s\nRUSTDOCFLAGS=%s\n' "${RUSTFLAGS:-}" "${RUSTDOCFLAGS:-}" >> "$COMMAND_LOG.env"
if [ "$1" = metadata ]; then exec "$REAL_CARGO" "$@"; fi
case " $* " in
  *" fuzz list "*)
    if [ "${FAIL_LIST:-0}" = 1 ]; then echo 'list failed' >&2; exit 1; fi
    printf '%s' "${FUZZ_TARGETS-alpha
beta
}"
    ;;
  *" fuzz build "*)
    if [ "${FAIL_BUILD:-0}" = 1 ]; then exit 1; fi
    ;;
  *" fuzz run "*)
    if [ "${FAIL_RUN:-0}" = 1 ]; then
      for arg in "$@"; do
        case "$arg" in -artifact_prefix=*) prefix=${arg#-artifact_prefix=}; printf crash > "${prefix}crash-fixture";; esac
      done
      exit 1
    fi
    ;;
  *" miri test "*) echo 'running 1 test';;
esac
"#).expect("Cargo stand-in");
    fs::set_permissions(cargo, fs::Permissions::from_mode(0o755)).expect("executable");
    root
}

/// Constructs a CLI command with child-only environment isolation.
fn cli(root: &Path, suite: &str) -> Command {
    let real_cargo = Command::new("rustup")
        .args(["which", "cargo"])
        .output()
        .expect("cargo path");
    assert!(real_cargo.status.success());
    let mut paths = vec![root.join("bin")];
    paths.extend(env::split_paths(&env::var_os("PATH").expect("PATH")));
    let mut command = Command::new(env!("CARGO_BIN_EXE_rs-infra-verify"));
    command
        .args(["--project", root.to_str().expect("path"), "run", "--suite", suite])
        .env("PATH", env::join_paths(paths).expect("PATH"))
        .env(
            "REAL_CARGO",
            String::from_utf8(real_cargo.stdout).expect("cargo path").trim(),
        )
        .env("COMMAND_LOG", root.join("commands.log"))
        .env_remove("RS_INFRA_NIGHTLY_TOOLCHAIN")
        .env_remove("RS_INFRA_FUZZ_SECONDS_PER_TARGET")
        .env_remove("RS_INFRA_FUZZ_MAX_LEN")
        .env_remove("RS_INFRA_FUZZ_MODE")
        .env_remove("FAIL_BUILD")
        .env_remove("FAIL_LIST")
        .env_remove("FAIL_RUN")
        .env_remove("FUZZ_TARGETS");
    command
}

#[test]
fn test_miri_and_sanitizer_select_configured_nightly() {
    for suite in ["miri", "address-sanitizer"] {
        for toolchain in [None, Some("nightly-2026-06-05")] {
            let root = fixture();
            let mut command = cli(root.path(), suite);
            if let Some(toolchain) = toolchain {
                command.env("RS_INFRA_NIGHTLY_TOOLCHAIN", toolchain);
            }
            let result = command.output().expect("suite");
            assert!(result.status.success(), "{result:?}");
            let log = fs::read_to_string(root.path().join("commands.log")).expect("commands");
            let subcommand = if suite == "miri" { "miri test" } else { "test" };
            assert!(
                log.contains(&format!("+{} {subcommand}", toolchain.unwrap_or("nightly"))),
                "{log}"
            );
        }
    }
}

#[test]
fn test_fuzz_runs_every_target_with_limits_and_artifact_directories() {
    for custom in [false, true] {
        let root = fixture();
        let mut command = cli(root.path(), "fuzz");
        if custom {
            command
                .env("RS_INFRA_NIGHTLY_TOOLCHAIN", "nightly-2026-06-05")
                .env("RS_INFRA_FUZZ_MODE", "smoke")
                .env("RS_INFRA_FUZZ_SECONDS_PER_TARGET", "2")
                .env("RS_INFRA_FUZZ_MAX_LEN", "128");
        }
        let result = command.output().expect("fuzz suite");
        assert!(result.status.success(), "{result:?}");
        let log = fs::read_to_string(root.path().join("commands.log")).expect("commands");
        let nightly = if custom { "nightly-2026-06-05" } else { "nightly" };
        for target in ["alpha", "beta"] {
            let run = log
                .lines()
                .find(|line| line.contains(&format!("fuzz run {target} ")))
                .expect("target must execute");
            assert!(run.starts_with(&format!("+{nightly} fuzz run")), "{run}");
            assert!(run.contains(if custom {
                "-max_total_time=2"
            } else {
                "-max_total_time=10"
            }));
            assert!(run.contains(if custom { "-max_len=128" } else { "-max_len=4096" }));
            assert!(run.contains(&format!(
                "-artifact_prefix={}/fuzz/artifacts/{target}/",
                root.path().display()
            )));
            assert!(root.path().join("fuzz/artifacts").join(target).is_dir());
        }
    }
}

#[test]
fn test_fuzz_failures_preserve_artifacts_and_propagate() {
    let root = fixture();
    let result = cli(root.path(), "fuzz").env("FAIL_RUN", "1").output().expect("fuzz");
    assert!(!result.status.success());
    assert!(root.path().join("fuzz/artifacts/alpha/crash-fixture").is_file());
    assert!(String::from_utf8_lossy(&result.stderr).contains("alpha"));
}

#[test]
fn test_fuzz_rejects_empty_targets_and_invalid_limits() {
    for (name, value) in [
        ("FUZZ_TARGETS", ""),
        ("RS_INFRA_FUZZ_SECONDS_PER_TARGET", "0"),
        ("RS_INFRA_FUZZ_MAX_LEN", "-1"),
        ("RS_INFRA_FUZZ_MAX_LEN", "abc"),
        ("FAIL_LIST", "1"),
    ] {
        let root = fixture();
        let result = cli(root.path(), "fuzz").env(name, value).output().expect("fuzz");
        assert!(!result.status.success(), "{name}={value}: {result:?}");
    }
}

#[test]
fn test_sanitizer_instruments_only_opted_in_workspace_packages() {
    let root = fixture();
    let manifest = root.path().join("Cargo.toml");
    let content = fs::read_to_string(&manifest).expect("manifest");
    fs::write(
        &manifest,
        format!("{content}\n[workspace]\nmembers=['enabled','disabled']\n"),
    )
    .expect("workspace");
    for name in ["enabled", "disabled"] {
        fs::create_dir_all(root.path().join(name).join("src")).expect("source dir");
        fs::write(root.path().join(name).join("src/lib.rs"), "").expect("source");
        let setting = if name == "enabled" { "['address']" } else { "[]" };
        fs::write(root.path().join(name).join("Cargo.toml"), format!("[package]\nname='{name}'\nversion='0.1.0'\nedition='2024'\n[package.metadata.rs-ci]\nsanitizers={setting}\n")).expect("member");
    }
    let result = cli(root.path(), "address-sanitizer")
        .env("RUSTFLAGS", "--cfg existing")
        .env("RUSTDOCFLAGS", "--cfg docs")
        .output()
        .expect("sanitizer");
    assert!(result.status.success(), "{result:?}");
    let log = fs::read_to_string(root.path().join("commands.log")).expect("commands");
    assert!(
        log.contains("--package demo") && log.contains("--package enabled"),
        "{log}"
    );
    assert!(
        !log.contains("--package disabled") && !log.contains("--workspace"),
        "{log}"
    );
    let flags = fs::read_to_string(root.path().join("commands.log.env")).expect("flags");
    assert!(
        flags.contains("RUSTFLAGS=--cfg existing -Zsanitizer=address"),
        "{flags}"
    );
    assert!(flags.contains("RUSTDOCFLAGS=--cfg docs -Zsanitizer=address"), "{flags}");
}

#[test]
fn test_sanitizer_rejects_malformed_package_configuration() {
    for value in ["'address'", "[42]", "['address','address']", "['thread']"] {
        let root = fixture();
        let path = root.path().join("Cargo.toml");
        let content = fs::read_to_string(&path).expect("manifest");
        fs::write(
            &path,
            content.replace("sanitizers=['address']", &format!("sanitizers={value}")),
        )
        .expect("invalid config");
        let result = cli(root.path(), "address-sanitizer").output().expect("suite");
        assert!(!result.status.success(), "{value}: {result:?}");
        assert!(String::from_utf8_lossy(&result.stderr).contains("sanitizers"));
    }
}

#[test]
fn test_sanitizer_ignores_unrelated_manifest_text() {
    let root = fixture();
    let path = root.path().join("Cargo.toml");
    let content = fs::read_to_string(&path).expect("manifest");
    fs::write(
        &path,
        content.replace("sanitizers=['address']", "sanitizers=[] # address is not enabled"),
    )
    .expect("disabled config");
    let result = cli(root.path(), "address-sanitizer").output().expect("suite");
    assert!(result.status.success(), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stdout).contains("skipped"));
    let log = fs::read_to_string(root.path().join("commands.log")).expect("commands");
    assert!(!log.contains("+nightly test"));
}

#[test]
fn test_fuzz_disabled_requires_no_cargo_or_nightly() {
    let root = fixture();
    let result = cli(root.path(), "fuzz")
        .env("RS_INFRA_FUZZ_MODE", "disabled")
        .env("RS_INFRA_NIGHTLY_TOOLCHAIN", "")
        .env("RS_INFRA_FUZZ_SECONDS_PER_TARGET", "invalid")
        .env("PATH", "")
        .output()
        .expect("disabled suite");
    assert!(result.status.success(), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stdout).contains("disabled"));
    assert!(!root.path().join("commands.log").exists());
    assert!(!root.path().join("fuzz/artifacts").exists());
}

#[test]
fn test_fuzz_build_only_builds_every_target_without_smoke_side_effects() {
    let root = fixture();
    let result = cli(root.path(), "fuzz")
        .env("RS_INFRA_FUZZ_MODE", "build-only")
        .env("RS_INFRA_FUZZ_SECONDS_PER_TARGET", "invalid")
        .env("RS_INFRA_FUZZ_MAX_LEN", "invalid")
        .output()
        .expect("build suite");
    assert!(result.status.success(), "{result:?}");
    let log = fs::read_to_string(root.path().join("commands.log")).expect("commands");
    for target in ["alpha", "beta"] {
        assert!(log.contains(&format!("+nightly fuzz build {target}")), "{log}");
    }
    assert!(!log.contains("fuzz run") && !log.contains(" install "), "{log}");
    assert!(!root.path().join("fuzz/artifacts").exists());
}

#[test]
fn test_fuzz_build_only_propagates_build_failure() {
    let root = fixture();
    let result = cli(root.path(), "fuzz")
        .env("RS_INFRA_FUZZ_MODE", "build-only")
        .env("FAIL_BUILD", "1")
        .output()
        .expect("build suite");
    assert!(!result.status.success(), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stderr).contains("alpha"));
}

#[test]
fn test_fuzz_invalid_mode_fails_before_cargo() {
    let root = fixture();
    let result = cli(root.path(), "fuzz")
        .env("RS_INFRA_FUZZ_MODE", "unknown")
        .output()
        .expect("invalid suite");
    assert!(!result.status.success(), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stderr).contains("RS_INFRA_FUZZ_MODE"));
    assert!(!root.path().join("commands.log").exists());
}
