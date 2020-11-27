use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use cargo::compile;
use cargo::CargoTarget;
use structopt::StructOpt;

mod cargo;
mod flamegraph;

#[derive(StructOpt)]
#[structopt(about = "The performance profiler for cargo")]
pub enum SubCommand {
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

        #[structopt(long)]
        release: bool,
    },

    /// Compile a binary using cargo and print absolute path to the file.
    ///
    /// Usage: perf record `cargo profile get-bin bench --bench fixture`
    GetBin {
        /// Compile library
        #[structopt(subcommand)]
        target: CargoTarget,

        #[structopt(long)]
        release: bool,
    },
}

fn main() -> Result<(), Error> {
    let cmd: SubCommand = SubCommand::from_args();

    match cmd {
        SubCommand::All => {}
        SubCommand::Flamegraph {
            root,
            target,
            release,
        } => {
            compile(release, &target).context("cargo execution failed")?;
        }

        SubCommand::GetBin { target, release } => {
            let binraries = compile(release, &target).context("cargo execution failed")?;
            if binraries.len() != 1 {
                bail!(
                    "cargo produced too many binaries, which is not supoprted by `cargo profile \
                     bin-path`"
                )
            }
            println!("{}", binraries[0].path.display());
        }
    }

    Ok(())
}
