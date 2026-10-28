use crate::host;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

pub fn write(level: Level, message: impl AsRef<str>) {
    host::log(level, message.as_ref())
}

pub fn debug(message: impl AsRef<str>) {
    write(Level::Debug, message)
}

pub fn info(message: impl AsRef<str>) {
    write(Level::Info, message)
}

pub fn warn(message: impl AsRef<str>) {
    write(Level::Warn, message)
}

pub fn error(message: impl AsRef<str>) {
    write(Level::Error, message)
}
