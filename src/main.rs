use anyhow::Context;
use anyhow::Error;
use cargo::compile;
use cargo::CargoTarget;
use structopt::StructOpt;

mod cargo;
mod flamegraph;

#[derive(StructOpt)]
#[structopt(about = "The performance profiler for cargo")]
pub enum Command {
    /// Run all benchmark and store result as a json file.
    All,
    /// Create a flamegraph for given target
    Flamegraph {
        /// Use sudo.
        #[structopt(long)]
        root: bool,

        /// Compile library
        #[structopt(subcommand)]
        target: CargoTarget,
    },
}

fn main() -> Result<(), Error> {
    let cmd: Command = Command::from_args();

    match cmd {
        Command::All => {}
        Command::Flamegraph { root, target } => {
            compile(true, &target).context("cargo execution failed")?;
        }
    }

    Ok(())
}
