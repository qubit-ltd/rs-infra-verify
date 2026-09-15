# rs-infra-verify

[![Rust CI](https://github.com/qubit-ltd/rs-infra-verify/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-infra-verify/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-infra-verify/coverage-badge.json)](https://qubit-ltd.github.io/rs-infra-verify/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-infra-verify.svg?color=blue)](https://crates.io/crates/qubit-infra-verify)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

为 Rust 项目提供可复用的 Cargo 构建、测试、锁文件和打包验证命令。

## 安装

```bash
cargo install --git https://github.com/qubit-ltd/rs-infra-verify.git --tag v0.1.0 qubit-infra-verify
```

## 快速开始

在 Rust 项目根目录查看命令帮助：

```bash
cargo run --manifest-path /path/to/rs-infra-verify/Cargo.toml -- --help
```

项目的 `.infra` 配置仍然是行为的唯一来源；工具仓库不会复制项目配置。具体策略由项目配置决定。

## 校验 suite

使用 `run --suite <name>` 运行单个 suite，或使用 `run --suite all` 运行全部 suite。可用 suite 包括 lock、build、test、doc、package、readme、clippy、feature-matrix、cross、platform、miri、address-sanitizer、loom、fuzz 和 audit。

`package` suite 对每个可发布的 workspace 成员执行
`cargo package --package <name> --allow-dirty`，由 Cargo 构建并验证打包结果。
`publish = false` 或 `publish = []` 的成员会跳过。对于本地路径依赖，工具按包
传入 crates.io patch，使尚未发布的兄弟包也能参与验证。验证使用的是这些本地依赖，
不能据此认定对应版本已经发布到 registry。

`readme` suite 读取各成员声明的 README（未声明时使用 `README.md`）及
`README.zh_CN.md`，检查其中所有 workspace 包的依赖声明。包版本为 `1.2.3` 时，
示例必须写作 `demo = "1.2"` 或 `demo = { version = "1.2" }`。
补丁版本、版本范围及缺少版本的声明都会报错，并给出文件路径和行号。
工具支持继承的 workspace 版本及自定义 README 路径；其他依赖不检查，
没有 README 或没有匹配声明时会明确提示跳过。

```bash
rs-infra-verify --project /path/to/project run --suite package
rs-infra-verify --project /path/to/project run --suite readme
```

可选 suite 沿用旧 rs-ci 的项目配置入口：`.rs-ci-cargo-matrix.json` 启用 feature matrix，`Cross.toml` 或 `.rs-ci-cross.toml` 启用 cross，`.rs-ci-platform.toml` 启用 platform；Cargo package metadata 启用 miri 和 AddressSanitizer，直接 Loom 依赖启用 loom；`fuzz/Cargo.toml` 必须包含 `cargo-fuzz = true` 标记才会启用 fuzz。

未配置的可选 suite 会明确输出 `skipped: not configured`。一旦配置，缺少工具或命令失败都会报错，不会静默跳过。

启用 Miri 的 package 可以在 `[package.metadata.rs-infra]` 中设置 `miri-test-args`，将 Cargo 测试目标或名称过滤条件传给 Miri。这样可以把耗时较长的检查集中到指定的安全关键测试上。

### Nightly、sanitizer、fuzz 和 Loom 检查

Miri、AddressSanitizer 和 fuzz 共用 `RS_INFRA_NIGHTLY_TOOLCHAIN`，默认值为
`nightly`。设置为 `nightly-2026-06-05` 可沿用旧 CI 固定的工具链。
工作流需要预先安装该工具链及所需的 `miri`、`rust-src` 组件，并安装指定版本的
cargo-fuzz。

AddressSanitizer 只运行明确声明 `sanitizers = ["address"]` 的 workspace 包。
配置放在 `[package.metadata.rs-infra]` 中，也兼容旧的
`[package.metadata.rs-ci]`；两者同时存在时以新配置为准。空列表表示不启用，
格式错误、重复条目或不支持的 sanitizer 都会报错。
工具逐包运行 `-Zbuild-std` 和全部 feature，并选择宿主 target，支持 Linux x86_64、
macOS x86_64 和 macOS aarch64；其他平台明确提示跳过。
运行时会在原有 `RUSTFLAGS`、`RUSTDOCFLAGS` 后追加 `-Zsanitizer=address`，
任一包的测试失败都会使检查失败。

`RS_INFRA_FUZZ_MODE` 控制 fuzz 的执行方式，未设置或为空时默认使用 `smoke`：

| 模式 | 行为 |
| --- | --- |
| `disabled` | 明确跳过，不校验 nightly、不调用 Cargo/cargo-fuzz，也不创建产物目录。 |
| `build-only` | 发现 target 后逐个执行 `cargo +<toolchain> fuzz build <target>`，不运行 smoke，也不创建 crash 目录。 |
| `smoke` | 发现 target 后逐个执行 `cargo +<toolchain> fuzz run <target>`，实际构建并运行 smoke。 |

非法 mode 会在调用 Cargo 前报错。工具本身不安装 cargo-fuzz；工作流也应在
`disabled` 时跳过 cargo-fuzz 安装步骤。`build-only` 会传播发现或构建失败；
运行时长和输入长度参数只在 `smoke` 模式下校验。
`RS_INFRA_FUZZ_SECONDS_PER_TARGET` 控制每个 target 的运行秒数，默认 `10`；
`RS_INFRA_FUZZ_MAX_LEN` 控制输入最大长度，默认 `4096`，两者必须为正整数。
旧项目需要更大输入时可设置 `RS_INFRA_FUZZ_MAX_LEN=16384`。
工具向 libFuzzer 传入 `-max_total_time`、`-max_len`，并为每个 target 设置独立的
`-artifact_prefix`，路径为 `fuzz/artifacts/<target>/`。
失败后保留 crash 文件，由工作流上传该目录；cargo-fuzz 也可能更新 corpus 和构建产物。
未发现 target、发现命令失败、参数无效或任一 target 失败都会报错。

Loom 根据 Cargo metadata 中的直接依赖筛选 workspace 成员，支持 dev、optional
和重命名的 `loom` 依赖。先设置 `RUSTFLAGS=--cfg loom`，逐包使用
`--release --all-features loom -- --list` 发现模型；任何已启用包没有模型时都会失败。
随后在 release 模式、全部 feature 下执行各包匹配的测试。
没有 Loom 依赖时明确提示跳过；注释和传递依赖不会启用检查。
Loom 使用项目配置的 Cargo 工具链。

```bash
RS_INFRA_NIGHTLY_TOOLCHAIN=nightly-2026-06-05 rs-infra-verify run --suite miri
RS_INFRA_NIGHTLY_TOOLCHAIN=nightly-2026-06-05 rs-infra-verify run --suite address-sanitizer
RS_INFRA_NIGHTLY_TOOLCHAIN=nightly-2026-06-05 RS_INFRA_FUZZ_MAX_LEN=16384 rs-infra-verify run --suite fuzz
rs-infra-verify run --suite loom
```

## 延伸阅读

可通过命令帮助和源码测试了解实际接口。切换到 [English README](README.md)。

## 测试

```bash
# 使用默认 feature 集运行测试
cargo test

# 使用项目声明的全部 feature 运行测试
cargo test --all-features

# 运行项目 CI 检查
./ci-check.sh

# 检查代码覆盖率
./coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./align-ci.sh`格式化代码，运行`./ci-check.sh`对齐CI要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-infra-verify](https://github.com/qubit-ltd/rs-infra-verify)
