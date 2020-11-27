use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use cargo::compile;
use cargo::CargoTarget;
use std::env;
use structopt::StructOpt;

mod cargo;
mod flamegraph;
mod perf_report;

#[derive(StructOpt)]
#[structopt(about = "The performance profiler for cargo")]
pub enum SubCommand {
    /// NOT IMPLEMENTED YET. Run all benchmark and store result as a json file.
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

    PerfReport {
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
    let mut args = env::args_os().collect::<Vec<_>>();

    if env::var("CARGO").is_ok() {
        args.remove(1);
    };

    let cmd: SubCommand = SubCommand::from_iter(args);

    match cmd {
        SubCommand::All => {}
        SubCommand::Flamegraph {
            root,
            target,
            release,
        } => {
            compile(release, &target).context("cargo execution failed")?;
        }

        SubCommand::PerfReport {
            root,
            target,
            release,
        } => {
            let binaries = compile(release, &target).context("cargo execution failed")?;
            for file in &binaries {
                self::perf_report::profile(file).context("perf-report failed")?;
            }
        }

        SubCommand::GetBin { target, release } => {
            let binraries = compile(release, &target).context("cargo execution failed")?;
            if binraries.len() != 1 {
                bail!(
                    "cargo produced too many binaries, which is not supoprted by `cargo profile \
                     bin-path`"
                )
            }
            print!("{}", binraries[0].path.display());
        }
    }

    Ok(())
}
