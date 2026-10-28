use std::path::{Path, PathBuf};

use crate::flag;

const USAGE: &str = "cargo ferrite new <path> [--name <name>] [--ferrite-path <dir>]";
const FERRITE_GIT: &str = "https://github.com/namekatvna/ferrite-horizon";
const RELEASE_PROFILE: &str = "\n[profile.release]\nopt-level = \"s\"\nlto = true\n";

const TEMPLATE: [(&str, &str); 5] = [
    ("Cargo.toml", include_str!("../template/Cargo.toml.in")),
    (
        "horizon-mod.toml",
        include_str!("../template/horizon-mod.toml"),
    ),
    ("src/lib.rs", include_str!("../template/src/lib.rs.in")),
    (
        "tests/hello.toml",
        include_str!("../template/tests/hello.toml"),
    ),
    (".gitignore", include_str!("../template/gitignore")),
];

#[derive(Debug)]
pub struct Created {
    pub id: String,
    pub unlisted_in: Option<PathBuf>,
}

pub fn run(args: &[String]) -> Result<(), String> {
    let path = args.first().filter(|a| !a.starts_with("--")).ok_or(USAGE)?;
    let ferrite_path = flag(args, "--ferrite-path").map(Path::new);
    let created = create(Path::new(path), flag(args, "--name"), ferrite_path)?;
    println!("Created {path} (mod id {})", created.id);
    if let Some(manifest) = created.unlisted_in {
        println!(
            "Note: {} is a workspace; add this mod to its members",
            manifest.display()
        );
    }
    println!("Next: cd {path} && cargo ferrite test");
    Ok(())
}

pub fn create(
    dir: &Path,
    name: Option<&str>,
    ferrite_path: Option<&Path>,
) -> Result<Created, String> {
    if dir.exists() {
        return Err(format!("{} already exists", dir.display()));
    }
    let dir = std::path::absolute(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let base = dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("the path needs a folder name")?;
    let id = mod_id(base)?;
    let name = name.map_or_else(|| base.replace(['-', '_'], " "), str::to_owned);
    let workspace = find_workspace(&dir);

    let ferrite = match (ferrite_path, &workspace) {
        (Some(path), _) => format!("ferrite-api = {{ path = {:?} }}", ferrite_api_dir(path)?),
        (None, Some((_, text))) if has_workspace_dependency(text, "ferrite-api") => {
            "ferrite-api.workspace = true".to_owned()
        }
        (None, _) => format!("ferrite-api = {{ git = {FERRITE_GIT:?} }}"),
    };
    let profile = if workspace.is_some() {
        ""
    } else {
        RELEASE_PROFILE
    };
    let fill = |text: &str| {
        text.replace("\r\n", "\n")
            .replace("%crate%", &id.replace('_', "-"))
            .replace("%id%", &id)
            .replace("%name%", &format!("{name:?}"))
            .replace("%type%", &type_name(&id))
            .replace("%ferrite%", &ferrite)
            .replace("%profile%", profile)
    };

    for (file, text) in TEMPLATE {
        let to = dir.join(file);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&to, fill(text)).map_err(|e| format!("{}: {e}", to.display()))?;
    }

    let unlisted_in = workspace.and_then(|(manifest, text)| {
        let member = dir.strip_prefix(manifest.parent()?).ok()?;
        let member = member.to_string_lossy().replace('\\', "/");
        (!text.contains(&format!("\"{member}\""))).then_some(manifest)
    });
    Ok(Created { id, unlisted_in })
}

fn mod_id(base: &str) -> Result<String, String> {
    let id: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if !id.starts_with(|c: char| c.is_ascii_lowercase()) || id.len() > 64 {
        return Err(format!(
            "mod id `{id}` from `{base}`: it must start with a letter and have at most 64 characters"
        ));
    }
    Ok(id)
}

fn type_name(id: &str) -> String {
    id.split('_')
        .filter(|part| !part.is_empty())
        .flat_map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|c| c.to_ascii_uppercase())
                .into_iter()
                .chain(chars)
        })
        .collect()
}

fn find_workspace(dir: &Path) -> Option<(PathBuf, String)> {
    dir.ancestors().skip(1).find_map(|ancestor| {
        let manifest = ancestor.join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest).ok()?;
        text.lines()
            .any(|line| line.trim() == "[workspace]")
            .then_some((manifest, text))
    })
}

fn has_workspace_dependency(manifest: &str, dependency: &str) -> bool {
    let mut section = "";
    manifest.lines().map(str::trim).any(|line| {
        if line.starts_with('[') {
            section = line;
            return false;
        }
        section == "[workspace.dependencies]"
            && line
                .strip_prefix(dependency)
                .is_some_and(|rest| rest.starts_with([' ', '=', '.']))
    })
}

fn ferrite_api_dir(path: &Path) -> Result<String, String> {
    let path = std::fs::canonicalize(path)
        .map_err(|e| format!("--ferrite-path {}: {e}", path.display()))?;
    let crate_dir = [path.join("crates").join("ferrite-api"), path]
        .into_iter()
        .find(|dir| {
            std::fs::read_to_string(dir.join("Cargo.toml"))
                .is_ok_and(|text| text.contains("name = \"ferrite-api\""))
        })
        .ok_or("--ferrite-path must be the Ferrite repository or its crates/ferrite-api")?;
    let shown = crate_dir.display().to_string();
    Ok(shown
        .strip_prefix(r"\\?\")
        .unwrap_or(&shown)
        .replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cargo-ferrite-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn read(dir: &Path, file: &str) -> String {
        std::fs::read_to_string(dir.join(file)).unwrap()
    }

    #[test]
    fn standalone_mod_uses_git_and_its_own_release_profile() {
        let root = temp("standalone");
        let dir = root.join("My-Mod");
        let created = create(&dir, None, None).unwrap();
        assert_eq!(created.id, "my_mod");
        assert!(created.unlisted_in.is_none());

        let cargo = read(&dir, "Cargo.toml");
        assert!(cargo.contains("name = \"my-mod\""));
        assert!(cargo.contains(&format!("ferrite-api = {{ git = \"{FERRITE_GIT}\" }}")));
        assert!(cargo.ends_with("lto = true\n"));

        let manifest = read(&dir, "horizon-mod.toml");
        assert!(manifest.contains("id = \"my_mod\"\nname = \"My Mod\"\n"));
        let lib = read(&dir, "src/lib.rs");
        assert!(lib.contains("struct MyMod {") && lib.contains("export_mod!(MyMod);"));
        assert!(lib.contains("log::info(\"my_mod: loaded\")"));
        assert!(
            !TEMPLATE
                .iter()
                .any(|(file, _)| read(&dir, file).contains('%'))
        );

        assert!(
            create(&dir, None, None)
                .unwrap_err()
                .contains("already exists")
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn workspace_member_reuses_workspace_dependency() {
        let root = temp("workspace");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"mods/listed\"]\n\n[workspace.dependencies]\nferrite-api = { path = \"x\" }\n",
        )
        .unwrap();

        let listed = create(&root.join("mods/listed"), Some("Listed \"One\""), None).unwrap();
        assert!(listed.unlisted_in.is_none());
        let cargo = read(&root.join("mods/listed"), "Cargo.toml");
        assert!(cargo.ends_with("[dependencies]\nferrite-api.workspace = true\n"));
        assert!(
            read(&root.join("mods/listed"), "horizon-mod.toml")
                .contains(r#"name = "Listed \"One\"""#)
        );

        let unlisted = create(&root.join("mods/other"), None, None).unwrap();
        assert_eq!(
            unlisted.unlisted_in,
            Some(std::path::absolute(root.join("Cargo.toml")).unwrap())
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn ferrite_path_points_at_the_api_crate() {
        let root = temp("path");
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        create(&root.join("local"), None, Some(&repo)).unwrap();
        let cargo = read(&root.join("local"), "Cargo.toml");
        assert!(
            cargo.contains("ferrite-api = { path = \"")
                && cargo.contains("/crates/ferrite-api\" }")
        );
        assert!(!cargo.contains('\\'));

        assert!(create(&root.join("bad"), None, Some(&root)).is_err());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn ids_and_type_names() {
        assert_eq!(mod_id("Cool Mod2").unwrap(), "cool_mod2");
        assert!(mod_id("9lives").is_err());
        assert!(mod_id(&"a".repeat(65)).is_err());
        assert_eq!(type_name("cool__mod2"), "CoolMod2");
        assert!(has_workspace_dependency(
            "[workspace.dependencies]\nferrite-api.workspace = true",
            "ferrite-api"
        ));
        assert!(!has_workspace_dependency(
            "[dependencies]\nferrite-api = \"1\"",
            "ferrite-api"
        ));
        assert!(!has_workspace_dependency(
            "[workspace.dependencies]\nferrite-api-extra = \"1\"",
            "ferrite-api"
        ));
    }
}
