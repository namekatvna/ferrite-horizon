use crate::__private::sdk;

pub fn info(message: &str) {
    sdk::info(message)
}

pub fn warn(message: &str) {
    sdk::warn(message)
}
