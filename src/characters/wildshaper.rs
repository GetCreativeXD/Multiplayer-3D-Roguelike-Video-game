use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

use crate::characters::animation::{
    CharacterAnimationOwner, SHARED_ANIMATION_PATH, initialize_character_animation_player,
};
use crate::player::Player;

/// Gameplay marker for players currently using the Wildshaper class.
/// See `characters::wildshaper_abilities` for the class's three
/// transformation abilities (Rhino Charge, Bear Slam, Panther Pounce --
/// project context doc, section 22).
#[derive(Component)]
pub struct Wildshaper;

#[derive(Component)]
pub struct WildshaperVisual;

/// Attaches the shared animated placeholder visual to a newly spawned
/// Wildshaper player, mirroring `characters::vanguard::attach_vanguard_visual`,
/// `characters::warden::attach_warden_visual`, and
/// `characters::runesage::attach_runesage_visual` exactly -- every class
/// currently uses the same shared `SHARED_ANIMATION_PATH` scene for its
/// animated body (idle/run/jump/dodge) until it gets a unique model. A
/// dedicated Wildshaper model (and animal-transformation visuals for its
/// three abilities) is future art work, not specific to wiring the class in.
pub fn attach_wildshaper_visual(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    players: Query<Entity, (With<Player>, Added<Wildshaper>)>,
) {
    for player in &players {
        println!("Attaching animated Wildshaper placeholder");

        let character_scene =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(SHARED_ANIMATION_PATH));

        commands.entity(player).with_children(|parent| {
            parent
                .spawn((
                    Name::new("Wildshaper Animated Visual"),
                    WildshaperVisual,
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
