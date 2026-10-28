mod new;

use std::process::ExitCode;

const USAGE: &str = "\
Usage: cargo ferrite <command>

Commands:
  new <path>     Create a mod project [--name <name>] [--ferrite-path <dir>]";

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "ferrite") {
        args.remove(0);
    }

    let result = match args.first().map(String::as_str) {
        None | Some("help" | "-h" | "--help") => {
            println!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        Some("new") => new::run(&args[1..]).map(|()| ExitCode::SUCCESS),
        Some(cmd) => Err(format!("unknown command `{cmd}`\n\n{USAGE}")),
    };
    result.unwrap_or_else(|e| {
        eprintln!("error: {e}");
        ExitCode::FAILURE
    })
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let i = args.iter().position(|a| a == name)?;
    args.get(i + 1).map(String::as_str)
}
