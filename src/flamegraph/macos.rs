use crate::cargo::BinFile;
use crate::util::command;
use anyhow::Context;
use anyhow::Error;
use inferno::collapse::dtrace::Folder;
use inferno::collapse::dtrace::Options as CollapseOptions;
use inferno::collapse::Collapse;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::BufWriter;
use std::path::Path;
use std::process::Command;

pub(super) const DTRACE_OUTPUT_FILENAME: &str = "cargo-profile-flamegraph.stacks";

pub(super) fn dtrace(
    root: bool,
    file: &BinFile,
    output: &Path,
    freq: Option<u32>,
    custom_cmd: Option<String>,
) -> Result<Command, Error> {
    let mut c = command(root, "dtrace");

    let dtrace_script = custom_cmd.unwrap_or(format!(
        "profile-{} /pid == $target/ {{ @[ustack(100)] = count(); }}",
        freq.unwrap_or(997)
    ));

    c.arg("-x");
    c.arg("ustackframes=100");

    c.arg("-n");
    c.arg(&dtrace_script);

    c.arg("-o");
    c.arg(output);

    c.arg("-c");
    c.arg(&file.path);

    Ok(c)
}

pub(super) fn to_collapsed(stacks_file: &Path) -> Result<Vec<u8>, Error> {
    let output = OpenOptions::new()
        .read(true)
        .write(false)
        .open(stacks_file)
        .with_context(|| {
            format!(
                "failed to open stacks file ({}) generated by dtrace",
                stacks_file.display()
            )
        })?;
    let perf_reader = BufReader::new(&output);

    let mut collapsed = vec![];

    let collapsed_writer = BufWriter::new(&mut collapsed);

    let collapse_options = CollapseOptions::default();

    Folder::from(collapse_options)
        .collapse(perf_reader, collapsed_writer)
        .with_context(|| {
            format!(
                "unable to collapse generated profile data from {}",
                stacks_file.display()
            )
        })?;

    Ok(collapsed)
}
