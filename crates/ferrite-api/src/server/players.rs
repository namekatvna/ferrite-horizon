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
