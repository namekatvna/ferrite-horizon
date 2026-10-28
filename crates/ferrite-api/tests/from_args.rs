use ferrite_api::hud::Corner;
use ferrite_api::prelude::*;

#[derive(Debug, PartialEq, FromArgs)]
enum Waypoint {
    #[default]
    List,
    Add {
        name: String,
        x: i32,
        y: i32,
        z: i32,
    },
    Remove(String),
    SetColor {
        name: String,
        color: Option<u32>,
    },
    Note {
        name: String,
        #[rest]
        text: String,
    },
}

#[derive(Debug, PartialEq, FromArgs)]
struct Teleport {
    x: f64,
    y: f64,
    z: f64,
    #[rest]
    reason: Option<String>,
}

#[derive(Debug, PartialEq, FromArgs)]
enum Strict {
    Move { corner: Corner },
}

fn parse<T: FromArgs>(line: &str) -> Result<T, String> {
    Args::for_command("wp", line)
        .parse()
        .map_err(Error::into_message)
}

#[test]
fn usage_is_generated_from_the_type() {
    assert_eq!(
        Waypoint::USAGE,
        "list | add <name> <x> <y> <z> | remove <arg1> | set-color <name> [color] | note <name> <text...>"
    );
    assert_eq!(Teleport::USAGE, "<x> <y> <z> [reason...]");
}

#[test]
fn variants_parse_with_named_unnamed_optional_and_rest_fields() {
    assert_eq!(parse(""), Ok(Waypoint::List));
    assert_eq!(parse("LIST"), Ok(Waypoint::List));
    assert_eq!(
        parse("add home -10 64 300"),
        Ok(Waypoint::Add {
            name: "home".into(),
            x: -10,
            y: 64,
            z: 300
        })
    );
    assert_eq!(parse("remove home"), Ok(Waypoint::Remove("home".into())));
    assert_eq!(
        parse("set-color home"),
        Ok(Waypoint::SetColor {
            name: "home".into(),
            color: None
        })
    );
    assert_eq!(
        parse("set-color home 255"),
        Ok(Waypoint::SetColor {
            name: "home".into(),
            color: Some(255)
        })
    );
    assert_eq!(
        parse("note home   the   big   tree "),
        Ok(Waypoint::Note {
            name: "home".into(),
            text: "the   big   tree".into()
        })
    );
}

#[test]
fn structs_parse_positionally() {
    assert_eq!(
        parse("1.5 64 -3"),
        Ok(Teleport {
            x: 1.5,
            y: 64.0,
            z: -3.0,
            reason: None
        })
    );
    assert_eq!(
        parse::<Teleport>("1 2 3 back to base").map(|t| t.reason),
        Ok(Some("back to base".into()))
    );
}

#[test]
fn errors_name_the_argument_and_show_usage() {
    assert_eq!(
        parse::<Waypoint>("add home 1 2"),
        Err("Missing <z>. Usage: /wp add <name> <x> <y> <z>".into())
    );
    assert_eq!(
        parse::<Waypoint>("add home 1 two 3"),
        Err("Invalid <y> `two`: invalid digit found in string".into())
    );
    assert_eq!(
        parse::<Waypoint>("fly"),
        Err(format!(
            "Unknown option `fly`. Usage: /wp {}",
            Waypoint::USAGE
        ))
    );
    assert_eq!(
        parse::<Waypoint>("remove home now"),
        Err("Unexpected `now`. Usage: /wp remove <arg1>".into())
    );
    assert_eq!(
        parse::<Waypoint>("note home"),
        Err("Missing <text>. Usage: /wp note <name> <text...>".into())
    );
    assert_eq!(
        parse::<Strict>(""),
        Err("Missing option. Usage: /wp move <corner>".into())
    );
    assert_eq!(
        parse::<Strict>("move middle"),
        Err(
            "Invalid <corner> `middle`: expected top-left, top-right, bottom-left or bottom-right"
                .into()
        )
    );
}

#[test]
fn without_a_command_name_usage_has_no_slash() {
    assert_eq!(
        Args::new("1 2").parse::<Teleport>().unwrap_err().message(),
        "Missing <z>. Usage: <x> <y> <z> [reason...]"
    );
}

#[test]
fn args_know_their_command() {
    assert_eq!(Args::for_command("wp", " list ").command(), "wp");
    assert_eq!(Args::new("list").command(), "");
}
