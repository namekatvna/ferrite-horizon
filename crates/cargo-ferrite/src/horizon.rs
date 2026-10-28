use std::io::ErrorKind;
use std::process::{Command, ExitCode, Stdio};

const TOOL: &str = "cargo-horizon-mod";
const TARGET: &str = "wasm32-unknown-unknown";
const UPSTREAM: &str = include_str!("../../ferrite-api/wit/UPSTREAM");

pub fn delegate(args: &[String]) -> Result<ExitCode, String> {
    match Command::new(TOOL).arg("horizon-mod").args(args).status() {
        Ok(status) => Ok(status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .map_or(ExitCode::FAILURE, ExitCode::from)),
        Err(e) if e.kind() == ErrorKind::NotFound => Err(format!(
            "{TOOL} is not installed. Install it with:\n  {}",
            install_command()
        )),
        Err(e) => Err(format!("{TOOL}: {e}")),
    }
}

pub fn doctor() -> ExitCode {
    let checks = [
        (
            format!("target {TARGET}"),
            target_installed(),
            format!("rustup target add {TARGET}"),
        ),
        (TOOL.to_owned(), tool_installed(), install_command()),
    ];
    let mut ready = true;
    for (what, ok, fix) in checks {
        if ok {
            println!("ok       {what}");
        } else {
            ready = false;
            println!("missing  {what}\n         fix: {fix}");
        }
    }
    if ready {
        println!(
            "Ready to build Ferrite mods (Horizon API rev {}).",
            upstream_rev()
        );
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn target_installed() -> bool {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .any(|l| l.trim() == TARGET)
        })
}

fn tool_installed() -> bool {
    Command::new(TOOL)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn upstream_rev() -> &'static str {
    UPSTREAM
        .split("/tree/")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or("unknown")
}

fn install_command() -> String {
    format!(
        "cargo install --git https://github.com/horizon-rust/horizon --rev {} {TOOL}",
        upstream_rev()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_pins_the_vendored_wit_revision() {
        assert_eq!(upstream_rev().len(), 40);
        assert!(install_command().contains(&format!("--rev {} {TOOL}", upstream_rev())));
    }
}
