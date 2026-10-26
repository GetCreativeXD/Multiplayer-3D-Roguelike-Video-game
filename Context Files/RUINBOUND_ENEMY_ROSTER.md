# Ruinbound Enemy Roster and Combat Rules

*Created: 2026-08-23*  
*Status: Confirmed design direction; exact numbers remain subject to playtesting.*

## 1. Universal Enemy Combat Rules

### 1.1 Fair and readable danger

- Enemy attacks require visible and/or audible telegraphs.
- Normal enemies should punish mistakes without routinely killing a full-health player in one hit.
- Elites, minibosses, and bosses become difficult through patterns, positioning, timing, and repeated decisions rather than unavoidable damage.
- Dangerous attacks require proportionally longer or clearer telegraphs.

### 1.2 Every boss attack must have counterplay

Every boss ability must always have at least one reliable response available to the player. Depending on the attack, valid responses can include:

- dodge through or away from the attack
- jump over a ground wave
- move out of a clearly marked area
- stand behind solid cover
- interrupt the cast when stagger rules allow
- destroy a spawned object or summoned enemy
- move into a deliberately safe section of the arena

Boss abilities must not become unavoidable merely because the player selected a particular class or weapon. The universal dodge and movement kit must always provide a valid solution.

### 1.3 Encounter-area activation

Enemies belong to the encounter area or room in which they spawn.

- An enemy does not target or pursue players who have not entered its encounter area.
- Entering a nearby room must not wake ranged enemies several rooms away.
- Once at least one player enters an encounter area, its enemies can activate and select valid players inside that area.
- Damaging any enemy activates and alerts every enemy with the same encounter ID, regardless of whether the damage came from a melee attack, ranged attack, weapon ability, class ability, or future damage source.
- Damage-triggered aggro persists for a configurable grace period so a long-range hit cannot wake an encounter for only one frame.
- Encounter activation should be shared by the room's enemy group so enemies awaken together when appropriate.
- After activation, enemies can pursue players outside the original encounter area.
- Every encounter has a configurable aggro extension beyond each edge of its spawn area.
- Enemies remain aggroed while a player stays inside this expanded boundary.
- Crossing the expanded boundary makes the enemies disengage and return toward their home positions.
- The extension is measured from the encounter area's edges rather than separately from every enemy, so enemies belonging to the same room behave consistently.
- Exact reset behavior—return home, recover health, or remain damaged—will be decided during multiplayer and room-reset work.

### 1.4 Ledge-safe movement

- All ordinary enemy chase, retreat, circling, and return-home movement checks for supporting ground before moving.
- When the direct route reaches a ledge, the shared movement policy tries alternate steering directions and otherwise stops rather than walking off.
- A final shared safety system restores any enemy that ends an unauthorized action over unsupported terrain, protecting current and future archetypes.
- Purpose-built leaps, lunges, and teleports may cross a gap only after registering a supported landing point through the shared authorized-traversal policy.
- If an ability has no valid landing point, it cancels or chooses a different behavior instead of allowing the enemy to fall.
- The Cinder Hound uses grounded safety while circling and validated landing safety for its lunge.

### 1.4 Ranged-enemy positioning

Ranged enemies have configurable minimum, preferred, and maximum attack ranges.

- Beyond maximum range: move closer instead of attacking.
- Within preferred range: stop, aim, and attack.
- Inside minimum range: retreat, sidestep, or use a specific close-range response.
- Ranged attacks require line of sight unless an ability explicitly says otherwise.
- Solid room geometry blocks ordinary and tracking projectiles.
- Ranged enemies cannot shoot into an encounter area that has not been activated by a player entering it.

### 1.5 Tracking projectile rules

- Tracking must be strong enough to influence movement but weak enough to dodge.
- Projectiles turn at a capped rate rather than instantly snapping toward a player.
- A well-timed dodge must break or outmaneuver their approach.
- Projectiles use swept physical collision and collide with walls, pillars, floors, and other solid geometry.
- Moving behind a wall is valid counterplay; a tracking projectile hits the wall instead of passing through it.
- Tracking ends after the projectile hits something, expires, or otherwise reaches its designed limit.

## 2. Normal Enemies — Outer Ruins

### 2.0 Testing-model rule

Every enemy receives a distinct code-generated placeholder model when its gameplay is first implemented.

- The placeholder must have a recognizable silhouette, proportions, equipment, and color language.
- Gameplay state, physics, health, and AI remain on the invisible enemy root entity.
- Placeholder meshes are visual children of that root.
- Final GLB models can later replace the visual children without rewriting gameplay systems.
- Color alone is not sufficient differentiation because enemies must remain recognizable during effects, darkness, and accessibility testing.

Planned placeholder identities:

| Enemy | Testing silhouette |
|---|---|
| Ruinbound Footman | Medium armored body, shield, sword, red tabard, glowing Ruin eye |
| Gatebow Marksman | Thin hooded body, long bow, raised shoulder quiver, amber aim glow |
| Cinder Hound | Low four-legged silhouette, long forelimbs, ember spine |
| Chain Wretch | Hunched body, oversized chain arm, hanging hook |
| Bell Husk | Narrow robed body, tall headpiece, oversized hand bell |
| Stonebound Laborer | Broad torso, stone shoulder, two-handed masonry hammer |
| Black Banner Knight | Tall armored body, large shield, banner rising above the silhouette |
| Riftbound Executioner | Very broad body, oversized axe, glowing Ruin cracks |
| Ruin Cantor | Floating layered robes, raised casting arms, orbiting rune fragments |
| Gatebreaker Vuld | Massive armored body carrying a horizontal battering ram |
| Forsaken Duelist | Slim spectral swordsman with long coat and bright blade trail |
| Hollow Castellan | Regal armor, crown-like helm, shield and spectral sword |
| Saint of the Broken Siege | Giant asymmetrical stone statue with breakable armor sections |
| Last Bellkeeper | Giant hunched frame carrying an enormous bell-shaped weapon |

### 2.1 Ruinbound Footman

**Role:** Foundational melee enemy.  
**Purpose:** Teaches melee telegraphs, flanking, blocking, and stagger.

Abilities and behavior:

- advances toward a player inside its active encounter area
- clearly telegraphed sword swing
- occasional damaged-shield block against frontal light attacks
- vulnerable from behind
- vulnerable to sufficiently strong stagger during attack preparation
- can protect ranged or support enemies by occupying the player

Useful encounter combinations:

- two Footmen protecting a Gatebow Marksman
- several Footmen surrounding a Bell Husk
- Footmen creating space for a Stonebound Laborer

### 2.2 Gatebow Marksman

**Role:** Ranged pressure.  
**Purpose:** Teaches line-of-sight play, ranged prioritization, and range management.

Abilities and behavior:

- visible aim line or equivalent aiming telegraph before firing
- fires a fast but dodgeable physical arrow
- attacks only between its minimum and maximum ranges
- moves closer when the player is beyond maximum range
- retreats or sidesteps when the player moves inside minimum range
- cannot target players until they enter the Marksman's encounter area
- arrows collide with walls and room geometry

### 2.3 Cinder Hound

**Status: IMPLEMENTED / VALUES SUBJECT TO TUNING**

**Role:** Fast flanker.  
**Purpose:** Applies mobile pressure while remaining fragile and interruptible.

Abilities and behavior:

- circles before committing to an attack
- telegraphed forward lunge
- low health and low stagger resistance
- attempts to approach from a different angle than nearby melee enemies
- vulnerable during recovery after missing its lunge

### 2.4 Chain Wretch

**Role:** Displacement.  
**Purpose:** Breaks safe positioning and creates enemy combinations.

Abilities and behavior:

- telegraphed hooked-chain throw
- a successful hook pulls the player a limited distance
- the hook can be dodged
- the throw can be interrupted during windup when stagger rules allow
- solid walls block the chain
- works especially well with Stonebound Laborers or dangerous arena zones

### 2.5 Bell Husk

**Role:** Support and target priority.  
**Purpose:** Encourages players to identify and remove high-value support targets.

Abilities and behavior:

- rings a corrupted hand bell after a readable channel
- temporarily increases nearby enemies' movement and attack speed
- weak when isolated
- the channel can be interrupted
- the buff has a visible range and clear visual effect
- multiple Bell Husk buffs should not stack without an intentional cap

### 2.6 Stonebound Laborer

**Role:** Slow bruiser.  
**Purpose:** Creates space and punishes careless positioning.

Abilities and behavior:

- slow masonry-hammer swings
- overhead slam creates a short forward ground shockwave
- players can dodge sideways or jump over the shockwave
- long recovery after the overhead slam
- high knockback but moderate damage

## 3. Elite Enemies — Outer Ruins

### 3.1 Black Banner Knight

**Role:** Armored commander.

Abilities and behavior:

- three-hit sword sequence with deliberately uneven timing
- shield rush with strong knockback and a clear preparation animation
- plants a corrupted banner that strengthens nearby normal enemies
- frontal shield block against light attacks
- vulnerable from behind and to high stagger during selected windups
- the banner can be destroyed or its placement interrupted

Counterplay:

- dodge the shield rush
- delay the dodge for the final sword strike
- flank the shield
- interrupt or destroy the banner

### 3.2 Riftbound Executioner

**Role:** Heavy delayed-attack elite.

Abilities and behavior:

- slow overhead execution strike
- wide spinning axe sweep
- dragging charge that leaves a temporary Ruin trail
- low-health escalation changes timing without creating unavoidable damage
- long recovery windows after its strongest attacks

Counterplay:

- dodge the overhead strike after its delayed release
- leave the sweep radius or dodge through its safe timing window
- interrupt the charge with sufficiently high stagger
- avoid the visible Ruin trail

### 3.3 Ruin Cantor

**Role:** Elite ranged controller and enemy support.

Abilities and behavior:

- fires a sequence of tracking Ruin bolts
- bolts have capped turning speed and can be dodged
- bolts collide with walls and all other solid geometry
- taking cover behind a wall causes a bolt to strike the wall rather than the player
- creates temporary, clearly telegraphed corruption zones
- channels power into another enemy to temporarily strengthen it
- performs a short teleport when surrounded
- obeys encounter-area activation, attack-range, and line-of-sight rules
- moves closer when its target is beyond maximum casting range

Counterplay:

- dodge across the tracking path instead of only running directly away
- use walls and pillars as cover
- interrupt the empowerment channel
- move out of corruption zones before they activate
- pursue after the Cantor teleports

## 4. Minibosses — Outer Ruins

### 4.1 Gatebreaker Vuld

**Role:** Geometry-driven armored miniboss.

Abilities and behavior:

- straight battering-ram charge
- wide horizontal ram sweep
- overhead ram slam that releases two traveling shockwaves
- breaks designated props and cover
- crashing into a solid wall briefly stuns Vuld and damages its armor

Counterplay:

- dodge sideways to make the charge miss
- bait the charge into a wall
- move outside or dodge through the horizontal sweep
- jump over the traveling shockwaves
- punish the wall-crash stun

### 4.2 The Forsaken Duelist

**Role:** Fast mechanical duel.

Abilities and behavior:

- fast but readable sword combinations
- sidesteps predictable ranged attacks
- visible parry stance
- spectral dash that ends behind or beside the target
- at low health, creates one fragile phantom that repeats selected attacks

Counterplay:

- do not attack into the visible parry stance
- turn and dodge the spectral dash follow-up
- destroy the phantom to reduce pressure
- punish the Duelist after completed combinations

## 5. Outer Ruins Boss Candidates

### 5.1 The Hollow Castellan

**Theme:** Former ruler and defender of the Fallen Gate.  
**Primary test:** Positioning, add control, stagger timing, and delayed dodges.

Phase one:

- sword-and-shield combinations
- shield charge
- summons spectral Footmen
- plants spectral banners that control sections of the arena

Phase two:

- discards the shield and uses a spectral two-handed blade
- faster combinations with delayed finishing strikes
- breaks designated arena walls, changing safe positioning without eliminating all safe routes

Required counterplay:

- shield charge can be dodged sideways
- sword strikes can be dodged with correct timing
- banner effects leave visible safe space or can be destroyed
- summoned enemies can be controlled or defeated
- broken arena sections remain clearly readable and never make an attack unavoidable

### 5.2 Saint of the Broken Siege

**Theme:** A giant castle statue animated by Ruin.  
**Primary test:** Large telegraphs, shockwave responses, and vulnerable-part targeting.

Phase one:

- heavy stone-fist strikes
- ground shockwaves
- telegraphed falling masonry
- breakable armor sections

Phase two:

- Ruin energy escapes through broken armor
- sweeping energy beams
- detached stone fragments become projectiles
- exposed core takes increased damage and stagger

Required counterplay:

- fist strikes can be dodged or moved away from
- ground shockwaves can be jumped over
- falling masonry shows impact areas before landing
- sweeping beams always contain a dodgeable timing window or a readable safe route
- stone projectiles can be dodged and collide with solid cover
- no phase combination may cover the entire arena without a valid escape

### 5.3 The Last Bellkeeper

**Theme:** A corrupted giant wielding the castle's warning bell.  
**Primary test:** Reading telegraphs and managing priority targets.

Abilities and behavior:

- swings the bell like a hammer
- bell impacts create expanding sound rings
- distinct telegraphs identify rings that should be dodged versus jumped
- summons Bell Husks that strengthen later attacks if ignored
- final phase cracks the bell and releases rhythmic Ruin pulses

Required counterplay:

- hammer swings can be dodged
- low sound rings can be jumped
- high or vertical rings can be dodged through or avoided by positioning
- Bell Husks can be interrupted and defeated
- final pulses follow a learnable rhythm and always preserve a valid response

## 6. Initial Implementation Roadmap

### Milestone 1 — Shared encounter awareness

**Status: IMPLEMENTED / EXTENSIBLE PROTOTYPE**

Create reusable components and systems for:

- encounter-area identity
- player-entry activation
- home position or home bounds
- target eligibility
- configurable aggro extension beyond encounter boundaries
- ranged minimum, preferred, and maximum ranges
- line-of-sight validation

This milestone prevents current and future enemies from targeting players across unrelated rooms.

### Milestone 2 — Ruinbound Footman

**Status: IMPLEMENTED / EXTENSIBLE PROTOTYPE**

Convert the existing prototype enemy into the first named normal-enemy archetype while preserving chase, windup, attack, recovery, stagger, death, health-bar, and loot behavior.

### Milestone 3 — Gatebow Marksman

**Status: IMPLEMENTED / VALUES SUBJECT TO TUNING**

Add the first ranged enemy using the shared encounter and range rules. Its arrows establish reusable hostile-projectile collision and dodge handling.

Current prototype includes configurable minimum/preferred/maximum ranges, approach and retreat behavior, physical line-of-sight checks, a visible aim telegraph, swept hostile-arrow collision, dodge handling, armor-aware damage, and a distinct placeholder model.

Current default encounter tuning uses a 50-unit aggro extension beyond every edge of an activated encounter area.

Current Marksman tuning uses a 20-unit preferred range, 32-unit maximum attack range, and 45-unit-per-second arrows. These values remain subject to playtesting.

### Milestone 4 — Cinder Hound and Bell Husk

**Cinder Hound status: IMPLEMENTED / VALUES SUBJECT TO TUNING**

Add flanking/lunge movement, followed by reusable enemy buff effects.

### Milestone 5 — Black Banner Knight

Use the established melee, support, stagger, and spawned-object systems to create the first elite.

### Milestone 6 — Gatebreaker Vuld

Create the first miniboss and establish geometry collisions, armor breaks, and jumpable hostile shockwaves.

### Milestone 7 — The Hollow Castellan

Create the first full boss after the normal, elite, miniboss, encounter-area, and hostile-projectile foundations are proven.

## 7. Confirmed First Fallen Gate Roster

- Ruinbound Footman
- Gatebow Marksman
- Cinder Hound
- Chain Wretch
- Bell Husk
- Stonebound Laborer
- Black Banner Knight
- Riftbound Executioner
- Ruin Cantor
- Gatebreaker Vuld
- The Forsaken Duelist
- The Hollow Castellan
- Saint of the Broken Siege
- The Last Bellkeeper
