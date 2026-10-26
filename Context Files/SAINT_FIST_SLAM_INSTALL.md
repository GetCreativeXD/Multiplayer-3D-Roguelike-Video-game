# Saint of the Broken Siege — Fist Slam pass

This package adds the boss model, animation integration, and the first boss mechanic. It does **not** spawn the boss in the arena yet.

## Install

1. Back up the project.
2. Replace the project's `src` folder with the packaged `src` folder.
3. Merge the packaged `assets` folder into the project's existing `assets` folder. Do not delete the other existing assets.
4. From the project folder, run:

```bash
cargo fmt
cargo check
cargo run
```

## Fist Slam readability

- A pulsing red-orange disk marks the direct impact area for 0.8 seconds.
- On impact, a bright three-layer ground shockwave expands to a 17-unit radius.
- The visible ring and the damage check share the same radius and speed.
- The shockwave deals 14 damage when its leading edge crosses a grounded player.
- Jumping over it avoids the hit; dodge invulnerability also avoids it.
- The central fist impact deals 22 damage inside its marked 3.2-unit radius.

The next implementation step is a temporary boss test spawn or the second boss ability. Permanent arena spawning remains intentionally disabled until the boss kit is complete.
