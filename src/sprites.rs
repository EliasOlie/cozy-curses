//! Handles dos PNGs gerados pelo pixelwind (art/sprites/*.json -> assets/sprites/*.png).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::AppState;
use crate::cycle::DayPhase;

pub fn plugin(app: &mut App) {
    app.init_resource::<Sprites>()
        .add_systems(OnExit(AppState::Loading), pack_atlas)
        .add_systems(PostUpdate, sync_dual_sprites);
}

/// Um asset do pixelwind: par dia/noite (iguais em assets de uma fase só) e,
/// se for spritesheet, o layout da grade.
#[derive(Clone)]
pub struct Art {
    pub day: Handle<Image>,
    pub night: Handle<Image>,
    pub layout: Option<Handle<TextureAtlasLayout>>,
}

impl Art {
    pub fn get(&self, phase: DayPhase) -> &Handle<Image> {
        match phase {
            DayPhase::Day => &self.day,
            DayPhase::Night => &self.night,
        }
    }

    /// Sprite da fase; `index` é o quadro da spritesheet (ignorado se não for sheet).
    pub fn sprite(&self, phase: DayPhase, index: usize) -> Sprite {
        let image = self.get(phase).clone();
        match &self.layout {
            Some(layout) => {
                Sprite::from_atlas_image(image, TextureAtlas { layout: layout.clone(), index })
            }
            None => Sprite::from_image(image),
        }
    }
}

/// Entidade cuja imagem acompanha o ciclo dia/noite.
#[derive(Component, Clone)]
pub struct DualSprite(pub Art);

impl DualSprite {
    /// O componente junto com o sprite inicial (sempre de dia: o jogo e os saves começam de manhã).
    pub fn bundle(art: &Art, index: usize) -> (Sprite, DualSprite) {
        (art.sprite(DayPhase::Day, index), DualSprite(art.clone()))
    }
}

#[derive(Resource)]
pub struct Sprites {
    pub grass: Art,
    pub path: Art,
    pub soil: Art,
    pub water: Art,
    pub cracked: Art,
    pub cavern: Art,
    pub decor: Art,
    pub rock: Art,
    pub bush: Art,
    pub fence: Art,
    pub oak: Art,
    pub pine: Art,
    pub house: Art,
    pub stand: Art,
    pub signpost: Art,
    pub factory: Art,
    /// Peças do fazendeiro, de baixo para cima: corpo, calça, camisa.
    pub farmer_body: Art,
    pub farmer_pants: Art,
    pub farmer_shirt: Art,
    pub farmer_hair: Vec<Art>,
    pub baker: Art,
    pub guard: Art,
    pub mayor: Art,
    pub teacher: Art,
    pub merchant: Art,
    pub smith: Art,
    pub florist: Art,
    pub inquisitor: Art,
    pub lantern: Handle<Image>,
    pub mask_worn: Handle<Image>,
    // Vila
    pub cobble: Art,
    pub bakery: Art,
    pub store: Art,
    pub smithy: Art,
    pub townhall: Art,
    pub library: Art,
    pub florist_shop: Art,
    pub guardpost: Art,
    pub fountain: Art,
    pub lamppost: Art,
    pub bench: Art,
    pub barrel: Art,
    pub crates: Art,
    pub planter: Art,
    // Interiores
    pub floor: Art,
    pub wall: Art,
    pub rug: Art,
    pub doormat: Art,
    pub bed: Art,
    pub forge: Art,
    pub table: Art,
    pub oven: Art,
    pub counter: Art,
    pub shelf: Art,
    pub plant: Art,
    // Efeitos
    pub fx: Art,
    pub cursor: Art,
    pub cloud_shadow: Handle<Image>,
    // Interface
    pub items: Art,
    // Fazenda (inspiração)
    pub barn: Art,
    pub coop: Art,
    pub windmill: Art,
    pub cart: Art,
    pub haybale: Art,
    pub haystack: Art,
    pub dock: Art,
    pub rowboat: Art,
    pub reeds: Art,
    pub tree_big: Art,
    pub pumpkin: Art,
    pub flowerbed: Art,
    pub chicken: Art,
    pub sheep: Art,
    pub dog: Art,
    pub cow: Art,
    pub goat: Art,
    pub rabbit: Art,
    pub horse: Art,
    /// Crescimento dos cultivos: linha = `CropKind`, coluna = estágio (16x32, base embaixo).
    pub crops: Art,
    /// Retratos da caixa de diálogo, um por `villagers::Kind`.
    pub portraits: Art,
    /// Pragas da noite (só existem à noite: paleta única).
    pub slime: Art,
    pub crow: Art,
    pub haunt: Art,
    pub phone: Handle<Image>,
    pub ui: UiKit,
    /// Sombra suave sob personagens e objetos.
    pub shadow: Art,
    /// Sombra larga de chão, sob a copa das árvores.
    pub shadow_big: Art,
    /// Balões em cima dos moradores: "!", "...", "?".
    pub bubble: Art,
    /// Setinha que quica em cima do alvo da interação.
    pub arrow: Art,
    pub font: Handle<Font>,
    /// Tudo que foi pedido ao carregador (a tela de carregamento espera por isso).
    pub pending: Vec<UntypedHandle>,
    /// Atlas de dia e de noite, onde moram todas as artes do mundo (ver `pack_atlas`).
    atlas: (Handle<Image>, Handle<Image>),
    sources: Vec<AtlasSource>,
}

/// Molduras da interface (assets/ui, já ampliadas 3x, fatiadas em 9).
#[derive(Clone)]
pub struct UiKit {
    pub panel: Handle<Image>,
    pub panel_dark: Handle<Image>,
    pub button: Handle<Image>,
    pub button_hover: Handle<Image>,
    pub button_pressed: Handle<Image>,
    pub slot: Handle<Image>,
    pub slot_selected: Handle<Image>,
    pub dialog: Handle<Image>,
    pub tag: Handle<Image>,
}

impl Sprites {
    pub fn building(&self, b: crate::map::Building) -> &Art {
        use crate::map::Building::*;
        match b {
            Bakery => &self.bakery,
            Store => &self.store,
            Smithy => &self.smithy,
            TownHall => &self.townhall,
            Library => &self.library,
            Florist => &self.florist_shop,
            GuardPost => &self.guardpost,
        }
    }
}

/// Quadros da spritesheet `fx` (art/CONTRACT.md).
pub mod fx {
    pub const BUTTERFLY: usize = 0;
    pub const LEAF: usize = 2;
    pub const FIREFLY: usize = 4;
    pub const SMOKE: usize = 6;
    pub const DUST: usize = 9;
    pub const SPARKLE: usize = 11;
    pub const TALK: usize = 13;
    pub const HEART: usize = 14;
    pub const COIN: usize = 15;
}

/// Quadros de cada spritesheet (contrato com os JSONs de art/sprites).
pub const GRASS_VARIANTS: usize = 4;
pub const PATH_VARIANTS: usize = 2;
pub const WATER_FRAMES: usize = 2;

impl FromWorld for Sprites {
    fn from_world(world: &mut World) -> Self {
        // Grade de cada spritesheet: (largura, altura, colunas, linhas) do quadro.
        let grid = |w: u32, h: u32, cols: u32, rows: u32| Some((w, h, cols, rows));
        let tile = |cols: u32| (16, 16, cols, 1);
        let sheets = [
            ("grass", tile(GRASS_VARIANTS as u32)),
            ("path", tile(PATH_VARIANTS as u32)),
            ("water", tile(WATER_FRAMES as u32)),
            ("decor", (16, 16, 6, 2)),
            ("fence", tile(5)),
            ("farmer", (16, 32, 3, 3)),
            ("villager", (16, 32, 3, 3)),
            ("crops", (16, 32, 3, 15)),
            ("portraits", (32, 32, 7, 1)),
            ("haunt", (16, 32, 4, 1)),
            ("cow", (32, 24, 4, 2)),
            ("goat", (24, 24, 4, 2)),
            ("rabbit", (16, 16, 4, 2)),
            ("horse", (32, 32, 4, 2)),
            ("chicken", (16, 16, 4, 2)),
            ("oak", (32, 48, 3, 1)),
            ("pine", (24, 40, 3, 1)),
            ("two_tiles", tile(2)),
            ("fx", (8, 8, 8, 2)),
            ("items", (16, 16, 8, 10)),
            ("bubble", (12, 12, 3, 1)),
            ("windmill", (48, 80, 4, 1)),
            ("reeds", (16, 16, 2, 1)),
            ("tree_big", (48, 64, 3, 1)),
            ("small_animal", (16, 16, 4, 1)),
            ("sheep", (24, 24, 4, 2)),
        ]
        .map(|(name, (w, h, c, r))| (name, grid(w, h, c, r)));
        let layout = |name: &str| sheets.iter().find(|(n, _)| *n == name).unwrap().1;

        let server = world.resource::<AssetServer>();
        let images = world.resource::<Assets<Image>>();
        let layouts = world.resource::<Assets<TextureAtlasLayout>>();
        // Toda carga passa por aqui: a tela de carregamento espera por todas.
        let pending = std::cell::RefCell::new(Vec::new());
        let load = |path: String| -> Handle<Image> {
            let handle: Handle<Image> = server.load(path);
            pending.borrow_mut().push(handle.clone().untyped());
            handle
        };
        // As artes do mundo apontam desde já para o atlas (um de dia, um de noite) e para um
        // layout próprio; os dois são preenchidos por `pack_atlas` quando os PNGs chegam.
        let atlas = (images.reserve_handle(), images.reserve_handle());
        let sources = std::cell::RefCell::new(Vec::new());
        let packed = |day: Handle<Image>, night: Handle<Image>, grid: Option<(u32, u32, u32, u32)>| {
            let layout = layouts.reserve_handle();
            sources.borrow_mut().push(AtlasSource { day, night, grid, layout: layout.clone() });
            Art { day: atlas.0.clone(), night: atlas.1.clone(), layout: Some(layout) }
        };
        let both = |file: &str, grid: Option<(u32, u32, u32, u32)>| {
            packed(load(format!("sprites/{file}_day.png")), load(format!("sprites/{file}_night.png")), grid)
        };
        let single = |file: &str, grid: Option<(u32, u32, u32, u32)>| {
            let image = load(format!("sprites/{file}.png"));
            packed(image.clone(), image, grid)
        };
        let ui = |file: &str| load(format!("ui/{file}.png"));
        let farmer = layout("farmer");

        Sprites {
            grass: both("tile_grass", layout("grass")),
            path: both("tile_path", layout("path")),
            soil: both("tile_soil", None),
            water: both("tile_water", layout("water")),
            cracked: both("tile_cracked", None),
            cavern: both("tile_cavern", None),
            decor: both("decor", layout("decor")),
            rock: both("rock", None),
            bush: both("bush", None),
            fence: both("fence", layout("fence")),
            oak: both("tree_oak", layout("oak")),
            pine: both("tree_pine", layout("pine")),
            house: both("house", None),
            stand: both("stand", None),
            signpost: both("signpost", None),
            factory: both("factory", None),
            farmer_body: single("farmer_body", farmer.clone()),
            farmer_pants: single("farmer_pants", farmer.clone()),
            farmer_shirt: single("farmer_shirt", farmer.clone()),
            farmer_hair: (0..3).map(|i| single(&format!("farmer_hair_{i}"), farmer.clone())).collect(),
            baker: both("baker", layout("villager")),
            guard: both("guard", layout("villager")),
            mayor: both("mayor", layout("villager")),
            teacher: both("teacher", layout("villager")),
            merchant: both("merchant", layout("villager")),
            smith: both("smith", layout("villager")),
            florist: both("florist", layout("villager")),
            inquisitor: single("inquisitor", None),
            lantern: load("sprites/lantern.png".into()),
            mask_worn: load("sprites/mask_gargoyle_worn.png".into()),
            cobble: both("tile_cobble", layout("two_tiles")),
            bakery: both("bakery", None),
            store: both("store", None),
            smithy: both("smithy", None),
            townhall: both("townhall", None),
            library: both("library", None),
            florist_shop: both("flower_shop", None),
            guardpost: both("guardpost", None),
            fountain: both("fountain", None),
            lamppost: both("lamppost", None),
            bench: both("bench", None),
            barrel: both("barrel", None),
            crates: both("crates", None),
            planter: both("planter", None),
            floor: both("floor_wood", layout("two_tiles")),
            wall: both("wall", layout("two_tiles")),
            rug: both("rug", None),
            doormat: both("doormat", None),
            bed: both("bed", None),
            forge: both("forge", None),
            table: both("table", None),
            oven: both("oven", None),
            counter: both("counter", None),
            shelf: both("shelf", None),
            plant: both("plant", None),
            fx: both("fx", layout("fx")),
            cursor: both("cursor", None),
            cloud_shadow: load("sprites/cloud_shadow.png".into()),
            items: single("items", layout("items")),
            barn: both("barn", None),
            coop: both("coop", None),
            windmill: both("windmill", layout("windmill")),
            cart: both("cart", None),
            haybale: both("haybale", None),
            haystack: both("haystack", None),
            dock: both("dock", None),
            rowboat: both("rowboat", None),
            reeds: both("reeds", layout("reeds")),
            tree_big: both("tree_big", layout("tree_big")),
            pumpkin: both("pumpkin", None),
            flowerbed: both("flowerbed", None),
            chicken: both("chicken", layout("chicken")),
            sheep: both("sheep", layout("sheep")),
            cow: both("cow", layout("cow")),
            goat: both("goat", layout("goat")),
            rabbit: both("rabbit", layout("rabbit")),
            horse: both("horse", layout("horse")),
            crops: both("crops", layout("crops")),
            portraits: both("portraits", layout("portraits")),
            slime: single("slime", layout("small_animal")),
            crow: single("crow", layout("small_animal")),
            haunt: single("haunt", layout("haunt")),
            dog: both("dog", layout("small_animal")),
            phone: load("sprites/phone.png".into()),
            ui: UiKit {
                panel: ui("panel"),
                panel_dark: ui("panel_dark"),
                button: ui("button"),
                button_hover: ui("button_hover"),
                button_pressed: ui("button_pressed"),
                slot: ui("slot"),
                slot_selected: ui("slot_selected"),
                dialog: ui("dialog"),
                tag: ui("tag"),
            },
            shadow: both("shadow", None),
            shadow_big: both("shadow_big", None),
            bubble: both("bubble", layout("bubble")),
            arrow: both("arrow", None),
            font: {
                let font: Handle<Font> = server.load("fonts/VT323-Regular.ttf");
                pending.borrow_mut().push(font.clone().untyped());
                font
            },
            pending: pending.take(),
            atlas,
            sources: sources.take(),
        }
    }
}

/// Um PNG (par dia/noite) que vai para dentro do atlas, e o layout que o descreve lá.
pub struct AtlasSource {
    day: Handle<Image>,
    night: Handle<Image>,
    grid: Option<(u32, u32, u32, u32)>,
    layout: Handle<TextureAtlasLayout>,
}

const ATLAS_WIDTH: u32 = 1024;

/// Junta todos os PNGs do mundo numa textura só por fase. Com uma textura por sprite a
/// placa de vídeo recebia mais de mil desenhos separados por quadro (cada troca de
/// textura quebra o lote); com o atlas, o mapa inteiro sai em meia dúzia.
pub fn pack_atlas(
    sprites: Res<Sprites>,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Tamanho e pixels de cada arte (dia, noite). O que não carregou vira 1x1 transparente.
    let read = |images: &Assets<Image>, handle: &Handle<Image>| -> (UVec2, Vec<u8>) {
        images
            .get(handle)
            .and_then(|image| image.data.as_ref().map(|data| (image.size(), data.clone())))
            .filter(|(size, data)| data.len() == (size.x * size.y * 4) as usize)
            .unwrap_or((UVec2::ONE, vec![0; 4]))
    };
    let arts: Vec<(UVec2, Vec<u8>, Vec<u8>)> = sprites
        .sources
        .iter()
        .map(|source| {
            let (size, day) = read(&images, &source.day);
            let (night_size, night) = read(&images, &source.night);
            // A versão noturna tem o mesmo desenho; se faltar, repete a de dia.
            (size, day.clone(), if night_size == size { night } else { day })
        })
        .collect();

    // Prateleiras: os mais altos primeiro, 1 px de folga em volta de cada arte.
    let mut order: Vec<usize> = (0..arts.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(arts[i].0.y));
    let mut corners = vec![UVec2::ZERO; arts.len()];
    let (mut x, mut y, mut shelf) = (1, 1, 0);
    for &i in &order {
        let size = arts[i].0;
        if x + size.x + 1 > ATLAS_WIDTH {
            x = 1;
            y += shelf + 1;
            shelf = 0;
        }
        corners[i] = UVec2::new(x, y);
        x += size.x + 1;
        shelf = shelf.max(size.y);
    }
    let atlas_size = UVec2::new(ATLAS_WIDTH, (y + shelf + 1).next_multiple_of(4));

    let mut pixels = [vec![0u8; (atlas_size.x * atlas_size.y * 4) as usize], Vec::new()];
    pixels[1] = pixels[0].clone();
    for (i, (size, day, night)) in arts.iter().enumerate() {
        for (data, out) in [day, night].into_iter().zip(pixels.iter_mut()) {
            for row in 0..size.y {
                let from = (row * size.x * 4) as usize;
                let to = (((corners[i].y + row) * atlas_size.x + corners[i].x) * 4) as usize;
                let len = (size.x * 4) as usize;
                out[to..to + len].copy_from_slice(&data[from..from + len]);
            }
        }
    }
    let [day, night] = pixels;
    for (handle, data) in [(&sprites.atlas.0, day), (&sprites.atlas.1, night)] {
        let image = Image::new(
            Extent3d { width: atlas_size.x, height: atlas_size.y, depth_or_array_layers: 1 },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        let _ = images.insert(handle.id(), image);
    }

    // Layout de cada arte: os quadros da grade (na mesma ordem de antes) dentro do atlas.
    for (source, (corner, (size, ..))) in sprites.sources.iter().zip(corners.iter().zip(&arts)) {
        let mut layout = TextureAtlasLayout::new_empty(atlas_size);
        let (w, h, cols, rows) = source.grid.unwrap_or((size.x, size.y, 1, 1));
        for row in 0..rows {
            for col in 0..cols {
                let min = *corner + UVec2::new(col * w, row * h);
                layout.add_texture(URect::from_corners(min, min + UVec2::new(w, h)));
            }
        }
        let _ = layouts.insert(source.layout.id(), layout);
    }
    info!("atlas de sprites: {} artes em {}x{}", arts.len(), atlas_size.x, atlas_size.y);
}

/// Fora do jogo (menu) não existe fase: o cenário de fundo fica de dia.
fn sync_dual_sprites(
    phase: Option<Res<State<DayPhase>>>,
    mut query: Query<(&DualSprite, &mut Sprite)>,
) {
    let phase = phase.map_or(DayPhase::Day, |p| *p.get());
    for (dual, mut sprite) in &mut query {
        let wanted = dual.0.get(phase);
        if sprite.image != *wanted {
            sprite.image = wanted.clone();
        }
    }
}
