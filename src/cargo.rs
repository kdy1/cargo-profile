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
    pub is_bench: bool,
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
pub struct CargoTarget {
    #[structopt(long)]
    lib: bool,

    #[structopt(long)]
    release: bool,

    #[structopt(long)]
    bin: Option<String>,

    #[structopt(long)]
    bench: Option<String>,

    #[structopt(long)]
    benches: bool,

    #[structopt(long)]
    test: Option<String>,

    #[structopt(long)]
    tests: bool,

    #[structopt(long)]
    example: Option<String>,

    #[structopt(long)]
    examples: bool,

    #[structopt(long)]
    features: Option<Vec<String>>,

    /// Arguments passed to the target binary.
    ///
    /// To pass flags, precede child args with `--`,
    /// e.g. `cargo profile subcommand -- -t test1.txt --slow-mode`.
    #[structopt(value_name = "ARGS")]
    target_args: Vec<String>,
}

impl CargoTarget {
    pub fn supports_release_flag(&self) -> bool {
        self.tests || self.test.is_some() || self.examples || self.example.is_some()
    }

    pub fn args(&self) -> &[String] {
        &self.target_args
    }
}

/// Compile one or more targets.
pub fn compile(target: &CargoTarget) -> Result<Vec<BinFile>, Error> {
    let release = target.release;
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());

    let mut is_bench = false;
    let mut cmd = Command::new(&cargo);

    if target.benches || target.bench.is_some() {
        is_bench = true;

        cmd.arg("bench").arg("--no-run");

        if !release {
            cmd.arg("--debug");
        }

        if target.benches {
            cmd.arg("--benches");
        }

        if let Some(target) = &target.bench {
            cmd.arg("--bench").arg(target);
        }
    } else if target.tests || target.test.is_some() {
        cmd.arg("test").arg("--no-run");

        if release {
            cmd.arg("--release");
        }

        if target.tests {
            cmd.arg("--tests");
        }

        if let Some(target) = &target.test {
            cmd.arg("--test").arg(target);
        }
    } else {
        cmd.arg("build");

        if release {
            cmd.arg("--release");
        }

        if target.lib {
            cmd.arg("--lib");
        }

        if target.examples {
            cmd.arg("--examples");
        }

        if let Some(target) = &target.example {
            cmd.arg("--example").arg(target);
        }
    }

    if let Some(features) = &target.features {
        cmd.arg("--features").arg(features.join(","));
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
            Message::CompilerMessage(..) => {}
            Message::CompilerArtifact(mut artifact) => {
                if artifact.target.kind.contains(&"bin".to_string())
                    || artifact.target.kind.contains(&"test".to_string())
                    || artifact.target.kind.contains(&"bench".to_string())
                    || artifact.target.kind.contains(&"example".to_string())
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
                        is_bench,
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
        bail!("cargo did not produce any useful binary\n{}", cmd_str)
    }

    binaries.sort_by_key(|b| b.path.clone());

    Ok(binaries)
}

pub fn cargo_workspace() -> Result<PathBuf, Error> {
    let md = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .context("cargo metadata failed")?;

    Ok(md.workspace_root)
}
