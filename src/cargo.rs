use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use cargo_metadata::Message;
use std::env;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use structopt::StructOpt;

/// Built bin file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BinFile {
    pub path: PathBuf,
}

#[derive(Debug, Clone, StructOpt)]
pub struct TestSpecifier {
    #[structopt(long)]
    pub lib: bool,

    #[structopt(long)]
    pub test: Option<String>,

    #[structopt(long)]
    pub tests: bool,
}

#[derive(Debug, Clone, StructOpt)]
pub struct BenchSpecifier {
    #[structopt(long)]
    pub lib: bool,

    #[structopt(long)]
    pub bench: Option<String>,

    #[structopt(long)]
    pub benches: bool,
}

#[derive(Debug, Clone, StructOpt)]
pub enum CargoTarget {
    Bin { name: String },
    Bench(BenchSpecifier),
    Test(TestSpecifier),
    Exmaple { name: String },
    Examples,
}

impl CargoTarget {
    pub fn supports_release_flag(&self) -> bool {
        match self {
            CargoTarget::Bench(_) => false,
            _ => true,
        }
    }
}

/// Compile one or more targets.
pub fn compile(release: bool, target: &CargoTarget) -> Result<Vec<BinFile>, Error> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());

    let mut cmd = Command::new(&cargo);

    match target {
        CargoTarget::Bin { name } => {
            cmd.arg("build").arg("--bin").arg(name);
        }
        CargoTarget::Bench(kind) => {
            cmd.arg("bench").arg("--no-run");
            // We forward error message generation to cargo.
            if kind.lib {
                cmd.arg("--lib");
            }

            if let Some(name) = &kind.bench {
                cmd.arg("--bench").arg(name);
            }

            if kind.benches {
                cmd.arg("--benches");
            }
        }
        CargoTarget::Test(kind) => {
            cmd.arg("test").arg("--no-run");

            // We forward error message generation to cargo.
            if kind.lib {
                cmd.arg("--lib");
            }

            if let Some(name) = &kind.test {
                cmd.arg("--test").arg(name);
            }

            if kind.tests {
                cmd.arg("--tests");
            }
        }
        CargoTarget::Exmaple { name } => {
            cmd.arg("build").arg("--example").arg(name);
        }
        CargoTarget::Examples => {
            cmd.arg("build").arg("--examples");
        }
    }

    if release && target.supports_release_flag() {
        cmd.arg("--release");
    }
    cmd.arg("--message-format=json");

    let cmd_str = format!("{:?}", cmd);

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn cargo\n{}", cmd_str))?;

    let reader = BufReader::new(child.stdout.take().unwrap());
    for message in Message::parse_stream(reader) {
        match message.unwrap() {
            Message::CompilerMessage(msg) => {
                println!("{:?}", msg);
            }
            Message::CompilerArtifact(artifact) => {
                println!("{:?}", artifact);
            }
            Message::BuildScriptExecuted(script) => {
                println!("{:?}", script);
            }
            Message::BuildFinished(finished) => {
                println!("{:?}", finished);
            }
            _ => (),
        }
    }

    let _output = child
        .wait()
        .with_context(|| format!("Couldn't get cargo's exit status\n{}", cmd_str))?;

    bail!("cargo did not produced any useful binary\n{}", cmd_str)
}
