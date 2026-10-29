use std::collections::HashMap;

use crate::host;
use crate::server::Player;

pub const MAX_MESSAGES_PER_CALL: usize = 8;

pub fn online() -> Vec<Player> {
    host::server_players()
}

pub fn find(name: &str) -> Option<Player> {
    online()
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
}

pub fn tell(player: &Player, text: impl AsRef<str>) -> bool {
    host::server_tell(&player.id, text.as_ref())
}

pub fn broadcast(text: impl AsRef<str>) {
    host::server_broadcast(text.as_ref());
}

#[derive(Clone, Debug)]
pub struct PlayerMap<T> {
    entries: HashMap<String, T>,
}

impl<T> Default for PlayerMap<T> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<T> PlayerMap<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, player: &Player) -> Option<&T> {
        self.entries.get(&player.id)
    }

    pub fn get_mut(&mut self, player: &Player) -> Option<&mut T> {
        self.entries.get_mut(&player.id)
    }

    pub fn get_or_default(&mut self, player: &Player) -> &mut T
    where
        T: Default,
    {
        self.entries.entry(player.id.clone()).or_default()
    }

    pub fn insert(&mut self, player: &Player, value: T) -> Option<T> {
        self.entries.insert(player.id.clone(), value)
    }

    pub fn remove(&mut self, player: &Player) -> Option<T> {
        self.entries.remove(&player.id)
    }

    pub fn contains(&self, player: &Player) -> bool {
        self.entries.contains_key(&player.id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.entries.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.entries.values_mut()
    }

    pub fn retain_online(&mut self) {
        let online = online();
        self.entries
            .retain(|id, _| online.iter().any(|player| player.id == *id));
    }
}
