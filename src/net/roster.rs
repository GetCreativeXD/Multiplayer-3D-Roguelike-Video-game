//! The host's list of connected players and the IDs it has handed out.
//!
//! Pure bookkeeping with no networking calls, so the ID rules are easy to
//! read and unit test. Only the host has a `PlayerRoster`.

use bevy::prelude::*;

use super::protocol::NetworkPlayerId;

/// Maximum players in one session, including the host.
pub const MAX_PLAYERS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterEntry {
    pub player_id: NetworkPlayerId,
    /// The host-side Lightyear connection entity for this player. For the
    /// host's own player this is the local host-client entity.
    pub connection: Entity,
    pub is_host: bool,
}

#[derive(Resource, Debug, Default)]
pub struct PlayerRoster {
    players: Vec<RosterEntry>,
    /// The last ID handed out. IDs start at 1 and are never reused within
    /// a session, so a reconnecting player can't be confused with whoever
    /// held their old ID.
    last_assigned_id: u64,
}

impl PlayerRoster {
    /// Adds a newly connected player and returns their ID. Calling this
    /// again for the same connection returns the existing ID instead of
    /// assigning a second one.
    pub fn add_player(&mut self, connection: Entity, is_host: bool) -> NetworkPlayerId {
        if let Some(existing) = self.player_for_connection(connection) {
            return existing.player_id;
        }
        self.last_assigned_id += 1;
        let player_id = NetworkPlayerId(self.last_assigned_id);
        self.players.push(RosterEntry {
            player_id,
            connection,
            is_host,
        });
        player_id
    }

    /// Removes the player using `connection`, returning their entry if
    /// they were on the roster.
    pub fn remove_connection(&mut self, connection: Entity) -> Option<RosterEntry> {
        let index = self
            .players
            .iter()
            .position(|player| player.connection == connection)?;
        Some(self.players.remove(index))
    }

    pub fn player_for_connection(&self, connection: Entity) -> Option<&RosterEntry> {
        self.players
            .iter()
            .find(|player| player.connection == connection)
    }

    pub fn len(&self) -> usize {
        self.players.len()
    }

    pub fn players(&self) -> impl Iterator<Item = &RosterEntry> {
        self.players.iter()
    }

    pub fn contains(&self, player_id: NetworkPlayerId) -> bool {
        self.players
            .iter()
            .any(|player| player.player_id == player_id)
    }

    /// The ID of the host's own player, once it has connected.
    pub fn host_player_id(&self) -> Option<NetworkPlayerId> {
        self.players
            .iter()
            .find(|player| player.is_host)
            .map(|player| player.player_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real entity IDs, so the tests don't depend on Bevy's raw-ID API.
    fn connections(count: usize) -> Vec<Entity> {
        let mut world = World::new();
        (0..count).map(|_| world.spawn_empty().id()).collect()
    }

    #[test]
    fn ids_start_at_one_and_increase() {
        let c = connections(2);
        let mut roster = PlayerRoster::default();
        assert_eq!(roster.add_player(c[0], true), NetworkPlayerId(1));
        assert_eq!(roster.add_player(c[1], false), NetworkPlayerId(2));
        assert_eq!(roster.len(), 2);
    }

    #[test]
    fn same_connection_keeps_its_id() {
        let c = connections(1);
        let mut roster = PlayerRoster::default();
        let first = roster.add_player(c[0], false);
        let second = roster.add_player(c[0], false);
        assert_eq!(first, second);
        assert_eq!(roster.len(), 1);
    }

    #[test]
    fn ids_are_not_reused_after_leaving() {
        let c = connections(3);
        let mut roster = PlayerRoster::default();
        roster.add_player(c[0], true);
        roster.add_player(c[1], false);

        let removed = roster.remove_connection(c[1]).unwrap();
        assert_eq!(removed.player_id, NetworkPlayerId(2));

        assert_eq!(roster.add_player(c[2], false), NetworkPlayerId(3));
        assert_eq!(roster.len(), 2);
    }

    #[test]
    fn finds_host_and_members() {
        let c = connections(2);
        let mut roster = PlayerRoster::default();
        assert_eq!(roster.host_player_id(), None);
        let guest = roster.add_player(c[0], false);
        let host = roster.add_player(c[1], true);
        assert_eq!(roster.host_player_id(), Some(host));
        assert!(roster.contains(guest));
        assert!(!roster.contains(NetworkPlayerId(99)));
    }

    #[test]
    fn removing_unknown_connection_is_harmless() {
        let c = connections(1);
        let mut roster = PlayerRoster::default();
        assert!(roster.remove_connection(c[0]).is_none());
    }
}
