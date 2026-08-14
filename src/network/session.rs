use bevy::prelude::*;

// Only referenced from the test below (which asserts it's untouched); importing it
// unconditionally would warn as unused in a non-test build.
#[cfg(test)]
use crate::network::login::CharacterList;
use crate::network::systems::{ConnectionState, LoginCredentials, LogoutRequested};

/// Drops everything tied to the TCP session that just ended.
///
/// `CharacterList` deliberately survives: the account is still logged in, and the
/// client returns to the character list rather than the login form. `ConnectionState`
/// has normally been removed already by `on_connection_lost_cleanup`; removing it
/// again is a no-op and keeps this system correct on its own.
pub(super) fn cleanup_session(mut commands: Commands) {
    commands.remove_resource::<ConnectionState>();
    commands.remove_resource::<LoginCredentials>();
    commands.remove_resource::<LogoutRequested>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// The auth token minted for the character we just left is useless — a fresh
    /// one is issued per character selection. Leaving it behind would let a later
    /// `connect()` reuse a dead token.
    #[test]
    fn cleanup_drops_the_credentials() {
        let mut world = World::new();
        world.insert_resource(LoginCredentials {
            auth_token: "stale".to_string(),
        });

        world.run_system_once(cleanup_session).unwrap();

        assert!(world.get_resource::<LoginCredentials>().is_none());
    }

    /// The account session is what lets the player come straight back to the
    /// character list instead of retyping their password.
    #[test]
    fn cleanup_keeps_the_account_session() {
        let mut world = World::new();
        world.insert_resource(CharacterList {
            characters: Vec::new(),
            session_token: "session".to_string(),
        });

        world.run_system_once(cleanup_session).unwrap();

        assert!(world.get_resource::<CharacterList>().is_some());
    }
}
