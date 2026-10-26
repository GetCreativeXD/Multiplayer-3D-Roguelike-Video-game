use avian3d::character_controller::move_and_slide::{
    MoveAndSlide, MoveAndSlideConfig, MoveAndSlideHitResponse,
};
use avian3d::math::{AdjustPrecision as _, AsF32 as _};
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::camera::CameraTarget;
use crate::characters::animation::CharacterMotion;
use crate::characters::class_abilities::CharacterCombatModifiers;
use crate::collision::GameLayer;
use crate::combat::StatBuffs;
use crate::run_setup::RunModifiers;
use crate::weapons::InventoryUiState;

mod life;

pub use life::{LifeState, REVIVE_SECONDS};

const PLAYER_SPEED: f32 = 6.0;

const DODGE_SPEED: f32 = 14.0;
const DODGE_DURATION: f32 = 0.40;
const DODGE_INVULNERABILITY_DURATION: f32 = 0.22;
const DODGE_RECOVERY: f32 = 0.20;

const JUMP_SPEED: f32 = 7.0;
const GRAVITY: f32 = 18.0;
const MAX_JUMPS: u32 = 2;

#[derive(Component)]
#[require(CameraTarget, PlayerIntent, CharacterMotion, PlayerCharacter)]
pub struct Player;

/// A character this machine's own input drives, besides the `Player`: a
/// joined client's own avatar while it is predicted (`net::prediction`,
/// Milestone 9). `apply_local_input_to_player` writes `LocalPlayerInput`
/// into its `PlayerIntent`, so the fixed-tick movement systems move it at
/// once instead of waiting for the host. The host still decides where it
/// really is; the client corrects toward that.
#[derive(Component)]
pub struct LocallyDriven;

/// Movement-speed multiplier for a predicted character, copied from the
/// host (`net::prediction`), which knows the buffs and class modifiers a
/// client doesn't simulate. Multiplies walking speed like
/// `movement_speed_multiplier`. Never present offline or on the host.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PredictedMovementSpeed(pub f32);

/// Walking-speed multiplier from class-innate modifiers (e.g. Vanguard's
/// Momentum) and granted buffs (e.g. Warden of Shades' Guardian Otter, see
/// `combat::StatBuffs`). They stack multiplicatively.
pub fn movement_speed_multiplier(
    modifiers: Option<&CharacterCombatModifiers>,
    stat_buffs: Option<&StatBuffs>,
) -> f32 {
    modifiers.map_or(1.0, |value| value.movement_speed)
        * stat_buffs.map_or(1.0, StatBuffs::movement_speed_multiplier)
}

/// Marks every character a player controls: this machine's `Player` and,
/// on a multiplayer host, each joined player's body (`net::player_bodies`).
///
/// Enemies use it to decide who they can notice, chase and damage
/// (`enemies::targeting`), and `combat` uses it to give every player
/// character barriers, death prevention and stat buffs. `Player` itself
/// still means "the character this machine's keyboard, HUD and gear
/// systems drive".
///
/// Every player character also has a `LifeState` (Downed -> Lost, see
/// `player::life`).
#[derive(Component, Default)]
#[require(LifeState)]
pub struct PlayerCharacter;

/// Capsule size shared by every player character (local `Player` and the
/// host's bodies for remote players). The model's origin is the capsule's
/// center, which is why character visuals sit 1.0 unit below it.
pub const PLAYER_CAPSULE_RADIUS: f32 = 0.45;
pub const PLAYER_CAPSULE_LENGTH: f32 = 1.0;

/// The physics and movement-state components every player character needs
/// for `PlayerPlugin`'s movement systems to drive it. Spawn alongside a
/// `Transform`. A character that also has `PlayerIntent` will move.
pub fn player_character_body() -> impl Bundle {
    (
        Dodge::default(),
        VerticalMovement::default(),
        RigidBody::Kinematic,
        Collider::capsule(PLAYER_CAPSULE_RADIUS, PLAYER_CAPSULE_LENGTH),
        CollisionLayers::new(GameLayer::Player, GameLayer::World),
    )
}

/// What a player character is being asked to do, in world space.
///
/// This is the boundary between *input* and *simulation*. Movement systems
/// read only this component, never the keyboard or camera, so the same
/// code drives:
/// - the local `Player`, filled from this machine's keyboard and camera
///   (`apply_local_input_to_player`);
/// - on the host, each remote player's body, filled from the intent that
///   player's client sends over the network (`net::intent`).
///
/// Jump and dodge are *request counters*, not booleans: a press is never
/// lost between being recorded and being acted on, and the network can
/// send the running totals over an unreliable channel without dropping a
/// press when a packet goes missing.
///
/// Characters with an intent move on the fixed network tick
/// (`PlayerMovementSet` is in `FixedUpdate`, Milestone 9), so they also get
/// Avian's `TransformInterpolation`, which smooths their `Transform` between
/// ticks on screens faster than the tick rate. Writing such a character's
/// `Transform` outside the fixed schedules (an attack's forward step, a
/// void reset) still works: Avian treats it as a teleport.
#[derive(Component, Clone, Debug, Default)]
#[require(TransformInterpolation)]
pub struct PlayerIntent {
    /// Desired horizontal movement: a unit vector, or zero to stand still.
    pub move_direction: Vec3,
    /// Horizontal "forward" for this player (the camera's facing). Used for
    /// a dodge with no movement input.
    pub facing: Vec3,
    /// When true the character doesn't move or fall this frame (the local
    /// player has the inventory open, or there is no camera yet).
    pub movement_paused: bool,
    /// Where this player's camera is and which way it looks. Weapons aim
    /// with it exactly as they used to aim with the local camera (the
    /// crosshair is the centre of that view): see `aim_transform`. For a
    /// remote player it is that client's camera, sent with their intent.
    pub aim_origin: Vec3,
    pub aim_rotation: Quat,
    /// True while this character is downed or lost (`player::life`): it
    /// doesn't move, and new presses are ignored and pending ones dropped.
    pub incapacitated: bool,
    jump_requests: u32,
    jumps_handled: u32,
    dodge_requests: u32,
    dodges_handled: u32,
    attack_requests: u32,
    attacks_handled: u32,
    ability_requests: u32,
    abilities_handled: u32,
    class_ability_requests: [u32; CLASS_ABILITY_SLOTS],
    class_abilities_handled: [u32; CLASS_ABILITY_SLOTS],
    interact_requests: u32,
    /// One "handled" counter per kind of interaction, so a single [F]
    /// press both opens a nearby chest and picks up a nearby item, as it
    /// always has (`InteractionKind`).
    interacts_handled: [u32; INTERACTION_KINDS],
    weapon_slot_requests: [u32; WEAPON_SLOT_KEYS.len()],
    weapon_slots_handled: [u32; WEAPON_SLOT_KEYS.len()],
}

/// Weapon slot keys, by slot: 1, 2, 3 (`weapons::switching`).
pub const WEAPON_SLOT_KEYS: [KeyCode; 3] = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3];

/// What an [F] press is consumed by. Each kind reads the same presses
/// independently (`PlayerIntent::take_interact`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionKind {
    /// Opening a loot chest (`loot::chests`).
    OpenChest = 0,
    /// Picking up a weapon or armor piece (`weapons::loot`).
    PickUp = 1,
}

const INTERACTION_KINDS: usize = 2;

/// Class ability keys, by slot: Q, E, R (the HUD's three class slots).
pub const CLASS_ABILITY_SLOTS: usize = 3;
pub const CLASS_ABILITY_KEYS: [KeyCode; CLASS_ABILITY_SLOTS] =
    [KeyCode::KeyQ, KeyCode::KeyE, KeyCode::KeyR];

impl PlayerIntent {
    pub fn request_jump(&mut self) {
        if self.incapacitated {
            return;
        }
        self.jump_requests = self.jump_requests.wrapping_add(1);
    }

    pub fn request_dodge(&mut self) {
        if self.incapacitated {
            return;
        }
        self.dodge_requests = self.dodge_requests.wrapping_add(1);
    }

    /// This player's camera as a `Transform`, for the weapon code that
    /// takes a `camera_transform` (its translation and forward).
    pub fn aim_transform(&self) -> Transform {
        Transform::from_translation(self.aim_origin).with_rotation(self.aim_rotation)
    }

    /// A weapon basic-attack press. Aimed with `aim_transform`.
    pub fn request_attack(&mut self) {
        if self.incapacitated {
            return;
        }
        self.attack_requests = self.attack_requests.wrapping_add(1);
    }

    /// Consumes any pending basic-attack press. Read by
    /// `weapons::melee::player_basic_attack` for every player character.
    pub fn take_attack(&mut self) -> bool {
        let pending = self.attack_requests != self.attacks_handled;
        self.attacks_handled = self.attack_requests;
        pending && !self.incapacitated
    }

    /// A weapon-ability press (right click). Aimed with `aim_transform`.
    pub fn request_ability(&mut self) {
        if self.incapacitated {
            return;
        }
        self.ability_requests = self.ability_requests.wrapping_add(1);
    }

    /// Consumes any pending weapon-ability press. Read by
    /// `weapons::abilities::weapon_ability_input` for every player character.
    pub fn take_ability(&mut self) -> bool {
        let pending = self.ability_requests != self.abilities_handled;
        self.abilities_handled = self.ability_requests;
        pending && !self.incapacitated
    }

    /// A class-ability key press: slot 0 = Q, 1 = E, 2 = R.
    pub fn request_class_ability(&mut self, slot: usize) {
        if self.incapacitated {
            return;
        }
        if let Some(requests) = self.class_ability_requests.get_mut(slot) {
            *requests = requests.wrapping_add(1);
        }
    }

    /// Consumes any pending press of class-ability `slot`. Each class's
    /// ability systems read the slots their keys use.
    pub fn take_class_ability(&mut self, slot: usize) -> bool {
        let (Some(requests), Some(handled)) = (
            self.class_ability_requests.get(slot).copied(),
            self.class_abilities_handled.get_mut(slot),
        ) else {
            return false;
        };
        let pending = requests != *handled;
        *handled = requests;
        pending && !self.incapacitated
    }

    /// An interact ([F]) press: opens a nearby chest and picks up a nearby
    /// item.
    pub fn request_interact(&mut self) {
        if self.incapacitated {
            return;
        }
        self.interact_requests = self.interact_requests.wrapping_add(1);
    }

    /// Consumes any pending interact press for one kind of interaction.
    /// Every kind sees every press once.
    pub fn take_interact(&mut self, kind: InteractionKind) -> bool {
        let handled = &mut self.interacts_handled[kind as usize];
        let pending = self.interact_requests != *handled;
        *handled = self.interact_requests;
        pending && !self.incapacitated
    }

    /// A weapon-slot key press: slot 0 = [1], 1 = [2], 2 = [3].
    pub fn request_weapon_slot(&mut self, slot: usize) {
        if self.incapacitated {
            return;
        }
        if let Some(requests) = self.weapon_slot_requests.get_mut(slot) {
            *requests = requests.wrapping_add(1);
        }
    }

    /// Consumes pending weapon-slot presses and returns the slot asked for
    /// (the lowest-numbered one if several keys were pressed at once, as
    /// the keyboard check always chose).
    pub fn take_weapon_slot(&mut self) -> Option<usize> {
        let mut requested = None;
        for slot in 0..self.weapon_slot_requests.len() {
            if requested.is_none()
                && self.weapon_slot_requests[slot] != self.weapon_slots_handled[slot]
            {
                requested = Some(slot);
            }
            self.weapon_slots_handled[slot] = self.weapon_slot_requests[slot];
        }
        requested.filter(|_| !self.incapacitated)
    }

    /// Total jump presses so far. Presentation (animation) compares this
    /// with the last value it saw to react to new presses.
    pub fn jump_requests(&self) -> u32 {
        self.jump_requests
    }

    pub fn is_moving(&self) -> bool {
        !self.incapacitated && self.move_direction != Vec3::ZERO
    }

    /// Consumes any pending jump press. Several presses before the next
    /// movement update count as one, as `just_pressed` did before.
    fn take_jump(&mut self) -> bool {
        let pending = self.jump_requests != self.jumps_handled;
        self.jumps_handled = self.jump_requests;
        pending && !self.incapacitated
    }

    fn take_dodge(&mut self) -> bool {
        let pending = self.dodge_requests != self.dodges_handled;
        self.dodges_handled = self.dodge_requests;
        pending && !self.incapacitated
    }
}

/// This machine's own keyboard and camera, resolved to world space, as of
/// this frame. Filled by `read_local_input` in every mode:
/// - offline and on the host it is copied into the local `Player`'s
///   `PlayerIntent`;
/// - on a joined client (which has no `Player`) it is sent to the host.
#[derive(Resource, Clone, Debug, Default)]
pub struct LocalPlayerInput {
    pub move_direction: Vec3,
    pub facing: Vec3,
    pub movement_paused: bool,
    /// The camera's world position and rotation (see `PlayerIntent::aim_origin`).
    pub aim_origin: Vec3,
    pub aim_rotation: Quat,
    /// Edge-triggered: true only on the frame the key went down.
    pub jump_pressed: bool,
    pub dodge_pressed: bool,
    pub attack_pressed: bool,
    pub ability_pressed: bool,
    /// Class ability keys (Q, E, R) pressed this frame; ignored while the
    /// inventory is open.
    pub class_ability_pressed: [bool; CLASS_ABILITY_SLOTS],
    /// [F] pressed this frame (open chest / pick up).
    pub interact_pressed: bool,
    /// Weapon slot keys (1, 2, 3) pressed this frame.
    pub weapon_slot_pressed: [bool; 3],
}

/// Reads this machine's keyboard and camera into `LocalPlayerInput`, then
/// into the local `Player`. Networking adds remote players' intents after
/// this set and before `PlayerMovementSet`.
///
/// Runs once per frame in `RunFixedMainLoop`, just before the fixed-tick
/// loop (`RunFixedMainLoopSystems::BeforeFixedMainLoop`), so a press is
/// seen by the very next tick and by this frame's `Update` systems.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerInputSet;

/// Turns every character's `PlayerIntent` into movement: dodge timers,
/// jumps, dodges, walking, ability dashes, gravity and collision.
///
/// Runs in `FixedUpdate`, once per fixed tick (64 Hz, matching
/// `net::NETWORK_TICK_HZ` and Avian's physics step), so movement is
/// independent of frame rate and can later be replayed tick by tick for
/// client-side prediction. `Update` systems ordered `.after(PlayerMovementSet)`
/// still see this frame's ticks, because the fixed loop runs before `Update`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerMovementSet;

#[derive(Component)]
pub struct Dodge {
    pub direction: Vec3,
    pub time_remaining: f32,
    pub invulnerability_remaining: f32,
    pub cooldown_remaining: f32,
}

impl Default for Dodge {
    fn default() -> Self {
        Self {
            direction: Vec3::ZERO,
            time_remaining: 0.0,
            invulnerability_remaining: 0.0,
            cooldown_remaining: 0.0,
        }
    }
}

impl Dodge {
    pub fn is_dodging(&self) -> bool {
        self.time_remaining > 0.0
    }

    pub fn is_invulnerable(&self) -> bool {
        self.invulnerability_remaining > 0.0
    }

    pub fn can_dodge(&self) -> bool {
        self.cooldown_remaining <= 0.0
    }
}

#[derive(Component)]
pub struct VerticalMovement {
    pub velocity: f32,
    pub grounded: bool,
    pub jumps_used: u32,
}

impl Default for VerticalMovement {
    fn default() -> Self {
        Self {
            velocity: 0.0,
            grounded: false,
            jumps_used: 0,
        }
    }
}

#[derive(Component)]
pub struct AbilityMovement {
    pub direction: Vec3,
    pub speed: f32,
    pub time_remaining: f32,
}

impl AbilityMovement {
    pub fn is_active(&self) -> bool {
        self.time_remaining > 0.0
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(life::PlayerLifePlugin)
            .init_resource::<LocalPlayerInput>()
            .configure_sets(
                RunFixedMainLoop,
                PlayerInputSet.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
            )
            .add_systems(
                RunFixedMainLoop,
                (read_local_input, apply_local_input_to_player)
                    .chain()
                    .in_set(PlayerInputSet),
            )
            .add_systems(
                FixedUpdate,
                (
                    update_dodge_timers,
                    start_jump,
                    start_dodge,
                    character_movement,
                )
                    .chain()
                    .in_set(PlayerMovementSet),
            );
    }
}

fn movement_input(keyboard: &ButtonInput<KeyCode>) -> Vec2 {
    let mut input = Vec2::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        input.y += 1.0;
    }

    if keyboard.pressed(KeyCode::KeyS) {
        input.y -= 1.0;
    }

    if keyboard.pressed(KeyCode::KeyA) {
        input.x -= 1.0;
    }

    if keyboard.pressed(KeyCode::KeyD) {
        input.x += 1.0;
    }

    input.normalize_or_zero()
}

fn camera_relative_direction(input: Vec2, camera_transform: &Transform) -> Vec3 {
    let mut forward = camera_transform.forward().as_vec3();
    let mut right = camera_transform.right().as_vec3();

    forward.y = 0.0;
    right.y = 0.0;

    forward = forward.normalize_or_zero();
    right = right.normalize_or_zero();

    (forward * input.y + right * input.x).normalize_or_zero()
}

/// Samples the keyboard and camera once per frame. Movement isn't gated on
/// the cursor (as before), but jump and dodge are ignored while the cursor
/// is released, and movement pauses while the inventory is open.
fn read_local_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    cursor_options: Query<&CursorOptions>,
    inventory_ui: Res<InventoryUiState>,
    camera_query: Query<&Transform, With<Camera3d>>,
    mut local_input: ResMut<LocalPlayerInput>,
) {
    let gameplay_keys_enabled = cursor_options
        .single()
        .is_ok_and(|options| options.grab_mode != CursorGrabMode::None);

    let Ok(camera_transform) = camera_query.single() else {
        *local_input = LocalPlayerInput {
            movement_paused: true,
            ..default()
        };
        return;
    };

    let input = movement_input(&keyboard);

    let mut facing = camera_transform.forward().as_vec3();
    facing.y = 0.0;

    *local_input = LocalPlayerInput {
        move_direction: if input != Vec2::ZERO {
            camera_relative_direction(input, camera_transform)
        } else {
            Vec3::ZERO
        },
        facing: facing.normalize_or_zero(),
        movement_paused: inventory_ui.open,
        aim_origin: camera_transform.translation,
        aim_rotation: camera_transform.rotation,
        jump_pressed: gameplay_keys_enabled && keyboard.just_pressed(KeyCode::Space),
        dodge_pressed: gameplay_keys_enabled && keyboard.just_pressed(KeyCode::ShiftLeft),
        attack_pressed: gameplay_keys_enabled && mouse_buttons.just_pressed(MouseButton::Left),
        ability_pressed: gameplay_keys_enabled && mouse_buttons.just_pressed(MouseButton::Right),
        class_ability_pressed: CLASS_ABILITY_KEYS
            .map(|key| gameplay_keys_enabled && !inventory_ui.open && keyboard.just_pressed(key)),
        interact_pressed: gameplay_keys_enabled && keyboard.just_pressed(KeyCode::KeyF),
        weapon_slot_pressed: WEAPON_SLOT_KEYS
            .map(|key| gameplay_keys_enabled && keyboard.just_pressed(key)),
    };
}

/// Offline and on the host, this machine's input drives its `Player`; on a
/// predicting client, its own avatar (`LocallyDriven`).
fn apply_local_input_to_player(
    local_input: Res<LocalPlayerInput>,
    mut player_query: Query<&mut PlayerIntent, Or<(With<Player>, With<LocallyDriven>)>>,
) {
    let Ok(mut intent) = player_query.single_mut() else {
        return;
    };

    intent.move_direction = local_input.move_direction;
    intent.facing = local_input.facing;
    intent.movement_paused = local_input.movement_paused;
    intent.aim_origin = local_input.aim_origin;
    intent.aim_rotation = local_input.aim_rotation;

    if local_input.jump_pressed {
        intent.request_jump();
    }

    if local_input.dodge_pressed {
        intent.request_dodge();
    }

    if local_input.attack_pressed {
        intent.request_attack();
    }

    if local_input.ability_pressed {
        intent.request_ability();
    }

    for (slot, pressed) in local_input.class_ability_pressed.iter().enumerate() {
        if *pressed {
            intent.request_class_ability(slot);
        }
    }

    if local_input.interact_pressed {
        intent.request_interact();
    }

    for (slot, pressed) in local_input.weapon_slot_pressed.iter().enumerate() {
        if *pressed {
            intent.request_weapon_slot(slot);
        }
    }
}

fn start_dodge(
    run_modifiers: Res<RunModifiers>,
    mut characters: Query<(&mut PlayerIntent, &mut Dodge)>,
) {
    for (mut intent, mut dodge) in &mut characters {
        if !intent.take_dodge() {
            continue;
        }

        if !dodge.can_dodge() {
            continue;
        }

        // Dodge toward the movement input, or, with no input, toward the
        // direction the player's camera is facing.
        let dodge_direction = if intent.is_moving() {
            intent.move_direction
        } else {
            intent.facing
        };

        if dodge_direction.length_squared() == 0.0 {
            continue;
        }

        dodge.direction = dodge_direction;
        dodge.time_remaining = DODGE_DURATION;
        dodge.invulnerability_remaining = DODGE_INVULNERABILITY_DURATION;

        // Recovery begins after the roll finishes. `DODGE_RECOVERY` alone is
        // scaled by the run's dodge-cooldown relic (e.g. Swiftstep Charm) --
        // see `run_setup::RunModifiers::dodge_cooldown_multiplier` -- the
        // roll's own duration is not a "recovery" and stays fixed.
        dodge.cooldown_remaining =
            DODGE_DURATION + DODGE_RECOVERY * run_modifiers.dodge_cooldown_multiplier;
    }
}

fn update_dodge_timers(time: Res<Time>, mut characters: Query<&mut Dodge, With<PlayerIntent>>) {
    let delta = time.delta_secs();

    for mut dodge in &mut characters {
        dodge.time_remaining = (dodge.time_remaining - delta).max(0.0);

        dodge.invulnerability_remaining = (dodge.invulnerability_remaining - delta).max(0.0);

        dodge.cooldown_remaining = (dodge.cooldown_remaining - delta).max(0.0);
    }
}

fn start_jump(mut characters: Query<(&mut PlayerIntent, &mut VerticalMovement)>) {
    for (mut intent, mut vertical) in &mut characters {
        if !intent.take_jump() {
            continue;
        }

        if vertical.jumps_used >= MAX_JUMPS {
            continue;
        }

        vertical.velocity = JUMP_SPEED;
        vertical.grounded = false;
        vertical.jumps_used += 1;
    }
}

fn character_movement(
    time: Res<Time>,
    mut characters: Query<(
        Entity,
        &Collider,
        &mut Transform,
        &Dodge,
        &mut VerticalMovement,
        &PlayerIntent,
        Option<&mut AbilityMovement>,
        Option<&CharacterCombatModifiers>,
        Option<&StatBuffs>,
        Option<&PredictedMovementSpeed>,
    )>,
    move_and_slide: MoveAndSlide,
) {
    let delta = time.delta_secs();

    for (
        character_entity,
        collider,
        mut transform,
        dodge,
        mut vertical,
        intent,
        mut ability_movement,
        modifiers,
        stat_buffs,
        predicted_speed,
    ) in &mut characters
    {
        if intent.movement_paused {
            continue;
        }

        // A predicted avatar has no modifiers or buffs of its own; it uses
        // the multiplier the host sent (`PredictedMovementSpeed`).
        let speed_multiplier = movement_speed_multiplier(modifiers, stat_buffs)
            * predicted_speed.map_or(1.0, |speed| speed.0);

        // -------------------------------------------------
        // Horizontal movement: ability dash > dodge > walking
        // -------------------------------------------------

        let horizontal_velocity = match ability_movement.as_deref_mut() {
            Some(movement) if movement.is_active() => {
                let velocity = movement.direction * movement.speed;

                movement.time_remaining = (movement.time_remaining - delta).max(0.0);

                velocity
            }
            _ if dodge.is_dodging() => dodge.direction * DODGE_SPEED,
            _ => {
                // A downed or lost character doesn't walk (it still falls).
                let direction = if intent.incapacitated {
                    Vec3::ZERO
                } else {
                    intent.move_direction
                };

                if direction != Vec3::ZERO {
                    let target_rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));

                    transform.rotation = transform.rotation.slerp(target_rotation, 12.0 * delta);

                    direction * PLAYER_SPEED * speed_multiplier
                } else {
                    Vec3::ZERO
                }
            }
        };

        // -------------------------------------------------
        // Gravity
        // -------------------------------------------------

        vertical.velocity -= GRAVITY * delta;

        let velocity = Vec3::new(
            horizontal_velocity.x,
            vertical.velocity,
            horizontal_velocity.z,
        );

        let filter = SpatialQueryFilter::from_excluded_entities([character_entity])
            .with_mask(GameLayer::World);

        let mut hit_ground = false;

        // -------------------------------------------------
        // Move with collision
        // -------------------------------------------------

        let output = move_and_slide.move_and_slide(
            collider,
            transform.translation.adjust_precision(),
            transform.rotation.adjust_precision(),
            velocity.adjust_precision(),
            time.delta(),
            &MoveAndSlideConfig::default(),
            &filter,
            |hit| {
                // Upward-facing surface = floor
                if hit.normal.y > 0.6 {
                    hit_ground = true;
                }

                MoveAndSlideHitResponse::Accept
            },
        );

        transform.translation = output.position.f32();

        // -------------------------------------------------
        // Use Avian's projected velocity
        // -------------------------------------------------

        vertical.velocity = output.projected_velocity.y as f32;

        vertical.grounded = hit_ground;

        if vertical.grounded {
            if vertical.velocity < 0.0 {
                vertical.velocity = 0.0;
            }

            vertical.jumps_used = 0;
        }
    }
}
