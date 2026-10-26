use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

use crate::characters::animation::{
    CharacterAnimationOwner, SHARED_ANIMATION_PATH, initialize_character_animation_player,
};
use crate::player::Player;

/// Gameplay marker for players currently using the Vanguard class.
#[derive(Component)]
pub struct Vanguard;

#[derive(Component)]
pub struct VanguardVisual;

pub fn attach_vanguard_visual(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    players: Query<Entity, (With<Player>, Added<Vanguard>)>,
) {
    for player in &players {
        println!("Attaching animated Vanguard placeholder");

        let character_scene =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(SHARED_ANIMATION_PATH));

        commands.entity(player).with_children(|parent| {
            parent
                .spawn((
                    Name::new("Vanguard Animated Visual"),
                    VanguardVisual,
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
