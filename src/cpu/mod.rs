use crate::cargo::compile;
use crate::cargo::CargoTarget;
use crate::cli_tools::dtrace::make_dtrace_command;
use crate::cli_tools::profiler::run_profiler;
use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use std::collections::HashMap;
use structopt::StructOpt;
use tempdir::TempDir;

mod merge;

/// WIP: Profiles cpu usage.
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
            } => {
                let binaries = compile(release, &target).context("failed to compile")?;

                for binary in &binaries {
                    let dir =
                        TempDir::new("cargo-profile-cpu").context("failed to create temp dir")?;

                    let cmd = if cfg!(target_os = "macos") {
                        make_dtrace_command(
                            root,
                            binary,
                            &dir.path().join("program.stacks"),
                            None,
                            None,
                            target.args(),
                        )?
                    } else {
                        bail!("cargo profile cpu currently supports only `macos`")
                    };
                    run_profiler(cmd).context("failed to profile program")?;

                    let collapsed: Vec<u8> = if cfg!(target_os = "macos") {
                        crate::cli_tools::dtrace::to_collapsed(&dir.path().join("program.stacks"))?
                    } else {
                        unreachable!()
                    };

                    let collapsed = String::from_utf8_lossy(&collapsed);

                    process_collapsed(&collapsed)
                        .context("failed to process collapsed stack data")?;
                }

                Ok(())
            }
        }
    }
}

fn process_collapsed(data: &str) -> Result<(), Error> {
    let mut lines: Vec<&str> = data.lines().into_iter().collect();
    lines.reverse();
    let (frames, time, ignored) =
        merge::frames(lines, true).context("failed to merge collapsed stack frame")?;

    let mut time_used_by_fns = HashMap::<_, f64>::new();

    for frame in &frames {
        let dur = frame.end_time - frame.start_time;
        *time_used_by_fns.entry(frame.location.function).or_default() += dur as f64 / time as f64;
    }

    dbg!(&time_used_by_fns);

    todo!()
}
