//! Generic room-adjacency graph.
//!
//! `connections::connect_room` computes *where* a new room sits in the
//! world; this records *that* two rooms are connected at all, completely
//! independent of room count or layout shape. Today the only reader is
//! the exploration map (`ui::map`), which walks the graph to know what
//! connects to what and to lay unexplored-but-discovered doorways out
//! next to the rooms the party has actually visited. A future procedural
//! dungeon generator (Section 4 of the context doc) can build the same
//! graph directly instead of one `connect` call per hand-placed room pair
//! -- nothing downstream needs to change either way.
//!
//! Usage: after spawning a new room and connecting its geometry with
//! `connections::connect_room`, register the same relationship here with
//! one call: `room_graph.connect(previous_room_entity, new_room_entity)`
//! (see `ruined_barracks::spawn_ruined_barracks` for the first real use).

use std::collections::HashSet;

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct RoomGraph {
    edges: HashSet<(Entity, Entity)>,
}

impl RoomGraph {
    /// Records that `a` and `b` are directly connected (in either
    /// direction -- the graph is undirected). Safe to call more than once
    /// for the same pair.
    pub fn connect(&mut self, a: Entity, b: Entity) {
        self.edges.insert(Self::normalize(a, b));
    }

    pub fn is_connected(&self, a: Entity, b: Entity) -> bool {
        self.edges.contains(&Self::normalize(a, b))
    }

    /// Every room directly connected to `room`.
    pub fn neighbors(&self, room: Entity) -> impl Iterator<Item = Entity> + '_ {
        self.edges.iter().filter_map(move |&(a, b)| {
            if a == room {
                Some(b)
            } else if b == room {
                Some(a)
            } else {
                None
            }
        })
    }

    /// Every connected pair, each yielded once. Order is otherwise
    /// unspecified (`HashSet` iteration).
    pub fn edges(&self) -> impl Iterator<Item = (Entity, Entity)> + '_ {
        self.edges.iter().copied()
    }

    /// Normalizes a pair so `(a, b)` and `(b, a)` hash/compare equal --
    /// entities implement `Ord`, so sorting the pair is enough.
    fn normalize(a: Entity, b: Entity) -> (Entity, Entity) {
        if a < b { (a, b) } else { (b, a) }
    }
}
