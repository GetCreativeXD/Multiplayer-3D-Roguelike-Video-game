use bevy::prelude::*;

use crate::items::{ItemDefinitionId, ItemMetadata};

// =========================================================
// GEAR TIER
// =========================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
// Shown on loot and in inventory snapshots that co-op replicates.
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum GearTier {
    Common,
    Rare,
    Epic,
    Legendary,
}

// =========================================================
// GEAR ARCHETYPE
// =========================================================

/// One of Ruinbound's three build-defining gear archetypes (see
/// RUINBOUND_GEAR_ARCHETYPES.md). Archetype does not lock a class or
/// weapon to a piece of gear -- it only describes whether the piece leans
/// toward ability damage (Arcanist), basic-attack damage (Striker), or
/// durability (Bulwark).
///
/// Legacy/placeholder armor created via `Armor::new` (the original Iron /
/// Gatewarden / Ruinforged sets) predates this system and has no
/// archetype, so `Armor::archetype` is `Option<GearArchetype>`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
// Part of `run_setup::DropPoolCategoryId`, which co-op sends.
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum GearArchetype {
    /// Light armor. Lowest defense of the three archetypes; leans on
    /// ability damage, cooldown recovery, and ability-resource generation.
    Arcanist,
    /// Medium armor. Leans on basic-attack damage, attack speed, and
    /// critical hits.
    Striker,
    /// Heavy armor. Highest defense; leans on damage that scales from
    /// defensive stats and defensive actions (blocking, mitigating,
    /// resisting stagger).
    Bulwark,
}

// =========================================================
// WEAPON TYPE
// =========================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
// Multiplayer sends a joined player's weapon choice to the host
// (`net::loadout`).
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum WeaponType {
    Longsword,
    Greatsword,
    Bow,
    Hammer,
    /// Slow, powerful ranged attacks -- distinct from Bow (see
    /// WEAPONS.md's Crossbow section). Basic-attack bolts travel faster
    /// than an arrow (`weapons::CROSSBOW_BOLT_SPEED` > `weapons::ARROW_SPEED`)
    /// but the weapon fires less often and hits much harder.
    Crossbow,
    /// Magic-focused weapon built around its own homing bolt rather than
    /// a raw damage race (see WEAPONS.md's Spellbook section and
    /// `weapons::spellbook`). Every tier grants some flat Ability Power
    /// (`Weapon::ability_power_bonus`) so it also strengthens the
    /// player's class abilities, not just its own casts.
    Spellbook,
    /// Extremely fast, short-range melee -- see WEAPONS.md's Daggers
    /// section and `weapons::daggers`. Lowest damage-per-hit and lowest
    /// stagger of any melee weapon in the roster, made up for by the
    /// fastest cooldown and a moveset built around mobility, backstab
    /// bonus damage, and bleed/poison effects rather than raw stagger or
    /// knockback.
    Daggers,
}

// =========================================================
// WEAPON
// =========================================================

#[derive(Component, Clone, Copy, Debug)]
pub struct Weapon {
    pub metadata: ItemMetadata,
    pub weapon_type: WeaponType,
    pub tier: GearTier,

    pub damage: f32,
    pub range: f32,
    pub cooldown: f32,
    pub knockback: f32,
    pub stagger_strength: u32,
    pub stagger_duration: f32,
    pub attack_angle: f32,

    /// Flat Ability Power this weapon grants just by being equipped,
    /// summed into `Equipment::total_ability_power_bonus` alongside
    /// armor's own bonus. Zero for every weapon except Spellbook (see
    /// WEAPONS.md's Apprentice's Grimoire "Basic Arcana": "slightly
    /// increases ability damage") -- a future Wand/Staff could set this
    /// too rather than each magic weapon inventing its own stat.
    pub ability_power_bonus: f32,
    /// Multiplies this weapon's tier ability cooldown once it's set
    /// (`weapons::abilities::weapon_ability_input`). `1.0` (no change)
    /// for every weapon except Spellbook, whose "Basic Arcana" passive
    /// also "slightly reduces ability cooldowns."
    pub ability_cooldown_multiplier: f32,
}

impl Weapon {
    pub fn longsword(tier: GearTier) -> Self {
        Self {
            metadata: ItemMetadata::new(
                weapon_definition_id(WeaponType::Longsword, tier),
                weapon_display_name(WeaponType::Longsword, tier),
            ),
            weapon_type: WeaponType::Longsword,
            tier,
            damage: 20.0,
            range: 2.8,
            cooldown: 0.45,
            knockback: 0.6,
            stagger_strength: 1,
            stagger_duration: 0.30,
            attack_angle: 0.55,
            ability_power_bonus: 0.0,
            ability_cooldown_multiplier: 1.0,
        }
    }

    pub fn greatsword(tier: GearTier) -> Self {
        Self {
            metadata: ItemMetadata::new(
                weapon_definition_id(WeaponType::Greatsword, tier),
                weapon_display_name(WeaponType::Greatsword, tier),
            ),
            weapon_type: WeaponType::Greatsword,
            tier,
            damage: 32.0,
            range: 3.2,
            cooldown: 0.75,
            knockback: 1.0,
            stagger_strength: 2,
            stagger_duration: 0.45,
            attack_angle: 0.45,
            ability_power_bonus: 0.0,
            ability_cooldown_multiplier: 1.0,
        }
    }

    pub fn bow(tier: GearTier) -> Self {
        Self {
            metadata: ItemMetadata::new(
                weapon_definition_id(WeaponType::Bow, tier),
                weapon_display_name(WeaponType::Bow, tier),
            ),
            weapon_type: WeaponType::Bow,
            tier,
            damage: 16.0,
            range: 25.0,
            cooldown: 0.55,
            knockback: 0.15,
            stagger_strength: 0,
            stagger_duration: 0.10,
            attack_angle: 0.99,
            ability_power_bonus: 0.0,
            ability_cooldown_multiplier: 1.0,
        }
    }

    /// Slower and more committed than every other melee weapon (WEAPONS.md:
    /// "Hammer should be slower and more committed than Longsword"), with
    /// the roster's highest ordinary stagger_strength and knockback. Like
    /// every other weapon, base stats don't change with tier -- only the
    /// weapon ability does.
    pub fn hammer(tier: GearTier) -> Self {
        Self {
            metadata: ItemMetadata::new(
                weapon_definition_id(WeaponType::Hammer, tier),
                weapon_display_name(WeaponType::Hammer, tier),
            ),
            weapon_type: WeaponType::Hammer,
            tier,
            damage: 40.0,
            range: 2.9,
            cooldown: 0.95,
            knockback: 1.7,
            stagger_strength: 3,
            stagger_duration: 0.55,
            attack_angle: 0.35,
            ability_power_bonus: 0.0,
            ability_cooldown_multiplier: 1.0,
        }
    }

    /// Slow, powerful, and deliberate (WEAPONS.md's Crossbow section):
    /// hits harder than Bow and its bolts travel faster
    /// (`weapons::CROSSBOW_BOLT_SPEED` vs `weapons::ARROW_SPEED`), but
    /// fires less often and carries a little real stagger, rewarding
    /// commitment over kiting. Like every other weapon, base stats don't
    /// change with tier -- only the weapon ability does.
    pub fn crossbow(tier: GearTier) -> Self {
        Self {
            metadata: ItemMetadata::new(
                weapon_definition_id(WeaponType::Crossbow, tier),
                weapon_display_name(WeaponType::Crossbow, tier),
            ),
            weapon_type: WeaponType::Crossbow,
            tier,
            damage: 26.0,
            range: 27.0,
            cooldown: 1.0,
            knockback: 0.35,
            stagger_strength: 1,
            stagger_duration: 0.2,
            attack_angle: 0.99,
            ability_power_bonus: 0.0,
            ability_cooldown_multiplier: 1.0,
        }
    }

    /// WEAPONS.md's Spellbook tiers are written as passives layered onto
    /// one shared identity rather than a mutually-exclusive ability per
    /// tier ("Basic Arcana... establishes the spellbook's ability-focused
    /// identity"), so unlike every other weapon family, that baseline
    /// passive is baked into every Spellbook tier's base stats here
    /// (`ability_power_bonus` / `ability_cooldown_multiplier`) rather
    /// than only appearing at Common. Rare/Epic/Legendary then layer
    /// their own unique weapon ability on top (see
    /// `weapons::spellbook`) -- Scorching Spells, Echoing Casts, and the
    /// three Legendary Spellbooks.
    pub fn spellbook(tier: GearTier) -> Self {
        Self {
            metadata: ItemMetadata::new(
                weapon_definition_id(WeaponType::Spellbook, tier),
                weapon_display_name(WeaponType::Spellbook, tier),
            ),
            weapon_type: WeaponType::Spellbook,
            tier,
            damage: 14.0,
            range: 22.0,
            cooldown: 0.65,
            knockback: 0.1,
            stagger_strength: 0,
            stagger_duration: 0.1,
            attack_angle: 0.99,
            ability_power_bonus: super::SPELLBOOK_BASE_ABILITY_POWER,
            ability_cooldown_multiplier: super::SPELLBOOK_ABILITY_COOLDOWN_MULTIPLIER,
        }
    }

    /// Fastest, shortest-range, lowest-stagger melee weapon in the
    /// roster (WEAPONS.md: "Extremely fast attacks, short range...
    /// Daggers should have high sustained damage but require the player
    /// to stay close and avoid taking hits"). Zero base stagger --
    /// Daggers lean entirely on mobility and bonus damage rather than
    /// interrupting enemies. Like every other weapon, base stats don't
    /// change with tier -- only the weapon ability does.
    pub fn daggers(tier: GearTier) -> Self {
        Self {
            metadata: ItemMetadata::new(
                weapon_definition_id(WeaponType::Daggers, tier),
                weapon_display_name(WeaponType::Daggers, tier),
            ),
            weapon_type: WeaponType::Daggers,
            tier,
            damage: 11.0,
            range: 2.2,
            cooldown: 0.3,
            knockback: 0.2,
            stagger_strength: 0,
            stagger_duration: 0.1,
            attack_angle: 0.55,
            ability_power_bonus: 0.0,
            ability_cooldown_multiplier: 1.0,
        }
    }

    /// Builds a weapon from a `WeaponType` chosen elsewhere (currently:
    /// the run-setup screen's starting-weapon choice -- see
    /// `run_setup::RunConfig::weapon` and
    /// `rooms::fallen_gate::setup_fallen_gate_player`). Kept as
    /// one small dispatcher over the constructors above rather than
    /// duplicating their stats, so a new weapon family only ever needs
    /// its own `Weapon::<name>` constructor plus one new match arm here.
    pub fn of_type(weapon_type: WeaponType, tier: GearTier) -> Self {
        match weapon_type {
            WeaponType::Longsword => Self::longsword(tier),
            WeaponType::Greatsword => Self::greatsword(tier),
            WeaponType::Bow => Self::bow(tier),
            WeaponType::Hammer => Self::hammer(tier),
            WeaponType::Crossbow => Self::crossbow(tier),
            WeaponType::Spellbook => Self::spellbook(tier),
            WeaponType::Daggers => Self::daggers(tier),
        }
    }

    /// Builds one specific *named* Legendary weapon (Saint's Nail, The
    /// Hollow Gospel, etc.) -- same base stats `of_type(weapon_type,
    /// Legendary)` would produce, but with its own definition id and
    /// display name so the ability dispatch
    /// (`weapons::abilities::weapon_ability_input`) and the loot system
    /// can tell exactly which Legendary this is, the same way
    /// `loot::gear`'s named archetype pieces are told apart by
    /// definition id. Every current and future named Legendary weapon
    /// should be built this way instead of each family inventing its own
    /// constructor.
    pub fn legendary(
        weapon_type: WeaponType,
        definition_id: &'static str,
        name: &'static str,
    ) -> Self {
        Self {
            metadata: ItemMetadata::new(definition_id, name),
            ..Self::of_type(weapon_type, GearTier::Legendary)
        }
    }
}

// =========================================================
// PLAYER COMBAT STATE
// =========================================================

#[derive(Component)]
pub struct PlayerCombat {
    pub attack_cooldown: f32,
    pub attack_timer: f32,
    pub pending_attack: Option<WeaponType>,

    pub ability_cooldown: f32,
    pub ability_timer: f32,
    pub pending_ability: Option<WeaponType>,
}

impl Default for PlayerCombat {
    fn default() -> Self {
        Self {
            attack_cooldown: 0.0,
            attack_timer: 0.0,
            pending_attack: None,

            ability_cooldown: 0.0,
            ability_timer: 0.0,
            pending_ability: None,
        }
    }
}

// =========================================================
// PER-WEAPON-ABILITY RUNTIME STATE
// =========================================================

/// Small per-player counters and cooldowns for weapon abilities that need
/// to remember something across casts -- Grimoire of Echoes' repeat-cast
/// counter, Saint's Nail's Final Judgment readiness, Ruinpiercer's
/// Cleansing Strike cooldown, Saint's Testament's Grace stacks, and The
/// Living Codex's repeat-cast counter. One shared component rather than
/// one marker component per weapon, the same way `PlayerCombat` holds
/// every weapon family's basic-attack/ability timers instead of each
/// family getting its own. Inserted once, for free, alongside
/// `Equipment` (`weapons::inventory::initialize_player_inventory`) --
/// mirrors `EnemyDebuffs` being given to every enemy for free, empty,
/// from `enemies::spawn_enemy_root`.
#[derive(Component, Debug)]
pub(crate) struct WeaponAbilityState {
    /// Seconds until Saint's Nail's Final Judgment bonus is ready again.
    /// Starts at `0.0` (ready) via `Default`.
    pub(crate) saints_nail_judgment_cooldown: f32,
    /// Seconds until Ruinpiercer's Cleansing Strike can trigger again.
    pub(crate) ruinpiercer_cleansing_cooldown: f32,
    /// Saint's Testament's Grace stacks, built by Grace Bolt hits.
    pub(crate) grace_stacks: u32,
    /// The Living Codex's consecutive-cast counter for Adaptive Cast.
    pub(crate) adaptive_cast_stacks: u32,
    /// Grimoire of Echoes' consecutive-cast counter for Echoing Cast.
    pub(crate) echoing_cast_counter: u32,
    /// Kingslayer's Execution stacks, built by backstabbing a Deathmarked
    /// enemy (see `weapons::daggers`). At
    /// `weapons::daggers::KINGSLAYER_MAX_EXECUTION_STACKS`, the next
    /// backstab consumes every stack for a bonus burst.
    pub(crate) kingslayer_execution_stacks: u32,
    /// Seconds until Kingslayer's Ruthless Opening passive is ready
    /// again. Starts at `0.0` (ready) via `Default`, same shape as
    /// `saints_nail_judgment_cooldown` above.
    pub(crate) kingslayer_ruthless_opening_cooldown: f32,
    /// True for exactly one dagger basic attack after Phantom Blades'
    /// Phantom Step (Epic Daggers) lands -- see `weapons::daggers`.
    /// Consumed (set back to `false`) the moment that attack resolves.
    pub(crate) phantom_step_empowered: bool,
    /// Dancer in the Dark's Shadow Charge stacks, built by Perfect
    /// Evasion (dodging through an enemy -- see `weapons::daggers`). At
    /// `weapons::daggers::DANCER_MAX_SHADOW_CHARGES`, the next dagger
    /// basic attack spawns a temporary shadow duplicate and resets this
    /// to zero.
    pub(crate) dancer_shadow_charges: u32,
    /// True once the current dodge has already granted a Shadow Charge,
    /// so one dodge -- however many enemies it passes near -- can only
    /// ever grant one stack. Reset whenever the player is not currently
    /// dodging (see `weapons::daggers::update_perfect_evasion`).
    pub(crate) dancer_shadow_charge_armed: bool,
}

impl Default for WeaponAbilityState {
    fn default() -> Self {
        Self {
            saints_nail_judgment_cooldown: 0.0,
            ruinpiercer_cleansing_cooldown: 0.0,
            grace_stacks: 0,
            adaptive_cast_stacks: 0,
            echoing_cast_counter: 0,
            kingslayer_execution_stacks: 0,
            kingslayer_ruthless_opening_cooldown: 0.0,
            phantom_step_empowered: false,
            dancer_shadow_charges: 0,
            dancer_shadow_charge_armed: false,
        }
    }
}

/// Counts down `WeaponAbilityState`'s cooldown fields. Registered by
/// `weapons::WeaponPlugin`. The stack/counter fields need no ticking --
/// they're reset directly by the abilities that consume them.
pub(crate) fn tick_weapon_ability_state(
    time: Res<Time>,
    mut states: Query<&mut WeaponAbilityState>,
) {
    let delta = time.delta_secs();
    for mut state in &mut states {
        state.saints_nail_judgment_cooldown =
            (state.saints_nail_judgment_cooldown - delta).max(0.0);
        state.ruinpiercer_cleansing_cooldown =
            (state.ruinpiercer_cleansing_cooldown - delta).max(0.0);
        state.kingslayer_ruthless_opening_cooldown =
            (state.kingslayer_ruthless_opening_cooldown - delta).max(0.0);
    }
}

// =========================================================
// PROJECTILES
// =========================================================

#[derive(Component)]
pub(crate) struct Projectile {
    pub(crate) velocity: Vec3,
    pub(crate) damage: f32,
    pub(crate) knockback: f32,
    pub(crate) stagger_strength: u32,
    pub(crate) stagger_duration: f32,
    pub(crate) lifetime: f32,
    pub(crate) piercing: bool,
    pub(crate) hit_entities: Vec<Entity>,

    // What kind of projectile this is.
    pub(crate) kind: ProjectileKind,

    // Used by homing projectiles such as Splinter fragments and the
    // Spellbook's Arcane Bolt family.
    pub(crate) target: Option<Entity>,
}

#[derive(Component)]
pub(crate) struct PlayerProjectile;

/// The player character that fired a projectile, so its hit is credited to
/// that player (e.g. their class's basic-attack hit tracker) rather than to
/// whichever `Player` this machine has. Set by the weapon basic attacks
/// (`bow::fire_bow`, `crossbow::fire_crossbow`, `spellbook::fire_arcane_bolt`).
/// Projectiles without one (abilities, split fragments) keep the old
/// behaviour of crediting the local `Player`.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct ProjectileOwner(pub(crate) Entity);

#[derive(Component)]
pub(crate) struct RicochetProjectile {
    pub(crate) ricochets_remaining: u32,
    pub(crate) split_lockout: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectileKind {
    Normal,
    Piercing,
    Splinter,
    SplinterFragment,
    PhantomCleave,
    Ricochet,
    Afterimage,
    // Warden of Shades' Shadow Stag -- see characters::warden_abilities.
    // A wide, piercing line hitbox, same shape family as PhantomCleave,
    // but sized like a charging animal rather than a blade sweep (see
    // move_projectiles_with_collision's collider selection).
    ShadowStag,
    // Runesage's Rune Blast -- see characters::runesage_abilities. A
    // fast single-target bolt that also splashes lighter damage onto
    // any other enemy near its impact point (see
    // move_projectiles_with_collision's RuneBlast-specific block).
    RuneBlast,
    // Runesage's Runic Surge -- see characters::runesage_abilities. A
    // wide, piercing wave in the same shape family as ShadowStag /
    // PhantomCleave, but shorter-ranged and faster.
    RunicSurge,
    // Hammer's Rare ability, Seismic Strike -- see weapons::abilities::hammer.
    // A low, wide ground fissure that pierces through enemies until it
    // hits world geometry.
    SeismicFissure,
    // Gravemind's Ruin Bolt -- see characters::gravemind_abilities. A
    // heavy single-target corruption bolt that also splashes lighter
    // damage onto any other enemy near its impact point (the same
    // "explosive" pattern RuneBlast already established -- see
    // move_projectiles_with_collision's RuinBolt-specific block).
    RuinBolt,
    // Crossbow's Rare ability, Gravewind Bolt -- see weapons::crossbow.
    // Pierces enemies with a damage falloff per additional enemy hit
    // (see crossbow::gravewind_damage_multiplier), unlike Piercing's
    // flat multiplier. Stops at world geometry like any other bolt.
    GravewindBolt,
    // Crossbow's Epic ability, Armorbreaker Shot -- see weapons::crossbow.
    // On hit, grants the target a brief increased-damage-taken window
    // (see combat::enemy_debuffs::DebuffSource::ArmorbreakerShot).
    ArmorbreakerBolt,
    // Spellbook's basic attack and Common ability (Focused Cast) -- see
    // weapons::spellbook. Homes in on `Projectile::target` when one is
    // set (see `ProjectileKind::homes`); otherwise flies straight like
    // any other bolt.
    ArcaneBolt,
    // Ember Codex's Scorching Spells (Rare Spellbook) -- see
    // weapons::spellbook. Same homing behavior as ArcaneBolt; on hit it
    // additionally applies Scorch (combat::dot::DotSource::Scorch).
    ScorchBolt,
    // Saint's Nail's Consecrated Bolt (Legendary Crossbow) -- see
    // weapons::crossbow. On hit, grants the target Consecrated Mark
    // (combat::enemy_debuffs::DebuffSource::ConsecratedMark).
    ConsecratedBolt,
    // Ruinpiercer's Corruption-Piercing Bolt (Legendary Crossbow) -- see
    // weapons::crossbow. Pierces with a damage falloff per additional
    // enemy hit (own falloff constant, independent of Gravewind's) and
    // tags whatever it damages with `crossbow::RuinpiercerMark` for
    // Cleansing Strike's kill-attribution.
    CorruptionPiercingBolt,
    // Bellringer's Resonant Bolt (Legendary Crossbow) -- see
    // weapons::crossbow. Builds Resonance stacks on the struck enemy;
    // the hit that fills Resonance to maximum triggers a shockwave.
    ResonantBolt,
    // Saint's Testament's Grace Bolt (Legendary Spellbook) -- see
    // weapons::spellbook. Homes like ArcaneBolt; on hit it builds the
    // caster's Grace stacks (WeaponAbilityState::grace_stacks).
    GraceBolt,
    // Widow's Kiss's Venomous Barrage (Legendary Daggers) -- see
    // weapons::daggers. A thrown dagger, one of a spread of several,
    // that applies Venom and Bleed on hit rather than homing (unlike
    // every other special Legendary projectile above).
    VenomousDagger,
}

impl ProjectileKind {
    /// True for every projectile kind that should steer toward
    /// `Projectile::target` each frame instead of flying in a straight
    /// line -- see `weapons::projectiles::homing::home_locked_projectiles`.
    /// Any future homing projectile just adds itself here.
    pub(crate) fn homes(self) -> bool {
        matches!(
            self,
            ProjectileKind::SplinterFragment
                | ProjectileKind::ArcaneBolt
                | ProjectileKind::ScorchBolt
                | ProjectileKind::GraceBolt
        )
    }
}

// =========================================================
// ATTACK VISUAL
// =========================================================

#[derive(Component)]
pub(crate) struct AttackVisual {
    pub(crate) lifetime: f32,
}

// =========================================================
// INVENTORY / EQUIPMENT
// =========================================================

pub const INVENTORY_CAPACITY: usize = 10;
pub const WEAPON_SLOT_COUNT: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "multiplayer", derive(serde::Serialize, serde::Deserialize))]
pub enum ArmorSlot {
    Head,
    Chest,
    Hands,
    Legs,
    Feet,
}

// =========================================================
// GEAR EFFECT (descriptive -- see note below)
// =========================================================

/// A named passive/proc effect carried by one piece of archetype gear
/// (Runebound Hood, Spellweaver Crown, Crown of the Final Word, etc).
///
/// `name` is the proper-noun label used by Epic and Legendary pieces
/// (e.g. "Arcane Rhythm"); Common and Rare pieces have no proper name in
/// the design doc, so `name` is left empty and only `description` is set.
///
/// IMPORTANT: this is currently *descriptive metadata only*, shown in the
/// inventory tooltip. None of these effects fire in gameplay yet. Most of
/// them need systems Ruinbound does not have today -- an ability-resource
/// pool, barriers, blocking, stagger-resistance stacks, momentum/stack
/// tracking -- which RUINBOUND_GEAR_ARCHETYPES.md itself lists under
/// "Future Design Work" as still undecided. Wiring up each effect is
/// separate future work and should land one confirmed mechanic at a time,
/// the same way the Saint's Fist Slam did.
#[derive(Clone, Copy, Debug)]
pub struct GearEffect {
    pub name: &'static str,
    pub description: &'static str,
}

impl GearEffect {
    /// No unique effect (legacy gear, or a slot with only a plain stat).
    pub const NONE: GearEffect = GearEffect {
        name: "",
        description: "",
    };

    pub const fn new(name: &'static str, description: &'static str) -> Self {
        Self { name, description }
    }

    pub fn is_present(&self) -> bool {
        !self.description.is_empty()
    }
}

// =========================================================
// ARMOR
// =========================================================

#[derive(Clone, Copy, Debug)]
pub struct Armor {
    pub metadata: ItemMetadata,
    pub slot: ArmorSlot,
    pub tier: GearTier,
    pub defense: f32,
    pub health_bonus: f32,
    /// Flat "Ability Power" points this piece grants. Read by
    /// `Equipment::ability_power_multiplier`, which every class ability
    /// that deals damage or grants a magnitude-based effect (shield size,
    /// buff strength, etc.) should multiply its base numbers by -- see
    /// that method's doc comment for the exact formula. Zero for
    /// legacy/placeholder armor and for archetype gear whose unique
    /// effect hasn't been itemized with a real number yet (see the
    /// `GearEffect` doc comment below); use `with_ability_power_bonus` to
    /// give a specific piece a real value once its effect is wired up.
    pub ability_power_bonus: f32,
    /// `None` for legacy/placeholder armor created via `Armor::new`.
    /// `Some(_)` for gear created via `Armor::from_gear` (the Arcanist /
    /// Striker / Bulwark sets in `src/loot/gear/`).
    pub archetype: Option<GearArchetype>,
    /// Flavor/rules text for the piece's unique effect, if any.
    /// See the `GearEffect` doc comment: descriptive only, not yet wired
    /// into gameplay.
    pub effect: GearEffect,
}

impl Armor {
    /// Builds a legacy/placeholder armor piece (the original Iron /
    /// Gatewarden / Ruinforged / Legendary generic sets). Unchanged from
    /// before the gear-archetype system: no archetype tag, no effect text.
    pub fn new(slot: ArmorSlot, tier: GearTier, defense: f32, health_bonus: f32) -> Self {
        Self {
            metadata: ItemMetadata::new(
                armor_definition_id(slot, tier),
                armor_display_name(slot, tier),
            ),
            slot,
            tier,
            defense,
            health_bonus,
            ability_power_bonus: 0.0,
            archetype: None,
            effect: GearEffect::NONE,
        }
    }

    /// Builds one piece of archetype gear (Arcanist / Striker / Bulwark).
    /// Used by `src/loot/gear/{arcanist,striker,bulwark}.rs`, which supply
    /// their own definition id and display name (the generic
    /// `armor_definition_id`/`armor_display_name` helpers below only know
    /// about the legacy generic sets).
    pub fn from_gear(
        definition_id: &'static str,
        name: &'static str,
        slot: ArmorSlot,
        tier: GearTier,
        archetype: GearArchetype,
        defense: f32,
        health_bonus: f32,
        effect: GearEffect,
    ) -> Self {
        Self {
            metadata: ItemMetadata::new(definition_id, name),
            slot,
            tier,
            defense,
            health_bonus,
            ability_power_bonus: 0.0,
            archetype: Some(archetype),
            effect,
        }
    }

    /// Overrides this piece's Ability Power with a real value. Chain onto
    /// `build(...)`/`from_gear(...)` for the specific items whose
    /// `GearEffect` text has actually been wired up to the numeric
    /// ability-power stat -- most archetype gear stays at the 0.0 default
    /// until its own effect gets a real implementation pass (see the
    /// `ability_power_bonus` field doc comment).
    pub fn with_ability_power_bonus(mut self, ability_power_bonus: f32) -> Self {
        self.ability_power_bonus = ability_power_bonus;
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub enum InventoryItem {
    Weapon(Weapon),
    Armor(Armor),
}

#[derive(Component, Debug)]
pub struct PlayerInventory {
    pub slots: [Option<InventoryItem>; INVENTORY_CAPACITY],
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self {
            slots: [None; INVENTORY_CAPACITY],
        }
    }
}

impl PlayerInventory {
    pub fn add(&mut self, item: InventoryItem) -> Result<usize, InventoryItem> {
        let Some(index) = self.slots.iter().position(Option::is_none) else {
            return Err(item);
        };

        self.slots[index] = Some(item);
        Ok(index)
    }

    pub fn occupied_slots(&self) -> usize {
        self.slots.iter().flatten().count()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ArmorEquipment {
    pub head: Option<Armor>,
    pub chest: Option<Armor>,
    pub hands: Option<Armor>,
    pub legs: Option<Armor>,
    pub feet: Option<Armor>,
}

impl ArmorEquipment {
    pub fn get_mut(&mut self, slot: ArmorSlot) -> &mut Option<Armor> {
        match slot {
            ArmorSlot::Head => &mut self.head,
            ArmorSlot::Chest => &mut self.chest,
            ArmorSlot::Hands => &mut self.hands,
            ArmorSlot::Legs => &mut self.legs,
            ArmorSlot::Feet => &mut self.feet,
        }
    }
}

#[derive(Component, Debug)]
pub struct Equipment {
    pub weapon_slots: [Option<Weapon>; WEAPON_SLOT_COUNT],
    pub active_weapon_slot: usize,
    pub armor: ArmorEquipment,
    /// Flat Ability Power granted by the run's active relics
    /// (`run_setup::RunModifiers::bonus_ability_power` --
    /// `RunEffect::BonusAbilityPower`), set once when the starting
    /// `Equipment` is created and unaffected by gear changes afterward.
    /// Summed into `total_ability_power_bonus` alongside armor's own
    /// `ability_power_bonus`, so it flows through
    /// `ability_power_multiplier` exactly like a gear-granted bonus would.
    pub bonus_ability_power: f32,
}

impl Equipment {
    pub fn with_starter_weapon(weapon: Weapon, bonus_ability_power: f32) -> Self {
        Self {
            weapon_slots: [Some(weapon), None, None],
            active_weapon_slot: 0,
            armor: ArmorEquipment::default(),
            bonus_ability_power,
        }
    }

    pub fn active_weapon(&self) -> Option<&Weapon> {
        self.weapon_slots
            .get(self.active_weapon_slot)
            .and_then(Option::as_ref)
    }

    pub fn first_empty_weapon_slot(&self) -> Option<usize> {
        self.weapon_slots.iter().position(Option::is_none)
    }

    pub fn equip_armor(&mut self, armor: Armor) -> Option<Armor> {
        self.armor.get_mut(armor.slot).replace(armor)
    }

    pub fn total_defense(&self) -> f32 {
        [
            self.armor.head,
            self.armor.chest,
            self.armor.hands,
            self.armor.legs,
            self.armor.feet,
        ]
        .into_iter()
        .flatten()
        .map(|armor| armor.defense)
        .sum()
    }

    pub fn total_health_bonus(&self) -> f32 {
        [
            self.armor.head,
            self.armor.chest,
            self.armor.hands,
            self.armor.legs,
            self.armor.feet,
        ]
        .into_iter()
        .flatten()
        .map(|armor| armor.health_bonus)
        .sum()
    }

    pub fn damage_reduction_percent(&self) -> f32 {
        let defense = self.total_defense().max(0.0);
        if defense <= 0.0 {
            0.0
        } else {
            defense / (100.0 + defense) * 100.0
        }
    }

    /// Sums `ability_power_bonus` across every equipped armor piece, the
    /// currently active weapon (`Weapon::ability_power_bonus` -- today
    /// only Spellbook grants any), plus any flat `bonus_ability_power`
    /// granted by the run's relics.
    pub fn total_ability_power_bonus(&self) -> f32 {
        let armor_bonus: f32 = [
            self.armor.head,
            self.armor.chest,
            self.armor.hands,
            self.armor.legs,
            self.armor.feet,
        ]
        .into_iter()
        .flatten()
        .map(|armor| armor.ability_power_bonus)
        .sum();

        let weapon_bonus = self
            .active_weapon()
            .map_or(0.0, |weapon| weapon.ability_power_bonus);

        armor_bonus + weapon_bonus + self.bonus_ability_power
    }

    /// Ruinbound's shared "normal ability scaling" formula: every class
    /// ability that deals damage or grants a magnitude-based effect
    /// (shield size, buff strength, spectral-strike damage, etc.) should
    /// multiply its base value by this. 100 Ability Power = double
    /// effect; 0 Ability Power (no gear granting any yet) = unchanged.
    /// This mirrors `damage_reduction_percent` in spirit -- one shared,
    /// gear-agnostic formula that every current and future class ability
    /// reads instead of each one inventing its own scaling.
    pub fn ability_power_multiplier(&self) -> f32 {
        1.0 + self.total_ability_power_bonus().max(0.0) / 100.0
    }

    /// True if the given gear piece (by definition id) is currently
    /// equipped in any armor slot. Lets gear-triggered effects (see
    /// `weapons::armor_procs`) ask "does the player have item X on right
    /// now" without duplicating the five-slot iteration used above.
    pub fn has_armor_piece(&self, definition_id: ItemDefinitionId) -> bool {
        [
            self.armor.head,
            self.armor.chest,
            self.armor.hands,
            self.armor.legs,
            self.armor.feet,
        ]
        .into_iter()
        .flatten()
        .any(|armor| armor.metadata.definition_id == definition_id)
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct WeaponPickup {
    pub weapon: Weapon,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct ArmorPickup {
    pub armor: Armor,
}

fn weapon_definition_id(weapon_type: WeaponType, tier: GearTier) -> &'static str {
    match (weapon_type, tier) {
        (WeaponType::Longsword, GearTier::Common) => "weapon.longsword.common",
        (WeaponType::Longsword, GearTier::Rare) => "weapon.longsword.rare",
        (WeaponType::Longsword, GearTier::Epic) => "weapon.longsword.epic",
        (WeaponType::Longsword, GearTier::Legendary) => "weapon.longsword.legendary",
        (WeaponType::Greatsword, GearTier::Common) => "weapon.greatsword.common",
        (WeaponType::Greatsword, GearTier::Rare) => "weapon.greatsword.rare",
        (WeaponType::Greatsword, GearTier::Epic) => "weapon.greatsword.epic",
        (WeaponType::Greatsword, GearTier::Legendary) => "weapon.greatsword.legendary",
        (WeaponType::Bow, GearTier::Common) => "weapon.bow.common",
        (WeaponType::Bow, GearTier::Rare) => "weapon.bow.rare",
        (WeaponType::Bow, GearTier::Epic) => "weapon.bow.epic",
        (WeaponType::Bow, GearTier::Legendary) => "weapon.bow.legendary",
        (WeaponType::Hammer, GearTier::Common) => "weapon.hammer.common",
        (WeaponType::Hammer, GearTier::Rare) => "weapon.hammer.rare",
        (WeaponType::Hammer, GearTier::Epic) => "weapon.hammer.epic",
        (WeaponType::Hammer, GearTier::Legendary) => "weapon.hammer.legendary",
        // Generic placeholder ids -- every actual Crossbow/Spellbook
        // Legendary is built via `Weapon::legendary`, which immediately
        // overrides `metadata` with its own unique id/name, so these two
        // arms are only ever seen as the base template `Weapon::of_type`
        // hands to `Weapon::legendary` before that override happens.
        (WeaponType::Crossbow, GearTier::Common) => "weapon.crossbow.common",
        (WeaponType::Crossbow, GearTier::Rare) => "weapon.crossbow.rare",
        (WeaponType::Crossbow, GearTier::Epic) => "weapon.crossbow.epic",
        (WeaponType::Crossbow, GearTier::Legendary) => "weapon.crossbow.legendary",
        (WeaponType::Spellbook, GearTier::Common) => "weapon.spellbook.common",
        (WeaponType::Spellbook, GearTier::Rare) => "weapon.spellbook.rare",
        (WeaponType::Spellbook, GearTier::Epic) => "weapon.spellbook.epic",
        (WeaponType::Spellbook, GearTier::Legendary) => "weapon.spellbook.legendary",
        // Generic placeholder ids -- every actual Daggers Legendary is
        // built via `Weapon::legendary`, which immediately overrides
        // `metadata` with its own unique id/name (see the matching note
        // on Crossbow/Spellbook's Legendary arms above).
        (WeaponType::Daggers, GearTier::Common) => "weapon.daggers.common",
        (WeaponType::Daggers, GearTier::Rare) => "weapon.daggers.rare",
        (WeaponType::Daggers, GearTier::Epic) => "weapon.daggers.epic",
        (WeaponType::Daggers, GearTier::Legendary) => "weapon.daggers.legendary",
    }
}

fn weapon_display_name(weapon_type: WeaponType, tier: GearTier) -> &'static str {
    match (weapon_type, tier) {
        (WeaponType::Longsword, GearTier::Common) => "Worn Longsword",
        (WeaponType::Longsword, GearTier::Rare) => "Knight's Longsword",
        (WeaponType::Longsword, GearTier::Epic) => "Riftsteel Longsword",
        (WeaponType::Longsword, GearTier::Legendary) => "Legendary Longsword",
        (WeaponType::Greatsword, GearTier::Common) => "Iron Greatsword",
        (WeaponType::Greatsword, GearTier::Rare) => "Executioner's Greatsword",
        (WeaponType::Greatsword, GearTier::Epic) => "Phantom Greatsword",
        (WeaponType::Greatsword, GearTier::Legendary) => "Legendary Greatsword",
        (WeaponType::Bow, GearTier::Common) => "Hunting Bow",
        (WeaponType::Bow, GearTier::Rare) => "Splinter Bow",
        (WeaponType::Bow, GearTier::Epic) => "Ricochet Bow",
        (WeaponType::Bow, GearTier::Legendary) => "Legendary Bow",
        (WeaponType::Hammer, GearTier::Common) => "Stonefall",
        (WeaponType::Hammer, GearTier::Rare) => "Faultline",
        (WeaponType::Hammer, GearTier::Epic) => "Earthshatter",
        (WeaponType::Hammer, GearTier::Legendary) => "Legendary Hammer",
        (WeaponType::Crossbow, GearTier::Common) => "Rusted Hunter's Crossbow",
        (WeaponType::Crossbow, GearTier::Rare) => "Gravewind Crossbow",
        (WeaponType::Crossbow, GearTier::Epic) => "Siegebreaker Crossbow",
        (WeaponType::Crossbow, GearTier::Legendary) => "Legendary Crossbow",
        (WeaponType::Spellbook, GearTier::Common) => "Apprentice's Grimoire",
        (WeaponType::Spellbook, GearTier::Rare) => "Ember Codex",
        (WeaponType::Spellbook, GearTier::Epic) => "Grimoire of Echoes",
        (WeaponType::Spellbook, GearTier::Legendary) => "Legendary Spellbook",
        (WeaponType::Daggers, GearTier::Common) => "Twin Fangs",
        (WeaponType::Daggers, GearTier::Rare) => "Bloodletters",
        (WeaponType::Daggers, GearTier::Epic) => "Phantom Blades",
        (WeaponType::Daggers, GearTier::Legendary) => "Legendary Daggers",
    }
}

fn armor_definition_id(slot: ArmorSlot, tier: GearTier) -> &'static str {
    match (slot, tier) {
        (ArmorSlot::Head, GearTier::Common) => "armor.head.common",
        (ArmorSlot::Head, GearTier::Rare) => "armor.head.rare",
        (ArmorSlot::Head, GearTier::Epic) => "armor.head.epic",
        (ArmorSlot::Head, GearTier::Legendary) => "armor.head.legendary",
        (ArmorSlot::Chest, GearTier::Common) => "armor.chest.common",
        (ArmorSlot::Chest, GearTier::Rare) => "armor.chest.rare",
        (ArmorSlot::Chest, GearTier::Epic) => "armor.chest.epic",
        (ArmorSlot::Chest, GearTier::Legendary) => "armor.chest.legendary",
        (ArmorSlot::Hands, GearTier::Common) => "armor.hands.common",
        (ArmorSlot::Hands, GearTier::Rare) => "armor.hands.rare",
        (ArmorSlot::Hands, GearTier::Epic) => "armor.hands.epic",
        (ArmorSlot::Hands, GearTier::Legendary) => "armor.hands.legendary",
        (ArmorSlot::Legs, GearTier::Common) => "armor.legs.common",
        (ArmorSlot::Legs, GearTier::Rare) => "armor.legs.rare",
        (ArmorSlot::Legs, GearTier::Epic) => "armor.legs.epic",
        (ArmorSlot::Legs, GearTier::Legendary) => "armor.legs.legendary",
        (ArmorSlot::Feet, GearTier::Common) => "armor.feet.common",
        (ArmorSlot::Feet, GearTier::Rare) => "armor.feet.rare",
        (ArmorSlot::Feet, GearTier::Epic) => "armor.feet.epic",
        (ArmorSlot::Feet, GearTier::Legendary) => "armor.feet.legendary",
    }
}

fn armor_display_name(slot: ArmorSlot, tier: GearTier) -> &'static str {
    match (slot, tier) {
        (ArmorSlot::Head, GearTier::Common) => "Iron Helm",
        (ArmorSlot::Head, GearTier::Rare) => "Gatewarden Helm",
        (ArmorSlot::Head, GearTier::Epic) => "Ruinforged Helm",
        (ArmorSlot::Head, GearTier::Legendary) => "Legendary Helm",
        (ArmorSlot::Chest, GearTier::Common) => "Iron Cuirass",
        (ArmorSlot::Chest, GearTier::Rare) => "Gatewarden Cuirass",
        (ArmorSlot::Chest, GearTier::Epic) => "Ruinforged Cuirass",
        (ArmorSlot::Chest, GearTier::Legendary) => "Legendary Cuirass",
        (ArmorSlot::Hands, GearTier::Common) => "Iron Gauntlets",
        (ArmorSlot::Hands, GearTier::Rare) => "Gatewarden Gauntlets",
        (ArmorSlot::Hands, GearTier::Epic) => "Ruinforged Gauntlets",
        (ArmorSlot::Hands, GearTier::Legendary) => "Legendary Gauntlets",
        (ArmorSlot::Legs, GearTier::Common) => "Iron Greaves",
        (ArmorSlot::Legs, GearTier::Rare) => "Gatewarden Greaves",
        (ArmorSlot::Legs, GearTier::Epic) => "Ruinforged Greaves",
        (ArmorSlot::Legs, GearTier::Legendary) => "Legendary Greaves",
        (ArmorSlot::Feet, GearTier::Common) => "Iron Boots",
        (ArmorSlot::Feet, GearTier::Rare) => "Gatewarden Boots",
        (ArmorSlot::Feet, GearTier::Epic) => "Ruinforged Boots",
        (ArmorSlot::Feet, GearTier::Legendary) => "Legendary Boots",
    }
}
