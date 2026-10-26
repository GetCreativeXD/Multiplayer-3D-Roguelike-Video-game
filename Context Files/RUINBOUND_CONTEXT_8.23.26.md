# Ruinbound --- Project Context & Decision Log

*Last updated: 2026-08-23*

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

**CONFIRMED**

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

## 16.4 Hammer --- planned

**Identity:** ground slams, shockwaves, launching, huge stagger.

Potential progression:

-   Common --- Ground Slam
-   Rare --- Seismic Strike
-   Epic --- Earthshatter
-   Legendary --- unique named Hammer

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

Implemented prototype weapon families:

-   Longsword
-   Greatsword
-   Bow

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
-   add Hammer and Sword & Shield families
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

**Status:** EXPERIMENTAL / ACTIVE BLOCKER

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

### Unresolved statue/wall navigation bug

**Status:** NOT FIXED --- RESUME HERE NEXT SESSION

Footmen and Cinder Hounds can still stop immediately in front of the Fallen
Gate statue pedestal or another wall and fail to route around it. They are not
necessarily inside the collider. The visible behavior is:

1. the enemy chases normally
2. it reaches the front of the statue/wall
3. forward movement is rejected
4. it remains stationary instead of following a side route
5. manually repositioning the enemy makes it chase again temporarily

This indicates that wall detection works, but the failed forward step is not
reliably producing or following a valid detour. Several attempted fixes did
not solve the runtime behavior:

- reactive left/right ledge steering (caused oscillation)
- coarse and fine sampled-ground A*
- shared navigation caching and increased search budgets
- accepting the enemy's real starting position when its rounded A* cell
  overlaps the authored obstacle footprint
- making direct chase, waypoint movement, melee attacks, and Hound pounces
  respect the authored statue footprint
- clearing dynamic blocked samples after stuck recovery
- a remembered `0.8`-second sideways bypass after `0.35` seconds without
  progress

Do not mark this issue solved without runtime confirmation. The next debugging
step should add temporary navigation diagnostics rather than another guessed
steering change. Useful diagnostics should show or log:

- enemy navigation mode and combat state
- current grid coordinate and whether it is classified as blocked
- current waypoint and full cached path
- whether A* returned a full path, partial path, or empty path
- why the proposed waypoint step was rejected: missing ground, excessive step
  down, authored obstacle, world shape cast, or crowd steering
- stuck timer and selected bypass direction

Once the exact rejected condition is visible, fix that condition in the shared
navigation layer and remove or disable the temporary diagnostics.

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

1. Add temporary on-screen or console navigation diagnostics for one selected
   Footman and one selected Cinder Hound.
2. Reproduce the statue stop and capture the exact A* result and rejected-step
   reason.
3. Correct the shared navigation failure without increasing per-frame path
   sampling enough to restore stuttering.
4. Verify Footman, Hound, and Marksman navigation around the statue, walls,
   ledges, and other enemies.
5. Verify that the corrected Run animation loops smoothly.
6. After navigation is stable, continue the normal-enemy roster with the Bell
   Husk support archetype.
