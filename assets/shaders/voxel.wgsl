//! Custom voxel shader: samples the atlas, multiplies by the per-vertex shade
//! baked into the mesh, and adds emissive blocks (uranium, lamps) on top.

#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_bindings::mesh

struct VoxelMaterial {
    emissive: vec4<f32>,
};

@group(2) @binding(0) var atlas_texture: texture_2d<f32>;
@group(2) @binding(1) var atlas_sampler: sampler;
@group(2) @binding(2) var<uniform> material: VoxelMaterial;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(atlas_texture, atlas_sampler, in.uv);
    // Per-vertex shade is stored in the colour attribute by the mesher.
    let shade = in.color.rgb;
    var out_color = tex.rgb * shade;
    // Emissive contribution keeps uranium and lamps readable in the dark.
    out_color = out_color + tex.rgb * material.emissive.rgb * 2.0;
    return vec4<f32>(out_color, tex.a);
}
