# Deepcore — plan

## Understanding of the game

Deepcore is a co-op first-person voxel mining game. One shared, persistent mine
(1024 × 1024 wide, 512 deep) is divided into six depth layers. Players start at
a surface camp with a steel pickaxe, dig down, haul ore back, and build
structures that unlock better tools and refining steps. Depth is the
progression axis: each layer needs a higher tool tier, and each era's tool is
gated by materials from the layer the previous tool opened.

The endgame is nuclear: find pitchblende in the Deep Crust, crush → leach →
precipitate → convert → enrich → fabricate, then build a reactor and a Nuclear
Drill that mines a 3×3 face 12× faster than the starting pickaxe.

Everything the players do must be a physical structure in the world that they
build, feed and operate — not a menu upgrade.

## Architecture

```
deepcore/
├── Cargo.toml            workspace, pinned versions, profiles
├── assets/data/*.ron     the single source of truth for all content
├── assets/shaders/       (voxel.wgsl kept for a future custom material)
├── crates/
│   ├── shared/           every game rule; no rendering, no sockets
│   ├── server/           authoritative simulation + net
│   ├── client/           rendering, input, HUD, UI
│   └── lobby/            accounts and server browser (M10)
└── tools/
    ├── atlas_gen/        procedural atlas generator
    └── content_gen/      authoring aid that emitted the RON files
```

The rule that keeps it sane: **`shared` holds every rule, `server` decides,
`client` shows.**

### `shared`

| Module | Contents |
|---|---|
| `content` | RON definitions, `ContentRegistry`, id resolution, fail-fast validation |
| `voxel` | `BlockPos`, `ChunkPos`, `Face`, palette-compressed `Chunk`, wire codec |
| `worldgen` | deterministic layers, caves, ore veins, camp clearing, bedrock bounds |
| `movement` | AABB-vs-voxel controller shared by prediction and authority |
| `inventory` | stacks, slots, insert/remove/transfer/split/quick-move |
| `crafting` | recipe matching, machine tick state machine, break-time formula |
| `power` | steam and electric network solver, brownouts, batteries |
| `hazards` | vitals, radiation dose ladder, heat, oxygen, cave-in rule |
| `blocks` | mine/place validation, area tools, drop application |
| `protocol` | replicated components, intents, server events, `PROTOCOL_VERSION` |

### `server`

`world` (chunk cache, raycast, edits), `sim` (mining, machines, power, hazards,
pickups, intent handlers), `net` (replicon/renet setup, handshake, chunk
streaming, broadcasts), `plugin` (builds the whole game on a Bevy `App`).

The server plugin is used by both the dedicated binary and the client's
"Host game" mode, so hosting is literally the same plugins in one process.

### `client`

`render` (procedural atlas, greedy mesher, async chunk meshing), `player`
(predicted first-person controller, targeting), `hud` (Bevy UI), `ui` (egui
screens, one pass per frame), `net_client`, `host`, `world` (client copy of the
voxel world).

## Milestone list

From the brief, in order. Networking comes early (M2) on purpose.

| # | Milestone | Status this session |
|---|---|---|
| M0 | Workspace, four crates, pinned versions, check script, window + headless server | **done** |
| M1 | Voxel world: palette chunks, worldgen, atlas, greedy meshing, controller, mining | **done** (single process) |
| M2 | Multiplayer foundation: handshake, streaming, replication, prediction, intents | **partially**: protocol, handshake, streaming, corrections and intent handlers are implemented; two-process play not yet exercised |
| M3 | Content, inventory, crafting (Era 0) | **partial**: content, validator, `balance_report`, inventory, hand crafting over intents |
| M4 | Machines, research, Era 1 | **partial**: machine tick, UI, protocol, tech tree; place→entity wiring missing |
| M5 | Era 2 steam and logistics | **partial**: steam solver, crusher/washer recipes, pipe blocks |
| M6 | Era 3 electricity | **partial**: electric solver, brownouts, batteries, machine list |
| M7 | Era 4 chemistry | **partial**: content and recipes only |
| M8 | Era 5 nuclear | **partial**: full recipe chain, centrifuge cascade data, reactor data, Nuclear Drill item with 3×3 area |
| M9 | Persistence and polish | **not started** |
| M10 | Lobby, accounts, server browser | **not started** |

`PROGRESS.md` lists exactly what is deferred and what to do next.
