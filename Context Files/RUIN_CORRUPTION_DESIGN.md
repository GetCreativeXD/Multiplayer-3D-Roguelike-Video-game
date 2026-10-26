# Ruinbound: Ruin Corruption Resource

## Overview

Ruin is a run-based resource representing how deeply the player is being infected by the dungeon. It creates meaningful choices: accept more Ruin for stronger rewards, or spend resources and take safer paths to reduce it.

Ruin should create pressure without making the player feel randomly punished or instantly killed. It changes enemies, rooms, rewards, and events while still leaving the player with ways to respond through combat skill, route choices, and cleansing.

## Core Rules

- Ruin ranges from `0–100`.
- Ruin normally changes during a dungeon run.
- Ruin is displayed clearly on the HUD.
- High Ruin makes rooms and enemies more dangerous, but also improves rewards.
- Ruin cannot directly kill the player.
- Corruption effects create tactical problems rather than unavoidable damage.
- At the end of a run, most Ruin is removed. Major choices may leave permanent consequences or unlocks later.

## Passive Ruin Growth

Ruin slowly increases as time passes, even when the player avoids corruption choices. This ensures that the dungeon becomes more unstable during a long run.

- Ruin increases by `+1 every 90 seconds` during the early run.
- The timer pauses in menus, shops, and safe rooms.
- The timer continues during combat and exploration.
- Entering a new room may add `+1 Ruin`.
- The passive increase should remain small compared with deliberate corruption choices.

### Suggested time scaling

| Run time | Passive growth |
|---:|---|
| 0–10 minutes | `+1 Ruin every 90 seconds` |
| 10–20 minutes | `+1 Ruin every 75 seconds` |
| 20+ minutes | `+1 Ruin every 60 seconds` |

The faster growth later in the run reflects the dungeon becoming increasingly unstable. Before each passive increase, the game should display a warning such as **“The Ruin is spreading…”**.

Passive growth means that players cannot remain completely safe forever, optional rooms become meaningful decisions, cleansing becomes valuable, and longer boss fights become more intense.

## How Players Gain Ruin

### Exploration choices

- Enter a corrupted side room: `+5 to +10 Ruin`
- Take a shortcut through a sealed ruin: `+8 Ruin`
- Break a forbidden shrine: `+10 to +15 Ruin`
- Steal an offering from a shrine: `+5 Ruin`
- Accept a cursed bargain: `+15 to +25 Ruin`
- Revive at a corrupted resurrection shrine: `+10 Ruin`
- Allow a corrupted enemy to escape: `+3 Ruin`

### Combat choices

- Use a powerful corrupted weapon ability: `+1 Ruin`
- Equip a corrupted legendary item: constant or periodic Ruin increases
- Defeat an elite without cleansing its corruption: `+5 Ruin`
- Destroy corruption nodes during an encounter: lowers local room corruption

### Reward choices

Players can choose between a normal reward, a stronger corrupted reward that increases Ruin, or a mystery reward with an uncertain result.

Example:

> Take the ordinary hammer, or accept the corrupted hammer with better damage and gain `+12 Ruin`.

## How Players Lower Ruin

Ruin reduction should always require a meaningful sacrifice so cleansing does not become automatic maintenance.

- Cleanse at a purified shrine: `-10 Ruin`
- Spend gold at a cleansing altar: `-5 to -15 Ruin`
- Complete a purification room: `-10 Ruin`
- Give up a reward to cleanse: `-8 Ruin`
- Defeat a corruption-focused miniboss: `-10 Ruin`
- Restore a ruined room objective: `-5 Ruin`
- Use a rare cleansing consumable: `-15 Ruin`

Possible cleansing costs include gold, healing, relics, rewards, time, safer routes, or a portion of the player's current power.

## Ruin Thresholds

| Ruin | State | Gameplay effects |
|---:|---|---|
| 0–19 | Unmarked | Normal rooms and enemy behavior |
| 20–39 | Touched | Small chance for corrupted enemies and altered room events |
| 40–59 | Stained | More elites, stronger rewards, and occasional room mutations |
| 60–79 | Corrupted | Common corruption events, new enemy abilities, and major room changes |
| 80–99 | Doomed | Dangerous mutations, elite corruption, weaker healing, and the best rewards |
| 100 | Ruinbreak | A major corruption event occurs and Ruin is forcibly reduced |

### 0–19: Unmarked

- Normal room layouts
- Normal enemy variants
- Standard rewards
- Inexpensive purification
- Rare corrupted events

### 20–39: Touched

- Some enemies may spawn as corrupted variants.
- Shrine choices may offer corrupted alternatives.
- Rooms gain minor visual mutations.
- Corrupted rewards appear more often.
- Whispers or false visual hints may appear.

### 40–59: Stained

- One or two enemies per combat room may be corrupted.
- Elites may gain one corruption affix.
- Rooms can gain an additional hazard or blocked route.
- Corrupted rewards become more powerful.
- Bosses gain one additional corruption mechanic.
- Healing shrines may offer stronger healing at a Ruin cost.

### 60–79: Corrupted

- Corrupted enemies become common.
- Some enemies receive entirely new attacks.
- Rooms may mutate when the player enters.
- Additional enemy waves may appear.
- Normal rewards become less common.
- Shops may sell powerful cursed items.
- Some doors may open only after a corruption challenge.

### 80–99: Doomed

- Most combat rooms contain at least one corrupted enemy.
- Elites can have two corruption affixes.
- Rooms contain environmental corruption hazards.
- Healing is reduced by approximately `15%`.
- Extremely powerful corruption bargains become available.
- Bosses gain a second phase mechanic or additional attack pattern.
- Rewards reach their best quality.

### 100: Ruinbreak

Reaching 100 should trigger a dramatic event instead of an automatic run failure.

Possible Ruinbreak events:

- A powerful Corrupted Elite invades the current room.
- The room becomes a special survival encounter.
- A friendly NPC becomes corrupted.
- The player is pulled into a short corrupted pocket realm.
- The next room becomes a boss-like challenge.
- A major reward becomes available after defeating the corruption.

After the event, Ruin drops to approximately `60–70` and the player receives a temporary **Ruinbreak Scar**. The run continues, but the player must recover from the event.

## Corrupted Enemies

Corruption should modify existing enemies rather than replace the entire enemy roster. Each corrupted enemy receives:

1. A visual mutation
2. One gameplay modifier
3. A corruption reward
4. A weakness or counterplay option

### Corruption affixes

#### Bloodbound

- Gains increased health.
- Heals slightly when damaging the player.
- Takes extra damage from heavy attacks.

#### Rift-Touched

- Occasionally teleports a short distance.
- Leaves behind a damaging rift.
- The rift can be destroyed before it activates.

#### Husked

- Cannot be staggered normally.
- Breaks apart at low health and creates smaller enemies.
- Area attacks are effective against the fragments.

#### Blighted

- Attacks apply stacking blight.
- Blight disappears after the player avoids damage for several seconds.
- Rewards careful dodging.

#### Frenzied

- Moves and attacks faster.
- Has lower defenses.
- Becomes vulnerable after missing a heavy attack.

#### Mirrorbound

- Creates a temporary shadow copy.
- The copy deals reduced damage.
- The real enemy has a visible corruption core.

#### Gravefed

- Revives once unless its corruption core is destroyed.
- The core is exposed during the revive animation.

#### Void-Armored

- Gains a temporary shield.
- The shield disappears when nearby corruption nodes are destroyed.
- Encourages interaction with the room.

#### Possessed

- Is empowered by a corruption source elsewhere in the room.
- Destroying the source weakens the enemy.
- Creates an important target-priority decision.

### Corrupted enemy rewards

Corrupted enemies can drop more gold, better equipment, corruption shards, temporary buffs, rare crafting materials, or corrupted relics. The reward should make the fight tempting while allowing some corrupted encounters to be avoided.

## Corrupted Rooms

Room corruption should alter how a room is played, not only change its colors or appearance.

### Room mutation types

#### Enemy corruption

- Existing enemies gain corruption affixes.
- Additional corrupted enemies spawn during later waves.
- An elite may replace a normal enemy.

#### Layout corruption

- A passage collapses.
- A previously closed route opens.
- Platforms shift position.
- Doors move or become sealed.
- The safest route changes after combat begins.

#### Environmental corruption

- Corruption pools appear on the floor.
- Ruin tendrils block parts of the arena.
- Floating projectiles move through the room.
- Platforms periodically become unsafe.
- Darkness reduces visibility in sections of the room.

#### Objective corruption

- Defend a shrine while corruption attacks it.
- Destroy corruption hearts before opening the exit.
- Carry a purification flame through the room.
- Rescue an NPC before they become corrupted.
- Activate bells in a specific order while enemies respawn.

#### Reward corruption

- A normal reward becomes a corrupted reward.
- A reward costs health or increases Ruin.
- Ruin can be converted into temporary bonus damage.

### Local room corruption

Each room may have a local corruption level separate from the player's global Ruin.

| Local level | Effect |
|---:|---|
| 0 | Normal room |
| 1 | Minor visual and enemy changes |
| 2 | New hazard or altered enemy |
| 3 | Major layout or objective mutation |
| 4 | Full corrupted challenge room |

Global Ruin determines how likely a room is to mutate. Local corruption determines how severe the mutation becomes.

## Corruption Sources

Many corrupted rooms should contain a source that players can interact with:

- Corruption heart
- Ruin obelisk
- Possessed bell
- Black altar
- Crawling root mass
- Shattered soul vessel
- Corrupted gate

Players may destroy the source, ignore it, use it for a reward, purify it, or sacrifice something to it.

Destroying or purifying a source can weaken corrupted enemies, remove room hazards, lower Ruin slightly, improve the room's reward, or reveal hidden paths.

## Corrupted Rewards

Corrupted items are stronger than normal items but carry a drawback. They should support unusual builds, including ability-focused, basic-attack, and tank-oriented gear.

Examples:

- Increased hammer damage, but gain Ruin whenever the hammer hits an elite.
- Stronger ability damage, but healing is reduced.
- Faster dodge cooldown, but taking damage creates a corruption pool.
- More armor, but movement speed decreases at high Ruin.
- Increased basic attack damage, but every third attack raises Ruin.
- Greater critical chance, but critical misses briefly expose the player.

Corrupted rewards should not always be strictly negative. They should create powerful build choices and let the player decide how much danger they are willing to accept.

## Player Choice Events

### The Black Altar

- Leave it alone.
- Sacrifice 20% of current health for a powerful relic and `+10 Ruin`.
- Destroy it and fight a corrupted elite to gain `-5 Ruin`.
- Purify it using gold to gain a normal relic.

### The Fallen Knight

- Kill the knight for a normal reward.
- Accept the knight's blessing for bonus damage and `+8 Ruin`.
- Purify the knight by fighting a harder version and receive a defensive reward.
- Walk away and potentially encounter the knight later as a corrupted elite.

### The Sealed Door

- Take the safe route.
- Open the corrupted route for `+12 Ruin` and a guaranteed rare reward.
- Purify the door using a cleansing item.
- Leave it sealed and potentially encounter its contents later.

## Recommended Gameplay Loop

1. The player starts with low Ruin and stable rooms.
2. Time slowly increases Ruin during the run.
3. Exploration and reward choices allow the player to gain Ruin faster for stronger rewards.
4. Thresholds introduce corrupted enemies, room mutations, and new events.
5. The player chooses when cleansing is worth the cost.
6. At 100 Ruin, a Ruinbreak encounter occurs instead of an instant loss.
7. The player survives the dungeon's reaction to their choices and continues toward the boss.

The central principle is:

> Accept Ruin to become stronger, then survive the dungeon's reaction to your choices.

