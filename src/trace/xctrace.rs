use crate::cargo::BinFile;
use crate::util::command;
use anyhow::Context;
use anyhow::Error;
use std::process::Stdio;

pub fn run_xctrace(root: bool, file: &BinFile) -> Result<(), Error> {
    let mut cmd = command(root, "xcrun");

    cmd.stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .arg("xctrace")
        .arg("record");

    cmd.arg("--template").arg("Time Profiler");

    cmd.arg("--launch")
        .arg("--")
        .arg(&file.path)
        .spawn()
        .with_context(|| {
            format!(
                "failed to spawn xcrun xctrace record `{}`",
                file.path.display(),
            )
        })?;

    Ok(())
}
