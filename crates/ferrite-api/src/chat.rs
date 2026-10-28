use crate::__private::sdk;

pub fn show(text: &str) {
    sdk::chat::show(text)
}
