//! Greedy chunk meshing.
//!
//! Each 32x32x32 chunk is meshed into one vertex/index buffer. Faces between
//! two solid blocks are culled, including across chunk borders. Quads are
//! merged along their longest axis when the neighbouring block is identical,
//! which is the classic greedy meshing pass and keeps big stone volumes cheap.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use deepcore_shared::voxel::Chunk;

/// One vertex of the chunk mesh.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VoxelVertex {
    /// Position.
    pub position: [f32; 3],
    /// Normal.
    pub normal: [f32; 3],
    /// Atlas UV.
    pub uv: [f32; 2],
    /// Per-face brightness for cheap ambient occlusion.
    pub shade: f32,
}

/// Builds a Bevy mesh from a chunk, sampling `solid` and `uv` callbacks.
pub fn mesh_chunk<F, U>(chunk: &Chunk, mut solid: F, uv: U) -> Mesh
where
    F: FnMut(i32, i32, i32) -> Option<u16>,
    U: Fn(u16) -> [f32; 4],
{
    let origin = chunk.pos().origin();
    let size = deepcore_shared::CHUNK_SIZE;
    let mut verts: Vec<VoxelVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Local block lookup with a cache of the chunk's own data.
    let local = |x: i32, y: i32, z: i32| -> u16 {
        if (0..size).contains(&x) && (0..size).contains(&y) && (0..size).contains(&z) {
            chunk.get_local(x as usize, y as usize, z as usize)
        } else {
            0
        }
    };

    // Six face directions: (normal, u axis, v axis, shade).
    type FaceSpec = ([i32; 3], [i32; 3], [i32; 3], f32);
    const FACES: [FaceSpec; 6] = [
        ([1, 0, 0], [0, 0, 1], [0, 1, 0], 0.82),
        ([-1, 0, 0], [0, 0, -1], [0, 1, 0], 0.82),
        ([0, 1, 0], [1, 0, 0], [0, 0, 1], 1.0),
        ([0, -1, 0], [1, 0, 0], [0, 0, -1], 0.62),
        ([0, 0, 1], [-1, 0, 0], [0, 1, 0], 0.72),
        ([0, 0, -1], [1, 0, 0], [0, 1, 0], 0.72),
    ];

    for (face, (normal, u_axis, v_axis, shade)) in FACES.iter().enumerate() {
        // For each slice along the face normal, run the greedy merge.
        for slice in 0..size {
            let mut mask: Vec<Option<u16>> = vec![None; (size * size) as usize];
            for v in 0..size {
                for u in 0..size {
                    // Convert (u, v, slice) into block coordinates.
                    let mut p = [0i32; 3];
                    for axis in 0..3 {
                        if normal[axis] != 0 {
                            p[axis] = slice;
                        } else if u_axis[axis] != 0 {
                            p[axis] = u * u_axis[axis].abs();
                        } else {
                            p[axis] = v * v_axis[axis].abs();
                        }
                    }
                    let here = local(p[0], p[1], p[2]);
                    if here == 0 {
                        continue;
                    }
                    let np = [p[0] + normal[0], p[1] + normal[1], p[2] + normal[2]];
                    let world = [origin.x + np[0], origin.y + np[1], origin.z + np[2]];
                    let neighbour = if (0..size).contains(&np[0])
                        && (0..size).contains(&np[1])
                        && (0..size).contains(&np[2])
                    {
                        local(np[0], np[1], np[2])
                    } else {
                        solid(world[0], world[1], world[2]).unwrap_or(0)
                    };
                    if neighbour == 0 {
                        mask[(v * size + u) as usize] = Some(here);
                    }
                }
            }

            // Greedy merge the mask into quads.
            let mut v = 0;
            while v < size {
                let mut u = 0;
                while u < size {
                    let Some(block) = mask[(v * size + u) as usize] else {
                        u += 1;
                        continue;
                    };
                    // Grow along u.
                    let mut width = 1;
                    while u + width < size && mask[(v * size + u + width) as usize] == Some(block) {
                        width += 1;
                    }
                    // Grow along v.
                    let mut height = 1;
                    'outer: while v + height < size {
                        for k in 0..width {
                            if mask[((v + height) * size + u + k) as usize] != Some(block) {
                                break 'outer;
                            }
                        }
                        height += 1;
                    }

                    push_quad(
                        &mut verts,
                        &mut indices,
                        [u, v, slice],
                        width,
                        height,
                        *normal,
                        *u_axis,
                        *v_axis,
                        *shade,
                        uv(block),
                        face,
                    );

                    for dv in 0..height {
                        for du in 0..width {
                            mask[((v + dv) * size + u + du) as usize] = None;
                        }
                    }
                    u += width;
                }
                v += 1;
            }
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        verts.iter().map(|v| v.position).collect::<Vec<_>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        verts.iter().map(|v| v.normal).collect::<Vec<_>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        verts.iter().map(|v| v.uv).collect::<Vec<_>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_COLOR,
        verts
            .iter()
            .map(|v| [v.shade, v.shade, v.shade, 1.0f32])
            .collect::<Vec<_>>(),
    );
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[allow(clippy::too_many_arguments)]
fn push_quad(
    verts: &mut Vec<VoxelVertex>,
    indices: &mut Vec<u32>,
    base: [i32; 3],
    width: i32,
    height: i32,
    normal: [i32; 3],
    u_axis: [i32; 3],
    v_axis: [i32; 3],
    shade: f32,
    uv_rect: [f32; 4],
    face: usize,
) {
    // A quad covers the block plane at `base`, extruded to the face side.
    //
    // The four corners are walked in (u, v) order. Along an axis that points
    // negative, growing in that direction moves the corner *down*, so the
    // starting offset must be the far edge instead of zero. Getting this
    // wrong emits vertices outside the block and produces the smeared
    // geometry that the position test guards against.
    let corner = |du: i32, dv: i32| -> [f32; 3] {
        let mut p = [base[0] as f32, base[1] as f32, base[2] as f32];
        for axis in 0..3 {
            // Offset to the face plane.
            if normal[axis] > 0 {
                p[axis] += 1.0;
            }
            if u_axis[axis] != 0 {
                if u_axis[axis] > 0 {
                    p[axis] += du as f32;
                } else {
                    p[axis] += (width - du) as f32;
                }
            }
            if v_axis[axis] != 0 {
                if v_axis[axis] > 0 {
                    p[axis] += dv as f32;
                } else {
                    p[axis] += (height - dv) as f32;
                }
            }
        }
        p
    };

    let [u0, v0, u1, v1] = uv_rect;
    // Texture tiles repeat across merged quads so texel density stays even.
    let uv_for = |du: i32, dv: i32| -> [f32; 2] {
        let fu = du as f32;
        let fv = dv as f32;
        [u0 + (u1 - u0) * fu, v0 + (v1 - v0) * fv]
    };

    let positions = [
        corner(0, 0),
        corner(width, 0),
        corner(width, height),
        corner(0, height),
    ];
    let uvs = [
        uv_for(0, 0),
        uv_for(width, 0),
        uv_for(width, height),
        uv_for(0, height),
    ];

    let start = verts.len() as u32;
    for i in 0..4 {
        verts.push(VoxelVertex {
            position: positions[i],
            normal: normal.map(|n| n as f32),
            uv: uvs[i],
            shade,
        });
    }
    // Winding order decides which side of the quad is visible. Bevy treats
    // counter-clockwise (when seen from the front) as front-facing, so the
    // order is chosen per face from the cross product of its (u, v) axes:
    // u x v points along the normal for +X, +Y and -Z, and against it for the
    // mirrored faces, which must therefore be wound the other way.
    let u = Vec3::new(u_axis[0] as f32, u_axis[1] as f32, u_axis[2] as f32);
    let v = Vec3::new(v_axis[0] as f32, v_axis[1] as f32, v_axis[2] as f32);
    let n = Vec3::new(normal[0] as f32, normal[1] as f32, normal[2] as f32);
    let ccw_from_outside = u.cross(v).dot(n) > 0.0;
    let _ = face;
    if ccw_from_outside {
        indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
    } else {
        indices.extend_from_slice(&[start, start + 2, start + 1, start, start + 3, start + 2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deepcore_shared::voxel::{Chunk, ChunkPos};

    #[test]
    fn empty_chunk_produces_no_geometry() {
        let chunk = Chunk::filled(ChunkPos::new(0, 0, 0), 0);
        let mesh = mesh_chunk(&chunk, |_, _, _| Some(0), |_| [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(mesh.count_vertices(), 0);
    }

    #[test]
    fn single_block_has_six_faces() {
        let mut chunk = Chunk::filled(ChunkPos::new(0, 0, 0), 0);
        chunk.set_local(5, 5, 5, 3);
        let mesh = mesh_chunk(&chunk, |_, _, _| Some(0), |_| [0.0, 0.0, 1.0, 1.0]);
        // 6 faces x 4 vertices, 6 x 6 indices.
        assert_eq!(mesh.count_vertices(), 24);
    }

    #[test]
    fn adjacent_blocks_cull_inner_faces() {
        let mut chunk = Chunk::filled(ChunkPos::new(0, 0, 0), 0);
        chunk.set_local(5, 5, 5, 3);
        chunk.set_local(6, 5, 5, 3);
        let mesh = mesh_chunk(&chunk, |_, _, _| Some(0), |_| [0.0, 0.0, 1.0, 1.0]);
        // The shared face is culled, so fewer than 12 quads remain, and the
        // merged top face means far fewer than 10 either.
        assert!(mesh.count_vertices() > 0);
        assert!(mesh.count_vertices() < 10 * 4);
    }

    #[test]
    fn vertex_positions_stay_inside_the_chunk() {
        let mut chunk = Chunk::filled(ChunkPos::new(0, 0, 0), 0);
        chunk.set_local(5, 5, 5, 3);
        let mesh = mesh_chunk(&chunk, |_, _, _| Some(0), |_| [0.0, 0.0, 1.0, 1.0]);
        let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions missing");
        };
        for p in positions {
            for (i, v) in p.iter().enumerate() {
                assert!(
                    *v >= 5.0 && *v <= 6.0,
                    "vertex component {i} = {v} outside the block at 5..6"
                );
            }
        }
    }

    #[test]
    fn uvs_stay_inside_the_atlas_tile() {
        let mut chunk = Chunk::filled(ChunkPos::new(0, 0, 0), 0);
        chunk.set_local(5, 5, 5, 3);
        // Tile 3 of a 16x16 atlas: u 0.1875..0.25.
        let mesh = mesh_chunk(&chunk, |_, _, _| Some(0), |_| [0.1875, 0.0, 0.25, 0.0625]);
        let Some(bevy::render::mesh::VertexAttributeValues::Float32x2(uvs)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("uvs missing");
        };
        for uv in uvs {
            assert!(
                uv[0] >= 0.1875 - 1e-5 && uv[0] <= 0.25 + 1e-5,
                "u={}",
                uv[0]
            );
            assert!(uv[1] >= -1e-5 && uv[1] <= 0.0625 + 1e-5, "v={}", uv[1]);
        }
    }

    #[test]
    fn a_flat_floor_of_blocks_has_one_quad_per_block() {
        // A 32x1x32 floor: top faces merge into a single quad, and the sides
        // and bottom merge into one quad each. Total 6 quads = 24 vertices.
        let mut chunk = Chunk::filled(ChunkPos::new(0, 0, 0), 0);
        for x in 0..32 {
            for z in 0..32 {
                chunk.set_local(x, 0, z, 4);
            }
        }
        let mesh = mesh_chunk(&chunk, |_, _, _| Some(0), |_| [0.0, 0.0, 0.1, 0.1]);
        assert_eq!(mesh.count_vertices(), 6 * 4, "expected fully merged floor");
    }

    #[test]
    fn full_solid_neighbour_culls_the_shared_face() {
        // A solid chunk whose top neighbour is solid must not emit its top
        // face: 5 quads remain.
        let chunk = Chunk::filled(ChunkPos::new(0, 0, 0), 5);
        let mesh = mesh_chunk(
            &chunk,
            |_, y, _| Some(if y >= 32 { 5 } else { 0 }),
            |_| [0.0, 0.0, 0.1, 0.1],
        );
        assert_eq!(mesh.count_vertices(), 5 * 4);
    }

    #[test]
    fn every_triangle_winds_outward() {
        // For each face of a single block, the triangle normal computed from
        // its winding must point the same way as the face normal, otherwise
        // the face is backface-culled and the block looks hollow.
        let mut chunk = Chunk::filled(ChunkPos::new(0, 0, 0), 0);
        chunk.set_local(5, 5, 5, 3);
        let mesh = mesh_chunk(&chunk, |_, _, _| Some(0), |_| [0.0, 0.0, 1.0, 1.0]);
        let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(pos)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("no positions");
        };
        let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(norm)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("no normals");
        };
        let Some(bevy::render::mesh::Indices::U32(idx)) = mesh.indices() else {
            panic!("no indices");
        };
        let mut checked = 0;
        for tri in idx.chunks(3) {
            let a = Vec3::from_array(pos[tri[0] as usize]);
            let b = Vec3::from_array(pos[tri[1] as usize]);
            let c = Vec3::from_array(pos[tri[2] as usize]);
            let winding = (b - a).cross(c - a).normalize_or_zero();
            let expected = Vec3::from_array(norm[tri[0] as usize]).normalize_or_zero();
            assert!(
                winding.dot(expected) > 0.9,
                "triangle winding {winding:?} does not match normal {expected:?}"
            );
            checked += 1;
        }
        assert!(checked > 0);
    }

    #[test]
    fn solid_world_merges_into_one_quad_per_face() {
        let chunk = Chunk::filled(ChunkPos::new(0, 0, 0), 5);
        // A fully solid chunk with air neighbours is 6 merged quads.
        let mesh = mesh_chunk(&chunk, |_, _, _| Some(0), |_| [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(mesh.count_vertices(), 6 * 4);
    }
}
