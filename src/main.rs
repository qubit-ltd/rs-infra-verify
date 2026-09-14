use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "rs-infra-verify")]
struct Cli {
    #[arg(long, default_value = ".")]
    project: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Lock {
        #[command(subcommand)]
        command: LockCommand,
    },
    Run {
        #[arg(long, value_enum)]
        suite: SuiteArg,
    },
}

#[derive(Debug, Subcommand)]
enum LockCommand {
    Check,
    Sync,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SuiteArg {
    All,
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let project = std::fs::canonicalize(cli.project)?;
    match cli.command {
        Command::Lock {
            command: LockCommand::Check,
        } => qubit_infra_verify::lock_check(&project),
        Command::Lock {
            command: LockCommand::Sync,
        } => qubit_infra_verify::lock_sync(&project),
        Command::Run { suite } => {
            if matches!(suite, SuiteArg::All) {
                for suite in qubit_infra_verify::Suite::all() {
                    qubit_infra_verify::run_suite(&project, *suite)?;
                }
                Ok(())
            } else {
                qubit_infra_verify::run_suite(&project, suite.into())
            }
        }
    }
}

impl From<SuiteArg> for qubit_infra_verify::Suite {
    fn from(suite: SuiteArg) -> Self {
        match suite {
            SuiteArg::All => unreachable!("all is handled before conversion"),
            SuiteArg::Lock => Self::Lock,
            SuiteArg::Build => Self::Build,
            SuiteArg::Test => Self::Test,
            SuiteArg::Doc => Self::Doc,
            SuiteArg::Package => Self::Package,
            SuiteArg::Clippy => Self::Clippy,
            SuiteArg::FeatureMatrix => Self::FeatureMatrix,
            SuiteArg::Cross => Self::Cross,
            SuiteArg::Platform => Self::Platform,
            SuiteArg::Miri => Self::Miri,
            SuiteArg::AddressSanitizer => Self::AddressSanitizer,
            SuiteArg::Loom => Self::Loom,
            SuiteArg::Fuzz => Self::Fuzz,
            SuiteArg::Audit => Self::Audit,
        }
    }
}
