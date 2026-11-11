//! "Assar" o chão: uma imagem só por mapa (uma de dia, uma de noite), pintada pixel a
//! pixel pelo terreno orgânico (terrain.rs). Em vez de ~3 mil sprites de chão, o mapa
//! desenha 2 imagens; arar, regar ou abrir a caverna repintam só os tiles em volta.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::sprites::Art;

pub type Rgba = [u8; 4];

/// Os 16x16 pixels de um quadro de uma arte (None se ainda não carregou).
pub fn tile_pixels(
    images: &Assets<Image>,
    layouts: &Assets<TextureAtlasLayout>,
    art: &Art,
    night: bool,
    index: usize,
) -> Option<Vec<Rgba>> {
    let src = images.get(if night { &art.night } else { &art.day })?;
    let data = src.data.as_ref()?;
    let rect = art
        .layout
        .as_ref()
        .and_then(|h| layouts.get(h))
        .and_then(|l| l.textures.get(index).copied())
        .unwrap_or(URect::new(0, 0, src.width(), src.height()));
    let w = src.width();
    let mut out = Vec::with_capacity(256);
    for y in 0..16 {
        for x in 0..16 {
            let (sx, sy) = (rect.min.x + x % rect.width().max(1), rect.min.y + y % rect.height().max(1));
            let i = ((sy * w + sx) * 4) as usize;
            out.push([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        }
    }
    Some(out)
}

pub struct Painter<'a> {
    pub images: &'a mut Assets<Image>,
    pub layouts: &'a Assets<TextureAtlasLayout>,
}

impl Painter<'_> {
    /// Imagem transparente de `size` pixels.
    pub fn canvas(&mut self, size: UVec2) -> Handle<Image> {
        self.images.add(Image::new_fill(
            Extent3d { width: size.x.max(1), height: size.y.max(1), depth_or_array_layers: 1 },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ))
    }

    /// Escreve um bloco de pixels (largura `w`) em `dst`, com o canto em `at`.
    pub fn write_block(&mut self, dst: &Handle<Image>, at: UVec2, w: u32, pixels: &[Rgba]) {
        let Some(mut dst) = self.images.get_mut(dst) else { return };
        let (dw, dh) = (dst.width(), dst.height());
        let Some(data) = dst.data.as_mut() else { return };
        for (i, p) in pixels.iter().enumerate() {
            let (x, y) = (at.x + i as u32 % w, at.y + i as u32 / w);
            if x < dw && y < dh {
                let d = ((y * dw + x) * 4) as usize;
                data[d..d + 4].copy_from_slice(p);
            }
        }
    }

    /// Imagem nova a partir de pixels (camada da água).
    pub fn image(&mut self, size: UVec2, pixels: &[Rgba]) -> Handle<Image> {
        let handle = self.canvas(size);
        self.write_block(&handle, UVec2::ZERO, size.x, pixels);
        handle
    }
}
