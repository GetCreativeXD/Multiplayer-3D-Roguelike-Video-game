# Ruinbound --- Project Context & Decision Log

*Last updated: 2026-09-05*

## Purpose

This file is the persistent handoff and source-of-truth document for
**Ruinbound**, a separate game project from Ashfall. Use it when
starting a fresh ChatGPT conversation so development can continue
without reconstructing prior decisions.

When a major design or implementation decision changes, update this
document rather than relying only on chat history.

### Decision-status rule

-   **CONFIRMED** --- approved direction; do not silently change it.
-   **EXPERIMENTAL / TUNING** --- approved concept, but exact
    numbers/details still need playtesting.
-   **SUPERSEDED / REJECTED** --- do not reintroduce unless
    intentionally reconsidered.
-   **IMPLEMENTED** --- confirmed to exist in the current prototype, not
    merely planned.

------------------------------------------------------------------------

# 0. Session Summary --- 2026-09-05

Quick-reference summary of the most recent session's work: the Ruin
Corruption resource and its Tab-key status screen. Full detail is in
Sections 70-72 below; nothing here overrides those sections, it just
points to them. The earlier session on this same date (Hammer weapon
family) is summarized in Sections 67-69, unaffected by this one.

## What was built

-   A new `RuinState` resource (`src/ruin/mod.rs`) implementing the
    0-100 Ruin scale, a six-band `RuinThreshold` (Unmarked / Touched /
    Stained / Corrupted / Doomed / Ruinbreak), and passive time-based
    growth per the supplied `RUIN_CORRUPTION_DESIGN.md`'s scaling table
    (+1 every 90s under 10 min, 75s from 10-20 min, 60s after), plus a
    HUD warning banner ("The Ruin is spreading...") on each tick.
-   A new Tab-key status screen (`src/ui/ruin_screen.rs`) showing
    current Ruin, threshold name, and run time, built around an
    extensible stat-row list for future additions.
-   Full three-way mutual exclusion between the Map (`M`), Inventory
    (`I`), and new Ruin (`Tab`) screens -- opening any one closes the
    other two.
-   Reaching 100 Ruin resets it to 65 rather than ending the run
    (design doc's "cannot directly kill the player"); the full
    Ruinbreak event itself is an explicit, disclosed gap.

## Design decisions made during implementation (not previously specified)

-   The design doc's "pauses in menus, shops, and safe rooms" was
    initially implemented literally (pausing for Map/Inventory/Tab, the
    game's only current menus), then explicitly reversed at the user's
    request: **Ruin and the run clock never pause**, regardless of which
    screen is open. See Section 70.5/71.
-   Only the resource, its passive growth, and its display were built.
    Corrupted enemies, corrupted rooms, player-choice Ruin events,
    cleansing mechanics, and the full Ruinbreak event are all
    deliberately deferred, per this project's "confirm one mechanic
    before moving to the next" workflow -- see Section 70.6.

## Verification status

**Status: COMPILES AND RUNS AS OF THE SECTION 70.4 FIX; THE SECTION 70.5
PAUSE-BEHAVIOR CHANGE IS NOT YET RE-CONFIRMED BY A FRESH BUILD.**
The user ran `cargo fmt && cargo check` and reported one compile error
(`RuinStatKind` private-type violation, same class of bug as Section
67.5's `HammerImpactVisual`), fixed by making the enum `pub(crate)`. The
user then reached runtime and reported the Tab screen's numbers weren't
updating while it was open -- which turned out to be the (then-
intentional) menu-pause behavior, resolved via the Section 70.5 decision
to remove pausing entirely. That removal has not yet been checked
against a fresh `cargo check`/`cargo run`. No `cargo` toolchain was
reachable in the development sandbox for any of this session's work, the
same standing limitation as every prior session.

**Not yet explicitly confirmed:** a post-Section-70.5 clean build;
in-game timing of passive growth and the warning banner; full
mutual-exclusion behavior across all three screens. See Section 70.7's
checklist.

------------------------------------------------------------------------

# 1. Game Overview

**Title:** Ruinbound\
**Language:** Rust\
**Engine:** Bevy 0.19.x\
**Physics:** Avian3D 0.7\
**Genre:** Real-time 3D cooperative dungeon-crawler roguelike\
**Primary intended party size:** 1--4 players, with solo play supported

Ruinbound is intended to be fast-paced but **not** a pure hack-and-slash
game. Positioning, timing, weapon behavior, class abilities, dodging,
enemy attack patterns, stagger, and room geometry should matter.

The central build philosophy is:

> **Class kit + weapon basic attack + weapon ability + gear/passives =
> the player's current combat build.**

Classes do not restrict weapons or armor. Finding a different weapon can
substantially change how the same class plays.

------------------------------------------------------------------------

# 2. Core Design Pillars

## 2.1 Fast but deliberate combat

Combat should feel responsive and energetic, but players should not win
by mindlessly holding attack.

Important principles:

-   positioning matters
-   timing matters
-   dodge timing matters
-   target priority matters
-   enemy telegraphs should be readable
-   attacks should have visible and mechanical physical impact
-   weapons should feel mechanically different, not like stat swaps
-   class abilities should add another playstyle layer without replacing
    weapon combat
-   bosses, elites, and minibosses should demand stronger mechanical
    play than ordinary enemies

## 2.2 Fair danger rather than constant one-shots

Ruinbound should not be built around ordinary enemies instantly deleting
players.

The desired encounter hierarchy is:

-   **Normal enemies** test damage, positioning, target priority, weapon
    use, and build efficiency.
-   **Elites / minibosses** more strongly test movement, dodge timing,
    interruption, and class tools.
-   **Bosses** test mastery of the combat system and require repeated
    successful decisions.

Players should generally be able to survive mistakes and recover.
Literal one-shots should be extremely rare and reserved, if used at all,
for unusual high-risk circumstances.

### Experimental damage feel

These are tuning targets, not locked balance values:

-   ordinary attacks: roughly 5--10% of player health
-   dangerous normal attacks: roughly 15--20%
-   elite/miniboss attacks: often roughly 15--30%
-   boss attacks: often roughly 10--35%, depending on telegraph and
    avoidability

The goal is for bosses to be difficult because the player must
repeatedly dodge and react correctly, not because one small mistake
automatically ends the fight.

## 2.3 Exploration is part of the run

A level is not a straight sequence of combat rooms. Players should
explore, choose branches, encounter dead ends, find optional rewards,
decide whether to challenge minibosses, and gradually uncover the map.

## 2.4 Co-op recovery creates dramatic runs

A player can be lost during a floor without being removed from the
entire expedition. A surviving teammate can carry the party to the next
floor and bring lost teammates back.

## 2.5 Ruin creates voluntary risk/reward

Ruin Corruption makes the expedition more dangerous while opening
stronger rewards and special build interactions. Players can sometimes
intentionally accept more Ruin.

------------------------------------------------------------------------

# 3. Core Run Structure

**CONFIRMED**

A standard run contains **20 levels**.

Every fifth level ends with a boss:

-   **Level 5** --- random boss
-   **Level 10** --- random boss
-   **Level 15** --- random boss
-   **Level 20** --- fixed final boss

Levels 5, 10, and 15 should choose from appropriate boss pools so the
journey changes between runs. The final boss remains consistent so
players can gradually learn and master it.

## 3.1 Four-run-section progression

The following thematic structure is approved as a direction; exact biome
names/content can still evolve.

### Levels 1--5 --- The Outer Ruins

-   players establish their early builds
-   more straightforward enemy combinations
-   introductory dungeon themes
-   first random boss

### Levels 6--10 --- The Depths

-   more dangerous enemy combinations
-   Ruin becomes more important
-   more specialized room layouts
-   second random boss

### Levels 11--15 --- The Forsaken Depths

-   elites become more common
-   harder room/enemy combinations
-   builds should be becoming powerful and specialized
-   third random boss

### Levels 16--20 --- The Heart of the Ruin

-   highest normal-run danger
-   rare/corrupted enemy variants
-   high Ruin pressure
-   powerful loot opportunities
-   Level 20 ends with the fixed final boss

The exact names of these four sections are not required to be final
production names, but the escalating four-part structure is approved.

------------------------------------------------------------------------

# 4. Procedural Dungeon Generation

**CONFIRMED**

Ruinbound uses **procedurally assembled handcrafted rooms**, not fully
random geometry and not a simple straight hallway chain.

The generator should:

1.  construct a dungeon graph
2.  decide the functional structure of the floor
3.  choose handcrafted room prefabs/templates with compatible
    entrances/exits
4.  connect those rooms into an explorable level
5.  populate room content from appropriate encounter/reward pools

## 4.1 Room-template philosophy

Handcrafted rooms can define:

-   geometry
-   door/connection sockets
-   enemy spawn points
-   loot spawn points
-   decoration sockets
-   special-event positions
-   cover/line-of-sight features
-   verticality/elevation
-   optional destructible props
-   lighting/environment hooks

The goal is **procedural layouts without procedural ugliness**.

## 4.2 Dungeon graphs must branch

Do not generate floors as:

`Start -> Combat -> Combat -> Combat -> Exit`

Instead, floors can contain:

-   branching paths
-   junctions
-   loops where appropriate
-   dead ends
-   treasure branches
-   elite branches
-   optional miniboss rooms
-   shrines
-   event/NPC rooms
-   secret areas
-   alternate routes
-   optional challenge rooms
-   ordinary non-combat spaces
-   the floor exit

Example conceptual graph:

``` text
                         [Treasure]
                             |
                         [Combat]
                             |
[Dead End] -- [Combat] -- [Junction] -- [Elite]
                |            |             |
              [Start]     [Combat]      [Treasure]
                             |
                          [Junction]
                          /        \
                     [Event]      [Combat]
                       |             |
                   [Dead End]      [Exit]
```

## 4.3 Dead ends are intentional

Dead ends are not automatically generator mistakes.

A dead end may contain:

-   treasure
-   shrine
-   elite encounter
-   miniboss
-   NPC
-   lore
-   rare merchant
-   corrupted artifact
-   secret-room access
-   or occasionally nothing

Not every dead end should guarantee a reward. Uncertainty makes
exploration more meaningful.

## 4.4 No trap-focused dungeon design

**REJECTED / SUPERSEDED:** trap-heavy dungeon gameplay.

Traps are not part of the desired core dungeon direction. Tactical
variety should come primarily from enemies, room geometry, branches,
optional encounters, cover, elevation, and build decisions.

## 4.5 Room geometry should affect combat

Rooms can create tactical differences through:

-   pillars that break line of sight
-   open arenas
-   narrow passages
-   bridges
-   staircases
-   multiple elevations
-   platforms
-   destructible objects
-   environmental cover
-   multiple enemy approach directions

The game should not require traps to make rooms mechanically distinct.

## 4.6 Not every room needs combat

Exploration needs breathing room. Some rooms should exist for traversal,
atmosphere, rewards, NPCs, lore, choices, or orientation rather than
another mandatory fight.

------------------------------------------------------------------------

# 5. Boss Floors and Optional Minibosses

## 5.1 Boss-floor structure

Boss floors can still contain exploration before the boss.

Conceptual structure:

``` text
           OPTIONAL BRANCH
             /        \
        Treasure     Shrine
            \          /
START --- Dungeon Exploration --- BOSS DOOR
                                  |
                                BOSS
                                  |
                              NEXT LEVEL
```

The boss should be the climax of the floor rather than the only content
on every fifth level.

## 5.2 Optional miniboss rooms

Minibosses can be optional and visibly signposted by a special/sealed
doorway or room presentation.

The player should understand that opening the room means accepting a
serious fight.

Possible reward direction:

-   guaranteed high-rarity item
-   special currency
-   Ruin reduction
-   other substantial reward

The purpose is to create party decisions such as whether the current
health/build state is strong enough to justify the risk.

------------------------------------------------------------------------

# 6. Exploration Map

**CONFIRMED**

The map should reveal itself as the party explores rather than showing
the entire procedural floor immediately.

Initial state:

``` text
[YOU]
```

After exploring:

``` text
[✓]--[YOU]
```

At a discovered junction:

``` text
     [?]
      |
[✓]--[YOU]--[?]
```

`?` represents a doorway/connection the player knows exists but has not
explored.

As rooms are discovered, the map can identify meaningful known room
types such as treasure or elite rooms.

Example:

``` text
     [TREASURE]
         |
[✓]--[✓]--[ELITE]
         |
        [?]
```

### Map rules

-   do not automatically reveal the whole floor
-   show rooms/connections as they are discovered
-   preserve unexplored branches as unknown
-   help the party remember where it has already been
-   support the exploration loop without removing discovery

------------------------------------------------------------------------

# 7. Multiplayer Death / Revival

**CONFIRMED**

The desired death flow is **Downed -\> Lost -\> Returned**, with a full
party wipe ending the expedition.

## 7.1 Downed

When a player reaches zero health, they initially become downed rather
than instantly leaving the floor.

Teammates should have a limited opportunity to revive them by staying
near/interacting with them. The exact revive timer is a tuning value; an
early idea was approximately 20--30 seconds.

Reviving during combat should expose the rescuer to danger.

## 7.2 Lost

If the player is not revived in time, they become lost for the remainder
of that floor.

Conceptual presentation:

> **PLAYER LOST TO THE RUINS**

The lost player does **not** lose their character, gear, or current run
build.

## 7.3 Returned

If at least one teammate survives and reaches the next level, all lost
teammates return at the start of the new level.

They keep:

-   weapons
-   armor
-   items
-   upgrades
-   run resources/currency as appropriate
-   character/run progression

This should create dramatic co-op moments where one surviving player can
save the expedition for the rest of the party.

## 7.4 Party wipe

If every player is dead/lost before the party reaches the next floor,
the expedition ends and the normal roguelike run-loss rules apply.

Solo play naturally treats the player's death as a party wipe unless a
future solo-specific revival mechanic is intentionally added.

------------------------------------------------------------------------

# 8. Run Progression vs Permanent Progression

**CONFIRMED**

Ruinbound should distinguish between power gained for the current
expedition and progression that persists across runs.

## 8.1 Run progression

Generally lost when the expedition ends:

-   temporary blessings
-   run-specific upgrades
-   temporary stat improvements
-   special run keys
-   temporary/run currency
-   current run build synergies

Exact inventory-loss rules still need a focused design pass.

## 8.2 Permanent progression

Potential persistent progression includes:

-   unlocked weapons
-   unlocked classes/characters/origins
-   crafting recipes
-   cosmetics
-   new room types
-   new enemies/content
-   lore discoveries
-   hub/town upgrades
-   other horizontal unlocks

**Gravemind is specifically intended to be an unlockable class.**

## 8.3 Meta-progression philosophy

Permanent progression should lean toward **unlocking possibilities**
rather than simply stacking huge permanent damage/health bonuses.

Avoid turning long-term progression into a system where enough grinding
trivializes the roguelike.

------------------------------------------------------------------------

# 9. Ruin Corruption

**CONFIRMED (concept) / PARTIALLY IMPLEMENTED** -- see Section 70 for
implementation detail and Section 9.4 for the numeric refinement now in
use.

Ruin is a major run-level risk/reward system.

The deeper the expedition goes and the more risky choices the party
makes, the more corrupted/dangerous the dungeon can become.

Ruin should generally **persist between floors during the same
expedition** rather than resetting every level.

Example run trajectory:

``` text
Level 1     0% Ruin
Level 5    18% Ruin
Level 10   42% Ruin
Level 15   71% Ruin
Level 19   93% Ruin
```

Those values are examples, not required progression.

## 9.1 Player choices can change Ruin

Possible approved direction:

-   cursed legendary gear can add Ruin
-   dangerous shrines can add Ruin
-   harder dungeon branches can add Ruin in exchange for better rewards
-   special enemies/objectives may reduce Ruin
-   class mechanics can interact with Ruin, especially Gravemind

Example choices previously discussed:

-   take cursed legendary gear: `+10 Ruin`
-   accept a dangerous shrine: `+15 Ruin`
-   choose a harder branch: `+8 Ruin` plus improved loot
-   kill a special creature/objective: `-5 Ruin`

Exact numbers are tuning placeholders.

## 9.2 Ruin threshold effects

Approved conceptual progression:

### Around 25%

-   more elite enemies can appear

### Around 50%

-   enemies can gain mutations/corrupted variants

### Around 75%

-   rooms can gain dangerous modifiers
-   visual corruption becomes much more obvious

### 100%

Something special and dangerous should happen.

**The exact 100% Ruin effect is intentionally unresolved.** It should be
significant enough that experienced players may deliberately attempt a
"100% Ruin run."

## 9.3 Ruin should visually transform the dungeon

As Ruin rises, environments and enemies should increasingly show
corruption.

Possible progression:

-   low Ruin: mostly normal environment
-   medium Ruin: dark/purple crystalline growths or supernatural changes
-   high Ruin: altered fog/sky/lighting, floating or distorted geometry,
    visibly corrupted enemies
-   extreme Ruin: strong world-state transformation

## 9.4 Numeric refinement + implementation status (2026-09-05)

**Status:** IMPLEMENTED (resource + passive growth + Tab status screen
only) -- see Section 70 for full detail.

A separate, more detailed `RUIN_CORRUPTION_DESIGN.md` document supplied
this session refines the 25/50/75/100% sketch above into a concrete
0-100 point scale with six named threshold bands -- Unmarked (0-19),
Touched (20-39), Stained (40-59), Corrupted (60-79), Doomed (80-99), and
Ruinbreak (100) -- plus exact passive-growth timing, gain/loss amounts
for specific player choices, corrupted-enemy affixes, room-mutation
types, and choice events. That document is now this system's numeric
source of truth; this section's original percentages and thresholds
remain the confirmed conceptual direction but are superseded numerically.

Implemented in code: the `RuinState` resource (0-100 clamp, the six
thresholds above, passive time-based growth, a Ruinbreak-at-100 stub
that resets to 65), and a Tab-key status screen showing current Ruin and
run time. **Not yet implemented:** player-choice Ruin gain/loss,
corrupted enemy affixes, room-level corruption, the full Ruinbreak event,
and the choice events described in `RUIN_CORRUPTION_DESIGN.md` -- see
Section 70.6 for the complete list of deferred work.

------------------------------------------------------------------------

# 10. Visual Direction

**CONFIRMED**

Ruinbound uses a **stylized low-poly dark-fantasy** visual direction
inspired by the supplied fox reference image, but the playable
characters are humanoid.

Important visual traits:

-   faceted low-poly geometry
-   readable silhouettes
-   simplified forms
-   limited texture dependence
-   strong lighting
-   atmospheric fog
-   readable VFX
-   exaggerated weapons where appropriate
-   strong animation poses
-   physical hit reactions

## 10.1 Playable characters

**REJECTED:** making the playable cast anthropomorphic animals.

Playable characters should be stylized humanoids. Animals remain special
through class summons and transformations.

## 10.2 Environment language

Potential environment motifs:

-   huge stone blocks
-   jagged cliffs
-   angular trees/roots
-   chunky wooden doors
-   crystals
-   broken statues
-   barrels/crates and simple props
-   large chains
-   braziers
-   fog
-   dust particles
-   torchlight

## 10.3 Lighting and magic

Lighting should do much of the visual work.

Examples:

-   cool ambient dungeon lighting
-   warm torch pools
-   bright magical effects
-   glowing treasure
-   colored class/weapon VFX

Magic can use stylized geometric effects rather than photorealism:

-   fire: glowing polygonal particles + point light
-   poison: stylized clouds
-   frost: angular crystalline shards
-   lightning: sharp branching lines
-   shadow animals: spectral low-poly silhouettes
-   runes: bright geometric symbols
-   time effects: translucent copies/afterimages

This should keep combat readable even in multiplayer.

------------------------------------------------------------------------

# 11. Universal Player Combat Kit

Every class shares the same fundamental combat framework.

## 11.1 Universal dodge

**CONFIRMED**

Every character has a dodge roll.

-   no stamina/energy resource is consumed
-   use a short recovery/cooldown to prevent meaningless infinite spam
-   include a brief invulnerability window
-   dodge timing should matter especially against elites, minibosses,
    and bosses
-   current prototype also supports airborne dodging

Class abilities and some weapons can provide additional
repositioning/evasion tools beyond the universal dodge.

## 11.2 Core actions

Conceptually, a character has:

-   movement
-   universal dodge
-   weapon basic attack
-   weapon ability
-   three class abilities
-   interact
-   jump/air movement as supported by the prototype

The exact final input bindings can evolve.

------------------------------------------------------------------------

# 12. Physical Impact, Stagger, Knockback, and Interruption

**CONFIRMED**

Attacks should feel physical.

Enemies can:

-   stagger
-   be knocked backward
-   be launched
-   be interrupted
-   react visibly to hits

Damage, knockback, stagger, and interruption are separate concepts. An
enemy can take damage and knockback without necessarily being
interrupted.

## 12.1 Current stagger architecture

The prototype uses **stagger strength vs stagger resistance**.

Conceptual scale:

-   Longsword --- stagger strength \~1
-   Greatsword --- \~2
-   Hammer --- very high stagger
-   powerful class abilities --- potentially \~4+

Enemy resistance examples:

-   normal enemy --- resistance \~0
-   elite --- resistance \~2
-   boss --- resistance \~4

If stagger strength exceeds resistance, the attack can interrupt the
enemy and reset its attack state.

Normal enemies should be easier to interrupt. Elites and bosses resist
light interruption so they cannot be permanently stun-locked by ordinary
attacks.

------------------------------------------------------------------------

# 13. Weapon Philosophy

**CONFIRMED**

All classes can use **any weapon and any piece of gear**.

Weapons are not merely stat sticks. Every weapon family should have:

1.  a distinct basic attack/moveset
2.  a distinct weapon ability
3.  its own range/damage/speed/knockback/stagger profile
4.  a recognizable mechanical identity

Finding a new weapon can change how a class plays.

Weapon families should own different mechanics rather than sharing the
same small pool of generic abilities.

------------------------------------------------------------------------

# 14. Planned Weapon Roster

The current broader roster contains **17 weapon families/types**.

Sword/Longsword is one family. Sword & Shield is separate because its
defensive/parry identity is mechanically different.

## 14.1 Melee / close-range

-   **Longsword / Sword** --- mobile, versatile melee and fast blade
    techniques
-   **Greatsword** --- large blade arcs, cleaves, execution, heavy
    slicing attacks
-   **Hammer** --- ground slams, shockwaves, launching, extreme stagger
-   **Spear** --- long melee reach and spacing-focused attacks
-   **Daggers** --- short-range, fast attacks and high mobility
-   **Axe** --- heavy committed chopping strikes
-   **Gauntlets** --- close-range brawling and rapid physical attacks
-   **Scythe** --- sweeping attacks, unusual reach, area control
-   **Sword & Shield** --- blocking, parrying, counters, defense

## 14.2 Ranged / magical

-   **Bow** --- precision, piercing, ricochets, projectile manipulation
-   **Crossbow** --- slow/powerful ranged family distinct from repeating
    crossbows
-   **Wand** --- fast magical ranged attacks
-   **Staff** --- larger/heavier spell-focused magical attacks
-   **Spellbook** --- magic-focused weapon with its own casting
    style/AoE potential
-   **Hand Cannon** --- powerful firearm-style ranged weapon
-   **Pistols** --- faster, mobile firearm-style ranged family
-   **Arcane Orb** --- magical projectile/control weapon family

## 14.3 Explicitly removed weapon types

**REJECTED / SUPERSEDED**

-   Repeating Crossbow
-   Throwing Knives
-   Chakram
-   Blunderbuss

Do not re-add these unless intentionally reconsidered.

------------------------------------------------------------------------

# 15. Four Gear Tiers

**CONFIRMED**

Ruinbound uses four gear tiers:

1.  **Common**
2.  **Rare**
3.  **Epic**
4.  **Legendary**

Weapon tier changes the weapon's ability progression.

### Common

Establishes the weapon family's basic signature ability.

### Rare

More advanced version or alternate technique that deepens the weapon
identity.

### Epic

Run-changing ability with a more dramatic mechanic.

### Legendary

Hand-designed named weapon with a unique ability and ideally a unique
passive.

Legendary gear should create an "I found THAT weapon" feeling rather
than merely increasing numbers.

Armor/other gear can use the same four tiers but should generally
emphasize affixes/passives rather than adding many more active buttons.

------------------------------------------------------------------------

# 16. Current Weapon Ability Plan

## 16.1 Longsword

**Identity:** mobility, fast blade techniques, versatile melee.

### Common --- Lunging Slash

Quick forward lunge followed by a slash.

### Rare --- Crosscut

Forward movement followed by two crossing cuts.

The prototype currently applies combined damage; eventually this should
become two distinct timed hits.

### Epic --- Blade Rush

Longer aggressive movement technique that damages enemies passed through
and ends with a wide slash.

### Legendary

Unique per named Legendary Longsword.

## 16.2 Greatsword

**Identity:** huge blade arcs, cleaves, execution, heavy slicing
attacks.

### Common --- Charged Cleave

Brief windup followed by a huge, wide, high-damage cleave with strong
stagger.

### Rare --- Executioner's Sweep

Extremely wide horizontal sweep. Deals additional damage to enemies
below a health threshold.

### Epic --- Phantom Cleave

Sends a large horizontal blade-shaped slash forward until it collides
with dungeon geometry. It passes through/damages enemies along its path
and has substantial stagger.

**Important identity rule:** Greatsword does **not** own major ground
shockwaves. Those belong primarily to Hammer.

### Legendary

Unique per named Legendary Greatsword.

## 16.3 Bow

**Identity:** precision, piercing, ricochets, projectile manipulation.

### Common --- Piercing Shot

High-damage arrow that passes through enemies and stops on world
geometry.

### Rare --- Splinter Shot

Initial projectile hits a target and creates **3 homing fragments**.

Target-selection rules:

-   3 secondary enemies nearby: one fragment to each of the 3 closest
    secondaries
-   2 secondaries: one to each; remaining fragment returns to main
    target if alive
-   1 secondary: one to secondary; two return to main target if alive
-   no secondaries: all 3 return to main target if alive
-   if the initial hit kills the main target, unused fragment slots
    redirect to the closest surviving secondary target

Fragments are **target-locked**. If another enemy intercepts a fragment,
it ignores that enemy and continues toward its assigned target.

This behavior has been successfully demonstrated in the prototype.

### Epic --- Ricochet Arrow

Precision projectile that can intentionally ricochet between enemies and
dungeon geometry.

The ability should reward aim and room geometry rather than becoming a
generic "rain of arrows" skill.

### Legendary

Unique per named Legendary Bow.

## 16.4 Hammer --- IMPLEMENTED (Common/Rare/Epic); Legendary planned

**Identity:** ground slams, shockwaves, launching, huge stagger. See
Section 67 for full implementation detail.

### Common --- Stonefall (Ground Slam)

Short-range circular ground impact. High stagger, moderate damage,
strong knockback against normal enemies, briefly interrupts nearby
enemy attacks. Elites resist the knockback but still take heavy
stagger.

### Rare --- Faultline (Seismic Strike)

A forward-traveling ground fissure with longer range than Ground Slam,
implemented as a wide piercing projectile that stops on solid dungeon
geometry and can hit multiple enemies.

### Epic --- Earthshatter

A large impact followed by two smaller aftershock pulses, then a
lingering unstable-ground hazard that collapses in a second damaging
burst. The "briefly slows enemies" half of the unstable-ground effect
is deferred pending a movement-slow field on `combat::EnemyDebuffs`
(Section 54.1) -- see Section 67.4.

### Legendary --- unique named Hammer, not yet itemized

Three named concepts exist in `WEAPONS.md` -- Worldbreaker (massive
shockwave, launch, Aftershock passive), Graviton Maul (gravity-well pull
+ detonation, Crushing Weight passive), and Stormanvil (lightning chain
+ Marked target, Conductive Blows passive) -- consistent with the
existing Worldbreaker concept already on file in Section 17.4.

## 16.5 Sword & Shield --- planned

**Identity:** blocking, parrying, counters, defense.

Parry belongs here rather than Longsword.

Potential progression:

-   Common --- Parry
-   Rare --- Shield Counter
-   Epic --- Bastion
-   Legendary --- unique named sword/shield ability

------------------------------------------------------------------------

# 17. Legendary Item Concept Archive

Legendary gear should be hand-designed, named, visually distinctive, and
mechanically memorable.

These are approved concepts/directions; exact balance values can change.

## 17.1 Riftedge --- Legendary Longsword

**Theme:** dimensional/fractured blade.

Possible established direction:

-   **Passive --- Fractured Edge:** every third basic attack sends a
    small dimensional slash.
-   **Ability --- Sever:** cut/open a dimensional rift that produces an
    additional spectral/dimensional slash at much greater range; an
    earlier variation explored having the rift repeat a subsequent
    attack.

Goal: preserve fast Longsword play while extending attacks through
dimensional cuts.

## 17.2 Stormwake --- Legendary Longsword

**Theme:** lightning mobility.

-   **Ability --- Thunderstep:** instantly dash to the targeted enemy
    and strike with lightning; nearby enemies are hit by smaller
    lightning arcs.

Goal: exaggerate the Longsword mobility identity.

## 17.3 Gravetide --- Legendary Greatsword

**Theme:** spectral/death-infused Greatsword.

-   **Ability --- Soul Rend:** perform a massive sweep; enemies defeated
    by the attack release spectral blades that seek nearby enemies.

Goal: preserve the huge physical sweep identity while adding a
supernatural kill-chain effect.

## 17.4 Worldbreaker --- Legendary Hammer

**Theme:** catastrophic impact / extreme stagger.

-   **Ability --- Rupture:** leap/rise into a heavy slam that creates a
    massive circular shockwave.
-   normal enemies can be launched
-   elites take enormous stagger
-   bosses take significant stagger pressure without becoming trivially
    interruptible

**Worldbreaker is a Hammer, not a Greatsword.**

## 17.5 Nightstring --- Legendary Bow

**Theme:** spectral predator / target hunting.

-   **Passive --- Predator's Mark:** repeated basic attacks against the
    same enemy gradually mark that target.
-   **Ability --- Shadow Hunt:** use/consume the mark to temporarily
    transform arrows into spectral projectiles that aggressively curve
    toward the marked enemy and can bend around some obstacles.

Goal: exceptional boss focus and projectile manipulation without generic
arrow-rain gameplay.

## 17.6 Aegis of the Fallen --- Legendary Sword & Shield

**Theme:** protective team-control Legendary.

-   **Ability --- Last Stand:** slam/plant the shield to create a
    protective area for the player and teammates; enemies entering the
    area are staggered.

Goal: emphasize defense, counterplay, and team protection.

## 17.7 Mantle of the Hollow King --- Legendary Armor

-   after avoiding damage for a period, gain a regenerating spectral
    shield
-   when the shield breaks, it releases a shockwave

Goal: demonstrate that Legendary armor can have memorable passives
without requiring another active button.

## 17.8 Legendary design rule

A Legendary should normally have:

-   a unique name
-   distinctive model/material/VFX treatment
-   a unique ability or major mechanical twist
-   ideally a unique passive
-   a reason for the player to consider changing their build around it

Legendary abilities do not have to follow the Common -\> Rare -\> Epic
family progression exactly, but they should still respect the broad
identity of the weapon.

------------------------------------------------------------------------

# 18. Six-Class Roster

**CONFIRMED DIRECTION**

Ruinbound is planned around six distinct classes/characters.

All classes:

-   can equip all weapon families
-   can equip all gear
-   have the universal movement/dodge framework
-   have **three class abilities**
-   combine their class kit with the equipped weapon's basic attack and
    weapon ability

Current roster:

  Class              Identity                  Combat Specialty
  ------------------ ------------------------- ---------------------------
  Warden of Shades   Shadow-animal summoner    Support / Control
  Runesage           Fast rune wizard          Rune resource / Magic
  Vanguard           Hyper-mobile fighter      Basic attacks / Mobility
  Wildshaper         Animal shapeshifter       Melee / Transformation
  Gravemind          Ruin-powered unlockable   Ramp-up / Burst / Sustain
  Chronowarden       Time manipulator          Control / Utility

------------------------------------------------------------------------

# 19. Warden of Shades

**Identity:** spectral/shadow animals used for support, control, and
empowered attacks.

The Warden should **not** be a passive healer. It should contribute
damage/control while protecting teammates.

## Ability 1 --- Guardian Otter

Summon a spectral shadow otter that travels to/circles the targeted
teammate.

Approved effect direction:

-   grants a shield
-   grants a damage buff
-   grants a movement-speed buff
-   can target the Warden when playing solo

Visual direction: the otter can swim/bound through the air or shadows
around the protected player.

The otter specifically replaces the earlier generic/wolf guardian
concept.

## Ability 2 --- Shadow Stag

Summon a large spectral stag that charges through enemies in a line.

Effects:

-   damage
-   stun
-   strong physical knockback/displacement
-   useful for lining up groups and interrupting dangerous enemies when
    stagger rules allow

## Ability 3 --- Predator's Spirit

Temporarily empower basic attacks with spectral animal strikes.

Visual/effect possibilities include:

-   spectral claws
-   jaws
-   birds
-   charging animals
-   sweeping ghost-animal effects

The effect should interact with the equipped weapon rather than forcing
one fixed attack style.

Examples:

-   fast weapons can trigger frequent spectral strikes
-   heavy weapons create larger/slower empowered impacts
-   Bow attacks can create spectral effects at arrow impacts

An earlier direction considered a larger spectral strike every few hits;
exact proc rules remain a tuning decision.

------------------------------------------------------------------------

# 20. Runesage

**Identity:** fast-paced rune wizard whose abilities are woven directly
between weapon attacks.

**REJECTED:** delayed rune circles / slow delayed-cast gameplay as the
core class identity.

## 20.1 Rune resource

-   abilities spend runes
-   every **two basic attacks** restores one rune
-   the two basic attacks do **not** need to be consecutive
-   using class abilities between those attacks does not reset progress
-   the class should encourage a rhythm of attacking, casting,
    attacking, and casting
-   an early approved tuning direction is a maximum around **6 runes**,
    but the exact cap is still experimental

Example:

``` text
Basic attack       1/2
Ability
Basic attack       2/2 -> +1 Rune
Ability
Basic attack       1/2
Basic attack       2/2 -> +1 Rune
```

## Ability 1 --- Rune Blast

**Early cost direction:** 1 Rune.

Immediately fire an explosive rune projectile.

Role:

-   fast
-   inexpensive
-   moderate damage
-   useful for frequent weaving

## Ability 2 --- Runic Surge

**Early cost direction:** 2 Runes.

Immediately release a wide magical wave in front of the Runesage.

Role:

-   close/mid-range answer
-   strong stagger/knockback
-   immediate rather than delayed

## Ability 3 --- Grand Rune

**Early cost direction:** 4 Runes.

Immediately unleash a large runic explosion around the targeted enemy.

Role:

-   high rune investment
-   heavy damage
-   strong payoff for saving resources
-   can apply a temporary **Runebroken**-style debuff that increases
    damage/stagger taken; exact debuff behavior is still tuning

The central decision is whether to spend runes frequently on smaller
abilities or save for the large payoff.

------------------------------------------------------------------------

# 21. Vanguard

**Identity:** highly mobile, basic-attack-focused character with a high
skill ceiling.

## Ability 1 --- Lunge

Short targeted dash/lunge toward an enemy.

Direction:

-   low cooldown
-   aggressive repositioning
-   helps maintain pressure
-   can function as an additional movement tool during dangerous
    encounters

## Ability 2 --- Momentum

Basic attacks build Momentum.

Momentum can increase:

-   attack speed
-   movement speed
-   potentially weapon-ability recharge speed

**CONFIRMED:** getting hit does **not** automatically break/reset
Momentum.

Preferred direction: Momentum naturally decays when the Vanguard stops
attacking, encouraging aggression without making one mistake erase the
mechanic.

Exact stacking/decay values remain tuning decisions.

## Ability 3 --- Afterimage

Activate to create a stationary spectral copy of the Vanguard at the
current position for several seconds.

The clone:

-   remains stationary
-   attacks the enemy the player is currently focusing
-   converts its copy of the player's basic attack into a **ranged
    spectral version**
-   lasts only a limited time

Reactivate the ability while the clone exists to **teleport to the
clone's position**.

This combines:

-   damage
-   boss repositioning
-   escape
-   skillful timing
-   weapon interaction

Examples:

-   Hammer clone can send spectral hammer-shaped shockwaves/projectiles
    rather than physically walking into melee.
-   Bow clone can fire spectral arrows.
-   Other weapons should receive readable ranged spectral equivalents.

------------------------------------------------------------------------

# 22. Wildshaper

**Identity:** unique melee shapeshifter that rapidly transforms into
different animals for different combat functions.

Transformations should be **fast attack/movement transformations**, not
long slow form-change sequences. The character should transform, execute
the move, and return to humanoid form quickly.

Visual direction: the character can burst into angular magical fragments
and rapidly reconstruct into the animal form.

## Ability 1 --- Rhino Charge

Transform into a rhino and charge forward.

Effects:

-   strong forward mobility
-   bulldoze/displace enemies
-   smaller enemies can be launched
-   collision with a wall can create a crash/stun effect
-   immediately return to humanoid form afterward

Role: displacement + mobility.

## Ability 2 --- Bear Slam

Transform into a large bear and slam the ground.

Effects:

-   large close-range damage
-   strong stagger
-   area impact
-   brief resistance to interruption during the transformation/attack is
    an approved direction

Role: power + stagger.

## Ability 3 --- Panther Pounce

Transform into a panther/large cat and rapidly leap toward/through an
enemy.

Effects:

-   fast damage
-   repositioning
-   can end behind or beyond the target
-   immediately return to humanoid form

Role: mobility + precision aggression.

------------------------------------------------------------------------

# 23. Gravemind

**Identity:** aggressive unlockable class that interacts directly with
Ruin and can ramp into dangerous power.

The class should interact with health, but it should **not** be entirely
focused on constant low-health micromanagement.

**Permanent-progression rule:** Gravemind is intended to be unlockable
rather than necessarily available at the start.

## Ability 1 --- Ruin Bolt

Powerful corruption projectile/blast.

Earlier health-cost versions were deemphasized so that ordinary ability
use does not make the entire class a constant health-management
exercise.

Exact damage/effect still needs tuning.

## Ability 2 --- Consume

A **strong heal**, not a tiny sustain effect.

Direction:

-   rip Ruin/corruption energy from nearby enemies
-   restore a substantial amount of health
-   an early tuning concept was roughly 30% max HP plus additional
    healing based on nearby affected enemies

Exact percentage is not locked.

The ability gives Gravemind a reason to enter dangerous positions and
provides recovery for Embrace the Ruin.

## Ability 3 --- Embrace the Ruin

Activate a dangerous empowered state.

While active:

-   health continuously drains
-   an internal **Embrace meter** rises while the player is attacking
-   the meter falls while the player is not attacking
-   the ability can remain active while the meter falls
-   filling the meter increases Ruin
-   filling the meter grants a significant damage increase / stronger
    corrupted state

Approved high-risk direction: a skilled player may be able to keep the
state active and fill the meter repeatedly, deliberately raising Ruin
further in exchange for escalating personal power.

The exact Ruin gain, damage increase, meter speed, and whether multiple
fills stack indefinitely require balance testing.

------------------------------------------------------------------------

# 24. Chronowarden

**Identity:** time manipulation, control, utility, and fast combat
support.

**REJECTED:** Rewind. Do not reintroduce a "mark health/location and
restore to it later" Rewind ability unless intentionally reconsidered.

The replacement kit direction below was approved.

## Ability 1 --- Phase Step

Instantly teleport a short distance in the movement direction.

Direction:

-   faster/more magical than the universal dodge
-   can pass through enemies
-   enemies passed through are briefly slowed

This gives Chronowarden a unique boss-dodging/repositioning tool.

## Ability 2 --- Time Lock

Create an area/sphere where:

-   enemies are dramatically slowed
-   enemy projectiles are dramatically slowed
-   affected enemy animations/actions visibly slow
-   players remain at normal speed

This is control rather than a generic hard stun.

## Ability 3 --- Echo

Create a temporal copy that repeats the player's basic attacks at a
slight delay for several seconds.

Concept:

``` text
YOU:       ATTACK
             |
           ~0.3s
             |
ECHO:      ATTACK
```

The Echo should interact with the equipped weapon, allowing the same
class to feel different with Bow, Greatsword, Hammer, etc.

Chronowarden's clone is about **time-delayed duplicated
attacks/control**, while Vanguard's Afterimage is about **stationary
ranged pressure + teleport repositioning**.

------------------------------------------------------------------------

# 25. Current Player Controls / Movement Prototype

**IMPLEMENTED / CURRENT PROTOTYPE**

-   **WASD** --- movement
-   **Mouse** --- camera / aiming
-   **Left Click** --- weapon basic attack
-   **Right Click** --- weapon ability
-   **Shift** --- dodge
-   **Space** --- jump / air jump
-   **1 / 2 / 3** --- switch between the three equipped weapon slots
-   **F** --- interact with nearby loot and chests
-   **I** --- open/close the inventory and equipment screen
-   **Esc** --- release/capture cursor

Movement is camera-relative.

The game uses a third-person camera with a centered crosshair.

The current prototype supports multiple jumps through `VerticalMovement`
/ `MAX_JUMPS`.

Dodging can be performed while airborne.

The player uses collision-aware kinematic movement through Avian rather
than manually clamping to a fake ground height.

------------------------------------------------------------------------

# 26. Bow / Projectile System

**IMPLEMENTED / CURRENT PROTOTYPE**

Bow aiming uses the crosshair:

1.  raycast from camera center
2.  determine crosshair target point
3.  spawn projectile from player
4.  aim projectile from spawn point toward target point

Projectile movement uses Avian spatial queries and swept shape collision
rather than only checking the projectile's current position.

A small sphere shape cast helps prevent high-speed tunneling and
provides reasonable projectile thickness.

Arrows stop on world geometry.

Current projectile kinds include:

-   `Normal`
-   `Piercing`
-   `Splinter`
-   `SplinterFragment`

Splinter fragments support optional target entities and homing behavior.

------------------------------------------------------------------------

# 27. Current Enemy Prototype

**IMPLEMENTED / CURRENT PROTOTYPE**

A basic enemy supports:

-   Chase
-   Windup
-   Attack
-   Recovery
-   Health
-   Knockback
-   Stagger
-   Attack interruption
-   Death/despawn
-   Floating health bar

Current state enum:

``` rust
EnemyState::Chase
EnemyState::Windup
EnemyState::Attack
EnemyState::Recovery
```

A reusable `spawn_basic_enemy(...)` helper exists so the test arena can
spawn multiple enemies without duplicating the full bundle.

Multiple enemies have been successfully used for Splinter Shot testing.

------------------------------------------------------------------------

# 28. Current UI

**IMPLEMENTED / CURRENT PROTOTYPE**

-   centered crosshair
-   player health bar
-   floating enemy health bars
-   equipped weapon indicator

The weapon indicator displays weapon/tier information.

Enemy health bars follow enemies in screen space and are cleaned up
after the associated enemy despawns.

Future UI will eventually need to communicate:

-   three class abilities/cooldowns
-   weapon ability
-   class-specific resources such as Runes
-   Ruin level
-   downed/lost teammate state
-   map/exploration
-   buffs/debuffs
-   gear/loot

------------------------------------------------------------------------

# 29. Current Code Organization

**IMPLEMENTED / CURRENT PROTOTYPE**

``` text
src/
├── main.rs
├── arena/
│   └── mod.rs
├── camera/
│   └── mod.rs
├── combat/
│   └── mod.rs
├── enemies/
│   └── mod.rs
├── player/
│   └── mod.rs
├── ui/
│   └── mod.rs
└── weapons/
    ├── mod.rs
    ├── abilities.rs
    ├── bow.rs
    ├── components.rs
    ├── melee.rs
    ├── projectiles.rs
    └── switching.rs
```

### `main.rs`

Primarily plugin/app wiring.

### `arena/mod.rs`

Prototype arena/startup scene, player spawn, test enemy placement,
camera/HUD startup calls.

### `camera/mod.rs`

Third-person camera, mouse look, follow behavior, cursor toggle.

### `combat/mod.rs`

Shared combat data such as `Health`.

### `player/mod.rs`

Player marker, movement, jumping, airborne jumping, dodge, airborne
dodge, kinematic movement.

### `enemies/mod.rs`

Enemy components/state machine, stagger, enemy combat/death, reusable
basic-enemy spawn helper.

### `ui/mod.rs`

Player health bar, enemy health bars, crosshair, weapon indicator.

### `weapons/components.rs`

`GearTier`, `WeaponType`, `Weapon`, `PlayerCombat`, projectile data,
attack visual data.

### `weapons/melee.rs`

Basic melee attacks, Greatsword basic attack timing, melee visuals,
player attack state updates.

### `weapons/bow.rs`

Bow projectile spawning including normal Bow, Piercing Shot, and
Splinter Shot initial projectile.

### `weapons/projectiles.rs`

Projectile movement, swept collision, piercing behavior, Splinter target
selection, fragment spawning/homing.

### `weapons/abilities.rs`

Tier-aware right-click ability dispatch plus Longsword/Greatsword
ability implementations.

### `weapons/switching.rs`

Prototype weapon switching and attack/ability state reset.

------------------------------------------------------------------------

# 30. Immediate Development Status

**IMPLEMENTED**

The current prototype has working:

-   Bevy application setup
-   Avian physics
-   third-person camera
-   cursor capture/release
-   crosshair aiming
-   camera-relative movement
-   jumping / airborne jumping
-   dodging / airborne dodging
-   kinematic collision
-   player health
-   enemy AI state machine
-   enemy damage to player
-   enemy death
-   enemy health bars
-   stagger and interruption
-   elite/boss-ready stagger resistance architecture
-   reusable weapon stats
-   Longsword basic attack
-   Greatsword basic attack with windup
-   Bow projectile attack
-   crosshair raycast targeting
-   swept projectile shape collision
-   attack visual arcs
-   weapon switching
-   four-tier weapon architecture
-   Common weapon abilities
-   Rare Longsword Crosscut
-   Rare Greatsword Executioner's Sweep
-   Rare Bow Splinter Shot with adaptive homing fragments
-   Common Hammer Ground Slam (Stonefall)
-   Rare Hammer Seismic Strike (Faultline)
-   Epic Hammer Earthshatter

Implemented prototype weapon families:

-   Longsword
-   Greatsword
-   Bow
-   Hammer

Equipped weapon switching:

-   `1` --- equipped weapon slot 1
-   `2` --- equipped weapon slot 2
-   `3` --- equipped weapon slot 3

Switching cancels unfinished attack/ability state.

The old debug weapon-generation hotkeys have been removed. Number keys now
select only weapons the player has actually acquired and placed in an equipped
weapon slot.

------------------------------------------------------------------------

# 31. Recommended Development Sequence

The broad development philosophy is to prove gameplay before building
the entire roguelike around it.

The original milestone strategy was:

**Combat prototype -\> Vanguard abilities -\> several enemies -\>
loot/equipment -\> second class -\> room system -\> procedural dungeon
graph -\> exploration map -\> multiplayer -\> death/revival -\> Ruin -\>
full class roster -\> bosses -\> 20-floor runs -\> permanent progression
-\> polish/content**

Multiplayer should arrive early enough that the project does not build
too many systems around single-player-only assumptions.

## Current near-term milestone

The current prototype has progressed beyond the initial basic-combat
milestone. The natural immediate next step recorded before this update
is **Epic weapon abilities**:

### Epic Longsword --- Blade Rush

Implement a collision-aware aggressive movement attack that can damage
enemies passed through and ends with a slash.

### Epic Greatsword --- Phantom Cleave

Implement a large horizontal blade-shaped projectile/slash that travels
forward until it hits world geometry and damages enemies along the path.

### Epic Bow --- Ricochet Arrow

Implement intentional ricochets between enemies and dungeon geometry,
rewarding aim and room geometry.

After Epic abilities, likely priorities:

-   design/implement first named Legendary weapons
-   add Sword & Shield family (Hammer added 2026-09-05 -- see Section 67)
-   improve attack animations/visuals
-   formalize weapon loot/equipment instead of debug hotkeys
-   begin room/dungeon-generation architecture
-   begin class framework
-   keep multiplayer architecture in mind before systems become too
    coupled to solo assumptions

------------------------------------------------------------------------

# 32. Open Design Questions / Backlog

These are intentionally **not fully decided yet** and should not be
silently invented in future chats.

-   exact inventory/equipment slots
-   armor-slot structure
-   affix system
-   exact class cooldowns
-   exact Runesage rune cap/cost balance
-   exact Gravemind health/Ruin numbers
-   exact enemy roster
-   exact elite roster
-   miniboss roster
-   boss pools for Levels 5/10/15
-   final boss identity/mechanics
-   multiplayer/networking library/architecture
-   permanent-progression hub/town
-   currencies
-   difficulty modes/scaling
-   sound/music direction
-   lore/story
-   exact 100% Ruin event
-   exact procedural-generation graph rules
-   secret-room rules
-   merchant/event systems
-   revive timer and revive interaction details
-   how death/revival works in solo beyond normal run loss
-   final UI layout
-   final input bindings/controller mapping
-   final names for the four five-level run sections

------------------------------------------------------------------------

# 33. Rejected / Superseded Ideas

Do not silently reintroduce these.

-   **Ashfall as the project name** --- Ruinbound is a separate game.
-   **Turn-based gameplay** --- Ruinbound is real-time 3D.
-   **Pure hack-and-slash combat** --- positioning/timing should matter.
-   **Stamina/energy cost for the universal dodge** --- dodge uses no
    stamina/energy.
-   **Anthropomorphic animal playable cast** --- playable characters are
    humanoid.
-   **Trap-focused dungeon design** --- not desired.
-   **Straight-line procedural floors** --- floors should branch and
    reward exploration.
-   **Repeating Crossbow** --- removed.
-   **Throwing Knives** --- removed.
-   **Chakram** --- removed.
-   **Blunderbuss** --- removed.
-   **Longsword parry identity** --- parry belongs to Sword & Shield.
-   **Greatsword owning major ground shockwaves** ---
    shockwave/ground-impact identity belongs primarily to Hammer.
-   **Bow Epic Rain of Arrows** --- Ricochet Arrow is preferred.
-   **Runesage delayed-cast/delayed-rune gameplay** --- class should be
    immediate and fast.
-   **Vanguard Momentum breaking when hit** --- getting hit must not
    automatically reset it.
-   **Old Vanguard Afterimage that simply repeats attacks beside the
    player** --- current version is a stationary ranged clone that can
    be teleported back to.
-   **Gravemind being dominated by constant low-health micromanagement**
    --- strong healing and more focused Ruin-risk gameplay are
    preferred.
-   **Chronowarden Rewind** --- explicitly rejected.
-   **Worldbreaker as a Greatsword** --- it is a Hammer concept.

------------------------------------------------------------------------

# 34. Decisions That Must Be Preserved

Unless the user intentionally revisits them:

-   This project is **Ruinbound**, separate from Ashfall.
-   Rust + Bevy + Avian3D.
-   Real-time 3D.
-   Intended cooperative dungeon-crawler roguelike with solo support.
-   Combat is fast but positioning/timing matter.
-   Ordinary combat should not be built around one-shots.
-   Universal dodge has no stamina/energy cost.
-   Bosses/elites/minibosses demand stronger dodge/mechanical play.
-   All classes can equip all weapons and gear.
-   Each class has three class abilities.
-   Weapons have distinct basic attacks and weapon abilities.
-   Four gear tiers: Common, Rare, Epic, Legendary.
-   Legendary items are handcrafted and mechanically memorable.
-   Hammer owns shockwaves/ground impacts.
-   Sword & Shield owns parry.
-   Bow Epic is Ricochet Arrow.
-   Greatsword Epic is Phantom Cleave.
-   17 current planned weapon families; Repeating Crossbow, Throwing
    Knives, Chakram, and Blunderbuss are removed.
-   Six-class direction: Warden of Shades, Runesage, Vanguard,
    Wildshaper, Gravemind, Chronowarden.
-   Warden guardian uses an otter.
-   Runesage restores one rune every two basic attacks, even when
    abilities are woven between them.
-   Runesage should not use slow delayed abilities as its core.
-   Vanguard Momentum is not broken by taking damage.
-   Vanguard Afterimage is stationary, attacks the focused enemy at
    range, and can be reactivated to teleport to it.
-   Wildshaper uses fast animal transformations including Rhino, Bear,
    and Panther roles.
-   Gravemind has a strong heal and an Embrace the Ruin attack-ramp
    meter.
-   Gravemind is unlockable.
-   Chronowarden Rewind is rejected; Phase Step / Time Lock / Echo are
    the current approved direction.
-   Standard runs contain 20 levels.
-   Bosses occur at 5, 10, 15, and 20.
-   Levels 5/10/15 use random bosses; Level 20 uses the fixed final
    boss.
-   Dungeons use procedurally assembled handcrafted rooms.
-   Dungeon graphs branch and include intentional dead ends/optional
    content.
-   No emphasis on traps.
-   Map reveals itself through exploration rather than exposing the
    whole floor.
-   Downed players can be revived during the floor.
-   Lost players return next floor if a teammate survives/reaches it.
-   Returning players keep their gear and run progress.
-   Full party wipe ends the expedition.
-   Run progression and permanent progression are separate.
-   Permanent progression should emphasize new possibilities more than
    raw permanent stat inflation.
-   Ruin persists through the expedition and creates risk/reward.
-   Higher Ruin should visibly corrupt the dungeon.
-   The exact 100% Ruin outcome remains intentionally unresolved.
-   Visual style is stylized low-poly atmospheric dark fantasy with
    humanoid playable characters and strong readable VFX.

------------------------------------------------------------------------

# 35. Decision Log

## 2026-08-15 --- Ruinbound core direction

**Status:** CONFIRMED

**Decision:**\
Ruinbound is a real-time 3D dungeon-crawler roguelike with
co-op-oriented design, procedural handcrafted-room assembly, class
abilities, universal weapon access, and a low-poly stylized fantasy
presentation.

**Reason:**\
This combines fast action with meaningful positioning, exploration,
build variety, and manageable visual scope for a Rust/Bevy project.

**Implementation impact:**\
Overall project architecture, camera, combat, procedural generation,
networking, content pipeline.

**Supersedes:**\
Early undefined/new-project state.

## 2026-08-15 --- Twenty-level run and boss cadence

**Status:** CONFIRMED

**Decision:**\
20 levels per standard run. Boss at the end of every fifth level. Levels
5/10/15 randomize bosses; Level 20 always uses the same final boss.

**Reason:**\
Creates predictable run pacing while preserving boss variety and
allowing mastery of the final encounter.

**Implementation impact:**\
Run state, dungeon generation, boss pools, progression pacing.

**Supersedes:**\
None.

## 2026-08-15 --- Branching handcrafted-room generation

**Status:** CONFIRMED

**Decision:**\
Build a dungeon graph first and populate it with compatible handcrafted
room templates. Include branches, dead ends, optional content, and
exploration. Avoid trap-focused design.

**Reason:**\
Procedural replayability without sacrificing room quality or turning the
run into a repetitive straight corridor.

**Implementation impact:**\
Room metadata, sockets, graph generator, map, encounter placement.

**Supersedes:**\
Straight-line room-chain concepts.

## 2026-08-15 --- Co-op death / return system

**Status:** CONFIRMED

**Decision:**\
Use Downed -\> Lost -\> Returned flow. Lost players return next level
with gear/progress if a teammate survives and reaches it. Full party
wipe ends the run.

**Reason:**\
Supports dramatic co-op recovery without making one player's floor death
remove them from the rest of the run.

**Implementation impact:**\
Health/death states, networking, floor transitions, inventory
persistence.

**Supersedes:**\
Traditional immediate permanent run death for each individual co-op
player.

## 2026-08-15 --- Six-class / universal-weapon structure

**Status:** CONFIRMED

**Decision:**\
Six classes, each with three class abilities, while every class can use
every weapon and piece of gear.

**Reason:**\
Class identity and loot identity can combine rather than weapon
restrictions dictating builds.

**Implementation impact:**\
Class framework, ability system, equipment, UI, networking.

**Supersedes:**\
Equipment-defined no-class direction.

## 2026-08-15 --- Runesage fast rune resource

**Status:** CONFIRMED

**Decision:**\
Runesage spends runes on immediate abilities and restores one rune every
two basic attacks. Attacks need not be consecutive and abilities can be
woven between them.

**Reason:**\
Keeps the wizard fast and interactive instead of relying on delayed
spell setup.

**Implementation impact:**\
Class resource component, basic-attack event tracking, ability costs,
HUD.

**Supersedes:**\
Delayed rune-placement/detonation identity.

## 2026-08-15 --- Vanguard Afterimage and Momentum

**Status:** CONFIRMED

**Decision:**\
Momentum is not broken by taking damage. Afterimage creates a stationary
ranged clone attacking the focused enemy and can be reactivated to
teleport to the clone.

**Reason:**\
Encourages aggression and creates a distinctive damage/repositioning
skill.

**Implementation impact:**\
Clone entity behavior, targeting, teleport, Momentum resource.

**Supersedes:**\
Hit-reset Momentum and earlier simple repeated-attack Afterimage
concepts.

## 2026-08-15 --- Gravemind Ruin ramp

**Status:** CONFIRMED

**Decision:**\
Gravemind is unlockable, has a strong heal, and Embrace the Ruin drains
health while active. Its meter rises while attacking, falls while not
attacking, and filling it raises Ruin and significantly increases
damage.

**Reason:**\
Creates a high-risk class without making every ability revolve around
low-health micromanagement.

**Implementation impact:**\
Class unlocks, health drain, meter, Ruin integration, buffs.

**Supersedes:**\
More health-cost-heavy Gravemind direction.

## 2026-08-15 --- Chronowarden Rewind rejected

**Status:** CONFIRMED

**Decision:**\
Do not use Rewind. Current approved direction is Phase Step, Time Lock,
and Echo.

**Reason:**\
Better matches the desired fast combat pace.

**Implementation impact:**\
Chronowarden class design.

**Supersedes:**\
Rewind ability.

## 2026-08-15 --- Removed weapon families

**Status:** CONFIRMED

**Decision:**\
Remove Repeating Crossbow, Throwing Knives, Chakram, and Blunderbuss.

**Reason:**\
User preference and roster focus.

**Implementation impact:**\
Weapon roadmap/content planning.

**Supersedes:**\
Earlier ranged-weapon brainstorm.

## 2026-08-17 --- Four weapon/gear tiers

**Status:** CONFIRMED / IMPLEMENTED ARCHITECTURE

**Decision:**\
Use Common, Rare, Epic, and Legendary tiers. Common/Rare/Epic weapon
abilities follow weapon-family progression. Legendary weapons are named
handcrafted items with unique abilities.

**Reason:**\
Higher-tier drops should change gameplay rather than only increasing
numbers.

**Implementation impact:**\
`GearTier` added to weapon architecture and tier-aware right-click
abilities introduced.

**Supersedes:**\
Earlier assumption that each weapon family would have one fixed
right-click ability.

## 2026-08-17 --- Greatsword vs Hammer identity

**Status:** CONFIRMED

**Decision:**\
Ground shockwaves and major ground-impact mechanics belong primarily to
Hammer. Greatsword focuses on blade arcs, cleaves, execution, and large
slicing attacks.

**Reason:**\
Weapon families need distinct mechanical identities.

**Implementation impact:**\
Rare Greatsword became Executioner's Sweep; Epic Greatsword planned as
Phantom Cleave; Worldbreaker moved to Hammer concept.

**Supersedes:**\
Earlier Greatsword shockwave/Earthsplitter concepts.

## 2026-08-17 --- Rare Bow Splinter Shot targeting

**Status:** CONFIRMED / IMPLEMENTED

**Decision:**\
Splinter Shot creates three homing target-locked fragments. It
prioritizes nearby secondary enemies and redistributes unused fragments
to the main target if alive, or the nearest surviving secondary if the
main target dies.

**Reason:**\
The ability should remain strong in both groups and single-target
encounters while behaving intelligently rather than firing a generic
spread.

**Implementation impact:**\
Projectile kinds, target entities, homing behavior, adaptive target
selection, and target-lock collision handling.

**Supersedes:**\
Initial fixed three-way spread concept.

## 2026-08-18 --- Detailed context preservation

**Status:** CONFIRMED

**Decision:**\
Preserve the detailed approved class kits, exploration/map rules, death
flow, Ruin behavior, visual direction, weapon roster, progression
philosophy, and rejected ideas in the project context rather than
retaining only short summaries.

**Reason:**\
Fresh chats should be able to continue Ruinbound without accidentally
redesigning approved systems or reintroducing rejected concepts.

**Implementation impact:**\
`RUINBOUND_CONTEXT.md` becomes the master design/development handoff.

**Supersedes:**\
Short-form class and system summaries where fuller approved detail is
available.

2026-08-18 --- Weapon swapping and shared ability cooldown

Status: CONFIRMED / IMPLEMENTED

Decision:
Weapon swapping does not cancel an already-active weapon technique and does
not reset the weapon-ability cooldown. Weapon abilities share the current
PlayerCombat.ability_cooldown, so a player cannot cast one weapon ability,
swap, and immediately use another weapon's ability.

Active techniques snapshot the originating weapon's relevant stats when
needed so swapping weapons cannot alter an ability already in progress.

Reason:
Preserves fluid weapon swapping without enabling cooldown bypass or changing
an already-cast technique into a different weapon's stats.

Implementation impact:
weapons/switching.rs, weapons/abilities.rs, active movement/projectile
ability state.

Supersedes:
Earlier prototype rule that weapon switching canceled unfinished
attack/ability state and reset ability cooldown.

2026-08-18 --- Epic Longsword Blade Rush implemented

Status: IMPLEMENTED

Decision:
Blade Rush is a sustained collision-aware rush rather than an instant
teleport/lunge. It can damage enemies passed through once each and finishes
with a wide slash.

Reason:
Makes the Epic Longsword technique meaningfully more aggressive and mobile
than Common Lunging Slash or Rare Crosscut while respecting world collision.

Implementation impact:
Ability-controlled player movement, Blade Rush state, hit tracking, finishing
slash, weapon-stat snapshotting.

Supersedes:
Planned-only Blade Rush implementation.

2026-08-18 --- Epic Greatsword Phantom Cleave implemented

Status: IMPLEMENTED

Decision:
Phantom Cleave is a wide spectral blade projectile that follows full 3D
crosshair aim, passes through enemies, applies strong stagger, and stops on
world geometry. It uses a wide cuboid collision shape.

Reason:
Preserves the Greatsword's large slicing/cleave identity while giving its Epic
ability unusual reach without stealing Hammer's ground-shockwave identity.

Implementation impact:
Projectile kinds, projectile collision shape/orientation, camera pitch range,
arena collision validation.

Supersedes:
Earlier horizontal-only Phantom Cleave implementation assumption.

2026-08-18 --- Epic Bow Ricochet Arrow implemented

Status: IMPLEMENTED

Decision:
Ricochet Arrow now splits instead of having one projectile continue forever.
On a valid ricochet, the parent despawns and creates three child projectiles.
The chain uses three generations: 1 -> 3 -> 9 -> 27. Final-generation
projectiles may hit but do not split again. The same enemy may be damaged more
than once by the ricochet family. Newly spawned children receive a short split
lockout so seams/corners do not consume multiple generations immediately.

Reason:
Creates a distinctive geometry-driven Epic Bow ability with escalating visual
and mechanical payoff while keeping the projectile count bounded.

Implementation impact:
ProjectileKind::Ricochet, RicochetProjectile, recursive child spawning,
ricochet collision normals, split-generation tracking, post-split lockout.

Supersedes:
Earlier single-projectile six-ricochet concept and unique-target restriction.

2026-08-18 --- Arena collider dimensions corrected

Status: IMPLEMENTED

Decision:
Static floor and wall colliders in the test arena now use dimensions matching
their visible Bevy cuboid meshes rather than half-sized values.

Reason:
The previous colliders were much smaller than their visible meshes, creating
apparent projectile tunneling at wall bottoms, seams, and corners. Matching
physics to visuals resolved the underlying collision gaps.

Implementation impact:
arena/mod.rs floor and wall collider dimensions; more reliable projectile and
future movement/combat collision testing.

Supersedes:
Half-sized arena cuboid collider dimensions.

2026-08-18 --- Weapon combat code refactor

Status: IMPLEMENTED

Decision:
Refactor the growing weapon ability and projectile files into focused modules
without changing combat behavior. Weapon abilities now use a dispatcher module
with separate Longsword and Greatsword implementations. Projectile code now
uses a module interface with separate shared movement/collision, Splinter, and
Ricochet modules.

Reason:
abilities.rs and projectiles.rs were becoming too large as Common, Rare,
and Epic mechanics accumulated. Splitting by responsibility keeps future
weapons and projectile behaviors maintainable without prematurely redesigning
the working combat architecture.

Implementation impact:
src/weapons/abilities/, src/weapons/projectiles/, module imports/re-exports,
and weapon plugin system paths. Existing gameplay behavior was preserved and
regression-tested after the moves.

Supersedes:
The earlier single-file weapons/abilities.rs and weapons/projectiles.rs
organization.

Notes:
Continue using the rule move behavior first, redesign second for future
refactors. Do not add abstractions solely to reduce file length when current
module responsibilities remain clear.



------------------------------------------------------------------------

# 38. 2026-08-21 Development Update --- Fallen Gate Playable-Room Integration

**Status:** IMPLEMENTED / CURRENT PROTOTYPE

The first handcrafted room, **ROOM 01 --- The Fallen Gate**, has been integrated into the main Ruinbound combat prototype rather than remaining a separate room-preview project.

## 38.1 Vanguard GLB player visual

The temporary code-generated Vanguard preview was replaced for gameplay use by a real `.glb` character asset.

Current asset location:

``` text
assets/characters/vanguard/vanguard.glb
```

The existing gameplay `Player` entity remains responsible for movement, Avian collision, health, dodge, jumping, `PlayerCombat`, and equipped weapon state. The Vanguard GLB is attached as the player's visual child rather than replacing the gameplay entity.

This preserves the rule that character visuals and gameplay/physics state are separate concerns and allows the same player/combat architecture to support future character models.

The Bevy 0.19 GLB integration uses the current glTF scene-loading approach (`GltfAssetLabel::Scene(0)` / `WorldAssetRoot`).

## 38.2 Fallen Gate integrated with main prototype

`RoomsPlugin` and `CharactersPlugin` are now wired into the main Ruinbound application alongside the existing camera, UI, weapons, player, enemies, arena/startup, and Avian physics plugins.

The old test-arena floor/walls were removed from the active playable space so the Fallen Gate can serve as the current test environment. The arena/startup code is still retained for responsibilities such as spawning the player, test enemies, camera, HUD, cursor setup, and lighting until those responsibilities are separated more formally.

The player currently starts at the Fallen Gate spawn-island area around the room entrance marker.

## 38.3 Fallen Gate visual/environment implementation

The Fallen Gate currently includes a floating-castle entrance environment with:

- floating spawn island
- broken causeway
- large floating courtyard
- gatehouse and towers
- broken courtyard walls/watchtowers
- central broken statue
- collapsed guard post
- crates and castle debris
- banners and castle details
- doorway torches
- courtyard/gate braziers
- distant floating ruins
- layered void/cloud atmosphere
- animated cloud drift
- flickering fire light
- animated low-poly torch flames
- room entrance/exit markers
- loot/lore markers
- void-failure marker/data

The castle/spawn area is intentionally suspended over an open void to reinforce the floating-ruin identity.

Bridge cleanup performed during integration:

- removed the chunky dark-gray broken bridge-railing blocks
- removed the two bridge braziers/brown cylinders near the spawn-side causeway
- retained the gate/courtyard braziers

## 38.4 Shared room geometry helpers

Reusable room geometry/collision helpers were moved out of `fallen_gate.rs` into:

``` text
src/rooms/geometry.rs
```

The room system now distinguishes between visual-only geometry and solid gameplay geometry through shared helpers such as:

``` text
spawn_block(...)                 visual only
spawn_rotated_block(...)         visual only
spawn_solid_block(...)           visible + static Avian collider
spawn_solid_rotated_block(...)   rotated visible + static Avian collider
```

This is intended to be reused by future handcrafted room files rather than duplicating generic block/collider construction inside every room.

**Design/implementation rule:** if geometry visually communicates that the player can stand on it or that it is a major physical obstruction, its physics should agree. Atmospheric/decorative geometry should not automatically receive collision.

## 38.5 Fallen Gate walkable collision

Avian static collision has been added selectively to the Fallen Gate's playable surfaces and major architecture.

Confirmed walkable collision work includes:

- Spawn Island
- Causeway Walkable Core
- Courtyard Walkable Core
- dedicated causeway-to-courtyard connector that closes the previous floor gap
- fractured surface pieces that visually extend playable floor where selected

The bridge/courtyard transition previously contained a gap between the central causeway and courtyard cores; a dedicated solid connector now provides continuous traversal.

Major architectural collision has also been added/selectively expanded for pieces such as courtyard walls, watchtower bodies, statue/pedestal pieces, guard-post structure, gatehouse pillars/arch/towers, and gatehouse buttresses.

Small rubble, banners, clouds, distant floating ruins, underside rocks, flames, and other atmospheric/detail pieces generally remain visual-only unless gameplay later requires collision.

Gatehouse buttress collision was adjusted because tilted solid buttresses behaved like ramps that the player could walk up. Current collision should favor predictable gameplay over perfectly matching every decorative visual slope.

Collision polish is still ongoing; remaining visual pieces should be tested before assuming they are solid.

## 38.6 Reusable void reset system

A reusable room-level void/reset system now exists in:

``` text
src/rooms/void.rs
```

`VoidKillZone` was upgraded from an empty marker to configurable room data containing a `kill_y` threshold.

Conceptually:

``` text
Player falls below room kill_y
        -> find RoomConnector::Entrance
        -> teleport Player to entrance world position
        -> reset VerticalMovement
```

The system uses the entrance marker's `GlobalTransform`, allowing the reset position to remain correct when future room instances are translated/positioned in the world.

The Fallen Gate currently uses a void threshold around:

``` text
kill_y: -25.0
```

For development, falling into the void **resets the player to the room entrance** rather than invoking the future full Downed/Lost/death system. This keeps room testing fast while establishing reusable room failure behavior.

Future rooms can provide different `kill_y` values without changing the generic reset system.

## 38.7 Current room-system organization

The room portion of the project now includes approximately:

``` text
src/rooms/
├── mod.rs
├── components.rs
├── geometry.rs
├── void.rs
└── fallen_gate.rs
```

`RoomsPlugin` currently spawns the Fallen Gate at startup and runs the room's atmosphere animation systems plus the generic void reset system.

Character assets/code now include a Vanguard visual integration under `src/characters/`, with the actual model stored under `assets/characters/vanguard/`.

## 38.8 Next recommended room milestone

The next useful room-system step is to stop hard-coding test-enemy placement in the old arena/startup code and introduce **room-owned enemy/encounter spawn markers and encounter logic**.

Recommended direction:

``` text
Handcrafted room
├── geometry/collision
├── entrance/exit connectors
├── enemy spawn markers
├── loot markers
├── lore/event markers
└── room encounter configuration
```

This will move the prototype closer to the confirmed procedural architecture where the dungeon generator assembles handcrafted rooms and populates them according to room/encounter metadata.

------------------------------------------------------------------------

# 39. 2026-08-21 Decision Log Additions

## 2026-08-21 --- Vanguard GLB integrated as Player visual

**Status:** IMPLEMENTED

**Decision:**  
Use the generated/exported Vanguard `.glb` as the visual child of the existing gameplay `Player` entity. Do not replace the established movement/combat/physics entity with the model itself.

**Reason:**  
Separates character presentation from gameplay state, preserves the working player/combat architecture, and creates a cleaner path for additional playable character models.

**Implementation impact:**  
`src/characters/`, `CharactersPlugin`, player visual spawning, `assets/characters/vanguard/vanguard.glb`.

**Supersedes:**  
Code-generated primitive Vanguard as the intended gameplay character visual.

## 2026-08-21 --- Fallen Gate becomes current playable room prototype

**Status:** IMPLEMENTED

**Decision:**  
Integrate the Fallen Gate into the main Ruinbound project and use it as the current playable room/environment instead of layering it over the old test-arena floor and walls.

**Reason:**  
Allows the existing combat prototype to be tested inside the first real handcrafted room and advances development toward the confirmed procedural-room architecture.

**Implementation impact:**  
`RoomsPlugin`, `arena/mod.rs` startup responsibilities, `src/rooms/fallen_gate.rs`, player/test-enemy placement.

**Supersedes:**  
Standalone Fallen Gate preview and old test-arena geometry as the primary active environment.

## 2026-08-21 --- Shared room geometry/collision helpers

**Status:** IMPLEMENTED

**Decision:**  
Move reusable visual/solid block helpers into `src/rooms/geometry.rs`. Keep room-specific architecture in individual room files.

**Reason:**  
Future handcrafted rooms need the same basic geometry/collision vocabulary without duplicating helper implementations or coupling generic room construction to the Fallen Gate.

**Implementation impact:**  
`src/rooms/geometry.rs`, `src/rooms/mod.rs`, `src/rooms/fallen_gate.rs`.

**Supersedes:**  
Generic block/collision helpers living only inside `fallen_gate.rs`.

**Notes:**  
Do not automatically make every decorative mesh solid. Collision should be intentional and gameplay-readable.

## 2026-08-21 --- Configurable room void reset

**Status:** IMPLEMENTED / PROTOTYPE BEHAVIOR

**Decision:**  
Rooms can define a `VoidKillZone { kill_y }`. The generic room void system resets a player who falls below that threshold to the room entrance and resets vertical movement.

**Reason:**  
Floating rooms and pits need reliable failure handling during development, while different rooms may require different vertical thresholds.

**Implementation impact:**  
`src/rooms/components.rs`, `src/rooms/void.rs`, `src/rooms/mod.rs`, Fallen Gate room metadata.

**Supersedes:**  
One global hard-coded void Y threshold and inert `VoidKillZone` marker.

**Notes:**  
This is currently a development reset, not the final Downed/Lost multiplayer death behavior.


------------------------------------------------------------------------

# 36. Decision Log Template

Append future important decisions using:

``` text
## YYYY-MM-DD — Decision title

Status:
CONFIRMED / EXPERIMENTAL / SUPERSEDED / IMPLEMENTED

Decision:
What was chosen.

Reason:
Why it was chosen.

Implementation impact:
Files/systems likely affected.

Supersedes:
Any older decision this replaces, or "None".

Notes:
Anything still requiring playtesting or a future decision.
```

------------------------------------------------------------------------

# 37. Fresh-Chat Handoff Prompt

When starting a new ChatGPT conversation, attach this file and use:

> We are continuing development of my Rust + Bevy game **Ruinbound**.
> Read `RUINBOUND_CONTEXT.md` first and treat it as the current source
> of truth for established design decisions and implementation status.
> Do not restart the design from scratch. CONFIRMED decisions may not be
> silently changed. EXPERIMENTAL ideas may be tuned. SUPERSEDED/REJECTED
> ideas should not be reintroduced unless I explicitly want to
> reconsider them. IMPLEMENTED status must reflect actual working code
> rather than planned features. Continue from the Recommended
> Development Sequence, and ask me before changing an established design
> decision.

If compiler errors occur, provide the exact current error output along
with the relevant function/file rather than relying only on this
document, because implementation details can change after this file was
last updated.

------------------------------------------------------------------------

# 40. 2026-08-22 Development Update --- Animation, Inventory, Equipment, and Loot

**Status:** IMPLEMENTED / CURRENT PROTOTYPE

This update replaced the stationary character placeholder and debug weapon
switching with reusable animation, inventory, equipment, item identity, armor,
and randomized loot foundations.

## 40.1 Reusable character animation controller

Character animation is separated from character-specific visual spawning.

Current test visual:

```text
assets/characters/xbot/xbot_animated.glb
```

Current animation states:

- Idle
- Run
- Jump
- Dodge

The animation state controller responds to gameplay movement, vertical state,
and dodge state. It does not belong to `vanguard.rs`; the intent is for every
future playable character to reuse the controller while supplying its own
animation-ready model and clips.

The XBot remains a temporary animation-ready placeholder until the Vanguard
model has a reliable rig, skin weights, and compatible animation clips.

## 40.2 Inventory and equipment rules

The player now has:

- three equipped weapon slots
- ten backpack/inventory slots
- one Head armor slot
- one Chest armor slot
- one Hands armor slot
- one Legs armor slot
- one Feet armor slot

Current controls:

- `F` --- pick up nearby equipment or open a nearby chest
- `I` --- open/close inventory
- `1 / 2 / 3` --- select equipped weapon slots
- left-mouse drag --- move, swap, equip, or unequip inventory items
- release a dragged item outside the inventory panel --- drop it into the world

Dragging supports:

- backpack slot to backpack slot
- backpack weapon to any equipped weapon slot
- equipped weapon to backpack
- equipped weapon slot to equipped weapon slot
- backpack armor to its matching armor slot
- equipped armor to backpack
- valid item swapping when both source and destination are occupied

Invalid placements return the item to its original location. The player cannot
drop or unequip the final remaining weapon because the combat architecture
requires one active `Weapon` component.

Dropped items return to the world as reusable pickups and retain their unique
item identity. The old debug `1/2/3` weapon creation was removed.

## 40.3 Weapon switching and active-technique rule

Changing the equipped weapon:

- cancels an unfinished basic attack
- does not cancel an already-active weapon technique
- does not reset the shared weapon-ability cooldown

This preserves the previously approved anti-cooldown-bypass rule. Active
techniques continue using their snapshotted originating weapon statistics.

## 40.4 Item definition and instance identity

Items now have two different IDs:

- `ItemDefinitionId` --- identifies the item design, such as
  `weapon.bow.rare`
- `ItemInstanceId` --- uniquely identifies one particular copy during the run

Every weapon and armor item also has a display name and rarity. Instance IDs
survive pickup, inventory movement, equipping, swapping, dropping, and
re-pickup.

This distinction is intended to support future:

- save data
- multiplayer replication
- rolled affixes
- durability or item history if ever desired
- authoritative loot ownership

## 40.5 Inventory tooltips and rarity presentation

Hovering an inventory or equipped item displays a tooltip.

Weapon tooltips show:

- item name
- rarity and weapon family
- definition ID
- unique instance ID
- damage
- range
- attack cooldown
- attacks per second
- knockback
- stagger power and duration
- attack angle

Armor tooltips show:

- item name
- rarity and armor slot
- definition ID
- unique instance ID
- defense
- maximum-health bonus

Common, Rare, Epic, and Legendary items have consistent rarity colors across
inventory slot backgrounds, borders, pickup visuals, and tooltips. Tooltips are
compact enough to remain readable at the current 1280x720 development window.

Inventory-full pickup attempts produce a visible temporary HUD message instead
of failing silently.

## 40.6 Armor defense and maximum health

Equipped armor now grants both:

- defense
- maximum health

Total defense uses diminishing returns:

```text
applied damage = incoming damage * 100 / (100 + total defense)
```

Examples:

- `0 Defense` --- 0% reduction
- `12 Defense` --- approximately 10.7% reduction
- `100 Defense` --- 50% reduction

All future player-damage sources should call the central
`apply_player_damage` function rather than directly subtracting from player
health. This ensures enemies, bosses, hazards, status effects, and future
network-authoritative damage all respect equipment consistently.

The player has a reusable `BaseHealth` component. Equipped armor health bonuses
are added to base maximum health. When armor changes, the player's current
health percentage is preserved. This prevents repeated armor swapping from
acting as free healing.

Current prototype armor values:

- Common Head --- `+6 Defense`, `+8 Health`
- Common Chest --- `+12 Defense`, `+18 Health`
- Common Hands --- `+5 Defense`, `+7 Health`
- Common Legs --- `+9 Defense`, `+14 Health`
- Common Feet --- `+5 Defense`, `+7 Health`
- Rare Head --- `+12 Defense`, `+16 Health`
- Rare Chest --- `+22 Defense`, `+32 Health`

These are tuning values, not final balance.

The inventory screen displays:

- current health / maximum health
- total armor health bonus
- total defense
- calculated damage-reduction percentage

## 40.7 Structured loot tables

Loot generation is separated from enemies and chests. A loot source references
a reusable `LootTableId`; it does not contain hard-coded item-spawning logic.

Current table categories:

- Normal Enemy
- Elite Enemy
- Miniboss
- Boss
- Common Chest
- Rare Chest
- Boss Chest

Each loot table supports:

- drop chance
- minimum and maximum rolls
- weighted item entries
- minimum floor
- optional maximum floor

The roll system uses a small deterministic random generator. In the future it
should be seeded from the run seed so multiplayer, replays, and bug reports can
reproduce loot results.

Normal prototype enemies currently use the Normal Enemy table and have a 45%
chance to drop one item. This percentage is a test value.

The Fallen Gate guard-post area currently contains a reusable Common Chest.
Pressing `F` opens it and rolls one or two items from the Common Chest table.

Enemy death and chest opening submit generic `LootDropRequest` values containing:

- loot-table ID
- world position
- current floor

The shared loot processor performs the roll and creates world pickups.

## 40.8 Item catalog

`src/loot/catalog.rs` is the central factory that converts an
`ItemDefinitionId` into a configured `Weapon` or `Armor` item.

Adding a future item should normally require:

1. adding its definition ID to the catalog
2. adding its construction/stat definition to `create_item`
3. referencing that ID from one or more loot tables

Enemy and chest code should not need modification when ordinary items are
added.

## 40.9 Current organization

New organization includes:

```text
src/
├── items/
│   └── mod.rs              # item metadata and unique-ID generation
├── loot/
│   ├── mod.rs              # LootPlugin wiring
│   ├── catalog.rs          # item definitions/factory
│   ├── tables.rs           # weighted reusable loot tables
│   ├── drops.rs            # deterministic rolling and drop requests
│   └── chests.rs           # reusable chest behavior
└── weapons/
    ├── inventory.rs        # player inventory and armor-health recalculation
    ├── inventory_ui.rs     # drag/drop UI, summaries, rarity, tooltips
    └── loot.rs             # world pickup visuals and F interaction
```

`LOOT_SYSTEM.md` documents how to add items, tables, enemy categories, and
chests.

## 40.10 Confirmed runtime tests

The following have compiled and run in the prototype:

- three owned weapon slots and `1/2/3` switching
- ten-slot backpack
- armor equipment slots
- drag-and-drop movement and swapping
- dropping outside the inventory panel
- re-picking dropped equipment with `F`
- randomized enemy and chest loot
- item tooltips and rarity colors
- unique item instance IDs
- armor defense reducing enemy damage
- armor increasing maximum health
- inventory health/defense summary
- inventory-full feedback

## 40.11 Next recommended work

Before expanding Legendary content, useful next milestones include:

1. test and tune Normal/Elite/Miniboss/Boss loot-table probabilities
2. add temporary Elite, Miniboss, and Boss test enemies with their table IDs
3. add a clearer chest-open visual state instead of immediately despawning
4. add floor/run context so loot restrictions use the real current floor
5. move prototype item statistics toward data-driven definitions if catalog
   size becomes difficult to maintain
6. implement Vanguard's three class abilities

------------------------------------------------------------------------

# 41. 2026-08-22 Decision Log Additions

## 2026-08-22 --- Three owned weapon slots replace debug weapon creation

**Status:** IMPLEMENTED

Decision:
`1/2/3` select weapons stored in three equipped slots. They no longer generate
test weapons.

Reason:
Weapon ownership, pickups, equipment choices, and loot progression must drive
combat loadouts.

## 2026-08-22 --- Ten-slot shared backpack and five armor slots

**Status:** IMPLEMENTED

Decision:
The player has ten backpack slots, three equipped weapon slots, and Head,
Chest, Hands, Legs, and Feet armor slots.

Reason:
This provides meaningful run inventory constraints without creating a large
grid-management game.

## 2026-08-22 --- Drag outside inventory to drop

**Status:** IMPLEMENTED

Decision:
Equipment is moved and equipped with drag-and-drop. Releasing a dragged item
outside the inventory panel drops it into the world.

Reason:
This is more direct and readable than a separate Drop button and allows the
same interaction model to handle moving, equipping, swapping, and dropping.

## 2026-08-22 --- Definition IDs and unique instance IDs

**Status:** IMPLEMENTED

Decision:
Every item has both a stable design ID and a unique per-copy instance ID.

Reason:
Catalog lookup, saving, multiplayer ownership, and future affixes require a
distinction between an item type and one physical copy.

## 2026-08-22 --- Loot sources reference reusable weighted tables

**Status:** IMPLEMENTED / EXTENSIBLE PROTOTYPE

Decision:
Enemies, elites, minibosses, bosses, and chests use `LootTableId` references and
generic drop requests. Table entries support weights and floor restrictions.

Reason:
Future content should be added through catalogs and tables instead of custom
drop code inside every enemy or room.

## 2026-08-22 --- Armor grants defense and maximum health

**Status:** IMPLEMENTED / VALUES SUBJECT TO TUNING

Decision:
Armor provides both defense and health bonuses. Defense uses diminishing
returns, and changing maximum health preserves current-health percentage.

Reason:
Armor should feel immediately meaningful while remaining scalable. Percentage
preservation prevents free healing through gear swapping.

## 2026-08-22 --- Central armor-aware player damage path

**Status:** IMPLEMENTED

Decision:
Player damage should use `apply_player_damage` rather than direct health
subtraction.

Reason:
All future enemies, bosses, hazards, and status effects must respect armor in a
consistent place.

------------------------------------------------------------------------

# 42. 2026-08-23 Development Update --- Vanguard Abilities, Cinder Hound, and Enemy Navigation

## 42.1 Vanguard abilities and HUD

**Status:** IMPLEMENTED / PLAYABLE PROTOTYPE

The Vanguard class currently has the following ability presentation:

- `Q` --- Lunge
- Passive --- Momentum
- `R` --- Afterimage
- `RMB` --- Piercing Shot while using the Bow

The ability HUD displays readiness and Momentum progress. The current test
loadout starts the player with a Common Bow instead of a Longsword.

## 42.2 Confirmed projectile-speed tuning

**Status:** IMPLEMENTED / VALUES SUBJECT TO TUNING

Current approved prototype values:

- normal player Bow arrows: `40.0`
- Bow Ricochet Arrow: `40.0`
- Gatebow Marksman projectiles: `45.0`
- encounter aggro extension: `50.0`

These values should be preserved in future source packages unless playtesting
intentionally changes them.

## 42.3 Cinder Hound enemy archetype

**Status:** IMPLEMENTED / BEHAVIOR STILL UNDER TEST

The Cinder Hound is the third normal-enemy archetype after the Ruinbound
Footman and Gatebow Marksman. It uses a low quadruped placeholder body with a
visible ember spine.

Current prototype tuning:

- health: `35`
- movement speed: `5.0`
- pounce range: `7.0`
- pounce speed: `15.0`
- pounce duration: `0.45` seconds
- pounce cooldown: `3.0` seconds
- hit range: `1.25`
- damage: `8`
- low stagger resistance

The original circling behavior was removed because its tangential movement
conflicted with navigation around gaps. The Hound now pursues the player
directly and uses an immediate pounce when the ability is available.

The pounce has no windup. It uses a cooldown so the Hound cannot repeatedly
pounce without normal pursuit between uses. A pounce that cannot make progress
for a short timeout cancels back into pursuit.

The Hound should only pounce when the route and landing are considered safe.
It should not let pounce state interrupt normal waypoint navigation around a
gap or solid obstacle.

## 42.4 Reusable committed traversal

**Status:** IMPLEMENTED / EXTENSIBLE PROTOTYPE

Committed movement was separated from ordinary locomotion so future abilities
can reuse it.

`src/enemies/traversal.rs` owns shared committed-traversal policies:

- `Grounded` --- the entire movement must remain supported and avoid world
  geometry
- `AuthorizedGap` --- a purpose-built leap or dash may cross unsupported space
  only when it has a validated landing

The Cinder Hound currently uses the shared system for its pounce. Future enemy
leaps, lunges, charges, and other committed abilities should use the same
policy layer instead of creating unrelated movement safety code.

## 42.5 Enemy code refactor

**Status:** IMPLEMENTED

Enemy responsibilities were separated to prevent `src/enemies/mod.rs` from
becoming too large:

```text
src/enemies/
├── mod.rs                  # shared enemy types, spawning, encounters, plugin wiring
├── navigation.rs           # ground checks, A*, waypoints, obstacle safety, recovery
├── traversal.rs            # reusable committed ability movement policies
├── crowd.rs                # local enemy separation steering
├── melee.rs                # Ruinbound Footman chase and attack behavior
├── hound.rs                # Cinder Hound pursuit and pounce behavior
├── ranged.rs               # Gatebow Marksman behavior
├── hostile_projectiles.rs  # hostile projectile behavior
└── visuals.rs              # placeholder enemy models
```

New enemy archetypes should reuse these shared modules and keep archetype-only
logic in their own files.

## 42.6 Collision-layer decision

**Status:** IMPLEMENTED

Reusable collision layers distinguish:

- World
- Player
- Enemy

Player and enemy character bodies should collide with the World while avoiding
hard physical blocking between characters. Lightweight crowd separation is
used to discourage enemies from occupying the same point without allowing one
enemy to permanently pin another.

## 42.7 Sampled-ground waypoint navigation

**Status:** IMPLEMENTED / RUNTIME-TUNED

Enemy navigation currently combines:

- continuous ground-ahead probes for ledge safety
- shared cached ground samples
- eight-direction A* waypoint searches
- diagonal `10/14` movement costs for shorter grid routes
- bounded and staggered path recalculation to avoid frame stutter
- cached paths and cheap waypoint following between searches
- partial paths when the full goal cannot be reached within the search budget
- world-geometry shape sweeps during actual movement
- local enemy separation
- stuck detection and attempted side-bypass recovery
- room-authored permanent obstacle footprints for major landmarks

Ordinary chase, circling, retreat, return-home movement, and melee movement are
intended to stay on supported terrain. Purpose-built abilities may cross gaps
only through an authorized traversal policy with a safe landing.

### Statue/wall corner-navigation resolution

**Status:** IMPLEMENTED / CONFIRMED WORKING ON GROUND

The Fallen Gate pedestal corner bug was reproduced repeatedly and corrected in
the shared navigation layer. The important runtime findings were:

- the landmark needed one authored rectangular footprint rather than many
  small collision discoveries
- the player's target position sometimes lay inside the enemy-clearance margin
  and had to be projected to a reachable approach edge
- a generic grid path could change sides near a corner and create left/right
  oscillation
- repeated dynamic blocked-cell samples could poison later searches if they
  were not expired
- crowd steering and generic stuck bypasses must not override a committed
  authored corner route
- enemies must face the direction selected by navigation, not always face the
  player, or correct corner movement looks like sideways/backward walking

The stable implementation now includes:

- an authored obstacle box for the statue pedestal
- shortest one- or two-corner authored routes around that box
- a locked obstacle-edge approach while an authored detour is active
- precise corner waypoint completion rather than early corner skipping
- periodic direct-route reconnection after the enemy clears a corner
- temporary dynamic blocked-cell expiration
- a tightened solid pedestal footprint of half extent `4.75`
- movement-direction facing during chase and detours

The Footman and Hound were runtime-confirmed to navigate the ground-level
pedestal corners correctly after these changes. Temporary `NAV` diagnostics
used during investigation were removed after confirmation.

Historical approaches that caused regressions and should not be restored
without a specific reason:

- continuously choosing a new left/right steering side
- advancing a corner waypoint merely because the enemy was nearby
- allowing crowd steering to bend an authored pedestal route
- permanently retaining failed-step samples
- treating the entire climbable surface as ordinary A* ground

## 42.8 Navigation performance findings

**Status:** CONFIRMED DURING TESTING

Rebuilding large ray-tested grids independently for every enemy caused game
stuttering. Performance improved after moving to shared cached samples,
bounded searches, staggered recalculation, and cheap path following.

Avoid returning to thousands of per-enemy ray or shape queries every frame.
An empty path must not trigger a full A* search every frame.

## 42.9 Test-enemy spawn correction

**Status:** IMPLEMENTED

The original Footman test spawn overlapped the statue pedestal. The courtyard
test enemies were moved to clearer floor:

- Footman: `(10, 2, 18)`
- Cinder Hound: `(12, 2, 28)`
- Gatebow Marksman: `(-12, 2, 30)`

Future encounter placements should account for the full collider radius and
must not begin inside landmark geometry.

## 42.10 Player Run animation correction

**Status:** IMPLEMENTED / NEEDS FINAL RUNTIME CONFIRMATION

The player walking/running animation stuttered even before enemies activated,
which isolated it from navigation performance.

Inspection of `xbot_animated.glb` found that the Run clip began at
`0.033333` seconds rather than `0.0`. This caused Bevy to hold the first pose
during the missing initial frame whenever the repeating animation wrapped.

The Run timeline was normalized to begin at `0.0` and end at `0.7` while
preserving all poses, the matching first/last pose, skeleton, mesh, materials,
and the Idle, Jump, and Dodge clips.

Current asset path:

```text
assets/characters/xbot/xbot_animated.glb
```

Confirm in the next runtime test that continuous running now loops smoothly.

## 42.11 Recommended next-session order

1. Runtime-test the latest elevated-target recognition while the player stands
   on both the outer pedestal ledge and the upper cap.
2. Confirm that enemies ascend, remain on top, chase/attack there, and choose a
   safe descent portal only after the player returns to the ground.
3. Re-test ordinary ground corner navigation to ensure traversal changes did
   not disturb the confirmed authored-corner behavior.
4. Verify Footman, Hound, and Marksman combat while on a registered platform.
5. Verify that the corrected Run animation loops smoothly.
6. After traversal is stable, continue the normal-enemy roster with the Bell
   Husk support archetype.

## 42.12 Dedicated step-platform traversal

**Status:** IMPLEMENTED / LATEST TARGET-ON-PLATFORM FIX NEEDS CONFIRMATION

Enemy step-up behavior is deliberately separate from normal ground navigation.
An earlier experiment treated climbable surfaces as ordinary waypoints. It
destabilized corner routing and left enemies stranded on top of the pedestal,
so that experiment was rolled back before the dedicated system was built.

The dedicated traversal state supports:

1. approach a room-authored portal on connected ground
2. commit to the vertical step-up phase
3. cross or remain on the registered platform surface
4. choose a room-authored exit portal
5. commit to the vertical step-down phase
6. receive a short landing-safety window
7. return control to the existing ground navigator

Only registered platforms are traversable. Walls, the statue, rubble, and
other scenery are not automatically climbable merely because a downward ray
finds their top surface.

The reusable room API is:

```rust
navigation_grid.register_step_platform(
    center,
    obstacle_half_extents,
    top_half_extents,
    ground_center_y,
    surface_center_y,
);
```

This generates bidirectional approach, ascent, crossing, descent, and landing
portals for a rectangular step platform. Future maps should register only
geometry intentionally designed for enemy traversal.

The Fallen Gate pedestal currently registers:

```rust
navigation_grid.register_step_platform(
    Vec2::new(0.0, 27.0),
    Vec2::new(4.75, 4.75),
    Vec2::new(3.2, 3.2),
    2.0,
    3.5,
);
```

Full ground-to-ground ascent, crossing, and descent were runtime-confirmed to
work. A later issue showed that a player standing on the layered pedestal was
not always classified as an elevated target because the wider outer ledge and
smaller upper cap have different footprints and player-center heights. The
latest code widens target recognition for the registered layered structure.
That newest change still requires runtime confirmation and must not yet be
marked confirmed.

Traversal-route selection uses horizontal distance plus a small vertical
penalty. This prevents the height change from making every climb appear more
expensive than a visibly much wider route around the platform. Ground-to-ground
traversal is still chosen only when it meaningfully shortens a valid route.

## 42.13 Combat on elevated traversal surfaces

**Status:** IMPLEMENTED / NEEDS COMPLETE RUNTIME CONFIRMATION

Enemies are combat-locked only during the actual committed lift and drop
phases. Once safely on the platform surface:

- the Ruinbound Footman may chase and attack immediately when in range
- the Footman still has no stationary melee windup
- the Cinder Hound may pounce when its cooldown, range, route, and landing
  checks pass
- the Gatebow Marksman may aim and fire with valid line of sight
- an enemy crossing a platform may pause for combat and resume traversal after
  its combat state finishes

If the player moves onto a platform during a crossing, combat becomes available
when the enemy reaches the safe horizontal surface segment. Combat remains
blocked while the enemy is vertically transitioning between levels.

## 42.14 Ruinbound Footman behavior changes

**Status:** IMPLEMENTED / GROUND BEHAVIOR CONFIRMED

The Footman's stationary melee windup was removed. When the player is within
range and the route is reachable, the Footman enters its short forward attack
immediately. A blocked committed attack cancels into recovery rather than
remaining stuck in attack state.

Footman attack reachability checks supporting ground, real world obstruction,
and authored obstacle clearance. While the Footman is confirmed on a registered
traversal surface, the platform's authored ground-level footprint no longer
incorrectly prevents an otherwise clear attack on that same surface.

## 42.15 Current navigation handoff

**Status:** ACTIVE TEST TARGET

**IMPORTANT:** Enemy step-up traversal is still being actively developed. Do
not treat the feature as complete. Ground-level corner navigation is the stable
baseline; elevated target selection, remaining on top, platform combat, and
safe descent must still be tested and tuned without regressing that baseline.

Known confirmed baseline:

- ground-level authored corner navigation works substantially better and was
  reported working as intended
- enemies no longer remain permanently stuck at the statue corners
- Hound navigation and pounce recovery improved
- complete pedestal step-up, crossing, and step-down worked in runtime testing
- navigation debug logs were removed

Newest unconfirmed behavior:

- recognize a player on either layer of the pedestal as an elevated target
- choose an ascent-only portal instead of requiring a complete crossing
- remain in `HoldingPlatform` while the player stays elevated
- attack or pursue on top
- choose a safe descent portal after the player leaves

If enemies still refuse to ascend toward an elevated player, diagnose the
ascent selector specifically. Do not modify the confirmed ground corner system.
The likely rejection points are target classification, ground approach safety,
or the authored raised portal's world-clearance check.

------------------------------------------------------------------------

# 43. August 26, 2026 Navigation Migration and Current Handoff

## 43.1 Shared Rerecast/Landmass navigation architecture

**Status:** IMPLEMENTED / RUNTIME TUNING IN PROGRESS

Footman, Cinder Hound, and Gatebow Marksman pursuit now share the same
Rerecast/Landmass navigation layer. Enemy-specific combat behavior remains
separate, but ordinary pursuit direction is obtained through the shared
navigation agent whenever Landmass has a usable route.

Every enemy created through `spawn_enemy_root` automatically receives its
navigation components and an enemy-specific profile. The profile currently
supports:

- agent radius
- agent height
- preferred movement speed
- maximum speed
- a reserved route-priority value for future enemy types

Future enemies should continue to spawn through `spawn_enemy_root` so they do
not bypass navigation setup. New enemy archetypes should define a profile
rather than adding another independent pathfinding system.

The room produces one asynchronously generated Rerecast navmesh per map. Do
not generate one mesh per enemy. Until the shared mesh is ready, or when
Landmass cannot provide a usable route, the existing Ruinbound navigation,
platform traversal, physics sweeps, corner handling, attacks, and pounces
remain available as safety fallbacks.

## 43.2 Navigation performance rules

**Status:** IMPLEMENTED / CONFIRMED ARCHITECTURAL CONSTRAINT

The shared pursuit system follows these limits:

- at most two navigation-target updates are issued per frame
- each enemy caches its most recent target
- an enemy waits at least `0.25` seconds before requesting another target
- the target normally must move at least `0.8` world units before replanning
- the map owns one asynchronous navmesh build
- an empty or temporarily unavailable shared route must not trigger
  unrestricted per-frame path searches

These rules are part of the future-enemy architecture and should not be
removed merely to make one enemy react more frequently. If faster reaction is
needed, use priority and scheduling rather than allowing every enemy to replan
every frame.

## 43.3 Shared-route anchoring and recovery

**Status:** IMPLEMENTED / NEEDS CONTINUED RUNTIME TESTING

The latest navigation work also includes:

- projection of a moving player target onto supported, enemy-sized floor
- a cached last-known on-navmesh anchor for each enemy
- anchor refreshes while Landmass reports the agent as moving
- recovery toward that anchor when an agent is detected off the shared mesh
- meaningful-displacement checks that ignore tiny physics jitter when
  detecting a platform stall
- Landmass direction taking priority whenever it supplies a usable route

Diagnostics established that the shared mesh may report `mesh_ready=false`
during its asynchronous startup period. Later route diagnostics must be used
to confirm whether an enemy is actually using Landmass or has entered the
legacy fallback. Relevant Landmass states include `Moving`, `NoPath`,
`TargetNotOnNavMesh`, and `AgentNotOnNavMesh`.

Do not diagnose all future stuck cases as steering failures. First determine
whether the mesh is ready, whether the target and agent are on the mesh, and
whether movement is coming from Landmass or the legacy fallback.

## 43.4 Remaining edge-route problem

**Status:** UNRESOLVED / HIGH-PRIORITY RUNTIME TEST

The courtyard gap shown during August 26 testing has a continuous walkable
floor route around it to the player. The player is not standing on isolated
scenery. Footman and Hound have nevertheless stopped at the near edge instead
of committing to the longer route; the Hound has also oscillated back and
forth there.

Legacy diagnostics repeatedly showed successful-looking A* results followed
by rejection of an early waypoint because `ground_clear=false`, for example
near positions around `(-4.6, 2.1, 7.9)`. This means the search can construct a
route whose next cell does not pass the movement-time ground support test.

The edge issue must remain open. The next session should capture, for the same
stuck enemy:

1. whether `mesh_ready` is true
2. its Landmass `AgentState`
3. whether the target is on the navmesh
4. whether its active direction is shared-nav or legacy fallback
5. the rejected waypoint and the ground-support sample that rejected it

Do not solve this by allowing unsupported movement across the gap. The desired
behavior is to select the continuous route around it.

## 43.5 Statue collision simplification

**Status:** IMPLEMENTED IN CODE / NEEDS FIRST RUNTIME CONFIRMATION

Repeated testing showed enemies becoming trapped at several seams around the
broken statue. They no longer consistently entered the gap between its legs,
but Footman and Hound could still catch on individual leg, torso, and pedestal
boundaries.

The statue's visible geometry is now separated from its lower-body navigation
collision:

- the two broken leg meshes remain visible
- their individual narrow solid colliders were removed
- one invisible static box now covers the legs and central lower body
- blocker center: `(0.0, 3.55, 27.1)`
- blocker size: `(3.4, 2.6, 2.3)`
- the blocker begins at the pedestal top surface and reaches the torso
- the pedestal tiers and the intended walkable area around the blocker remain
  unchanged

The blocker is named `Statue Lower Body Navigation Blocker`. It uses Avian's
`RigidBody::Static` and `Collider::cuboid`. `fallen_gate.rs` explicitly imports
`avian3d::prelude::{Collider, RigidBody}`; this import is required for the
current implementation to compile.

This design intentionally presents one clean convex boundary to the navmesh,
legacy planner, and physics sweeps while preserving the broken-statue visual.
It must be runtime-tested from multiple approach angles before the statue issue
is marked resolved.

## 43.6 Known unrelated warning

**Status:** OPEN / NOT THE PRIMARY NAVIGATION CAUSE

Bevy has emitted warning `B0004` because an animated visual entity with
`InheritedVisibility` has a parent without `InheritedVisibility`. This can
cause inconsistent visual behavior and should be corrected, but it does not
explain diagnostics where the shared mesh is unavailable or a waypoint fails
ground support.

## 43.7 Next-session checklist

1. Run `cargo fmt` and `cargo check` on the latest source package.
2. Run the Fallen Gate room and confirm the shared navmesh reaches ready state.
3. Test Footman and Hound approaching the statue from the front, rear, and both
   sides while the player stands beside or behind it.
4. Confirm enemies route around the new solid lower-body blocker and do not
   enter the visible leg gap.
5. Confirm the pedestal top and its intended surrounding walkable area still
   work.
6. Reproduce the edge case and record the shared-nav/legacy status plus the
   first rejected waypoint.
7. Fix the mismatch between legacy A* cell acceptance and movement-time ground
   support without permitting gap crossing.
8. After statue and edge tests pass repeatedly, remove or gate the temporary
   verbose navigation diagnostics.

## 43.8 Current build handoff

The latest packaged source is `Ruinbound_navigation_fix.zip`, version 16 in the
August 26 work session. Version 16 includes the missing Avian imports required
by the invisible statue blocker. The user has not yet reported the runtime
result of that blocker, so no claim that the statue problem is resolved should
be made at the start of the next session.

------------------------------------------------------------------------

# 44. August 27--29 Navigation Resolution and Cleanup

## 44.1 Broken-edge navigation result

**Status:** IMPLEMENTED / RUNTIME CONFIRMED

The remaining courtyard failure occurred where fractured decorative edges met
solid walkable edges. Enemies could reach the seam and stop even though a
continuous route existed around the opening. The room collision was adjusted
so the walkable connection uses clean, level collision rather than inheriting
the fractured visual geometry.

Runtime testing confirmed that the enemies now traverse this part of the map.
Captured Landmass diagnostics showed multiple enemies in `Moving` state with
usable velocity and one correctly reaching its target. The user subsequently
reported that both navigation test cases were working.

The design rule established by this fix is:

- fractured floor edges may remain visually irregular
- surfaces intended to form one walkable route should meet through flat,
  uninterrupted navigation collision
- decorative broken geometry must not create tiny collision seams at the
  boundary of a solid walkable surface

## 44.2 Navigation diagnostic cleanup

**Status:** IMPLEMENTED / CONFIRMED

The temporary `NAV ROUTE` logging used to diagnose mesh readiness, Landmass
state, position, and desired velocity was removed after the navigation tests
passed. Do not restore continuous per-enemy navigation logging unless a new
route failure requires a focused diagnostic pass.

## 44.3 Visibility warning cleanup

**Status:** IMPLEMENTED

The earlier `InheritedVisibility` warnings were addressed by ensuring spawned
visual parents and their relevant descendants participate in Bevy's visibility
hierarchy correctly. These warnings were not the cause of the navigation bug.

## 44.4 Large enemy spawn bundle compile issue

**Status:** RESOLVED

While cleaning the enemy setup, `cargo check` reported that the tuple passed to
`Commands::spawn` was too large to implement `Bundle`. The spawn components
were reorganized so the project compiled again. This was an ECS tuple-size
limit rather than a missing component implementation.

------------------------------------------------------------------------

# 45. Saint of the Broken Siege Boss Development

## 45.1 Boss identity and visual direction

**Status:** CONFIRMED / MODEL IMPLEMENTED

The next boss is the **Saint of the Broken Siege**. His visual direction is a
massive low-poly siege saint constructed from pale, fractured armor and stone,
with asymmetrical heavy limbs, gold trim, a crown-like battlement helm, and a
bright broken red-orange reliquary/core in his chest. The style matches the
low-poly, faceted stone-and-metal language of his arena.

Four Blender model passes were completed. The fourth pass established the
approved current silhouette and chest-core treatment.

Current rigged model asset:

`assets/bosses/saint_broken_siege_rigged_v1.glb`

The verified GLB contains one skin, approximately 70 meshes, and the required
animation actions.

## 45.2 Armature and animation clips

**Status:** IMPLEMENTED / ALL CLIPS PREVIEWED SUCCESSFULLY

The Saint uses a rigid mechanical armature appropriate for his segmented
low-poly construction. The user previewed every clip in Blender's Dope Sheet / 
Action Editor and confirmed that all clips work.

Verified animation names and exported GLB indices:

| GLB index | Animation action |
| ---: | --- |
| 0 | `Saint_FistSlam_Impact` |
| 1 | `Saint_FistSlam_Recovery` |
| 2 | `Saint_FistSlam_Windup` |
| 3 | `Saint_Idle` |
| 4 | `Saint_Walk` |

The Bevy animation graph loads clips using this verified order. Idle and walk
repeat; the slam actions play as one-shot phases with short transitions.

## 45.3 Current boss entity implementation

**Status:** IMPLEMENTED / FIRST MECHANIC ONLY

The new `src/enemies/saint.rs` module defines the boss entity, model scene,
animation setup, locomotion, combat state, fist-slam timing, impact telegraph,
direct impact damage, and expanding shockwave.

Current provisional boss values:

- health: `1600`
- movement speed: `2.15`
- collider radius: `1.2`
- collider cylindrical height: `3.0`
- stagger resistance: `6`
- boss loot table: `LootTableId::BOSS`

The values remain **EXPERIMENTAL / TUNING** until the full encounter can be
playtested.

## 45.4 Fist Slam and visible shockwave

**Status:** IMPLEMENTED / NEEDS IN-ARENA RUNTIME TEST

The Fist Slam is organized as:

1. approach
2. windup
3. impact
4. recovery
5. cooldown and return to approach

Current timing and damage:

- slam range: `5.5`
- windup: `0.8` seconds
- impact phase: `0.20` seconds
- recovery: `1.15` seconds
- cooldown: `1.0` second
- marked central impact radius: `3.2`
- central impact damage: `22`
- shockwave damage: `14`
- shockwave speed: `11` world units per second
- maximum shockwave radius: `17`

Readability is a required part of the mechanic, not optional polish. During the
windup, a translucent emissive red-orange floor disk pulses over the direct
impact area. On impact, a three-layer ground wave appears:

- a bright energy crest
- a hot inner rim
- a wider translucent dust band

The visual ring and gameplay damage check use the same expanding radius. The
player is damaged only when the leading edge crosses their horizontal distance,
preventing invisible or mismatched damage. A grounded player is hit; an airborne
player avoids the wave, making the universal jump the intended response. Dodge
invulnerability remains a valid emergency response for every class.

## 45.5 Saint integration compile and runtime fixes

**Status:** RESOLVED

The Saint module initially produced a privacy error because internal ECS query
component types were more private than the systems registered from
`enemies/mod.rs`. The internal component visibility was changed to
`pub(super)`.

The next build compiled but Bevy panicked at startup with `B0001` because the
shockwave system read the player's `Transform` and mutably queried shockwave
`Transform` without proving the entity sets were disjoint. The player query now
includes `Without<SaintShockwave>`. The corrected version compiled and the game
loaded successfully.

The macOS linker warning that `__eh_frame` is too large for compact-unwind
offsets and the remaining unused/dead-code warnings are non-blocking. They are
not current gameplay failures.

## 45.6 Boss arena asset

**Status:** EXPORTED / VALIDATED

The completed Blender arena was exported as:

`assets/rooms/saint_broken_siege_room.glb`

The export was inspected and contains:

- one scene named `Scene`
- 582 nodes
- 577 meshes
- 15 materials
- no animations
- dedicated reference meshes named `COLLIDER_*`

Important measured arena geometry:

- main arena floor: approximately `72.6 x 1.2 x 72.6`, centered near
  `(0.0, -0.58, 0.0)`
- raised boss platform: approximately `26.4 x 3.0 x 10.6`, centered near
  `(0.0, 1.5, -24.7)`
- four large pillars centered near `(±20.08, 7.5, ±20.08)`

Bevy cannot load the `.blend` file directly; the `.glb` is the runtime asset.

## 45.7 Current startup state and in-progress arena wiring

**Status:** IN PROGRESS / NOT YET DELIVERED FOR RUNTIME TEST

The latest package actually tested on the user's Mac is
`Ruinbound_saint_fist_slam_v3.zip`. That build contains the Saint mechanic and
model but intentionally does **not** spawn the boss. It still starts in the
Fallen Gate. The successful game load therefore did not show the boss or boss
room by design.

An arena-integration pass is now being prepared in the working source. Its
intended changes are:

- change `RoomsPlugin` startup from the Fallen Gate to the Saint arena
- load `rooms/saint_broken_siege_room.glb` as the visible scene
- hide the exported `COLLIDER_*` reference meshes
- add deterministic Avian static collision matching the floor, platform, and
  four major pillars
- create a boss-room entrance and configurable void-reset zone
- spawn the player at approximately `(0.0, 2.0, 24.0)`
- spawn the Saint on the main floor at approximately `(0.0, 2.7, -8.0)`
- remove the Footman, Cinder Hound, and Gatebow Marksman test spawns
- give the Saint an encounter region covering the arena
- attach the existing enemy health-bar UI to the Saint

The Saint is deliberately starting on the main circular floor instead of the
raised rear platform. This allows the first locomotion and shockwave tests to
occur on one continuous navigation surface before stair/platform behavior is
added to the boss encounter.

## 45.8 Next-session checklist

1. Finish and package the boss-arena startup integration.
2. On the user's Mac, merge both `src` and `assets`, then run `cargo fmt`,
   `cargo check`, and `cargo run`.
3. Confirm that the Saint arena replaces the Fallen Gate at startup.
4. Confirm that the player appears near the front of the circular arena and the
   Saint appears on the main floor.
5. Confirm the room model scale, orientation, lighting, and collision match the
   Blender preview.
6. Confirm the Saint model is upright, correctly scaled, and attached to the
   logical boss entity.
7. Confirm idle, walk, windup, impact, and recovery actions transition at the
   intended times.
8. Confirm the floor warning disk is visible for the complete windup.
9. Confirm the shockwave's visible leading edge matches damage timing.
10. Confirm jumping over the wave and dodging through it both avoid damage.
11. Tune height offsets, spawn positions, colors, speed, radius, and damage only
    after the first runtime evidence.
12. After the first mechanic is reliable, build the remaining Saint mechanics
    before treating the boss encounter as complete.

------------------------------------------------------------------------

# 46. 2026-08-31 Development Update — Gear Archetype System (Arcanist / Striker / Bulwark)

**Status:** IMPLEMENTED / CONFIRMED VIA `cargo check`

Ruinbound's armor now has a real archetype system layered on top of the existing defense/max-health armor stats, built from a supplied design document (`RUINBOUND_GEAR_ARCHETYPES.md`).

## 46.1 Three build-defining archetypes

Every piece of archetype armor belongs to one of three archetypes. Archetype does not lock a class or weapon to a piece of gear — a Runesage can wear Bulwark armor, a Vanguard can wear Arcanist armor, and so on. Archetype only describes what the piece leans toward:

- **Arcanist** — light armor, lowest defense of the three; leans on ability damage, cooldown recovery, ability-resource generation.
- **Striker** — medium armor; leans on basic-attack damage, attack speed, critical hits.
- **Bulwark** — heavy armor, highest defense; leans on damage/effects that scale from defensive stats and defensive actions (blocking, mitigating, resisting stagger).

Legacy/placeholder armor created through the original `Armor::new` constructor (the pre-existing Iron / Gatewarden / Ruinforged / generic Legendary sets) predates this system and has no archetype (`Armor::archetype: Option<GearArchetype>` is `None` for those pieces). Nothing about the legacy sets changed.

## 46.2 Four rarity tiers per archetype, 19 items each

Each archetype has 19 items across the existing four gear tiers:

- Common: Head / Chest / Hands / Legs / Feet (5 items)
- Rare: Head / Chest / Hands / Legs / Feet (5 items)
- Epic: Head / Chest / Hands / Legs / Feet (5 items)
- Legendary: Head / Chest / Hands / Feet (4 items — no Legendary Legs piece in the current set)

That is 57 total new gear pieces across the three archetypes.

Base defense/health values scale per tier through a shared multiplier table:

```rust
const TIER_MULTIPLIER: [f32; 4] = [1.0, 1.6, 2.3, 3.2]; // Common, Rare, Epic, Legendary
```

## 46.3 File organization

```text
src/loot/gear/
├── mod.rs        # tier scaling helper (`build`), shared re-exports
├── arcanist.rs   # 19 Arcanist item definitions
├── striker.rs    # 19 Striker item definitions
└── bulwark.rs    # 19 Bulwark item definitions
```

`loot/gear` is a private submodule of `loot` (not `pub`), consistent with the existing rule that catalog internals should not leak outward. `loot/catalog.rs` wires `loot::gear::create_*` into `create_item()`'s existing dispatch. Individual gear constants that need to be referenced from outside `loot` (see the barrier system in Section 48) are re-exported explicitly from `loot/mod.rs` rather than making the whole `gear` module public.

## 46.4 `Armor` struct extended, not replaced

`weapons/components.rs`'s `Armor` struct gained two new fields without changing any existing behavior:

```rust
pub archetype: Option<GearArchetype>,
pub effect: GearEffect,
```

`GearEffect { name: &'static str, description: &'static str }` carries a unique effect's display name and rules text. `Armor::new` (the legacy path) is untouched and still produces `archetype: None, effect: GearEffect::NONE`. A new `Armor::from_gear(...)` constructor is used exclusively by the three archetype catalogs.

**Important distinction, disclosed honestly at the time this work landed:** only defense and max-health are real, working numeric stats. Every unique effect described in `GearEffect` (barrier grants, crit bonuses, resource generation, stagger-resistance scaling, and so on) was, at the time of this section, descriptive/flavor-only — the text exists and displays in tooltips, but no gameplay system read it yet. Section 48 records which of these effects have since been wired to real systems.

Inventory tooltips (`weapons/inventory_ui.rs`) were extended to show an item's archetype and effect text when present.

## 46.5 Loot table integration

`loot/tables.rs` wires all 57 new items into the existing seven `LootTableId` tables (Normal Enemy, Elite Enemy, Miniboss, Boss, Common Chest, Rare Chest, Boss Chest) using the existing floor-gating mechanism:

- Common gear: floor 1+
- Rare gear: floor 3+
- Epic gear: floor 5+
- Legendary gear: floor 8+

These floor gates reuse the loot table system's existing `min_floor`/`max_floor` fields — no new loot-table mechanism was introduced.

## 46.6 Confirmed compile

The user ran `cargo fmt`, `cargo check`, and reported a successful build (`Finished` `dev` profile, pre-existing warning count unchanged, no new warnings/errors attributable to the gear work).

------------------------------------------------------------------------

# 47. 2026-08-31 Development Update — Test Grounds Loot & Encounter Testing Zone

**Status:** IMPLEMENTED / USER-REPORTED WORKING, NOT YET FORMALLY CONFIRMED VIA PASTED `cargo check` OUTPUT IN THIS LOG

To playtest the new gear archetypes and loot tables without needing a full dungeon run, a new dedicated test zone was built and made the game's active Startup zone.

## 47.1 Purpose and scope

Test Grounds is intentionally simple: a flat testing space with many loot chests and on-demand enemy spawning, not a handcrafted narrative room. It exists purely to exercise loot rolls and combat against different enemy categories quickly.

## 47.2 Startup zone change

**Test Grounds now replaces both the Fallen Gate and the Saint of the Broken Siege arena as the game's active Startup zone.**

```rust
// main.rs
.add_plugins(rooms::test_grounds::TestGroundsPlugin)
```

This is a **temporary testing override, not a rejection** of either room. The Fallen Gate (`arena` / `rooms::fallen_gate`) and the Saint arena integration work recorded in Section 45.7–45.8 are unchanged and still compile; swapping the Startup plugin line in `main.rs` brings either back. The Saint-arena-as-Startup-zone integration checklist from Section 45.8 remains open and paused while gear/loot content is under active test — it is the natural next milestone once loot testing is done, not abandoned work.

## 47.3 Chests

Test Grounds spawns a row of loot chests for repeated sampling. A new `repeatable` flag was added to the existing chest system rather than creating a parallel chest type:

```rust
pub struct LootChest {
    ...
    pub repeatable: bool,
}
```

`spawn_repeatable_loot_chest(...)` sits alongside the existing `spawn_loot_chest(...)`. `open_nearby_chest` now only despawns a chest after it's opened when `repeatable` is `false` — a repeatable chest can be opened over and over to sample loot-table rolls without needing to reset the scene.

## 47.4 On-demand enemy spawning

Test Grounds can spawn normal enemies, elites, minibosses, and bosses without inventing new enemy archetypes or a parallel spawn system. It reuses each enemy's existing `spawn_*` function and then overwrites specific components on the freshly spawned entity (Bevy's tuple-bundle `insert` replaces existing components of the same type, which is how the existing codebase already handles this kind of override):

- **Normal enemies** — spawned via each archetype's existing spawn function (Footman, Gatebow Marksman, Cinder Hound), unmodified.
- **Elites** — a normal enemy spawn with `Health` overridden to a higher value (`ELITE_HEALTH = 150.0`) and `LootFloor` set for elite-tier loot rolls.
- **Minibosses** — similarly, `MINIBOSS_HEALTH = 400.0`.
- **Bosses** — spawns the Saint of the Broken Siege via its existing spawn path.

Debug key bindings (`F1`–`F4`) spawn one enemy of each category near the player, tracked by a `TestSpawnCounter` resource so repeated spawns don't stack on top of each other.

## 47.5 Floor override for loot testing

Rare/Epic/Legendary gear is floor-gated (Section 46.5), but no floor-progression system exists yet outside a real dungeon run. A minimal, purely additive `LootFloor(pub u32)` component was added to `enemies/mod.rs`:

```rust
pub struct LootFloor(pub u32);
```

`handle_enemy_death`'s loot-roll logic now reads an enemy's `LootFloor` if present, falling back to `1` (the prior hardcoded behavior) if absent. This lets Test Grounds spawn an "elite" or "miniboss" with `LootFloor(5)` or `LootFloor(8)` to exercise higher-tier loot without needing a real floor-progression system, while every other enemy in the game keeps behaving exactly as before.

Current Test Grounds floor constants:

```rust
const ELITE_LOOT_FLOOR: u32 = 5;
const MINIBOSS_LOOT_FLOOR: u32 = 5;
const BOSS_LOOT_FLOOR: u32 = 8;
```

These are testing conveniences, not final floor-progression balance.

## 47.6 File organization

```text
src/rooms/
└── test_grounds.rs   # TestGroundsPlugin, spawn_test_grounds(), debug spawn keys
```

`rooms/components.rs` gained a `TestGrounds` marker component (alongside the existing `SaintArena` marker) so room-specific systems can identify which room is active if ever needed.

## 47.7 Status and outstanding verification

This work has not yet been confirmed in this log via a pasted `cargo check`/`cargo run` transcript the way the gear-archetype work in Section 46 was. The user reported later in this same session that things "seem to be working," which is the basis for this document's IMPLEMENTED tag, but a full formal compile/runtime confirmation pass — per this project's own rule that IMPLEMENTED status must reflect actual working code, not merely a general impression — is still recommended before this is treated as fully closed out.

------------------------------------------------------------------------

# 48. 2026-08-31 Development Update — Barrier (Shield) System and Death Prevention

**Status:** IMPLEMENTED / USER-REPORTED WORKING, NOT YET FORMALLY CONFIRMED VIA PASTED `cargo check` OUTPUT IN THIS LOG

A generic, reusable temporary-shield system was built and wired into the three archetype gear pieces whose `GearEffect` text already referenced barriers (the gap disclosed in Section 46.4).

## 48.1 Design goal

The user's explicit requirement: barriers should last 15 seconds, and the underlying code should be reusable for gear now and for character abilities and anything else later — not a one-off hack for a single item. A follow-up requirement clarified that damage must be absorbed by a barrier before it touches the player's health bar.

## 48.2 `combat::Barriers` — generic shield component

New file `src/combat/barrier.rs`:

- `Barriers` component: holds a list of active shield charges (`Vec<Barrier>`), each with a source tag, remaining amount, and remaining lifetime.
- `DEFAULT_BARRIER_DURATION: f32 = 15.0` — the project's standard barrier lifetime.
- `Barriers::grant_default(source, amount)` — the entry point most gear/ability code should use; grants a 15-second barrier.
- `Barriers::grant(source, amount, duration)` — explicit-duration variant for the rare case something needs a different lifetime.
- `Barriers::absorb(incoming) -> f32` — consumes damage from the oldest barrier first, returns how much was absorbed.
- `Barriers::total_amount()`, `Barriers::has_any()` — read-only queries (used by the HUD, Section 48.6).
- `tick_barriers` system — ages every barrier and drops ones whose duration expired.
- `BarrierSource` enum — a purely informational tag (`RuneboundRobes`, `RobesOfTheHollowStar`, `CrownOfTheUnbroken`, `Other`) for debugging/HUD purposes; the absorption logic itself does not care about source.

Nothing in `barrier.rs` knows about any specific item. It is intentionally gear-agnostic so future class abilities, consumables, or boss mechanics can grant a barrier through the same component.

## 48.3 `combat::DeathPrevention` — generic "not this time" hook

New file `src/combat/death_prevention.rs`. This is the generic mechanism behind Crown of the Unbroken's "fatal damage instead leaves you at one health" effect, deliberately built so `combat::apply_player_damage` never needs to know Crown of the Unbroken exists:

- `DeathPrevention` component: `activate(barrier_fraction_of_max_health, cooldown_duration)` / `deactivate()` — called every frame by whatever currently grants the effect; `is_active_and_ready()` / `trigger()` — read/consumed by `apply_player_damage`.
- `tick_death_prevention` system — counts down the cooldown.

This follows the same design principle as `CharacterCombatModifiers` (an existing generic component that Vanguard's Momentum already writes into every frame): the generic combat component doesn't know which source is currently driving it.

## 48.4 `apply_player_damage` order of operations updated

`combat::apply_player_damage` (the single centralized player-damage function established back in Section 40.6/41) now applies, in order:

1. defense mitigation (unchanged, existing diminishing-returns formula)
2. **barrier absorption** — a barrier always eats damage before the health bar does
3. death prevention (only checked if the remaining damage would be fatal)
4. whatever remains is subtracted from `Health.current`

```rust
pub fn apply_player_damage(
    health: &mut Health,
    equipment: &Equipment,
    barriers: Option<&mut Barriers>,
    death_prevention: Option<&mut DeathPrevention>,
    incoming_damage: f32,
) -> DamageResult
```

Both new parameters are `Option` so the function stays usable for any entity, with or without either system. `DamageResult` gained `barrier_absorbed: f32` and `prevented_death: bool` fields alongside the existing `incoming`/`blocked`/`applied`.

All five existing player-damage call sites were updated to pass the player's `Barriers`/`DeathPrevention` through: the Ruinbound Footman (`enemies/melee.rs`), the Gatebow Marksman (`enemies/hostile_projectiles.rs`), the Cinder Hound (`enemies/hound.rs`), and both Saint of the Broken Siege damage sources — the Fist Slam direct impact and the expanding shockwave (`enemies/saint.rs`). A read-only audit of the full codebase confirmed no other code path writes to the player's `Health.current` outside this function, so nothing can bypass barrier absorption.

`Barriers` and `DeathPrevention` are auto-inserted onto any entity with a `Player` marker component via a new `combat::CombatPlugin` (the `combat` module previously had no `Plugin` at all — it was pure data plus one function). This works regardless of which room spawns the player (Fallen Gate, Saint arena, or Test Grounds), since it only depends on the `Player` marker.

## 48.5 Ability-use tracking, generalized

Gear effects that trigger "on ability use" needed a generic way to detect that event across every different ability-casting system in the codebase (weapon abilities, Vanguard's Lunge, Vanguard's Afterimage), without duplicating detection logic in each one.

A new `AbilityUseTracker` component (`characters/class_abilities.rs`) mirrors the existing `BasicAttackTracker`/`BasicAttackHitTracker` observed-count pattern already used by Vanguard's Momentum:

```rust
#[derive(Component, Default)]
pub struct AbilityUseTracker {
    total_used: u64,
}
```

It's incremented at three call sites:

- `weapons/abilities/mod.rs::weapon_ability_input` — once, after the match block, gated on `combat.ability_cooldown > 0.0` (every real ability arm sets this cooldown; only the still-unimplemented Legendary placeholder arm doesn't, so this single check after the match reliably detects "a real ability fired" without touching every match arm individually).
- `characters/vanguard_abilities.rs::lunge_input` — on a successful Lunge.
- `characters/vanguard_abilities.rs::afterimage_input` — on the "cast" branch only (spawning a new clone), not the "return to clone" branch.

## 48.6 Gear wired to the barrier system

New file `src/weapons/armor_procs.rs` is the **only** file in the barrier/death-prevention chain that knows any specific item's name — `combat::apply_player_damage` and `combat::Barriers` remain fully gear-agnostic, and any future item is added here without touching combat code.

A new `Equipment::has_armor_piece(definition_id) -> bool` helper (`weapons/components.rs`) lets this file ask "is item X currently equipped in any slot" without duplicating the five-slot iteration pattern.

Wired:

- **Runebound Robes** (Rare Arcanist chest) — using an ability grants a small barrier (`15.0`, current placeholder value).
- **Robes of the Hollow Star** (Legendary Arcanist chest) — using an ability while critically wounded (health at or below 30% of max, current placeholder threshold) grants a larger barrier (`35.0`), on its own 12-second cooldown.
- **Crown of the Unbroken** (Legendary Bulwark head) — fatal damage instead leaves the player at 1 health and grants a barrier equal to 30% of max health, on a 60-second cooldown. This is the one gear piece wired through `DeathPrevention` rather than reacting to ability use; a per-frame sync system keeps `DeathPrevention` activated only while the Crown is actually equipped.

**Explicitly not wired**, disclosed honestly rather than left silently unstated:

- **Spellweaver Grips** ("Overflowing Power", Epic Arcanist hands) — depends on an ability-resource pool that does not exist anywhere in the project yet. Its `GearEffect` text is in place; there is no proc for it.
- The other half of Robes of the Hollow Star's effect text ("ability damage increases as health decreases") — there is no ability-damage stat in the project yet for it to scale.

All numeric values above (barrier amounts, cooldowns, the "critically wounded" threshold, Crown of the Unbroken's health fraction) are explicitly experimental/placeholder, matching the same tuning-value convention used for the Saint's Fist Slam numbers (Section 45.4) — expect them to move after playtesting.

## 48.7 HUD: barrier overlay on the player health bar

`ui/mod.rs`'s existing player health bar (a dark frame containing one red fill `Node` sized by health percentage) gained a second child `Node`, `PlayerBarrierOverlay`, spawned after the red fill so it renders on top of it:

- Positioned at `left = health_percent`, i.e. right where the red fill currently ends.
- Width equals `barrier_total / max_health`, **clamped so it never extends past the bar's right edge** — a barrier visually fills whatever bar space isn't already occupied by health, rather than growing the bar past 100%. This keeps the HUD honest about the fact that a barrier absorbs damage before the health bar moves at all (Section 48.4): as a hit lands, the cyan strip shrinks first, and the red fill only starts dropping once the barrier is exhausted.
- Translucent cyan (`Color::srgba(0.35, 0.85, 1.0, 0.6)`), left as an easy tuning target once seen in-game.
- Zero width (invisible) when there's no active barrier.

## 48.8 File organization additions

```text
src/combat/
├── mod.rs               # Health, DamageResult, apply_player_damage, CombatPlugin (new)
├── barrier.rs            # NEW — generic Barriers/Barrier/BarrierSource
└── death_prevention.rs   # NEW — generic DeathPrevention

src/weapons/
└── armor_procs.rs        # NEW — the only file that names specific gear pieces
```

## 48.9 Status and outstanding verification

Same caveat as Section 47.7: this was built and statically verified (brace/paren/bracket balance, cross-reference checks, borrow-pattern review) without an actual compiler available in the development sandbox. The user reported the overall session's work "seems to be working." A dedicated verification pass is still recommended:

1. `cargo fmt && cargo check && cargo run` with full pasted output.
2. Equip Runebound Robes, use an ability, confirm a barrier grants and the HUD overlay appears.
3. Get critically wounded while wearing Robes of the Hollow Star, use an ability, confirm the larger barrier grants (and respects its cooldown).
4. Equip Crown of the Unbroken, take fatal damage, confirm the player is left at 1 HP with a new barrier instead of dying, and confirm the 60-second cooldown prevents it from re-triggering immediately.
5. Confirm a barrier visibly absorbs a hit (cyan strip shrinks) before the red health fill moves at all.

------------------------------------------------------------------------

# 49. 2026-08-31 Decision Log Additions

## 2026-08-31 — Gear archetype system (Arcanist / Striker / Bulwark)

**Status:** IMPLEMENTED / CONFIRMED VIA `cargo check`

**Decision:**
Add three build-defining armor archetypes (Arcanist, Striker, Bulwark), each with 19 items across the four existing gear tiers (57 total), layered onto the existing `Armor` struct via new `archetype`/`effect` fields rather than replacing it.

**Reason:**
Matches the supplied `RUINBOUND_GEAR_ARCHETYPES.md` design document and gives armor the same "changes how you play" identity weapons already have, without touching class or weapon restrictions.

**Implementation impact:**
`src/loot/gear/` (new), `weapons/components.rs` (`Armor`, `GearArchetype`, `GearEffect`), `weapons/inventory_ui.rs` (tooltips), `loot/catalog.rs`, `loot/tables.rs`.

**Supersedes:**
None — purely additive to the existing armor/loot systems.

**Notes:**
Only defense/health are real numeric stats as of this decision. Unique `GearEffect` text is descriptive-only until a system reads it — see the barrier-system decision below for the first three effects wired to real gameplay.

## 2026-08-31 — Test Grounds becomes the active Startup zone (temporary)

**Status:** IMPLEMENTED / TEMPORARY OVERRIDE, NOT A REJECTION

**Decision:**
Build a simple dedicated loot/encounter testing zone (`rooms::test_grounds`) with repeatable chests and on-demand normal/elite/miniboss/boss spawning, and make it the game's Startup zone in place of the Fallen Gate and the in-progress Saint arena integration.

**Reason:**
Gear archetypes and loot tables needed fast, repeatable testing across enemy categories and floor-gated rarities without requiring a full dungeon run or the not-yet-finished Saint arena startup wiring.

**Implementation impact:**
`src/rooms/test_grounds.rs` (new), `main.rs` (Startup plugin swap), `rooms/components.rs` (`TestGrounds` marker), `enemies/mod.rs` (`LootFloor` component), `loot/chests.rs` (`repeatable` flag).

**Supersedes:**
Nothing permanently. The Fallen Gate and the Saint-arena-as-Startup integration plan (Section 45.7–45.8) are unchanged and still compile; swapping one line in `main.rs` restores either. The Saint arena integration checklist remains the next milestone once loot/gear testing is done.

## 2026-08-31 — Reusable barrier (temporary shield) system

**Status:** IMPLEMENTED / VALUES SUBJECT TO TUNING

**Decision:**
Add a generic `combat::Barriers` component (15-second default duration) that any current or future system — gear, class abilities, consumables, boss mechanics — can grant into, and have `combat::apply_player_damage` absorb from it before touching `Health.current`. Add a parallel generic `combat::DeathPrevention` hook for "fatal damage instead leaves you at 1 HP" effects, kept equally gear-agnostic.

**Reason:**
The user explicitly required barrier code to be reusable beyond the current three gear pieces that need it, not a one-off hack, and required damage to be absorbed by a barrier before it touches the health bar.

**Implementation impact:**
`src/combat/barrier.rs` (new), `src/combat/death_prevention.rs` (new), `src/combat/mod.rs` (`apply_player_damage` signature, `CombatPlugin`), `src/weapons/armor_procs.rs` (new — Runebound Robes, Robes of the Hollow Star, Crown of the Unbroken), `characters/class_abilities.rs` (`AbilityUseTracker`), `weapons/abilities/mod.rs`, `characters/vanguard_abilities.rs`, all five existing player-damage call sites in `enemies/`, `ui/mod.rs` (HUD barrier overlay).

**Supersedes:**
None — `apply_player_damage`'s existing defense-mitigation behavior is unchanged; barrier/death-prevention are additional steps in its pipeline.

**Notes:**
Spellweaver Grips ("Overflowing Power") remains unwired pending a future ability-resource-pool system. Robes of the Hollow Star's ability-damage-scaling half remains unwired pending a future ability-damage stat. All barrier amounts, cooldowns, and thresholds are placeholder tuning values pending playtesting, the same convention used for the Saint's Fist Slam numbers. Not yet confirmed via a pasted `cargo check`/`cargo run` transcript in this log — see Section 48.9.

------------------------------------------------------------------------

# 50. Current Handoff (as of 2026-08-31)

Before moving to new mechanics, the following should be confirmed on the user's machine and this document updated with the actual result (per this project's own rule that IMPLEMENTED status must reflect real working code, not general impressions):

1. `cargo fmt && cargo check && cargo run` — full output for the combined gear-archetype, Test Grounds, and barrier-system work landed in Sections 46–48.
2. Runtime-test Test Grounds: repeatable chests rolling loot at multiple floor overrides, and F1–F4 spawning normal/elite/miniboss/boss enemies correctly.
3. Runtime-test the barrier system end-to-end per the checklist in Section 48.9, including the HUD overlay.
4. Once loot/gear/barrier testing is satisfied, resume the paused Saint arena Startup integration — Section 45.8's checklist is still the relevant next milestone for boss work — by swapping the Startup plugin back and finishing the remaining unconfirmed items there.

No CONFIRMED design decisions from Sections 1–45 were changed by this session's work. Everything in Sections 46–49 is additive to the existing architecture.


------------------------------------------------------------------------

# 51. 2026-08-31 Development Update --- Warden of Shades Class Implemented (Guardian Otter, Shadow Stag, Predator's Spirit)

**Status:** IMPLEMENTED / NOT YET FORMALLY CONFIRMED VIA PASTED `cargo check`/`cargo run` OUTPUT IN THIS LOG

The Warden of Shades class described in Section 19 has a full first implementation: the class itself, all three approved abilities, and a new generic gear-driven Ability Power stat that all three abilities scale from. This session's code could not be compiled in the development sandbox (no working `cargo` toolchain reachable from either the user's machine or the sandbox's network-restricted container -- see Section 51.8), so per this project's own rule this section is intentionally **not** marked CONFIRMED. It follows the same disclosure pattern already used for Sections 47--48.

## 51.1 New generic Ability Power gear stat

**Status:** IMPLEMENTED

The project previously had no concept of gear scaling ability damage even though several `GearEffect` descriptions promised it (Section 46.4's honesty note). A real, minimal, additive stat was built:

```rust
// weapons/components.rs
pub struct Armor {
    ...
    pub ability_power_bonus: f32, // new field, defaults to 0.0 everywhere
}

impl Armor {
    pub fn with_ability_power_bonus(mut self, ability_power_bonus: f32) -> Self { ... }
}

impl Equipment {
    pub fn total_ability_power_bonus(&self) -> f32 { /* sum across all 5 armor slots */ }
    pub fn ability_power_multiplier(&self) -> f32 { 1.0 + self.total_ability_power_bonus().max(0.0) / 100.0 }
}
```

This is the same linear-percentage shape as the existing defense-mitigation formula (Section 40.6), not a new balancing pattern. `Armor::new` and `Armor::from_gear` both default `ability_power_bonus` to `0.0`, so every one of the 57 archetype items and every legacy item is unaffected unless explicitly opted in.

**Only two items are currently wired to a real number:** Apprentice Hood and Apprentice Leggings (Common Arcanist, `src/loot/gear/arcanist.rs`) -- both already had "Increased ability damage" as their `GearEffect` flavor text with no backing mechanic (exactly the gap Section 46.4 disclosed). Each now grants `ability_power_bonus: 8.0`. Wearing both gives `total_ability_power_bonus = 16.0`, so `ability_power_multiplier = 1.16` (every Warden ability below is 16% stronger). No other gear piece was touched -- adding Ability Power to more items is a future, deliberately incremental step, matching this project's established "land one confirmed mechanic at a time" gear-rollout pattern.

**This directly satisfies the user's explicit requirement:** Predator's Spirit was required to "scale with normal ability scaling," meaning gear that increases ability power in any way affects it *and every other ability in the class*. `ability_power_multiplier()` is read by all three Warden abilities (Sections 51.4--51.6), so any future gear roll that raises Ability Power strengthens Guardian Otter's shield/buffs, Shadow Stag's damage, and Predator's Spirit's spectral strikes together, from one shared number.

## 51.2 New generic `combat::StatBuffs` component

**Status:** IMPLEMENTED

Guardian Otter needed a way to grant a temporary movement-speed/damage buff to a *target* that is not necessarily the caster (future multiplayer), without fighting Vanguard's Momentum system, which fully rewrites `CharacterCombatModifiers` every frame. Rather than special-case that interaction, a new component was added alongside the existing `Barriers`/`DeathPrevention` pattern (Section 48.2):

```rust
// combat/buffs.rs (new file)
pub enum BuffSource { GuardianOtter, Other }

pub struct StatBuffs { /* Vec of source-tagged, timed entries */ }

impl StatBuffs {
    pub fn grant(&mut self, source: BuffSource, movement_speed_multiplier: f32, damage_multiplier: f32, duration: f32);
    pub fn movement_speed_multiplier(&self) -> f32; // product of all active entries
    pub fn damage_multiplier(&self) -> f32;
}
```

`StatBuffs` is auto-inserted alongside `Barriers`/`DeathPrevention` by the existing `combat::CombatPlugin` (Section 48.4) and ticked by a new `tick_stat_buffs` system in the same plugin. It is read in exactly two places, both additive on top of `CharacterCombatModifiers` rather than replacing it:

- `player::character_movement` -- movement speed.
- All three weapons' basic attacks (Longsword, Bow, Greatsword in `weapons/melee.rs` and `weapons/bow.rs::fire_bow`) -- damage.

Weapon *abilities* (Crosscut, Charged Cleave, Executioner's Sweep, Blade Rush, Phantom Cleave, Splinter Shot, Ricochet Arrow, Piercing Shot) intentionally do **not** read `StatBuffs` -- only basic attacks scale, matching how Guardian Otter's buff is described in Section 19 ("grants a damage buff" alongside the basic-attack-centric class kit, not a blanket ability-damage buff, which is what `ability_power_multiplier` already covers separately).

## 51.3 Warden of Shades class scaffold

**Status:** IMPLEMENTED

`src/characters/warden.rs` (new file) mirrors `characters::vanguard`'s existing pattern exactly:

```rust
pub struct Warden; // class marker component
pub struct WardenVisual; // visual-child marker

pub fn attach_warden_visual(...) { /* spawns the shared xbot_animated.glb visual, same as Vanguard (Section 40.1) -- Warden has no dedicated model yet, same placeholder-visual status the class framework already uses for every non-Vanguard class */ }
```

Wired into `CharactersPlugin` (`characters/mod.rs`) alongside Vanguard's existing systems; `attach_warden_visual` was added into the same `.chain()`ed Update tuple as `attach_vanguard_visual`.

## 51.4 Ability 1 --- Guardian Otter (`Q`)

**Status:** IMPLEMENTED

Current tuning (`characters/warden_abilities.rs`):

- cooldown: `14.0` seconds
- blessing duration: `8.0` seconds
- crosshair-raycast target range: `25.0` units
- base shield: `25.0` (scaled by Ability Power)
- base damage buff: `+15%` at 0 Ability Power (scaled by Ability Power)
- base movement-speed buff: `+20%` at 0 Ability Power (scaled by Ability Power)

**Targeting -- explicit user correction implemented as specified:** the original design was "nearest teammate within range." Mid-task the user corrected this: "if the player is hovering the crosshair over another player it uses it on them, but if no player is being targeted it is used on the player playing Warden of Shades that used the ability." The implemented `guardian_otter_target` function now does exactly that -- a camera-forward raycast (`SpatialQuery::cast_ray`, same technique already used elsewhere in the codebase for crosshair targeting) checked against a `With<Player>` filter; a hit on another player entity blesses them, and anything else (no hit, or the ray doesn't resolve) falls back to the caster. Ruinbound's current prototype only ever has one live `Player` entity, so in solo testing this will always resolve to self-targeting until a second player exists -- this matches the approved direction in Section 19 ("can target the Warden when playing solo").

On cast, the target's `Barriers` receives a 15-second-default shield (`Barriers::grant_default`, reusing the Section 48.2 barrier system rather than inventing a second shield mechanic) and their `StatBuffs` receives the movement/damage buff for the blessing's `8.0`-second duration. A spectral otter visual (`GuardianOtterVisual`) orbits the blessed player for the same duration, purely cosmetic.

## 51.5 Ability 2 --- Shadow Stag (`E`)

**Status:** IMPLEMENTED / REDESIGNED MID-SESSION PER USER CORRECTION

The first implementation made the Warden themselves dash forward through enemies (mirroring Epic Longsword's Blade Rush pattern). The user explicitly rejected this: "it's not supposed to be a dash ability its supposed to send a shadow out from the player and stun and damage and knockback all enemies it hits while not have the character dash." Shadow Stag was rebuilt around a different, already-existing piece of shared infrastructure instead of a bespoke fix:

Shadow Stag now spawns an independent spectral stag as a wide, piercing `Projectile` entity (new `ProjectileKind::ShadowStag` variant, `weapons/components.rs`) that travels forward on its own through the same shared movement/collision system every arrow and Epic Greatsword's Phantom Cleave already use (`weapons::projectiles::move_projectiles_with_collision`, Section 43/16.2). The Warden only rotates to face the cast direction; nothing in the new implementation ever touches the caster's `Transform` again or drives `AbilityMovement`, so the player does not move.

Current tuning:

- cooldown: `9.0` seconds
- travel speed: `22.0` units/second
- range: `13.0` units (lifetime is derived as `range / speed` rather than hand-tuned separately, so the one number to retune is range)
- base damage: `26.0` (scaled by Ability Power)
- knockback: `2.75`, applied along the stag's travel direction (the shared projectile system's existing knockback-in-travel-direction behavior, not a radial push from the player)
- stagger strength: `3`, stagger duration: `0.9` seconds (comfortably above elite resistance `~2` per Section 12.1's table, below boss resistance `~4`, so it reliably interrupts normal/elite groups without trivializing bosses)
- piercing: every enemy along the stag's path is hit exactly once (the shared system's existing `hit_entities` tracking, reused rather than reimplemented)
- hitbox/visual size: `2.4 x 1.8 x 3.0`, defined once (`weapons::SHADOW_STAG_SIZE`, in `weapons/mod.rs` rather than in `characters`, so the lower-level `weapons` module doesn't depend on `characters`) and read by both the spawned mesh and the collision system's collider selection, so the visible stag and its actual hitbox cannot drift apart -- consistent with this project's core combat principle that "visual effects must accurately match gameplay hitboxes and timing."

`update_shadow_stag` and the old `ShadowStagCharge` player-attached component were deleted entirely; the shared projectile system replaces both.

## 51.6 Ability 3 --- Predator's Spirit (`R`)

**Status:** IMPLEMENTED

Current tuning:

- cooldown: `16.0` seconds
- active duration: `10.0` seconds
- spectral-strike damage: `50%` of the equipped weapon's basic-attack damage (before Ability Power scaling), except Greatsword which multiplies that fraction by `1.4x` for a larger, slower proc matching its heavier swing
- spectral-strike raycast range: `30.0` units

While active, the ability watches the existing `BasicAttackHitTracker` (the same shared hit counter Vanguard's Momentum already observes, Section 48.5) for newly confirmed basic-attack hits. For each one, it raycasts from the camera (the same simplification Vanguard's Afterimage already uses for its ranged clone's target, since `BasicAttackHitTracker` records that a hit landed but not which enemy) and, if the crosshair is over an enemy, lands a bonus spectral strike on it. Damage is `weapon.damage * fraction * equipment.ability_power_multiplier()` -- explicitly reading the same Ability Power multiplier as Guardian Otter and Shadow Stag, per the user's scaling requirement (Section 51.1). The visual/color varies slightly per weapon type (green-tinted spectral bursts, larger for Greatsword) per Section 19's "the effect should interact with the equipped weapon rather than forcing one fixed attack style."

## 51.7 Test-class swap

**Status:** IMPLEMENTED

`src/rooms/test_grounds.rs` -- the game's active Startup zone since Section 47 -- now spawns `Warden` in place of `Vanguard` as the player's class marker. `src/arena/mod.rs` (the inactive Fallen Gate/Saint-arena startup code, currently not the active Startup plugin per Section 47.2) still references `Vanguard` and was deliberately left unchanged, since it isn't the active test zone and changing it wasn't requested.

## 51.8 Verification status and outstanding work

**Status:** OPEN

No `cargo fmt`/`cargo check`/`cargo run` could be run for this work in the development sandbox: the user's machine (reached via the device bridge) has no Rust toolchain installed, and the sandbox's own cloud container has `cargo`/`rustc` but no network access to crates.io (dependency downloads returned `403 Forbidden`). In place of compiling, a manual review was performed instead: brace/paren/bracket balance across every touched/created file, and a full re-read confirming every new/changed function signature matches its call sites and that every system's multiple `Query<...>` parameters are provably disjoint under Bevy's runtime rules (checked explicitly for `guardian_otter_input`'s four queries, `update_guardian_otter_visuals`'s target/visual pair, and `predators_spirit_proc`'s camera/warden/enemy trio). A repo-wide grep also confirmed no other exhaustive `match` on `ProjectileKind` exists that the new `ShadowStag` variant could break, and that no stale references to the deleted `ShadowStagCharge`/`update_shadow_stag` remain anywhere.

Per this project's own rule (IMPLEMENTED status must reflect actual working code, not general impressions -- the same standard already applied to Sections 47.7 and 48.9), the next required step before this section can be marked CONFIRMED is:

1. `cargo fmt && cargo check && cargo run` on the user's machine, full output.
2. Runtime-test Guardian Otter: verify the shield/buff visibly applies (barrier HUD overlay per Section 48.7, and a felt movement/damage change), the otter visual orbits correctly, and -- once a second player exists -- crosshair targeting picks them up instead of defaulting to self.
3. Runtime-test Shadow Stag: verify the spectral stag travels forward on its own without moving the Warden, damages/staggers/knocks back every enemy in its path exactly once, and its visible shape matches where it actually hits.
4. Runtime-test Predator's Spirit: verify spectral strikes proc on confirmed basic-attack hits for at least Longsword, Bow, and Greatsword, and that equipping Apprentice Hood/Leggings visibly increases the damage of all three abilities together (the Ability Power scaling requirement).
5. Once confirmed, update this section's status tag from IMPLEMENTED to IMPLEMENTED / CONFIRMED, matching the convention used in Section 46.6.

------------------------------------------------------------------------

# 52. 2026-08-31 Decision Log Additions (Warden of Shades)

## 2026-08-31 --- Generic Ability Power gear stat

**Status:** IMPLEMENTED / VALUES SUBJECT TO TUNING

**Decision:**
Add `Armor::ability_power_bonus` (defaults to `0.0`) and `Equipment::ability_power_multiplier()` (linear percentage formula, same shape as existing defense mitigation) as a real, shared stat every class ability can read. Wire it to two existing Arcanist Common items (Apprentice Hood, Apprentice Leggings) whose flavor text already promised it.

**Reason:**
The user required Predator's Spirit to scale with "normal ability scaling," meaning any gear that increases ability power affects it and every other ability in the Warden's kit together, not a Predator's-Spirit-specific number. This also closes part of the honesty gap disclosed in Section 46.4 (unique `GearEffect` text with no backing mechanic).

**Implementation impact:**
`weapons/components.rs` (`Armor`, `Equipment`), `loot/gear/arcanist.rs` (two items wired).

**Supersedes:**
None -- purely additive; every other gear piece defaults to no Ability Power.

## 2026-08-31 --- Generic `combat::StatBuffs` component

**Status:** IMPLEMENTED

**Decision:**
Add a reusable, source-tagged, timed movement-speed/damage-multiplier buff component (`combat::StatBuffs`), auto-inserted by the existing `CombatPlugin` alongside `Barriers`/`DeathPrevention`, read additively by player movement and all three weapons' basic attacks (not weapon abilities).

**Reason:**
Guardian Otter needed to grant a temporary buff to a target that may not be the caster, without conflicting with Vanguard's Momentum, which rewrites `CharacterCombatModifiers` every frame. A separate, class-agnostic component avoids any cross-class system-ordering hazard and is reusable by future gear/abilities.

**Implementation impact:**
`src/combat/buffs.rs` (new), `src/combat/mod.rs` (`CombatPlugin`), `player/mod.rs` (`character_movement`), `weapons/melee.rs`, `weapons/bow.rs`.

**Supersedes:**
None -- additive alongside `CharacterCombatModifiers`.

## 2026-08-31 --- Warden of Shades class implemented

**Status:** IMPLEMENTED / NOT YET CONFIRMED VIA `cargo check`/`cargo run`

**Decision:**
Implement the Warden of Shades class (Section 19) in full: class scaffold, and all three abilities -- Guardian Otter (crosshair-targeted shield + damage/movement buff), Shadow Stag (independent piercing spectral-stag projectile, not a player dash), and Predator's Spirit (weapon-reactive spectral basic-attack procs). All three read `Equipment::ability_power_multiplier()`. Test Grounds (the active Startup zone) now spawns Warden instead of Vanguard.

**Reason:**
Requested class build-out, following the approved Section 19 kit and the user's explicit Ability Power scaling and Guardian Otter targeting requirements.

**Implementation impact:**
`characters/warden.rs` (new), `characters/warden_abilities.rs` (new), `characters/mod.rs`, `weapons/components.rs` (`ProjectileKind::ShadowStag`), `weapons/mod.rs` (`SHADOW_STAG_SIZE`), `weapons/projectiles/movement.rs` (collider selection), `rooms/test_grounds.rs`.

**Supersedes:**
None design-wise -- Section 19's approved kit is unchanged. Mid-session implementation correction: Shadow Stag's first pass (a player-dash technique modeled on Blade Rush) was replaced with the independent-projectile design described above, per explicit user correction.

**Notes:**
Not yet confirmed via a pasted `cargo check`/`cargo run` transcript in this log -- see Section 51.8. All numeric tuning values are placeholders pending playtesting, the same convention used for the Saint's Fist Slam and the barrier system.

------------------------------------------------------------------------

# 53. Current Handoff (as of 2026-08-31, updated)

This supersedes Section 50 only by addition -- none of Section 50's outstanding items (gear archetypes, Test Grounds, barrier system) are resolved by this session's work, and none of them were touched by it.

1. Everything listed in Section 50 (items 1--4) is still outstanding and unchanged.
2. `cargo fmt && cargo check && cargo run` -- full output needed for the Warden of Shades work in Sections 51--52, in addition to the still-unconfirmed work from Sections 46--49.
3. Runtime-test the Warden of Shades per the checklist in Section 51.8.
4. Once confirmed, update Section 51's status tag and this handoff section accordingly.

No CONFIRMED design decisions from Sections 1--45 were changed by this session's work. Section 19's approved Warden of Shades kit is unchanged in direction; only its implementation (and Shadow Stag's execution mechanism, corrected mid-session per the user) is new.


------------------------------------------------------------------------

# 54. 2026-09-02 Development Update --- RuneSage Class Implemented (Rune Blast, Runic Surge, Grand Rune)

**Status:** IMPLEMENTED / NOT YET FORMALLY CONFIRMED VIA PASTED `cargo check`/`cargo run` OUTPUT IN THIS LOG

The RuneSage class described in Section 20 has a full first implementation: the class itself, a new resource-gated ability economy (Runes, distinct from every other class's cooldown-gated kit), and all three approved abilities. All three read the existing `Equipment::ability_power_multiplier()` stat (Section 51.1), per the explicit requirement that every RuneSage ability scale with gear. This session's code could not be compiled in the development sandbox -- the sandbox's device-bridge shell has no `cargo`/`rustc` toolchain at all (`which cargo` resolves to nothing; confirmed again immediately before writing this section) -- so per this project's own rule this section is intentionally **not** marked CONFIRMED. It follows the same disclosure pattern already used for Sections 47--48 and 51.8.

## 54.1 New generic `combat::EnemyDebuffs` component

**Status:** IMPLEMENTED

Grand Rune's Runebroken effect (increased damage taken, reduced stagger resistance) needed an enemy-side equivalent of the player-side `combat::StatBuffs` (Section 51.2) and `combat::Barriers` (Section 48.2). No such mechanism existed -- enemy damage application was done inline, separately, at each call site rather than through a shared function. Rather than special-case Runebroken into one ability's code, a new component mirrors the existing pattern:

```rust
// combat/enemy_debuffs.rs (new file)
pub enum DebuffSource { GrandRune, Other }

pub struct EnemyDebuffs { /* Vec of source-tagged, timed entries */ }

impl EnemyDebuffs {
    pub fn grant(&mut self, source: DebuffSource, damage_taken_multiplier: f32, stagger_resistance_reduction: u32, duration: f32);
    pub fn damage_taken_multiplier(&self) -> f32; // product of all active entries
    pub fn stagger_resistance_reduction(&self) -> u32; // max, not sum, of all active entries
    pub fn has_source(&self, source: DebuffSource) -> bool;
}
```

`stagger_resistance_reduction` deliberately takes the **max** across active entries rather than summing them, so stacking the same kind of debuff from multiple sources (or multiple RuneSages in a future multiplayer session) can't zero out a boss's stagger resistance entirely.

Two new shared helper functions were added alongside it in `combat/mod.rs`, following the same shape as the existing `apply_player_damage`:

```rust
pub fn apply_enemy_damage(health: &mut Health, debuffs: Option<&EnemyDebuffs>, base_damage: f32);
pub fn effective_stagger_resistance(base_resistance: u32, debuffs: Option<&EnemyDebuffs>) -> u32;
```

`EnemyDebuffs::default()` is now inserted on every enemy archetype via `enemies::spawn_enemy_root` (the shared spawn function every enemy type already goes through, Section 12), and `tick_enemy_debuffs` is registered in the existing `CombatPlugin` alongside `tick_stat_buffs`/`tick_barriers`.

**Scope decision:** rather than touch every scattered per-ability damage call site in one pass, the two highest-traffic enemy-damage paths were wired to the new helpers -- `weapons::melee.rs`'s basic-attack damage/stagger checks (both occurrences) and `weapons::projectiles::movement.rs`'s shared `move_projectiles_with_collision` system, which already handles all projectile kinds generically (arrows, Splinter Shot fragments, Shadow Stag, and both new RuneSage projectiles) in one place. This makes Runebroken genuinely functional against the majority of combat without a broader, riskier sweep through every weapon-ability file in one session. Any damage path not yet wired to `apply_enemy_damage` simply doesn't apply Runebroken's multiplier yet -- it does not double-apply or break anything, since `EnemyDebuffs` is inert unless read.

## 54.2 RuneSage class scaffold

**Status:** IMPLEMENTED

`src/characters/runesage.rs` (new file) mirrors `characters::warden`'s existing pattern exactly (Section 51.3):

```rust
pub struct Runesage; // class marker component
pub struct RunesageVisual; // visual-child marker

pub fn attach_runesage_visual(...) { /* spawns the shared xbot_animated.glb visual, same placeholder-visual status already used by Warden and every non-Vanguard class */ }
```

Wired into `CharactersPlugin` (`characters/mod.rs`) alongside Vanguard's and Warden's visual-attach systems, in the same chained Update tuple.

## 54.3 New Rune resource economy (distinct from every other class's cooldowns)

**Status:** IMPLEMENTED

Section 20's design calls for RuneSage abilities to be gated by an accumulating "Runes" resource rather than individual per-ability cooldowns, unlike Vanguard and Warden. This is a genuine new mechanic, not a reskin of the cooldown system:

```rust
// characters/runesage_abilities.rs
const RUNESAGE_MAX_RUNES: u8 = 6;
const ATTACKS_PER_RUNE: u8 = 2;   // basic attacks landed before a new Rune is granted
const RUNE_BLAST_COST: u8 = 1;
const RUNIC_SURGE_COST: u8 = 2;
const GRAND_RUNE_COST: u8 = 4;

pub struct RuneResource { current: u8, max: u8, attacks_observed: u64, progress: u8 }
impl RuneResource {
    pub fn current(&self) -> u8;
    pub fn max(&self) -> u8;
    pub fn can_afford(&self, cost: u8) -> bool;
    pub fn spend(&mut self, cost: u8) -> bool;
    pub fn gain(&mut self, amount: u8);
}
```

`update_rune_resource` watches the existing `BasicAttackHitTracker` (the same shared confirmed-hit counter Warden's Predator's Spirit and Vanguard's Momentum already read, Section 51.6/48.5) for newly confirmed (landed) basic-attack hits, and grants one Rune every `ATTACKS_PER_RUNE` (2) confirmed hits, up to the `RUNESAGE_MAX_RUNES` (6) cap. A swing that misses grants no progress. This directly implements Section 20's "Runes build from landing basic attacks" description using existing infrastructure rather than inventing a second attack-tracking mechanism. (Corrected 2026-09-02 -- see Section 57; the first pass read `BasicAttackTracker`, which counts every attack *started* rather than every attack that *lands*.)

Each ability's input system checks `RuneResource::can_afford` before doing anything and calls `spend` on cast; a failed afford check is a silent no-op (matching how every other class's on-cooldown key-press is already a silent no-op).

**New generic HUD support:** no UI existed for a resource bar (only cooldown-based ability icons existed). Two additions were made to the already class-agnostic `characters::class_abilities` module, following its established "generic component, per-class code fills it in" pattern:

- `ClassAbilityDisplay::Locked(&'static str)` -- a new display variant so an ability whose Rune cost isn't currently affordable shows a short reason (e.g. `"NEED 2"`) instead of a plain cooldown timer.
- `ClassResourceHud { label, current, max }` -- a new component any class can attach to drive a generic resource bar.

`ui/mod.rs` gained a matching generic `ClassResourceBar`/`ClassResourceLabel`/`ClassResourcePip` UI element (label plus an 8-pip row, `CLASS_RESOURCE_MAX_PIPS = 8`) that is spawned once in `spawn_hud()` and hidden automatically for any class without a `ClassResourceHud` -- Vanguard and Warden are unaffected and show no resource bar, exactly as before. `sync_runesage_class_state` (in `runesage_abilities.rs`) writes both the per-ability `Locked`/`Ready` state and the `ClassResourceHud` values every frame.

## 54.4 Ability 1 --- Rune Blast (`Q`, cost 1 Rune)

**Status:** IMPLEMENTED

Current tuning (`characters/runesage_abilities.rs`):

- Rune cost: `1`
- travel speed: `30.0` units/second
- range: `26.0` units (lifetime derived as `range / speed`, same pattern as Shadow Stag, Section 51.5)
- base damage: `14.0` (scaled by Ability Power)
- knockback: `0.5`
- stagger strength: `1`, stagger duration: `0.25` seconds (a light, frequent poke -- intentionally the weakest stagger in the kit, since it's the cheapest and most repeatable ability)

Targeting reuses the exact crosshair-raycast-to-point technique already established by `weapons::bow::fire_bow` (Section 16 precedent, also reused for Warden's Guardian Otter, Section 51.4): a raycast from the camera finds the aim point, and the projectile is aimed from its actual spawn position toward that point rather than along raw camera-forward, avoiding parallax drift between the visible reticle and the actual shot. Implemented as a new `ProjectileKind::RuneBlast` variant running through the same shared `move_projectiles_with_collision` system every other projectile in the game uses (Section 43), so it automatically gets correct collision, piercing-vs-non-piercing handling, and now `EnemyDebuffs`-aware damage (Section 54.1) for free.

## 54.5 Ability 2 --- Runic Surge (`E`, cost 2 Runes)

**Status:** IMPLEMENTED

Current tuning:

- Rune cost: `2`
- travel speed: `26.0` units/second
- range: `7.0` units (short, horizontal-only)
- base damage: `22.0` (scaled by Ability Power)
- knockback: `2.4`
- stagger strength: `3`, stagger duration: `0.85` seconds (comfortably above elite resistance, matching the Section 12.1 stagger-strength table already used to tune Shadow Stag, Section 51.5)
- hitbox/visual size: `weapons::RUNIC_SURGE_SIZE = (3.4, 1.6, 1.3)`, defined once in `weapons/mod.rs` (not in `characters`) and read by both the spawned mesh and the collision system's collider selection -- the same "visual and hitbox cannot drift apart" pattern already used for `SHADOW_STAG_SIZE` (Section 51.5), directly satisfying this project's combat principle that visual effects must match gameplay hitboxes exactly.

New `ProjectileKind::RunicSurge` variant, wide and piercing (every enemy in its path is hit once, via the shared system's existing `hit_entities` tracking). A short-lived `RunesageVisualEffect` emergence burst (a torus mesh) spawns at cast time as a purely cosmetic telegraph of the surge appearing in front of the RuneSage.

## 54.6 Ability 3 --- Grand Rune (`R`, cost 4 Runes)

**Status:** IMPLEMENTED

Current tuning:

- Rune cost: `4` (two-thirds of the 6-Rune cap -- the kit's clear capstone)
- target-acquisition raycast range: `30.0` units
- explosion radius: `5.0` units
- base damage: `55.0` (scaled by Ability Power)
- knockback: `3.0`, applied radially away from the explosion center
- stagger strength: `3`, stagger duration: `1.0` second
- Runebroken debuff applied to every enemy hit: `1.35x` damage taken, `-2` stagger resistance, `6.0`-second duration (Section 54.1)

Grand Rune is a targeted point-detonation, not a self-centered nova: a camera raycast finds a target enemy (`Query::contains` against the hit entity, a boolean-only check with no borrow held across the later mutable pass), and the explosion center is computed directly from the raycast's own hit distance (`origin + direction * hit.distance`) rather than by holding a reference to the target's `Transform`. This sidesteps a Rust borrow-checker conflict cleanly instead of working around it: the same system later needs to mutably iterate *all* enemies (to find everyone inside the 5-unit radius, not just the original target), and no reference from the earlier lookup is still alive by that point.

Every enemy within the radius takes damage (via `combat::apply_enemy_damage`), is checked against `combat::effective_stagger_resistance` for a possible stagger/interrupt, is knocked back away from the explosion center, and -- if it carries an `EnemyDebuffs` component (every enemy does, Section 54.1) -- is granted the Runebroken debuff. This means Grand Rune's own follow-up casts, and any other RuneSage ability or weapon hit landed within the 6-second window, all deal 35% more damage and stagger more easily against anything just hit by a Grand Rune, which is the intended "empowers a follow-up" role for the kit's most expensive ability.

## 54.7 Test-class swap

**Status:** IMPLEMENTED

`src/rooms/test_grounds.rs` -- the game's active Startup zone (Section 47) -- now spawns `Runesage` in place of `Warden` as the player's class marker (`use crate::characters::runesage::Runesage;`, `Runesage,` in the player spawn bundle). `src/arena/mod.rs` (the inactive Fallen Gate/Saint-arena startup code, not the active Startup plugin) was deliberately left unchanged, matching the same precedent already documented for the prior Vanguard-to-Warden swap in Section 51.7.

## 54.8 Verification status and outstanding work

**Status:** OPEN

No `cargo fmt`/`cargo check`/`cargo run` could be run for this work in the development sandbox: the device-bridge shell used to read and edit files on the user's machine has no Rust toolchain reachable from it at all (`which cargo` / `which rustc` both resolve to nothing, confirmed immediately before writing this section). In place of compiling, a manual review was performed instead: brace/paren/bracket balance across every touched/created file; a full re-read confirming every new/changed function signature matches its call sites; and validation of the two Bevy-specific technique reuses (crosshair-raycast-to-point aiming, `Query::contains`-only target validation to avoid a borrow conflict) against the already-compiling precedents they were copied from (`bow::fire_bow`, Warden's Guardian Otter). One arity mistake was caught and fixed during this same review: the new "Rune Blast splash-explosion" block in `weapons/projectiles/movement.rs` initially destructured an `Entity` from a `ParamSet` query tuple that hadn't been given one; the query tuple and its destructure were both corrected to include `Entity`, and both `.p0()`/`.p1()` call sites were re-checked afterward for consistency.

Per this project's own rule (IMPLEMENTED status must reflect actual working code, not general impressions -- the same standard already applied to Sections 47.7, 48.9, and 51.8), the next required step before this section can be marked CONFIRMED is:

1. `cargo fmt && cargo check && cargo run` on the user's machine, full output.
2. Runtime-test the Rune resource: verify Runes accumulate from landed basic attacks (1 every 2 hits, capped at 6), the HUD resource bar/pips update correctly, and each ability's icon shows the new "NEED N" locked state when unaffordable.
3. Runtime-test Rune Blast: verify crosshair-accurate aim, correct range/speed, and that it costs exactly 1 Rune.
4. Runtime-test Runic Surge: verify the wide piercing hitbox visually matches where it actually hits multiple enemies, and that it costs exactly 2 Runes.
5. Runtime-test Grand Rune: verify it detonates at the targeted enemy's position (not the RuneSage's own position), damages/staggers/knocks back everyone in the 5-unit radius, and that Runebroken visibly increases damage taken by a follow-up hit within its 6-second window. Verify it costs exactly 4 Runes.
6. Verify Ability Power scaling: equipping Apprentice Hood/Leggings (Section 51.1) should visibly increase the damage of all three RuneSage abilities together.
7. Once confirmed, update this section's status tag from IMPLEMENTED to IMPLEMENTED / CONFIRMED, matching the convention used in Sections 46.6 and 51.8.

------------------------------------------------------------------------

# 55. 2026-09-02 Decision Log Additions (RuneSage)

## 2026-09-02 --- Generic `combat::EnemyDebuffs` component

**Status:** IMPLEMENTED

**Decision:**
Add a reusable, source-tagged, timed enemy-side debuff component (`combat::EnemyDebuffs`), auto-inserted on every enemy via the shared `enemies::spawn_enemy_root`, and two shared helpers (`apply_enemy_damage`, `effective_stagger_resistance`) that read it. Wired into the two highest-traffic enemy-damage paths: `weapons/melee.rs` basic attacks and the shared projectile collision system in `weapons/projectiles/movement.rs`.

**Reason:**
Grand Rune's Runebroken effect (increased damage taken, reduced stagger resistance) needed a real mechanism to attach to and read from. No enemy-side equivalent of the existing player-side `StatBuffs`/`Barriers` components existed. Building the mirror-image component keeps the design consistent with established patterns rather than inventing a one-off flag.

**Implementation impact:**
`combat/enemy_debuffs.rs` (new), `combat/mod.rs` (`CombatPlugin`, new helper functions), `enemies/mod.rs` (`spawn_enemy_root`), `weapons/melee.rs`, `weapons/projectiles/movement.rs`.

**Supersedes:**
None -- additive. Enemy damage paths not yet wired to `apply_enemy_damage` are unaffected and simply don't apply Runebroken yet, matching the same deliberately-bounded rollout precedent as Section 51.1's Ability Power stat.

**Notes:**
`stagger_resistance_reduction` takes the max across stacked debuff entries, not the sum, specifically to prevent multiple Runebroken applications (or a future second RuneSage in multiplayer) from zeroing out a boss's stagger resistance.

## 2026-09-02 --- RuneSage class implemented, Rune resource economy added

**Status:** IMPLEMENTED / NOT YET CONFIRMED VIA `cargo check`/`cargo run`

**Decision:**
Implement the RuneSage class (Section 20) in full: class scaffold, a new accumulating Rune resource (distinct from every other class's per-ability cooldowns), and all three abilities -- Rune Blast (crosshair projectile), Runic Surge (short-range wide piercing burst), and Grand Rune (targeted point-detonation with a Runebroken follow-up debuff). All three read `Equipment::ability_power_multiplier()`. Test Grounds (the active Startup zone) now spawns RuneSage instead of Warden of Shades.

**Reason:**
Requested class build-out, following the approved Section 20 kit and the user's explicit requirement that all RuneSage abilities scale with gear.

**Implementation impact:**
`characters/runesage.rs` (new), `characters/runesage_abilities.rs` (new), `characters/mod.rs`, `characters/class_abilities.rs` (`ClassAbilityDisplay::Locked`, `ClassResourceHud`), `combat/enemy_debuffs.rs` (new, see above), `weapons/components.rs` (`ProjectileKind::RuneBlast`, `ProjectileKind::RunicSurge`), `weapons/mod.rs` (`RUNIC_SURGE_SIZE`), `weapons/melee.rs`, `weapons/projectiles/movement.rs`, `ui/mod.rs` (generic resource bar), `rooms/test_grounds.rs`.

**Supersedes:**
None design-wise -- Section 20's approved kit is unchanged. This is the first class in the project to use a resource economy instead of per-ability cooldowns; the pattern (`RuneResource`, gain-on-landed-basic-attack) is new infrastructure future resource-based classes could reuse, not a one-off.

**Notes:**
Not yet confirmed via a pasted `cargo check`/`cargo run` transcript in this log -- see Section 54.8. All numeric tuning values (Rune costs, damage, stagger, range) are placeholders pending playtesting, the same convention used for the Saint's Fist Slam, the barrier system, and Warden of Shades.

------------------------------------------------------------------------

# 56. Current Handoff (as of 2026-09-02, updated)

This supersedes Section 53 only by addition -- none of Section 50's outstanding items (gear archetypes, Test Grounds, barrier system) are resolved by this session's work, and none of them were touched by it.

1. Everything listed in Section 50 (items 1--4) is still outstanding and unchanged.
2. `cargo fmt && cargo check && cargo run` -- full output needed for the RuneSage work in Sections 54--55, in addition to the still-unconfirmed work from Sections 46--51.
3. Runtime-test the RuneSage per the checklist in Section 54.8.
4. Once confirmed, update Section 54's status tag and this handoff section accordingly.

No CONFIRMED design decisions from Sections 1--45 were changed by this session's work. Section 20's approved RuneSage kit is unchanged in direction; only its implementation is new.


------------------------------------------------------------------------

# 57. 2026-09-02 Post-Implementation Fixes (RuneSage)

## 57.1 Compile error fix -- `charged_cleave` enemy-query mismatch

**Status:** IMPLEMENTED

The user ran `cargo fmt && cargo check` on Section 54's work and returned one error: `E0308` in `src/weapons/abilities/greatsword.rs`, "expected a tuple with 4 elements, found one with 5 elements." Widening `weapons::melee.rs`'s basic-attack `enemy_query` to a 5-element tuple (Section 54.1's `Option<&EnemyDebuffs>` addition) left `charged_cleave` -- Greatsword's Rare-tier ability, called once from `melee.rs::update_player_combat` -- still declared against the old 4-element shape.

Confirmed `charged_cleave` has exactly one call site (`melee.rs`, no other file references it) before widening its signature, so the fix could not silently break another caller. `src/weapons/abilities/greatsword.rs` was updated to match the 5-element tuple and, since the file was already open for that reason, its inline damage/stagger logic was also switched from raw field mutation to the shared `combat::apply_enemy_damage`/`combat::effective_stagger_resistance` helpers (Section 54.1) -- the same small cleanup already applied to `melee.rs`'s basic attacks. Charged Cleave now respects the Runebroken debuff for free; no other Greatsword ability (`greatsword_executioners_sweep`, `fire_phantom_cleave`) shares this query and neither was touched.

**Implementation impact:** `weapons/abilities/greatsword.rs` only.

## 57.2 Rune generation corrected to require confirmed (landed) hits

**Status:** IMPLEMENTED

**User correction:** "lets make it for that the player only gets rune charges if the attack hits an enemy." Section 54.3's first pass watched `BasicAttackTracker`, which counts every basic attack *started* regardless of whether it connects -- so a player could generate Runes by swinging at empty air. This did not match the user's intent and has been corrected.

`characters/runesage_abilities.rs::update_rune_resource` now watches `BasicAttackHitTracker` (`total_confirmed()`) instead of `BasicAttackTracker` (`total_started()`) -- the same confirmed-hit counter Vanguard's Momentum and Warden's Predator's Spirit already read (Sections 48.5, 51.6), inserted on every player automatically by the existing `initialize_character_combat_state` system (`characters/class_abilities.rs`), so no new component wiring was needed. A miss now grants zero progress toward the next Rune; only attacks that actually land on an enemy count, and they still do not need to be consecutive.

For readability, `RuneResource`'s internal `attacks_observed` field was also renamed to `hits_observed` and its doc comments updated to describe confirmed hits rather than attack attempts, so the code reads consistently with the corrected behavior rather than leaving stale "attacks" language next to hit-counting logic.

**Implementation impact:** `characters/runesage_abilities.rs` only (import swap, query type, doc comments, one internal field rename). No other file reads `RuneResource` or `update_rune_resource`.

**Supersedes:** Section 54.3 and the parallel entry in Section 55's decision log, both of which described (and the code, until now, implemented) attack-attempt-based generation. Those sections were also corrected in place (see the note appended to Section 54.3's `update_rune_resource` paragraph) rather than left to silently disagree with the current code.

## 57.3 Verification status

**Status:** OPEN

Neither fix above could be compiled in this session -- same standing limitation as Section 54.8 (no `cargo`/`rustc` reachable from the device-bridge shell). Both were verified manually: brace/paren balance, confirming `charged_cleave` has no other call site before widening its signature, and confirming `BasicAttackHitTracker` is unconditionally inserted on every player entity so the new query in `update_rune_resource` will actually match the Runesage's own entity at runtime.

Next step is the same `cargo fmt && cargo check && cargo run` cycle as Section 54.8, now against this corrected code. Runtime-testing item 2 in Section 54.8's checklist should specifically confirm the new behavior: swinging and missing should **not** advance Rune progress, only a landed hit should.

------------------------------------------------------------------------


------------------------------------------------------------------------

# 58. 2026-09-02 Decision Log Addition -- Rune generation changed to 1 hit = 1 Rune

**Status:** IMPLEMENTED / NOT YET CONFIRMED VIA `cargo check`/`cargo run`

**Decision:**
Every confirmed (landed) basic-attack hit now grants one full Rune immediately. The 2-hits-bank-into-1-Rune system from Section 20.1 / Section 54.3 / Section 57.2 has been removed entirely -- there is no more banked "progress" toward a Rune, only a direct hit-to-Rune grant.

**Reason:**
Playtesting feedback: "sometimes it does not give a rune charge even when the attack hit. It is taking two hits to generate a charge instead of just one attack hit." Asked the user to confirm which of two possible causes this was -- a real drop bug, or a request to change the 2-hits-per-Rune pacing -- since the two have very different fixes and the 2-hit pacing was the explicitly approved design in Section 20.1. The user chose the pacing change: every landed hit should grant a Rune. In hindsight, the "sometimes it does not give a charge" half of the report is very likely just the old 2-hit design itself: the hit that lands on an odd count (1/2 progress) correctly grants nothing, which reads to a player as "I hit and got nothing." Switching to a 1:1 grant removes that ambiguity structurally, so no separate drop bug was chased down this session -- if hits still go uncredited after this change, that would now point to a genuine bug rather than the old banking design, and is worth a fresh report with the console log around it.

**Implementation impact:**
`characters/runesage_abilities.rs` only:
- Removed the `ATTACKS_PER_RUNE` constant.
- Removed `RuneResource`'s `progress` field (nothing left to bank).
- `update_rune_resource` now calls `runes.gain(1)` once per newly confirmed hit directly, instead of incrementing a progress counter and only granting a Rune every `ATTACKS_PER_RUNE` hits.

No other file reads `RuneResource`/`update_rune_resource`, so nothing else needed to change.

**Supersedes:**
Section 20.1's "every two basic attacks restores one rune" approved direction, and the matching descriptions in Sections 54.3 and 57.2. Rune generation is now roughly twice as fast as originally approved -- Rune Blast (1 Rune) is available after every landed hit, Runic Surge (2 Runes) after two, Grand Rune (4 Runes) after four. This is a real balance swing, not a bugfix, and should be kept in mind during playtesting alongside the other still-experimental numbers in Sections 54.4--54.6.

**Notes:**
Not yet confirmed via a pasted `cargo check`/`cargo run` transcript in this log -- same standing limitation as Sections 54.8 and 57.3 (no `cargo`/`rustc` reachable from the device-bridge shell used this session). Verified manually: brace/paren balance, and confirmed no other code in the file still references the removed `ATTACKS_PER_RUNE` constant or `progress` field.

------------------------------------------------------------------------

# 59. 2026-09-02 Ruined Barracks room, connected to Fallen Gate

**Status:** IMPLEMENTED / NOT YET CONFIRMED VIA `cargo check`/`cargo run`

## 59.1 What was built

A new room, Ruined Barracks (`assets/rooms/ruined_barracks.glb`), was wired up and connected to Fallen Gate so the player walks from one straight into the other through Fallen Gate's existing gatehouse arch. Fallen Gate is now the active Startup zone (replacing Test Grounds), and Ruined Barracks spawns immediately after it in the same persistent world -- there is no level-streaming or teleport system, so "connected" means both rooms' static geometry simply coexists with no gap and no overlap at the seam.

Stage 1 scope only, per the requested build order: normal combat with no bell/reinforcement-wave logic yet.

- Player enters from Fallen Gate's existing spawn point.
- Four Ruinbound Footmen (melee) and two Gatebow Marksmen (ranged, on the raised rear platform) activate together on entry, sharing one `EnemyEncounterConfig`.
- The rear gate is a static collider from the moment the room spawns (no unlock condition yet other than combat).
- `open_rear_gate_when_cleared` despawns the rear gate's collider once every `BarracksCombatant` is defeated.
- One guaranteed reward chest sits on the platform, reachable before the exit.
- The two signal bells and the bell-triggered reinforcement wave (`EnemySpawn_Wave2_*`, `EliteSpawn` -- already present as named nodes in the GLB) are deliberately unused this pass, per the explicit request to confirm normal combat and navigation first.

**Files created:**
- `src/rooms/connections.rs` -- new, shared room-to-room connection math (see 59.2).
- `src/rooms/ruined_barracks.rs` -- new, the room module (GLB load, deterministic Avian colliders, entrance/exit markers, enemy spawns, `open_rear_gate_when_cleared`).

**Files edited:**
- `src/rooms/fallen_gate.rs` -- added `FallenGatePlugin` + `setup_fallen_gate_player_and_scene` (player/camera/HUD spawn, mirroring `test_grounds.rs`'s now-inactive `TestGroundsPlugin`), and an `exit_socket()` function (see 59.2) plus a `GATEHOUSE_DEPTH` constant. `spawn_fallen_gate` itself (the room geometry) is untouched.
- `src/rooms/mod.rs` -- registered the two new modules; `RoomsPlugin`'s Startup now runs `(fallen_gate::spawn_fallen_gate, ruined_barracks::spawn_ruined_barracks).chain()` in place of `test_grounds::spawn_test_grounds`; `ruined_barracks::open_rear_gate_when_cleared` added to the Update tuple.
- `src/rooms/components.rs` -- added `RuinedBarracks`, `RearGate`, `BarracksCombatant` marker components.
- `src/main.rs` -- swapped `.add_plugins(rooms::test_grounds::TestGroundsPlugin)` for `.add_plugins(rooms::fallen_gate::FallenGatePlugin)`. Test Grounds and the Saint arena (`arena::ArenaPlugin`) are unchanged and still compile, per the project's established swap-line convention.

**Terminology correction:** the request described "Rapier colliders." This project uses Avian3D 0.7, not Rapier (confirmed in `Cargo.toml` and every existing room module) -- Avian3D was used throughout, consistent with the rest of the codebase.

## 59.2 Reusable room-connection system (`rooms/connections.rs`)

The user specifically asked that this connection code be reusable for future rooms rather than a one-off. Rather than hand-deriving a rotation and Z-offset for this one room pair (an early draft did exactly that, and would have needed the same derivation redone by hand for every future connection), a small shared module was built:

- `RoomSocket { position: Vec3, forward: Vec3 }` -- one end of a connection: a position (X/Z for the seam, Y for the floor's *top surface* height) plus the horizontal direction of travel through it, continuing further along the route.
- `connect_room(exit: RoomSocket, entrance: RoomSocket) -> Transform` -- computes the new room's root `Transform` (a Y-axis-only yaw rotation plus translation; no pitch/roll/scale, matching every room's flat placement) that places the entrance socket exactly on the exit socket and keeps the direction of travel continuous across the seam. The yaw is solved from each socket's horizontal heading (`atan2(x, z)`); because a pure Y rotation never touches a vector's Y component, the entrance socket's local floor height is carried through to a correct, uniform Y lift automatically.

Pattern for a future room: the room being connected *from* exposes an `exit_socket()` function (`fallen_gate::exit_socket` is the first example -- it returns the point at the back face of the gatehouse pillars, at the courtyard floor's own top-surface height, facing further into the level). The new room defines its own local entrance socket (measured from its GLB) and calls `connect_room` once, at the top of its spawn function, to get its room-root `Transform` -- no per-pair math derivation needed. Anything that needs to convert one of the new room's other local-space measurements into world space by hand (independently-spawned enemies, since they don't inherit the room root's transform the way a child entity does) reuses the same `Transform` via `room_transform.transform_point(local)`, which folds in rotation, translation, and the Y lift uniformly. A room can expose more than one `exit_socket`-style function if it has more than one connection point; nothing in the module assumes exactly one.

`ruined_barracks.rs` is the first real usage: its `ENTRANCE_SOCKET` is `Entrance_FromFallenGate` (local Z=+21, forward -Z since deeper progression is local -Z), and `fallen_gate::exit_socket()` is the gatehouse's back face (world Z=55, at Fallen Gate's floor-top height of `FLOOR_THICKNESS / 2.0` = 0.75) facing +Z. `connect_room` resolves this to a 180-degree yaw (Fallen Gate progresses +Z; Ruined Barracks' own "deeper" direction is local -Z, so the room turns to face the opposite way) and a translation of `(0, 0.75, 76)` -- both derived, not hand-typed.

## 59.3 Two geometry issues caught and fixed before this reached the user

Two real bugs were found and corrected while re-deriving the connection math for reuse, neither of which had been surfaced in the earlier (pre-summary) draft of this room:

1. **Floor-height mismatch.** Ruined Barracks' GLB was authored independently with its own floor's top surface at local Y=0.0 (confirmed by directly re-parsing the GLB's `Collision_Floor` node: center Y=-0.18, half-thickness 0.18). Fallen Gate's courtyard floor sits with its top surface at world Y=0.75 (floor center Y=0, `FLOOR_THICKNESS` 1.5). A naive join would have left a 0.75-unit vertical step at the doorway. `connect_room`'s Y handling (see 59.2) now lifts the entire room -- floor, walls, platform, stairs, gate, markers, chest, and enemy spawns alike, since all of them convert through the same `Transform` -- by the correct amount automatically.
2. **Floor gap under the gatehouse arch.** Re-reading `fallen_gate.rs` directly showed its own courtyard floor ("Courtyard Rear Fragment" pieces) stops at world Z=52.5, while the gatehouse structure itself (the pillars a player walks between) runs to Z=55 -- a roughly 2.5-unit strip under the arch has no floor on Fallen Gate's side, a pre-existing gap in that room's own (previously never spawned/tested) geometry. Rather than edit Fallen Gate's already-built room to add a patch there, Ruined Barracks brings its own floor forward past its entrance wall to cover the gap with margin (`Barracks Entrance Threshold`, a small collider sized to the gatehouse's own walkable opening width).

An earlier version of this room (drafted, but never transferred to the repository or reported to the user) used a hand-picked Z-offset of 71 with no Y adjustment and no threshold patch, which would have both mismatched floor heights by 0.75 units and left a void gap under the arch. That version was discarded in favor of the socket-based one described above before any file reached disk.

## 59.4 Known pre-existing limitation, unchanged by this work

`rooms::void::reset_player_from_void` still assumes exactly one `RoomConnector::Entrance` and one `VoidKillZone` exist globally (both still Fallen Gate's own). Ruined Barracks' entrance/exit points are recorded as plain named markers, not `RoomConnector`s, specifically to avoid making that lookup ambiguous. This is a pre-existing single-room assumption, surfaced by connecting a second room for the first time but not fixed by this task -- worth a dedicated pass once a third room makes it matter more.

## 59.5 Verification status

**Status:** OPEN

Nothing in this session could be compiled (no `cargo`/`rustc` reachable from the device-bridge shell). Verified manually instead: brace/paren balance on every new/edited file; every new symbol (`RoomSocket`, `connect_room`, `exit_socket`, the new marker components, `FallenGatePlugin`) cross-referenced against its actual definition and call sites; `WorldAssetRoot` confirmed to resolve the same way it does in `saint_arena.rs` (via `bevy::prelude::*`, no explicit import needed); the new `FallenGatePlugin`/player-spawn code checked line-for-line against `test_grounds.rs`'s already-working `TestGroundsPlugin` for identical API usage; and the connection math re-derived independently (see 59.2-59.3) rather than trusted from the first draft.

Next step is the standard `cargo fmt && cargo check && cargo run` cycle. Specific things worth confirming in-game beyond "does it compile": the player can walk from Fallen Gate's spawn through the gatehouse arch into Ruined Barracks with no visible seam, step, or gap in the floor; all six enemies activate together on entry; the rear gate blocks the exit until every enemy is dead, then despawns; enemy navigation actually routes through the gatehouse opening (this is the first time Landmass/Rerecast pathing has had to cross a room-to-room seam, not just a single room's interior); and the reward chest and its loot are reachable on the platform.

------------------------------------------------------------------------

# 60. 2026-09-02 Fallen Gate / Ruined Barracks: first playtest fixes (fall-through seam, navigation lag)

**Status:** IMPLEMENTED / NOT YET CONFIRMED VIA `cargo check`/`cargo run`

Fallen Gate and Ruined Barracks compiled and ran for the first time (Section 59's work). Playtesting surfaced two real bugs, both diagnosed from the actual runtime console log and a screenshot rather than guessed at blind, then fixed.

## 60.1 Fall-through gap at the gatehouse arch

**Symptom:** the player fell through the floor specifically at Fallen Gate's gatehouse arch (screenshot showed the player mid-fall over a dark void, with Ruined Barracks' lit floor and a Footman visible through the archway beyond).

**Diagnosis:** Section 59's "Barracks Entrance Threshold" patch (added to bridge the ~2.5-unit gap between Fallen Gate's own courtyard floor and its gatehouse's back face) had its far edge positioned at exactly world Z=55 -- the same coordinate where Ruined Barracks' own "Barracks Floor Collision" begins. Two separate static Avian colliders meeting at exactly the same coordinate, with zero overlap, is a well-known case for a kinematic character controller to catch and fall through at the seam.

**Fix:** `src/rooms/ruined_barracks.rs` -- widened and re-centered the threshold patch (local Z 19.0..25.5, was 21.0..25.5; X width 10.0, was 9.0) so it now overlaps Ruined Barracks' own floor by 2 units at one edge and Fallen Gate's rear courtyard fragment by 2 units at the other, instead of touching either exactly. Confirmed the same collider is still an Avian `RigidBody::Static` + `Collider::cuboid`, same Y placement (unaffected -- the original floor-height/Y-lift derivation from Section 59 was not in question here, only the Z-extent).

## 60.2 Navigation lag near Ruined Barracks' stairs/platform

**Symptom:** noticeable frame-rate drop specifically after entering Ruined Barracks, worse while on/near the raised platform and stairs, gone once every enemy in the room was dead.

**Diagnosis:** `src/navigation_mesh.rs::update_navigation_targets` called `project_target_onto_supported_floor` -- a fallback search trying up to 4 radii x 16 samples, each doing up to 5 ground raycasts (as many as ~320 `SpatialQuery` calls) -- for *every* engaged enemy on *every single frame*, before checking whether a repath was even due yet. `NavigationQueryBudget` only gated the cheap final step (actually inserting `AgentTarget3d`), not this expensive projection, contradicting the project's own confirmed navigation requirement that "navigation queries must remain budgeted and cached to prevent frame stuttering." With all 6 of Ruined Barracks' enemies engaged simultaneously (confirmed via the console log -- six consecutive "Enemy encounter activated!" lines) near stairs/platform geometry where the height-discontinuity check (`maximum_ground - minimum_ground <= 0.8`) is far more likely to fail on the first several candidates, forcing the search closer to its worst case, this matches every reported symptom: Barracks-specific, worse near the stairs/platform, gone once the query count (engaged enemies) drops to zero.

**Fix:** `src/navigation_mesh.rs::update_navigation_targets` -- added a cheap pre-check (`cache.has_target && cache.seconds_since_request < budget.minimum_repath_interval`) before calling `project_target_onto_supported_floor`, duplicating the time-since-last-request half of `NavmeshRouteCache::should_repath`'s own condition. `should_repath`'s full check (including its distance-moved half, which still needs the projected target) still runs afterward before anything is actually requested, so the final repath decision is provably unchanged -- this only skips computing a target that would have been discarded anyway. This is a calling-frequency fix, not a change to steering/pathing behavior, per the project's standing caution about changing enemy navigation.

The console log also showed melee combat working correctly during this same playtest (`Enemy: ATTACK` / `Enemy: CHASE` / `Player hit for 10.0`), which the original bug report ("melee enemies not targeting the player, just walking back and forth") had not shown. The working theory is that severe per-frame lag was itself making melee movement look erratic rather than there being a second, separate targeting bug -- flagged to the user to specifically retest melee behavior once 60.2's fix is confirmed, rather than assumed fixed.

**Not addressed this pass:** the log also showed the two Gatebow Marksmen dealing 9 damage per hit on a fast double-fire cadence, taking the player from 91 HP to 0 in a very short window. This is a real balance concern surfaced by the same playtest but is a tuning question, not a bug, and was left for the user to weigh in on rather than changed unilaterally.

**Verification status:** not compiled (no `cargo`/`rustc` in this environment, same standing limitation as every other entry in this log). Verified manually: brace/paren balance on both edited files, and confirmed `update_navigation_targets`'s final `AgentTarget3d`/cache-update logic is byte-for-byte unchanged from Section 59's version, only reordered behind the new cheap check.

------------------------------------------------------------------------

# 61. 2026-09-02 Ruined Barracks: platform navigation registration + flattened floor tiles

**Status:** IMPLEMENTED / NOT YET CONFIRMED VIA `cargo check`/`cargo run`

Second round of playtest fixes, following Section 60's fall-through-seam and navigation-lag fixes.

## 61.1 Melee enemies still not reaching the player ("getting stuck on things")

**Symptom:** after Section 60's fixes, ranged enemies (Gatebow Marksmen) worked correctly, but melee enemies (Ruinbound Footmen) still failed to close on a player standing on Ruined Barracks' raised platform -- described as getting stuck rather than pathing around.

**Diagnosis:** `spawn_ruined_barracks` never registered the raised platform with `EnemyNavigationGrid`, unlike `spawn_fallen_gate`, which registers its own statue pedestal via both `register_obstacle_box` (so ground-level A* knows to route *around* the landmark instead of into it) and `register_step_platform` (which builds the "portal" waypoints that are the *only* sanctioned way ordinary enemy locomotion may move from ground level onto a raised surface -- confirmed by reading `enemies/navigation.rs::register_step_platform` and `enemies/mod.rs::enforce_enemy_ground_safety`, whose grounded-safety check explicitly allows bypassing itself only when `EnemyGroundSafety.authorized_landing` is set or `navigation.allows_vertical_traversal()` -- both of which trace back to this registration). Without it, ground melee had no legal path onto the platform at all and no grid awareness that its footprint is solid, so it could only walk into the platform's vertical face and stop -- exactly "stuck on things." Ranged enemies were unaffected because they don't chase; they aim and fire from a fixed spawn position.

**Fix:** `src/rooms/ruined_barracks.rs::spawn_ruined_barracks` -- added `mut navigation_grid: ResMut<EnemyNavigationGrid>` as a new system parameter (Ruined Barracks now shares this resource with Fallen Gate the same way both already share `EnemyPlugin`'s other systems) and, before spawning the room root, calls both `register_obstacle_box` and `register_step_platform` for the platform. Because `EnemyNavigationGrid` operates in world space and Ruined Barracks' room root is not at world-space identity (unlike Fallen Gate's, per Section 59), every number passed is the platform's known local placement (`Vec3::new(0.0, 2.05, -15.9)`, half-extents `(10.6, 4.3)` from its `Collider::cuboid(21.2, 0.4, 8.6)`) converted through `room_transform.transform_point` by hand, plus this room's own already-established ground/platform enemy-root heights (0.18 local / 2.43 local, both +0.75 for the room's floor-height lift from Section 59) as `ground_center_y`/`surface_center_y`. `top_half_extents` and the `with_target_region` margins mirror the proportions `spawn_fallen_gate` uses for its own pedestal rather than being invented fresh. `with_lane_inset(3.75)` pulls the two climb lanes in from the platform's bare outer edge to approximately world X=+-6.6, matching where this room's actual visual stairs sit, so climbing at least roughly tracks the stairs instead of appearing at a bare corner.

**Caveat, stated plainly:** this is a best-effort first pass at genuinely new numbers (Fallen Gate's own pedestal registration was tuned through actual play; Barracks' has not been). It mirrors the established, working pattern exactly rather than inventing a new mechanism, but the exact height/margin values may still need retuning once seen in motion -- confirm melee enemies can actually reach a player standing on the platform, and that they climb via roughly the stair positions rather than an odd corner, before treating this as done.

## 61.2 Ground floor visual flattening (`ruined_barracks.glb`)

**Request:** keep the ground floor's paneled look (it's built from many individual tile meshes) but make the floor read as one flat, even surface rather than each panel sitting at a visibly different height.

**What was found:** the room's main floor is authored as 336 individual `Floor_XX_YY` mesh nodes (a 16x21 grid), each with a small random per-tile Y jitter (roughly -0.175 to -0.225, clustered around the `Collision_Floor` node's own center Y of -0.18) -- a "broken slab" look. This is purely a visual-asset property of the GLB; it has no effect on gameplay collision, which has always used one single flat `Collider::cuboid` box (Section 59) independent of the visual mesh.

**Fix:** edited `assets/rooms/ruined_barracks.glb` directly (a from-scratch Python glTF-binary reader/writer, no Blender round-trip) to set every `Floor_XX_YY` node's Y translation to a single flat value, -0.18 -- exactly matching `Collision_Floor`'s own center Y, so the visual floor now sits flush with the surface the physics/collision has always assumed. Each tile's individual X/Z position and mesh geometry (so the grid/panel seams are still visible) are untouched -- only the per-tile height jitter was removed. The original file was preserved as `assets/rooms/ruined_barracks.glb.bak` before writing, in case the change needs to be reverted. Re-parsed the rewritten file afterward to confirm it's still valid glTF (node/mesh counts and header length all check out) -- this is not a substitute for looking at it in-game, but rules out a corrupted asset before the user spends a `cargo run` cycle on it.

**Scope note:** every `Floor_XX_YY` node was flattened, not only the tiles "before the stairs" as literally requested -- the room's raised platform is a separate structural piece entirely (Barracks Platform Collision / its own visual mesh), so any floor tiles nominally extending underneath it are already hidden regardless of their height. Worth a follow-up if this scope guess turns out wrong once seen in-game.

**Verification status:** not compiled (no `cargo`/`rustc` in this environment). The Rust change was verified manually (brace/paren balance, `EnemyNavigationGrid`/`EnemyStepPlatformConfig` imported and used identically to `fallen_gate.rs`'s own confirmed-working pattern, `room_transform.transform_point` math re-derived by hand for the platform's world position). The GLB change was verified by re-parsing the rewritten file and confirming its structure and node count are intact.

------------------------------------------------------------------------

# 62. 2026-09-02 Ruined Barracks: melee pathing still broken after two fixes -- temporary diagnostic instrumentation added

**Status:** OPEN / UNRESOLVED -- deferred to a future session. See 62.5 for the corrected conclusion; 62.3's speed-mismatch read has been superseded.

## 62.1 Section 61.1 confirmed ineffective

The user retested after Section 61.1's platform navigation-grid registration and reported explicitly that it did not work, and clarified the melee enemies are not stuck at the platform or stairs at all -- they are failing to find a path to the player somewhere else entirely. This rules out Section 61.1's diagnosis as the (or at least the only) cause. A full console log from a complete Ruined Barracks encounter was provided: all six enemies activate together, two early `Enemy: ATTACK` / `Player hit for 10.0` transitions occur near the start, and then there are zero further melee state-machine transitions (no further `ATTACK`, `RECOVERY`, or `CHASE` log lines) for the remainder of a multi-minute log, while the two Gatebow Marksmen continue functioning normally and eventually the player kills every enemy with ranged/AoE damage alone. This matches "melee enemies permanently stall in Chase" rather than an intermittent or platform-specific failure.

## 62.2 Decision: stop guessing, gather evidence

Two consecutive attempted fixes (60.2's navigation-lag reordering, 61.1's platform registration) did not resolve the underlying problem, and the project's own standing navigation rule is explicit: "Diagnose whether movement is using Landmass or fallback behavior before changing steering." Rather than attempt a third blind change, temporary diagnostic logging was added at the exact Chase-state reachability decision in `src/enemies/melee.rs::enemy_combat`, mirroring the project's own established pattern of temporary verbose navigation logging used and later removed during the original edge-navigation tuning work.

**What was added:** a `Local<f32>` throttle timer (logs at most once per second per melee enemy, to keep output readable over a multi-minute encounter). Inside the `Chase` branch, the previously single chained boolean (`enemy_grounded_path_is_safe(..) && (navigation.is_on_traversal_surface() || navigation_grid.segment_avoids_permanent_obstacles(..)) && enemy_world_path_is_clear(..)`) was split into three named booleans -- `grounded_safe`, `clear_of_obstacles`, `world_clear` -- combined into `player_is_reachable` exactly as before (behavior is unchanged; this is a pure refactor for observability). When the throttle fires, a `[nav-diag]` line prints the entity id, distance to player, all three booleans plus the combined `reachable` result, the Landmass `AgentState` (`ON_NAVMESH` / `NOT_ON_NAVMESH` / `NO_AGENT_STATE`), whether the entity currently has a `NavmeshRouteCache`, and the raw Landmass `AgentDesiredVelocity3d` if present.

**Why these specific signals:** they cover every plausible failure point identified while re-reading `navigation_mesh.rs` and `enemies/navigation.rs` this pass -- a permanently-false `grounded_safe`/`world_clear` would point at a raycast/collision issue (possibly Section 60.1's widened overlapping threshold collider confusing a raycast or the Rerecast navmesh bake); a permanently-false `clear_of_obstacles` would point at the navigation grid's obstacle/traversal bookkeeping; and `NOT_ON_NAVMESH` would point at Landmass never having agreed the enemy (or the room-connection seam itself) is on walkable navmesh in the first place -- which would also explain why this was never caught before, since Section 59 was the first time any enemy pathing had to cross a room-to-room seam rather than stay inside one room's own baked navmesh.

**Not yet done:** no hypothesis has been acted on. Section 61.1's platform registration has not been reverted (it may still be correct/needed even if it wasn't the cause of this specific bug) and Section 60.1's threshold-collider overlap has not been touched. Both remain candidate causes pending the diagnostic log.

**Verification status:** not compiled (no `cargo`/`rustc` in this environment). Verified manually: brace/paren balance on the edited file (45/45 braces, 52/52 parens). The diagnostic print is additive and gated behind the throttle; the reachability logic it observes was refactored into named variables but is arithmetically identical to the prior single expression.

**Next step:** user to run `cargo fmt && cargo check && cargo run`, reproduce a melee enemy stalling in Chase, and paste back the resulting `[nav-diag]` log lines so the actual failure point can be identified before any further fix is attempted.

## 62.3 Diagnostic result (superseded -- see 62.5): initially read as a speed/balance issue, not a pathfinding bug

A full encounter log with `[nav-diag]` output was provided. At the time, reading it looked like it closed this out -- **this conclusion was later corrected, see 62.5. It is kept here unedited as a record of the reasoning that turned out to be wrong, not as the current diagnosis.**

- `navmesh=ON_NAVMESH` on every single line, for every melee enemy, for the entire encounter. Landmass never lost track of any enemy.
- `grounded_safe=true` and `clear_of_obstacles=true` on every single line, with no exceptions. Ground safety and the obstacle grid were never the problem.
- `landmass_velocity` is non-zero and pointed in sensible, varying directions (e.g. `Vec3(-2.28, 0.0, -1.95)`, `Vec3(2.17, -0.0, -2.07)`) throughout -- Landmass is actively commanding real movement toward the player the whole time. Enemies are not idle, not off-mesh, and not blocked by the obstacle grid.
- `world_clear` is the only condition that ever reads `false`, and it flips true/false frequently even at a stable distance. This is expected and correct, not a bug: `enemy_world_path_is_clear` (`enemies/navigation.rs:1047`) does a straight-line shape-cast from the enemy directly to the player's exact position, over the *entire* remaining distance, against `GameLayer::World` only. Its only job is to gate the instant "skip the last approach and swing now" shortcut in the Chase branch, not to gate movement -- movement always goes through Landmass/`safe_enemy_locomotion` regardless of `world_clear`. A `false` here every so often, at distances of several units, next to pillars/platform/stairs geometry, is exactly what this check is supposed to report and has no effect on whether the enemy is approaching.
- Distance-to-player climbs down steadily from ~19-13 units to a plateau around 4-6 units for every melee enemy, and stays there for the rest of the log with real, ongoing navmesh velocity -- it never reads "stuck at a fixed point," it reads "still closing, but never closing the last few meters."

**The actual cause:** confirmed via `enemies/mod.rs:442` and `player/mod.rs:14` -- the Ruinbound Footman's `movement_speed` is **3.0**, and `PLAYER_SPEED` is **6.0**. The player (playing the bow-equipped Runesage in this test) moves at exactly twice the Footman's speed. A player who keeps moving -- which is exactly what a bow-user kiting backward while firing does -- can never be run down by a Footman at half their speed; the enemy will always plateau a few units out, matching the log precisely. The two `Enemy: ATTACK` lines in the whole encounter both fired at the moments the player's distance briefly dropped to 3.4-3.5 units (i.e. the player stopped moving or got cornered for a moment), which is also consistent with this explanation rather than an intermittent pathing failure.

**What this means for Sections 60-61:** neither the Section 60.1 threshold-collider overlap nor the Section 61.1 platform navigation-grid registration caused this, and neither needs to be reverted -- the diagnostic data shows both are working as intended (`ON_NAVMESH`, `grounded_safe`, `clear_of_obstacles` all clean throughout, including while enemies were on/near the platform area). Section 61.1's caveat about its exact numbers being untuned still stands and is worth confirming separately once melee can actually reach the player, but it is not implicated in this bug.

**This is a design/balance question, not a bug fix, and is left for the user to decide before any code changes:** possible directions include raising Footman `movement_speed` (closing the gap with `PLAYER_SPEED`, at the cost of making melee harder to outrun generally, not just while kiting), giving melee a situational speed boost while actively chasing an engaged, moving target, adding a gap-closing ability/lunge, or treating this as intended counterplay (a slower melee enemy that rewards a kiting playstyle, consistent with the project's own combat principle that "positioning... should matter") and instead tuning around it (e.g. via encounter design, enemy count, or ranged-enemy pressure to punish pure kiting). No change has been made pending the user's decision.

## 62.4 Cleanup

The temporary `[nav-diag]` logging and its `Local<f32>` throttle timer were removed from `src/enemies/melee.rs` now that they've served their purpose (matching this project's own established practice of removing temporary verbose navigation logging once a diagnosis is confirmed). The Chase-branch reachability check keeps the three named booleans (`grounded_safe` / `clear_of_obstacles` / `world_clear`) introduced for the diagnostic instead of the original single chained expression -- purely a readability improvement, arithmetically identical, and left in place rather than reverted a second time.

**Verification status:** not compiled (no `cargo`/`rustc` in this environment). Verified manually: brace/paren balance on the reverted file (31/31 braces, 42/42 parens), and confirmed `desired_velocity`, `agent_state`, and `route_cache` are still consumed by the unchanged `navmesh_pursuit_step` call later in the same branch, so removing the diagnostic block did not orphan any query field into an unused-variable warning.

## 62.5 Correction: this is a pathing problem, not a speed/balance problem

The 62.3 conclusion above (Footman `movement_speed` 3.0 vs `PLAYER_SPEED`
6.0 explaining the plateau) has been corrected. The melee-pathing issue
is confirmed **still unresolved** -- it was not a speed/balance tuning
question, it is an actual pathing problem. The 62.3 reasoning is kept in
this document as a record of what was tried and why it looked plausible
at the time (every `[nav-diag]` flag read clean -- `ON_NAVMESH`,
`grounded_safe`, `clear_of_obstacles`, real non-zero Landmass velocity --
so the plateauing distance was attributed to the speed differential
rather than to the path itself), but it is not the accepted diagnosis
going forward.

No further fix has been attempted yet. This is being **intentionally
deferred to a later session** rather than guessed at a fourth time. The
temporary `[nav-diag]` logging described in 62.2 has already been
removed from `src/enemies/melee.rs` (its job was done regardless of
which conclusion turned out to be right), and neither Section 60.1's
threshold-collider fix nor Section 61.1's platform navigation-grid
registration has been reverted -- both remain in place, unconfirmed as
either helpful or harmful to the actual pathing problem.

**When this is picked up again:** the existing `[nav-diag]` flags
(`grounded_safe` / `clear_of_obstacles` / `world_clear` /
`navmesh=ON_NAVMESH` / raw Landmass velocity) were all clean and did not
reveal the failure, so the next diagnostic pass should look past them --
at the actual path/waypoints Landmass is generating and following
(not just its instantaneous desired velocity), whether repathing is
oscillating or thrashing near the room-connection seam introduced in
Section 59, and whether the Rerecast navmesh bake itself is well-formed
across that seam now that two rooms' geometry meet there for the first
time. Do not re-assume a speed/balance explanation without new evidence.

------------------------------------------------------------------------

# 63. 2026-09-03 Ruined Barracks: generalized melee navigation fix confirmed

**Status: IMPLEMENTED / CONFIRMED IN PLAYTEST**

The remaining melee-enemy navigation problem was solved in the shared
navigation system. The fix is not specific to Ruined Barracks and applies to
future rooms and enemy archetypes that use `safe_enemy_locomotion`.

## 63.1 Final diagnosis

The logs showed no elevation failure: `ground_clear=true`, with matching
`current_ground` and `next_ground` values near `0.93`. The repeated blockers
were the room's supporting floor/platform colliders. The enemy-sized
horizontal sweep used a sphere centered directly at the locomotion reference
position, so the sphere began partially inside the supporting collider. The
floor was consequently treated as a wall, causing repeated switching between
direct movement, A*, and bypass recovery.

## 63.2 Shared navigation changes

`src/enemies/navigation.rs` was updated to:

- exclude the pursuit target from target-aware world sweeps;
- use `enemy_step_is_safe_ignoring` for pursuit bypass and waypoint movement;
- lift the horizontal sweep sphere by `ENEMY_NAVIGATION_RADIUS`, placing its
  bottom at the locomotion reference plane instead of inside the floor;
- continue detecting real solid geometry such as walls, statues, rubble,
  gates, and pillars.

These changes use shared collider and locomotion behavior rather than room
names, tile coordinates, or Ruined Barracks special cases. The platform
registration from Section 61.1 remains in place as the reusable mechanism for
future raised platforms and traversal routes.

## 63.3 Verification

The user tested the resulting v17 package in Ruined Barracks. The confirmation
log showed no `[nav-reject]` messages, stable `Direct`/`Landmass` decisions,
no unnecessary ground-enemy `GridAStar`/`Bypass` oscillation, and normal
movement for both ground-level melee enemies and raised ranged enemies. Only
unrelated Rust warnings about unused prototype code remained.

## 63.4 Diagnostics retained but disabled

Navigation diagnostics are retained for future debugging but are disabled by
default through this constant near the top of `src/enemies/navigation.rs`:

```rust
const NAVIGATION_DIAGNOSTICS_ENABLED: bool = false;
```

Temporarily set it to `true` when investigating a future navigation issue,
capture the `[nav-diag]` or `[nav-reject]` lines, then set it back to `false`
for normal development and play.

## 63.5 Files and handoff

The work is contained primarily in `src/enemies/navigation.rs`, with the
target-aware call-site updates in `melee.rs`, `hound.rs`, `ranged.rs`,
`saint.rs`, and `enemies/mod.rs`. `NAVIGATION_DEBUG_README.md` documents the
diagnostic setting. The latest handoff package is
`ruinbound_navigation_fix_2026-09-04_v18.zip`; it contains the updated `src/`
tree and README. Keep the existing `Cargo.toml`, assets, and room GLBs.

**Next session:** continue with the next gameplay or room task. If navigation
breaks again, enable the shared diagnostics first and determine whether the
failure is a real collider, unsupported ground, traversal authorization, or
route-authority change before changing enemy movement values.

------------------------------------------------------------------------

# 64. 2026-09-04 Development Update --- Wildshaper Class Implemented (Rhino Charge, Bear Slam, Panther Pounce)

**Status:** IMPLEMENTED / NOT YET FORMALLY CONFIRMED VIA PASTED `cargo check`/`cargo run` OUTPUT IN THIS LOG

The Wildshaper class described in Section 22 has a full first implementation: the class scaffold and all three approved abilities. It follows the exact same structural pattern as Warden of Shades (Section 51) and Runesage (Section 54) -- one `*Ability` component per ability, an input system per key press, a companion update system for anything that plays out over time, and a `sync_wildshaper_class_state` system writing into the shared `ClassAbilityHud`. Nothing about Vanguard, Warden, or Runesage's own code was touched to build this. As with those two classes, this session's code could not be compiled in the development sandbox (see Section 64.6), so per this project's own rule this section is intentionally **not** marked CONFIRMED.

## 64.1 Wildshaper class scaffold

**Status:** IMPLEMENTED

`src/characters/wildshaper.rs` (new file) mirrors `characters::vanguard`/`characters::warden`/`characters::runesage`'s existing pattern exactly:

```rust
pub struct Wildshaper; // class marker component
pub struct WildshaperVisual; // visual-child marker

pub fn attach_wildshaper_visual(...) { /* spawns the shared xbot_animated.glb placeholder visual, same as every other class -- Wildshaper has no dedicated model or animal-transformation visuals yet */ }
```

Wired into `CharactersPlugin` (`characters/mod.rs`) alongside the other three classes' systems; `attach_wildshaper_visual` was added into the same `.chain()`ed Update tuple, and `WildshaperAbilitiesPlugin` was added alongside `RunesageAbilitiesPlugin`/`VanguardAbilitiesPlugin`/`WardenAbilitiesPlugin`.

## 64.2 Ability 1 --- Rhino Charge (`Q`)

**Status:** IMPLEMENTED

Current tuning (`characters/wildshaper_abilities.rs`):

-   cooldown: `8.0` seconds
-   charge speed: `16.0` units/second, duration `0.55` seconds (~8.8 units of travel)
-   hit radius: `1.5` units, base damage `24.0` (scaled by Ability Power)
-   knockback: `2.2`, stagger strength `2` / duration `0.5` seconds
-   wall-crash burst radius: `3.0` units, half damage, stagger strength `3`

Structurally this reuses the exact shape Epic Longsword's Blade Rush already established (`weapons::abilities::longsword::BladeRush`): a marker component snapshotting the ability's stats plus a `hit_enemies` list drained as the player passes through enemies, driven by the shared `AbilityMovement` component every frame while active.

**Wall-crash mechanic (new, not previously specified):** each frame while charging, a single forward `SpatialQuery::cast_ray` (filtered to `GameLayer::World`, the same layer-masking approach `player::character_movement` already uses) checks for a wall roughly one frame's travel ahead. On a hit, the charge ends immediately, `AbilityMovement` is zeroed, and a radius burst deals half damage plus a strong stagger to anything standing near the impact point -- rewarding charging enemies into room geometry, matching the project's "room geometry should matter" pillar (Section 2.1) and Rhino Charge's stated "collision with a wall can create a crash/stun effect."

**"Smaller enemies can be launched" (approximated):** Ruinbound's enemies are kinematic and `enemies::enforce_enemy_ground_safety` immediately snaps one back to the ground the instant it leaves it -- there is no airborne-enemy state to put a launched enemy into. Rather than invent one unprompted (a much larger, cross-cutting change touching the shared navigation/ground-safety architecture the project context doc says to change cautiously), enemies at or below stagger resistance `1` (ordinary enemies, not elites/bosses, per the Section 12.1 stagger table) instead receive knockback multiplied `2.25x` -- a visibly stronger shove than anything else in the project, without touching enemy ground-safety code. This is flagged here as an open design gap, not a finished mechanic: if/when a real airborne-enemy state is built, Rhino Charge should be revisited to use it.

## 64.3 Ability 2 --- Bear Slam (`E`)

**Status:** IMPLEMENTED

Current tuning:

-   cooldown: `9.0` seconds
-   windup: `0.22` seconds (short, matching "fast attack/movement transformations, not long slow form-change sequences")
-   radius: `4.0` units, base damage `34.0` (scaled by Ability Power)
-   knockback: `2.6`, stagger strength `3` / duration `0.9` seconds (comfortably above elite resistance `~2`, close to boss resistance `~4` per Section 12.1's table -- Bear Slam is Wildshaper's dedicated interrupt/power tool)

During the windup, a pulsing ground-disk telegraph is spawned sized to the exact same `BEAR_SLAM_RADIUS` the damage check uses on impact (mirroring `enemies::saint::SaintSlamTelegraph`'s pulse animation and the Saint's own "visible leading edge and gameplay damage calculation must always use the same radius" rule, Section 22's boss-mechanic principle applied here to a player ability). Impact deals AoE damage/knockback/stagger to everything within radius, centered on the caster, and spawns a matching impact-burst visual.

Ruinbound has no formal player-ability-interruption system yet (nothing currently cancels an in-progress ability once started), so Section 22's "brief resistance to interruption during the transformation" is satisfied by construction rather than by a new hyperarmor flag -- documented in a code comment pointing at `BearSlamAbility::windup_remaining` as the gate to use if/when such a system is added later.

## 64.4 Ability 3 --- Panther Pounce (`R`)

**Status:** IMPLEMENTED

Current tuning:

-   cooldown: `7.0` seconds
-   pounce speed: `24.0` units/second, target-acquisition range `12.0` units
-   overshoot: `2.0` units past the target (the pounce lands beyond it, not on top of it)
-   hit radius: `1.3` units, base damage `30.0` (scaled by Ability Power)
-   knockback: `1.0` (light -- a precision strike, not a bulldozer), stagger strength `1` / duration `0.3` seconds

Target selection reuses the same "closest enemy within a forward aim cone" heuristic Vanguard's Lunge already established (`characters::vanguard_abilities::lunge_input`) -- a local equivalent was written for Wildshaper rather than importing Vanguard's private helper, keeping the two classes' code independent. Unlike Lunge, which stops short of its target, Panther Pounce's `AbilityMovement.time_remaining` is computed from `(distance_to_target + overshoot) / speed`, so the pounce carries the player through and past the target, matching Section 22's "can end behind or beyond the target." The same wall-ahead check Rhino Charge uses (Section 64.2) stops the pounce early at a wall instead of clipping through it.

## 64.5 Test-class swap

**Status:** IMPLEMENTED

`src/rooms/fallen_gate.rs` -- the game's active Startup zone since Section 38/47 -- now spawns `Wildshaper` in place of `Runesage` as the player's class marker, and the starter weapon was changed from `Weapon::bow(GearTier::Common)` to `Weapon::longsword(GearTier::Common)` so the active test class is actually testing a melee kit. `src/rooms/test_grounds.rs` (inactive since Section 47.2) still references `Runesage` and was deliberately left unchanged, since it isn't the active test zone and changing it wasn't requested.

## 64.6 Verification status and outstanding work

**Status:** OPEN

No `cargo fmt`/`cargo check`/`cargo run` could be run for this work: the user's machine (reached via the device bridge) has no Rust toolchain reachable from that shell, and the sandbox's own cloud container has no `Ruinbound` project on it at all. In place of compiling, a manual review was performed instead, the same standard applied in Sections 51.8/54.8/57.3: brace/paren/bracket balance across every touched/created file; every new function signature checked against its call sites; every system's multiple `Query<...>` parameters checked for provable disjointness under Bevy's runtime rules (`With<Player>` vs `(With<Enemy>, Without<Player>)` throughout, matching `weapons::abilities::longsword::update_blade_rush`'s existing pattern); and a byte-for-byte checksum comparison between the files written in this session and the files actually present on the user's machine afterward, to rule out any transfer corruption.

Per this project's own rule (IMPLEMENTED status must reflect actual working code, not general impressions), the next required steps before this section can be marked CONFIRMED are:

1.  `cargo fmt && cargo check && cargo run` on the user's machine, full output.
2.  Runtime-test Rhino Charge: verify the charge travels and damages/knocks back enemies in its path, the wall-crash burst triggers and staggers nearby enemies when charging into geometry, and ordinary enemies visibly get shoved further than an elite/boss would.
3.  Runtime-test Bear Slam: verify the ground telegraph appears and matches the actual impact radius, and that the impact damages/staggers everything caught in it.
4.  Runtime-test Panther Pounce: verify the pounce selects the intended target, passes through it, and lands beyond it rather than stopping short or clipping through walls.
5.  Once confirmed, update this section's status tag from IMPLEMENTED to IMPLEMENTED / CONFIRMED, matching the convention used in Section 46.6.

------------------------------------------------------------------------

# 65. 2026-09-04 Decision Log Additions (Wildshaper)

## 2026-09-04 --- Wildshaper class implemented

**Status:** IMPLEMENTED / NOT YET CONFIRMED VIA `cargo check`/`cargo run`

**Decision:**
Implement the Wildshaper class (Section 22) in full: class scaffold, and all three abilities -- Rhino Charge (collision-aware bulldozing charge with a wall-crash finisher), Bear Slam (telegraphed AoE ground slam), and Panther Pounce (dash-through-and-past-target leap). All three read `Equipment::ability_power_multiplier()` and route damage/stagger through `combat::apply_enemy_damage`/`combat::effective_stagger_resistance`. The active test scene (Fallen Gate) now spawns Wildshaper with a Longsword instead of Runesage with a Bow.

**Reason:**
Requested class build-out, following the approved Section 22 kit.

**Implementation impact:**
`characters/wildshaper.rs` (new), `characters/wildshaper_abilities.rs` (new), `characters/mod.rs`, `rooms/fallen_gate.rs`.

**Supersedes:**
None design-wise -- Section 22's approved kit is unchanged.

**Notes:**
Not yet confirmed via a pasted `cargo check`/`cargo run` transcript in this log -- see Section 64.6. All numeric tuning values are placeholders pending playtesting, the same convention used for every other class and the Saint's Fist Slam.

## 2026-09-04 --- Rhino Charge "launch" approximated via bonus knockback, not a new airborne-enemy state

**Status:** IMPLEMENTED / OPEN DESIGN GAP

**Decision:**
Rather than build a new airborne/launched enemy state to satisfy Rhino Charge's "smaller enemies can be launched" wording, ordinary enemies (stagger resistance at or below `1`) instead receive knockback multiplied `2.25x` on hit -- a much stronger shove than any other knockback in the project, but still ground-based.

**Reason:**
Ruinbound's enemies are kinematic and `enemies::enforce_enemy_ground_safety` immediately snaps one back to the ground the instant it leaves it (by design -- this is the same system that keeps enemies from falling through fractured geometry and off unsupported voids, per the project's confirmed navigation requirements). Adding real airborne enemy state is a cross-cutting change to shared navigation/ground-safety code, which this project's own rules say to change cautiously and not for one ability. Approximating with stronger knockback keeps Rhino Charge's identity intact without risking that architecture.

**Implementation impact:**
`characters/wildshaper_abilities.rs` only (`RHINO_CHARGE_LAUNCH_RESISTANCE_THRESHOLD`, `RHINO_CHARGE_LAUNCH_KNOCKBACK_MULTIPLIER`).

**Supersedes:**
None -- this is a documented gap against Section 22's original wording, not a change to it. If a real launch/air state is ever built for another reason, Rhino Charge should be revisited to use it instead of this approximation.

------------------------------------------------------------------------

# 66. Current Handoff (as of 2026-09-04, updated)

This supersedes Section 56 only by addition -- none of Section 50's outstanding items (gear archetypes, Test Grounds, barrier system) are resolved by this session's work, and none of them were touched by it. Sections 57-63's Ruined Barracks/navigation work is also unaffected.

1.  Everything listed in Section 50 (items 1-4) is still outstanding and unchanged.
2.  `cargo fmt && cargo check && cargo run` -- full output needed for the Wildshaper work in Sections 64-65, in addition to the still-unconfirmed work from Sections 46-63.
3.  Runtime-test Wildshaper per the checklist in Section 64.6.
4.  Once confirmed, update Section 64's status tag and this handoff section accordingly.
5.  Open design gap to revisit eventually (not blocking): Rhino Charge's "launch" is currently just strong knockback, not real airborne enemy state -- see Section 65's second decision entry.

No CONFIRMED design decisions from Sections 1-45 were changed by this session's work. Section 22's approved Wildshaper kit is unchanged in direction; only its implementation is new.

------------------------------------------------------------------------

# 67. 2026-09-05 Development Update --- Hammer Weapon Implemented (Stonefall, Faultline, Earthshatter)

**Status:** IMPLEMENTED / COMPILES AND RUNS ON THE USER'S MACHINE; INDIVIDUAL ABILITY PLAYTESTING NOT YET CONFIRMED

The Hammer weapon family described in Section 14.1/16.4 has its first three tiers implemented, using a supplied `WEAPONS.md` design document as the source of truth. It follows the same Common/Rare/Epic depth already established for Longsword, Greatsword, and Bow (Section 16), reusing the existing tier-dispatch architecture (`weapons::abilities::weapon_ability_input`'s `match (weapon.weapon_type, weapon.tier)`) rather than adding a parallel system.

## 67.1 Shared architecture: one AOE helper for the whole family

**Status:** IMPLEMENTED

New file `src/weapons/abilities/hammer.rs`. Every Hammer ground-impact ability -- Ground Slam, every Earthshatter pulse, and the Unstable Ground collapse -- routes through one shared function rather than each tier reimplementing its own damage/stagger/knockback loop:

```rust
fn apply_hammer_ground_impact(
    center: Vec3,
    radius: f32,
    damage: f32,
    stagger_strength: u32,
    stagger_duration: f32,
    knockback: f32,
    enemy_query: &mut Query<(&mut Transform, &mut Health, &mut Stagger, &mut EnemyCombat, Option<&EnemyDebuffs>), (With<Enemy>, Without<Player>)>,
)
```

It applies, per enemy in radius: damage via `combat::apply_enemy_damage` (Section 54.1, so Runebroken and any future enemy-damage debuff apply automatically), a stagger/interrupt check via `combat::effective_stagger_resistance` (resetting `EnemyCombat.state` to `Chase` on success -- the same interrupt mechanism every other weapon in the project already uses, Section 12.1), then knockback. This is also what WEAPONS.md's "briefly interrupts nearby enemy attacks" means in practice.

**"Normal enemies can be launched" / "knocked into the air"** (Seismic Strike, Earthshatter): Ruinbound's enemies are kinematic and have no airborne state to target (the same gap already disclosed for Wildshaper's Rhino Charge, Section 64.2/65). Rather than invent a second approximation, this reuses the exact one already on file: enemies at or below stagger resistance `1` (ordinary enemies, comfortably under elite resistance `~2` per Section 12.1's table) receive a bonus knockback multiplier (`HAMMER_LAUNCH_KNOCKBACK_MULTIPLIER = 1.8`) instead of a real launch. Elites/bosses still take the full stagger check but don't get the bonus shove, matching WEAPONS.md's "elites resist the knockback but still receive heavy stagger."

A shared ground-impact burst visual (`HammerImpactVisual`, a timed despawn component, `HAMMER_IMPACT_VISUAL_LIFETIME = 0.3` seconds) and a shared vertical ground-offset constant (`HAMMER_GROUND_VISUAL_Y_OFFSET`, matching the same player-capsule-to-floor offset Wildshaper's Bear Slam telegraph already established, Section 64.3) are reused by every tier's ground visuals.

## 67.2 Common Hammer -- Stonefall (Ground Slam)

**Status:** IMPLEMENTED

`Weapon::hammer(GearTier::Common)` base stats (`weapons/components.rs`): damage `40.0`, range `2.9`, cooldown `0.95`, knockback `1.7`, stagger strength `3`, stagger duration `0.55`, attack angle `0.35`. Hammer's basic attack (`hammer_attack` in `melee.rs`, mirroring `greatsword_attack`'s structure) uses a `0.42`-second windup (`HAMMER_WINDUP`).

Ground Slam (`hammer_ground_slam`, called from `melee.rs::update_player_combat` the same way Greatsword's `charged_cleave` already is) resolves after a `0.5`-second windup (`HAMMER_GROUND_SLAM_WINDUP`) with a `5.5`-second cooldown (`HAMMER_GROUND_SLAM_COOLDOWN`):

- radius: `4.4` units (`GROUND_SLAM_RADIUS`)
- damage: weapon damage x `1.15`
- stagger: weapon stagger strength + `2` (the roster's highest ordinary weapon stagger, per WEAPONS.md)
- knockback: weapon knockback x `1.4`

During the windup, `GroundSlamTelegraph` spawns a pulsing warning disk sized to the exact same `GROUND_SLAM_RADIUS` the damage check uses on impact -- the same "visible leading edge and gameplay hitbox must always agree" rule already established by the Saint's Fist Slam (Section 45.4) and Wildshaper's Bear Slam (Section 64.3).

## 67.3 Rare Hammer -- Faultline (Seismic Strike)

**Status:** IMPLEMENTED

Seismic Strike (`hammer_seismic_strike`, `3.2`-second cooldown) is a forward-traveling ground fissure, implemented as a wide piercing `Projectile` (new `ProjectileKind::SeismicFissure` variant) running through the exact same shared movement/collision pipeline every other wide-line ability already uses (`weapons::projectiles::move_projectiles_with_collision`; precedent: Phantom Cleave, Shadow Stag, Runic Surge) -- which already stops any piercing projectile the instant it hits world geometry, directly satisfying WEAPONS.md's "stops when it reaches solid dungeon geometry" with no new collision code.

- travel speed: `15.0` units/second, lifetime `0.75` seconds (~11.25 units of travel -- comfortably past Ground Slam's 4.4-unit radius, matching "longer range than Ground Slam")
- damage: weapon damage x `0.95`
- knockback: weapon knockback x `1.7`
- stagger: weapon stagger strength + `2` ("maximum stagger near the impact point")
- hitbox/visual size: `weapons::SEISMIC_FISSURE_SIZE = (1.6, 0.4, 2.2)`, defined once in `weapons/mod.rs` and read by both the spawned mesh and the collision system's collider selection -- the same "visual and hitbox cannot drift apart" pattern already used for `SHADOW_STAG_SIZE` (Section 51.5) and `RUNIC_SURGE_SIZE` (Section 54.5)
- piercing: every enemy along the fissure's path is hit once, via the shared system's existing `hit_entities` tracking

## 67.4 Epic Hammer -- Earthshatter

**Status:** IMPLEMENTED (unstable-ground slow explicitly deferred)

Earthshatter (`hammer_earthshatter`, `6.5`-second cooldown) is an instant multi-pulse impact, matching the "instant on cast" shape every other Epic weapon ability already uses (Blade Rush, Phantom Cleave, Ricochet Arrow):

- an `EarthshatterSequence` entity drives 3 pulses on a `0.35`-second interval (`EARTHSHATTER_AFTERSHOCK_INTERVAL`), the first firing on the very next tick: impact radius `6.0` (damage x`1.35`), then two aftershock pulses at radius `4.5` (damage x`0.55` each)
- knockback: weapon knockback x `1.6`
- stagger: weapon stagger strength + `3` -- exactly matching the Saint of the Broken Siege's own stagger resistance (`6`, Section 45.3), so even Earthshatter cannot interrupt a boss outright ("cannot be permanently stun-locked"), while still landing the full stagger check against everything else

A separate `HammerUnstableGround` entity is spawned at the same moment: a `2.0`-second lingering hazard (`UNSTABLE_GROUND_DURATION`) at radius `4.5` that visually pulses, then collapses in a second damaging burst -- damage weapon damage x`0.7`, stagger weapon stagger strength + `1` -- directly implementing WEAPONS.md's "the unstable ground collapses in a second damaging burst."

**Deferred, disclosed honestly:** WEAPONS.md also specifies the unstable ground "briefly slows enemies." `combat::EnemyDebuffs` (Section 54.1) currently only supports a damage-taken multiplier and a stagger-resistance reduction -- there is no movement-speed-slow field on any enemy-debuff component in the project yet. Rather than add one unprompted mid-feature, the module's own doc comment flags this explicitly (`HammerUnstableGround`'s doc comment: "Purely a damage/visual timer today -- see the module doc comment above about the deferred slow effect"), and the collapse-burst half of the design (confirmed to be implementable with existing systems) was built in full. Adding a slow field to `EnemyDebuffs` is a natural, small follow-up once requested.

## 67.5 Compile error fix -- `HammerImpactVisual` private-type violation

**Status:** RESOLVED

The user ran `cargo fmt && cargo check` on the first pass and reported one error:

```
error: type `HammerImpactVisual` is private
   --> src/weapons/mod.rs:114:9
```

`weapons/mod.rs::WeaponPlugin::build` registers `abilities::update_hammer_impact_visuals` (along with `update_ground_slam_telegraphs`, `update_earthshatter_aftershocks`, and `update_unstable_ground`) in one `app.add_systems` block. `update_hammer_impact_visuals` takes `Query<(Entity, &mut HammerImpactVisual)>` as a parameter, but `HammerImpactVisual` was declared as a plain private `struct` in `hammer.rs` -- narrower visibility than the `pub(crate) fn` that exposes it in a signature referenced from a different module, which Rust's private-in-public rule rejects.

**Fix:** changed `struct HammerImpactVisual` to `pub(crate) struct HammerImpactVisual` in `src/weapons/abilities/hammer.rs` (one line). Before applying the fix, all three sibling types used the same way in that same `app.add_systems` block (`GroundSlamTelegraph`, `EarthshatterSequence`, `HammerUnstableGround`) were checked and confirmed already correctly `pub(crate)` -- this was the only instance of the bug, not a symptom of a broader pattern. Verified with `rustfmt --edition 2024 --check` (still no `cargo` toolchain reachable in the development sandbox). The user then ran `cargo fmt && cargo check && cargo run` and confirmed a clean build.

## 67.6 Loot, UI, and class-ability integration

**Status:** IMPLEMENTED (Epic Hammer not yet loot-obtainable -- disclosed below)

Adding Hammer required a new arm in every exhaustive `match` on `WeaponType` across the codebase, matching this project's established "no weapon family should require modifying unrelated systems beyond adding its own match arm" pattern:

- `weapons/components.rs` -- `WeaponType::Hammer` variant, `Weapon::hammer(tier)` constructor, `ProjectileKind::SeismicFissure` variant, `weapon_definition_id`/`weapon_display_name` arms (display names: Stonefall / Faultline / Earthshatter / "Legendary Hammer")
- `weapons/projectiles/movement.rs` -- collider shape/rotation selection for `ProjectileKind::SeismicFissure`
- `ui/mod.rs` -- ability-name HUD text ("GROUND SLAM" / "SEISMIC STRIKE" / "EARTHSHATTER") and the equipped-weapon-name HUD label ("Hammer")
- `weapons/loot.rs` -- world-pickup label ("Hammer")
- `characters/vanguard_abilities.rs` -- Afterimage's per-weapon clone-visual match now has a Hammer arm (orange-tinted, matching the "Hammer clone can send spectral hammer-shaped shockwaves" design note already on file in Section 21/17)
- `characters/warden_abilities.rs` -- Predator's Spirit's per-weapon proc-visual match now has a Hammer arm, with a dedicated `PREDATORS_SPIRIT_HAMMER_FRACTION_MULTIPLIER = 1.6` bonus (a larger, slower spectral proc matching a heavy weapon, the same design intent Section 19 already states for heavy weapons generally)
- `loot/catalog.rs` -- `COMMON_HAMMER`/`RARE_HAMMER` definition IDs and their `create_item` arms
- `loot/tables.rs` -- Common and Rare Hammer wired into the Normal Enemy, Elite, Miniboss, Boss, Common Chest, Rare Chest, and Boss Chest tables, using the existing floor-gating mechanism (Section 46.5), same convention as every other weapon's loot rollout

**Disclosed gap:** only Common and Rare Hammer are reachable through loot -- there is no `EPIC_HAMMER` catalog entry or loot-table wiring yet. Epic Hammer (Earthshatter) is currently only obtainable via a direct `Weapon::hammer(GearTier::Epic)` call (used for the current test spawn is Common; Epic has not been placed in any loot table). This mirrors the disclosed gaps already on file for other systems (Section 46.4, 48.6) rather than silently claiming full loot coverage.

## 67.7 Test-scene spawn change

**Status:** IMPLEMENTED

`src/rooms/fallen_gate.rs::setup_fallen_gate_player_and_scene` -- the game's active Startup zone since Section 38/47/59 -- now spawns the player with `Weapon::hammer(GearTier::Common)` in place of `Weapon::longsword(GearTier::Common)`, so the active test scene actually plays the new weapon. This is a test-scene-only change, the same precedent already used for the Vanguard-to-Warden-to-RuneSage-to-Wildshaper class swaps (Sections 51.7, 54.7, 64.5) and Wildshaper's own weapon swap (Section 64.5) -- the underlying "any class can equip any weapon" rule (Section 13) is untouched, and `src/rooms/test_grounds.rs` (inactive since Section 47.2) was deliberately left unchanged.

## 67.8 Verification status and outstanding work

**Status:** COMPILES AND RUNS; ABILITY-LEVEL PLAYTESTING OPEN

Unlike every class-implementation session since Section 51 (Warden of Shades onward), this session's fix cycle *was* run against a real compiler -- the user's own `cargo fmt`/`cargo check`/`cargo run` on their machine, twice, both times reporting success (once for the compile fix in Section 67.5, once for the spawn change in Section 67.7). The development sandbox itself still has no working path to a full `cargo check` (no network access to crates.io from the sandbox's cloud container; no `cargo`/`rustc` at all from the device-bridge shell used to edit files on the user's machine directly) -- `rustfmt --edition 2024 --check` was used there as a syntax-only proxy, the same standing limitation documented in every prior session (Sections 51.8, 54.8, 57.3, 64.6).

Per this project's own rule that IMPLEMENTED status must reflect actual working code and not general impressions, the following is still open before Hammer can be called fully confirmed:

1. Runtime-test Ground Slam: verify the windup telegraph disk matches the actual impact radius, damage/stagger/knockback land correctly, and elites resist the knockback bonus while still taking heavy stagger.
2. Runtime-test Seismic Strike: verify the fissure travels the intended distance, pierces multiple enemies, and stops cleanly on world geometry.
3. Runtime-test Earthshatter: verify all three pulses land, the unstable ground visibly lingers for its full duration and then collapses in a second damaging burst, and that the "slow" half of the effect is not expected yet (deferred per Section 67.4).
4. Verify the ordinary-enemy bonus-knockback "launch" approximation reads as intended in play, not just in code review.
5. Once confirmed, update this section's status tag and Section 69's handoff accordingly.

------------------------------------------------------------------------

# 68. 2026-09-05 Decision Log Additions (Hammer)

## 2026-09-05 --- Hammer weapon family implemented (Common/Rare/Epic)

**Status:** IMPLEMENTED / COMPILES AND RUNS; ABILITY-LEVEL PLAYTESTING OPEN

**Decision:**
Implement the Hammer weapon family (Section 14.1/16.4) in full for its first three tiers: Common (Stonefall -- Ground Slam), Rare (Faultline -- Seismic Strike), and Epic (Earthshatter), from a supplied `WEAPONS.md` design document. All three share one AOE helper (`apply_hammer_ground_impact`) and route damage/stagger through the existing `combat::apply_enemy_damage`/`combat::effective_stagger_resistance` helpers. The active test scene (Fallen Gate) now spawns the player with a Common Hammer instead of a Common Longsword.

**Reason:**
Requested weapon build-out, following the supplied `WEAPONS.md` design document and this project's established weapon-tier architecture (Section 15/16).

**Implementation impact:**
`src/weapons/abilities/hammer.rs` (new), `weapons/components.rs` (`WeaponType::Hammer`, `Weapon::hammer`, `ProjectileKind::SeismicFissure`), `weapons/mod.rs` (Hammer constants, new `app.add_systems` block), `weapons/abilities/mod.rs` (tier dispatch, ability cooldown/windup constants), `weapons/melee.rs` (`hammer_attack`, basic-attack/ability dispatch arms), `weapons/projectiles/movement.rs` (`SeismicFissure` collider selection), `ui/mod.rs`, `weapons/loot.rs`, `characters/vanguard_abilities.rs`, `characters/warden_abilities.rs`, `loot/catalog.rs`, `loot/tables.rs`, `rooms/fallen_gate.rs` (test spawn).

**Supersedes:**
None design-wise -- Section 14.1/16.4's approved Hammer identity is unchanged; only its implementation is new.

**Notes:**
Compiles and runs, confirmed via the user's own `cargo fmt && cargo check && cargo run` (twice). Individual ability playtesting (Ground Slam, Seismic Strike, Earthshatter) is not yet confirmed -- see Section 67.8. Legendary Hammer (Worldbreaker, Graviton Maul, Stormanvil) is not itemized yet, matching every other weapon family's current Legendary gap. Epic Hammer has no loot-table entry yet (Common/Rare only) -- see Section 67.6.

## 2026-09-05 --- Earthshatter's "slows enemies" deferred pending an `EnemyDebuffs` slow field

**Status:** IMPLEMENTED / OPEN DESIGN GAP

**Decision:**
Earthshatter's unstable-ground hazard was implemented as a damage/stagger/visual timer that collapses in a second damaging burst, but does not slow enemies standing in it, even though `WEAPONS.md` specifies "leaves unstable ground that briefly slows enemies."

**Reason:**
`combat::EnemyDebuffs` (Section 54.1) currently only supports a damage-taken multiplier and a stagger-resistance reduction -- there is no movement-speed-slow field on any enemy-debuff component in the project. Adding one was judged out of scope for this pass rather than invented unprompted mid-feature; the module's own doc comment flags the gap explicitly so it isn't silently lost.

**Implementation impact:**
`src/weapons/abilities/hammer.rs` only, for the current (deferred) state. A future fix would touch `combat/enemy_debuffs.rs` (`EnemyDebuffs`, a new movement-speed-multiplier field) and wherever enemy movement speed is applied (`enemies/navigation.rs` / each archetype's locomotion).

**Supersedes:**
None -- this is a documented gap against `WEAPONS.md`'s Epic Hammer wording, not a change to it. Follows the same disclosure precedent as Wildshaper's Rhino Charge "launch" approximation (Section 65).

------------------------------------------------------------------------

# 69. Current Handoff (as of 2026-09-05, updated)

This supersedes Section 66 only by addition -- none of Section 50's outstanding items (gear archetypes, Test Grounds, barrier system) are resolved by this session's work, and none of them were touched by it. Sections 51-65's class work (Warden, RuneSage, Wildshaper) and Sections 57-63's Ruined Barracks/navigation work are also unaffected.

1.  Everything listed in Section 50 (items 1-4) is still outstanding and unchanged.
2.  Sections 51.8/54.8/64.6's class-verification checklists (Warden of Shades, RuneSage, Wildshaper) are still outstanding and unchanged -- this session did not touch any class code.
3.  Hammer (Sections 67-68) compiles and runs, confirmed twice via the user's own `cargo fmt && cargo check && cargo run` -- but its three abilities have not yet been individually runtime-tested. Work through Section 67.8's checklist (Ground Slam telegraph/impact, Seismic Strike fissure, Earthshatter pulses + unstable-ground collapse, and the ordinary-enemy bonus-knockback "launch" feel) before treating Hammer as fully confirmed.
4.  Once Hammer's abilities are confirmed, update Section 67's status tag and this handoff accordingly.
5.  Open design gaps to revisit eventually (not blocking): Earthshatter's unstable ground does not yet slow enemies, pending a movement-slow field on `combat::EnemyDebuffs` (Section 68's second decision entry); Epic Hammer has no loot-table entry yet (Section 67.6); Legendary Hammer is not itemized yet; Rhino Charge's "launch" is still just strong knockback, not real airborne enemy state (Section 65, unchanged by this session).

No CONFIRMED design decisions from Sections 1-45 were changed by this session's work. Section 14.1/16.4's approved Hammer identity is unchanged in direction; only its implementation is new.

------------------------------------------------------------------------

# 70. 2026-09-05 Development Update --- Ruin Corruption Resource & Tab Status Screen

**Status:** IMPLEMENTED (resource + passive growth + status screen only); COMPILES AND RUNS ON THE USER'S MACHINE AFTER ONE FIX; CORRUPTION GAMEPLAY (ENEMIES/ROOMS/REWARDS/EVENTS) NOT YET BUILT

A second session on 2026-09-05, separate from the Hammer work in Section 67. The user supplied a new, more detailed `RUIN_CORRUPTION_DESIGN.md` document -- a full numeric refinement of Section 9's original 25/50/75/100% concept, with an explicit 0-100 scale and six named threshold bands -- and asked for the Ruin resource itself to be added to the game, along with a Tab-key screen showing current Ruin and elapsed run time, built with future additions to that screen in mind, and mutually exclusive with the existing Map (`M`) and Inventory (`I`) screens.

## 70.1 `RuinState` resource (`src/ruin/mod.rs`, new file)

**Status:** IMPLEMENTED

New top-level module, registered as `ruin::RuinPlugin` in `main.rs`. `RuinState` is a resource holding:

-   `ruin: f32`, always clamped to `RUIN_MIN..=RUIN_MAX` (`0.0..=100.0`) by its only two mutators, `add_ruin`/`remove_ruin` -- every future Ruin-granting or Ruin-costing mechanic (cursed gear, shrines, harder branches, cleansing) is meant to go through these two methods rather than touching the field directly, so the clamp and the Ruinbreak trigger (below) stay in one place.
-   `run_elapsed_seconds: f32`, a real-time run clock (see Section 70.5 for how it's paused/not paused).
-   `RuinThreshold`, a six-variant enum (`Unmarked` 0-19, `Touched` 20-39, `Stained` 40-59, `Corrupted` 60-79, `Doomed` 80-99, `Ruinbreak` 100) matching `RUIN_CORRUPTION_DESIGN.md`'s threshold table exactly, with a `label()` and an `accent_color()` for UI use -- the same "descriptive enum, not gameplay logic itself" pattern already established by `rooms::components::RoomKind` (Section 6).

Passive growth (`advance_run_clock_and_ruin`, runs every frame) follows the design doc's time-scaling table: `+1 Ruin` every 90 seconds under 10 minutes of run time, every 75 seconds from 10-20 minutes, every 60 seconds after that. Each tick also arms a 3-second HUD warning ("The Ruin is spreading...") via `RuinState::warning_message()`, rendered by a new small banner in `ui/mod.rs` (`RuinWarningBanner`) spawned alongside the rest of the HUD in `spawn_hud`.

**Ruinbreak (100 Ruin):** per the design doc's "Ruin cannot directly kill the player," reaching 100 resets Ruin to 65 (`RUINBREAK_RESET_RUIN`) and logs an `info!` line rather than ending the run. The full Ruinbreak event described in the design doc (a Corrupted Elite invasion, a survival encounter, a pocket-realm detour, a temporary "Ruinbreak Scar") is explicitly not built -- `trigger_ruinbreak`'s doc comment flags this as a deferred follow-up, the same disclosure pattern used for Earthshatter's slow effect (Section 67.4/68).

## 70.2 Tab status screen (`src/ui/ruin_screen.rs`, new file)

**Status:** IMPLEMENTED

A new full-screen overlay, opened/closed with `Tab`, following the exact same structural pattern as the existing Map screen (`ui/map.rs`, `M` key) and Inventory screen (`weapons/inventory_ui.rs`, `I` key): a `RuinScreenState { open: bool }` resource, a toggle system that shows/hides the panel and releases/relocks the cursor, and a display-refresh system that only runs while open.

Built for the "we'll add to this screen later" requirement specifically: rows are driven by a small `RuinStatKind` enum (`Ruin`, `Threshold`, `RunTime`) plus one `spawn_stat_row` helper, rather than hand-laid-out fixed text. Adding a future row (local room corruption, active corruption sources, Ruinbreak history, etc. -- all described in `RUIN_CORRUPTION_DESIGN.md` but not yet built) is one enum variant, one `spawn_stat_row` call, and one match arm -- nothing else about the panel changes. Currently shows three rows: Ruin (`x / 100`), current threshold name, and run time as `mm:ss`.

## 70.3 Three-way mutual exclusion between Map, Inventory, and the Ruin screen

**Status:** IMPLEMENTED

Requested explicitly: opening Tab must close Map and Inventory, and opening either of those must close Tab. This extends the existing bilateral Map<->Inventory pattern (`ui::map::close_map_when_inventory_opens` / `weapons::inventory_ui::close_inventory_when_map_opens`) to a third screen with the same shape, rather than introducing a new shared "which overlay is open" abstraction -- one new function per screen:

-   `ui::map::close_map_when_ruin_opens`
-   `weapons::inventory_ui::close_inventory_when_ruin_opens`
-   `ui::ruin_screen::close_ruin_screen_when_other_ui_opens` (checks both `MapUiState` and `InventoryUiState` in one system, since the Ruin screen has two screens to yield to)

## 70.4 Compile error fix -- `RuinStatKind` private-type violation

**Status:** RESOLVED

The user ran `cargo fmt && cargo check` and reported:

```
error: type `RuinStatKind` is private
   --> src/ui/mod.rs:119:9
```

`ruin_screen::update_ruin_screen_display` (registered from `ui/mod.rs::UiPlugin::build`, a different module) takes `Query<(&RuinStatKind, ...)>`, but `RuinStatKind` had been declared with no visibility modifier at all -- private even to the rest of the crate, narrower than the `pub(crate) fn` that exposes it. Same class of bug as Section 67.5's `HammerImpactVisual` issue. **Fix:** changed `enum RuinStatKind` to `pub(crate) enum RuinStatKind` (one line) in `src/ui/ruin_screen.rs`. The user re-ran `cargo fmt && cargo check && cargo run` and reached runtime successfully (confirmed implicitly by then reporting on in-game Tab-screen behavior -- see Section 70.5).

## 70.5 Design change -- Ruin and the run clock do not pause for any menu

**Status:** IMPLEMENTED (reverses the first pass's behavior)

The first implementation paused `run_elapsed_seconds` and passive growth while *any* full-screen overlay (Map, Inventory, or the Ruin screen itself) was open, reading `RUIN_CORRUPTION_DESIGN.md`'s "the timer pauses in menus, shops, and safe rooms" literally -- Map/Inventory/Tab being the game's only current menus. The user reported this as "the time and ruin is not changing while the tab screen is open," which was clarified as intentional-but-questionable and resolved via one clarifying question:

1.  First answer: keep the Ruin/run clock ticking while the Tab screen specifically is open (it's a status readout, not an interactive menu), but still pause for Map/Inventory.
2.  The user then explicitly overrode that mid-implementation: **don't pause for Map or Inventory either.**

Final behavior: `advance_run_clock_and_ruin` no longer reads `MapUiState`/`InventoryUiState`/`RuinScreenState` at all -- it advances `run_elapsed_seconds` and passive growth unconditionally, every frame, regardless of which (if any) full-screen overlay is open. This is a deliberate, explicit deviation from `RUIN_CORRUPTION_DESIGN.md`'s literal "pauses in menus, shops, and safe rooms" wording for this project's current menus; the design doc's underlying intent (don't let a player stop the clock by parking in a genuine safe room or shop) still has no way to be tested since neither of those exists yet. Revisit if/when a real shop or safe-room mechanic is added.

**Not yet re-verified against a real build:** this change (and Section 70.4's fix) were applied directly to the user's project files, but the user has not yet reported a fresh `cargo fmt && cargo check && cargo run` since this specific change. Treat as implemented-but-unconfirmed until they do.

## 70.6 What this pass deliberately does not implement

**Status:** DISCLOSED GAP, MATCHES THE PROJECT'S "ONE MECHANIC AT A TIME" WORKFLOW

Per this project's own rule (do not proceed to the next mechanic until the current one is confirmed working, unless explicitly told otherwise), only the Ruin *resource*, its passive growth, and its status screen were built. Everything else in `RUIN_CORRUPTION_DESIGN.md` reads/writes the same `RuinState` through `add_ruin`/`remove_ruin` but is not built yet, and was not asked for in this pass:

-   Player-choice Ruin gain/loss (corrupted side rooms, cursed bargains, shrines, cleansing altars, reward-screen corrupted alternatives, etc.)
-   Corrupted enemy affixes (Bloodbound, Rift-Touched, Husked, Blighted, Frenzied, Mirrorbound, Gravefed, Void-Armored, Possessed)
-   Room-level corruption (local corruption level, layout/environmental/objective/reward mutation types, corruption sources)
-   The full Ruinbreak event at 100 Ruin (Section 70.1's reset-to-65 stub only)
-   Player-choice events (the Black Altar, the Fallen Knight, the Sealed Door)
-   "Entering a new room may add +1 Ruin" (the design doc marks this as optional/"may"; not wired up)

## 70.7 Verification status and outstanding work

**Status:** COMPILES AND RUNS AS OF SECTION 70.4'S FIX; THE SECTION 70.5 BEHAVIOR CHANGE IS UNCONFIRMED BY A FRESH BUILD; NO GAMEPLAY-LEVEL TESTING OF PASSIVE GROWTH TIMING YET

Same standing sandbox limitation as every prior session (Sections 51.8/54.8/57.3/64.6/67.8): no `cargo` toolchain reachable from either the cloud container or the device-bridge shell used to edit the user's files directly. All edits were verified by exact-pattern reuse against code already known to compile, plus programmatic brace/paren balance checks and byte-for-byte hash verification of freshly written files -- not a substitute for `cargo check`.

Outstanding before Ruin can be called fully confirmed:

1.  Re-run `cargo fmt && cargo check && cargo run` after Section 70.5's change (removing all menu-pause logic) and confirm a clean build.
2.  Confirm in play that Ruin and the run-time readout visibly count up while Tab, Map, and Inventory are all open.
3.  Confirm the passive-growth warning banner ("The Ruin is spreading...") appears and fades correctly, and that its timing roughly matches the 90/75/60-second table as run time crosses 10 and 20 minutes.
4.  Confirm Tab/Map/Inventory remain fully mutually exclusive in every direction (nine open/close combinations, not just the ones exercised so far).
5.  Once confirmed, update this section's status tag and Section 72's handoff accordingly.

------------------------------------------------------------------------

# 71. 2026-09-05 Decision Log Additions (Ruin Corruption)

## 2026-09-05 --- Ruin Corruption resource + Tab status screen implemented (first pass)

**Status:** IMPLEMENTED (resource, passive growth, screen only) / CORRUPTION GAMEPLAY NOT YET BUILT

**Decision:**
Implement `RuinState` (0-100 Ruin, a six-band `RuinThreshold`, passive time-based growth per `RUIN_CORRUPTION_DESIGN.md`'s scaling table, and a Ruinbreak-at-100 stub that resets to 65) plus a new Tab-key status screen showing current Ruin and run time, built around a small extensible stat-row list. Made mutually exclusive with the existing Map and Inventory screens in both directions.

**Reason:**
Requested build-out of the Ruin Corruption system from the supplied `RUIN_CORRUPTION_DESIGN.md` design document, scoped to the resource and its display per this project's "confirm one mechanic before moving to the next" workflow -- corrupted enemies, rooms, rewards, and choice events are explicitly follow-up work (Section 70.6).

**Implementation impact:**
`src/ruin/mod.rs` (new), `src/ui/ruin_screen.rs` (new), `src/ui/mod.rs` (HUD warning banner, plugin registration), `src/ui/map.rs` (`close_map_when_ruin_opens`), `src/weapons/inventory_ui.rs` (`close_inventory_when_ruin_opens`), `src/weapons/mod.rs` (system registration), `src/main.rs` (`mod ruin;`, `ruin::RuinPlugin`).

**Supersedes:**
Refines Section 9's original informal 25/50/75/100% Ruin-percentage concept with `RUIN_CORRUPTION_DESIGN.md`'s exact 0-100 point scale and six named thresholds (Unmarked/Touched/Stained/Corrupted/Doomed/Ruinbreak); does not change Section 9's core CONFIRMED direction.

**Notes:**
One compile error (`RuinStatKind` private-type violation, Section 70.4) was fixed. Not yet re-confirmed by the user's own `cargo check`/`cargo run` since Section 70.5's pause-behavior change.

## 2026-09-05 --- Ruin and the run clock never pause, in any menu

**Status:** IMPLEMENTED / EXPLICIT DEVIATION FROM `RUIN_CORRUPTION_DESIGN.md`'S LITERAL WORDING

**Decision:**
`advance_run_clock_and_ruin` advances `RuinState`'s run clock and passive growth unconditionally every frame. Opening Map, Inventory, or the Tab screen has no effect on it.

**Reason:**
The design doc says the timer "pauses in menus, shops, and safe rooms." The first pass implemented that literally for all three of the game's current full-screen menus. The user first asked to exempt just the Tab screen (a status readout, not an interactive menu), then explicitly asked to remove the pause for Map and Inventory too.

**Implementation impact:**
`src/ruin/mod.rs` only -- `advance_run_clock_and_ruin` no longer takes `MapUiState`/`InventoryUiState`/`RuinScreenState` as parameters at all. `src/main.rs`'s comment on `ruin::RuinPlugin`'s plugin-ordering rationale was corrected to match (no resource-ordering dependency on `UiPlugin`/`WeaponPlugin` anymore).

**Supersedes:**
The initial (same-session) "pause while any menu is open" behavior. Revisit only if a real shop or safe-room mechanic is added later and the design doc's original intent needs to be honored for those specifically.

------------------------------------------------------------------------

# 72. Current Handoff (as of 2026-09-05, updated)

This supersedes Section 69 only by addition -- none of Section 50's outstanding items, Sections 51.8/54.8/64.6's class-verification checklists, or Section 67.8's Hammer ability-playtesting checklist are resolved or touched by this session's work.

1.  Everything listed in Section 69 (items 1-5) is still outstanding and unchanged.
2.  Ruin Corruption (Sections 70-71) has its resource, passive growth, and Tab status screen implemented, with one compile-error fix already applied (Section 70.4) and confirmed by the user reaching runtime. The pause-behavior change in Section 70.5 (removing all menu-based pausing) has **not yet** been confirmed against a fresh `cargo fmt && cargo check && cargo run` -- do that first.
3.  No gameplay-level testing of Ruin has happened yet: passive-growth timing/warning banner, the Ruinbreak-at-100 reset, and full nine-way Tab/Map/Inventory mutual exclusion are all still open (Section 70.7's checklist).
4.  Everything in Section 70.6 (corrupted enemies, corrupted rooms, player-choice Ruin events, the full Ruinbreak event, cleansing mechanics) is deliberately not built yet and is the natural next Ruin-related work once the resource itself is confirmed.
5.  Once Ruin is confirmed, update Section 70's status tag and this handoff accordingly.

No CONFIRMED design decisions from Sections 1-69 were changed by this session's work, aside from Section 9's numeric refinement (still CONFIRMED in direction; only the exact numbers/thresholds are now sourced from `RUIN_CORRUPTION_DESIGN.md` instead of the original percentage sketch).

------------------------------------------------------------------------

# 73. 2026-09-05 Development Update --- Gravemind Class Implemented (Ruin Bolt, Consume, Embrace the Ruin)

**Status:** IMPLEMENTED / CONFIRMED -- the user ran `cargo fmt && cargo check && cargo run` on their own machine (no Rust toolchain was reachable from either this session's cloud container or the device-bridge shell used to edit the files directly, so the sandbox itself could not compile this), confirmed a clean build, and confirmed in-game that Gravemind spawns correctly, all three abilities fire, and full mechanic testing was performed. See 73.6 for what "confirmed" covers here.

The Gravemind class described in Section 23 has a full first implementation: the class scaffold and all three approved abilities (Ruin Bolt, Consume, Embrace the Ruin). It follows the exact same structural pattern as Warden of Shades (Section 51), Runesage (Section 54), and Wildshaper (Section 64) -- one `*Ability` component per ability, an input system per key press, a companion update system for anything that plays out over time, and a `sync_gravemind_class_state` system writing into the shared `ClassAbilityHud`/`ClassResourceHud`. Nothing about any other class's code was touched.

## 73.1 Gravemind class scaffold

**Status:** IMPLEMENTED / CONFIRMED

`src/characters/gravemind.rs` (new file) mirrors `characters::warden`/`characters::wildshaper`'s existing pattern exactly: a `Gravemind` marker component, a `GravemindVisual` marker, and `attach_gravemind_visual` spawning the same shared `xbot_animated.glb` placeholder visual every other class currently uses. Wired into `CharactersPlugin` (`characters/mod.rs`) alongside the other four classes' systems.

## 73.2 Ability 1 --- Ruin Bolt (`Q`)

**Status:** IMPLEMENTED / CONFIRMED

Current tuning (`characters/gravemind_abilities.rs`):

-   cooldown: `6.0` seconds
-   projectile speed: `24.0` units/second, range `24.0` units (lifetime derived as `range / speed`)
-   base damage: `32.0` (scaled by Ability Power)
-   a "blast" splash: `2.5`-unit radius, `40%` of the direct hit's damage to any other nearby enemy
-   knockback `1.2`, stagger strength `2` / duration `0.5` seconds

Per Section 23's note that "earlier health-cost versions were deemphasized," Ruin Bolt has no health cost of its own -- it is an ordinary cooldown-gated projectile, aimed with the same crosshair-raycast-to-point technique already used by Runesage's Rune Blast (`characters::runesage_abilities::rune_blast_input`) and `weapons::bow::fire_bow`. A new `ProjectileKind::RuinBolt` variant was added (`weapons/components.rs`) and a matching "blast" splash block was added to `weapons::projectiles::movement::move_projectiles_with_collision`, structurally identical to the existing Rune Blast splash block but with its own independent radius/fraction constants so the two classes' tuning never has to move together. Ruin Bolt reuses the default sphere collider (no new hitbox shape needed, unlike Shadow Stag/Runic Surge/Seismic Fissure).

Ruin Bolt's damage also multiplies by Embrace the Ruin's current damage bonus (see 73.4) when active, in addition to Ability Power -- see 73.4's note on why this goes through a direct read of Gravemind's own component rather than `combat::StatBuffs`.

## 73.3 Ability 2 --- Consume (`E`)

**Status:** IMPLEMENTED / CONFIRMED

Current tuning:

-   cooldown: `22.0` seconds
-   base heal: `30%` of max health
-   bonus heal: `+6%` of max health per enemy within a `6.0`-unit radius, capped at `4` enemies (so total healing tops out around `54%` max health with four or more enemies nearby)
-   each enemy within that radius also takes a direct "drained" hit of `10.0` damage (scaled by Ability Power)

Matches Section 23's "rip Ruin/corruption energy from nearby enemies" and "restore a substantial amount of health ... plus additional healing based on nearby affected enemies," with the per-enemy bonus explicitly capped (a disclosed balance choice, not a locked number -- an uncapped version would let a large mob heal Gravemind for an unreasonable amount). Consume is the project's first ability that heals the player; no shared "central heal path" existed (unlike `combat::apply_player_damage` on the damage side), so it writes directly into `Health`, clamped to max -- flagged in-code as something a second heal source should probably factor into a shared helper instead of duplicating.

## 73.4 Ability 3 --- Embrace the Ruin (`R`)

**Status:** IMPLEMENTED / CONFIRMED

Current tuning:

-   toggle, not a one-shot cast; `3.0`-second cooldown before it can be reactivated after being turned off (prevents toggling on for one frame just to refresh its damage buff without paying the drain cost)
-   health drain while active: `3%` of max health per second, floored at `1` HP -- Embrace's own drain cannot kill the player; damage from enemies/hazards while active is unaffected and can still kill normally
-   meter rises at `45`/second while the caster is "attacking," falls at `30`/second otherwise, on a `0-100` scale
-   at a fully-filled meter: `+50%` damage bonus, before stacks
-   each time the meter completely fills: resets to zero, raises Ruin by `6.0` (via `RuinState::add_ruin`), and grants one permanent-for-this-activation stack of `+8%` damage, capped at `5` stacks
-   the state can remain active while the meter falls, matching Section 23's explicit direction

This is Gravemind's signature risk/reward tool and the class's confirmed interaction with Ruin (Section 9.1: "class mechanics can interact with Ruin, especially Gravemind"). Three disclosed approximations, documented in-code:

1.  **"Attacking" is approximated as `PlayerCombat::attack_cooldown > 0.0`** -- the same flag `weapons::melee`'s own attack system already gates the next swing on. Ruinbound has no separate "is currently mid-attack" state, and building one for a single ability would be a larger change than this pass should make unprompted.
2.  **The health-drain floor (1 HP) is a deliberate deviation** from a literal "continuously drains" reading, so the ability can't silently end a run on its own -- matching Section 23's requirement that the class not be "entirely focused on constant low-health micromanagement."
3.  **Stacks are capped at 5** rather than stacking indefinitely -- Section 23 explicitly leaves "whether multiple fills stack indefinitely" open as a balance question, and an uncapped stack is very easy to make absurd.

Embrace grants its damage bonus to the caster's own basic attacks through `combat::StatBuffs` (a new `BuffSource::EmbraceTheRuin` variant, `combat/buffs.rs`) every frame while active, the same mechanism Warden of Shades' Guardian Otter already uses. Ruin Bolt and Consume apply the same multiplier a second way -- reading `GravemindEmbrace::damage_multiplier()` directly -- since the project's existing convention is that weapon *abilities* don't read `StatBuffs` (only basic attacks do, per Section 51.2's note); this keeps Embrace's "significant damage increase" felt across the whole kit without changing that existing rule. A pulsing red-orange aura is spawned as a child of the caster while active and despawned when it ends.

## 73.5 Test-class swap

**Status:** IMPLEMENTED / CONFIRMED

`src/rooms/fallen_gate.rs` -- the game's active Startup zone -- now spawns `Gravemind` in place of `Wildshaper` as the player's class marker. The equipped starter weapon (Hammer, Common) was left unchanged, since Gravemind has no stated weapon preference and the project's confirmed rule is that classes don't restrict weapon choice. `src/rooms/test_grounds.rs` (inactive since Section 47.2) still references `Runesage` and was deliberately left unchanged, matching the same convention every previous class swap has followed.

## 73.6 Verification status

**Status:** CONFIRMED

Before this was written to the user's files, the same manual-review standard as every previous class was applied in place of compiling (no Rust toolchain was reachable from either this session's cloud container or the device-bridge shell): brace/paren/bracket balance checked programmatically across every touched/created file; every new function signature checked against its call sites; every system's multiple `Query<...>` parameters checked for provable disjointness under Bevy's `With`/`Without` rules (Player vs. `(With<Enemy>, Without<Player>)` throughout, the same pattern already proven elsewhere in this codebase); a repo-wide grep confirmed the only two `match` expressions on `ProjectileKind` both already carry a wildcard arm (so the new `RuinBolt` variant cannot break exhaustiveness), and that `BuffSource` is never matched exhaustively anywhere (only constructed), so the new `EmbraceTheRuin` variant is safe there too. One design mistake was caught and fixed during that same review pass: the Embrace the Ruin aura was originally going to be parented onto the caster with an `EntityCommands::add_child` call that has no precedent anywhere in this codebase; it was rewritten to spawn the aura inside a `with_children` closure instead, the same hierarchy API every other class's visual-attach function already proves compiles here.

The user then ran `cargo fmt && cargo check && cargo run` on their own machine and confirmed a clean build, and reported (via a multi-select check, not a pasted transcript or a beat-by-beat playtest log) that: the class loads and all three abilities visibly fire in-game, and full mechanic testing was performed. Per this project's own rule (IMPLEMENTED status must reflect actual working code, not general impressions), that combination -- a clean build plus the user's own confirmation of full mechanic testing -- is what moves this section from IMPLEMENTED to IMPLEMENTED / CONFIRMED, matching the convention used in Section 46.6. No specific bugs, numeric complaints, or feel adjustments were reported alongside this confirmation; if playtesting turns up a tuning issue later (the numbers in 73.2-73.4 remain explicit placeholders), that becomes its own follow-up entry rather than reopening this one.

------------------------------------------------------------------------

# 74. 2026-09-05 Decision Log Additions (Gravemind)

## 2026-09-05 --- Gravemind class implemented

**Status:** IMPLEMENTED / CONFIRMED

**Decision:**
Implement the Gravemind class (Section 23) in full: class scaffold, and all three abilities -- Ruin Bolt (a heavy corruption bolt with a splash "blast"), Consume (a strong self-heal that drains nearby enemies, scaling with how many are close), and Embrace the Ruin (a toggled high-risk state with a rising/falling meter that raises Ruin and grants stacking damage as it repeatedly fills). Ruin Bolt and Consume read `Equipment::ability_power_multiplier()`; all three route damage through `combat::apply_enemy_damage`/`combat::effective_stagger_resistance` where applicable. The active test scene (Fallen Gate) now spawns Gravemind instead of Wildshaper, keeping the existing Hammer starter weapon.

**Reason:**
Requested class build-out, following the approved Section 23 kit, with three disclosed design decisions where the source document left specifics open: "attacking" for Embrace's meter is approximated via the existing `PlayerCombat::attack_cooldown` flag rather than new state; Embrace's self-drain is floored at 1 HP rather than able to end a run on its own; and repeated meter fills grant a capped (5) stack rather than stacking indefinitely.

**Implementation impact:**
`characters/gravemind.rs` (new), `characters/gravemind_abilities.rs` (new), `characters/mod.rs`, `combat/buffs.rs` (`BuffSource::EmbraceTheRuin`), `weapons/components.rs` (`ProjectileKind::RuinBolt`), `weapons/projectiles/movement.rs` (Ruin Bolt splash block), `rooms/fallen_gate.rs` (test-class swap).

**Supersedes:**
None design-wise -- Section 23's approved kit is unchanged; this is its first implementation.

**Notes:**
Confirmed by the user: `cargo fmt && cargo check && cargo run` completed cleanly, the class loads and all three abilities fire in-game, and full mechanic testing was performed -- see Section 73.6. All numeric tuning values remain explicit placeholders pending further playtesting/balance feedback, the same convention used for every other class and the Saint's Fist Slam.

------------------------------------------------------------------------

# 75. Current Handoff (as of 2026-09-05, updated)

This supersedes Section 72 only by addition -- none of Section 69's outstanding items, Sections 51.8/54.8/64.6's class-verification checklists, or Section 70.7's Ruin verification checklist are resolved or touched by this session's work.

1.  Everything listed in Sections 69 and 72 is still outstanding and unchanged.
2.  Gravemind (Sections 73-74) is now IMPLEMENTED / CONFIRMED: a full first implementation of its class scaffold and all three abilities (Ruin Bolt, Consume, Embrace the Ruin), confirmed by the user via a clean `cargo fmt && cargo check && cargo run` and in-game testing (class loads, all three abilities fire, full mechanic testing performed -- see Section 73.6 for exactly what that confirmation covers). The active test scene (Fallen Gate) now spawns Gravemind in place of Wildshaper.
3.  Gravemind is the first class in this project to reach CONFIRMED status without a pasted `cargo check`/`cargo run` transcript in this log -- the confirmation came via the user's own summary rather than raw output. If a specific bug, numeric complaint, or feel issue turns up in further play, log it as its own follow-up entry rather than reopening Section 73; the numeric tuning throughout Section 73 remains an explicit placeholder pending balance feedback regardless of CONFIRMED status.
4.  Wildshaper (Sections 64-65) remains its own unconfirmed, un-playtested implementation now that it is no longer the active test class -- its own verification checklist (Section 64.6) is untouched and still outstanding whenever it next becomes the focus.
5.  With Gravemind confirmed, the natural next steps are either playtesting/tuning passes on Gravemind's numbers, confirming Wildshaper (still outstanding), or picking up the next item from Section 69/72's broader outstanding list -- no specific next feature has been requested yet.
