//! Deterministic world generation.
//!
//! The world is `1024 x 512 x 1024` blocks with bedrock walls and floor. Depth
//! is the progression axis: each layer from `layers.ron` fills a band below the
//! surface with a base block, a mix of secondary blocks, noise-carved caves and
//! clustered ore veins.
//!
//! Everything is a pure function of `(seed, chunk position, content)`, so the
//! same seed produces the same world on every machine and chunks can be
//! regenerated instead of saved.

use noise::{NoiseFn, OpenSimplex};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::content::{ContentRegistry, LayerDef, OreSpawnDef};
use crate::voxel::pos::{BlockPos, ChunkPos};
use crate::voxel::{CHUNK_VOLUME, Chunk};
use crate::{CHUNK_SIZE, SURFACE_Y, WORLD_HEIGHT, WORLD_SIZE};

/// Radius of the flat camp area around the spawn point.
pub const CAMP_RADIUS: i32 = 40;
/// Depth below the surface that is guaranteed cave-free inside the camp.
pub const CAMP_DEPTH: i32 = 12;

/// World generator. Cheap to clone; owns no mutable state.
pub struct WorldGen {
    seed: u64,
    /// Per-layer resolved data.
    layers: Vec<ResolvedLayer>,
    air: u16,
    bedrock: u16,
    surface: u16,
    cave_noise: OpenSimplex,
    cave_noise2: OpenSimplex,
}

#[derive(Clone)]
struct ResolvedLayer {
    /// Inclusive top and bottom Y of the band.
    top_y: i32,
    bottom_y: i32,
    base: u16,
    mix: Vec<(u16, f32)>,
    ores: Vec<ResolvedOre>,
    cave_density: f32,
}

#[derive(Clone, Copy)]
struct ResolvedOre {
    block: u16,
    frequency: f32,
    min_size: u32,
    max_size: u32,
}

impl WorldGen {
    /// Builds a generator from content and a seed.
    pub fn new(registry: &ContentRegistry, seed: u64) -> Self {
        let mut layers = Vec::new();
        for l in registry.layers() {
            let Some(base) = registry.block(&l.base_block) else {
                continue;
            };
            let top_y = SURFACE_Y - l.depth.0 as i32;
            let bottom_y = SURFACE_Y - l.depth.1 as i32;
            let mix = l
                .mix
                .iter()
                .filter_map(|(id, w)| registry.block(id).map(|b| (b.0, *w)))
                .collect();
            let ores = l
                .ores
                .iter()
                .filter_map(|o| resolve_ore(registry, o))
                .collect();
            layers.push(ResolvedLayer {
                top_y,
                bottom_y,
                base: base.0,
                mix,
                ores,
                cave_density: l.cave_density,
            });
        }
        let surface = registry.block("grass").map(|b| b.0).unwrap_or(1);
        Self {
            seed,
            layers,
            air: registry.air.0,
            bedrock: registry.bedrock.0,
            surface,
            cave_noise: OpenSimplex::new(seed as u32),
            cave_noise2: OpenSimplex::new((seed >> 32) as u32 ^ 0x9E37_79B9),
        }
    }

    /// The seed this generator was built with.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Generates one chunk.
    pub fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        let mut blocks = vec![self.air; CHUNK_VOLUME];
        let origin = pos.origin();

        for ly in 0..CHUNK_SIZE as usize {
            let world_y = origin.y + ly as i32;
            if !(0..WORLD_HEIGHT).contains(&world_y) {
                continue;
            }
            for lz in 0..CHUNK_SIZE as usize {
                let world_z = origin.z + lz as i32;
                for lx in 0..CHUNK_SIZE as usize {
                    let world_x = origin.x + lx as i32;
                    let idx = (ly * CHUNK_SIZE as usize + lz) * CHUNK_SIZE as usize + lx;
                    blocks[idx] = self.block_at(world_x, world_y, world_z);
                }
            }
        }
        Chunk::from_blocks(pos, &blocks)
    }

    /// Generates only the blocks in a chunk that differ from the air fill,
    /// which is what the ore and cave passes need.
    fn block_at(&self, x: i32, y: i32, z: i32) -> u16 {
        if !in_bounds(x, y, z) {
            return self.bedrock;
        }
        // Bedrock floor and world walls are unbreakable. The sky is open: the
        // player is bounded horizontally and below, never above.
        if y == 0 || x == 0 || z == 0 || x == WORLD_SIZE - 1 || z == WORLD_SIZE - 1 {
            return self.bedrock;
        }
        if y > SURFACE_Y {
            return self.air;
        }
        if y == SURFACE_Y {
            return self.surface;
        }

        // Find the layer band for this depth.
        let Some(layer) = self.layer_at(y) else {
            return self.bedrock;
        };

        // The camp is a flat, cave-free clearing so players always spawn on
        // solid ground inside a safe base area.
        let camp = in_camp(x, z) && y > SURFACE_Y - CAMP_DEPTH;

        // Caves carve out of solid rock below the topsoil.
        if !camp && layer.bottom_y + 2 < y && self.is_cave(x, y, z, layer.cave_density) {
            return self.air;
        }

        // Ore veins take precedence over the base block.
        if let Some(ore) = self.ore_at(x, y, z, layer) {
            return ore;
        }

        // Secondary blocks by weight.
        if !layer.mix.is_empty() {
            let mut rng = self.rng_for(x, y, z, 0x4D49_5821_u64);
            let roll: f32 = rng.random();
            let mut acc = 0.0;
            for (block, weight) in &layer.mix {
                acc += *weight;
                if roll < acc {
                    return *block;
                }
            }
        }
        layer.base
    }

    fn layer_at(&self, y: i32) -> Option<&ResolvedLayer> {
        self.layers
            .iter()
            .find(|l| y <= l.top_y && y > l.bottom_y)
            .or_else(|| self.layers.last())
    }

    fn is_cave(&self, x: i32, y: i32, z: i32, density: f32) -> bool {
        let scale = 0.035;
        let n1 = self
            .cave_noise
            .get([x as f64 * scale, y as f64 * scale * 1.7, z as f64 * scale]);
        let n2 = self.cave_noise2.get([
            x as f64 * scale * 1.3,
            y as f64 * scale,
            z as f64 * scale * 1.3,
        ]);
        let tunnel = n1.abs() < 0.055 * density as f64 && n2.abs() < 0.09;
        let cavern = n1 * n2 > 0.16 / density.max(0.1) as f64;
        tunnel || cavern
    }

    /// Returns the ore block at a position if a vein covers it.
    ///
    /// Veins are blobs: a seeded per-cell decision picks a centre, then the
    /// surrounding sphere of random radius is filled.
    fn ore_at(&self, x: i32, y: i32, z: i32, layer: &ResolvedLayer) -> Option<u16> {
        // Cell size must be >= max vein diameter so blobs do not need to be
        // tracked across cells.
        const CELL: i32 = 16;
        let cx = x.div_euclid(CELL);
        let cy = y.div_euclid(CELL);
        let cz = z.div_euclid(CELL);
        for (dx, dy, dz) in neighbourhood_offsets() {
            let cell = (cx + dx, cy + dy, cz + dz);
            let mut rng = self.cell_rng(cell.0, cell.1, cell.2, 0x0_2E5_u64);
            for ore in &layer.ores {
                if rng.random::<f32>() >= ore.frequency {
                    continue;
                }
                let ox = cell.0 * CELL + rng.random_range(0..CELL);
                let oy = cell.1 * CELL + rng.random_range(0..CELL);
                let oz = cell.2 * CELL + rng.random_range(0..CELL);
                let size = rng.random_range(ore.min_size..=ore.max_size);
                let radius = (size as f32 * 3.0 / (4.0 * std::f32::consts::PI)).cbrt() * 1.6;
                let dist = (((x - ox).pow(2) + (y - oy).pow(2) + (z - oz).pow(2)) as f32).sqrt();
                if dist <= radius && y <= layer.top_y && y > layer.bottom_y {
                    return Some(ore.block);
                }
            }
        }
        None
    }

    fn cell_rng(&self, cx: i32, cy: i32, cz: i32, salt: u64) -> ChaCha8Rng {
        let mut h = self.seed ^ salt;
        for v in [cx, cy, cz] {
            h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            h ^= v as u64;
            h ^= h >> 29;
        }
        ChaCha8Rng::seed_from_u64(h)
    }

    fn rng_for(&self, x: i32, y: i32, z: i32, salt: u64) -> ChaCha8Rng {
        let mut h = self.seed ^ salt;
        for v in [x, y, z] {
            h = h.wrapping_mul(0x1000_0000_01b3);
            h ^= v as u64;
        }
        ChaCha8Rng::seed_from_u64(h)
    }

    /// The layer that applies at a given depth, for hazard lookups.
    pub fn layer_at_depth(&self, y: i32) -> Option<usize> {
        self.layers
            .iter()
            .position(|l| y <= l.top_y && y > l.bottom_y)
    }

    /// Layer count.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }
}

fn resolve_ore(registry: &ContentRegistry, o: &OreSpawnDef) -> Option<ResolvedOre> {
    let block = registry.block(&o.block)?;
    Some(ResolvedOre {
        block: block.0,
        frequency: o.frequency,
        min_size: o.vein_size.0.max(1),
        max_size: o.vein_size.1.max(o.vein_size.0.max(1)),
    })
}

fn neighbourhood_offsets() -> impl Iterator<Item = (i32, i32, i32)> {
    (-1..=1).flat_map(|x| (-1..=1).flat_map(move |y| (-1..=1).map(move |z| (x, y, z))))
}

/// True if a position is inside the world bounds.
pub fn in_bounds(x: i32, y: i32, z: i32) -> bool {
    (0..WORLD_SIZE).contains(&x) && (0..WORLD_SIZE).contains(&z) && (0..WORLD_HEIGHT).contains(&y)
}

/// True if the position is inside the flat camp area at the surface.
pub fn in_camp(x: i32, z: i32) -> bool {
    let cx = WORLD_SIZE / 2;
    let cz = WORLD_SIZE / 2;
    (x - cx).abs() <= CAMP_RADIUS && (z - cz).abs() <= CAMP_RADIUS
}

/// Spawn point at the centre of the camp, standing on the surface.
pub fn spawn_pos() -> BlockPos {
    BlockPos::new(WORLD_SIZE / 2, SURFACE_Y + 1, WORLD_SIZE / 2)
}

/// Maps a block position to the layer index used by hazards.
pub fn depth_below_surface(y: i32) -> i32 {
    SURFACE_Y - y
}

/// Layers definitions are also needed by the server for hazards.
pub fn layer_def_at(layers: &[LayerDef], y: i32) -> Option<&LayerDef> {
    let depth = depth_below_surface(y);
    layers
        .iter()
        .find(|l| depth as u32 >= l.depth.0 && depth as u32 <= l.depth.1)
        .or_else(|| layers.last())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ContentRegistry;

    fn registry() -> ContentRegistry {
        ContentRegistry::load_dir(crate::data_dir()).expect("content loads")
    }

    #[test]
    fn same_seed_generates_identical_chunks() {
        let reg = registry();
        let a = WorldGen::new(&reg, 12345);
        let b = WorldGen::new(&reg, 12345);
        let pos = ChunkPos::new(16, 15, 16);
        let ca = a.generate_chunk(pos);
        let cb = b.generate_chunk(pos);
        for (p, block) in ca.iter_blocks() {
            assert_eq!(cb.get_local(p[0], p[1], p[2]), block);
        }
    }

    #[test]
    fn different_seeds_generate_different_worlds() {
        let reg = registry();
        let a = WorldGen::new(&reg, 1);
        let b = WorldGen::new(&reg, 2);
        let pos = ChunkPos::new(20, 8, 20);
        let ca = a.generate_chunk(pos);
        let cb = b.generate_chunk(pos);
        let mut diff = 0;
        for (p, block) in ca.iter_blocks() {
            if cb.get_local(p[0], p[1], p[2]) != block {
                diff += 1;
            }
        }
        assert!(diff > 0, "two seeds produced identical terrain");
    }

    #[test]
    fn camp_is_flat_and_cave_free() {
        let reg = registry();
        let w = WorldGen::new(&reg, 999);
        let spawn = spawn_pos();
        let air = reg.air.0;
        for dx in [-20, 0, 20] {
            for dz in [-20, 0, 20] {
                // The spawn cell itself is open air; the ground starts one
                // block below it.
                for dy in 1..=CAMP_DEPTH {
                    let p = spawn.offset([dx, -dy, dz]);
                    let chunk = w.generate_chunk(p.chunk());
                    let [x, y, z] = p.local();
                    let block = chunk.get_local(x, y, z);
                    assert_ne!(block, air, "camp has a hole at {p}");
                }
            }
        }
        // Above the surface inside the camp is open air.
        for dx in [-5, 0, 5] {
            let p = spawn.offset([dx, 0, 0]);
            let chunk = w.generate_chunk(p.chunk());
            let [x, y, z] = p.local();
            assert_eq!(chunk.get_local(x, y, z), air);
        }
    }

    #[test]
    fn layers_are_ordered_from_surface_to_bedrock() {
        let reg = registry();
        let w = WorldGen::new(&reg, 7);
        assert_eq!(w.layer_count(), 6);
        // A point just under the surface is topsoil, deep down is mantle.
        assert_eq!(w.layer_at_depth(SURFACE_Y - 8), Some(0));
        assert_eq!(w.layer_at_depth(4), Some(5));
    }

    #[test]
    fn bedrock_bounds_the_world() {
        let reg = registry();
        let w = WorldGen::new(&reg, 3);
        let chunk = w.generate_chunk(ChunkPos::new(0, 0, 0));
        let bedrock = reg.bedrock.0;
        assert_eq!(chunk.get_local(0, 5, 5), bedrock, "west wall");
        assert_eq!(chunk.get_local(5, 0, 5), bedrock, "floor");
    }
}
