use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::camera::CameraTarget;
use crate::player::{InteractionKind, PlayerCharacter, PlayerIntent};
use crate::weapons::WorldLoot;

use super::{LootDropRequest, LootDropRequests, LootTableId};

const CHEST_OPEN_RANGE: f32 = 3.0;

#[derive(Component)]
pub struct LootChest {
    pub table_id: LootTableId,
    pub floor: u32,
    /// When true, opening this chest rolls loot but does not despawn it,
    /// so it can be opened again immediately. Used by the Test Grounds
    /// (`rooms::test_grounds`) to let a tester repeatedly sample a loot
    /// table without needing a fresh chest each time. Real dungeon chests
    /// (`spawn_loot_chest`) are always `false` -- one-time as before.
    pub repeatable: bool,
}

#[derive(Component)]
pub(crate) struct ChestPrompt;

pub fn spawn_loot_chest(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    table_id: LootTableId,
    floor: u32,
) -> Entity {
    let loot = WorldLoot::Chest { repeatable: false };
    parent
        .spawn((
            Name::new(format!("Loot Chest: {}", table_id.0)),
            LootChest {
                table_id,
                floor,
                repeatable: false,
            },
            Mesh3d(meshes.add(loot.mesh())),
            MeshMaterial3d(materials.add(loot.material())),
            Transform::from_translation(position),
            loot,
        ))
        .id()
}

/// Same as `spawn_loot_chest`, but the chest never despawns after opening --
/// each `[F]` press rolls another set of loot from `table_id`/`floor` and
/// leaves the chest in place. Built for `rooms::test_grounds` so a tester
/// can sample a loot table's drop odds and item variety without repeatedly
/// respawning chests. Tinted teal instead of brown so it's obviously a test
/// fixture, not a real dungeon reward.
pub fn spawn_repeatable_loot_chest(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    table_id: LootTableId,
    floor: u32,
) -> Entity {
    let loot = WorldLoot::Chest { repeatable: true };
    parent
        .spawn((
            Name::new(format!(
                "Test Chest ({}, floor {}): {}",
                table_id.0, floor, table_id.0
            )),
            LootChest {
                table_id,
                floor,
                repeatable: true,
            },
            Mesh3d(meshes.add(loot.mesh())),
            MeshMaterial3d(materials.add(loot.material())),
            Transform::from_translation(position),
            loot,
        ))
        .id()
}

pub(crate) fn spawn_chest_prompt(mut commands: Commands) {
    commands.spawn((
        ChestPrompt,
        Text::new("[F] Open Chest"),
        TextFont {
            font_size: FontSize::Px(24.0),
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.78, 0.35)),
        Node {
            position_type: PositionType::Absolute,
            left: percent(50.0),
            bottom: px(140.0),
            margin: UiRect {
                left: px(-100.0),
                ..default()
            },
            width: px(200.0),
            justify_content: JustifyContent::Center,
            display: Display::None,
            ..default()
        },
    ));
}

/// "[F] Open Chest" for the character this machine's camera follows (the
/// local `Player`, or a joined player's own character). Reads `WorldLoot`,
/// which a joined client has for the host's chests too.
pub(crate) fn update_chest_prompt(
    cursor: Single<&CursorOptions>,
    viewer: Query<&GlobalTransform, With<CameraTarget>>,
    loot: Query<(&GlobalTransform, &WorldLoot)>,
    mut prompt: Single<&mut Node, With<ChestPrompt>>,
) {
    if cursor.grab_mode == CursorGrabMode::None {
        prompt.display = Display::None;
        return;
    }
    let Ok(viewer) = viewer.single() else {
        prompt.display = Display::None;
        return;
    };
    let nearby = loot.iter().any(|(transform, loot)| {
        loot.is_chest()
            && viewer.translation().distance(transform.translation()) <= CHEST_OPEN_RANGE
    });
    prompt.display = if nearby { Display::Flex } else { Display::None };
}

/// Opens the nearest chest for every player character that pressed [F]
/// (the local `Player` and, on a multiplayer host, each joined player's
/// body). The loot drops on the ground for anyone to take. Simulating
/// machine only.
pub(crate) fn open_nearby_chest(
    mut commands: Commands,
    mut players: Query<(&GlobalTransform, &mut PlayerIntent), With<PlayerCharacter>>,
    chests: Query<(Entity, &GlobalTransform, &LootChest)>,
    mut requests: ResMut<LootDropRequests>,
) {
    // Chests opened this frame, so two players can't open one twice.
    let mut opened: Vec<Entity> = Vec::new();

    for (player, mut intent) in &mut players {
        if !intent.take_interact(InteractionKind::OpenChest) {
            continue;
        }

        let nearest = chests
            .iter()
            .filter(|(entity, _, _)| !opened.contains(entity))
            .filter_map(|(entity, transform, chest)| {
                let distance = player.translation().distance(transform.translation());
                (distance <= CHEST_OPEN_RANGE).then_some((distance, entity, transform, chest))
            })
            .min_by(|(a, _, _, _), (b, _, _, _)| a.total_cmp(b));

        let Some((_, entity, transform, chest)) = nearest else {
            continue;
        };

        requests.0.push(LootDropRequest {
            table_id: chest.table_id,
            position: transform.translation(),
            floor: chest.floor,
        });

        if chest.repeatable {
            println!(
                "Rolled test chest using loot table {} (floor {}) -- reopen with [F] anytime",
                chest.table_id.0, chest.floor
            );
        } else {
            opened.push(entity);
            commands.entity(entity).despawn();
            println!("Opened chest using loot table {}", chest.table_id.0);
        }
    }
}
