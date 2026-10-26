pub mod animation;
pub mod class_abilities;
pub mod gravemind;
pub mod gravemind_abilities;
pub mod runesage;
pub mod runesage_abilities;
pub mod vanguard;
pub mod vanguard_abilities;
pub mod warden;
pub mod warden_abilities;
pub mod wildshaper;
pub mod wildshaper_abilities;

use bevy::prelude::*;

pub struct CharactersPlugin;

impl Plugin for CharactersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, animation::setup_character_animations)
            .add_plugins(class_abilities::ClassAbilitiesPlugin)
            .add_plugins(gravemind_abilities::GravemindAbilitiesPlugin)
            .add_plugins(runesage_abilities::RunesageAbilitiesPlugin)
            .add_plugins(vanguard_abilities::VanguardAbilitiesPlugin)
            .add_plugins(warden_abilities::WardenAbilitiesPlugin)
            .add_plugins(wildshaper_abilities::WildshaperAbilitiesPlugin)
            .add_systems(
                Update,
                (
                    gravemind::attach_gravemind_visual,
                    runesage::attach_runesage_visual,
                    vanguard::attach_vanguard_visual,
                    warden::attach_warden_visual,
                    wildshaper::attach_wildshaper_visual,
                    animation::update_character_motion,
                    animation::update_character_animation,
                )
                    .chain()
                    // Motion reflects this frame's movement, jump and dodge.
                    .after(crate::player::PlayerMovementSet),
            );
    }
}

// =========================================================
// CLASS IDENTIFIER (run-setup screen, save data, etc.)
// =========================================================

/// Identifies one of the five classes without needing to reach for its
/// concrete marker component (`vanguard::Vanguard`, `warden::Warden`,
/// etc.). Anything that needs to talk about "which class" in the
/// abstract -- the run-setup screen's catalog (`run_setup::catalog`) and
/// its `RunConfig`, and eventually save data -- uses this instead of the
/// marker components directly.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
// Multiplayer: each joined player sends the class they chose (`net::loadout`).
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum ClassId {
    Vanguard,
    Warden,
    Wildshaper,
    Runesage,
    Gravemind,
}

/// Inserts the marker component for `class` onto `player`. The single
/// place that translates a `ClassId` into the real gameplay component --
/// `attach_*_visual` (above) and every `*_abilities` plugin key off the
/// concrete component, so this is what the run-setup screen's chosen
/// class (`run_setup::RunConfig::class`) turns into at spawn time (see
/// `rooms::fallen_gate::setup_fallen_gate_player`). A future
/// sixth class needs one new arm here, matching the one new `ClassId`
/// variant and one new catalog entry it also needs.
pub fn insert_class(commands: &mut Commands, player: Entity, class: ClassId) {
    match class {
        ClassId::Vanguard => commands.entity(player).insert(vanguard::Vanguard),
        ClassId::Warden => commands.entity(player).insert(warden::Warden),
        ClassId::Wildshaper => commands.entity(player).insert(wildshaper::Wildshaper),
        ClassId::Runesage => commands.entity(player).insert(runesage::Runesage),
        ClassId::Gravemind => commands.entity(player).insert(gravemind::Gravemind),
    };
}
