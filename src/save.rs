//! Jogos salvos: 3 slots, gravados ao amanhecer (como dormir no Stardew).
//! Navegador: localStorage. Nativo: saves/slotN.json.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::character::Character;
use crate::cycle::Clock;
use crate::economy::{FarmState, Game};
use crate::inventory::Inventory;
use crate::map::Ground;
use crate::worlddef::{self, WorldDef};

pub const SLOTS: usize = 3;
const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone)]
pub struct SaveData {
    pub version: u32,
    pub character: Character,
    pub day: u32,
    pub game: Game,
    /// Só o chão que mudou em relação ao farm.txt.
    pub ground: Vec<([i32; 2], Ground)>,
    /// (tile, dias regados)
    pub crops: Vec<([i32; 2], f32)>,
    pub factories: Vec<[i32; 2]>,
    // Campos novos têm padrão: saves antigos continuam abrindo.
    #[serde(default)]
    pub watered: Vec<[i32; 2]>,
    /// Objetos tirados da fazenda (árvores cortadas, pedras quebradas, mato).
    #[serde(default)]
    pub removed: Vec<[i32; 2]>,
    #[serde(default)]
    pub inventory: Inventory,
    #[serde(default)]
    pub crops_v2: Vec<([i32; 2], crate::inventory::CropKind, u32)>,
    #[serde(default)]
    pub bought_animals: Vec<(crate::animals::Species, crate::animals::Gender)>,
}

impl SaveData {
    pub fn new(character: Character) -> Self {
        SaveData {
            version: VERSION,
            character,
            day: 1,
            game: Game::default(),
            ground: Vec::new(),
            crops: Vec::new(),
            factories: Vec::new(),
            watered: Vec::new(),
            removed: Vec::new(),
            inventory: Inventory::default(),
            crops_v2: Vec::new(),
            bought_animals: Vec::new(),
        }
    }
}

/// O jogo em andamento e o slot onde ele é salvo.
#[derive(Resource)]
pub struct ActiveSave {
    pub slot: usize,
    pub data: SaveData,
}

/// Foto do estado atual, chamada logo depois do amanhecer.
pub fn autosave(
    mut active: ResMut<ActiveSave>,
    mut game: ResMut<Game>,
    clock: Res<Clock>,
    farm: Res<FarmState>,
    inv: Res<Inventory>,
) {
    let data = &mut active.data;
    data.day = clock.day;
    data.game = game.clone();
    data.ground = farm.edits.ground.iter().map(|(t, g)| (t.to_array(), *g)).collect();
    data.removed = farm.edits.removed.iter().map(|t| t.to_array()).collect();
    data.crops = farm.crops.iter().map(|(t, c)| (t.to_array(), c.days as f32)).collect();
    data.crops_v2 = farm.crops.iter().map(|(t, c)| (t.to_array(), c.kind, c.days)).collect();
    data.bought_animals = farm.bought_animals.clone();
    data.watered = farm.watered.iter().map(|t| t.to_array()).collect();
    data.factories = farm.factories.iter().map(|t| t.to_array()).collect();
    data.inventory = inv.clone();
    match write(active.slot, &active.data) {
        Ok(()) => game.say("Jogo salvo."),
        Err(e) => game.say(format!("Não consegui salvar: {e}")),
    }
}

pub fn read(slot: usize) -> Option<SaveData> {
    let text = backend::read(slot)?;
    match serde_json::from_str::<SaveData>(&text) {
        Ok(data) if data.version == VERSION => Some(data),
        Ok(data) => {
            warn!("slot {slot}: versão {} incompatível", data.version);
            None
        }
        Err(e) => {
            warn!("slot {slot}: save ilegível: {e}");
            None
        }
    }
}

pub fn write(slot: usize, data: &SaveData) -> Result<(), String> {
    let text = serde_json::to_string(data).map_err(|e| e.to_string())?;
    backend::write(slot, &text)
}

pub fn delete(slot: usize) {
    backend::delete(slot);
}

/// Primeiro slot vazio, se houver.
pub fn free_slot() -> Option<usize> {
    (0..SLOTS).find(|&s| backend::read(s).is_none())
}

// ---------------------------------------------------------------------------
// Mapas do editor. Navegador: localStorage. Nativo: os próprios assets/maps/*.txt.
// ---------------------------------------------------------------------------

/// No início: o mundo e os mapas salvos pelo editor passam a valer no lugar dos embutidos.
pub fn load_map_overrides() {
    if let Some(text) = backend::read_map(WORLD_FILE)
        && let Ok(def) = serde_json::from_str::<WorldDef>(&text)
    {
        worlddef::set_world(def);
    }
    for info in worlddef::world().maps {
        if let Some(text) = backend::read_map(&info.id)
            && Some(text.as_str()) != worlddef::original_text(&info.id)
        {
            worlddef::set_map_text(&info.id, text);
        }
    }
}

/// Nome do "mapa" onde fica o manifesto (no nativo: assets/maps/world.json).
const WORLD_FILE: &str = "world";

pub fn save_map(id: &str, text: &str) -> Result<(), String> {
    backend::write_map(id, text)?;
    worlddef::set_map_text(id, text.to_string());
    Ok(())
}

pub fn save_world(def: &WorldDef) -> Result<(), String> {
    let text = serde_json::to_string_pretty(def).map_err(|e| e.to_string())?;
    backend::write_map(WORLD_FILE, &text)?;
    worlddef::set_world(def.clone());
    Ok(())
}

/// Volta um mapa ao original do jogo (mapas novos voltam a ficar vazios).
pub fn reset_map(id: &str) {
    if let Some(original) = worlddef::original_text(id) {
        backend::reset_map(id, original);
        worlddef::set_map_text(id, original.to_string());
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(catch, js_namespace = localStorage, js_name = getItem)]
        fn get_item(key: &str) -> Result<Option<String>, JsValue>;
        #[wasm_bindgen(catch, js_namespace = localStorage, js_name = setItem)]
        fn set_item(key: &str, value: &str) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_namespace = localStorage, js_name = removeItem)]
        fn remove_item(key: &str) -> Result<(), JsValue>;
    }

    fn key(slot: usize) -> String {
        format!("cozy-curses/slot{slot}")
    }

    pub fn read(slot: usize) -> Option<String> {
        get_item(&key(slot)).ok()?
    }

    pub fn write(slot: usize, text: &str) -> Result<(), String> {
        set_item(&key(slot), text)
            .map_err(|_| "o navegador recusou (armazenamento cheio ou bloqueado)".to_string())
    }

    pub fn delete(slot: usize) {
        let _ = remove_item(&key(slot));
    }

    fn map_key(name: &str) -> String {
        format!("cozy-curses/map/{name}")
    }

    pub fn read_map(name: &str) -> Option<String> {
        get_item(&map_key(name)).ok()?
    }

    pub fn write_map(name: &str, text: &str) -> Result<(), String> {
        set_item(&map_key(name), text).map_err(|_| "o navegador recusou salvar o mapa".to_string())
    }

    pub fn reset_map(name: &str, _original: &str) {
        let _ = remove_item(&map_key(name));
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::path::PathBuf;

    fn path(slot: usize) -> PathBuf {
        PathBuf::from("saves").join(format!("slot{slot}.json"))
    }

    pub fn read(slot: usize) -> Option<String> {
        std::fs::read_to_string(path(slot)).ok()
    }

    pub fn write(slot: usize, text: &str) -> Result<(), String> {
        std::fs::create_dir_all("saves").map_err(|e| e.to_string())?;
        std::fs::write(path(slot), text).map_err(|e| e.to_string())
    }

    pub fn delete(slot: usize) {
        let _ = std::fs::remove_file(path(slot));
    }

    /// No nativo o editor grava direto no repositório (rodando da raiz do projeto).
    fn map_path(name: &str) -> PathBuf {
        let ext = if name == super::WORLD_FILE { "json" } else { "txt" };
        PathBuf::from("assets").join("maps").join(format!("{name}.{ext}"))
    }

    pub fn read_map(name: &str) -> Option<String> {
        std::fs::read_to_string(map_path(name)).ok()
    }

    pub fn write_map(name: &str, text: &str) -> Result<(), String> {
        std::fs::write(map_path(name), text).map_err(|e| e.to_string())
    }

    pub fn reset_map(name: &str, original: &str) {
        let _ = std::fs::write(map_path(name), original);
    }
}
