use std::collections::HashMap;

use crate::cargo::compile;
use crate::cargo::CargoTarget;
use crate::cli_tools::dtrace::make_dtrace_command;
use crate::cli_tools::profiler::run_profiler;
use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use structopt::StructOpt;
use tempdir::TempDir;

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

                    process_collapsed(997, &collapsed)
                        .context("failed to process collapsed stack data")?;
                }

                Ok(())
            }
        }
    }
}

fn process_collapsed(freq: u32, data: &str) -> Result<(), Error> {
    let lines = data.lines();

    let mut fn_time_including_deps = HashMap::<_, usize>::new();
    let mut fn_time_itself = HashMap::<_, usize>::new();

    for line in data.lines() {
        let items = line.split(";");
        let items_count = items.clone().count();

        for (idx, mut item) in items.enumerate() {
            let is_last = idx == items_count - 1;

            println!("Item: {}", item);

            let splitted = item.split('`');
            if splitted.clone().count() != 2 {
                log::warn!("process_collapsed: ignoring wrong item (not separated by '`')");
                continue;
            }

            if is_last {
                item = item.trim_end_matches(|c: char| c.is_digit(10) || c == ' ');
            }

            println!("Correct item: {}", item);

            *fn_time_including_deps.entry(item).or_default() += 1;
            if is_last {
                // TODO: This is wrong.
                *fn_time_itself.entry(item).or_default() += 1;
            }
        }
    }

    println!("{:#?}", fn_time_including_deps);

    Ok(())
}
