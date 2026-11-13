//! Chunk rendering: async meshing on the compute task pool, chunk entities and
//! the material used by every chunk mesh.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use deepcore_shared::voxel::{Chunk, ChunkPos};

use crate::render::atlas::AtlasLayout;
use crate::render::mesher::mesh_chunk;
use crate::world::ClientWorld;

/// Material shared by every chunk.
#[derive(Resource)]
pub struct ChunkMaterial(pub Handle<crate::render::VoxelMaterial>);

/// Marker for a chunk entity.
#[derive(Component)]
pub struct ChunkEntity(pub ChunkPos);

/// A mesh job running on the compute pool.
#[derive(Component)]
pub struct MeshJob {
    /// Chunk being meshed.
    pub pos: ChunkPos,
    /// The task.
    pub task: Task<Mesh>,
}

/// Data handed to the meshing task so it needs no world access.
pub struct MeshInput {
    /// The chunk.
    pub chunk: Chunk,
    /// Neighbouring border blocks keyed by this chunk's local coordinates
    /// (one step outside the chunk), for cross-chunk face culling.
    pub borders: Vec<((i32, i32, i32), u16)>,
    /// UV rects per block id.
    pub uvs: Vec<[f32; 4]>,
}

/// Starts async meshing for every loaded chunk that has no entity yet.
pub fn schedule_meshing(
    mut commands: Commands,
    world: Res<ClientWorld>,
    atlas: Res<AtlasLayout>,
    existing: Query<&ChunkEntity>,
    mut pending: Local<Vec<ChunkPos>>,
) {
    let have: std::collections::HashSet<ChunkPos> = existing.iter().map(|c| c.0).collect();
    let pool = AsyncComputeTaskPool::get();
    let uvs = atlas.uvs.clone();

    for pos in world.chunk_positions() {
        if have.contains(&pos) || pending.contains(&pos) {
            continue;
        }
        let Some(chunk) = world.chunk_clone(pos) else {
            continue;
        };
        // Neighbour border blocks, keyed by the coordinates the mesher asks
        // for: one step outside this chunk on the shared face, i.e. -1 or 32.
        let s = deepcore_shared::CHUNK_SIZE;
        let mut borders: Vec<((i32, i32, i32), u16)> = Vec::new();
        for (dx, dy, dz) in [
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ] {
            let npos = pos.offset([dx, dy, dz]);
            let Some(n) = world.chunk_clone(npos) else {
                continue;
            };
            // Our-space coordinate of the face touching the neighbour.
            let out_x = if dx > 0 {
                s
            } else if dx < 0 {
                -1
            } else {
                0
            };
            let out_y = if dy > 0 {
                s
            } else if dy < 0 {
                -1
            } else {
                0
            };
            let out_z = if dz > 0 {
                s
            } else if dz < 0 {
                -1
            } else {
                0
            };
            // The neighbour's local coordinate on that shared face.
            let in_x = if dx > 0 {
                0
            } else if dx < 0 {
                s - 1
            } else {
                0
            };
            let in_y = if dy > 0 {
                0
            } else if dy < 0 {
                s - 1
            } else {
                0
            };
            let in_z = if dz > 0 {
                0
            } else if dz < 0 {
                s - 1
            } else {
                0
            };
            for a in 0..s {
                for b in 0..s {
                    let (nx, ny, nz) = match (dx, dy, dz) {
                        (1, 0, 0) | (-1, 0, 0) => (in_x, a, b),
                        (0, 1, 0) | (0, -1, 0) => (a, in_y, b),
                        _ => (a, b, in_z),
                    };
                    let block = n.get_local(nx as usize, ny as usize, nz as usize);
                    let key = match (dx, dy, dz) {
                        (1, 0, 0) | (-1, 0, 0) => (out_x, a, b),
                        (0, 1, 0) | (0, -1, 0) => (a, out_y, b),
                        _ => (a, b, out_z),
                    };
                    borders.push((key, block));
                }
            }
        }
        let input = MeshInput {
            chunk,
            borders,
            uvs: uvs.clone(),
        };
        let task = pool.spawn(async move {
            let border_lookup: HashMap<(i32, i32, i32), u16> =
                input.borders.iter().copied().collect();
            let uvs = input.uvs.clone();
            mesh_chunk(
                &input.chunk,
                |x, y, z| border_lookup.get(&(x, y, z)).copied(),
                |block| {
                    uvs.get(block as usize)
                        .copied()
                        .unwrap_or([0.0, 0.0, 1.0, 1.0])
                },
            )
        });
        commands.spawn(MeshJob { pos, task });
        pending.push(pos);
    }
    pending.retain(|p| !have.contains(p));
}

/// Applies finished mesh jobs, spawning chunk entities.
pub fn apply_mesh_jobs(
    mut commands: Commands,
    mut jobs: Query<(Entity, &mut MeshJob)>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<ChunkMaterial>,
) {
    for (entity, mut job) in jobs.iter_mut() {
        if let Some(mesh) = block_on(poll_once(&mut job.task)) {
            let handle = meshes.add(mesh);
            let origin = job.pos.origin();
            commands.spawn((
                Mesh3d(handle),
                MeshMaterial3d(material.0.clone()),
                Transform::from_xyz(origin.x as f32, origin.y as f32, origin.z as f32),
                ChunkEntity(job.pos),
            ));
            commands.entity(entity).despawn();
        }
    }
}

/// Despawns chunk entities for chunks that are no longer loaded.
pub fn unload_chunks(
    mut commands: Commands,
    world: Res<ClientWorld>,
    chunks: Query<(Entity, &ChunkEntity)>,
) {
    for (entity, chunk) in chunks.iter() {
        if !world.has_chunk(chunk.0) {
            commands.entity(entity).despawn();
        }
    }
}
