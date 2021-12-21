use crate::cargo::compile;
use crate::cargo::CargoTarget;
use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use structopt::StructOpt;
use xctrace::run_xctrace;

pub mod xctrace;

/// Invokes tracing tool.
#[derive(Debug, Clone, StructOpt)]
#[structopt(setting = structopt::clap::AppSettings::TrailingVarArg)]
pub struct TraceCommand {
    /// Use sudo.
    #[structopt(long)]
    root: bool,

    #[structopt(subcommand)]
    tool: TraceTool,
}

impl TraceCommand {
    pub fn run(self) -> Result<(), Error> {
        let Self { root, tool } = self;

        let target = tool.target();

        let binaries = compile(target).context("cargo execution failed")?;

        if binaries.len() != 1 {
            bail!(
                "cargo profile trace expects cargo to produce one binary file, but got {} files",
                binaries.len()
            )
        }

        let binary = binaries.into_iter().next().unwrap();

        match tool {
            TraceTool::Dtrace { .. } => {}
            TraceTool::Perf { .. } => {}
            TraceTool::Xctrace { .. } => {
                run_xctrace(root, &binary, target.args()).context("failed to run xctrace")?;
            }
        }

        Ok(())
    }
}

/// Tool used to generate trace.
#[derive(Debug, Clone, StructOpt)]
pub enum TraceTool {
    /// WIP
    Dtrace {
        /// Compile library
        #[structopt(subcommand)]
        target: CargoTarget,
    },
    /// WIP
    Perf {
        /// Compile library
        #[structopt(subcommand)]
        target: CargoTarget,
    },
    /// Invokes xctrace to create `.trace` file.
    Xctrace {
        /// Compile library
        #[structopt(subcommand)]
        target: CargoTarget,
    },
}

impl TraceTool {
    pub fn target(&self) -> &CargoTarget {
        match self {
            TraceTool::Dtrace { target }
            | TraceTool::Perf { target }
            | TraceTool::Xctrace { target } => target,
        }
    }
}
