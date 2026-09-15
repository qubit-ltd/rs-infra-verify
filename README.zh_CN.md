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

可选 suite 沿用旧 rs-ci 的项目配置入口：`.rs-ci-cargo-matrix.json` 启用 feature matrix，`Cross.toml` 或 `.rs-ci-cross.toml` 启用 cross，`.rs-ci-platform.toml` 启用 platform；Cargo package metadata 启用 miri、AddressSanitizer 和 loom；`fuzz/Cargo.toml` 必须包含 `cargo-fuzz = true` 标记才会启用 fuzz。

未配置的可选 suite 会明确输出 `skipped: not configured`。一旦配置，缺少工具或命令失败都会报错，不会静默跳过。

启用 Miri 的 package 可以在 `[package.metadata.rs-infra]` 中设置 `miri-test-args`，将 Cargo 测试目标或名称过滤条件传给 Miri。这样可以把耗时较长的检查集中到指定的安全关键测试上。

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
