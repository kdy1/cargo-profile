use self::instruments::profile_target;
use crate::cargo::{cargo_workspace, compile, CargoTarget};
use anyhow::{anyhow, bail, Context, Error};
use std::{path::Path, process::Command};
use structopt::StructOpt;

mod instruments;

/// Profile a crate using xcode instruments.
///
/// This is fork of [cargo-instruments] which uses CLI instead of cargo to build crate.
/// This is required because cargo-instruments uses statically linked cargo and removes the cache for cargo build.
#[derive(Debug, Clone, StructOpt)]
pub struct InstrumentsCommand {
    /// Compile library
    #[structopt(flatten)]
    target: CargoTarget,

    /// List available templates
    #[structopt(short = "l", long)]
    list_templates: bool,

    /// Specify the instruments template to run
    ///
    /// To see available templates, pass `--list-templates`.
    #[structopt(
        short = "t",
        long = "template",
        value_name = "TEMPLATE",
        required_unless = "list-templates"
    )]
    template_name: Option<String>,

    /// Limit recording time to the specified value (in milliseconds)
    ///
    /// The program will be terminated after this limit is exceeded.
    #[structopt(long, value_name = "MILLIS")]
    time_limit: Option<usize>,

    /// Do not open the generated trace file in Instruments.app.
    #[structopt(long)]
    no_open: bool,
}

impl InstrumentsCommand {
    pub fn run(self) -> Result<(), Error> {
        // 1. Detect the type of Xcode Instruments installation
        let xctrace_tool = instruments::XcodeInstruments::detect()?;

        // 2. Render available templates if the user asked
        if self.list_templates {
            let catalog = xctrace_tool.available_templates()?;
            println!("{}", self::instruments::render_template_catalog(&catalog));
            return Ok(());
        }

        // 3. Build the specified target
        let workspace = cargo_workspace()?;
        let binaries = compile(&self.target).context("failed to compile")?;

        if binaries.len() != 1 {
            bail!(
                "This command only supports one binary, but got {:?}",
                binaries
            )
        }

        let target_filepath = compile(&self.target).context("failed to compile")?;
        let target_filepath = if target_filepath.len() == 1 {
            target_filepath.into_iter().next().unwrap()
        } else {
            bail!(
                "This command only supports one binary, but got {:?}",
                target_filepath
            )
        };

        if cfg!(target_arch = "aarch64") {
            codesign(&target_filepath.path)?;
        }

        // 4. Profile the built target, will display menu if no template was selected
        let trace_filepath = profile_target(&target_filepath.path, &xctrace_tool, &self)
            .context("failed to profile built binary")?;

        // 5. Print the trace file's relative path
        {
            let trace_shortpath = trace_filepath
                .strip_prefix(&workspace)
                .unwrap_or_else(|_| trace_filepath.as_path())
                .to_string_lossy();

            eprintln!("Trace file {}", trace_shortpath);
        }

        // 6. Open Xcode Instruments if asked
        if !self.no_open {
            launch_instruments(&trace_filepath)?;
        }

        Ok(())
    }
}

/// Launch Xcode Instruments on the provided trace file.
fn launch_instruments(trace_filepath: &Path) -> Result<(), Error> {
    let status = Command::new("open").arg(trace_filepath).status()?;

    if !status.success() {
        bail!("open failed")
    }
    Ok(())
}

/// On M1 we need to resign with the specified entitlement.
///
/// See https://github.com/cmyr/cargo-instruments/issues/40#issuecomment-894287229
/// for more information.
fn codesign(path: &Path) -> Result<(), Error> {
    static ENTITLEMENTS_FILENAME: &str = "entitlements.plist";
    static ENTITLEMENTS_PLIST_DATA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
    <dict>
        <key>com.apple.security.get-task-allow</key>
        <true/>
    </dict>
</plist>"#;

    let target_dir = path
        .parent()
        .ok_or_else(|| anyhow!("failed to get target directory"))?;
    let entitlement_path = target_dir.join(ENTITLEMENTS_FILENAME);
    std::fs::write(&entitlement_path, ENTITLEMENTS_PLIST_DATA.as_bytes())?;

    let output = Command::new("codesign")
        .args(&["-s", "-", "-f", "--entitlements"])
        .args([&entitlement_path, path])
        .output()?;
    if !output.status.success() {
        let mut msg = String::new();
        if !output.stdout.is_empty() {
            msg = format!("stdout: \"{}\"", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            if !msg.is_empty() {
                msg.push('\n');
            }
            msg.push_str(&format!(
                "stderr: \"{}\"",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        bail!("Code signing failed: {}", msg);
    }
    Ok(())
}
