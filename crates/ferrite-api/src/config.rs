use std::fmt;
use std::ops::Deref;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::storage::{self, Backend, Host, StorageError};

pub struct Config<T> {
    key: String,
    value: T,
}

impl<T: Serialize + DeserializeOwned + Default> Config<T> {
    pub const DEFAULT_NAME: &'static str = "config";

    pub fn load() -> Self {
        Self::load_named(Self::DEFAULT_NAME)
    }

    pub fn load_named(name: &str) -> Self {
        Self::load_in(&Host, name)
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn save(&self) -> Result<(), StorageError> {
        self.save_in(&Host)
    }

    pub fn update<R>(&mut self, change: impl FnOnce(&mut T) -> R) -> Result<R, StorageError> {
        let result = change(&mut self.value);
        self.save()?;
        Ok(result)
    }

    pub fn edit<R>(&mut self, change: impl FnOnce(&mut T) -> R) -> R {
        let result = change(&mut self.value);
        if let Err(error) = self.save() {
            Host.warn(&format!("config: could not save {}: {error}", self.key));
        }
        result
    }

    pub fn reset(&mut self) -> Result<(), StorageError> {
        self.value = T::default();
        self.save()
    }

    pub fn reload(&mut self) {
        self.value = read(&Host, &self.key);
    }

    pub fn into_inner(self) -> T {
        self.value
    }

    pub(crate) fn load_in(backend: &impl Backend, name: &str) -> Self {
        let key = format!("{name}.json");
        let value = read(backend, &key);
        Self { key, value }
    }

    pub(crate) fn save_in(&self, backend: &impl Backend) -> Result<(), StorageError> {
        write(backend, &self.key, &self.value)
    }
}

impl<T> Deref for Config<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: fmt::Debug> fmt::Debug for Config<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("key", &self.key)
            .field("value", &self.value)
            .finish()
    }
}

fn read<T: Serialize + DeserializeOwned + Default>(backend: &impl Backend, key: &str) -> T {
    let stored = match storage::get_in(backend, key) {
        Ok(stored) => stored,
        Err(error) => {
            backend.warn(&format!("config: {error}; using defaults"));
            return T::default();
        }
    };
    let Some(bytes) = stored else {
        let value = T::default();
        if let Err(error) = write(backend, key, &value) {
            backend.warn(&format!("config: could not create {key}: {error}"));
        }
        return value;
    };
    match resolve::<T>(&bytes) {
        Ok((value, outdated)) => {
            if outdated && let Err(error) = write(backend, key, &value) {
                backend.warn(&format!("config: could not update {key}: {error}"));
            }
            value
        }
        Err(error) => {
            backend.warn(&format!(
                "config: {key} is broken ({error}); using defaults until it is fixed"
            ));
            T::default()
        }
    }
}

fn resolve<T: Serialize + DeserializeOwned + Default>(bytes: &[u8]) -> Result<(T, bool), String> {
    let stored: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let mut merged = serde_json::to_value(T::default()).map_err(|e| e.to_string())?;
    merge(&mut merged, stored.clone());
    let value: T = serde_json::from_value(merged).map_err(|e| e.to_string())?;
    let normalized = serde_json::to_value(&value).map_err(|e| e.to_string())?;
    Ok((value, normalized != stored))
}

fn merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base), Value::Object(overlay)) => {
            for (key, value) in overlay {
                match base.get_mut(&key) {
                    Some(slot) => merge(slot, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        }
        (slot, value) => *slot = value,
    }
}

fn write<T: Serialize>(backend: &impl Backend, key: &str, value: &T) -> Result<(), StorageError> {
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|e| StorageError::Encode(e.to_string()))?;
    storage::set_in(backend, key, &bytes)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::Deserialize;

    use super::*;
    use crate::storage::memory::Memory;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Settings {
        radius: u32,
        visible: bool,
        homes: BTreeMap<String, i32>,
    }

    impl Default for Settings {
        fn default() -> Self {
            Self {
                radius: 16,
                visible: true,
                homes: BTreeMap::new(),
            }
        }
    }

    #[test]
    fn missing_config_is_created_with_defaults() {
        let memory = Memory::default();
        let config = Config::<Settings>::load_in(&memory, "config");

        assert_eq!(*config.get(), Settings::default());
        assert_eq!(config.key(), "config.json");
        let text = memory.text("config.json").unwrap();
        assert!(text.contains("\"radius\": 16"), "{text}");
    }

    #[test]
    fn new_fields_are_filled_in_and_written_back() {
        let memory = Memory::default();
        memory.put("config.json", r#"{ "radius": 4 }"#);

        let config = Config::<Settings>::load_in(&memory, "config");

        assert_eq!(config.radius, 4);
        assert!(config.visible);
        assert!(
            memory
                .text("config.json")
                .unwrap()
                .contains("\"visible\": true")
        );
    }

    #[test]
    fn map_entries_and_player_edits_survive() {
        let memory = Memory::default();
        memory.put(
            "config.json",
            r#"{ "radius": 2, "visible": false, "homes": { "base": 64 } }"#,
        );
        let before = memory.text("config.json");

        let config = Config::<Settings>::load_in(&memory, "config");

        assert_eq!(config.homes["base"], 64);
        assert!(!config.visible);
        assert_eq!(memory.text("config.json"), before);
    }

    #[test]
    fn removed_fields_are_dropped() {
        let memory = Memory::default();
        memory.put(
            "config.json",
            r#"{ "radius": 2, "visible": true, "homes": {}, "old": 1 }"#,
        );

        Config::<Settings>::load_in(&memory, "config");

        assert!(!memory.text("config.json").unwrap().contains("old"));
    }

    #[test]
    fn broken_config_falls_back_without_overwriting() {
        let memory = Memory::default();
        memory.put("config.json", r#"{ "radius": "far" "#);

        let config = Config::<Settings>::load_in(&memory, "config");

        assert_eq!(*config.get(), Settings::default());
        assert_eq!(memory.text("config.json").unwrap(), r#"{ "radius": "far" "#);
        assert_eq!(memory.warnings.borrow().len(), 1);
    }

    #[test]
    fn wrong_types_fall_back_without_overwriting() {
        let memory = Memory::default();
        memory.put("config.json", r#"{ "radius": "far" }"#);

        let config = Config::<Settings>::load_in(&memory, "config");

        assert_eq!(config.radius, 16);
        assert_eq!(
            memory.text("config.json").unwrap(),
            r#"{ "radius": "far" }"#
        );
    }

    #[test]
    fn invalid_names_warn_and_use_defaults() {
        let memory = Memory::default();
        let config = Config::<Settings>::load_in(&memory, "My Config");

        assert_eq!(*config.get(), Settings::default());
        assert!(memory.files.borrow().is_empty());
        assert!(memory.warnings.borrow()[0].contains("invalid storage key"));
    }

    #[test]
    fn saved_changes_are_read_back() {
        let memory = Memory::default();
        let mut config = Config::<Settings>::load_in(&memory, "config");
        config.value.radius = 99;
        config.save_in(&memory).unwrap();

        assert_eq!(Config::<Settings>::load_in(&memory, "config").radius, 99);
    }
}
