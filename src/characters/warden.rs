use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

use crate::characters::animation::{
    CharacterAnimationOwner, SHARED_ANIMATION_PATH, initialize_character_animation_player,
};
use crate::player::Player;

/// Gameplay marker for players currently using the Warden of Shades class.
/// See `characters::warden_abilities` for the class's three abilities
/// (Guardian Otter, Shadow Stag, Predator's Spirit).
#[derive(Component)]
pub struct Warden;

#[derive(Component)]
pub struct WardenVisual;

/// Attaches the shared animated placeholder visual to a newly spawned
/// Warden player, mirroring `characters::vanguard::attach_vanguard_visual`
/// exactly -- every class currently uses the same shared
/// `SHARED_ANIMATION_PATH` scene for its animated body (idle/run/jump/
/// dodge), regardless of the unique per-class GLB art already sitting in
/// `assets/characters/`. Swapping in the unique Warden of Shades model is
/// future work shared with the rest of the roster, not specific to this
/// class.
pub fn attach_warden_visual(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    players: Query<Entity, (With<Player>, Added<Warden>)>,
) {
    for player in &players {
        println!("Attaching animated Warden of Shades placeholder");

        let character_scene =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(SHARED_ANIMATION_PATH));

        commands.entity(player).with_children(|parent| {
            parent
                .spawn((
                    Name::new("Warden of Shades Animated Visual"),
                    WardenVisual,
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
