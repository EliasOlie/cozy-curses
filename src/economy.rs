//! Mercado micro (B2C, dia) e macro (B2B, noite), e o equilíbrio da maldição.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;
use bevy::sprite::Anchor;
use serde::{Deserialize, Serialize};

use crate::bake::Painter;
use crate::cycle::{Clock, DayPhase, Season};
use crate::{Gameplay, Overlay};
use crate::dialogue::Dialogue;
use crate::fx::{Focus, FocusKind, Pickups, Popups, spawn_burst};
use crate::inventory::{CropData, CropKind, Inventory, Item, MaskKind};
use crate::map::{Building, Ground, Layout, Map, MapEdits, MapState, Prop, TILE, standing_at, tile_base, tile_center};
use crate::player::{Player, target_tile};
use crate::save::{self, ActiveSave};
use crate::shop::Shop;
use crate::sprites::{DualSprite, Sprites, fx};
use crate::animals::{Animal, Gender, Species};
use crate::villagers::{Inquisitor, Kind, Villager, spawn_inquisitor};
use crate::world::Travel;

pub fn plugin(app: &mut App) {
    app.init_resource::<TargetQol>()
        .add_systems(
            Update,
            (grow_views, wet_soil, run_factories.run_if(in_state(DayPhase::Night)), interact, check_goal)
                .in_set(Gameplay),
        )
    .add_systems(
        Update,
        spawn_farm_views
            .in_set(crate::world::MapEntered)
            .after(crate::world::enter_map)
            .run_if(state_changed::<MapState>),
    )
    .add_systems(OnTransition { exited: DayPhase::Night, entered: DayPhase::Day }, (dawn, save::autosave).chain())
    .add_systems(OnEnter(DayPhase::Night), dusk);

    #[cfg(debug_assertions)]
    app.add_systems(Update, dev_cheat.in_set(Gameplay));
}

/// Só em builds de desenvolvimento, para testar sem grind:
/// [G] +100 ouro · [C] +30 maldição
#[cfg(debug_assertions)]
fn dev_cheat(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>) {
    if keys.just_pressed(KeyCode::KeyG) {
        game.gold += 100;
        game.say("[dev] +100 ouro");
    }
    if keys.just_pressed(KeyCode::KeyC) {
        game.add_curse(30.0);
        game.say("[dev] +30 maldição");
    }
}

pub const CARROT_PRICE: i32 = 6;
const MASK_COST: i32 = 40;
pub const ESSENCE_PRICE: i32 = 4;
pub const GOAL: i32 = 1000;
pub const MAX_ENERGY: f32 = 100.0;
/// Maldição a partir da qual o Inquisidor aparece ao amanhecer.
const INQUISITOR_AT: f32 = 60.0;
/// Abaixo disso ele vai embora sozinho.
const INQUISITOR_LEAVES_AT: f32 = 35.0;
/// Acima disso um líder de facção se radicaliza e sabota uma fornalha ao anoitecer.
const NEMESIS_AT: f32 = 85.0;
/// Cenouras dadas ao Ghoul numa noite que esvaziam a padaria no dia seguinte.
const GHOUL_GLUTTONY: u32 = 3;

fn full_energy() -> f32 {
    MAX_ENERGY
}

pub const MAX_HEALTH: f32 = 100.0;

fn full_health() -> f32 {
    MAX_HEALTH
}

#[derive(Resource, Serialize, Deserialize, Clone)]
pub struct Game {
    pub gold: i32,
    pub essence: f32,
    /// 0–100. Sobe com os lucros noturnos, desce com a boa vizinhança.
    pub curse: f32,
    #[serde(default = "full_energy")]
    pub energy: f32,
    /// Vida (pragas noturnas machucam). Volta cheia ao amanhecer.
    #[serde(default = "full_health")]
    pub health: f32,
    pub has_mask: bool,
    #[serde(default)]
    pub equipped_mask: Option<MaskKind>,
    #[serde(default)]
    pub unlocked_masks: Vec<MaskKind>,
    pub factories: u32,
    pub ghoul_fed: u32,
    pub bakery_closed: bool,
    pub bread_today: bool,
    pub mined_tonight: bool,
    pub inquisitor: bool,
    pub goal_reached: bool,
    /// Quantas vezes o feed foi rolado (a mecânica do doomscroll vem depois).
    #[serde(default)]
    pub doomscrolls: u32,
    /// Cutscenes de gatilho (`when`) que já tocaram neste save.
    #[serde(default)]
    pub seen_cutscenes: Vec<String>,
    /// Ação disponível agora, mostrada na interface.
    #[serde(skip)]
    pub hint: Option<String>,
    #[serde(skip)]
    pub log: VecDeque<String>,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            gold: 20,
            essence: 0.0,
            curse: 0.0,
            energy: MAX_ENERGY,
            health: MAX_HEALTH,
            has_mask: false,
            equipped_mask: None,
            unlocked_masks: Vec::new(),
            factories: 0,
            ghoul_fed: 0,
            bakery_closed: false,
            bread_today: false,
            mined_tonight: false,
            inquisitor: false,
            goal_reached: false,
            doomscrolls: 0,
            seen_cutscenes: Vec::new(),
            hint: None,
            log: VecDeque::new(),
        }
    }
}

impl Game {
    pub fn say(&mut self, msg: impl Into<String>) {
        self.log.push_back(msg.into());
        while self.log.len() > 4 {
            self.log.pop_front();
        }
    }

    pub fn equip_mask(&mut self, mask: Option<MaskKind>) {
        self.equipped_mask = mask;
        self.has_mask = mask.is_some();
    }

    pub fn unlock_mask(&mut self, mask: MaskKind) {
        if !self.unlocked_masks.contains(&mask) {
            self.unlocked_masks.push(mask);
        }
        self.equip_mask(Some(mask));
    }

    pub fn cycle_mask(&mut self) -> Option<MaskKind> {
        if self.unlocked_masks.is_empty() {
            return None;
        }
        let next = match self.equipped_mask {
            None => Some(self.unlocked_masks[0]),
            Some(current) => {
                let idx = self.unlocked_masks.iter().position(|&m| m == current).unwrap_or(0);
                if idx + 1 < self.unlocked_masks.len() {
                    Some(self.unlocked_masks[idx + 1])
                } else {
                    None
                }
            }
        };
        self.equip_mask(next);
        next
    }

    pub fn add_curse(&mut self, mut amount: f32) {
        if amount > 0.0 && self.equipped_mask == Some(MaskKind::Crow) {
            amount *= 0.65;
        }
        self.curse = (self.curse + amount).clamp(0.0, 100.0);
    }

    /// Gasta energia, se houver.
    pub fn spend(&mut self, energy: f32) -> bool {
        if self.energy < energy {
            return false;
        }
        self.energy -= energy;
        true
    }

    pub fn rest(&mut self, energy: f32) {
        self.energy = (self.energy + energy).min(MAX_ENERGY);
    }

    /// Cura e devolve quanto de fato curou.
    pub fn heal(&mut self, amount: f32) -> f32 {
        let before = self.health;
        self.health = (self.health + amount).min(MAX_HEALTH);
        self.health - before
    }

    pub fn factory_cost(&self) -> i32 {
        (60.0 * 1.6f32.powi(self.factories as i32)).round() as i32
    }

    /// Lucro exponencial (AdVenture Capitalist): cada fornalha acelera todas.
    pub fn essence_rate(&self) -> f32 {
        self.factories as f32 * 0.5 * 1.15f32.powi(self.factories as i32)
    }

    pub fn carrot_price(&self) -> i32 {
        if self.inquisitor { CARROT_PRICE / 2 } else { CARROT_PRICE }
    }

    fn fine(&self) -> i32 {
        50 + self.gold / 5
    }
}

/// O que existe na fazenda mesmo quando o jogador está longe dela.
#[derive(Resource, Default)]
pub struct FarmState {
    /// tile -> plantação (cultura e dias)
    pub crops: HashMap<IVec2, CropData>,
    /// Animais comprados na fazenda (persistidos entre dias e saves)
    pub bought_animals: Vec<(Species, Gender)>,
    /// Canteiros regados hoje (secam ao amanhecer).
    pub watered: HashSet<IVec2>,
    pub factories: Vec<IVec2>,
    /// Para o resumo do amanhecer.
    pub harvested_today: u32,
    pub gold_at_dawn: i32,
    /// Terra arada e objetos removidos (árvores cortadas, pedras quebradas).
    pub edits: MapEdits,
}

impl FarmState {
    pub fn ripe(&self, t: IVec2) -> Option<bool> {
        self.crops.get(&t).map(|c| c.days >= c.kind.growth_days())
    }

    /// Na virada de estação, árvores e arbustos cortados voltam a brotar na fazenda.
    pub fn reforest(&mut self, farm_layout: &Layout) -> u32 {
        let mut restored = 0;
        self.edits.removed.retain(|&tile| {
            let ch = farm_layout.at(tile);
            if matches!(ch, 'T' | 'P' | 'U' | 'B') {
                restored += 1;
                false // remove de removed, restaurando a árvore no mapa
            } else {
                true
            }
        });
        restored
    }
}

fn crop_stage(days: u32, kind: CropKind) -> usize {
    if days >= kind.growth_days() {
        2
    } else {
        (days * 2 / kind.growth_days().max(1)).min(1) as usize
    }
}

/// Desenho de uma plantação da fazenda (só existe com o jogador na fazenda).
#[derive(Component)]
pub struct CropView(pub IVec2);

#[derive(Component)]
pub struct FactoryView(pub IVec2);

/// Quadro da folha `crops`: uma linha por cultivo (ordem do enum), uma coluna por estágio.
fn crop_frame(kind: CropKind, days: u32) -> usize {
    kind as usize * 3 + crop_stage(days, kind)
}

pub fn spawn_crop_view(commands: &mut Commands, sprites: &Sprites, scope: &str, tile: IVec2, kind: CropKind, days: u32) -> Entity {
    let (sprite, dual) = DualSprite::bundle(&sprites.crops, crop_frame(kind, days));
    commands
        .spawn((
            CropView(tile),
            sprite,
            dual,
            Anchor::BOTTOM_CENTER,
            standing_at(tile_base(tile)),
            DespawnOnExit(MapState(scope.to_string())),
        ))
        .id()
}

fn spawn_factory_view(commands: &mut Commands, sprites: &Sprites, scope: &str, tile: IVec2) -> Entity {
    commands
        .spawn((
            FactoryView(tile),
            DualSprite::bundle(&sprites.factory, 0),
            Anchor::BOTTOM_CENTER,
            standing_at(tile_base(tile)),
            DespawnOnExit(MapState(scope.to_string())),
        ))
        .id()
}

/// Colhe a plantação do tile (E ou foice): some o desenho, entra o produto colhido com qualidade por chance.
pub fn harvest(
    commands: &mut Commands,
    sprites: &Sprites,
    farm: &mut FarmState,
    inv: &mut Inventory,
    views: &Query<(Entity, &CropView)>,
    scope: &str,
    tile: IVec2,
) -> Option<(Item, crate::inventory::Quality)> {
    let crop = farm.crops.remove(&tile)?;
    for (entity, view) in views {
        if view.0 == tile {
            commands.entity(entity).despawn();
        }
    }
    let item = crop.kind.harvest_item();
    let roll = ((tile.x * 37 + tile.y * 59 + farm.harvested_today as i32 * 17).abs() % 100) as u32;
    let quality = crate::inventory::Quality::roll(roll);
    inv.add_with_quality(item, 1, quality);
    spawn_burst(commands, sprites, scope, tile_base(tile), fx::LEAF, 4);
    farm.harvested_today += 1;
    Some((item, quality))
}

/// Recria o estado do save: recursos do jogo, relógio, mochila e a fazenda.
pub fn restore(
    mut commands: Commands,
    active: Res<ActiveSave>,
    mut cutscenes: Option<ResMut<crate::cutscene::CutsceneManager>>,
) {
    let data = &active.data;
    let mut game = data.game.clone();
    let tile = |t: &[i32; 2]| IVec2::from_array(*t);
    let farm = FarmState {
        crops: if !data.crops_v2.is_empty() {
            data.crops_v2.iter().map(|(t, k, d)| (tile(t), CropData { kind: *k, days: *d })).collect()
        } else {
            data.crops.iter().map(|(t, d)| (tile(t), CropData { kind: CropKind::Carrot, days: *d as u32 })).collect()
        },
        bought_animals: data.bought_animals.clone(),
        watered: data.watered.iter().map(tile).collect(),
        factories: data.factories.iter().map(tile).collect(),
        edits: MapEdits {
            ground: data.ground.iter().map(|(t, g)| (tile(t), *g)).collect(),
            removed: data.removed.iter().map(tile).collect(),
        },
        ..default()
    };
    game.factories = farm.factories.len() as u32;
    let mut farm = farm;
    farm.gold_at_dawn = game.gold;

    game.log.clear();
    if data.day == 1 {
        game.say(format!("Bem-vindo ao {}, {}.", data.character.farm, data.character.name));
        game.say("Are a terra com a enxada, plante e regue todo dia. [Espaço] usa o item.");
        game.say("Florinda vende sementes da estação; Seu Bartô vende animais e carnes.");
        if let Some(cutscenes) = &mut cutscenes {
            cutscenes.play_id(
                "intro",
                &[("nome", data.character.name.clone()), ("sitio", data.character.farm.clone())],
            );
        }
    } else {
        game.say(format!("Dia {}. Bom dia, {}.", data.day, data.character.name));
    }
    commands.insert_resource(Clock::new(data.day));
    commands.insert_resource(data.inventory.clone());
    commands.insert_resource(game);
    commands.insert_resource(farm);
}

#[derive(Component)]
pub struct IntroMayor;

/// Ao chegar na fazenda: plantações, fornalhas e, se for o caso, o Inquisidor.
fn spawn_farm_views(
    mut commands: Commands,
    sprites: Res<Sprites>,
    farm: Res<FarmState>,
    game: Res<Game>,
    map: Option<Res<Map>>,
    cutscenes: Option<Res<crate::cutscene::CutsceneManager>>,
) {
    // O mapa pode ainda não existir no primeiro quadro do jogo (estado vazio).
    let Some(map) = map.filter(|m| m.info.farm) else { return };
    for (tile, crop) in &farm.crops {
        spawn_crop_view(&mut commands, &sprites, &map.id, *tile, crop.kind, crop.days);
    }
    for tile in &farm.factories {
        spawn_factory_view(&mut commands, &sprites, &map.id, *tile);
    }
    if game.inquisitor {
        spawn_inquisitor(&mut commands, &sprites, &map.id, inquisitor_spot(&map));
    }
    if let Some(cutscenes) = cutscenes {
        if cutscenes.is_playing() && cutscenes.active.as_ref().is_some_and(|a| a.cutscene.id == "intro") {
            let mayor_tile = map.spawn.unwrap_or(map.size / 2) + IVec2::new(2, 0);
            // Linha 2 da folha: de lado, espelhado (olhando para o jogador à esquerda).
            let mut mayor_sprite = sprites.mayor.sprite(DayPhase::Day, 6);
            mayor_sprite.flip_x = true;
            commands.spawn((
                IntroMayor,
                mayor_sprite,
                DualSprite(sprites.mayor.clone()),
                Anchor::BOTTOM_CENTER,
                standing_at(tile_base(mayor_tile)),
                DespawnOnExit(MapState(map.id.clone())),
            ))
            .with_child((DualSprite::bundle(&sprites.shadow, 0), Transform::from_xyz(0.0, 1.0, -0.01)));
        }
    }
}

fn inquisitor_spot(map: &Map) -> IVec2 {
    map.find(Prop::Stand).map_or(IVec2::ZERO, |s| s.anchor + IVec2::new(-1, 1))
}

fn grow_views(farm: Res<FarmState>, mut views: Query<(&CropView, &mut Sprite)>) {
    if !farm.is_changed() {
        return;
    }
    for (view, mut sprite) in &mut views {
        let Some(crop) = farm.crops.get(&view.0) else { continue };
        let frame = crop_frame(crop.kind, crop.days);
        if let Some(atlas) = &mut sprite.texture_atlas {
            if atlas.index != frame {
                atlas.index = frame;
            }
        }
    }
}

/// Canteiro regado fica mais escuro (repinta só o que mudou no chão assado).
fn wet_soil(
    farm: Res<FarmState>,
    mut map: ResMut<Map>,
    sprites: Res<Sprites>,
    mut images: ResMut<Assets<Image>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
) {
    if !map.info.farm || !(farm.is_changed() || map.is_added()) {
        return;
    }
    let wanted: HashSet<IVec2> =
        farm.watered.iter().copied().filter(|t| map.ground(*t) == Some(Ground::Soil)).collect();
    if wanted == map.wet {
        return;
    }
    let changed: Vec<IVec2> = wanted.symmetric_difference(&map.wet).copied().collect();
    map.wet = wanted;
    map.repaint_exact(&mut Painter { images: &mut images, layouts: &layouts }, &sprites, &changed);
}

fn run_factories(time: Res<Time>, mut game: ResMut<Game>) {
    let rate = game.essence_rate();
    game.essence += rate * time.delta_secs();
}

/// Informações contextuais de QoL projetadas sobre o alvo (tempo para colheita, pronto, etc).
#[derive(Resource, Default)]
pub struct TargetQol {
    pub text: Option<String>,
    pub world_pos: Vec2,
}

enum Action {
    Harvest(CropKind),
    CropInfo(String),
    ForgeMask,
    BuildFactory,
    BreakFloor,
    Mine,
    FeedGhoul,
    Talk(Kind),
    Pet(Species, Gender, bool),
    CollectProduct(Entity, Species, Gender),
    OpenShop(Shop),
    PayInquisitor(Entity),
    Nap,
    Sleep,
    /// Nada a fazer, mas vale explicar o porquê.
    Info(&'static str),
}

impl Action {
    fn label(&self, game: &Game, _inv: &Inventory) -> String {
        match self {
            Action::Harvest(kind) => format!("[E] Colher {} (Pronto!)", kind.name()),
            Action::CropInfo(text) => text.clone(),
            Action::ForgeMask => format!("[E] Forjar Máscara de Gárgula (-{MASK_COST} ouro)"),
            Action::BuildFactory => format!("[E] Erguer Fornalha de Almas (-{} ouro)", game.factory_cost()),
            Action::BreakFloor => "[E] Pisar com todo o peso da gárgula".into(),
            Action::Mine => "[E] Minerar a caverna (+15 essência)".into(),
            Action::FeedGhoul => "[E] Dar uma cenoura ao Ghoul".into(),
            Action::Talk(kind) => format!("[E] Conversar com {}", kind.short_name()),
            Action::Pet(species, gender, has_product) => {
                let name = species.name(*gender);
                if *has_product && species.product(*gender).is_some() {
                    let act = species.product_action(*gender);
                    format!("[E] {act} {name} (Pronto!)")
                } else if species.product(*gender).is_some() {
                    format!("[E] Fazer carinho n{name} · (Produto amanhã)")
                } else {
                    format!("[E] Fazer carinho n{name}")
                }
            }
            Action::CollectProduct(_, species, gender) => {
                let act = species.product_action(*gender);
                let name = species.name(*gender);
                format!("[E] {act} {name}")
            }
            Action::OpenShop(shop) => format!("[E] {}", shop.verb()),
            Action::PayInquisitor(_) => format!("[E] Pagar a auditoria (-{} ouro)", game.fine()),
            Action::Nap => "[E] Cochilar até o anoitecer (marco)".into(),
            Action::Sleep => "[E] Dormir até o amanhecer (marco)".into(),
            Action::Info(text) => (*text).into(),
        }
    }
}

/// O que está ao alcance do jogador neste frame, com o retângulo de cada alvo.
struct Surroundings {
    /// Tile à frente do jogador (onde as ações de chão acontecem).
    tile: IVec2,
    /// Mapa de plantar (ferramentas, fornalhas, caverna).
    farm: bool,
    /// Loja aberta pelo balcão deste mapa.
    shop: Option<Shop>,
    ground: Option<Ground>,
    /// Plantação no tile: espécie, dias e se foi regado hoje
    crop: Option<(CropKind, u32, bool)>,
    factory: bool,
    prop_on_tile: bool,
    prop: Option<(Prop, Rect)>,
    villager: Option<(Kind, Rect)>,
    animal: Option<(Entity, Species, Gender, bool, Rect)>,
    inquisitor: Option<(Entity, Rect)>,
}

/// Ação e o alvo (retângulo e tipo, para o jeito de apontar).
fn resolve(s: &Surroundings, game: &Game, inv: &Inventory, night: bool) -> Option<(Action, Option<(Rect, FocusKind)>)> {
    const NO_MASK: &str = "Sem máscara, a noite não negocia com você. Forje uma em casa.";
    let tile_rect = Some((Rect::from_center_size(tile_center(s.tile), Vec2::splat(TILE)), FocusKind::Tile));

    if let Some((inquisitor, rect)) = s.inquisitor.filter(|_| !night) {
        return Some((Action::PayInquisitor(inquisitor), Some((rect, FocusKind::Person))));
    }
    if let Some((kind, rect)) = s.villager {
        let action = match (kind, night, inv.count(Item::Carrot)) {
            (Kind::Baker, false, _) => Action::OpenShop(Shop::Bakery),
            (Kind::Merchant, false, _) => Action::OpenShop(Shop::Store),
            (Kind::Florist, false, _) => Action::OpenShop(Shop::Florist),
            (Kind::Baker, true, 0) => Action::Info("O Ghoul fareja seus bolsos. Ele quer cenouras."),
            (Kind::Baker, true, _) => Action::FeedGhoul,
            (kind, _, _) => Action::Talk(kind),
        };
        return Some((action, Some((rect, FocusKind::Person))));
    }
    if let Some((prop, rect)) = s.prop {
        let action = match prop {
            Prop::Oven => Action::OpenShop(Shop::Kitchen),
            Prop::Forge if game.has_mask => Action::Info("A forja descansa. Outras máscaras virão."),
            Prop::Forge => Action::ForgeMask,
            Prop::Bed if night => Action::Sleep,
            Prop::Bed => Action::Nap,
            Prop::Stand => match (night, game.has_mask) {
                (false, _) => Action::OpenShop(Shop::Stand),
                (true, false) => Action::Info(NO_MASK),
                (true, true) => Action::OpenShop(Shop::Stand),
            },
            Prop::Sign if s.farm => Action::Info("-> Vila Serena"),
            Prop::Sign => Action::Info("<- Sítio (a sua fazenda)"),
            Prop::Building(b) => Action::Info(closed_door(b)),
            // Fala-se com quem atende por cima do balcão.
            Prop::Counter if night => Action::Info("O balcão está vazio. Quem atende saiu para a noite."),
            Prop::Counter => match s.shop {
                Some(shop) => Action::OpenShop(shop),
                None => Action::Info("Um balcão vazio. Ninguém atende aqui... ainda."),
            },
            _ => unreachable!("só objetos interativos chegam aqui"),
        };
        return Some((action, Some((rect, FocusKind::Thing))));
    }
    if let Some((kind, days, watered)) = s.crop {
        if days >= kind.growth_days() {
            return Some((Action::Harvest(kind), tile_rect));
        } else {
            let left = kind.growth_days().saturating_sub(days);
            let w_status = if watered { "Regado" } else { "Seco" };
            let msg = format!("{} (Faltam {left} dia{} · {w_status})", kind.name(), if left == 1 { "" } else { "s" });
            return Some((Action::CropInfo(msg), tile_rect));
        }
    }
    // Bichos depois dos objetos e da plantação: eles andam e não devem roubar o E.
    if let Some((entity, species, gender, has_product, rect)) = s.animal {
        let action = if has_product && species.product(gender).is_some() {
            Action::CollectProduct(entity, species, gender)
        } else {
            Action::Pet(species, gender, has_product)
        };
        return Some((action, Some((rect, FocusKind::Person))));
    }
    if s.factory {
        return night.then_some((Action::Info("A fornalha ronrona, queimando almas."), tile_rect));
    }
    if !s.farm || s.prop_on_tile {
        return None;
    }
    let action = match (s.ground?, night) {
        (Ground::Grass, true) if game.has_mask => Action::BuildFactory,
        (Ground::Cracked, true) if game.has_mask => Action::BreakFloor,
        (Ground::Cracked, _) => Action::Info("Piso rachado. Algo bem pesado o quebraria."),
        (Ground::Cavern, true) if !game.mined_tonight => Action::Mine,
        (Ground::Cavern, true) => Action::Info("A caverna precisa de uma noite para brilhar."),
        _ => return None,
    };
    Some((action, tile_rect))
}

fn closed_door(b: Building) -> &'static str {
    match b {
        Building::Smithy => "Ferraria: \"Volte amanhã! Ou depois. Ou... quando eu for bom nisso.\"",
        Building::TownHall => "Prefeitura: um aviso diz \"Atendimento das 6h às 23h59. Sempre.\"",
        Building::Library => "Biblioteca: fechada para \"arrumação humilde do acervo\".",
        Building::Florist => "Floricultura: um perfume doce escapa pela fresta da porta.",
        Building::GuardPost => "Guarita: \"Não incomode. Estou com medo.\" — Xerife",
        Building::Bakery | Building::Store => "A porta está trancada.",
    }
}

/// Objetos com os quais dá para interagir de perto.
fn interactive(p: Prop) -> bool {
    matches!(p, Prop::Forge | Prop::Bed | Prop::Stand | Prop::Sign | Prop::Building(_) | Prop::Counter | Prop::Oven)
}

/// O bicho (pés em `feet`) está no tile à frente do jogador, com uma folguinha.
fn facing_animal(feet: Vec2, tile: IVec2) -> bool {
    Rect::from_center_size(tile_center(tile), Vec2::splat(TILE + 8.0)).contains(feet)
}

/// Retângulo do corpo de um personagem com a base em `p`.
fn body_rect(p: Vec2) -> Rect {
    Rect::new(p.x - 8.0, p.y, p.x + 8.0, p.y + 30.0)
}

#[allow(clippy::too_many_arguments)]
pub fn interact(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    (mouse, window): (Res<ButtonInput<MouseButton>>, Single<&Window, With<bevy::window::PrimaryWindow>>),
    sprites: Res<Sprites>,
    (phase, travel, mut overlay): (Res<State<DayPhase>>, Res<Travel>, ResMut<Overlay>),
    (mut game, mut inv, mut farm): (ResMut<Game>, ResMut<Inventory>, ResMut<FarmState>),
    (mut map, mut clock, mut dialogue, mut cutscenes): (
        ResMut<Map>,
        ResMut<Clock>,
        ResMut<Dialogue>,
        Option<ResMut<crate::cutscene::CutsceneManager>>,
    ),
    (mut focus, mut popups, mut pickups, mut target_qol): (
        ResMut<Focus>,
        ResMut<Popups>,
        ResMut<Pickups>,
        ResMut<TargetQol>,
    ),
    (mut images, layouts): (ResMut<Assets<Image>>, Res<Assets<TextureAtlasLayout>>),
    player: Single<(&Transform, &Player)>,
    villagers: Query<(&Villager, &Transform), Without<Player>>,
    inquisitors: Query<(Entity, &Transform), (With<Inquisitor>, Without<Player>)>,
    crop_views: Query<(Entity, &CropView)>,
    mut animals: Query<(Entity, &mut Animal, &Transform), Without<Player>>,
) {
    let pressed = keys.just_pressed(KeyCode::KeyE) || crate::player::world_click(&mouse, &window, MouseButton::Right);
    // Durante a cutscene quem cuida do E (e das falas) é ela.
    let in_cutscene = cutscenes.as_ref().is_some_and(|c| c.is_playing());
    if in_cutscene || dialogue.is_open() || *overlay != Overlay::None || travel.busy() {
        focus.0 = None;
        game.hint = None;
        target_qol.text = None;
        if pressed && !in_cutscene && dialogue.is_open() {
            if dialogue.is_typing() {
                dialogue.finish_typing();
            } else {
                dialogue.advance_page();
            }
        }
        return;
    }

    let night = *phase.get() == DayPhase::Night;
    let (transform, player) = *player;
    // O "pé" do jogador fica um pouco acima da base do sprite.
    let pos = transform.translation.truncate() + Vec2::new(0.0, 3.0);
    let tile = target_tile(transform, player.facing);
    // Moradores andam: o raio é generoso para dar tempo de apertar E.
    let near = |other: &Transform| other.translation.truncate().distance(pos) < 30.0;
    let on_farm = map.info.farm;
    let surroundings = Surroundings {
        tile,
        farm: map.info.farm,
        shop: map.info.shop,
        ground: map.ground(tile),
        crop: on_farm.then(|| farm.crops.get(&tile).map(|c| (c.kind, c.days, farm.watered.contains(&tile)))).flatten(),
        factory: on_farm && farm.factories.contains(&tile),
        prop_on_tile: map.occupied(tile),
        // Portas que levam a um interior não precisam de dica: basta entrar andando.
        prop: map
            .near(crate::map::world_to_tile(pos), |k| interactive(k))
            .filter(|p| !map.warps.contains_key(&p.anchor))
            .map(|p| (p.kind, p.rect())),
        villager: villagers
            .iter()
            .filter(|(_, t)| near(t))
            .map(|(v, t)| (v.kind, body_rect(t.translation.truncate())))
            .next(),
        inquisitor: inquisitors
            .iter()
            .find(|(_, t)| near(t))
            .map(|(e, t)| (e, body_rect(t.translation.truncate()))),
        // Só o bicho que está à sua frente (no tile para onde você olha): um cachorro do
        // lado não rouba o E da cama ou da plantação.
        animal: animals.iter().find(|(_, _, t)| facing_animal(t.translation.truncate(), tile)).map(|(e, a, t)| {
            let p = t.translation.truncate();
            (e, a.species, a.gender, a.has_product, Rect::new(p.x - 7.0, p.y, p.x + 7.0, p.y + 14.0))
        }),
    };

    if let Some((kind, days, watered)) = surroundings.crop {
        let pos = tile_center(surroundings.tile) + Vec2::new(0.0, 14.0);
        let text = if days >= kind.growth_days() {
            format!("{} (Pronto!)", kind.name())
        } else {
            let left = kind.growth_days().saturating_sub(days);
            let w_status = if watered { "Regado" } else { "Seco" };
            format!("{}: {}d ({w_status})", kind.name(), left)
        };
        target_qol.text = Some(text);
        target_qol.world_pos = pos;
    } else if let Some((_, species, gender, has_product, rect)) = surroundings.animal {
        let pos = Vec2::new(rect.center().x, rect.max.y + 8.0);
        let text = if has_product && species.product(gender).is_some() {
            let act = species.product_action(gender);
            format!("{act} {} (Pronto!)", species.name(gender))
        } else if species.product(gender).is_some() {
            format!("{}: Produto amanhã", species.name(gender))
        } else {
            format!("{}", species.name(gender))
        };
        target_qol.text = Some(text);
        target_qol.world_pos = pos;
    } else {
        target_qol.text = None;
    }

    let resolved = resolve(&surroundings, &game, &inv, night);
    game.hint = resolved.as_ref().map(|(a, _)| a.label(&game, &inv));
    focus.0 = resolved.as_ref().and_then(|(a, target)| target.map(|(r, kind)| (r, !matches!(a, Action::Info(_)), kind)));
    let Some((action, _)) = resolved else { return };
    if !pressed {
        return;
    }

    let before = (game.gold, inv.count(Item::Carrot) as i32, game.essence.floor() as i32);
    let head = transform.translation.truncate() + Vec2::new(0.0, 34.0);
    let here = map.id.clone();
    let game = &mut *game;
    match &action {
        Action::Talk(_) | Action::FeedGhoul => spawn_burst(&mut commands, &sprites, &here, head, fx::TALK, 1),
        _ => {}
    }
    match action {
        Action::Harvest(_) => {
            if let Some((item, quality)) = harvest(&mut commands, &sprites, &mut farm, &mut inv, &crop_views, &here, tile) {
                pickups.0.push((tile_center(tile), item.icon()));
                let badge = quality.badge();
                let pop_str = if !badge.is_empty() {
                    format!("+1 {} {}", item.name(), badge)
                } else {
                    format!("+1 {}", item.name())
                };
                popups.push(head, pop_str, Popups::GOOD);
                if quality == crate::inventory::Quality::Supernatural {
                    game.say(format!("Incrível! A {} colhida reluz com modificador SOBRENATURAL!", item.name()));
                } else if quality != crate::inventory::Quality::Normal {
                    game.say(format!("Colheu {} de qualidade {}! {}", item.name(), quality.name(), badge));
                }
            }
        }
        Action::CropInfo(msg) => game.say(msg),
        Action::ForgeMask => {
            if game.gold < MASK_COST {
                game.say(format!("A forja exige {MASK_COST} ouro em material amaldiçoado."));
                return;
            }
            game.gold -= MASK_COST;
            game.unlock_mask(MaskKind::Gargoyle);
            if let Some(cutscenes) = &mut cutscenes {
                cutscenes.play_id("first_mask", &[]);
            } else {
                dialogue.open(
                    "Forja",
                    "O metal esfria e ganha um rosto de pedra. Pesada. Fria. Perfeita para a noite.\nMáscara de Gárgula forjada!",
                );
            }
        }
        Action::BuildFactory => {
            let cost = game.factory_cost();
            if game.gold < cost {
                game.say(format!("Uma fornalha custa {cost} ouro. Venda mais."));
                return;
            }
            game.gold -= cost;
            game.factories += 1;
            farm.factories.push(tile);
            let view = spawn_factory_view(&mut commands, &sprites, &here, tile);
            commands.entity(view).insert(crate::fx::Pop::default());
            spawn_burst(&mut commands, &sprites, &here, tile_base(tile), fx::SMOKE, 4);
            game.say("Fornalha erguida. De dia, ninguém verá nada além de feno.");
        }
        Action::BreakFloor => {
            map.set_ground(&mut Painter { images: &mut images, layouts: &layouts }, &sprites, tile, Ground::Cavern);
            farm.edits.ground.insert(tile, Ground::Cavern);
            game.essence += 40.0;
            game.mined_tonight = true;
            spawn_burst(&mut commands, &sprites, &here, tile_base(tile), fx::DUST, 6);
            game.say("CRACK! O piso cedeu: uma caverna de minério que brilha. (+40 essência)");
        }
        Action::Mine => {
            game.essence += 15.0;
            game.mined_tonight = true;
            spawn_burst(&mut commands, &sprites, &here, tile_center(tile), fx::SPARKLE, 4);
            game.say("Você raspa as paredes da caverna. (+15 essência)");
        }
        Action::FeedGhoul => {
            inv.remove(Item::Carrot, 1);
            game.ghoul_fed += 1;
            game.essence += 12.0;
            let mut line = format!(
                "O Ghoul arranca a cenoura da sua mão e cospe essência brilhante. [{}/{GHOUL_GLUTTONY}]",
                game.ghoul_fed
            );
            if game.ghoul_fed == GHOUL_GLUTTONY {
                line.push_str("\n...amanhã a padaria não vai abrir.");
            }
            dialogue.open(Kind::Baker.name(true), &line);
        }
        Action::Pet(species, gender, _) => {
            spawn_burst(&mut commands, &sprites, &here, head, fx::HEART, 3);
            game.say(format!("Você faz carinho n{}. Que fofura.", species.name(gender)));
        }
        Action::CollectProduct(entity, species, gender) => {
            if let Ok((_, mut a, _)) = animals.get_mut(entity) {
                a.has_product = false;
            }
            if let Some(product) = species.product(gender) {
                let roll = ((clock.day as i32 * 29 + game.gold % 89 + entity.to_bits() as i32 * 13).abs() % 100) as u32;
                let quality = crate::inventory::Quality::roll(roll);
                inv.add_with_quality(product, 1, quality);
                spawn_burst(&mut commands, &sprites, &here, head, fx::HEART, 4);
                let badge = quality.badge();
                let pop_label = if !badge.is_empty() {
                    format!("+1 {} {}", product.name(), badge)
                } else {
                    format!("+1 {}", product.name())
                };
                popups.push(head, pop_label, Popups::GOOD);
                pickups.0.push((head, product.icon()));
                let qual_msg = if quality != crate::inventory::Quality::Normal {
                    format!(" (Qualidade {}! {})", quality.name(), badge)
                } else {
                    String::new()
                };
                game.say(format!("Você coletou {} d{}!{}", product.name(), species.name(gender), qual_msg));
            }
        }
        Action::Talk(kind) => {
            let season = clock.season();
            let pages = kind.seasonal_dialogue(season, night, game.has_mask, clock.day, game.curse);
            dialogue.open_pages(kind.name(night), pages);
        }
        Action::OpenShop(kind) => {
            if kind == Shop::Bakery && game.bakery_closed {
                dialogue.open(
                    Kind::Baker.name(false),
                    "Acordei tão cansado... e o estoque sumiu de novo! Hoje não tem nada, querido.",
                );
                return;
            }
            *overlay = Overlay::Shop(kind);
        }
        Action::PayInquisitor(inquisitor) => {
            let fine = game.fine();
            if game.gold < fine {
                if let Some(cutscenes) = &mut cutscenes {
                    cutscenes.play_id(
                        "inquisitor_visit",
                        &[("multa", fine.to_string()), ("maldicao", format!("{:.0}", game.curse))],
                    );
                } else {
                    dialogue.open("Inquisidor", &format!("A multa é de {fine} ouro. Eu espero. Eu sempre espero."));
                }
                return;
            }
            game.gold -= fine;
            game.add_curse(-35.0);
            game.inquisitor = false;
            commands.entity(inquisitor).despawn();
            dialogue.open("Inquisidor", "Livros carimbados. Contas em ordem. Por enquanto.");
        }
        Action::Nap => {
            clock.skip_to_phase_end();
            game.rest(40.0);
            game.say("Um cochilo... e o sol já está se pondo. (+40 energia)");
        }
        Action::Sleep => {
            clock.skip_to_phase_end();
            game.rest(MAX_ENERGY);
            game.say("Você dorme. A fazenda trabalha sozinha.");
        }
        Action::Info(text) => game.say(text),
    }

    // Números subindo em cima da cabeça: o retorno de cada ação.
    let after = (game.gold, inv.count(Item::Carrot) as i32, game.essence.floor() as i32);
    let deltas = [
        (after.0 - before.0, "ouro", Popups::GOLD),
        (after.1 - before.1, "cenoura", Popups::GOOD),
        (after.2 - before.2, "essência", Popups::ESSENCE),
    ];
    for (i, (delta, label, color)) in deltas.into_iter().filter(|(d, _, _)| *d != 0).enumerate() {
        let sign = if delta > 0 { "+" } else { "" };
        popups.push(head + Vec2::new(0.0, i as f32 * 9.0), format!("{sign}{delta} {label}"), color);
    }
}

fn dawn(
    mut commands: Commands,
    clock: Res<Clock>,
    sprites: Res<Sprites>,
    mut map: Option<ResMut<Map>>,
    mut game: ResMut<Game>,
    mut farm: ResMut<FarmState>,
    mut animals: Query<&mut Animal>,
    inquisitors: Query<Entity, With<Inquisitor>>,
    mut images: ResMut<Assets<Image>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
) {
    // Só cresce o que foi regado ontem; a terra seca de novo.
    let farm = &mut *farm;
    for (tile, crop) in farm.crops.iter_mut() {
        if farm.watered.contains(tile) {
            crop.days += 1;
        }
    }
    farm.watered.clear();
    game.rest(MAX_ENERGY);
    game.heal(MAX_HEALTH);

    // Animais da fazenda voltam a ter leite, ovos, lã
    for mut animal in &mut animals {
        animal.has_product = animal.species.product(animal.gender).is_some();
    }

    let prev_day = clock.day.saturating_sub(1);
    let old_season = Season::from_day(prev_day);
    let new_season = clock.season();
    let season_changed = old_season != new_season;

    if season_changed {
        let farm_layout = Layout::of("farm");
        let restored = farm.reforest(&farm_layout);
        game.say(new_season.greeting());
        if restored > 0 {
            game.say(format!("Reflorestamento: {} árvores brotaram novamente na fazenda!", restored));
        }
        if let Some(map) = map.as_deref_mut() {
            let mut painter = Painter { images: &mut images, layouts: &layouts };
            map.set_season(&mut painter, &sprites, new_season);
        }
    }

    let earned = game.gold - farm.gold_at_dawn;
    let badge = if season_changed {
        format!("\n{}", new_season.greeting())
    } else {
        String::new()
    };
    let summary = format!(
        "{} · Dia {}{}\nOntem: {}{} ouro · {} colheita{}",
        new_season.name(),
        clock.day_of_season(),
        badge,
        if earned >= 0 { "+" } else { "" },
        earned,
        farm.harvested_today,
        if farm.harvested_today == 1 { "" } else { "s" }
    );
    commands.insert_resource(DawnCard { text: summary, t: 0.0 });
    farm.harvested_today = 0;
    farm.gold_at_dawn = game.gold;

    game.bakery_closed = game.ghoul_fed >= GHOUL_GLUTTONY;
    game.ghoul_fed = 0;
    game.bread_today = false;
    game.mined_tonight = false;
    game.add_curse(-5.0);
    game.say(format!("{} · Dia {}. A vila acorda sem lembrar de nada.", new_season.name(), clock.day_of_season()));
    if game.bakery_closed {
        game.say("Fofoca: a padaria amanheceu sem estoque. Ninguém entende.");
    }

    if game.inquisitor && game.curse < INQUISITOR_LEAVES_AT {
        for entity in &inquisitors {
            commands.entity(entity).despawn();
        }
        game.inquisitor = false;
        game.say("O Inquisidor não encontrou nada e foi embora.");
    } else if !game.inquisitor && game.curse >= INQUISITOR_AT {
        game.inquisitor = true;
        // Fora da fazenda ele aparece quando o jogador voltar (spawn_farm_views).
        if let Some(map) = map.as_ref().filter(|m| m.info.farm) {
            spawn_inquisitor(&mut commands, &sprites, &map.id, inquisitor_spot(&map));
        }
        game.say("Um INQUISIDOR chegou à barraca! Preços pela metade até a multa.");
    }
}

fn dusk(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut farm: ResMut<FarmState>,
    views: Query<(Entity, &FactoryView)>,
) {
    game.say("A noite cai. A verdade acorda.");
    if !game.has_mask {
        game.say("Sem máscara, você só observa. A forja fica em casa.");
    }
    if game.curse >= NEMESIS_AT && !farm.factories.is_empty() {
        let tile = farm.factories.remove(0);
        for (entity, view) in &views {
            if view.0 == tile {
                commands.entity(entity).despawn();
            }
        }
        game.factories -= 1;
        game.say("NÊMESIS: um Ghoul radicalizado sabotou uma de suas fornalhas!");
    }
}

fn check_goal(mut game: ResMut<Game>) {
    if !game.goal_reached && game.gold >= GOAL {
        game.goal_reached = true;
        game.say(format!("{GOAL} de ouro. A vila sorri. A noite trabalha. O império é seu."));
    }
}

/// Mantém os recursos do jogo vivos só durante o jogo.
pub fn cleanup(mut commands: Commands) {
    commands.remove_resource::<FarmState>();
    commands.remove_resource::<Map>();
}

/// Cartão do amanhecer: o dia novo e um resumo de ontem (some sozinho).
#[derive(Resource)]
pub struct DawnCard {
    pub text: String,
    pub t: f32,
}
