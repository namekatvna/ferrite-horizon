use std::fmt::Display;
use std::str::FromStr;

use crate::Error;
use crate::game::BlockPos;

pub type CommandResult = Result<Option<String>, Error>;

pub fn reply(text: impl Into<String>) -> CommandResult {
    Ok(Some(text.into()))
}

pub fn no_reply() -> CommandResult {
    Ok(None)
}

pub trait FromArgs: Sized {
    const USAGE: &'static str;

    fn from_args(cursor: &mut ArgCursor<'_>) -> Result<Self, Error>;

    #[doc(hidden)]
    fn server_arguments() -> Option<Vec<crate::server::ArgumentSpec>> {
        None
    }

    #[doc(hidden)]
    fn from_server(_values: &crate::server::Values<'_>) -> Result<Self, Error> {
        Err(Error::new("this command has no typed server arguments"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Args<'a> {
    command: &'a str,
    raw: &'a str,
}

impl<'a> Args<'a> {
    pub fn new(raw: &'a str) -> Self {
        Self::for_command("", raw)
    }

    pub fn for_command(command: &'a str, raw: &'a str) -> Self {
        Self {
            command,
            raw: raw.trim(),
        }
    }

    pub fn command(&self) -> &'a str {
        self.command
    }

    pub fn parse<T: FromArgs>(&self) -> Result<T, Error> {
        let mut cursor = ArgCursor::new(self.command, self.raw);
        let value = T::from_args(&mut cursor)?;
        cursor.finish()?;
        Ok(value)
    }

    pub fn as_str(&self) -> &'a str {
        self.raw
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    pub fn len(&self) -> usize {
        self.words().count()
    }

    pub fn words(&self) -> impl Iterator<Item = &'a str> + 'a {
        self.raw.split_whitespace()
    }

    pub fn word(&self, index: usize) -> Option<&'a str> {
        self.words().nth(index)
    }

    pub fn rest(&self, index: usize) -> &'a str {
        let mut rest = self.raw;
        for _ in 0..index {
            rest = rest.trim_start();
            match rest.find(char::is_whitespace) {
                Some(end) => rest = &rest[end..],
                None => return "",
            }
        }
        rest.trim()
    }

    pub fn get<T>(&self, index: usize) -> Result<T, Error>
    where
        T: FromStr,
        T::Err: Display,
    {
        let word = self
            .word(index)
            .ok_or_else(|| Error::new(format!("Missing argument #{}", index + 1)))?;
        parse(word, index)
    }

    pub fn get_or<T>(&self, index: usize, default: T) -> Result<T, Error>
    where
        T: FromStr,
        T::Err: Display,
    {
        match self.word(index) {
            Some(word) => parse(word, index),
            None => Ok(default),
        }
    }
}

fn parse<T>(word: &str, index: usize) -> Result<T, Error>
where
    T: FromStr,
    T::Err: Display,
{
    word.parse().map_err(|reason| {
        Error::new(format!(
            "Invalid argument #{} `{word}`: {reason}",
            index + 1
        ))
    })
}

#[derive(Debug)]
pub struct ArgCursor<'a> {
    command: &'a str,
    rest: &'a str,
    usage: &'static str,
}

impl<'a> ArgCursor<'a> {
    fn new(command: &'a str, raw: &'a str) -> Self {
        Self {
            command,
            rest: raw.trim(),
            usage: "",
        }
    }

    pub fn set_usage(&mut self, usage: &'static str) {
        self.usage = usage;
    }

    pub fn usage(&self) -> String {
        let line = match self.command {
            "" => self.usage.to_owned(),
            command => format!("/{command} {}", self.usage),
        };
        format!("Usage: {}", line.trim_end())
    }

    pub fn next_word(&mut self) -> Option<&'a str> {
        let rest = self.rest.trim_start();
        if rest.is_empty() {
            self.rest = rest;
            return None;
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let (word, tail) = rest.split_at(end);
        self.rest = tail;
        Some(word)
    }

    pub fn required<T>(&mut self, name: &str) -> Result<T, Error>
    where
        T: FromStr,
        T::Err: Display,
    {
        match self.next_word() {
            Some(word) => parse_named(word, name),
            None => Err(self.missing_argument(name)),
        }
    }

    pub fn optional<T>(&mut self, name: &str) -> Result<Option<T>, Error>
    where
        T: FromStr,
        T::Err: Display,
    {
        self.next_word()
            .map(|word| parse_named(word, name))
            .transpose()
    }

    pub fn required_rest<T>(&mut self, name: &str) -> Result<T, Error>
    where
        T: FromStr,
        T::Err: Display,
    {
        match self.take_rest() {
            "" => Err(self.missing_argument(name)),
            rest => parse_named(rest, name),
        }
    }

    pub fn optional_rest<T>(&mut self, name: &str) -> Result<Option<T>, Error>
    where
        T: FromStr,
        T::Err: Display,
    {
        match self.take_rest() {
            "" => Ok(None),
            rest => parse_named(rest, name).map(Some),
        }
    }

    pub fn required_block_pos(&mut self, name: &str) -> Result<BlockPos, Error> {
        match self.optional_block_pos(name)? {
            Some(pos) => Ok(pos),
            None => Err(self.missing_argument(name)),
        }
    }

    pub fn optional_block_pos(&mut self, name: &str) -> Result<Option<BlockPos>, Error> {
        let Some(x) = self.next_word() else {
            return Ok(None);
        };
        let (Some(y), Some(z)) = (self.next_word(), self.next_word()) else {
            return Err(self.missing_argument(name));
        };
        let coordinate = |word: &str| {
            word.parse::<i32>().map_err(|_| {
                Error::new(format!(
                    "Invalid <{name}> `{x} {y} {z}`: expected three whole numbers"
                ))
            })
        };
        Ok(Some(BlockPos::new(
            coordinate(x)?,
            coordinate(y)?,
            coordinate(z)?,
        )))
    }

    pub fn missing_argument(&self, name: &str) -> Error {
        Error::new(format!("Missing <{name}>. {}", self.usage()))
    }

    pub fn missing_option(&self) -> Error {
        Error::new(format!("Missing option. {}", self.usage()))
    }

    pub fn unknown_option(&self, word: &str) -> Error {
        Error::new(format!("Unknown option `{word}`. {}", self.usage()))
    }

    fn take_rest(&mut self) -> &'a str {
        std::mem::take(&mut self.rest).trim()
    }

    fn finish(&mut self) -> Result<(), Error> {
        match self.next_word() {
            Some(word) => Err(Error::new(format!("Unexpected `{word}`. {}", self.usage()))),
            None => Ok(()),
        }
    }
}

fn parse_named<T>(text: &str, name: &str) -> Result<T, Error>
where
    T: FromStr,
    T::Err: Display,
{
    text.parse()
        .map_err(|reason| Error::new(format!("Invalid <{name}> `{text}`: {reason}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_words_and_ignores_extra_spaces() {
        let args = Args::new("  set   home  5 ");
        assert_eq!(args.as_str(), "set   home  5");
        assert_eq!(args.len(), 3);
        assert_eq!(args.word(1), Some("home"));
        assert_eq!(args.word(3), None);
        assert!(Args::new("   ").is_empty());
    }

    #[test]
    fn rest_keeps_inner_spacing() {
        let args = Args::new("say hello   big world");
        assert_eq!(args.rest(0), "say hello   big world");
        assert_eq!(args.rest(1), "hello   big world");
        assert_eq!(args.rest(4), "");
        assert_eq!(args.rest(9), "");
    }

    #[test]
    fn typed_access_reports_position() {
        let args = Args::new("tp 10 x");
        assert_eq!(args.get::<i32>(1), Ok(10));
        assert_eq!(
            args.get::<i32>(2).unwrap_err().message(),
            "Invalid argument #3 `x`: invalid digit found in string"
        );
        assert_eq!(
            args.get::<i32>(3).unwrap_err().message(),
            "Missing argument #4"
        );
    }

    #[test]
    fn optional_arguments_fall_back_only_when_missing() {
        let args = Args::new("radius");
        assert_eq!(args.get_or(1, 16u32), Ok(16));
        assert!(Args::new("radius big").get_or(1, 16u32).is_err());
    }
}
