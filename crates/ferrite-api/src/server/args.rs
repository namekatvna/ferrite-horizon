use crate::Error;
use crate::game::BlockPos;
use crate::server::{BlockState, Player};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentKind {
    Word,
    Text,
    Integer { min: Option<i32>, max: Option<i32> },
    Number,
    Boolean,
    Player,
    BlockPos,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArgumentSpec {
    pub name: String,
    pub kind: ArgumentKind,
    pub optional: bool,
}

impl ArgumentSpec {
    pub fn new(name: impl Into<String>, kind: ArgumentKind, optional: bool) -> Self {
        Self {
            name: name.into(),
            kind,
            optional,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArgumentValue {
    Word(String),
    Text(String),
    Integer(i32),
    Number(f64),
    Boolean(bool),
    Player(Player),
    BlockPos(BlockPos),
}

#[derive(Clone, Copy, Debug)]
pub struct Values<'a> {
    values: &'a [(String, ArgumentValue)],
}

impl<'a> Values<'a> {
    pub fn new(values: &'a [(String, ArgumentValue)]) -> Self {
        Self { values }
    }

    pub fn get(&self, name: &str) -> Option<&'a ArgumentValue> {
        self.values.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    pub fn required<T: ServerArgument>(&self, name: &str) -> Result<T, Error> {
        self.optional(name)?
            .ok_or_else(|| Error::new(format!("Missing <{name}>")))
    }

    pub fn optional<T: ServerArgument>(&self, name: &str) -> Result<Option<T>, Error> {
        self.get(name)
            .map(|value| {
                T::from_value(value).ok_or_else(|| Error::new(format!("Invalid <{name}>")))
            })
            .transpose()
    }
}

pub trait ServerArgument: Sized {
    fn kind() -> ArgumentKind;
    fn from_value(value: &ArgumentValue) -> Option<Self>;
}

macro_rules! integers {
    ($($t:ty),*) => {$(
        impl ServerArgument for $t {
            fn kind() -> ArgumentKind {
                ArgumentKind::Integer {
                    min: i32::try_from(<$t>::MIN).ok(),
                    max: i32::try_from(<$t>::MAX).ok(),
                }
            }

            fn from_value(value: &ArgumentValue) -> Option<Self> {
                match value {
                    ArgumentValue::Integer(n) => <$t>::try_from(*n).ok(),
                    _ => None,
                }
            }
        }
    )*};
}

integers!(i8, i16, i32, u8, u16, u32);

impl ServerArgument for f64 {
    fn kind() -> ArgumentKind {
        ArgumentKind::Number
    }

    fn from_value(value: &ArgumentValue) -> Option<Self> {
        match value {
            ArgumentValue::Number(n) => Some(*n),
            ArgumentValue::Integer(n) => Some(f64::from(*n)),
            _ => None,
        }
    }
}

impl ServerArgument for f32 {
    fn kind() -> ArgumentKind {
        ArgumentKind::Number
    }

    fn from_value(value: &ArgumentValue) -> Option<Self> {
        f64::from_value(value).map(|n| n as f32)
    }
}

impl ServerArgument for bool {
    fn kind() -> ArgumentKind {
        ArgumentKind::Boolean
    }

    fn from_value(value: &ArgumentValue) -> Option<Self> {
        match value {
            ArgumentValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

impl ServerArgument for String {
    fn kind() -> ArgumentKind {
        ArgumentKind::Word
    }

    fn from_value(value: &ArgumentValue) -> Option<Self> {
        match value {
            ArgumentValue::Word(s) | ArgumentValue::Text(s) => Some(s.clone()),
            _ => None,
        }
    }
}

impl ServerArgument for BlockState {
    fn kind() -> ArgumentKind {
        ArgumentKind::Word
    }

    fn from_value(value: &ArgumentValue) -> Option<Self> {
        String::from_value(value).map(BlockState::new)
    }
}

impl ServerArgument for BlockPos {
    fn kind() -> ArgumentKind {
        ArgumentKind::BlockPos
    }

    fn from_value(value: &ArgumentValue) -> Option<Self> {
        match value {
            ArgumentValue::BlockPos(pos) => Some(*pos),
            _ => None,
        }
    }
}

impl ServerArgument for Player {
    fn kind() -> ArgumentKind {
        ArgumentKind::Player
    }

    fn from_value(value: &ArgumentValue) -> Option<Self> {
        match value {
            ArgumentValue::Player(player) => Some(player.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_kinds_carry_the_type_bounds() {
        assert_eq!(
            u8::kind(),
            ArgumentKind::Integer {
                min: Some(0),
                max: Some(255)
            }
        );
        assert_eq!(
            u32::kind(),
            ArgumentKind::Integer {
                min: Some(0),
                max: None
            }
        );
        assert_eq!(u8::from_value(&ArgumentValue::Integer(300)), None);
    }

    #[test]
    fn values_are_read_by_name() {
        let values = [
            ("height".to_owned(), ArgumentValue::Integer(4)),
            ("block".to_owned(), ArgumentValue::Word("stone".into())),
        ];
        let values = Values::new(&values);
        assert_eq!(values.required::<u8>("height"), Ok(4));
        assert_eq!(
            values.required::<BlockState>("block").map(String::from),
            Ok("minecraft:stone".to_owned())
        );
        assert_eq!(values.optional::<bool>("loud"), Ok(None));
        assert!(values.required::<bool>("height").is_err());
    }
}
