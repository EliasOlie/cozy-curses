//! Mapas em tiles 16x16 vistos de cima (top-down, como Stardew Valley).
//! Cada mapa é um .txt em assets/maps (um caractere por tile; legenda em `build`).

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy::sprite::Anchor;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::bake::{Painter, Rgba};
use crate::cycle::{Clock, Season};
use crate::terrain::{self, TexCache};
use crate::character::Facing;
use crate::lighting::LightSource;
use crate::worlddef::{self, MapInfo};
use crate::sprites::{Art, DualSprite, GRASS_VARIANTS, PATH_VARIANTS, Sprites};

pub fn plugin(app: &mut App) {
    app.add_sub_state::<MapState>()
        .add_systems(OnEnter(AppState::Menu), spawn_scenery)
        .add_systems(Update, (animate_water, spin, sync_flora_seasons));
}

/// Tipo de flora para adaptação sazonal (copas de árvores, arbustos, flores).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum FloraKind {
    Oak,
    Pine,
    BigTree,
    Bush,
    Decor,
}

impl FloraKind {
    pub fn tint(&self, season: Season) -> Color {
        match self {
            FloraKind::Oak | FloraKind::BigTree => match season {
                Season::Spring => Color::srgb(1.0, 1.05, 0.98),
                Season::Summer => Color::srgb(0.98, 1.0, 0.94),
                Season::Autumn => Color::srgb(1.35, 0.76, 0.38),
                Season::Winter => Color::srgb(0.92, 0.98, 1.18),
            },
            FloraKind::Pine => match season {
                Season::Winter => Color::srgb(0.88, 0.95, 1.15),
                Season::Autumn => Color::srgb(1.08, 0.96, 0.86),
                _ => Color::WHITE,
            },
            FloraKind::Bush => match season {
                Season::Autumn => Color::srgb(1.28, 0.82, 0.44),
                Season::Winter => Color::srgb(0.90, 0.95, 1.12),
                _ => Color::WHITE,
            },
            FloraKind::Decor => match season {
                Season::Spring => Color::srgb(1.05, 1.08, 1.0),
                Season::Summer => Color::WHITE,
                Season::Autumn => Color::srgb(1.2, 0.9, 0.6),
                Season::Winter => Color::srgb(0.85, 0.92, 1.14),
            },
        }
    }
}

pub fn sync_flora_seasons(
    clock: Option<Res<Clock>>,
    mut query: Query<(&FloraKind, &mut Sprite)>,
) {
    let season = clock.as_ref().map_or(Season::Spring, |c| c.season());
    for (flora, mut sprite) in &mut query {
        // Só a cor: a transparência é de `fade_occluders` (copa translúcida com o jogador
        // atrás). Antes os dois sistemas brigavam pelo alfa e a árvore piscava.
        let wanted = flora.tint(season).with_alpha(sprite.color.alpha());
        if sprite.color != wanted {
            sprite.color = wanted;
        }
    }
}

pub const TILE: f32 = 16.0;

/// Mapa onde o jogador está (o id do world.json). Vazio enquanto o jogo começa.
#[derive(SubStates, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Playing)]
pub struct MapState(pub String);

fn default_ground(info: &MapInfo) -> Ground {
    if info.indoor { Ground::Floor } else { Ground::Grass }
}

/// O .txt de um mapa já separado em linhas.
#[derive(Clone)]
pub struct Layout {
    pub size: IVec2,
    grid: Vec<Vec<char>>,
}

impl Layout {
    /// O mapa como está valendo (o que o editor salvou ou o que veio com o jogo).
    pub fn of(id: &str) -> Layout {
        worlddef::map_text(id)
            .and_then(|text| Layout::parse(&text).ok())
            .unwrap_or_else(|| Layout::blank(IVec2::new(16, 12), worlddef::info(id).blank()))
    }

    /// O original que veio com o jogo (mapas novos voltam a ficar vazios).
    pub fn original(id: &str) -> Layout {
        worlddef::original_text(id)
            .and_then(|text| Layout::parse(text).ok())
            .unwrap_or_else(|| Layout::blank(IVec2::new(16, 12), worlddef::info(id).blank()))
    }

    pub fn blank(size: IVec2, fill: char) -> Layout {
        Layout { size, grid: vec![vec![fill; size.x as usize]; size.y as usize] }
    }

    pub fn parse(text: &str) -> Result<Layout, String> {
        let grid: Vec<Vec<char>> =
            text.lines().filter(|l| !l.trim().is_empty()).map(|l| l.trim_end().chars().collect()).collect();
        let Some(first) = grid.first() else { return Err("mapa vazio".into()) };
        let size = IVec2::new(first.len() as i32, grid.len() as i32);
        for (y, row) in grid.iter().enumerate() {
            if row.len() as i32 != size.x {
                return Err(format!("linha {} tem {} colunas, a primeira tem {}", y + 1, row.len(), size.x));
            }
        }
        Ok(Layout { size, grid })
    }

    pub fn to_text(&self) -> String {
        let mut text: String =
            self.grid.iter().map(|row| row.iter().collect::<String>()).collect::<Vec<_>>().join("\n");
        text.push('\n');
        text
    }

    pub fn inside(&self, t: IVec2) -> bool {
        t.x >= 0 && t.y >= 0 && t.x < self.size.x && t.y < self.size.y
    }

    pub fn set(&mut self, t: IVec2, ch: char) {
        if self.inside(t) {
            self.grid[t.y as usize][t.x as usize] = ch;
        }
    }

    /// Muda o tamanho preenchendo o espaço novo com `fill`.
    pub fn resize(&mut self, size: IVec2, fill: char) {
        let size = size.max(IVec2::splat(4));
        self.grid.resize(size.y as usize, vec![fill; self.size.x as usize]);
        for row in &mut self.grid {
            row.resize(size.x as usize, fill);
        }
        self.size = size;
    }

    /// Caractere do tile; fora do mapa conta como floresta.
    pub fn at(&self, t: IVec2) -> char {
        if t.x < 0 || t.y < 0 || t.x >= self.size.x || t.y >= self.size.y {
            return 'T';
        }
        self.grid[t.y as usize][t.x as usize]
    }

    fn find_all(&self, ch: char) -> Vec<IVec2> {
        let mut found = Vec::new();
        for y in 0..self.size.y {
            for x in 0..self.size.x {
                if self.at(IVec2::new(x, y)) == ch {
                    found.push(IVec2::new(x, y));
                }
            }
        }
        found
    }

    /// O do meio, quando o caractere se repete (saídas pela borda).
    pub fn find(&self, ch: char) -> Option<IVec2> {
        let all = self.find_all(ch);
        all.get(all.len() / 2).copied()
    }
}

/// O que mudou num mapa durante o jogo (só a fazenda muda: terra arada, objetos removidos).
#[derive(Default, Clone)]
pub struct MapEdits {
    pub ground: HashMap<IVec2, Ground>,
    pub removed: HashSet<IVec2>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Ground {
    Grass,
    Path,
    Soil,
    Water,
    Cracked,
    Cavern,
    Cobble,
    Floor,
    Rug,
    Wall,
    WallTop,
    Mat,
    /// Píer de tábuas: anda-se por cima da água.
    Dock,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Building {
    Bakery,
    Store,
    Smithy,
    TownHall,
    Library,
    Florist,
    GuardPost,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prop {
    Oak,
    Pine,
    Rock,
    Bush,
    Fence,
    Sign,
    House,
    Stand,
    Building(Building),
    Fountain,
    Lamppost,
    Bench,
    Barrel,
    Crates,
    Planter,
    Bed,
    Forge,
    Table,
    Oven,
    Counter,
    Shelf,
    Plant,
    Barn,
    Coop,
    Windmill,
    Cart,
    HayBale,
    HayStack,
    Rowboat,
    BigTree,
}

/// Como a pegada se estende a partir do tile marcado no mapa.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    /// Marcador no meio da fileira de baixo (portas de prédios, chafariz, cama).
    BottomCenter,
    /// Marcador no tile da esquerda (móveis, bancos, barraca).
    Left,
}

/// (objeto, largura, altura em tiles, origem). Contrato com art/CONTRACT.md.
fn prop_spec(ch: char) -> Option<(Prop, i32, i32, Origin)> {
    use Origin::*;
    Some(match ch {
        'T' => (Prop::Oak, 1, 1, Left),
        'P' => (Prop::Pine, 1, 1, Left),
        'r' => (Prop::Rock, 1, 1, Left),
        'B' => (Prop::Bush, 1, 1, Left),
        'f' => (Prop::Fence, 1, 1, Left),
        'g' => (Prop::Sign, 1, 1, Left),
        'H' => (Prop::House, 5, 3, BottomCenter),
        'S' => (Prop::Stand, 2, 1, Left),
        '1' => (Prop::Building(Building::Bakery), 5, 2, BottomCenter),
        '2' => (Prop::Building(Building::Store), 5, 2, BottomCenter),
        '3' => (Prop::Building(Building::Smithy), 5, 2, BottomCenter),
        '4' => (Prop::Building(Building::TownHall), 7, 3, BottomCenter),
        '5' => (Prop::Building(Building::Library), 5, 3, BottomCenter),
        '6' => (Prop::Building(Building::Florist), 3, 2, BottomCenter),
        '7' => (Prop::Building(Building::GuardPost), 3, 2, BottomCenter),
        'W' => (Prop::Fountain, 3, 2, BottomCenter),
        'L' => (Prop::Lamppost, 1, 1, Left),
        'n' => (Prop::Bench, 2, 1, Left),
        'u' => (Prop::Barrel, 1, 1, Left),
        'x' => (Prop::Crates, 1, 1, Left),
        'y' => (Prop::Planter, 1, 1, Left),
        'Z' => (Prop::Bed, 1, 2, BottomCenter),
        'F' => (Prop::Forge, 2, 1, Left),
        'Y' => (Prop::Table, 2, 1, Left),
        'O' => (Prop::Oven, 2, 1, Left),
        'K' => (Prop::Counter, 3, 1, Left),
        'X' => (Prop::Shelf, 2, 1, Left),
        'V' => (Prop::Plant, 1, 1, Left),
        'A' => (Prop::Barn, 5, 3, BottomCenter),
        'C' => (Prop::Coop, 3, 2, BottomCenter),
        'M' => (Prop::Windmill, 3, 2, BottomCenter),
        'J' => (Prop::Cart, 3, 1, Left),
        'b' => (Prop::HayBale, 1, 1, Left),
        'G' => (Prop::HayStack, 2, 1, Left),
        'k' => (Prop::Rowboat, 2, 1, Left),
        'U' => (Prop::BigTree, 1, 1, Left),
        _ => return None,
    })
}

/// Que objeto o caractere desenha (None se for chão ou marcador).
pub fn prop_kind(ch: char) -> Option<Prop> {
    prop_spec(ch).map(|(kind, ..)| kind)
}

/// Tiles ocupados pelo objeto marcado com `ch` em `t` (None se não for objeto).
pub fn footprint_of(ch: char, t: IVec2) -> Option<Vec<IVec2>> {
    let (_, w, h, origin) = prop_spec(ch)?;
    let xs = match origin {
        Origin::BottomCenter => -(w / 2)..=(w / 2),
        Origin::Left => 0..=(w - 1),
    };
    Some(xs.flat_map(|dx| (-(h - 1)..=0).map(move |dy| t + IVec2::new(dx, dy))).collect())
}

pub struct PlacedProp {
    pub kind: Prop,
    /// Tile marcado no mapa (porta, no caso dos prédios).
    pub anchor: IVec2,
    pub footprint: Vec<IVec2>,
    /// Entidades que desenham o objeto (a cerca tem várias peças).
    pub entities: Vec<Entity>,
}

impl PlacedProp {
    /// Retângulo da pegada no mundo (para o cursor de interação).
    pub fn rect(&self) -> Rect {
        let mut r = Rect::from_center_size(tile_center(self.anchor), Vec2::splat(TILE));
        for t in &self.footprint {
            r = r.union(Rect::from_center_size(tile_center(*t), Vec2::splat(TILE)));
        }
        r
    }
}

/// Destino de uma porta ou saída.
#[derive(Clone, Debug)]
pub struct Warp {
    pub to: String,
    pub arrive: IVec2,
    pub facing: Facing,
}

#[derive(Resource)]
pub struct Map {
    pub id: String,
    pub info: MapInfo,
    pub size: IVec2,
    pub season: Season,
    ground: Vec<Ground>,
    /// Canteiros regados hoje (terra mais escura no chão assado).
    pub wet: HashSet<IVec2>,
    /// O chão "assado" em uma imagem: [dia, noite].
    baked: [Handle<Image>; 2],
    /// O mapa em caracteres, com as edições do jogo aplicadas.
    layout: Layout,
    /// Enfeites no chão (tufos, flores), que a foice corta.
    decor: HashMap<IVec2, Entity>,
    /// Caixas de colisão dos objetos, no mundo (do tamanho da base do desenho).
    colliders: Vec<Rect>,
    /// Grade espacial: tile -> colisores que encostam nele (para não testar todos).
    grid: HashMap<IVec2, Vec<usize>>,
    pub props: Vec<PlacedProp>,
    /// Tiles que levam a outro mapa (portas, capachos, bordas).
    pub warps: HashMap<IVec2, Warp>,
    /// `@`: onde o jogador acorda.
    pub spawn: Option<IVec2>,
    /// `N`: onde fica quem atende no balcão.
    pub keeper_spot: Option<IVec2>,
    /// Onde nascem os bichos: `a` galinha, `e` ovelha, `j` cachorro.
    pub animals: Vec<(char, IVec2)>,
    /// Pontos (no mundo) de onde sai fumaça.
    pub chimneys: Vec<Vec2>,
}

impl Map {
    fn index(&self, t: IVec2) -> Option<usize> {
        (t.x >= 0 && t.y >= 0 && t.x < self.size.x && t.y < self.size.y)
            .then(|| (t.y * self.size.x + t.x) as usize)
    }

    pub fn ground(&self, t: IVec2) -> Option<Ground> {
        self.index(t).map(|i| self.ground[i])
    }

    pub fn set_ground(&mut self, painter: &mut Painter, sprites: &Sprites, t: IVec2, g: Ground) {
        if let Some(i) = self.index(t) {
            self.ground[i] = g;
            self.repaint(painter, sprites, &[t]);
        }
    }

    /// Repinta tiles no chão assado (e os vizinhos: as bordas orgânicas passam de um
    /// tile para o outro).
    pub fn repaint(&self, painter: &mut Painter, sprites: &Sprites, tiles: &[IVec2]) {
        let mut area: Vec<IVec2> = tiles
            .iter()
            .flat_map(|t| (-1..=1).flat_map(move |dy| (-1..=1).map(move |dx| *t + IVec2::new(dx, dy))))
            .filter(|t| self.index(*t).is_some())
            .collect();
        area.sort_by_key(|t| (t.y, t.x));
        area.dedup();
        self.render_baked_tiles(painter, sprites, &area);
    }

    /// Repinta exatamente os tiles passados (sem os 8 vizinhos).
    /// Usado para rega (wet_soil), onde apenas o próprio canteiro escurece sem mudar bordas de terreno.
    pub fn repaint_exact(&self, painter: &mut Painter, sprites: &Sprites, tiles: &[IVec2]) {
        let mut area: Vec<IVec2> = tiles.iter().copied().filter(|t| self.index(*t).is_some()).collect();
        area.sort_by_key(|t| (t.y, t.x));
        area.dedup();
        self.render_baked_tiles(painter, sprites, &area);
    }

    fn render_baked_tiles(&self, painter: &mut Painter, sprites: &Sprites, area: &[IVec2]) {
        let ground = |t: IVec2| self.ground(t);
        let texture = |g: Ground, t: IVec2| ground_art(sprites, g, t);
        let base = default_ground(&self.info);
        let season = self.season;
        for (night, dst) in [false, true].into_iter().zip(&self.baked) {
            let blocks: Vec<(IVec2, Vec<Rgba>)> = {
                let mut cache = TexCache::new(painter.images, painter.layouts, night);
                area.iter()
                    .map(|&t| {
                        let wet = self.wet.contains(&t);
                        let px = (0..TILE as u32 * TILE as u32)
                            .map(|i| terrain::pixel(&mut cache, &ground, &texture, base, t, i % 16, i / 16, wet, season))
                            .collect();
                        (t, px)
                    })
                    .collect()
            };
            for (t, px) in blocks {
                painter.write_block(dst, (t.as_uvec2()) * TILE as u32, TILE as u32, &px);
            }
        }
    }

    /// Atualiza a estação e repinta todo o chão assado com a nova paleta sazonal.
    pub fn set_season(&mut self, painter: &mut Painter, sprites: &Sprites, season: Season) {
        if self.season != season {
            self.season = season;
            let all: Vec<IVec2> = self.tiles().collect();
            self.repaint(painter, sprites, &all);
        }
    }

    /// Chão que dá para pisar (água e paredes não; objetos são vistos em `solid`).
    pub fn walkable(&self, t: IVec2) -> bool {
        matches!(self.ground(t), Some(g) if !matches!(g, Ground::Water | Ground::Wall | Ground::WallTop))
    }

    /// A caixa (no mundo) bate em chão intransponível ou em algum objeto?
    pub fn solid(&self, r: Rect) -> bool {
        let corners = [r.min, Vec2::new(r.max.x, r.min.y), Vec2::new(r.min.x, r.max.y), r.max];
        if corners.iter().any(|c| !self.walkable(world_to_tile(*c))) {
            return true;
        }
        let (lo, hi) = (world_to_tile(Vec2::new(r.min.x, r.max.y)), world_to_tile(Vec2::new(r.max.x, r.min.y)));
        (lo.y..=hi.y).any(|y| {
            (lo.x..=hi.x).any(|x| {
                self.grid.get(&IVec2::new(x, y)).is_some_and(|ids| {
                    ids.iter().any(|&i| !self.colliders[i].intersect(r).is_empty())
                })
            })
        })
    }

    /// Tile ocupado por um objeto (não dá para plantar nem construir ali).
    pub fn occupied(&self, t: IVec2) -> bool {
        self.prop_at(t).is_some()
    }

    pub fn prop_at(&self, t: IVec2) -> Option<&PlacedProp> {
        self.props.iter().find(|p| p.footprint.contains(&t))
    }

    /// Tira um objeto do mapa (árvore cortada, pedra quebrada) e devolve a âncora dele.
    pub fn remove_prop(&mut self, commands: &mut Commands, t: IVec2) -> Option<IVec2> {
        let i = self.props.iter().position(|p| p.footprint.contains(&t))?;
        let prop = self.props.remove(i);
        for entity in prop.entities {
            commands.entity(entity).despawn();
        }
        self.layout.set(prop.anchor, '.');
        self.rebuild_colliders();
        Some(prop.anchor)
    }

    /// Corta o enfeite do tile, se houver.
    pub fn remove_decor(&mut self, commands: &mut Commands, t: IVec2) -> bool {
        let Some(entity) = self.decor.remove(&t) else { return false };
        commands.entity(entity).despawn();
        self.layout.set(t, '.');
        true
    }

    pub fn decor_at(&self, t: IVec2) -> Option<Entity> {
        self.decor.get(&t).copied()
    }

    pub fn has_decor(&self, t: IVec2) -> bool {
        self.decor.contains_key(&t)
    }


    fn rebuild_colliders(&mut self) {
        // Portas com interior ficam com uma abertura: entrar nelas troca de mapa.
        self.colliders = self
            .props
            .iter()
            .flat_map(|p| colliders_of(p, &self.layout, self.warps.contains_key(&p.anchor)))
            .collect();
        self.grid.clear();
        for (i, c) in self.colliders.iter().enumerate() {
            let (lo, hi) = (world_to_tile(Vec2::new(c.min.x, c.max.y)), world_to_tile(Vec2::new(c.max.x, c.min.y)));
            for y in lo.y..=hi.y {
                for x in lo.x..=hi.x {
                    self.grid.entry(IVec2::new(x, y)).or_default().push(i);
                }
            }
        }
    }

    /// Todos os tiles do mapa.
    pub fn tiles(&self) -> impl Iterator<Item = IVec2> + '_ {
        (0..self.size.y).flat_map(move |y| (0..self.size.x).map(move |x| IVec2::new(x, y)))
    }

    pub fn find(&self, kind: Prop) -> Option<&PlacedProp> {
        self.props.iter().find(|p| p.kind == kind)
    }

    /// Objeto do tipo pedido colado (inclusive na diagonal) no tile.
    pub fn near(&self, t: IVec2, wanted: impl Fn(Prop) -> bool) -> Option<&PlacedProp> {
        self.props.iter().filter(|p| wanted(p.kind)).find(|p| {
            p.footprint.iter().any(|f| {
                let d = (*f - t).abs();
                d.x <= 1 && d.y <= 1
            })
        })
    }
}

/// Centro do tile no mundo. O eixo y do mapa cresce para baixo; o do mundo, para cima.
pub fn tile_center(t: IVec2) -> Vec2 {
    Vec2::new(t.x as f32 * TILE + TILE / 2.0, -(t.y as f32 * TILE + TILE / 2.0))
}

/// Ponto do meio da borda de baixo do tile: onde objetos "pisam".
pub fn tile_base(t: IVec2) -> Vec2 {
    tile_center(t) - Vec2::new(0.0, TILE / 2.0)
}

pub fn world_to_tile(p: Vec2) -> IVec2 {
    IVec2::new((p.x / TILE).floor() as i32, (-p.y / TILE).floor() as i32)
}

/// Ordenação top-down: quanto mais baixo na tela, mais na frente.
pub fn depth(y: f32) -> f32 {
    10.0 - y * 0.001
}

/// Transform de algo em pé com a base no ponto `p`.
pub fn standing_at(p: Vec2) -> Transform {
    // Desempate por x: vizinhos na mesma linha (copas que se sobrepõem) ficariam com a
    // mesma profundidade e a ordem de desenho trocaria de um quadro para o outro.
    Transform::from_xyz(p.x, p.y, depth(p.y) + p.x * 2e-7)
}

/// Altura das sombras de chão: acima do chão e da água, abaixo de tudo que fica em pé.
const SHADOW_Z: f32 = 0.5;

/// Sombra de chão de um objeto: (larga?, deslocamento vertical a partir da base).
fn ground_shadow(kind: Prop) -> Option<(bool, f32)> {
    match kind {
        Prop::Oak | Prop::BigTree => Some((true, 2.0)),
        Prop::Pine => Some((true, 3.0)),
        Prop::Rock | Prop::Bush | Prop::Barrel | Prop::Crates | Prop::HayBale | Prop::Planter | Prop::Sign
        | Prop::Lamppost => Some((false, 2.0)),
        _ => None,
    }
}

/// Tamanho do mapa em unidades do mundo.
pub fn world_size(size: IVec2) -> Vec2 {
    size.as_vec2() * TILE
}

/// Inteiro pseudoaleatório estável por tile (variações de grama etc.).
pub fn tile_hash(t: IVec2, salt: u32) -> u32 {
    let mut h = (t.x as u32).wrapping_mul(0x9E37_79B1) ^ (t.y as u32).wrapping_mul(0x85EB_CA77) ^ salt;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 13)
}

fn ground_art(sprites: &Sprites, g: Ground, t: IVec2) -> (Art, usize) {
    match g {
        Ground::Grass => {
            // A variação 0 (lisa) domina; as outras salpicam o campo.
            let index = match tile_hash(t, 1) % 20 {
                0..11 => 0,
                11..15 => 1,
                15..18 => 2,
                _ => 3,
            };
            (sprites.grass.clone(), index.min(GRASS_VARIANTS - 1))
        }
        Ground::Path => (sprites.path.clone(), tile_hash(t, 2) as usize % PATH_VARIANTS),
        Ground::Soil => (sprites.soil.clone(), 0),
        Ground::Water => (sprites.water.clone(), 0),
        Ground::Cracked => (sprites.cracked.clone(), 0),
        Ground::Cavern => (sprites.cavern.clone(), 0),
        Ground::Cobble => (sprites.cobble.clone(), (tile_hash(t, 4) % 3 == 0) as usize),
        Ground::Floor => (sprites.floor.clone(), (tile_hash(t, 5) % 4 == 0) as usize),
        Ground::Rug => (sprites.rug.clone(), 0),
        Ground::Wall => (sprites.wall.clone(), 0),
        Ground::WallTop => (sprites.wall.clone(), 1),
        Ground::Mat => (sprites.doormat.clone(), 0),
        Ground::Dock => (sprites.dock.clone(), 0),
    }
}


/// Objeto alto que fica translúcido quando o jogador passa por trás dele.
#[derive(Component)]
pub struct Occluder {
    pub half_width: f32,
    pub height: f32,
}

/// Árvores e enfeites que balançam com o vento (ver fx.rs).
#[derive(Component)]
pub struct Sway {
    /// Quadro parado; o balanço usa `base + offset`.
    pub base: usize,
    pub kind: SwayKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SwayKind {
    /// Árvores: quadros 0 (parado), 1 (esquerda), 2 (direita).
    Tree,
    /// Enfeites: a versão balançando fica tantos quadros adiante.
    Decor(usize),
}

/// Pás do moinho girando (4 quadros).
#[derive(Component)]
pub struct Spin;

/// Enfeite de chão (sem colisão): arte, quadro e quantos quadros adiante fica a versão
/// balançando (se balançar).
fn decor_of(sprites: &Sprites, ch: char) -> Option<(Art, usize, Option<usize>)> {
    if let Some(index) = decor_index(ch) {
        return Some((sprites.decor.clone(), index, Some(6)));
    }
    Some(match ch {
        'i' => (sprites.reeds.clone(), 0, Some(1)),
        'z' => (sprites.pumpkin.clone(), 0, None),
        'l' => (sprites.flowerbed.clone(), 0, None),
        _ => return None,
    })
}

/// Quadros da spritesheet `decor` (contrato com art/sprites/decor.json).
fn decor_index(ch: char) -> Option<usize> {
    Some(match ch {
        ',' => 0,
        ';' => 1,
        '*' => 2,
        '+' => 3,
        'o' => 4,
        'm' => 5,
        _ => return None,
    })
}

/// Monta um mapa, aplicando mudanças de chão (ex.: piso que virou caverna).
/// Todas as entidades recebem `scope` (some junto com o mapa).
///
/// Legenda:
/// - chão: `.` grama · `p` caminho · `s` canteiro · `w` água · `c` piso rachado · `q` pedra
///   `_` assoalho · `=` tapete · `#` parede · `%` borda da parede · `E` capacho (saída)
/// - fazenda/vila: `T` carvalho · `P` pinheiro · `r` pedra · `B` arbusto · `f` cerca · `g` placa
///   `H` porta da casa · `S` barraca · `1`-`7` portas da padaria, armazém, ferraria, prefeitura,
///   biblioteca, floricultura, guarita · `W` chafariz · `L` poste · `n` banco · `u` barril
///   `x` caixotes · `y` floreira · `A` celeiro · `C` galinheiro · `M` moinho · `J` carroça
///   `b` fardo · `G` pilha de feno · `d` píer · `k` barco · `i` taboas · `U` árvore grande
///   `z` abóbora · `l` flores · `a` `e` `j` galinha, ovelha e cachorro (onde nascem)
///   · `<` `>` saídas para a vila / a fazenda
/// - interiores: `Z` cama · `F` forja · `Y` mesa · `O` forno · `K` balcão · `X` prateleira
///   `V` vaso · `@` onde o jogador acorda · `N` quem atende no balcão
/// - `h` reservado (chão comum sob prédios e móveis)
/// - enfeites: `,` tufo · `;` tufo alto · `*` flores rosa · `+` flores amarelas · `o` pedrinhas
///   `m` cogumelos
pub fn build(
    commands: &mut Commands,
    sprites: &Sprites,
    painter: &mut Painter,
    info: &MapInfo,
    layout: &Layout,
    scope: impl Bundle + Clone,
    edits: &MapEdits,
    season: Season,
) -> Map {
    // Objetos removidos (árvore cortada, pedra quebrada) viram grama.
    let mut layout = layout.clone();
    for t in &edits.removed {
        layout.set(*t, '.');
    }
    let changes = &edits.ground;
    let size = layout.size;
    let mut map = Map {
        id: info.id.clone(),
        info: info.clone(),
        size,
        season,
        ground: Vec::new(),
        wet: HashSet::new(),
        baked: [Handle::default(), Handle::default()],
        layout: Layout { size, grid: Vec::new() },
        decor: HashMap::new(),
        colliders: Vec::new(),
        grid: HashMap::new(),
        props: Vec::new(),
        warps: HashMap::new(),
        spawn: None,
        keeper_spot: None,
        animals: Vec::new(),
        chimneys: Vec::new(),
    };

    for y in 0..size.y {
        for x in 0..size.x {
            let t = IVec2::new(x, y);
            let ch = layout.at(t);
            let base = match ground_of(ch) {
                Some(g) => g,
                // Embaixo de objetos, o chão vem dos vizinhos (poste no meio da praça).
                None if prop_spec(ch).is_some() || matches!(ch, 'h' | '@' | 'N') => {
                    under(&layout, t, default_ground(info))
                }
                None => default_ground(info),
            };
            map.ground.push(changes.get(&t).copied().unwrap_or(base));

            if let (Some((kind, ..)), Some(footprint)) = (prop_spec(ch), footprint_of(ch, t)) {
                map.props.push(PlacedProp { kind, anchor: t, footprint, entities: Vec::new() });
            }
            if let Some(warp) = warp_for(&info.id, ch, t) {
                map.warps.insert(t, warp);
            }
            match ch {
                '@' => map.spawn = Some(t),
                'N' => map.keeper_spot = Some(t),
                _ => {}
            }
            if let Some((art, index, sway)) = decor_of(sprites, ch) {
                let mut entity = commands.spawn((
                    DualSprite::bundle(&art, index),
                    Transform::from_translation(tile_center(t).extend(0.2)),
                    scope.clone(),
                    FloraKind::Decor,
                ));
                if let Some(offset) = sway {
                    entity.insert(Sway { base: index, kind: SwayKind::Decor(offset) });
                }
                map.decor.insert(t, entity.id());
            }
            if matches!(ch, 'a' | 'e' | 'j') {
                map.animals.push((ch, t));
            }
        }
    }

    // Chão assado: uma imagem por fase, ancorada no canto de cima à esquerda do mapa,
    // com as bordas orgânicas de caminho, água e canteiro já desenhadas.
    let px = (size.as_vec2() * TILE).as_uvec2();
    map.baked = [painter.canvas(px), painter.canvas(px)];
    let all: Vec<IVec2> = map.tiles().collect();
    map.repaint(painter, sprites, &all);
    let baked = Art { day: map.baked[0].clone(), night: map.baked[1].clone(), layout: None };
    commands.spawn((DualSprite::bundle(&baked, 0), Anchor::TOP_LEFT, Transform::default(), scope.clone()));
    spawn_water(commands, sprites, painter, &map, &scope);

    let mut chimneys = Vec::new();
    for prop in &mut map.props {
        prop.entities = spawn_prop(commands, sprites, prop, &layout, &scope, &mut chimneys);
    }
    map.chimneys = chimneys;
    map.layout = layout;
    map.rebuild_colliders();
    map
}

/// Caixa de colisão (largura, altura em pixels) na base de cada objeto: só o "pé" do
/// desenho bloqueia — copas, telhados e encostos ficam de fora.
fn collider_size(kind: Prop) -> Vec2 {
    let (w, h) = match kind {
        Prop::Oak => (10.0, 6.0),
        Prop::Pine => (8.0, 6.0),
        Prop::Rock => (12.0, 7.0),
        Prop::Bush => (13.0, 8.0),
        Prop::Sign | Prop::Lamppost => (6.0, 5.0),
        Prop::House => (74.0, 38.0),
        Prop::Stand => (30.0, 12.0),
        Prop::Building(Building::Bakery | Building::Store | Building::Smithy) => (74.0, 24.0),
        Prop::Building(Building::TownHall) => (106.0, 40.0),
        Prop::Building(Building::Library) => (74.0, 40.0),
        Prop::Building(Building::Florist | Building::GuardPost) => (42.0, 24.0),
        Prop::Fountain => (42.0, 20.0),
        Prop::Bench => (28.0, 6.0),
        Prop::Barrel => (12.0, 9.0),
        Prop::Crates => (14.0, 10.0),
        Prop::Planter => (14.0, 8.0),
        Prop::Bed => (14.0, 28.0),
        Prop::Forge | Prop::Table | Prop::Shelf => (30.0, 12.0),
        Prop::Oven => (30.0, 14.0),
        Prop::Counter => (46.0, 12.0),
        Prop::Plant => (10.0, 6.0),
        Prop::Barn => (74.0, 38.0),
        Prop::Coop => (42.0, 24.0),
        Prop::Windmill => (40.0, 22.0),
        Prop::Cart => (44.0, 12.0),
        Prop::HayBale => (14.0, 10.0),
        Prop::HayStack => (30.0, 12.0),
        // O barco fica na água, que já não deixa passar.
        Prop::Rowboat => (0.0, 0.0),
        Prop::BigTree => (12.0, 8.0),
        Prop::Fence => (6.0, 6.0),
    };
    Vec2::new(w, h)
}

fn colliders_of(prop: &PlacedProp, layout: &Layout, door: bool) -> Vec<Rect> {
    let footprint = prop.rect();
    let base = Vec2::new(footprint.center().x, footprint.min.y);
    let size = collider_size(prop.kind);
    let body = Rect::new(base.x - size.x / 2.0, base.y, base.x + size.x / 2.0, base.y + size.y);
    if prop.kind == Prop::Fence {
        // Mourão + uma travessa na direção de cada cerca vizinha.
        let c = tile_center(prop.anchor);
        let mut rects = vec![body];
        let rail = |dx: f32, dy: f32| {
            Rect::from_corners(Vec2::new(c.x, base.y), Vec2::new(c.x + dx, base.y + 6.0 + dy))
        };
        if layout.at(prop.anchor + IVec2::X) == 'f' {
            rects.push(rail(TILE / 2.0, 0.0));
        }
        if layout.at(prop.anchor + IVec2::NEG_X) == 'f' {
            rects.push(rail(-TILE / 2.0, 0.0));
        }
        if layout.at(prop.anchor + IVec2::NEG_Y) == 'f' {
            rects.push(Rect::new(c.x - 2.0, base.y, c.x + 2.0, base.y + TILE));
        }
        return rects;
    }
    if !door {
        return vec![body];
    }
    // Abertura de 14 px no meio para a porta.
    let gap = 7.0;
    vec![
        Rect::new(body.min.x, body.min.y, base.x - gap, body.max.y),
        Rect::new(base.x + gap, body.min.y, body.max.x, body.max.y),
        // O batente de cima da porta continua sólido (só se entra por baixo).
        Rect::new(base.x - gap, base.y + TILE, base.x + gap, body.max.y),
    ]
}

fn ground_of(ch: char) -> Option<Ground> {
    Some(match ch {
        's' => Ground::Soil,
        'p' | '<' | '>' => Ground::Path,
        'w' => Ground::Water,
        'c' => Ground::Cracked,
        'q' => Ground::Cobble,
        '_' => Ground::Floor,
        '=' => Ground::Rug,
        '#' => Ground::Wall,
        '%' => Ground::WallTop,
        'E' => Ground::Mat,
        'd' => Ground::Dock,
        _ => return None,
    })
}

/// Chão embaixo de um objeto: o calçamento/caminho/assoalho mais próximo nas 4 direções,
/// para um poste no meio da praça não ficar com um quadrado de grama embaixo.
fn under(layout: &Layout, t: IVec2, default: Ground) -> Ground {
    // Árvores, pedras e cercas ficam sempre na grama.
    if matches!(layout.at(t), 'T' | 'P' | 'r' | 'B' | 'f' | 'U') {
        return default;
    }
    // O barco boia: embaixo dele (e da metade direita reservada) é água.
    if layout.at(t) == 'k' || (layout.at(t) == 'h' && layout.at(t - IVec2::X) == 'k') {
        return Ground::Water;
    }
    for dist in 1..=2 {
        for d in [IVec2::NEG_X, IVec2::NEG_Y, IVec2::X, IVec2::Y] {
            if let Some(g @ (Ground::Cobble | Ground::Path | Ground::Floor)) = ground_of(layout.at(t + d * dist)) {
                return g;
            }
        }
    }
    default
}

/// Direção "para dentro" de uma borda marcada (para não nascer em cima da saída).
fn inward(edge: char) -> (IVec2, Facing) {
    match edge {
        '<' => (IVec2::X, Facing::Right),
        '>' => (IVec2::NEG_X, Facing::Left),
        '^' => (IVec2::Y, Facing::Down),
        _ => (IVec2::NEG_Y, Facing::Up),
    }
}

/// Para onde leva o tile, se for porta, capacho ou borda ligada no world.json.
fn warp_for(id: &str, ch: char, t: IVec2) -> Option<Warp> {
    let world = worlddef::world();
    // Porta de prédio ligada a um interior: entra e aparece logo acima do capacho.
    if let Some(door) = world.door_at(id, t) {
        let arrive = Layout::of(&door.to).find('E')? + IVec2::NEG_Y;
        return Some(Warp { to: door.to.clone(), arrive, facing: Facing::Up });
    }
    if ch == 'E' {
        let door = world.door_into(id)?;
        return Some(Warp { to: door.map.clone(), arrive: IVec2::from_array(door.at) + IVec2::Y, facing: Facing::Down });
    }
    if worlddef::EDGES.contains(&ch) {
        let link = world.portal(id, ch)?;
        let (step, facing) = inward(link.to_edge);
        let arrive = Layout::of(&link.to).find(link.to_edge)? + step;
        return Some(Warp { to: link.to.clone(), arrive, facing });
    }
    None
}

/// Problemas que impedem o mapa de funcionar no jogo (o editor mostra antes de salvar).
pub fn problems(info: &MapInfo, layout: &Layout) -> Vec<String> {
    let world = worlddef::world();
    let mut out = Vec::new();
    if world.start == info.id && layout.find('@').is_none() {
        out.push("o mapa inicial precisa de '@' (onde se acorda)".into());
    }
    if info.indoor {
        if layout.find('E').is_none() {
            out.push("falta o capacho de saída ('E')".into());
        }
        if world.door_into(&info.id).is_none() {
            out.push("nenhuma porta leva a este interior".into());
        }
    }
    if info.keeper.is_some() && layout.find('N').is_none() {
        out.push("falta o lugar de quem atende ('N')".into());
    }
    for edge in worlddef::EDGES {
        if layout.find(edge).is_some() && world.portal(&info.id, edge).is_none() {
            out.push(format!("a borda '{edge}' não leva a lugar nenhum"));
        }
    }
    for door in world.doors.iter().filter(|d| d.map == info.id) {
        if footprint_of(layout.at(IVec2::from_array(door.at)), IVec2::from_array(door.at)).is_none() {
            out.push(format!("porta ligada em {:?} sem prédio", door.at));
        }
    }
    out
}

/// Arte, deslocamento vertical do desenho (linhas vazias embaixo) e se é alto o bastante
/// para esconder o jogador.
fn prop_art(sprites: &Sprites, kind: Prop) -> (&Art, f32) {
    match kind {
        Prop::Oak => (&sprites.oak, 0.0),
        Prop::Pine => (&sprites.pine, -2.0),
        Prop::Rock => (&sprites.rock, 0.0),
        Prop::Bush => (&sprites.bush, 0.0),
        Prop::Fence => (&sprites.fence, 0.0),
        Prop::Sign => (&sprites.signpost, 0.0),
        Prop::House => (&sprites.house, 0.0),
        Prop::Stand => (&sprites.stand, 0.0),
        Prop::Building(b) => (sprites.building(b), 0.0),
        Prop::Fountain => (&sprites.fountain, 0.0),
        Prop::Lamppost => (&sprites.lamppost, 0.0),
        Prop::Bench => (&sprites.bench, 0.0),
        Prop::Barrel => (&sprites.barrel, 0.0),
        Prop::Crates => (&sprites.crates, 0.0),
        Prop::Planter => (&sprites.planter, 0.0),
        Prop::Bed => (&sprites.bed, 0.0),
        Prop::Forge => (&sprites.forge, 0.0),
        Prop::Table => (&sprites.table, 0.0),
        Prop::Oven => (&sprites.oven, 0.0),
        Prop::Counter => (&sprites.counter, 0.0),
        Prop::Shelf => (&sprites.shelf, 0.0),
        Prop::Plant => (&sprites.plant, 0.0),
        Prop::Barn => (&sprites.barn, 0.0),
        Prop::Coop => (&sprites.coop, 0.0),
        Prop::Windmill => (&sprites.windmill, 0.0),
        Prop::Cart => (&sprites.cart, 0.0),
        Prop::HayBale => (&sprites.haybale, 0.0),
        Prop::HayStack => (&sprites.haystack, 0.0),
        Prop::Rowboat => (&sprites.rowboat, 4.0),
        Prop::BigTree => (&sprites.tree_big, 0.0),
    }
}

/// Chaminés, relativas à base do sprite (meio da borda de baixo), em pixels do mundo.
fn chimney_of(kind: Prop) -> Option<Vec2> {
    match kind {
        // house.json: chaminé por volta de (60, 3) num desenho de 80x80.
        Prop::House => Some(Vec2::new(20.0, 77.0)),
        // Medidas dos JSONs da vila: (x - largura/2, altura - y).
        Prop::Building(Building::Bakery) => Some(Vec2::new(20.0, 71.0)),
        Prop::Building(Building::Smithy) => Some(Vec2::new(-27.0, 64.0)),
        Prop::Building(Building::Florist) => Some(Vec2::new(11.0, 55.0)),
        _ => None,
    }
}

fn spawn_prop(
    commands: &mut Commands,
    sprites: &Sprites,
    prop: &PlacedProp,
    layout: &Layout,
    scope: &(impl Bundle + Clone),
    chimneys: &mut Vec<Vec2>,
) -> Vec<Entity> {
    let t = prop.anchor;
    if prop.kind == Prop::Fence {
        // Mourão + travessas na direção de cada cerca vizinha: [post, L, O, N, S].
        let z = depth(tile_base(t).y);
        let c = tile_center(t);
        let mut parts = vec![(0, z)];
        for (i, d) in [(1, IVec2::X), (2, IVec2::NEG_X), (3, IVec2::NEG_Y), (4, IVec2::Y)] {
            if layout.at(t + d) == 'f' {
                parts.push((i, z - 0.0001));
            }
        }
        return parts
            .into_iter()
            .map(|(index, z)| {
                commands
                    .spawn((DualSprite::bundle(&sprites.fence, index), Transform::from_xyz(c.x, c.y, z), scope.clone()))
                    .id()
            })
            .collect();
    }

    // Base do desenho: meio da borda de baixo da pegada.
    let rect = prop.rect();
    let (art, dy) = prop_art(sprites, prop.kind);
    let base = Vec2::new(rect.center().x, rect.min.y + dy);
    let mut entity =
        commands.spawn((DualSprite::bundle(art, 0), Anchor::BOTTOM_CENTER, standing_at(base), scope.clone()));
    match prop.kind {
        Prop::Oak => _ = entity.insert((Occluder { half_width: 15.0, height: 46.0 }, Sway { base: 0, kind: SwayKind::Tree }, FloraKind::Oak)),
        Prop::Pine => _ = entity.insert((Occluder { half_width: 11.0, height: 38.0 }, Sway { base: 0, kind: SwayKind::Tree }, FloraKind::Pine)),
        Prop::Bush => _ = entity.insert(FloraKind::Bush),
        Prop::House | Prop::Building(_) => {
            let width = rect.width();
            let height = sprites_height(prop.kind);
            entity.insert(Occluder { half_width: width / 2.0, height });
        }
        Prop::Lamppost | Prop::Oven | Prop::Forge => _ = entity.insert(LightSource),
        Prop::BigTree => _ = entity.insert((Occluder { half_width: 22.0, height: 62.0 }, Sway { base: 0, kind: SwayKind::Tree }, FloraKind::BigTree)),
        Prop::Barn => _ = entity.insert(Occluder { half_width: 40.0, height: 78.0 }),
        Prop::Windmill => _ = entity.insert((Occluder { half_width: 20.0, height: 78.0 }, Spin)),
        Prop::Coop => _ = entity.insert(Occluder { half_width: 24.0, height: 46.0 }),
        _ => {}
    }
    chimneys.extend(chimney_of(prop.kind).map(|c| base + c));
    let mut entities = vec![entity.id()];
    // Sombra no chão: "assenta" árvores e objetos soltos, como já acontece com gente e bicho.
    if let Some((big, dy)) = ground_shadow(prop.kind) {
        let art = if big { &sprites.shadow_big } else { &sprites.shadow };
        entities.push(
            commands.spawn((DualSprite::bundle(art, 0), Transform::from_xyz(base.x, base.y + dy, SHADOW_Z), scope.clone())).id(),
        );
    }
    entities
}

/// Altura dos desenhos dos prédios (art/CONTRACT.md).
fn sprites_height(kind: Prop) -> f32 {
    match kind {
        Prop::House => 80.0,
        Prop::Building(Building::Bakery | Building::Store) => 72.0,
        Prop::Building(Building::Smithy) => 64.0,
        Prop::Building(Building::TownHall) => 96.0,
        Prop::Building(Building::Library) => 80.0,
        Prop::Building(Building::Florist | Building::GuardPost) => 56.0,
        _ => 32.0,
    }
}

/// Fundo animado do menu: a fazenda sem save.
fn spawn_scenery(
    mut commands: Commands,
    sprites: Res<Sprites>,
    mut images: ResMut<Assets<Image>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
) {
    let world = worlddef::world();
    let id = world.farm_id().unwrap_or(world.start.clone());
    let info = world.info(&id);
    let mut painter = Painter { images: &mut images, layouts: &layouts };
    build(&mut commands, &sprites, &mut painter, &info, &Layout::of(&id), DespawnOnExit(AppState::Menu), &MapEdits::default(), Season::Spring);
}

/// Camada animada da água: só o retângulo dos lagos, com N quadros por fase.
#[derive(Component)]
struct WaterLayer {
    day: Vec<Handle<Image>>,
    night: Vec<Handle<Image>>,
}

fn spawn_water(commands: &mut Commands, sprites: &Sprites, painter: &mut Painter, map: &Map, scope: &(impl Bundle + Clone)) {
    let water: Vec<IVec2> = map.tiles().filter(|t| map.ground(*t) == Some(Ground::Water)).collect();
    let (Some(min), Some(max)) = (
        water.iter().copied().reduce(|a, b| a.min(b)),
        water.iter().copied().reduce(|a, b| a.max(b)),
    ) else {
        return;
    };
    // Um tile de folga: a borda orgânica pode passar um pouco do tile de água.
    let (min, max) = (min - IVec2::ONE, max + IVec2::ONE);
    let size = ((max - min + IVec2::ONE).as_vec2() * TILE).as_uvec2();
    let ground = |t: IVec2| map.ground(t);
    let mut frames = [Vec::new(), Vec::new()];
    for (night, out) in [false, true].into_iter().zip(frames.iter_mut()) {
        let images: Vec<Vec<Rgba>> = {
            let mut cache = TexCache::new(painter.images, painter.layouts, night);
            (0..terrain::WATER_FRAMES)
                .map(|frame| {
                    (0..size.x * size.y)
                        .map(|i| {
                            let u = min.x as f32 + ((i % size.x) as f32 + 0.5) / TILE;
                            let v = min.y as f32 + ((i / size.x) as f32 + 0.5) / TILE;
                            terrain::water_pixel(&mut cache, &ground, &sprites.water, u, v, frame)
                        })
                        .collect()
                })
                .collect()
        };
        *out = images.iter().map(|px| painter.image(size, px)).collect();
    }
    let [day, night] = frames;
    let corner = Vec2::new(min.x as f32 * TILE, -(min.y as f32) * TILE);
    commands.spawn((
        Sprite::from_image(day[0].clone()),
        WaterLayer { day, night },
        Anchor::TOP_LEFT,
        Transform::from_translation(corner.extend(0.02)),
        scope.clone(),
    ));
}

fn animate_water(
    time: Res<Time<Real>>,
    phase: Option<Res<State<crate::cycle::DayPhase>>>,
    mut layers: Query<(&WaterLayer, &mut Sprite)>,
) {
    let frame = (time.elapsed_secs() * 4.0) as usize % terrain::WATER_FRAMES;
    let night = phase.is_some_and(|p| *p.get() == crate::cycle::DayPhase::Night);
    for (layer, mut sprite) in &mut layers {
        let frames = if night { &layer.night } else { &layer.day };
        if let Some(image) = frames.get(frame)
            && sprite.image != *image
        {
            sprite.image = image.clone();
        }
    }
}

fn spin(time: Res<Time<Real>>, mut mills: Query<&mut Sprite, With<Spin>>) {
    let frame = (time.elapsed_secs() * 5.0) as usize % 4;
    for mut sprite in &mut mills {
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = frame;
        }
    }
}
