use crate::cargo::CargoTarget;
use anyhow::Error;
use structopt::StructOpt;

pub mod xctrace;

#[derive(Debug, Clone, StructOpt)]
pub struct TraceCommand {
    /// Use sudo.
    #[structopt(long)]
    pub root: bool,

    #[structopt(long)]
    pub release: bool,

    #[structopt(subcommand)]
    pub tool: TraceTool,
}

impl TraceCommand {
    pub fn run(self) -> Result<(), Error> {
        let Self {
            root,
            release,
            tool,
        } = self;

        match tool {
            TraceTool::Dtrace { target } => {}
            TraceTool::Perf { target } => {}
            TraceTool::Xctrace { target } => {}
        }

        Ok(())
    }
}

/// Tool used to generate trace.
#[derive(Debug, Clone, StructOpt)]
pub enum TraceTool {
    Dtrace {
        /// Compile library
        #[structopt(subcommand)]
        target: CargoTarget,
    },
    Perf {
        /// Compile library
        #[structopt(subcommand)]
        target: CargoTarget,
    },
    Xctrace {
        /// Compile library
        #[structopt(subcommand)]
        target: CargoTarget,
    },
}
