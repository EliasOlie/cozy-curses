//! Aparência do fazendeiro: peças tingíveis empilhadas (corpo, calça, camisa, cabelo).
//!
//! Os PNGs das peças são desenhados em tons de cinza (chaves tint_* do tema) e o jogo
//! multiplica cada peça pela cor escolhida no criador de personagem.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::sprites::{Art, Sprites};

pub const SKINS: [(u8, u8, u8); 6] = [
    (0xf7, 0xd7, 0xb5),
    (0xf0, 0xbf, 0x92),
    (0xd6, 0x9a, 0x6a),
    (0xb0, 0x76, 0x4c),
    (0x86, 0x55, 0x36),
    (0x5c, 0x3a, 0x26),
];
pub const HAIR_COLORS: [(u8, u8, u8); 8] = [
    (0x4a, 0x30, 0x22),
    (0x7a, 0x4a, 0x2a),
    (0xc0, 0x7a, 0x3c),
    (0xf0, 0xcc, 0x70),
    (0xb8, 0x3e, 0x32),
    (0x30, 0x2c, 0x34),
    (0xdc, 0xd8, 0xd2),
    (0x6c, 0x8c, 0xd8),
];
pub const SHIRTS: [(u8, u8, u8); 8] = [
    (0xe0, 0x5a, 0x4a),
    (0x4f, 0x7c, 0xc2),
    (0x6f, 0xb0, 0x4a),
    (0xe6, 0xbf, 0x62),
    (0x9a, 0x6b, 0xd6),
    (0xf6, 0xee, 0xdb),
    (0x48, 0x48, 0x54),
    (0xf2, 0x8c, 0xb1),
];
pub const PANTS: [(u8, u8, u8); 6] = [
    (0x3e, 0x62, 0x9c),
    (0x7a, 0x50, 0x30),
    (0x3a, 0x3a, 0x44),
    (0x6f, 0x7d, 0x4a),
    (0xa7, 0xa1, 0x98),
    (0x8d, 0x45, 0x2e),
];
pub const HAIR_STYLES: usize = 3;
pub const HAIR_STYLE_NAMES: [&str; HAIR_STYLES] = ["Curto", "Comprido", "Coque"];
pub const NAME_MAX: usize = 14;

/// À noite as peças tingidas são escurecidas para o tom azulado da paleta noturna.
pub const NIGHT_TINT: Color = Color::srgb(0.52, 0.55, 0.82);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Character {
    pub name: String,
    pub farm: String,
    pub skin: usize,
    pub hair: usize,
    pub hair_color: usize,
    pub shirt: usize,
    pub pants: usize,
}

impl Default for Character {
    fn default() -> Self {
        Character {
            name: "Forasteiro".into(),
            farm: "Sítio Sereno".into(),
            skin: 1,
            hair: 0,
            hair_color: 1,
            shirt: 1,
            pants: 0,
        }
    }
}

/// Um item que o criador de personagem deixa trocar com < e >.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Trait {
    Skin,
    Hair,
    HairColor,
    Shirt,
    Pants,
}

impl Trait {
    pub const ALL: [Trait; 5] = [Trait::Skin, Trait::Hair, Trait::HairColor, Trait::Shirt, Trait::Pants];

    pub fn label(self) -> &'static str {
        match self {
            Trait::Skin => "Pele",
            Trait::Hair => "Cabelo",
            Trait::HairColor => "Cor do cabelo",
            Trait::Shirt => "Camisa",
            Trait::Pants => "Calça",
        }
    }

    fn count(self) -> usize {
        match self {
            Trait::Skin => SKINS.len(),
            Trait::Hair => HAIR_STYLES,
            Trait::HairColor => HAIR_COLORS.len(),
            Trait::Shirt => SHIRTS.len(),
            Trait::Pants => PANTS.len(),
        }
    }
}

impl Character {
    fn slot(&mut self, t: Trait) -> &mut usize {
        match t {
            Trait::Skin => &mut self.skin,
            Trait::Hair => &mut self.hair,
            Trait::HairColor => &mut self.hair_color,
            Trait::Shirt => &mut self.shirt,
            Trait::Pants => &mut self.pants,
        }
    }

    /// Avança (+1) ou volta (-1) uma opção, dando a volta no fim da lista.
    pub fn cycle(&mut self, t: Trait, step: i32) {
        let n = t.count() as i32;
        let v = self.slot(t);
        *v = ((*v as i32 + step).rem_euclid(n)) as usize;
    }

    pub fn value_label(&self, t: Trait) -> String {
        let mut c = self.clone();
        let v = *c.slot(t);
        match t {
            Trait::Hair => HAIR_STYLE_NAMES[v.min(HAIR_STYLES - 1)].into(),
            _ => format!("{}/{}", v + 1, t.count()),
        }
    }

    /// Peças de baixo para cima, cada uma com a cor que a tinge.
    pub fn parts(&self, sprites: &Sprites) -> [(Art, Color); 4] {
        let pick = |list: &[(u8, u8, u8)], i: usize| {
            let (r, g, b) = list[i.min(list.len() - 1)];
            Color::srgb_u8(r, g, b)
        };
        [
            (sprites.farmer_body.clone(), pick(&SKINS, self.skin)),
            (sprites.farmer_pants.clone(), pick(&PANTS, self.pants)),
            (sprites.farmer_shirt.clone(), pick(&SHIRTS, self.shirt)),
            (
                sprites.farmer_hair[self.hair.min(HAIR_STYLES - 1)].clone(),
                pick(&HAIR_COLORS, self.hair_color),
            ),
        ]
    }
}

/// Linha da spritesheet do fazendeiro (contrato com art/sprites/farmer_*.json).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Facing {
    #[default]
    Down,
    Up,
    Left,
    Right,
}

impl Facing {
    /// Índice do quadro: 3 quadros por linha (parado, passo A, passo B);
    /// linhas: baixo, cima, lado (olhando para a direita).
    pub fn frame(self, step: usize) -> usize {
        let row = match self {
            Facing::Down => 0,
            Facing::Up => 1,
            Facing::Left | Facing::Right => 2,
        };
        row * 3 + step.min(2)
    }

    pub fn flip(self) -> bool {
        self == Facing::Left
    }
}
