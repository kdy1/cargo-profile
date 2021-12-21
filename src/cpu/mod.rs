use crate::cargo::compile;
use crate::cargo::CargoTarget;
use crate::cli_tools::dtrace::make_dtrace_command;
use crate::cli_tools::profiler::run_profiler;
use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use std::collections::HashMap;
use std::collections::HashSet;
use structopt::StructOpt;
use tempdir::TempDir;

mod merge;

/// WIP: Profiles cpu usage.
#[derive(Debug, Clone, StructOpt)]
pub enum CpuCommand {
    /// Profiles the program and print results in order of (total, local, function name).
    ///
    /// Note that cpu usage can be larger thsn 100%, if threads are used.
    PerFn {
        /// Use sudo.
        #[structopt(long)]
        root: bool,

        /// Compile library
        #[structopt(flatten)]
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

                    let (time, mut data) = process_collapsed(&collapsed)
                        .context("failed to process collapsed stack data")?;
                    data.sort_by_key(|info| info.total_used);

                    println!(
                        "{: <10}  | {: <10}  | {}",
                        "Totql time", "Own time", "File name",
                    );
                    for info in data.iter().rev() {
                        println!(
                            "{: <10.1}% | {: <10.1}% | {}",
                            info.total_used as f64 / time as f64 * 100f64,
                            info.self_used as f64 / time as f64 * 100f64,
                            info.name,
                        );
                    }
                }

                Ok(())
            }
        }
    }
}

struct FnTimingInfo {
    name: String,
    total_used: usize,
    /// The percentage of time used by function code itself.
    self_used: usize,
}

fn process_collapsed(data: &str) -> Result<(usize, Vec<FnTimingInfo>), Error> {
    let mut lines: Vec<&str> = data.lines().into_iter().collect();
    lines.reverse();
    let (frames, time, ignored) =
        merge::frames(lines, true).context("failed to merge collapsed stack frame")?;

    if time == 0 {
        bail!("No stack counts found")
    }

    if ignored > 0 {
        eprintln!("ignored {} lines with invalid format", ignored)
    }

    let mut total_time = HashMap::<_, usize>::new();
    let mut itself_time = HashMap::<_, usize>::new();

    // Check if time collapses
    for frame in &frames {
        let fn_dur = frame.end_time - frame.start_time;
        *total_time.entry(frame.location.function).or_default() += fn_dur;

        let children = frames.iter().filter(|child| {
            frame.location.depth + 1 == child.location.depth
                && frame.start_time <= child.start_time
                && child.end_time <= frame.end_time
        });

        let mut itself_dur = fn_dur;

        for child in children {
            let fn_dur = child.end_time - child.start_time;
            itself_dur -= fn_dur;
        }

        *itself_time.entry(frame.location.function).or_default() += itself_dur;
    }

    let mut result = vec![];
    let mut done = HashSet::new();

    for frame in &frames {
        if !done.insert(&frame.location.function) {
            continue;
        }
        result.push(FnTimingInfo {
            name: frame.location.function.to_string(),
            total_used: *total_time.entry(frame.location.function).or_default(),
            self_used: *itself_time.entry(frame.location.function).or_default(),
        });
    }

    Ok((time, result))
}
