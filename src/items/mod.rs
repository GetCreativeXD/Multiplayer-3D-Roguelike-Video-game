use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ItemDefinitionId(pub &'static str);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct ItemInstanceId(pub u64);

impl ItemInstanceId {
    pub const UNASSIGNED: Self = Self(0);

    pub fn is_assigned(self) -> bool {
        self != Self::UNASSIGNED
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ItemMetadata {
    pub definition_id: ItemDefinitionId,
    pub instance_id: ItemInstanceId,
    pub name: &'static str,
}

impl ItemMetadata {
    pub const fn new(definition_id: &'static str, name: &'static str) -> Self {
        Self {
            definition_id: ItemDefinitionId(definition_id),
            instance_id: ItemInstanceId::UNASSIGNED,
            name,
        }
    }
}

#[derive(Resource, Debug)]
pub struct ItemIdGenerator {
    next: u64,
}

impl Default for ItemIdGenerator {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl ItemIdGenerator {
    pub fn next(&mut self) -> ItemInstanceId {
        let id = ItemInstanceId(self.next);
        self.next = self.next.saturating_add(1);
        id
    }
}
