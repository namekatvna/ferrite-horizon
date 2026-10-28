use crate::__private::sdk::log::{Level, write};

pub fn debug(message: impl AsRef<str>) {
    write(Level::Debug, message.as_ref())
}

pub fn info(message: impl AsRef<str>) {
    write(Level::Info, message.as_ref())
}

pub fn warn(message: impl AsRef<str>) {
    write(Level::Warn, message.as_ref())
}

pub fn error(message: impl AsRef<str>) {
    write(Level::Error, message.as_ref())
}
