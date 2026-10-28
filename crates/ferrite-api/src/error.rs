use std::fmt;

#[derive(Clone, PartialEq, Eq)]
pub struct Error(String);

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    pub fn message(&self) -> &str {
        &self.0
    }

    pub fn into_message(self) -> String {
        self.0
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<E: fmt::Display> From<E> for Error {
    fn from(error: E) -> Self {
        Self(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_from_anything_displayable() {
        let from_str: Error = "bad".into();
        let from_string: Error = String::from("worse").into();
        let from_parse: Error = "x".parse::<u32>().unwrap_err().into();

        assert_eq!(from_str.message(), "bad");
        assert_eq!(from_string.into_message(), "worse");
        assert_eq!(from_parse.message(), "invalid digit found in string");
    }

    #[test]
    fn question_mark_works_in_functions_returning_error() {
        fn parse(text: &str) -> Result<u8, Error> {
            Ok(text.parse::<u8>()?)
        }

        assert_eq!(parse("7"), Ok(7));
        assert!(parse("300").is_err());
    }
}
