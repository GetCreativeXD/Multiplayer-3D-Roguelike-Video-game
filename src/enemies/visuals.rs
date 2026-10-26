use bevy::prelude::*;

#[derive(Component)]
pub struct EnemyPlaceholderVisual;

pub fn attach_footman_visual(
    commands: &mut Commands,
    enemy: Entity,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let armor = materials.add(Color::srgb(0.28, 0.31, 0.35));
    let cloth = materials.add(Color::srgb(0.42, 0.08, 0.08));
    let ruin = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.12, 0.10),
        emissive: LinearRgba::new(0.8, 0.04, 0.02, 1.0),
        ..default()
    });
    let weapon = materials.add(Color::srgb(0.50, 0.52, 0.55));

    commands.entity(enemy).with_children(|parent| {
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Footman Torso"),
            Mesh3d(meshes.add(Cuboid::new(0.75, 1.05, 0.42))),
            MeshMaterial3d(armor.clone()),
            Transform::from_xyz(0.0, 0.25, 0.0),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Footman Tabard"),
            Mesh3d(meshes.add(Cuboid::new(0.48, 0.78, 0.08))),
            MeshMaterial3d(cloth),
            Transform::from_xyz(0.0, 0.15, 0.25),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Footman Helmet"),
            Mesh3d(meshes.add(Sphere::new(0.34))),
            MeshMaterial3d(armor),
            Transform::from_xyz(0.0, 1.02, 0.0),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Footman Ruin Eye"),
            Mesh3d(meshes.add(Sphere::new(0.075))),
            MeshMaterial3d(ruin),
            Transform::from_xyz(0.0, 1.04, 0.32),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Footman Shield"),
            Mesh3d(meshes.add(Cuboid::new(0.58, 0.82, 0.12))),
            MeshMaterial3d(weapon.clone()),
            Transform {
                translation: Vec3::new(-0.58, 0.25, 0.18),
                rotation: Quat::from_rotation_z(-0.12),
                ..default()
            },
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Footman Sword"),
            Mesh3d(meshes.add(Cuboid::new(0.10, 1.15, 0.10))),
            MeshMaterial3d(weapon),
            Transform {
                translation: Vec3::new(0.55, 0.20, 0.10),
                rotation: Quat::from_rotation_z(-0.35),
                ..default()
            },
        ));
    });
}

pub fn attach_marksman_visual(
    commands: &mut Commands,
    enemy: Entity,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let leather = materials.add(Color::srgb(0.20, 0.14, 0.10));
    let hood = materials.add(Color::srgb(0.16, 0.20, 0.16));
    let bow = materials.add(Color::srgb(0.52, 0.30, 0.10));
    let glow = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.42, 0.06),
        emissive: LinearRgba::new(1.2, 0.22, 0.01, 1.0),
        ..default()
    });

    commands.entity(enemy).with_children(|parent| {
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Marksman Body"),
            Mesh3d(meshes.add(Cuboid::new(0.52, 1.12, 0.34))),
            MeshMaterial3d(leather),
            Transform::from_xyz(0.0, 0.25, 0.0),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Marksman Hood"),
            Mesh3d(meshes.add(Cone::new(0.38, 0.58))),
            MeshMaterial3d(hood),
            Transform::from_xyz(0.0, 1.03, 0.0),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Marksman Eye"),
            Mesh3d(meshes.add(Sphere::new(0.065))),
            MeshMaterial3d(glow),
            Transform::from_xyz(0.0, 1.00, 0.31),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Marksman Bow"),
            Mesh3d(meshes.add(Torus::new(0.03, 0.62))),
            MeshMaterial3d(bow.clone()),
            Transform {
                translation: Vec3::new(-0.48, 0.35, 0.20),
                rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                scale: Vec3::new(0.45, 1.0, 1.0),
            },
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Marksman Quiver"),
            Mesh3d(meshes.add(Cylinder::new(0.14, 0.75))),
            MeshMaterial3d(bow),
            Transform {
                translation: Vec3::new(0.30, 0.45, -0.20),
                rotation: Quat::from_rotation_z(-0.25),
                ..default()
            },
        ));
    });
}

pub fn attach_cinder_hound_visual(
    commands: &mut Commands,
    enemy: Entity,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let hide = materials.add(Color::srgb(0.16, 0.10, 0.08));
    let dark = materials.add(Color::srgb(0.07, 0.055, 0.05));
    let ember = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.22, 0.025),
        emissive: LinearRgba::new(2.0, 0.12, 0.01, 1.0),
        ..default()
    });

    commands.entity(enemy).with_children(|parent| {
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Cinder Hound Body"),
            Mesh3d(meshes.add(Capsule3d::new(0.34, 0.85))),
            MeshMaterial3d(hide.clone()),
            Transform {
                translation: Vec3::new(0.0, 0.0, 0.0),
                rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                ..default()
            },
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Cinder Hound Head"),
            Mesh3d(meshes.add(Cuboid::new(0.48, 0.38, 0.62))),
            MeshMaterial3d(dark.clone()),
            Transform::from_xyz(0.0, 0.12, 0.72),
        ));

        for x in [-0.27, 0.27] {
            for z in [-0.38, 0.42] {
                parent.spawn((
                    EnemyPlaceholderVisual,
                    Name::new("Cinder Hound Leg"),
                    Mesh3d(meshes.add(Cuboid::new(0.16, 0.62, 0.16))),
                    MeshMaterial3d(hide.clone()),
                    Transform::from_xyz(x, -0.34, z),
                ));
            }
        }

        for z in [-0.34, 0.0, 0.34] {
            parent.spawn((
                EnemyPlaceholderVisual,
                Name::new("Cinder Hound Ember Spine"),
                Mesh3d(meshes.add(Cone::new(0.13, 0.42))),
                MeshMaterial3d(ember.clone()),
                Transform::from_xyz(0.0, 0.46, z),
            ));
        }

        for x in [-0.13, 0.13] {
            parent.spawn((
                EnemyPlaceholderVisual,
                Name::new("Cinder Hound Eye"),
                Mesh3d(meshes.add(Sphere::new(0.055))),
                MeshMaterial3d(ember.clone()),
                Transform::from_xyz(x, 0.19, 1.04),
            ));
        }
    });
}

pub fn attach_black_banner_knight_visual(
    commands: &mut Commands,
    enemy: Entity,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let armor = materials.add(Color::srgb(0.12, 0.12, 0.14));
    let gold = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.52, 0.16),
        emissive: LinearRgba::new(0.35, 0.28, 0.05, 1.0),
        ..default()
    });
    let cloth = materials.add(Color::srgb(0.32, 0.03, 0.04));
    let ruin = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.12, 0.10),
        emissive: LinearRgba::new(0.9, 0.05, 0.02, 1.0),
        ..default()
    });
    let steel = materials.add(Color::srgb(0.30, 0.30, 0.33));

    commands.entity(enemy).with_children(|parent| {
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Knight Torso"),
            Mesh3d(meshes.add(Cuboid::new(0.95, 1.25, 0.55))),
            MeshMaterial3d(armor.clone()),
            Transform::from_xyz(0.0, 0.35, 0.0),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Knight Gold Trim"),
            Mesh3d(meshes.add(Cuboid::new(0.98, 0.12, 0.58))),
            MeshMaterial3d(gold.clone()),
            Transform::from_xyz(0.0, 0.75, 0.0),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Knight Helmet"),
            Mesh3d(meshes.add(Cuboid::new(0.42, 0.42, 0.42))),
            MeshMaterial3d(armor.clone()),
            Transform::from_xyz(0.0, 1.28, 0.0),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Knight Crown Trim"),
            Mesh3d(meshes.add(Cuboid::new(0.46, 0.08, 0.46))),
            MeshMaterial3d(gold.clone()),
            Transform::from_xyz(0.0, 1.50, 0.0),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Knight Ruin Eye"),
            Mesh3d(meshes.add(Sphere::new(0.08))),
            MeshMaterial3d(ruin.clone()),
            Transform::from_xyz(0.0, 1.30, 0.23),
        ));
        // Large shield -- the roster's namesake defensive silhouette.
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Knight Shield"),
            Mesh3d(meshes.add(Cuboid::new(0.75, 1.15, 0.14))),
            MeshMaterial3d(steel.clone()),
            Transform {
                translation: Vec3::new(-0.72, 0.35, 0.20),
                rotation: Quat::from_rotation_z(-0.08),
                ..default()
            },
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Knight Shield Emblem"),
            Mesh3d(meshes.add(Sphere::new(0.14))),
            MeshMaterial3d(ruin),
            Transform::from_xyz(-0.72, 0.35, 0.28),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Knight Sword"),
            Mesh3d(meshes.add(Cuboid::new(0.12, 1.35, 0.12))),
            MeshMaterial3d(steel),
            Transform {
                translation: Vec3::new(0.68, 0.30, 0.05),
                rotation: Quat::from_rotation_z(-0.30),
                ..default()
            },
        ));
        // Banner rising above the silhouette -- the roster's own placeholder
        // identity for this enemy ("large shield, banner rising above the
        // silhouette").
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Knight Banner Pole"),
            Mesh3d(meshes.add(Cylinder::new(0.035, 1.6))),
            MeshMaterial3d(gold.clone()),
            Transform::from_xyz(0.0, 1.55, -0.32),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Knight Banner Cloth"),
            Mesh3d(meshes.add(Cuboid::new(0.55, 0.85, 0.04))),
            MeshMaterial3d(cloth),
            Transform::from_xyz(0.0, 1.85, -0.32),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Knight Banner Emblem"),
            Mesh3d(meshes.add(Sphere::new(0.10))),
            MeshMaterial3d(gold),
            Transform::from_xyz(0.0, 1.95, -0.30),
        ));
    });
}

/// `aura_radius` is passed in (rather than imported from `enemies::knight`)
/// so this file stays decoupled from any specific archetype's tuning --
/// the same reasoning `enemies::aura::AuraEmitter` itself is generic.
pub fn attach_corrupted_banner_visual(
    commands: &mut Commands,
    enemy: Entity,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    aura_radius: f32,
) {
    let pole = materials.add(Color::srgb(0.35, 0.30, 0.10));
    let cloth = materials.add(Color::srgb(0.30, 0.03, 0.04));
    let ruin = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.12, 0.10),
        emissive: LinearRgba::new(1.1, 0.06, 0.02, 1.0),
        ..default()
    });
    let ring = materials.add(StandardMaterial {
        base_color: Color::srgba(0.55, 0.05, 0.08, 0.16),
        emissive: LinearRgba::new(0.7, 0.05, 0.01, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    commands.entity(enemy).with_children(|parent| {
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Banner Pole"),
            Mesh3d(meshes.add(Cylinder::new(0.06, 2.6))),
            MeshMaterial3d(pole),
            Transform::from_xyz(0.0, 1.1, 0.0),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Banner Cloth"),
            Mesh3d(meshes.add(Cuboid::new(0.75, 1.3, 0.05))),
            MeshMaterial3d(cloth),
            Transform::from_xyz(0.0, 1.55, 0.0),
        ));
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Banner Ruin Emblem"),
            Mesh3d(meshes.add(Sphere::new(0.16))),
            MeshMaterial3d(ruin),
            Transform::from_xyz(0.0, 1.75, 0.045),
        ));
        // Subtle static ring showing the aura's radius, so the buff's range
        // is visible at a glance -- matching the roster's "the buff has a
        // visible range and clear visual effect" requirement (written for
        // Bell Husk; applied here since this is the exact same aura shape).
        parent.spawn((
            EnemyPlaceholderVisual,
            Name::new("Banner Aura Ring"),
            Mesh3d(meshes.add(Torus::new(0.08, aura_radius))),
            MeshMaterial3d(ring),
            Transform::from_xyz(0.0, 0.06, 0.0),
        ));
    });
}
