use std::io::ErrorKind;
use std::process::{Command, ExitCode, Stdio};

const TOOL: &str = "cargo-horizon-mod";
const TARGET: &str = "wasm32-unknown-unknown";
const HORIZON_REV: &str = "cba52f4e5d7f99c592a26823fe8727beafe6ebff";

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
        println!("Ready to build Ferrite mods (Horizon API rev {HORIZON_REV}).");
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

fn install_command() -> String {
    format!(
        "cargo install --git https://github.com/horizon-rust/horizon --rev {HORIZON_REV} {TOOL}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_pins_the_vendored_wit_revision() {
        let upstream = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../ferrite-api/wit/UPSTREAM"
        ))
        .unwrap();
        assert!(
            upstream.contains(&format!("/tree/{HORIZON_REV}/")),
            "update HORIZON_REV to the revision in ferrite-api/wit/UPSTREAM: {upstream}"
        );
        assert!(install_command().contains(&format!("--rev {HORIZON_REV} {TOOL}")));
    }
}
