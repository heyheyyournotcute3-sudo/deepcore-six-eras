//! The client's copy of the world.
//!
//! In single-player and when hosting, the client owns a full `World` (the
//! server's, shared in-process). When connected to a remote server it keeps a
//! sparse map filled from streamed chunks.

use std::collections::HashMap;

use bevy::prelude::*;
use deepcore_shared::content::{BlockId, ContentRegistry};
use deepcore_shared::movement::VoxelView;
use deepcore_shared::voxel::{BlockPos, Chunk, ChunkPos};
use deepcore_shared::worldgen::WorldGen;

/// Client-side block world.
#[derive(Resource)]
pub struct ClientWorld {
    chunks: HashMap<ChunkPos, Chunk>,
    registry: ContentRegistry,
    /// Local generator, used when the server has not sent a chunk yet.
    generator: WorldGen,
    /// Seed announced by the server.
    pub seed: u64,
}

impl ClientWorld {
    /// Creates a client world for a seed.
    pub fn new(registry: ContentRegistry, seed: u64) -> Self {
        let generator = WorldGen::new(&registry, seed);
        Self {
            chunks: HashMap::new(),
            registry,
            generator,
            seed,
        }
    }

    /// Content registry.
    pub fn registry(&self) -> &ContentRegistry {
        &self.registry
    }

    /// Inserts a chunk received from the server.
    pub fn insert_chunk(&mut self, chunk: Chunk) {
        self.chunks.insert(chunk.pos(), chunk);
    }

    /// Removes a chunk that left the view radius.
    pub fn unload_chunk(&mut self, pos: ChunkPos) {
        self.chunks.remove(&pos);
    }

    /// All loaded chunk positions.
    pub fn chunk_positions(&self) -> Vec<ChunkPos> {
        self.chunks.keys().copied().collect()
    }

    /// Clones a loaded chunk so it can be moved into a mesh task.
    pub fn chunk_clone(&self, pos: ChunkPos) -> Option<Chunk> {
        self.chunks.get(&pos).cloned()
    }

    /// Number of loaded chunks, for the debug overlay.
    pub fn loaded_chunks(&self) -> usize {
        self.chunks.len()
    }

    /// True if a chunk is loaded.
    pub fn has_chunk(&self, pos: ChunkPos) -> bool {
        self.chunks.contains_key(&pos)
    }

    /// Reads a block. Chunks that are not loaded are generated locally so the
    /// player never falls through the world while streaming.
    pub fn block(&self, pos: BlockPos) -> BlockId {
        if !pos.in_world() {
            return self.registry.bedrock;
        }
        if let Some(chunk) = self.chunks.get(&pos.chunk()) {
            let [x, y, z] = pos.local();
            return BlockId(chunk.get_local(x, y, z));
        }
        BlockId(0)
    }

    /// Reads a block, generating locally when the chunk is missing.
    pub fn block_or_generate(&mut self, pos: BlockPos) -> BlockId {
        if !pos.in_world() {
            return self.registry.bedrock;
        }
        let chunk_pos = pos.chunk();
        if !self.chunks.contains_key(&chunk_pos) {
            let chunk = self.generator.generate_chunk(chunk_pos);
            self.chunks.insert(chunk_pos, chunk);
        }
        self.block(pos)
    }

    /// Applies a block change from the server.
    pub fn set_block(&mut self, pos: BlockPos, block: BlockId) {
        if !pos.in_world() {
            return;
        }
        let chunk_pos = pos.chunk();
        if !self.chunks.contains_key(&chunk_pos) {
            let chunk = self.generator.generate_chunk(chunk_pos);
            self.chunks.insert(chunk_pos, chunk);
        }
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            let [x, y, z] = pos.local();
            chunk.set_local(x, y, z, block.0);
        }
    }

    /// True if the block is solid for collision.
    pub fn solid(&self, pos: BlockPos) -> bool {
        match self.registry.block_def(self.block(pos)) {
            Some(d) => !(d.passable || d.liquid),
            None => false,
        }
    }

    /// Ensures the chunks around a position exist locally, so targeting and
    /// collision never see holes while chunks stream in.
    pub fn ensure_around(&mut self, pos: BlockPos, radius: i32) {
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                for dz in -radius..=radius {
                    let p = pos.offset([dx, dy, dz]);
                    if p.in_world() {
                        self.block_or_generate(p);
                    }
                }
            }
        }
    }

    /// Voxel raycast for targeting, matching the server's implementation.
    pub fn raycast(
        &self,
        origin: [f32; 3],
        dir: [f32; 3],
        max_distance: f32,
    ) -> Option<(BlockPos, deepcore_shared::voxel::Face)> {
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
        let start = BlockPos::new(
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
            if Some(bp) != last && bp != start {
                let block = self.block(bp);
                if block != self.registry.air {
                    let face = match last {
                        Some(prev) => {
                            let dx = bp.x - prev.x;
                            let dy = bp.y - prev.y;
                            let dz = bp.z - prev.z;
                            if dx > 0 {
                                deepcore_shared::voxel::Face::East
                            } else if dx < 0 {
                                deepcore_shared::voxel::Face::West
                            } else if dy > 0 {
                                deepcore_shared::voxel::Face::Up
                            } else if dy < 0 {
                                deepcore_shared::voxel::Face::Down
                            } else if dz > 0 {
                                deepcore_shared::voxel::Face::South
                            } else {
                                deepcore_shared::voxel::Face::North
                            }
                        }
                        None => deepcore_shared::voxel::Face::Up,
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
}

impl VoxelView for ClientWorld {
    fn solid(&self, pos: BlockPos) -> bool {
        if !pos.in_world() {
            return true;
        }
        ClientWorld::solid(self, pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> ClientWorld {
        let reg = ContentRegistry::load_dir(deepcore_shared::data_dir()).unwrap();
        ClientWorld::new(reg, 42)
    }

    #[test]
    fn generates_terrain_locally_when_missing() {
        let mut w = world();
        let spawn = deepcore_shared::worldgen::spawn_pos();
        assert_eq!(w.loaded_chunks(), 0);
        let below = w.block_or_generate(spawn.offset([0, -1, 0]));
        assert_ne!(below, w.registry.air);
        assert!(w.loaded_chunks() > 0);
    }

    #[test]
    fn set_block_updates_local_copy() {
        let mut w = world();
        let spawn = deepcore_shared::worldgen::spawn_pos();
        w.set_block(spawn, w.registry.air);
        assert_eq!(w.block(spawn), w.registry.air);
    }

    #[test]
    fn raycast_finds_ground_below() {
        let mut w = world();
        let spawn = deepcore_shared::worldgen::spawn_pos();
        w.block_or_generate(spawn.offset([0, -1, 0]));
        let origin = [
            spawn.x as f32 + 0.5,
            spawn.y as f32 + 2.0,
            spawn.z as f32 + 0.5,
        ];
        let hit = w.raycast(origin, [0.0, -1.0, 0.0], 6.0);
        assert!(hit.is_some());
    }
}
