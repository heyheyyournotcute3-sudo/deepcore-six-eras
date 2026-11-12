//! Chunk storage with palette compression.
//!
//! A chunk is `32 x 32 x 32` blocks. Every block is a `u16` block id. Most
//! chunks only contain a handful of distinct ids, so blocks are stored as an
//! index into a per-chunk palette. Indices are one byte while the palette fits
//! in 256 entries and two bytes afterwards, which keeps a uniform chunk at
//! roughly 32 KiB before zstd instead of 64 KiB.

use serde::{Deserialize, Serialize};

use super::pos::{BlockPos, ChunkPos};
use crate::CHUNK_SIZE;

/// Number of blocks in a chunk.
pub const CHUNK_VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;

/// Linear index of a local block coordinate inside a chunk.
#[inline]
pub const fn index(x: usize, y: usize, z: usize) -> usize {
    (y * CHUNK_SIZE as usize + z) * CHUNK_SIZE as usize + x
}

/// Palette-compressed block storage for one chunk.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chunk {
    pos: ChunkPos,
    palette: Vec<u16>,
    /// One or two bytes per block, depending on `wide`.
    indices: Vec<u8>,
    wide: bool,
    /// True once any block differs from the generated state.
    modified: bool,
}

impl Chunk {
    /// Creates an empty chunk filled with `fill`.
    pub fn filled(pos: ChunkPos, fill: u16) -> Self {
        Self {
            pos,
            palette: vec![fill],
            indices: vec![0; CHUNK_VOLUME],
            wide: false,
            modified: false,
        }
    }

    /// Builds a chunk from a full `u16` volume, deriving the palette.
    pub fn from_blocks(pos: ChunkPos, blocks: &[u16]) -> Self {
        debug_assert_eq!(blocks.len(), CHUNK_VOLUME);
        let mut chunk = Self {
            pos,
            palette: Vec::new(),
            indices: vec![0; CHUNK_VOLUME],
            wide: false,
            modified: false,
        };
        for (i, &b) in blocks.iter().enumerate() {
            let idx = chunk.palette_index_or_insert(b);
            chunk.set_raw(i, idx);
        }
        chunk.wide = chunk.palette.len() > u8::MAX as usize;
        chunk
    }

    /// Chunk coordinate.
    pub fn pos(&self) -> ChunkPos {
        self.pos
    }

    /// True if the chunk has been edited since generation.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Marks the chunk as edited (called by the server when a block changes).
    pub fn set_modified(&mut self, modified: bool) {
        self.modified = modified;
    }

    /// Number of distinct block ids in the palette.
    pub fn palette_len(&self) -> usize {
        self.palette.len()
    }

    /// The palette itself.
    pub fn palette(&self) -> &[u16] {
        &self.palette
    }

    fn palette_index_or_insert(&mut self, block: u16) -> u16 {
        if let Some(i) = self.palette.iter().position(|&b| b == block) {
            return i as u16;
        }
        self.palette.push(block);
        (self.palette.len() - 1) as u16
    }

    #[inline]
    fn get_raw(&self, i: usize) -> u16 {
        if self.wide {
            u16::from_le_bytes([self.indices[i * 2], self.indices[i * 2 + 1]])
        } else {
            self.indices[i] as u16
        }
    }

    #[inline]
    fn set_raw(&mut self, i: usize, value: u16) {
        if self.wide {
            let [lo, hi] = value.to_le_bytes();
            self.indices[i * 2] = lo;
            self.indices[i * 2 + 1] = hi;
        } else {
            self.indices[i] = value as u8;
        }
    }

    /// Reads a block by local coordinates. Out-of-range coordinates return 0.
    pub fn get_local(&self, x: usize, y: usize, z: usize) -> u16 {
        if x >= CHUNK_SIZE as usize || y >= CHUNK_SIZE as usize || z >= CHUNK_SIZE as usize {
            return 0;
        }
        let idx = self.get_raw(index(x, y, z));
        self.palette.get(idx as usize).copied().unwrap_or(0)
    }

    /// Writes a block by local coordinates. Returns false if out of range.
    pub fn set_local(&mut self, x: usize, y: usize, z: usize, block: u16) -> bool {
        if x >= CHUNK_SIZE as usize || y >= CHUNK_SIZE as usize || z >= CHUNK_SIZE as usize {
            return false;
        }
        let slot = index(x, y, z);
        let current = self.palette.get(self.get_raw(slot) as usize).copied();
        if current == Some(block) {
            return true;
        }
        let idx = self.palette_index_or_insert(block);
        if !self.wide && self.palette.len() > u8::MAX as usize {
            self.widen();
        }
        self.set_raw(slot, idx);
        true
    }

    fn widen(&mut self) {
        let old = std::mem::take(&mut self.indices);
        self.indices = vec![0; CHUNK_VOLUME * 2];
        self.wide = true;
        for (i, b) in old.into_iter().enumerate() {
            self.set_raw(i, b as u16);
        }
    }

    /// Packs the chunk for network transfer: palette followed by indices.
    pub fn to_wire(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + self.palette.len() * 2 + self.indices.len());
        out.extend_from_slice(&self.pos.x.to_le_bytes());
        out.extend_from_slice(&self.pos.y.to_le_bytes());
        out.extend_from_slice(&self.pos.z.to_le_bytes());
        out.push(self.wide as u8);
        out.extend_from_slice(&(self.palette.len() as u32).to_le_bytes());
        for b in &self.palette {
            out.extend_from_slice(&b.to_le_bytes());
        }
        out.extend_from_slice(&self.indices);
        out
    }

    /// Unpacks a chunk produced by [`Chunk::to_wire`].
    pub fn from_wire(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 17 {
            return None;
        }
        let rd = |o: usize| i32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
        let pos = ChunkPos::new(rd(0), rd(4), rd(8));
        let wide = bytes[12] != 0;
        let palette_len = u32::from_le_bytes([bytes[13], bytes[14], bytes[15], bytes[16]]) as usize;
        let mut off = 17;
        if palette_len > 4096 || bytes.len() < off + palette_len * 2 {
            return None;
        }
        let mut palette = Vec::with_capacity(palette_len);
        for _ in 0..palette_len {
            palette.push(u16::from_le_bytes([bytes[off], bytes[off + 1]]));
            off += 2;
        }
        let index_bytes = CHUNK_VOLUME * if wide { 2 } else { 1 };
        if bytes.len() < off + index_bytes {
            return None;
        }
        Some(Self {
            pos,
            palette,
            indices: bytes[off..off + index_bytes].to_vec(),
            wide,
            modified: false,
        })
    }

    /// Iterates every block of the chunk with its local coordinates.
    pub fn iter_blocks(&self) -> impl Iterator<Item = ([usize; 3], u16)> + '_ {
        (0..CHUNK_VOLUME).map(move |i| {
            let x = i % CHUNK_SIZE as usize;
            let z = (i / CHUNK_SIZE as usize) % CHUNK_SIZE as usize;
            let y = i / (CHUNK_SIZE as usize * CHUNK_SIZE as usize);
            let idx = self.get_raw(i);
            ([x, y, z], self.palette.get(idx as usize).copied().unwrap_or(0))
        })
    }
}

/// A sparse set of block edits, used for saving only modified chunks.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BlockEdits {
    /// The edits, in application order.
    pub edits: Vec<(BlockPos, u16)>,
}

impl BlockEdits {
    /// Records an edit.
    pub fn push(&mut self, pos: BlockPos, block: u16) {
        self.edits.push((pos, block));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_chunk_roundtrip() {
        let chunk = Chunk::filled(ChunkPos::new(1, 2, 3), 7);
        assert_eq!(chunk.palette_len(), 1);
        for (_, b) in chunk.iter_blocks() {
            assert_eq!(b, 7);
        }
    }

    #[test]
    fn set_and_get_preserves_values() {
        let mut chunk = Chunk::filled(ChunkPos::new(0, 0, 0), 1);
        assert!(chunk.set_local(0, 0, 0, 42));
        assert!(chunk.set_local(31, 31, 31, 99));
        assert!(chunk.set_local(5, 9, 17, 1234));
        assert_eq!(chunk.get_local(0, 0, 0), 42);
        assert_eq!(chunk.get_local(31, 31, 31), 99);
        assert_eq!(chunk.get_local(5, 9, 17), 1234);
        assert_eq!(chunk.get_local(1, 0, 0), 1);
        assert!(!chunk.set_local(32, 0, 0, 5));
    }

    #[test]
    fn palette_grows_past_255_entries() {
        let mut chunk = Chunk::filled(ChunkPos::new(0, 0, 0), 0);
        for i in 0..300u16 {
            assert!(chunk.set_local(i as usize % 32, i as usize / 32, 0, i + 1));
        }
        assert!(chunk.palette_len() > 255);
        for i in 0..300u16 {
            assert_eq!(chunk.get_local(i as usize % 32, i as usize / 32, 0), i + 1);
        }
    }

    #[test]
    fn wire_roundtrip() {
        let mut chunk = Chunk::filled(ChunkPos::new(-3, 5, 7), 3);
        chunk.set_local(1, 2, 3, 500);
        chunk.set_local(4, 5, 6, 501);
        let bytes = chunk.to_wire();
        let back = Chunk::from_wire(&bytes).expect("valid wire chunk");
        assert_eq!(back.pos(), chunk.pos());
        assert_eq!(back.get_local(1, 2, 3), 500);
        assert_eq!(back.get_local(4, 5, 6), 501);
        assert_eq!(back.get_local(0, 0, 0), 3);
    }

    #[test]
    fn wire_rejects_truncated_input() {
        assert!(Chunk::from_wire(&[0, 1, 2]).is_none());
    }

    #[test]
    fn index_matches_iteration_order() {
        assert_eq!(index(0, 0, 0), 0);
        assert_eq!(index(1, 0, 0), 1);
        assert_eq!(index(0, 0, 1), 32);
        assert_eq!(index(0, 1, 0), 1024);
    }
}
