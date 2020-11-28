use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::io::Cursor;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::ExitStatus;
use structopt::StructOpt;
use tempdir::TempDir;

use crate::cargo::compile;
use crate::cargo::CargoTarget;

mod linux;
mod macos;

/// Creates a flamegraph for given target. This command
#[derive(Debug, Clone, StructOpt)]
pub struct FlameGraphCommand {
    /// Use sudo.
    #[structopt(long)]
    root: bool,

    /// Compile library
    #[structopt(subcommand)]
    target: CargoTarget,

    #[structopt(long)]
    release: bool,
}

#[cfg(unix)]
fn terminated_by_error(status: ExitStatus) -> bool {
    status
        .signal() // the default needs to be true because that's the neutral element for `&&`
        .map_or(true, |code| {
            code != signal_hook::SIGINT && code != signal_hook::SIGTERM
        })
        && !status.success()
}

#[cfg(not(unix))]
fn terminated_by_error(status: ExitStatus) -> bool {
    !status.success()
}

impl FlameGraphCommand {
    pub fn run(self) -> Result<(), Error> {
        let Self {
            root,
            target,
            release,
        } = self;

        let binaries = compile(release, &target).context("cargo execution failed")?;

        if binaries.len() != 1 {
            // TODO
            bail!(
                "Currently cargo profile flaemgraph only supports single binary, but cargo \
                 produced {} binaries",
                binaries.len()
            )
        }

        for binary in &binaries {
            let dir = TempDir::new("cargo-profile").context("failed to create temp dir")?;

            //
            eprintln!("Profiling {}", binary.path.display());

            let mut cmd = if cfg!(target_os = "macos") {
                self::macos::dtrace(
                    root,
                    binary,
                    &dir.path().join(self::macos::DTRACE_OUTPUT_FILENAME),
                    None,
                    None,
                )?
            } else {
                bail!("cargo profile flamegraph currently supports only mac os")
            };
            let cmd_str = format!("{:?}", cmd);

            // Handle SIGINT with an empty handler. This has the
            // implicit effect of allowing the signal to reach the
            // process under observation while we continue to
            // generate our flamegraph.  (ctrl+c will send the
            // SIGINT signal to all processes in the foreground
            // process group).
            #[cfg(unix)]
            let handler = unsafe {
                signal_hook::register(signal_hook::SIGINT, || {})
                    .expect("cannot register signal handler")
            };

            let mut recorder = cmd
                .spawn()
                .with_context(|| format!("failed to spawn: {}", cmd_str))?;

            let exit_status = recorder
                .wait()
                .with_context(|| format!("failed to wait for child proceess: {}", cmd_str))?;

            #[cfg(unix)]
            signal_hook::unregister(handler);

            // only stop if perf exited unsuccessfully, but
            // was not killed by a signal (assuming that the
            // latter case usually means the user interrupted
            // it in some way)
            if terminated_by_error(exit_status) {
                bail!("failed to sample program: {}", cmd_str);
            }

            let collapsed: Vec<u8> = if cfg!(target_os = "macos") {
                self::macos::to_collapsed(&dir.path().join(self::macos::DTRACE_OUTPUT_FILENAME))?
            } else {
                bail!("cargo profile flamegraph currently supports only mac os")
            };
            let mut collapsed = Cursor::new(collapsed);

            // TODO
            let flamegraph_file_path = Path::new("flamegraph.svg");
            let flamegraph_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&flamegraph_file_path)
                .context("unable to create flamegraph.svg output file")?;

            let flamegraph_writer = BufWriter::new(flamegraph_file);

            let mut flamegraph_options = inferno::flamegraph::Options::default();

            inferno::flamegraph::from_reader(
                &mut flamegraph_options,
                &mut collapsed,
                flamegraph_writer,
            )
            .with_context(|| {
                format!(
                    "unable to generate a flamegraph file ({}) from the collapsed stack data",
                    flamegraph_file_path.display()
                )
            })?;
        }

        Ok(())
    }
}
