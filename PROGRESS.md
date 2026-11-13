# Deepcore — progress log

> Read this file first. It is written so a future session (possibly a different
> model) can continue from here alone.

Last updated: after the first working session.

## What this project is

Deepcore is a co-op 3D voxel mining game in Rust: players spawn at a surface
camp with a steel pickaxe, dig down through six layers, haul ore home, and build
machines that unlock better tools — ending in the full uranium refining chain,
a Nuclear Drill and a reactor. See `docs/PLAN.md` for the design summary and
`PROMPT.md` for the original brief.

## Exact commands

```bash
# Everything (fmt + clippy + tests)
./scripts/check.sh

# Dedicated server (headless, 30 Hz)
cargo run -p deepcore_server --bin deepcore-server -- --port 7788 --seed 1

# Client, hosting the server in-process (single player / host a game)
cargo run -p deepcore_client --bin deepcore-client -- --host

# Client joining a server
cargo run -p deepcore_client --bin deepcore-client -- --join 127.0.0.1:7788 --name Ada

# Simulated latency and loss (both binaries)
cargo run -p deepcore_server -- --simulate-latency 150 --simulate-loss 2
cargo run -p deepcore_client -- --host --simulate-latency 150

# Content tools
cargo run -p deepcore_shared --bin balance_report
```

`--bots <N>` is parsed by the server config but bot clients are **not**
implemented yet (see Deferred).

## Pinned versions (verified against the crates.io index on 2026-12-06)

| Crate | Version | Note |
|---|---|---|
| bevy | 0.20.1 | newest stable that every other crate supports |
| bevy_replicon | 0.45.5 | requires bevy ^0.20 |
| bevy_replicon_renet | 0.21.0 | requires replicon ^0.45, bevy_renet 6 |
| bevy_egui | 0.43.1 | egui 0.36 |
| ron | 0.12.3 | content files |
| rand | 0.10.3 | note: the trait is `RngExt`, not `Rng` |
| noise | 0.9.0 | `OpenSimplex` |

Toolchain: rustc 1.99.1, edition 2024.

## State of the build

Everything in the list below compiles, passes `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` is green
(**80 tests**).

### Working

- **Workspace**: 4 crates + `tools/atlas_gen`, pinned versions, profiles,
  `scripts/check.sh`, `rustfmt.toml`.
- **Content (data-driven)**: `assets/data/{blocks,items,recipes,machines,
  tech_tree,layers}.ron` — 96 blocks, 194 items, 151 recipes, 39 machines,
  27 tech nodes, 6 layers. Loaded and cross-validated at startup; a bad
  reference fails fast with a readable message.
- **Voxel storage**: palette-compressed 32³ chunks with wire encode/decode,
  chunk/block maths, unit tested.
- **World generation**: deterministic from a seed; six layers, secondary block
  mixes, noise caves, clustered ore veins, bedrock floor and walls, open sky,
  a flat cave-free camp around spawn. Tests cover determinism, seed variation,
  camp safety, layer ordering and bedrock bounds.
- **Shared simulation**: break-time formula and tick rounding, inventory
  stacking/transfer/split/quick-move, machine tick state machine with idle
  reasons, steam and electric network solver with brownouts and batteries,
  radiation/heat/oxygen hazard maths, AABB-voxel movement with step-up.
- **Server**: authoritative world (chunk cache, raycast, line of sight),
  mining with reach + line-of-sight + tier validation, placement validation,
  pickups for overflow drops, machine ticking, hazard sampling, tech tree,
  intent handlers for move/mine/place/interact/move-items/craft/chat/respawn.
- **Networking**: replicon + renet wired on both sides with a protocol constant,
  handshake that checks protocol version and content hash, chunk streaming with
  per-client load/unload, batched block changes, movement corrections,
  inventory snapshots, tech tree and chat broadcast.
- **Client**: procedural 16×16 texture atlas generated from `blocks.ron`
  (no downloaded art), greedy chunk meshing on the async compute pool with
  cross-chunk face culling, async mesh jobs, first-person controller with
  prediction and correction smoothing, block targeting with a crosshair that
  turns red for "too hard", HUD (hotbar, vitals, depth/layer meter, Geiger
  readout, F3 debug overlay, objectives line), egui inventory/crafting/machine/
  tech-tree/pause/chat screens, host mode that runs the server in-process.
- **Verified visually**: under Xvfb + llvmpipe the client renders the camp
  surface with correct face winding, open sky and the camera at eye height.

## Bugs found and fixed this session (worth knowing)

1. **Triangle winding was inverted on all six faces.** Every quad was
   backface-culled, so the player saw the inside of the world. There is now a
   test (`every_triangle_winds_outward`) that checks each triangle's winding
   against its normal.
2. **Quad corners used the wrong offset on negative axes**, emitting vertices
   outside the block. Guarded by `vertex_positions_stay_inside_the_chunk`.
3. **Cross-chunk face culling used the wrong lookup keys**, so chunk borders
   had stray faces. Now keyed by the mesher's own out-of-chunk coordinates.
4. **egui UI systems must run in `EguiPrimaryContextPass`**, not `Update`.
   Calling `ctx_mut()` in `Update` panics with "No fonts available until first
   call to Context::run()". Also: only one egui pass per frame.
5. **Bevy silently skips systems with conflicting resource access.** Several
   HUD/streaming systems were being dropped this way. Any group that writes the
   same resource is now explicitly `.chain()`ed.
6. **`AmbientLight` is a camera component in Bevy 0.20**, not a resource.
7. **Pitch sign**: `Quat::from_euler(YXZ, yaw, pitch, 0)` applied to `-Z` tips
   the view *down* for positive pitch; `look_rotation` now matches
   `look_direction`, with a test.
8. World ceiling of bedrock was above the surface (off-by-one), then removed
   entirely — the sky is open.

## Deferred / not implemented

These are deliberately absent rather than stubbed:

- **Block entity spawning**: placing a machine block does not yet create a
  `Machine` entity, and the machine UI is not opened by interacting. The
  machine simulation, UI and protocol exist and are unit tested, but the
  place→entity→UI path is not connected.
- **Crafting from the UI** sends intents, but machine recipe selection,
  research deposits and control-rod messages are not handled server-side yet.
- **Chunk streaming for the network client** is implemented but only the host
  path has been exercised end-to-end.
- **Persistence** (region files, SQLite, autosave) — nothing yet.
- **Audio, particles, animations** — nothing yet.
- **Lobby crate** (accounts, server browser, Docker) — placeholder binary only.
- **Bot clients** (`--bots`) — flag parsed, no implementation.
- **Progression integration tests per era** — not written.
- **Performance**: the sandbox here has no GPU (llvmpipe runs at ~0.2 FPS), so
  frame-rate and bandwidth targets have **not** been measured.
- Lighting is flat ambient + one directional light; no block light propagation,
  so deep tunnels are uniformly dim rather than lamp-lit.

## Known issues

- Terrain in screenshots shows noticeable per-face colour noise. That is the
  procedural atlas speckle pattern at 16 px, not a geometry bug, but it reads
  poorly at distance and should be toned down.
- The client world and the hosted server world are separate copies; host mode
  streams chunks from the server world each frame. That works but is wasteful
  and will not scale to large view distances.

## Next steps (in order)

1. Connect placing a machine block to spawning a `Machine` entity, and
   `Interact` to sending `MachineUiState`. That unlocks the whole Era 1–5 loop.
2. Handle `SetMachineRecipe`, `ResearchDeposit`, `SetControlRods` on the server.
3. Wire the networked client path end to end (two processes on one machine).
4. Persistence: region files for modified chunks, SQLite for players/machines.
5. Era-by-era progression integration tests (headless `App`, scripted intents).
6. Block light propagation for lamps and headlamps.
7. Bot clients and the latency/loss test harness.
