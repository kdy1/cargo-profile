use std::process::Command;

pub fn command(root: bool, cmd: &str) -> Command {
    if root {
        let mut c = Command::new("sudo");
        c.arg(cmd);
        c
    } else {
        Command::new(cmd)
    }
}
