//! Chunk and block coordinates plus the world-space conversions used by both
//! the server and the client.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{CHUNK_SIZE, WORLD_HEIGHT, WORLD_SIZE};

/// A position in the voxel grid.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize, Component)]
pub struct BlockPos {
    /// Grid X.
    pub x: i32,
    /// Grid Y (0 = bedrock, increases upward).
    pub y: i32,
    /// Grid Z.
    pub z: i32,
}

impl BlockPos {
    /// Creates a block position.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// True if the position is inside the world bounds.
    pub const fn in_world(self) -> bool {
        self.x >= 0
            && self.x < WORLD_SIZE
            && self.z >= 0
            && self.z < WORLD_SIZE
            && self.y >= 0
            && self.y < WORLD_HEIGHT
    }

    /// Chunk containing this block.
    pub const fn chunk(self) -> ChunkPos {
        ChunkPos {
            x: self.x.div_euclid(CHUNK_SIZE),
            y: self.y.div_euclid(CHUNK_SIZE),
            z: self.z.div_euclid(CHUNK_SIZE),
        }
    }

    /// Position relative to the containing chunk (each axis in `0..CHUNK_SIZE`).
    pub const fn local(self) -> [usize; 3] {
        [
            self.x.rem_euclid(CHUNK_SIZE) as usize,
            self.y.rem_euclid(CHUNK_SIZE) as usize,
            self.z.rem_euclid(CHUNK_SIZE) as usize,
        ]
    }

    /// Center of the block in world space.
    pub fn center(self) -> Vec3 {
        Vec3::new(self.x as f32 + 0.5, self.y as f32 + 0.5, self.z as f32 + 0.5)
    }

    /// Offset by a delta.
    pub const fn offset(self, d: [i32; 3]) -> Self {
        Self {
            x: self.x + d[0],
            y: self.y + d[1],
            z: self.z + d[2],
        }
    }

    /// Squared distance between two block positions.
    pub fn distance_squared(self, other: Self) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        let dz = (self.z - other.z) as f32;
        dx * dx + dy * dy + dz * dz
    }
}

impl std::fmt::Display for BlockPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// A chunk coordinate.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChunkPos {
    /// Chunk X.
    pub x: i32,
    /// Chunk Y.
    pub y: i32,
    /// Chunk Z.
    pub z: i32,
}

impl ChunkPos {
    /// Creates a chunk position.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Chunk containing the given block position.
    pub fn of_block(pos: BlockPos) -> Self {
        pos.chunk()
    }

    /// Origin block of this chunk (minimum corner).
    pub const fn origin(self) -> BlockPos {
        BlockPos::new(self.x * CHUNK_SIZE, self.y * CHUNK_SIZE, self.z * CHUNK_SIZE)
    }

    /// Neighbouring chunk in the given direction.
    pub const fn offset(self, d: [i32; 3]) -> Self {
        Self {
            x: self.x + d[0],
            y: self.y + d[1],
            z: self.z + d[2],
        }
    }

    /// Chebyshev-ish distance used for interest management ordering.
    pub fn distance_to(self, other: Self) -> i32 {
        (self.x - other.x).abs().max((self.y - other.y).abs()).max((self.z - other.z).abs())
    }
}

/// Direction of a cube face, used for placing blocks and for mesh culling.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Face {
    /// +X
    East,
    /// -X
    West,
    /// +Y
    Up,
    /// -Y
    Down,
    /// +Z
    South,
    /// -Z
    North,
}

impl Face {
    /// Unit offset of the neighbouring block on this face.
    pub const fn offset(self) -> [i32; 3] {
        match self {
            Face::East => [1, 0, 0],
            Face::West => [-1, 0, 0],
            Face::Up => [0, 1, 0],
            Face::Down => [0, -1, 0],
            Face::South => [0, 0, 1],
            Face::North => [0, 0, -1],
        }
    }

    /// All six faces.
    pub const ALL: [Face; 6] = [
        Face::East,
        Face::West,
        Face::Up,
        Face::Down,
        Face::South,
        Face::North,
    ];
}

/// Converts a float world position to the block containing it.
pub fn world_to_block(p: Vec3) -> BlockPos {
    BlockPos::new(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_and_local_roundtrip() {
        for (x, y, z) in [(0, 0, 0), (31, 31, 31), (32, 33, 64), (1000, 511, 700)] {
            let pos = BlockPos::new(x, y, z);
            let chunk = pos.chunk();
            let local = pos.local();
            let rebuilt = BlockPos::new(
                chunk.x * CHUNK_SIZE + local[0] as i32,
                chunk.y * CHUNK_SIZE + local[1] as i32,
                chunk.z * CHUNK_SIZE + local[2] as i32,
            );
            assert_eq!(pos, rebuilt);
        }
    }

    #[test]
    fn negative_coordinates_use_euclid() {
        let pos = BlockPos::new(-1, -33, -32);
        assert_eq!(pos.chunk(), ChunkPos::new(-1, -2, -1));
        assert_eq!(pos.local(), [31, 31, 0]);
    }

    #[test]
    fn face_offsets_are_unit() {
        for f in Face::ALL {
            let o = f.offset();
            assert_eq!(o[0].abs() + o[1].abs() + o[2].abs(), 1);
        }
    }
}
