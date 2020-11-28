use crate::cargo::compile;
use crate::cargo::CargoTarget;
use crate::cli_tools::dtrace::make_dtrace_command;
use crate::cli_tools::profiler::run_profiler;
use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::io::Cursor;
use std::path::Path;
use structopt::StructOpt;
use tempdir::TempDir;

mod linux;
mod macos;

/// Creates a flamegraph for given target.
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

            let cmd = if cfg!(target_os = "macos") {
                make_dtrace_command(
                    root,
                    binary,
                    &dir.path().join(self::macos::DTRACE_OUTPUT_FILENAME),
                    None,
                    None,
                    target.args(),
                )?
            } else if cfg!(target_os = "linux") {
                self::linux::perf(root, binary, None, target.args())?
            } else {
                bail!("cargo profile flamegraph currently supports only `linux` and `macos`")
            };

            run_profiler(cmd).context("failed to profile program")?;

            let collapsed: Vec<u8> = if cfg!(target_os = "macos") {
                crate::cli_tools::dtrace::to_collapsed(
                    &dir.path().join(self::macos::DTRACE_OUTPUT_FILENAME),
                )?
            } else if cfg!(target_os = "linux") {
                self::linux::to_collapsed()?
            } else {
                bail!("cargo profile flamegraph currently supports only `linux` and `macos`")
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
