const USAGE: &str = "\
Usage: cargo ferrite <command>

Commands:
  new <name>   Create a mod project (not implemented yet)
  build        Build the mod as .hmod (not implemented yet)
  run          Build and start Horizon with the mod (not implemented yet)
  test         Run mod test scripts (not implemented yet)";

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .skip_while(|a| a == "ferrite")
        .collect();

    match args.first().map(String::as_str) {
        None | Some("help" | "-h" | "--help") => println!("{USAGE}"),
        Some(cmd) => {
            eprintln!("cargo ferrite: unknown or unimplemented command `{cmd}`\n\n{USAGE}");
            std::process::exit(2);
        }
    }
}
