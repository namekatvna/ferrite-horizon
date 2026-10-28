use crate::__private::sdk;

pub fn show(text: impl AsRef<str>) {
    sdk::chat::show(text.as_ref())
}
