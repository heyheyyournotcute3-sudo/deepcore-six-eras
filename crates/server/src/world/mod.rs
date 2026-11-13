//! Authoritative world storage: chunk map, block access, edits and raycasts.
//!
//! Chunks are generated on demand from the seed and only modified chunks are
//! kept dirty for saving. A `VoxelView` implementation is provided so the
//! shared movement code can collide against the server world directly.

use std::collections::HashMap;

use deepcore_shared::content::{BlockId, ContentRegistry};
use deepcore_shared::movement::VoxelView;
use deepcore_shared::voxel::{BlockEdits, BlockPos, Chunk, ChunkPos, Face};
use deepcore_shared::worldgen::WorldGen;

/// The server's block world.
pub struct World {
    generator: WorldGen,
    chunks: HashMap<ChunkPos, Chunk>,
    registry: ContentRegistry,
    /// Number of chunks generated since start, for diagnostics.
    pub generated: u64,
}

impl World {
    /// Creates a world for the given seed.
    pub fn new(registry: ContentRegistry, seed: u64) -> Self {
        let generator = WorldGen::new(&registry, seed);
        Self {
            generator,
            chunks: HashMap::new(),
            registry,
            generated: 0,
        }
    }

    /// The world seed.
    pub fn seed(&self) -> u64 {
        self.generator.seed()
    }

    /// Content registry.
    pub fn registry(&self) -> &ContentRegistry {
        &self.registry
    }

    /// Air block id, cached so callers do not need to hold the registry.
    pub fn air(&self) -> BlockId {
        self.registry.air
    }

    /// Bedrock block id.
    pub fn bedrock(&self) -> BlockId {
        self.registry.bedrock
    }

    /// Content hash of the registry.
    pub fn content_hash(&self) -> u64 {
        self.registry.hash
    }

    /// Layer definitions, cloned to avoid borrowing the registry while the
    /// world is mutated.
    pub fn layers(&self) -> Vec<deepcore_shared::content::LayerDef> {
        self.registry.layers().to_vec()
    }

    /// Block definition lookup by id, cloned for the same reason.
    pub fn block_def(&self, id: BlockId) -> Option<deepcore_shared::content::BlockDef> {
        self.registry.block_def(id).cloned()
    }

    /// Ensures a chunk exists in memory and returns a mutable reference.
    pub fn chunk_mut(&mut self, pos: ChunkPos) -> &mut Chunk {
        if !self.chunks.contains_key(&pos) {
            let chunk = self.generator.generate_chunk(pos);
            self.generated += 1;
            self.chunks.insert(pos, chunk);
        }
        self.chunks.get_mut(&pos).expect("chunk was just inserted")
    }

    /// Ensures a chunk exists in memory and returns a shared reference.
    pub fn chunk(&mut self, pos: ChunkPos) -> &Chunk {
        self.chunk_mut(pos)
    }

    /// Clones a chunk (generating it if needed) so a local host can hand it to
    /// the client world without sharing mutable state.
    pub fn clone_chunk(&mut self, pos: ChunkPos) -> Option<Chunk> {
        Some(self.chunk_mut(pos).clone())
    }

    /// True if the chunk is already loaded.
    pub fn is_loaded(&self, pos: ChunkPos) -> bool {
        self.chunks.contains_key(&pos)
    }

    /// Number of loaded chunks.
    pub fn loaded_count(&self) -> usize {
        self.chunks.len()
    }

    /// Reads a block, generating the chunk if needed. Horizontal and floor
    /// out-of-world reads return bedrock so players cannot leave the map;
    /// above the world is open sky.
    pub fn block(&mut self, pos: BlockPos) -> BlockId {
        if pos.y >= deepcore_shared::WORLD_HEIGHT {
            return self.registry.air;
        }
        if !pos.in_world() {
            return self.registry.bedrock;
        }
        let chunk = self.chunk_mut(pos.chunk());
        let [x, y, z] = pos.local();
        BlockId(chunk.get_local(x, y, z))
    }

    /// Reads a block without generating (returns air when not loaded).
    pub fn block_loaded(&self, pos: BlockPos) -> Option<BlockId> {
        if !pos.in_world() {
            return Some(self.registry.bedrock);
        }
        let chunk = self.chunks.get(&pos.chunk())?;
        let [x, y, z] = pos.local();
        Some(BlockId(chunk.get_local(x, y, z)))
    }

    /// Writes a block and marks the chunk modified.
    pub fn set_block(&mut self, pos: BlockPos, block: BlockId) {
        if !pos.in_world() {
            return;
        }
        let chunk = self.chunk_mut(pos.chunk());
        let [x, y, z] = pos.local();
        chunk.set_local(x, y, z, block.0);
        chunk.set_modified(true);
    }

    /// True if the block is solid for collision purposes.
    pub fn solid(&mut self, pos: BlockPos) -> bool {
        let block = self.block(pos);
        match self.registry.block_def(block) {
            Some(d) => !(d.passable || d.liquid),
            None => true,
        }
    }

    /// Voxel raycast used for targeting and line-of-sight checks.
    ///
    /// Returns the first non-air block along the ray and the face that was
    /// hit. Uses a simple stepping traversal which is accurate enough at the
    /// short ranges the game uses.
    pub fn raycast(
        &mut self,
        origin: [f32; 3],
        dir: [f32; 3],
        max_distance: f32,
        include_liquids: bool,
    ) -> Option<(BlockPos, Face)> {
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if len < 1e-6 {
            return None;
        }
        let step = 0.02f32;
        let d = [
            dir[0] / len * step,
            dir[1] / len * step,
            dir[2] / len * step,
        ];
        let mut p = origin;
        let mut last: Option<BlockPos> = None;
        let mut travelled = 0.0;
        // The cell the ray starts in is never reported: a player standing in
        // air (or inside water) must not "hit" that cell.
        let start_cell = BlockPos::new(
            origin[0].floor() as i32,
            origin[1].floor() as i32,
            origin[2].floor() as i32,
        );
        while travelled <= max_distance {
            let bp = BlockPos::new(
                p[0].floor() as i32,
                p[1].floor() as i32,
                p[2].floor() as i32,
            );
            if Some(bp) != last && bp != start_cell {
                let block = self.block(bp);
                let def = self.registry.block_def(block);
                let hit = match def {
                    Some(d) => !(block == self.registry.air || (d.liquid && !include_liquids)),
                    None => false,
                };
                if hit {
                    let face = match last {
                        Some(prev) => face_between(prev, bp),
                        None => Face::Up,
                    };
                    return Some((bp, face));
                }
                last = Some(bp);
            }
            p[0] += d[0];
            p[1] += d[1];
            p[2] += d[2];
            travelled += step;
        }
        None
    }

    /// True if the straight line between two points is unobstructed.
    pub fn line_of_sight(&mut self, from: [f32; 3], to: [f32; 3]) -> bool {
        let dir = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let dist = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if dist < 1e-3 {
            return true;
        }
        match self.raycast(from, dir, dist, true) {
            None => true,
            Some((pos, _)) => {
                let end = BlockPos::new(
                    to[0].floor() as i32,
                    to[1].floor() as i32,
                    to[2].floor() as i32,
                );
                pos == end || pos.distance_squared(end) <= 2.0
            }
        }
    }

    /// All chunks that have been modified, for saving.
    pub fn modified_chunks(&self) -> Vec<&Chunk> {
        self.chunks.values().filter(|c| c.is_modified()).collect()
    }

    /// Exports the edits of a chunk so it can be saved without the full data.
    pub fn chunk_edits(&self, chunk: &Chunk) -> BlockEdits {
        let mut edits = BlockEdits::default();
        let origin = chunk.pos().origin();
        for ([x, y, z], block) in chunk.iter_blocks() {
            edits.push(
                BlockPos::new(
                    origin.x + x as i32,
                    origin.y + y as i32,
                    origin.z + z as i32,
                ),
                block,
            );
        }
        edits
    }

    /// Applies saved edits on top of generated terrain.
    pub fn apply_edits(&mut self, edits: &BlockEdits) {
        for (pos, block) in &edits.edits {
            self.set_block(*pos, BlockId(*block));
        }
    }

    /// Counts ore blocks in a cube around a position, used by the Ore Scanner.
    pub fn scan_ores(&mut self, center: BlockPos, radius: i32) -> Vec<(BlockPos, BlockId)> {
        let mut found = Vec::new();
        for x in -radius..=radius {
            for y in -radius..=radius {
                for z in -radius..=radius {
                    let p = center.offset([x, y, z]);
                    if !p.in_world() {
                        continue;
                    }
                    let b = self.block(p);
                    let is_ore = self
                        .registry
                        .block_def(b)
                        .map(|d| !d.drops.is_empty() && d.min_tier >= 1 && d.hardness > 1.0)
                        .unwrap_or(false);
                    if is_ore {
                        found.push((p, b));
                    }
                }
            }
        }
        found
    }
}

impl VoxelView for World {
    fn solid(&self, pos: BlockPos) -> bool {
        if !pos.in_world() {
            return true;
        }
        match self.block_loaded(pos) {
            Some(b) => match self.registry.block_def(b) {
                Some(d) => !(d.passable || d.liquid),
                None => true,
            },
            // Unloaded chunks are treated as solid so players never fall out
            // of the world while a chunk streams in.
            None => true,
        }
    }
}

fn face_between(from: BlockPos, to: BlockPos) -> Face {
    if to.x > from.x {
        Face::East
    } else if to.x < from.x {
        Face::West
    } else if to.y > from.y {
        Face::Up
    } else if to.y < from.y {
        Face::Down
    } else if to.z > from.z {
        Face::South
    } else {
        Face::North
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let reg = ContentRegistry::load_dir(deepcore_shared::data_dir()).unwrap();
        World::new(reg, 12345)
    }

    #[test]
    fn surface_is_solid_and_sky_is_air() {
        let mut w = world();
        let spawn = deepcore_shared::worldgen::spawn_pos();
        let air = w.air();
        // The spawn cell itself is empty so the player can stand in it...
        assert_eq!(w.block(spawn), air);
        // ...and the block below it is ground.
        assert_ne!(w.block(spawn.offset([0, -1, 0])), air);
        let above = spawn.offset([0, 2, 0]);
        let b = w.block(above);
        assert_eq!(
            b,
            air,
            "block at {:?} is {:?}",
            above,
            w.block_def(b).map(|d| d.id)
        );
    }

    #[test]
    fn out_of_world_is_bedrock_except_open_sky() {
        let mut w = world();
        let bedrock = w.bedrock();
        let air = w.air();
        assert_eq!(w.block(BlockPos::new(-5, 10, 10)), bedrock, "west wall");
        assert_eq!(w.block(BlockPos::new(5000, 10, 10)), bedrock, "east wall");
        assert_eq!(
            w.block(BlockPos::new(10, -5, 10)),
            bedrock,
            "below the floor"
        );
        // Above the world is open sky, not a lid.
        assert_eq!(w.block(BlockPos::new(10, 9000, 10)), air);
    }

    #[test]
    fn set_block_marks_chunk_modified() {
        let mut w = world();
        let spawn = deepcore_shared::worldgen::spawn_pos();
        w.set_block(spawn, w.registry.air);
        assert_eq!(w.block(spawn), w.registry.air);
        assert!(!w.modified_chunks().is_empty());
    }

    #[test]
    fn raycast_hits_ground() {
        let mut w = world();
        let spawn = deepcore_shared::worldgen::spawn_pos();
        let origin = [
            spawn.x as f32 + 0.5,
            spawn.y as f32 + 3.0,
            spawn.z as f32 + 0.5,
        ];
        let hit = w.raycast(origin, [0.0, -1.0, 0.0], 8.0, false);
        assert!(hit.is_some());
        let (pos, face) = hit.unwrap();
        assert_eq!(pos.y, spawn.y - 1);
        // Entering the block from above means we hit its top face, which is
        // the face the player can place against.
        assert_eq!(face, Face::Down);
    }

    #[test]
    fn raycast_upwards_hits_nothing_in_the_open_sky() {
        let mut w = world();
        let spawn = deepcore_shared::worldgen::spawn_pos();
        let origin = [spawn.x as f32, spawn.y as f32 + 2.0, spawn.z as f32];
        assert!(w.raycast(origin, [0.0, 1.0, 0.0], 16.0, false).is_none());
    }

    #[test]
    fn raycast_beyond_world_returns_bedrock_for_walls() {
        let mut w = world();
        // Pointing at the world wall from inside the map.
        let hit = w.raycast([1.5, 300.0, 512.5], [-1.0, 0.0, 0.0], 4.0, false);
        assert_eq!(hit.map(|(p, _)| p), Some(BlockPos::new(0, 300, 512)));
    }

    #[test]
    fn raycast_reports_the_face_entered() {
        let mut w = world();
        let spawn = deepcore_shared::worldgen::spawn_pos();
        // Straight down hits the top face of the block below (Face::Down).
        let origin = [
            spawn.x as f32 + 0.5,
            spawn.y as f32 + 1.5,
            spawn.z as f32 + 0.5,
        ];
        let (_, face) = w.raycast(origin, [0.0, -1.0, 0.0], 6.0, false).unwrap();
        assert_eq!(face, Face::Down);
    }
}
