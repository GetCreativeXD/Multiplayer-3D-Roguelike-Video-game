use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

use crate::characters::animation::{
    CharacterAnimationOwner, SHARED_ANIMATION_PATH, initialize_character_animation_player,
};
use crate::player::Player;

/// Gameplay marker for players currently using the Gravemind class. See
/// `characters::gravemind_abilities` for the class's three abilities
/// (Ruin Bolt, Consume, Embrace the Ruin -- project context doc, section
/// 23). Gravemind is intended to be an unlockable class (not necessarily
/// available at run start, per the context doc's "Permanent-progression
/// rule") -- nothing about that unlock gate is implemented yet, only the
/// class kit itself; see `characters::gravemind_abilities`'s module doc
/// comment for the full list of what this pass does and doesn't cover.
#[derive(Component)]
pub struct Gravemind;

#[derive(Component)]
pub struct GravemindVisual;

/// Attaches the shared animated placeholder visual to a newly spawned
/// Gravemind player, mirroring `characters::vanguard::attach_vanguard_visual`,
/// `characters::warden::attach_warden_visual`,
/// `characters::runesage::attach_runesage_visual`, and
/// `characters::wildshaper::attach_wildshaper_visual` exactly -- every
/// class currently uses the same shared `SHARED_ANIMATION_PATH` scene for
/// its animated body (idle/run/jump/dodge) until it gets a unique model.
/// A dedicated Gravemind model is future art work, not specific to wiring
/// the class in.
pub fn attach_gravemind_visual(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    players: Query<Entity, (With<Player>, Added<Gravemind>)>,
) {
    for player in &players {
        println!("Attaching animated Gravemind placeholder");

        let character_scene =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(SHARED_ANIMATION_PATH));

        commands.entity(player).with_children(|parent| {
            parent
                .spawn((
                    Name::new("Gravemind Animated Visual"),
                    GravemindVisual,
                    CharacterAnimationOwner::new(player),
                    WorldAssetRoot(character_scene.clone()),
                    Transform {
                        translation: Vec3::new(0.0, -1.0, 0.0),
                        ..default()
                    },
                ))
                .observe(initialize_character_animation_player);
        });
    }
}
