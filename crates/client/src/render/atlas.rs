//! Procedural texture atlas generated from `blocks.ron`, so the game ships
//! with zero downloaded art.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use deepcore_shared::content::{ContentRegistry, Pattern};

/// Tile size in pixels.
pub const TILE: u32 = 16;
/// Tiles per atlas row.
pub const ATLAS_COLS: u32 = 16;

/// Where each block lives in the atlas.
#[derive(Resource, Clone, Default)]
pub struct AtlasLayout {
    /// UV rect `(u0, v0, u1, v1)` per block id.
    pub uvs: Vec<[f32; 4]>,
}

/// Builds the atlas image and layout from the content registry.
pub fn build_atlas(reg: &ContentRegistry) -> (Image, AtlasLayout) {
    let count = reg.blocks().len().max(1) as u32;
    let rows = count.div_ceil(ATLAS_COLS).max(1);
    let width = ATLAS_COLS * TILE;
    let height = rows * TILE;
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let mut layout = AtlasLayout {
        uvs: vec![[0.0; 4]; count as usize],
    };

    for (i, def) in reg.blocks().iter().enumerate() {
        let tile = i as u32;
        let tx = (tile % ATLAS_COLS) * TILE;
        let ty = (tile / ATLAS_COLS) * TILE;
        let mut rng = SimpleRng::new(0x9E37_79B9 ^ (i as u32).wrapping_mul(2654435761));

        for py in 0..TILE {
            for px in 0..TILE {
                let (r, g, b) = shade(def, px, py, &mut rng);
                let o = (((ty + py) * width + tx + px) * 4) as usize;
                pixels[o] = (r.clamp(0.0, 1.0) * 255.0) as u8;
                pixels[o + 1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
                pixels[o + 2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
                pixels[o + 3] = 255;
            }
        }

        // UVs: Bevy textures have v pointing down.
        let u0 = tx as f32 / width as f32;
        let u1 = (tx + TILE) as f32 / width as f32;
        let v0 = ty as f32 / height as f32;
        let v1 = (ty + TILE) as f32 / height as f32;
        layout.uvs[i] = [u0, v0, u1, v1];
    }

    let image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    (image, layout)
}

fn shade(
    def: &deepcore_shared::content::BlockDef,
    px: u32,
    py: u32,
    rng: &mut SimpleRng,
) -> (f32, f32, f32) {
    let (br, bg, bb) = def.color;
    match def.pattern {
        Pattern::Solid => (br, bg, bb),
        Pattern::Speckle => {
            let n = rng.next_f32() * 0.22 - 0.11;
            (br + n, bg + n, bb + n)
        }
        Pattern::Bands => {
            let band = if (py / 4).is_multiple_of(2) { 0.06 } else { -0.06 };
            let n = rng.next_f32() * 0.06 - 0.03;
            (br + band + n, bg + band + n, bb + band + n)
        }
        Pattern::OreFleck((fr, fg, fb)) => {
            // Bright flecks clustered in a few spots on the tile.
            let h = ((px * 7 + py * 13 + (rng.next_u32() % 5)) % 11) as f32 / 11.0;
            if h > 0.72 {
                (fr, fg, fb)
            } else {
                let n = rng.next_f32() * 0.16 - 0.08;
                (br + n, bg + n, bb + n)
            }
        }
        Pattern::Glow => {
            let d = ((px as f32 - 8.0).powi(2) + (py as f32 - 8.0).powi(2)).sqrt() / 11.3;
            let k = (1.0 - d).clamp(0.0, 1.0);
            (br * 0.6 + k * 0.9, bg * 0.6 + k * 0.9, bb * 0.6 + k * 1.0)
        }
    }
}

/// Tiny deterministic PRNG for texture noise; not cryptographic.
pub struct SimpleRng(u32);

impl SimpleRng {
    /// Seeds the generator.
    pub fn new(seed: u32) -> Self {
        Self(seed | 1)
    }
    /// Next raw value.
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// Next float in `0..1`.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_deterministic() {
        let mut a = SimpleRng::new(7);
        let mut b = SimpleRng::new(7);
        for _ in 0..10 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn rng_floats_in_range() {
        let mut r = SimpleRng::new(99);
        for _ in 0..1000 {
            let f = r.next_f32();
            assert!((0.0..1.0).contains(&f));
        }
    }
}
