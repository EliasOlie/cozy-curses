//! O mundo como dados: quais mapas existem, como se ligam (portas e bordas) e o papel de
//! cada um (interior, fazenda, quem atende no balcão, que loja). Fonte: assets/maps/world.json
//! + um .txt por mapa. O editor cria e muda tudo isso; o jogo só lê.

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::shop::Shop;
use crate::villagers::Kind;

include!(concat!(env!("OUT_DIR"), "/embedded_maps.rs"));

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MapInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub indoor: bool,
    /// O mapa onde se planta (ferramentas, plantações, fornalhas). Só um.
    #[serde(default)]
    pub farm: bool,
    /// Quem fica atrás do balcão (`N`) de dia.
    #[serde(default)]
    pub keeper: Option<Kind>,
    /// O balcão abre esta loja.
    #[serde(default)]
    pub shop: Option<Shop>,
}

impl MapInfo {
    pub fn new(id: &str, name: &str, indoor: bool) -> Self {
        MapInfo { id: id.into(), name: name.into(), indoor, farm: false, keeper: None, shop: None }
    }

    /// Caractere do chão "vazio" (o que a borracha do editor deixa).
    pub fn blank(&self) -> char {
        if self.indoor { '_' } else { '.' }
    }
}

/// Porta de um prédio (o tile da porta) que leva a um interior.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DoorLink {
    pub map: String,
    pub at: [i32; 2],
    pub to: String,
}

/// Bordas marcadas com `<`, `>`, `^` ou `v` que levam às bordas de outro mapa.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PortalLink {
    pub map: String,
    pub edge: char,
    pub to: String,
    pub to_edge: char,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WorldDef {
    /// Onde o jogo começa e onde se acorda ao carregar.
    pub start: String,
    pub maps: Vec<MapInfo>,
    #[serde(default)]
    pub doors: Vec<DoorLink>,
    #[serde(default)]
    pub portals: Vec<PortalLink>,
}

impl WorldDef {
    pub fn info(&self, id: &str) -> MapInfo {
        self.maps.iter().find(|m| m.id == id).cloned().unwrap_or_else(|| MapInfo::new(id, id, false))
    }

    pub fn farm_id(&self) -> Option<String> {
        self.maps.iter().find(|m| m.farm).map(|m| m.id.clone())
    }

    pub fn door_at(&self, map: &str, t: IVec2) -> Option<&DoorLink> {
        self.doors.iter().find(|d| d.map == map && IVec2::from_array(d.at) == t)
    }

    /// A porta (de fora) que leva a este interior.
    pub fn door_into(&self, interior: &str) -> Option<&DoorLink> {
        self.doors.iter().find(|d| d.to == interior)
    }

    pub fn portal(&self, map: &str, edge: char) -> Option<&PortalLink> {
        self.portals.iter().find(|p| p.map == map && p.edge == edge)
    }
}

pub const EDGES: [char; 4] = ['<', '>', '^', 'v'];

static WORLD: LazyLock<RwLock<WorldDef>> = LazyLock::new(|| {
    RwLock::new(serde_json::from_str(EMBEDDED_WORLD).expect("assets/maps/world.json inválido"))
});

/// Texto atual de cada mapa (embutido, trocado pelo que o editor salvou).
static TEXTS: LazyLock<RwLock<HashMap<String, String>>> = LazyLock::new(|| {
    RwLock::new(EMBEDDED_MAPS.iter().map(|(id, text)| (id.to_string(), text.to_string())).collect())
});

pub fn world() -> WorldDef {
    WORLD.read().unwrap().clone()
}

pub fn set_world(def: WorldDef) {
    *WORLD.write().unwrap() = def;
}

pub fn info(id: &str) -> MapInfo {
    WORLD.read().unwrap().info(id)
}

pub fn map_text(id: &str) -> Option<String> {
    TEXTS.read().unwrap().get(id).cloned()
}

pub fn set_map_text(id: &str, text: String) {
    TEXTS.write().unwrap().insert(id.to_string(), text);
}

/// O texto que veio com o jogo (None para mapas criados no editor).
pub fn original_text(id: &str) -> Option<&'static str> {
    EMBEDDED_MAPS.iter().find(|(name, _)| *name == id).map(|(_, text)| *text)
}

