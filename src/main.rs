use crate::trace::TraceCommand;
use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use cargo::compile;
use cargo::CargoTarget;
use flamegraph::FlameGraphCommand;
use std::env;
use structopt::StructOpt;

mod cargo;
mod flamegraph;
mod trace;
mod util;

#[derive(StructOpt)]
#[structopt(author, about = "The performance profiler for cargo")]
pub enum SubCommand {
    /// WIP. Run all benchmark and store result as a json file.
    All,
    Flamegraph(FlameGraphCommand),
    Trace(TraceCommand),

    /// Compile a binary using cargo and print absolute path to the file.
    ///
    /// Usage: perf record `cargo profile bin-path bench --bench fixture`
    BinPath {
        /// Compile library
        #[structopt(subcommand)]
        target: CargoTarget,

        #[structopt(long)]
        release: bool,
    },
}

fn main() -> Result<(), Error> {
    let mut args = env::args_os().collect::<Vec<_>>();

    if args.first().unwrap() == "cargo" && env::var("CARGO").is_ok() {
        args.remove(1);
    };

    let cmd: SubCommand = SubCommand::from_iter(args);

    match cmd {
        SubCommand::All => {}
        SubCommand::Flamegraph(cmd) => {
            cmd.run().context("faield to create flamegraph")?;
        }

        SubCommand::BinPath { target, release } => {
            let binraries = compile(release, &target).context("cargo execution failed")?;
            if binraries.len() != 1 {
                bail!(
                    "cargo produced too many binaries, which is not supoprted by `cargo profile \
                     bin-path`"
                )
            }
            print!("{}", binraries[0].path.display());
        }

        SubCommand::Trace(trace) => trace.run().context("failed to trace")?,
    }

    Ok(())
}
