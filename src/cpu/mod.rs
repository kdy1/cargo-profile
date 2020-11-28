use crate::cargo::CargoTarget;
use anyhow::Error;
use structopt::StructOpt;

/// Profiles cpu usage.
#[derive(Debug, Clone, StructOpt)]
pub enum CpuCommand {
    PerFn {
        /// Use sudo.
        #[structopt(long)]
        root: bool,

        /// Compile library
        #[structopt(subcommand)]
        target: CargoTarget,

        #[structopt(long)]
        release: bool,
    },
}

impl CpuCommand {
    pub fn run(self) -> Result<(), Error> {
        match self {
            CpuCommand::PerFn {
                root,
                target,
                release,
            } => Ok(()),
        }
    }
}
