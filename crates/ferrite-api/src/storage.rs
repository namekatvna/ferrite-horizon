use std::fmt;

use crate::host;

pub const MAX_KEY_LEN: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageError {
    InvalidKey(String),
    Rejected(String),
    Encode(String),
    Decode(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey(key) => write!(
                f,
                "invalid storage key `{key}`: use 1-{MAX_KEY_LEN} of a-z 0-9 _ . -"
            ),
            Self::Rejected(reason) => write!(f, "storage refused the write: {reason}"),
            Self::Encode(reason) => write!(f, "could not encode the value: {reason}"),
            Self::Decode(reason) => write!(f, "could not decode the stored value: {reason}"),
        }
    }
}

impl std::error::Error for StorageError {}

pub fn is_valid_key(key: &str) -> bool {
    (1..=MAX_KEY_LEN).contains(&key.len())
        && key
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-'))
}

pub fn get(key: &str) -> Result<Option<Vec<u8>>, StorageError> {
    get_in(&Host, key)
}

pub fn set(key: &str, value: &[u8]) -> Result<(), StorageError> {
    set_in(&Host, key, value)
}

pub fn remove(key: &str) -> Result<(), StorageError> {
    check(key)?;
    Host.delete(key);
    Ok(())
}

pub fn contains(key: &str) -> Result<bool, StorageError> {
    Ok(get(key)?.is_some())
}

#[cfg(feature = "serde")]
pub fn save<T: serde::Serialize + ?Sized>(key: &str, value: &T) -> Result<(), StorageError> {
    save_in(&Host, key, value)
}

#[cfg(feature = "serde")]
pub fn load<T: serde::de::DeserializeOwned>(key: &str) -> Result<Option<T>, StorageError> {
    load_in(&Host, key)
}

#[cfg(feature = "serde")]
pub fn load_or_default<T: serde::de::DeserializeOwned + Default>(
    key: &str,
) -> Result<T, StorageError> {
    Ok(load(key)?.unwrap_or_default())
}

pub(crate) trait Backend {
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    fn set(&self, key: &str, value: &[u8]) -> Result<(), String>;
    fn delete(&self, key: &str);
    #[cfg(feature = "serde")]
    fn warn(&self, message: &str);
}

pub(crate) struct Host;

impl Backend for Host {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        host::storage_get(key)
    }

    fn set(&self, key: &str, value: &[u8]) -> Result<(), String> {
        host::storage_set(key, value)
    }

    fn delete(&self, key: &str) {
        host::storage_delete(key)
    }

    #[cfg(feature = "serde")]
    fn warn(&self, message: &str) {
        crate::log::warn(message)
    }
}

pub(crate) fn check(key: &str) -> Result<(), StorageError> {
    if is_valid_key(key) {
        Ok(())
    } else {
        Err(StorageError::InvalidKey(key.to_owned()))
    }
}

pub(crate) fn get_in(backend: &impl Backend, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
    check(key)?;
    Ok(backend.get(key))
}

pub(crate) fn set_in(backend: &impl Backend, key: &str, value: &[u8]) -> Result<(), StorageError> {
    check(key)?;
    backend.set(key, value).map_err(StorageError::Rejected)
}

#[cfg(feature = "serde")]
pub(crate) fn save_in<T: serde::Serialize + ?Sized>(
    backend: &impl Backend,
    key: &str,
    value: &T,
) -> Result<(), StorageError> {
    let bytes = postcard::to_allocvec(value).map_err(|e| StorageError::Encode(e.to_string()))?;
    set_in(backend, key, &bytes)
}

#[cfg(feature = "serde")]
pub(crate) fn load_in<T: serde::de::DeserializeOwned>(
    backend: &impl Backend,
    key: &str,
) -> Result<Option<T>, StorageError> {
    get_in(backend, key)?
        .map(|bytes| postcard::from_bytes(&bytes).map_err(|e| StorageError::Decode(e.to_string())))
        .transpose()
}

#[cfg(test)]
pub(crate) mod memory {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::Backend;

    #[derive(Default)]
    pub struct Memory {
        pub files: RefCell<BTreeMap<String, Vec<u8>>>,
        #[cfg(feature = "serde")]
        pub warnings: RefCell<Vec<String>>,
        pub quota: Option<usize>,
    }

    #[cfg(feature = "serde")]
    impl Memory {
        pub fn text(&self, key: &str) -> Option<String> {
            self.files
                .borrow()
                .get(key)
                .map(|b| String::from_utf8(b.clone()).unwrap())
        }

        pub fn put(&self, key: &str, text: &str) {
            self.files
                .borrow_mut()
                .insert(key.to_owned(), text.as_bytes().to_vec());
        }
    }

    impl Backend for Memory {
        fn get(&self, key: &str) -> Option<Vec<u8>> {
            self.files.borrow().get(key).cloned()
        }

        fn set(&self, key: &str, value: &[u8]) -> Result<(), String> {
            if self.quota.is_some_and(|q| value.len() > q) {
                return Err("over quota".into());
            }
            self.files
                .borrow_mut()
                .insert(key.to_owned(), value.to_vec());
            Ok(())
        }

        fn delete(&self, key: &str) {
            self.files.borrow_mut().remove(key);
        }

        #[cfg(feature = "serde")]
        fn warn(&self, message: &str) {
            self.warnings.borrow_mut().push(message.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::memory::Memory;
    use super::*;

    #[test]
    fn keys_follow_the_host_rules() {
        for good in ["a", "home", "player_1.dat", "x-y.z", &"k".repeat(64)] {
            assert!(is_valid_key(good), "{good}");
        }
        for bad in ["", "Home", "a/b", "a b", "ключ", &"k".repeat(65)] {
            assert!(!is_valid_key(bad), "{bad}");
        }
    }

    #[test]
    fn invalid_keys_fail_before_reaching_the_host() {
        let memory = Memory::default();
        assert_eq!(
            get_in(&memory, "Bad Key"),
            Err(StorageError::InvalidKey("Bad Key".into()))
        );
        assert!(set_in(&memory, "a/b", b"x").is_err());
        assert!(memory.files.borrow().is_empty());
    }

    #[test]
    fn host_rejections_are_reported() {
        let memory = Memory {
            quota: Some(2),
            ..Memory::default()
        };
        assert_eq!(
            set_in(&memory, "big", b"12345"),
            Err(StorageError::Rejected("over quota".into()))
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn typed_values_round_trip_compactly() {
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        struct Home {
            name: String,
            pos: crate::game::BlockPos,
        }

        let memory = Memory::default();
        let home = Home {
            name: "base".into(),
            pos: crate::game::BlockPos::new(-120, 64, 3000),
        };
        save_in(&memory, "home", &home).unwrap();

        assert!(memory.files.borrow()["home"].len() < 16);
        assert_eq!(load_in::<Home>(&memory, "home"), Ok(Some(home)));
        assert_eq!(load_in::<Home>(&memory, "missing"), Ok(None));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn mismatched_types_are_decode_errors() {
        let memory = Memory::default();
        save_in(&memory, "n", &7u8).unwrap();
        assert!(matches!(
            load_in::<String>(&memory, "n"),
            Err(StorageError::Decode(_))
        ));
    }
}
