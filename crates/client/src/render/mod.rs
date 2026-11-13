//! Rendering: procedural atlas, greedy meshing, async chunk streaming.

pub mod atlas;
pub mod chunks;
pub mod mesher;

use bevy::prelude::*;

use crate::world::ClientWorld;

/// Plugin that owns chunk meshing and the block material.
pub struct VoxelRenderPlugin;

impl Plugin for VoxelRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_atlas).add_systems(
            Update,
            (
                chunks::unload_chunks,
                chunks::schedule_meshing,
                chunks::apply_mesh_jobs,
            )
                .chain(),
        );
    }
}

/// Block material: the atlas texture with per-vertex shading.
///
/// Per-vertex shade is baked into the mesh colour attribute by the mesher, so
/// the standard PBR material renders it without a custom shader. Emissive
/// blocks are handled by a second material (see `emissive_material`).
pub type VoxelMaterial = StandardMaterial;

fn setup_atlas(
    mut commands: Commands,
    world: Res<ClientWorld>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let (image, layout) = atlas::build_atlas(world.registry());
    let handle = images.add(image);
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(handle),
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });
    commands.insert_resource(atlas::AtlasLayout {
        uvs: layout.uvs.clone(),
    });
    commands.insert_resource(chunks::ChunkMaterial(material));
}
