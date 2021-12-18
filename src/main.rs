use crate::cargo::compile;
use crate::cargo::CargoTarget;
use crate::cpu::CpuCommand;
use crate::flamegraph::FlameGraphCommand;
use crate::instrument::InstrumentCommand;
use crate::trace::TraceCommand;
use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use std::env;
use structopt::StructOpt;

mod cargo;
mod cli_tools;
mod cpu;
mod flamegraph;
mod instrument;
mod trace;
mod util;

#[derive(StructOpt)]
#[structopt(
    name = "cargo profile",
    author,
    about = "The performance profiler for cargo"
)]
pub enum SubCommand {
    /// WIP. Run all benchmark and store result as a json file.
    All,
    Flamegraph(FlameGraphCommand),
    Trace(TraceCommand),
    Cpu(CpuCommand),

    Instrument(InstrumentCommand),

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

    if env::var("CARGO").is_ok() {
        if args.first().unwrap() == "cargo" {
            args.remove(1);
        } else {
            if match args.get(1) {
                Some(arg) if arg == "profile" => true,
                _ => false,
            } {
                args.remove(1);
            }
        }
    }

    let cmd: SubCommand = SubCommand::from_iter(args);

    match cmd {
        SubCommand::All => {}
        SubCommand::Flamegraph(cmd) => {
            cmd.run().context("failed to create flamegraph")?;
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
        SubCommand::Cpu(cmd) => cmd.run().context("failed to profile cpu usage")?,
        SubCommand::Instrument(cmd) => cmd.run().context("failed to instrument")?,
    }

    Ok(())
}
