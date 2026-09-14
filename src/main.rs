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
    Build,
    Test,
    Doc,
    Package,
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
        Command::Run { suite } => qubit_infra_verify::run_suite(
            &project,
            match suite {
                SuiteArg::Build => qubit_infra_verify::Suite::Build,
                SuiteArg::Test => qubit_infra_verify::Suite::Test,
                SuiteArg::Doc => qubit_infra_verify::Suite::Doc,
                SuiteArg::Package => qubit_infra_verify::Suite::Package,
            },
        ),
    }
}
