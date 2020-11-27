use crate::cargo::BinFile;
use anyhow::Context;
use anyhow::Error;
use std::process::Command;
use std::process::Stdio;

pub fn run_xctrace(file: &BinFile) -> Result<(), Error> {
    let mut cmd = Command::new("xcrun");

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
