use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use cargo_metadata::ArtifactProfile;
use cargo_metadata::Message;
use is_executable::IsExecutable;
use std::env;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use structopt::StructOpt;

/// Built bin file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BinFile {
    pub path: PathBuf,
    /// `.dSYM`,
    pub extra_files: Vec<PathBuf>,
    pub profile: ArtifactProfile,
}

#[derive(Debug, Clone, StructOpt)]
#[structopt(setting = structopt::clap::AppSettings::TrailingVarArg)]
pub struct TestSpecifier {
    #[structopt(long)]
    pub lib: bool,

    #[structopt(long)]
    pub test: Option<String>,

    #[structopt(long)]
    pub tests: bool,

    args: Vec<String>,
}

#[derive(Debug, Clone, StructOpt)]
#[structopt(setting = structopt::clap::AppSettings::TrailingVarArg)]
pub struct BenchSpecifier {
    #[structopt(long)]
    pub lib: bool,

    #[structopt(long)]
    pub bench: Option<String>,

    #[structopt(long)]
    pub benches: bool,

    args: Vec<String>,
}

#[derive(Debug, Clone, StructOpt)]
#[structopt(setting = structopt::clap::AppSettings::TrailingVarArg)]
pub enum CargoTarget {
    Bin { name: String, args: Vec<String> },
    Bench(BenchSpecifier),
    Test(TestSpecifier),
    Exmaple { name: String, args: Vec<String> },
    Examples { args: Vec<String> },
}

impl CargoTarget {
    pub fn supports_release_flag(&self) -> bool {
        match self {
            CargoTarget::Bench(_) => false,
            _ => true,
        }
    }

    pub fn args(&self) -> &[String] {
        match self {
            CargoTarget::Exmaple { args, .. }
            | CargoTarget::Examples { args, .. }
            | CargoTarget::Bin { args, .. } => &args,
            CargoTarget::Bench(b) => &b.args,
            CargoTarget::Test(t) => &t.args,
        }
    }
}

/// Compile one or more targets.
pub fn compile(release: bool, target: &CargoTarget) -> Result<Vec<BinFile>, Error> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());

    let mut cmd = Command::new(&cargo);

    match target {
        CargoTarget::Bin { name, .. } => {
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
        CargoTarget::Exmaple { name, .. } => {
            cmd.arg("build").arg("--example").arg(name);
        }
        CargoTarget::Examples { .. } => {
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

    let mut binaries = vec![];
    let reader = BufReader::new(child.stdout.take().unwrap());
    for message in Message::parse_stream(reader) {
        match message.unwrap() {
            Message::CompilerMessage(msg) => {
                eprintln!("{}", msg.message.message);
            }
            Message::CompilerArtifact(mut artifact) => {
                if artifact.target.kind.contains(&"bin".to_string())
                    || artifact.target.kind.contains(&"test".to_string())
                    || artifact.target.kind.contains(&"bench".to_string())
                {
                    let mut executable = None;

                    artifact.filenames.retain(|path| {
                        if executable.is_none() {
                            if path.is_executable() {
                                executable = Some(path.clone());
                                return false;
                            }
                        }

                        true
                    });

                    binaries.push(BinFile {
                        path: match executable {
                            Some(v) => v,
                            None => continue,
                        },
                        extra_files: artifact.filenames,
                        profile: artifact.profile,
                    });
                    continue;
                }

                if artifact.target.kind == vec!["lib".to_string()] {
                    continue;
                }
                // println!("{:?}", artifact);
            }
            Message::BuildScriptExecuted(_script) => {
                // eprintln!("Executed build script of `{}`",
                // script.package_id.repr);
            }
            Message::BuildFinished(finished) => {
                if !finished.success {
                    bail!("Failed to compile binary using cargo\n{}", cmd_str)
                }
            }
            _ => (),
        }
    }

    let _output = child
        .wait()
        .with_context(|| format!("Couldn't get cargo's exit status\n{}", cmd_str))?;

    if binaries.is_empty() {
        bail!("cargo did not produced any useful binary\n{}", cmd_str)
    }

    binaries.sort_by_key(|b| b.path.clone());

    Ok(binaries)
}
