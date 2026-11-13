# Deepcore

A co-op 3D voxel mining game in Rust. Start at a surface camp with a steel
pickaxe, dig down through six layers, haul ore home, and build the machines
that carry you from hand tools to a working nuclear reactor and a Nuclear Drill.

Built with [Bevy](https://bevyengine.org) 0.20, server-authoritative, with
`bevy_replicon` + renet for multiplayer and `bevy_egui` for the dense
inventory, machine and tech-tree screens.

> **Status:** early. The world, rendering, mining, content and the simulation
> rules work and are tested; the machine-placement loop and persistence are
> not finished. See [PROGRESS.md](PROGRESS.md) for exactly what works and what
> does not.

## Build

Requires Rust 1.85+ (edition 2024) and, on Linux, the usual Bevy system
dependencies:

```bash
# Debian / Ubuntu
sudo apt install build-essential pkg-config libasound2-dev libudev-dev \
  libx11-dev libxi-dev libxcursor-dev libxrandr-dev libwayland-dev \
  libxkbcommon-dev
```

```bash
cargo build --release
./scripts/check.sh      # fmt + clippy + tests
```

No art assets are downloaded: the block texture atlas is generated
procedurally from `assets/data/blocks.ron` at startup.

## Play

Single player, or host a game for friends (the server runs inside the client):

```bash
cargo run -p deepcore_client --bin deepcore-client -- --host
```

Join a dedicated server:

```bash
# one machine hosts...
cargo run -p deepcore_server --bin deepcore-server -- --port 7788 --seed 1

# ...and others join
cargo run -p deepcore_client --bin deepcore-client -- --join 192.168.1.10:7788 --name Ada
```

Useful flags on both binaries: `--simulate-latency <ms>`, `--simulate-loss <pct>`.

### Controls

| Key | Action |
|---|---|
| WASD / Space / Shift / Ctrl | move, jump, sprint, crouch |
| Mouse | look; left button mines, right button interacts |
| E or Tab | backpack and hand crafting |
| T | tech tree |
| Y | chat |
| F3 | debug overlay |
| Esc | pause |

## Adding content

Everything — blocks, items, recipes, machines, the tech tree, the layers — lives
in RON files under `assets/data/`. Adding an ore or a recipe never requires
touching Rust:

1. Add the item to `items.ron` and the block to `blocks.ron`.
2. Add a recipe to `recipes.ron` and place the ore in a layer in `layers.ron`.
3. Add or extend a tech node in `tech_tree.ron` so the recipe is gated.
4. Check it:

```bash
cargo run -p deepcore_shared --bin balance_report
```

The loader validates on startup and fails fast with a readable message for
unknown item references, recipes no machine can make, tech nodes with missing
parents, items that cannot be obtained and ores no layer places. Clients with a
different content hash are refused at the handshake with "Content mismatch".

## Layout

```
crates/shared   every game rule (content, voxels, worldgen, movement,
                inventory, crafting, power, hazards, protocol)
crates/server   authoritative simulation and networking
crates/client   rendering, input, HUD and UI
crates/lobby    accounts and server browser (not yet implemented)
assets/data     the RON content files
docs/PLAN.md    architecture and milestone status
PROGRESS.md     what works, what is deferred, and what to do next
```

## License

MIT.
