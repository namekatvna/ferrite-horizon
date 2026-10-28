use crate::host;

pub fn show(text: impl AsRef<str>) {
    host::chat_show(text.as_ref())
}
