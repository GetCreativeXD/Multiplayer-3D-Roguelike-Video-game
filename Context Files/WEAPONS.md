# Ruinbound Weapons

*Weapon design source of truth — updated 2026-09-05*

## Weapon Philosophy

All classes can equip every weapon family. A weapon is more than a stat
stick: each family must have a distinct basic attack, weapon ability,
range, damage profile, speed, knockback, and stagger identity.

The intended combat formula is:

> Class kit + weapon basic attack + weapon ability + gear/passives

Ruinbound uses four weapon tiers:

1. **Common** — establishes the weapon family's signature ability.
2. **Rare** — adds a more advanced technique.
3. **Epic** — introduces a run-changing mechanic.
4. **Legendary** — a named weapon with a unique ability and ideally a
   unique passive.

Weapon abilities and exact numbers are subject to playtesting. Legendary
weapons should feel mechanically memorable rather than being simple
stat upgrades.

## Development Status

### Implemented weapon families

These families currently exist in the prototype:

- **Longsword** — basic attack implemented; Rare Crosscut implemented.
- **Greatsword** — basic attack with windup implemented; Rare
  Executioner's Sweep implemented.
- **Bow** — projectile attack implemented; Rare Splinter Shot with
  adaptive homing fragments implemented.

### Planned weapon families

These families are designed or planned but still need to be added to the
prototype:

- Hammer
- Spear
- Daggers
- Axe
- Gauntlets
- Scythe
- Sword & Shield
- Crossbow
- Wand
- Staff
- Spellbook
- Hand Cannon
- Pistols
- Arcane Orb

The current roster contains 17 weapon families total: 3 implemented and
14 planned.

## Weapon Roster

### Longsword — Implemented

**Identity:** Mobile, versatile melee and fast blade techniques.

| Tier | Weapon ability | Status |
|---|---|---|
| Common | **Lunging Slash** — a quick forward lunge followed by a slash. | Designed / prototype framework |
| Rare | **Crosscut** — forward movement followed by two crossing cuts. | Implemented |
| Epic | **Blade Rush** — damages enemies passed through and ends with a wide slash. | Designed; implementation pending |
| Legendary | Named Longswords with unique abilities and passives. | Future |

### Greatsword — Implemented

**Identity:** Huge blade arcs, cleaves, execution, and heavy slicing
attacks. Major ground shockwaves belong primarily to Hammer, not
Greatsword.

| Tier | Weapon ability | Status |
|---|---|---|
| Common | **Charged Cleave** — a windup followed by a huge, wide cleave with strong stagger. | Designed / prototype framework |
| Rare | **Executioner's Sweep** — an extremely wide sweep that deals bonus damage to low-health enemies. | Implemented |
| Epic | **Phantom Cleave** — sends a large blade-shaped slash forward until it hits dungeon geometry. | Designed; implementation pending |
| Legendary | Named Greatswords with unique abilities and passives. | Future |

### Bow — Implemented

**Identity:** Precision, piercing, ricochets, and projectile manipulation.

| Tier | Weapon ability | Status |
|---|---|---|
| Common | **Piercing Shot** — a high-damage arrow that passes through enemies and stops on world geometry. | Designed / prototype framework |
| Rare | **Splinter Shot** — the initial projectile creates three target-locked homing fragments. | Implemented |
| Epic | **Ricochet Arrow** — intentionally ricochets between enemies and dungeon geometry. | Designed; implementation pending |
| Legendary | Named Bows with unique abilities and passives. | Future |

## Hammer — Planned

**Identity:** Ground slams, shockwaves, launching, knockback, and the
highest ordinary weapon stagger in the roster. Hammer should be slower
and more committed than Longsword, while Greatsword remains the primary
weapon for huge blade cleaves.

### Common Hammer — Stonefall

**Ability: Ground Slam**

The player raises the hammer and slams the ground, creating a short-range
circular impact.

- High stagger
- Moderate damage
- Strong knockback against normal enemies
- Briefly interrupts nearby enemy attacks
- Elites resist the knockback but still receive heavy stagger
- Bosses receive stagger pressure without being permanently stun-locked

### Rare Hammer — Faultline

**Ability: Seismic Strike**

The player smashes the hammer into the ground, sending a powerful fissure
forward in a straight line.

- Longer range than Ground Slam
- Maximum stagger near the impact point
- Normal enemies can be knocked into the air
- The fissure can hit multiple enemies
- Stops when it reaches solid dungeon geometry

### Epic Hammer — Earthshatter

**Ability: Earthshatter**

A massive overhead slam creates a large impact zone followed by several
cracking shockwaves.

- Large area of effect
- Multiple waves deal stagger and knockback
- Normal enemies can be launched
- Elites are heavily staggered but cannot be permanently stun-locked
- Leaves unstable ground that briefly slows enemies
- The unstable ground collapses in a second damaging burst

## Legendary Hammers

### Worldbreaker

**Theme:** Catastrophic impact and extreme stagger.

**Ability: Rupture**

The player leaps or rises into the air and crashes down with an enormous
hammer strike.

- Creates a massive circular shockwave
- Launches normal enemies a great distance
- Applies enormous stagger to elites
- Applies significant stagger pressure to bosses without trivially interrupting them
- Cracks the ground and damages enemies that remain inside the rupture zone

**Passive: Aftershock**

Enemies heavily staggered or launched by Worldbreaker trigger a smaller
delayed shockwave beneath them.

**Playstyle:** The ultimate crowd-control hammer. It dominates enemy
positioning and creates openings for the whole party.

### Graviton Maul

**Theme:** Gravity control and enemy grouping.

**Ability: Collapse**

The player drives the hammer into the ground, creating a gravity well that
pulls nearby enemies together before violently detonating.

- Pulls enemies toward the center
- Deals moderate damage during the pull
- The final detonation causes heavy stagger
- Normal enemies are briefly lifted
- Elites can be pulled but resist being lifted
- Bosses cannot be moved but still take detonation damage and stagger

**Passive: Crushing Weight**

Enemies affected by Collapse become **Weighted** briefly. Weighted enemies
deal slightly less damage and receive increased stagger from the player's
hammer attacks.

**Playstyle:** A setup-focused hammer that groups enemies for team attacks
and makes difficult encounters easier to control.

### Stormanvil

**Theme:** Lightning-powered impacts and aggressive area damage.

**Ability: Thunderous Verdict**

The player charges the hammer with lightning and slams a targeted location.
Lightning strikes the impact point and arcs between nearby enemies.

- Heavy impact damage at the primary target
- Lightning chains to nearby enemies
- Each lightning hit adds stagger
- The final target struck by the chain becomes Marked
- Striking the Marked target with a basic attack calls down another small lightning strike

**Passive: Conductive Blows**

Every few basic hammer attacks become electrically charged. Charged
attacks release a short-range lightning burst on impact.

**Playstyle:** The damage-focused legendary hammer. It improves area
damage while preserving slow, weighty melee attacks.

## Other Planned Melee Weapons

### Spear

**Identity:** Long melee reach, spacing, thrusts, and keeping enemies at
the preferred distance.

### Daggers

**Identity:** Extremely fast attacks, short range, mobility, evasive
positioning, combo attacks, and bonus damage from attacking vulnerable
enemies. Daggers should have high sustained damage but require the player
to stay close and avoid taking hits.

### Common Daggers — Twin Fangs

**Ability: Rapid Flurry**

The player performs a quick series of dagger strikes in front of them.

- Fast attack speed
- Moderate damage per hit
- The final strike slightly knocks enemies back
- Can be canceled into a dodge near the end of the attack
- Best for maintaining pressure on normal enemies

### Rare Daggers — Bloodletters

**Ability: Serrated Rush**

The player quickly dashes through enemies, striking each target along the
path.

- Applies **Bleed** to enemies hit
- Deals bonus damage when striking an enemy from behind
- Can pass through normal enemies
- Elites cannot be passed through, but still receive damage and Bleed
- Encourages aggressive positioning

### Epic Daggers — Phantom Blades

**Ability: Phantom Step**

The player vanishes briefly and dashes to a targeted location or enemy.
Their next dagger attack becomes an empowered backstab.

- Grants brief protection during the step
- The empowered attack deals heavy bonus damage
- Hitting an enemy from behind creates a second phantom strike
- Rewards dodging through attacks and repositioning behind enemies
- Bosses cannot be instantly executed by the backstab

## Legendary Daggers

### Kingslayer

**Theme:** Precision assassinations and elite hunting.

**Ability: Deathmark**

The player marks an enemy for a short duration. Attacking the marked enemy
from behind deals greatly increased damage and builds Execution stacks.

At maximum Execution stacks, the next backstab deals a powerful burst of
damage.

- Normal enemies below a health threshold can be executed
- Elites take heavy bonus damage but cannot be instantly executed
- Bosses receive bonus damage without being subject to the execution effect

**Passive: Ruthless Opening**

The first attack against a full-health elite or boss deals bonus damage and
instantly applies one Execution stack.

**Playstyle:** A single-target dagger for players who focus on positioning
and killing dangerous enemies quickly.

### Widow's Kiss

**Theme:** Bleed, poison, and damage over time.

**Ability: Venomous Barrage**

The player throws a spread of poisoned daggers, applying Venom and Bleed to
enemies hit.

- Venom deals damage over time
- Bleeding enemies take increased damage from basic dagger attacks
- Reapplying Venom refreshes its duration
- Venom and Bleed together cause the target to become **Envenomed**, briefly
  slowing it

**Passive: Spreading Agony**

When an enemy dies while affected by Venom or Bleed, the effect spreads to
nearby enemies at reduced strength.

**Playstyle:** A damage-over-time dagger that becomes stronger in groups
and longer fights.

### Dancer in the Dark

**Theme:** Evasion, movement, and shadow duplicates.

**Ability: Shadowstep Dance**

The player performs a rapid sequence of dashes around nearby enemies,
leaving behind shadow copies that repeat the player's last dagger strike.

- Each dash can avoid enemy attacks
- Shadow copies deal reduced damage
- The final strike returns the player to their original position
- Striking enemies from behind reduces the ability's cooldown

**Passive: Perfect Evasion**

Successfully dodging through an enemy grants a Shadow Charge. At maximum
charges, the next basic attack creates a temporary shadow duplicate that
attacks alongside the player.

**Playstyle:** A high-skill mobility weapon that rewards perfectly timed
dodges and staying in constant motion.

### Axe

**Identity:** Heavy committed chopping strikes, armor-breaking pressure,
and strong single-target damage.

### Gauntlets

**Identity:** Close-range brawling, rapid attacks, combos, and physical
mobility.

### Scythe

**Identity:** Sweeping attacks, unusual reach, wide area control, and
reaping mechanics.

### Sword & Shield

**Identity:** Blocking, parrying, counters, defense, and protecting allies.
Parry belongs to Sword & Shield rather than Longsword.

Planned progression:

- Common — **Parry**
- Rare — **Shield Counter**
- Epic — **Bastion**
- Legendary — named defensive Sword & Shield weapons

## Other Planned Ranged and Magical Weapons

### Crossbow

**Identity:** Slow, powerful ranged attacks distinct from the removed
Repeating Crossbow. Crossbows reward careful positioning, charged shots,
armor penetration, and meaningful individual hits.

### Common Crossbow — Rusted Hunter's Crossbow

**Ability: Hunter's Bolt**

The player fires a powerful bolt that deals increased damage at longer
range.

- Moderate reload time
- Stronger than a normal bow shot
- Rewards staying at a safe distance
- Stops when it reaches dungeon geometry

### Rare Crossbow — Gravewind Crossbow

**Ability: Gravewind Bolt**

The player fires a spectral bolt that passes through the first enemy it
hits and strikes enemies behind it for reduced damage.

- Useful against lined-up enemies
- The piercing effect does not pass through walls
- Keeps the crossbow's deliberate, high-impact identity

### Epic Crossbow — Siegebreaker Crossbow

**Ability: Armorbreaker Shot**

The player charges and fires a heavy bolt that pierces armor and heavily
staggers enemies.

- Fully charged shots reduce the target's defenses briefly
- Strong against armored enemies and elites
- Elites resist excessive crowd control but still receive heavy stagger
- The shot has a longer commitment window than ordinary crossbow attacks

## Legendary Crossbows

### Saint's Nail

**Theme:** Holy precision and explosive marks.

**Ability: Consecrated Bolt**

The player fires a blessed bolt that marks the target. Marked enemies take
bonus damage from abilities. When a marked enemy is defeated, it releases a
small holy burst that damages nearby enemies.

**Passive: Final Judgment**

The first fully charged shot against an elite or boss deals bonus damage
and applies the mark immediately.

**Playstyle:** A precision crossbow for players who coordinate weapon shots
with ability damage and elite encounters.

### The Ruinpiercer

**Theme:** Hunting corruption and turning Ruin against itself.

**Ability: Corruption-Piercing Bolt**

The player fires a bolt that deals increased damage to corrupted enemies.
The bolt can pass through corrupted enemies and deals reduced damage to
each additional target.

**Passive: Cleansing Strike**

Defeating a corrupted enemy with The Ruinpiercer has a chance to slightly
reduce the player's Ruin. This effect has a brief cooldown so it cannot be
repeatedly farmed from a single encounter.

**Playstyle:** A specialized anti-corruption weapon that becomes more
valuable as Ruin changes the run.

### Bellringer

**Theme:** Resonance, repeated precision, and battlefield interruption.

**Ability: Resonant Bolt**

Repeatedly hitting the same enemy builds Resonance. At maximum Resonance,
the next bolt creates a shockwave that damages nearby enemies and briefly
interrupts normal enemy attacks.

**Passive: Reverberation**

The first basic attack against a fully Resonant target causes a smaller
secondary impact.

**Playstyle:** A focused single-target crossbow that rewards accuracy and
maintaining pressure on dangerous enemies.

### Wand

**Identity:** Fast magical ranged attacks and rapid spell projectiles.

### Staff

**Identity:** Larger, heavier spell-focused attacks and area effects.

### Spellbook

**Identity:** Magic-focused weapon with a distinct casting style and
area-of-effect potential. Spellbooks improve abilities and encourage
players to build around cooldowns, elemental effects, and spell patterns.

### Common Spellbook — Apprentice's Grimoire

**Passive: Basic Arcana**

Slightly increases ability damage and slightly reduces ability cooldowns.

This establishes the spellbook's ability-focused identity without adding a
complicated mechanic at Common rarity.

### Rare Spellbook — Ember Codex

**Passive: Scorching Spells**

Abilities apply **Scorch** to enemies they damage.

**Scorch effect:** The afflicted enemy takes **6 fire damage per second for
4 seconds**. Reapplying Scorch refreshes its duration instead of stacking
the effect.

The damage and duration are initial tuning targets and should be adjusted
through playtesting. Scorch damage should scale from the player's relevant
ability or spell-power systems if those systems support scaling later.

### Epic Spellbook — Grimoire of Echoes

**Passive: Echoing Casts**

Every few ability casts repeats a weaker version of the last ability at the
target location.

- The echo deals reduced damage
- The echo cannot endlessly create additional echoes
- The effect rewards rotating abilities instead of repeating one spell

## Legendary Spellbooks

### The Hollow Gospel

**Theme:** Power gained by embracing corruption.

**Passive: Ruin-Fed Magic**

Abilities become stronger as the player's Ruin increases. At high Ruin,
abilities deal significantly increased damage but have a chance to cause a
small backlash against the player.

**Playstyle:** A high-risk offensive spellbook for players willing to let
Ruin grow in exchange for greater magical power.

### Saint's Testament

**Theme:** Protective magic, Grace, and spell mastery.

**Passive: Graceful Casting**

Damaging enemies with abilities builds Grace. At maximum Grace, the player
gains a protective shield and the next ability costs no resource or enters
cooldown normally, depending on the game's final ability system.

**Playstyle:** A defensive spellbook that rewards consistent spell use and
helps fragile ability-focused builds survive longer fights.

### The Living Codex

**Theme:** A spellbook that learns from the player's casting patterns.

**Passive: Evolving Spells**

Repeatedly using the same ability changes its behavior for the rest of the
encounter or until the player changes rooms. Possible evolutions include
additional projectiles, increased area, or chaining to another enemy.

**Playstyle:** A flexible spellbook that changes the value of the player's
abilities during combat and rewards adapting to the situation.

### Hand Cannon

**Identity:** Powerful firearm-style ranged attacks with deliberate reload
or commitment windows.

### Pistols

**Identity:** Faster, more mobile firearm-style attacks.

### Arcane Orb

**Identity:** Magical projectiles, control effects, and battlefield
manipulation.

## Removed Weapon Types

These weapon types are not part of the current roster and should not be
reintroduced unless intentionally reconsidered:

- Repeating Crossbow
- Throwing Knives
- Chakram
- Blunderbuss

## Legendary Design Rules

Every Legendary weapon should normally have:

- A unique name
- A distinctive model, material, and VFX treatment
- A unique ability or major mechanical twist
- Ideally, a unique passive
- A reason for the player to consider changing their build around it

Legendary abilities do not have to follow the Common → Rare → Epic
progression exactly, but they must respect the weapon family's identity.
