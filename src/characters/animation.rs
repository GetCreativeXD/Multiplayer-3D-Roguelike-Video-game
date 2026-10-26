use std::time::Duration;

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use crate::player::{Dodge, PlayerIntent};

pub const SHARED_ANIMATION_PATH: &str = "characters/xbot/xbot_animated.glb";

const IDLE_ANIMATION: usize = 0;
const RUN_ANIMATION: usize = 1;
const JUMP_ANIMATION: usize = 2;
const DODGE_ANIMATION: usize = 3;

const ANIMATION_BLEND_MS: u64 = 120;

// The Mixamo dodge lasts about 1.17 seconds.
// Ruinbound's gameplay dodge lasts 0.40 seconds.
const JUMP_ANIMATION_DURATION: f32 = 0.80;
const DODGE_ANIMATION_DURATION: f32 = 0.65;

const JUMP_PLAYBACK_SPEED: f32 = 1.0;
const DODGE_PLAYBACK_SPEED: f32 = 2.75;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum CharacterAnimationState {
    #[default]
    Idle,
    Run,
    Jump,
    Dodge,
}

/// What a player character's animation needs to know, and nothing more:
/// whether it is walking, dodging, and how many jumps it has started.
///
/// The machine that simulates the character fills it
/// (`update_character_motion`, from `PlayerIntent` and `Dodge`); the
/// animation system only reads it. In multiplayer it is replicated, so a
/// client animates every avatar from the host's state without running any
/// movement code itself.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub struct CharacterMotion {
    pub moving: bool,
    pub dodging: bool,
    /// Total jump presses. Animation plays a jump when this changes.
    pub jump_count: u32,
}

#[derive(Component)]
pub struct CharacterAnimationOwner(Entity);

impl CharacterAnimationOwner {
    pub fn new(player: Entity) -> Self {
        Self(player)
    }

    /// The character this model belongs to.
    pub fn owner(&self) -> Entity {
        self.0
    }
}

#[derive(Component)]
pub(crate) struct CharacterAnimationController {
    state: CharacterAnimationState,
    jump_time_remaining: f32,
    dodge_time_remaining: f32,
    was_dodging: bool,
    /// The owner's `CharacterMotion::jump_count` as of last frame. `None`
    /// until first seen, so presses made before the model loaded don't
    /// play a jump.
    seen_jump_requests: Option<u32>,
}

#[derive(Resource)]
pub struct CharacterAnimations {
    graph: Handle<AnimationGraph>,
    idle: AnimationNodeIndex,
    run: AnimationNodeIndex,
    jump: AnimationNodeIndex,
    dodge: AnimationNodeIndex,
}

pub fn setup_character_animations(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let clips = [
        asset_server
            .load(GltfAssetLabel::Animation(IDLE_ANIMATION).from_asset(SHARED_ANIMATION_PATH)),
        asset_server
            .load(GltfAssetLabel::Animation(RUN_ANIMATION).from_asset(SHARED_ANIMATION_PATH)),
        asset_server
            .load(GltfAssetLabel::Animation(JUMP_ANIMATION).from_asset(SHARED_ANIMATION_PATH)),
        asset_server
            .load(GltfAssetLabel::Animation(DODGE_ANIMATION).from_asset(SHARED_ANIMATION_PATH)),
    ];

    let (graph, nodes) = AnimationGraph::from_clips(clips);

    commands.insert_resource(CharacterAnimations {
        graph: graphs.add(graph),
        idle: nodes[0],
        run: nodes[1],
        jump: nodes[2],
        dodge: nodes[3],
    });
}

pub fn initialize_character_animation_player(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    owners: Query<&CharacterAnimationOwner>,

    mut animation_players: Query<&mut AnimationPlayer>,
    animations: Res<CharacterAnimations>,
) {
    let Ok(owner) = owners.get(scene_ready.entity) else {
        return;
    };

    for descendant in children.iter_descendants(scene_ready.entity) {
        let Ok(mut animation_player) = animation_players.get_mut(descendant) else {
            continue;
        };

        let mut transitions = AnimationTransitions::new();

        transitions
            .play(&mut animation_player, animations.idle, Duration::ZERO)
            .repeat();

        commands.entity(descendant).insert((
            AnimationGraphHandle(animations.graph.clone()),
            transitions,
            CharacterAnimationOwner::new(owner.0),
            CharacterAnimationController {
                state: CharacterAnimationState::Idle,
                jump_time_remaining: 0.0,
                dodge_time_remaining: 0.0,
                was_dodging: false,
                seen_jump_requests: None,
            },
        ));
    }
}

/// Derives each simulated character's `CharacterMotion`. Only writes when
/// something changed, so an idle character isn't re-sent over the network.
pub fn update_character_motion(
    mut characters: Query<(&PlayerIntent, &Dodge, &mut CharacterMotion)>,
) {
    for (intent, dodge, mut motion) in &mut characters {
        let next = CharacterMotion {
            moving: intent.is_moving(),
            dodging: dodge.is_dodging(),
            jump_count: intent.jump_requests(),
        };
        if *motion != next {
            *motion = next;
        }
    }
}

pub fn update_character_animation(
    time: Res<Time>,
    // Any player character: the local `Player`, a remote player's body on
    // the host, or (in multiplayer) a client's replicated avatar.
    character_states: Query<&CharacterMotion>,
    animations: Res<CharacterAnimations>,
    mut animation_players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &CharacterAnimationOwner,
        &mut CharacterAnimationController,
    )>,
) {
    let delta = time.delta_secs();

    for (mut animation_player, mut transitions, owner, mut controller) in &mut animation_players {
        let Ok(motion) = character_states.get(owner.0) else {
            continue;
        };

        let movement_pressed = motion.moving;

        let jump_requests = motion.jump_count;
        let jump_pressed = controller
            .seen_jump_requests
            .is_some_and(|seen| seen != jump_requests);
        controller.seen_jump_requests = Some(jump_requests);

        controller.jump_time_remaining = (controller.jump_time_remaining - delta).max(0.0);

        controller.dodge_time_remaining = (controller.dodge_time_remaining - delta).max(0.0);

        let gameplay_dodging = motion.dodging;

        if gameplay_dodging && !controller.was_dodging {
            controller.dodge_time_remaining = DODGE_ANIMATION_DURATION;
        }

        controller.was_dodging = gameplay_dodging;

        if jump_pressed {
            controller.jump_time_remaining = JUMP_ANIMATION_DURATION;
        }

        let desired_state = if controller.dodge_time_remaining > 0.0 {
            CharacterAnimationState::Dodge
        } else if controller.jump_time_remaining > 0.0 {
            CharacterAnimationState::Jump
        } else if movement_pressed {
            CharacterAnimationState::Run
        } else {
            CharacterAnimationState::Idle
        };

        if desired_state != controller.state {
            let animation_node = match desired_state {
                CharacterAnimationState::Idle => animations.idle,
                CharacterAnimationState::Run => animations.run,
                CharacterAnimationState::Jump => animations.jump,
                CharacterAnimationState::Dodge => animations.dodge,
            };

            let blend_duration = if desired_state == CharacterAnimationState::Dodge {
                Duration::from_millis(40)
            } else {
                Duration::from_millis(ANIMATION_BLEND_MS)
            };

            let active_animation =
                transitions.play(&mut animation_player, animation_node, blend_duration);

            match desired_state {
                CharacterAnimationState::Idle | CharacterAnimationState::Run => {
                    active_animation.repeat();
                }

                CharacterAnimationState::Jump => {
                    active_animation.set_speed(JUMP_PLAYBACK_SPEED);
                }

                CharacterAnimationState::Dodge => {
                    active_animation.set_speed(DODGE_PLAYBACK_SPEED);
                }
            }

            controller.state = desired_state;
        }
    }
}
