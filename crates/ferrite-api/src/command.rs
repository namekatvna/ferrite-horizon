use crate::__private::sdk::command as host;

pub const MAX_RUN_PER_HOOK: usize = 8;
pub const MAX_QUIET_PER_CALL: usize = 256;

pub fn run(line: impl AsRef<str>) {
    host::run(normalize(line.as_ref()))
}

pub fn run_quiet<I, S>(lines: I)
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for batch in batches(lines) {
        host::run_quiet(&batch);
    }
}

fn normalize(line: &str) -> &str {
    let line = line.trim();
    line.strip_prefix('/').unwrap_or(line)
}

fn batches<I, S>(lines: I) -> Vec<Vec<String>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut batches: Vec<Vec<String>> = Vec::new();
    for line in lines {
        let line = normalize(line.as_ref());
        if line.is_empty() {
            continue;
        }
        match batches.last_mut() {
            Some(batch) if batch.len() < MAX_QUIET_PER_CALL => batch.push(line.to_owned()),
            _ => batches.push(vec![line.to_owned()]),
        }
    }
    batches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_slash_and_spaces_are_optional() {
        assert_eq!(normalize("/time set day"), "time set day");
        assert_eq!(normalize("  time set day "), "time set day");
    }

    #[test]
    fn quiet_lines_are_split_into_host_sized_batches() {
        let lines = (0..600).map(|i| format!("/setblock {i} 64 0 stone"));
        let batches = batches(lines);
        assert_eq!(
            batches.iter().map(Vec::len).collect::<Vec<_>>(),
            [256, 256, 88]
        );
        assert_eq!(batches[0][0], "setblock 0 64 0 stone");
    }

    #[test]
    fn blank_lines_are_skipped() {
        assert!(batches(["", "  ", "/"]).is_empty());
    }
}
