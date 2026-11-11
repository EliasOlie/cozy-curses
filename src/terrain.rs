//! Terreno orgânico: caminhos, água e canteiros com bordas curvas, como nas imagens de
//! inspiração (nada de grade aparente).
//!
//! Para cada pixel, cada terreno "misturável" tem um campo suave: a interpolação dos
//! tiles vizinhos (1 = é desse terreno) somada a um ruído leve. Onde o campo passa de 0,5
//! o pixel é daquele terreno; perto da borda ganha um friso (sombra, margem, espuma).
//! As texturas continuam sendo os tiles do pixelwind, amostrados pixel a pixel.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::bake::{Rgba, tile_pixels};
use crate::cycle::Season;
use crate::map::{Ground, TILE};
use crate::sprites::Art;

/// Terrenos com borda orgânica, do de cima para o de baixo.
const LAYERS: [Ground; 3] = [Ground::Water, Ground::Path, Ground::Soil];

/// Quanto o ruído entorta a borda (0 = borda redonda e lisa).
fn wobble(g: Ground) -> f32 {
    match g {
        Ground::Water => 0.28,
        Ground::Path => 0.32,
        _ => 0.1,
    }
}

/// Ruído de valor suave (estável por posição), em -0.5..0.5.
fn noise(u: f32, v: f32) -> f32 {
    let hash = |x: i32, y: i32| {
        let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77) ^ 0x5bd1_e995;
        h ^= h >> 15;
        h = h.wrapping_mul(0x2C1B_3C6D);
        h ^= h >> 12;
        (h & 0xFFFF) as f32 / 65536.0 - 0.5
    };
    // Duas oitavas: ondulação larga + rugosidade fina.
    let octave = |u: f32, v: f32| {
        let (x, y) = (u.floor() as i32, v.floor() as i32);
        let (fx, fy) = (smooth(u.fract()), smooth(v.fract()));
        let top = lerp(hash(x, y), hash(x + 1, y), fx);
        let bottom = lerp(hash(x, y + 1), hash(x + 1, y + 1), fx);
        lerp(top, bottom, fy)
    };
    octave(u * 1.6, v * 1.6) * 0.7 + octave(u * 4.0 + 17.0, v * 4.0 - 9.0) * 0.3
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Campo do terreno `g` no ponto (u, v) em unidades de tile.
pub fn field(ground: &impl Fn(IVec2) -> Option<Ground>, g: Ground, u: f32, v: f32) -> f32 {
    let (a, b) = ((u - 0.5).floor() as i32, (v - 0.5).floor() as i32);
    let (fx, fy) = (smooth(u - 0.5 - a as f32), smooth(v - 0.5 - b as f32));
    let is = |x: i32, y: i32| if ground(IVec2::new(x, y)) == Some(g) { 1.0 } else { 0.0 };
    let top = lerp(is(a, b), is(a + 1, b), fx);
    let bottom = lerp(is(a, b + 1), is(a + 1, b + 1), fx);
    lerp(top, bottom, fy) + noise(u, v) * wobble(g)
}

fn shade(c: Rgba, k: f32) -> Rgba {
    [(c[0] as f32 * k) as u8, (c[1] as f32 * k) as u8, (c[2] as f32 * k) as u8, c[3]]
}

fn tint(c: Rgba, t: [f32; 3]) -> Rgba {
    [(c[0] as f32 * t[0]) as u8, (c[1] as f32 * t[1]) as u8, (c[2] as f32 * t[2]) as u8, c[3]]
}

fn lighten(c: Rgba, k: f32) -> Rgba {
    let up = |v: u8| (v as f32 + (255.0 - v as f32) * k) as u8;
    [up(c[0]), up(c[1]), up(c[2]), c[3]]
}

/// Texturas 16x16 dos tiles, lidas uma vez por montagem.
pub struct TexCache<'p> {
    images: &'p Assets<Image>,
    layouts: &'p Assets<TextureAtlasLayout>,
    night: bool,
    /// Todas as artes moram no mesmo atlas: quem distingue uma da outra é o layout.
    tiles: HashMap<(Option<AssetId<TextureAtlasLayout>>, usize), Option<Vec<Rgba>>>,
}

impl<'p> TexCache<'p> {
    pub fn new(images: &'p Assets<Image>, layouts: &'p Assets<TextureAtlasLayout>, night: bool) -> Self {
        TexCache { images, layouts, night, tiles: HashMap::new() }
    }

    fn texel(&mut self, art: &Art, index: usize, x: u32, y: u32) -> Rgba {
        let key = (art.layout.as_ref().map(|l| l.id()), index);
        let (images, layouts, night) = (self.images, self.layouts, self.night);
        let tile = self.tiles.entry(key).or_insert_with(|| tile_pixels(images, layouts, art, night, index));
        match tile {
            Some(px) => px[((y % TILE as u32) * TILE as u32 + (x % TILE as u32)) as usize],
            None => [255, 0, 255, 255],
        }
    }
}

/// Modulação de cor sazonal para o chão e corpos d'água.
pub fn apply_season_ground(c: Rgba, g: Ground, season: Season) -> Rgba {
    match g {
        Ground::Grass => match season {
            Season::Spring => tint(c, [1.0, 1.05, 1.0]),
            Season::Summer => tint(c, [1.04, 1.02, 0.93]),
            Season::Autumn => tint(c, [1.22, 0.94, 0.58]),
            Season::Winter => lighten(tint(c, [0.86, 0.94, 1.15]), 0.16),
        },
        Ground::Water => match season {
            Season::Winter => tint(c, [0.85, 0.92, 1.15]),
            Season::Autumn => tint(c, [0.95, 0.92, 0.98]),
            _ => c,
        },
        _ => c,
    }
}

/// Cor de um pixel do chão. `texture(g, tile)` diz qual arte usar para o terreno no tile;
/// `base` é o chão "de fundo" do tile (grama sob caminhos e lagos).
#[allow(clippy::too_many_arguments)]
pub fn pixel(
    cache: &mut TexCache,
    ground: &impl Fn(IVec2) -> Option<Ground>,
    texture: &impl Fn(Ground, IVec2) -> (Art, usize),
    base: Ground,
    t: IVec2,
    px: u32,
    py: u32,
    wet: bool,
    season: Season,
) -> Rgba {
    let u = t.x as f32 + (px as f32 + 0.5) / TILE;
    let v = t.y as f32 + (py as f32 + 0.5) / TILE;
    for g in LAYERS {
        // Só calcula o campo se o terreno estiver por perto (economia).
        let near = (-1..=1).any(|dy| (-1..=1).any(|dx| ground(t + IVec2::new(dx, dy)) == Some(g)));
        if !near {
            continue;
        }
        let f = field(ground, g, u, v);
        if f >= 0.5 {
            let (art, index) = texture(g, t);
            let mut c = cache.texel(&art, index, px, py);
            match g {
                // Friso escuro na beira do caminho e do canteiro.
                Ground::Path if f < 0.6 => c = shade(c, 0.82),
                Ground::Soil if f < 0.58 => c = shade(c, 0.7),
                // Margem clara (espuma parada; a animada fica na camada da água).
                Ground::Water if f < 0.56 => c = lighten(c, 0.35),
                _ => {}
            }
            if g == Ground::Soil && wet {
                c = tint(c, WET);
            }
            if g == Ground::Water {
                c = apply_season_ground(c, Ground::Water, season);
            }
            return c;
        }
        // Do lado de fora da borda: sombrinha na grama / barranco da água.
        if f > 0.4 {
            let (art, index) = texture(base, t);
            let mut c = cache.texel(&art, index, px, py);
            c = apply_season_ground(c, base, season);
            return if g == Ground::Water { shade(c, 0.72) } else { shade(c, 0.88) };
        }
    }
    let own = ground(t).unwrap_or(base);
    let g = if LAYERS.contains(&own) { base } else { own };
    let (art, index) = texture(g, t);
    let c = cache.texel(&art, index, px, py);
    apply_season_ground(c, g, season)
}

/// Canteiro regado: terra mais escura e fria.
pub const WET: [f32; 3] = [0.68, 0.62, 0.7];

/// Quadros da camada animada da água (ondas andando e espuma piscando).
pub const WATER_FRAMES: usize = 4;

/// Pixel da camada animada da água no quadro `frame` (transparente fora do lago).
pub fn water_pixel(
    cache: &mut TexCache,
    ground: &impl Fn(IVec2) -> Option<Ground>,
    water: &Art,
    u: f32,
    v: f32,
    frame: usize,
) -> Rgba {
    let f = field(ground, Ground::Water, u, v);
    if f < 0.5 {
        return [0, 0, 0, 0];
    }
    let (x, y) = ((u * TILE) as u32, (v * TILE) as u32);
    let mut c = cache.texel(water, frame % 2, x, y);
    let phase = frame as f32 * std::f32::consts::FRAC_PI_2;
    let (px, py) = (u * TILE, v * TILE);
    // Reflexos: tracinhos horizontais ondulados que deslizam a cada quadro, só em
    // algumas regiões do lago (o ruído decide onde), como na imagem de inspiração.
    let ripple = (py * 0.9 + (px * 0.18 + phase).sin() * 1.6 - phase).sin();
    let patch = noise(u * 0.9 + frame as f32 * 0.08, v * 1.4);
    if ripple > 0.94 && patch > 0.02 {
        c = lighten(c, 0.5);
    }
    // Brilhinhos que acendem e apagam.
    let sparkle = noise(px * 0.37 + frame as f32 * 5.1, py * 0.41 - frame as f32 * 3.7);
    if sparkle > 0.47 {
        c = lighten(c, 0.85);
    }
    // Espuma na margem que "respira".
    if f < 0.57 && (noise(u * 3.0, v * 3.0 + frame as f32 * 0.7) > -0.05) {
        c = lighten(c, 0.6);
    }
    c
}
