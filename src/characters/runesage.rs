use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

use crate::characters::animation::{
    CharacterAnimationOwner, SHARED_ANIMATION_PATH, initialize_character_animation_player,
};
use crate::player::Player;

/// Gameplay marker for players currently using the Runesage class.
/// See `characters::runesage_abilities` for the class's rune resource and
/// three abilities (Rune Blast, Runic Surge, Grand Rune).
#[derive(Component)]
pub struct Runesage;

#[derive(Component)]
pub struct RunesageVisual;

/// Attaches the shared animated placeholder visual to a newly spawned
/// Runesage player, mirroring `characters::vanguard::attach_vanguard_visual`
/// and `characters::warden::attach_warden_visual` exactly -- every class
/// currently uses the same shared `SHARED_ANIMATION_PATH` scene for its
/// animated body (idle/run/jump/dodge). Swapping in a unique Runesage
/// model is future work shared with the rest of the roster, not specific
/// to this class.
pub fn attach_runesage_visual(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    players: Query<Entity, (With<Player>, Added<Runesage>)>,
) {
    for player in &players {
        println!("Attaching animated Runesage placeholder");

        let character_scene =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(SHARED_ANIMATION_PATH));

        commands.entity(player).with_children(|parent| {
            parent
                .spawn((
                    Name::new("Runesage Animated Visual"),
                    RunesageVisual,
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
