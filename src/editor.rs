//! Editor de mapas dentro do jogo (menu principal -> Editor de mapas).
//!
//! Pinta os caracteres do .txt de cada mapa e mostra o resultado na hora (o mesmo
//! `map::build` do jogo). Cria áreas novas (exteriores e interiores), define o papel de
//! cada mapa e liga portas de prédios a interiores e bordas a outros mapas.
//! Salvar grava no navegador (localStorage) ou, no nativo, em assets/maps/ — o
//! build.rs embute essa pasta, então mapas novos entram no próximo build.

use std::collections::{HashMap, HashSet};

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::AppState;
use crate::bake::Painter;
use crate::cycle::Season;
use crate::map::{Layout, MapEdits, Prop, TILE, build, footprint_of, problems, tile_center, world_to_tile};
use crate::menu::{GameClicked, UiAction};
use crate::save;
use crate::shop::Shop;
use crate::sprites::Sprites;
use crate::ui::{HINT, MUTED, PANEL_BORDER, PANEL_SOLID, TEXT, font, spawn_button};
use crate::villagers::Kind;
use crate::worlddef::{self, DoorLink, EDGES, MapInfo, PortalLink, WorldDef};

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Editor), open)
        .add_systems(OnExit(AppState::Editor), |mut c: Commands| c.remove_resource::<Editor>())
        .add_systems(
            Update,
            (actions, type_name, paint, rebuild, hover, pan_zoom, redraw_ui).chain().run_if(in_state(AppState::Editor)),
        );
}

#[derive(Clone, Debug, PartialEq)]
pub enum EditorAction {
    SelectMap(String),
    NewMap { indoor: bool },
    DeleteMap,
    Brush(char),
    Resize(i32, i32),
    Rename,
    ToggleIndoor,
    ToggleFarm,
    SetStart,
    CycleKeeper,
    CycleShop,
    /// Porta (tile do prédio) -> próximo interior da lista.
    CycleDoor([i32; 2]),
    CyclePortalMap(char),
    CyclePortalEdge(char),
    Save,
    Copy,
    Reset,
    Back,
}

/// (caractere, nome) dos pincéis, por grupo. A legenda completa está em `map::build`.
const BRUSHES: &[(&str, &[(char, &str)])] = &[
    (
        "Chão",
        &[
            ('.', "Grama"),
            ('p', "Caminho"),
            ('s', "Canteiro"),
            ('w', "Água"),
            ('c', "Rachado"),
            ('q', "Pedra"),
            ('_', "Assoalho"),
            ('=', "Tapete"),
            ('#', "Parede"),
            ('%', "Borda"),
            ('E', "Capacho"),
        ],
    ),
    (
        "Natureza",
        &[
            ('T', "Carvalho"),
            ('P', "Pinheiro"),
            ('B', "Arbusto"),
            ('r', "Rocha"),
            ('f', "Cerca"),
            (',', "Tufo"),
            (';', "Tufo alto"),
            ('*', "Flor rosa"),
            ('+', "Flor amarela"),
            ('o', "Pedrinhas"),
            ('m', "Cogumelos"),
        ],
    ),
    (
        "Construções",
        &[
            ('H', "Casa"),
            ('S', "Barraca"),
            ('g', "Placa"),
            ('1', "Padaria"),
            ('2', "Armazém"),
            ('3', "Ferraria"),
            ('4', "Prefeitura"),
            ('5', "Biblioteca"),
            ('6', "Floricultura"),
            ('7', "Guarita"),
            ('W', "Chafariz"),
            ('L', "Poste"),
            ('n', "Banco"),
            ('u', "Barril"),
            ('x', "Caixotes"),
            ('y', "Floreira"),
        ],
    ),
    (
        "Fazenda",
        &[
            ('A', "Celeiro"),
            ('C', "Galinheiro"),
            ('M', "Moinho"),
            ('J', "Carroça"),
            ('b', "Fardo"),
            ('G', "Feno"),
            ('d', "Píer"),
            ('k', "Barco"),
            ('i', "Taboa"),
            ('U', "Árvore grande"),
            ('z', "Abóbora"),
            ('l', "Flores"),
            ('a', "Galinha"),
            ('e', "Ovelha"),
            ('j', "Cachorro"),
        ],
    ),
    (
        "Interior",
        &[('Z', "Cama"), ('F', "Forja"), ('Y', "Mesa"), ('O', "Forno"), ('K', "Balcão"), ('X', "Prateleira"), ('V', "Vaso")],
    ),
    ("Marcadores", &[('@', "Acorda"), ('N', "Balconista"), ('<', "Borda oeste"), ('>', "Borda leste"), ('^', "Borda norte"), ('v', "Borda sul")]),
];

/// Faixas de interface (o mouse não pinta por baixo delas).
const LEFT_PANEL: f32 = 290.0;
const RIGHT_PANEL: f32 = 300.0;
const TOP_BAR: f32 = 50.0;
const BOTTOM_BAR: f32 = 32.0;

#[derive(Resource)]
struct Editor {
    world: WorldDef,
    /// Cópias de trabalho dos mapas abertos nesta sessão.
    layouts: HashMap<String, Layout>,
    /// Mapas (e o mundo, como "world") com mudanças não salvas.
    unsaved: HashSet<String>,
    map: String,
    brush: char,
    dirty: bool,
    since_build: f32,
    zoom: f32,
    renaming: bool,
    message: String,
    hover: Option<IVec2>,
}

impl Editor {
    fn info(&self) -> MapInfo {
        self.world.info(&self.map)
    }

    fn info_mut(&mut self) -> &mut MapInfo {
        let id = self.map.clone();
        if !self.world.maps.iter().any(|m| m.id == id) {
            self.world.maps.push(MapInfo::new(&id, &id, false));
        }
        self.unsaved.insert("world".into());
        self.world.maps.iter_mut().find(|m| m.id == id).unwrap()
    }

    fn layout(&mut self) -> &mut Layout {
        let id = self.map.clone();
        self.layouts.entry(id.clone()).or_insert_with(|| Layout::of(&id))
    }

    fn layout_ref(&self) -> Layout {
        self.layouts.get(&self.map).cloned().unwrap_or_else(|| Layout::of(&self.map))
    }

    fn touched(&mut self) {
        self.unsaved.insert(self.map.clone());
        self.dirty = true;
    }
}

#[derive(Component, Clone)]
struct EditorView;

#[derive(Component)]
struct EditorUi;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct HoverBox;

fn open(mut commands: Commands, camera: Single<&mut Transform, With<Camera2d>>) {
    let world = worlddef::world();
    let map = world.farm_id().unwrap_or(world.start.clone());
    let layout = Layout::of(&map);
    let mut camera = camera.into_inner();
    camera.translation = tile_center(layout.size / 2).extend(camera.translation.z);
    commands.insert_resource(Editor {
        world,
        layouts: HashMap::from([(map.clone(), layout)]),
        unsaved: HashSet::new(),
        map,
        brush: '.',
        dirty: true,
        since_build: 1.0,
        zoom: 2.0,
        renaming: false,
        message: "Esquerdo pinta · direito apaga · WASD move · rodinha dá zoom".into(),
        hover: None,
    });
    commands.spawn((
        HoverBox,
        Sprite::from_color(Color::srgba(1.0, 0.95, 0.5, 0.35), Vec2::splat(TILE)),
        Transform::from_xyz(0.0, 0.0, 40.0),
        Visibility::Hidden,
        DespawnOnExit(AppState::Editor),
    ));
}

/// Próximo da lista, dando a volta (None conta como uma opção).
fn cycle<T: Clone + PartialEq>(options: &[T], current: &Option<T>) -> Option<T> {
    let i = current.as_ref().and_then(|c| options.iter().position(|o| o == c));
    match i {
        None => options.first().cloned(),
        Some(i) if i + 1 < options.len() => Some(options[i + 1].clone()),
        Some(_) => None,
    }
}

fn new_id(world: &WorldDef, indoor: bool) -> String {
    let prefix = if indoor { "interior" } else { "area" };
    (1..).map(|n| format!("{prefix}{n}")).find(|id| !world.maps.iter().any(|m| &m.id == id)).unwrap()
}

/// Mapa inicial para uma área nova: grama, ou um cômodo com paredes e capacho.
fn starter_layout(indoor: bool) -> Layout {
    if !indoor {
        return Layout::blank(IVec2::new(32, 24), '.');
    }
    let mut layout = Layout::blank(IVec2::new(12, 9), '_');
    for x in 0..12 {
        layout.set(IVec2::new(x, 0), '%');
        layout.set(IVec2::new(x, 1), '#');
        layout.set(IVec2::new(x, 2), '#');
        layout.set(IVec2::new(x, 8), '%');
    }
    for y in 0..9 {
        layout.set(IVec2::new(0, y), '%');
        layout.set(IVec2::new(11, y), '%');
    }
    layout.set(IVec2::new(5, 8), 'E');
    layout
}

#[allow(clippy::too_many_arguments)]
fn actions(
    mut clicked: ResMut<GameClicked>,
    mut editor: ResMut<Editor>,
    mut next: ResMut<NextState<AppState>>,
    camera: Single<&mut Transform, With<Camera2d>>,
) {
    let mut camera = camera.into_inner();
    for action in std::mem::take(&mut clicked.0) {
        let UiAction::Editor(action) = action else { continue };
        let e = &mut *editor;
        match action {
            EditorAction::SelectMap(id) => {
                e.map = id.clone();
                let size = e.layout().size;
                e.dirty = true;
                e.renaming = false;
                e.message = format!("Editando: {}", e.info().name);
                camera.translation = tile_center(size / 2).extend(camera.translation.z);
            }
            EditorAction::NewMap { indoor } => {
                let id = new_id(&e.world, indoor);
                // Começa sem nome: o que for digitado já é o nome.
                e.world.maps.push(MapInfo::new(&id, "", indoor));
                e.layouts.insert(id.clone(), starter_layout(indoor));
                e.unsaved.insert("world".into());
                e.map = id.clone();
                e.touched();
                e.renaming = true;
                e.message = format!("Área criada ({id}). Digite o nome e Enter.");
                camera.translation = tile_center(e.layout().size / 2).extend(camera.translation.z);
            }
            EditorAction::DeleteMap => {
                if worlddef::original_text(&e.map).is_some() || e.world.start == e.map {
                    e.message = "Só dá para excluir áreas criadas no editor (e que não sejam a inicial).".into();
                    continue;
                }
                let id = e.map.clone();
                e.world.maps.retain(|m| m.id != id);
                e.world.doors.retain(|d| d.map != id && d.to != id);
                e.world.portals.retain(|p| p.map != id && p.to != id);
                e.layouts.remove(&id);
                e.unsaved.insert("world".into());
                e.map = e.world.start.clone();
                e.dirty = true;
                e.message = format!("{id} excluída (salve para confirmar).");
            }
            EditorAction::Brush(ch) => e.brush = ch,
            EditorAction::Resize(dx, dy) => {
                let blank = e.info().blank();
                let layout = e.layout();
                let size = layout.size + IVec2::new(dx, dy);
                layout.resize(size, blank);
                let size = layout.size;
                e.touched();
                e.message = format!("Tamanho: {} x {}", size.x, size.y);
            }
            EditorAction::Rename => e.renaming = !e.renaming,
            EditorAction::ToggleIndoor => {
                let info = e.info_mut();
                info.indoor = !info.indoor;
                e.dirty = true;
            }
            EditorAction::ToggleFarm => {
                let id = e.map.clone();
                let on = !e.info().farm;
                // Só uma fazenda: a plantação, as ferramentas e as fornalhas moram nela.
                for m in &mut e.world.maps {
                    m.farm = on && m.id == id;
                }
                e.unsaved.insert("world".into());
            }
            EditorAction::SetStart => {
                e.world.start = e.map.clone();
                e.unsaved.insert("world".into());
                e.message = "Este é o mapa onde se acorda.".into();
            }
            EditorAction::CycleKeeper => {
                let current = e.info().keeper;
                e.info_mut().keeper = cycle(&Kind::ALL, &current);
                e.dirty = true;
            }
            EditorAction::CycleShop => {
                let current = e.info().shop;
                e.info_mut().shop = cycle(&[Shop::Store, Shop::Bakery, Shop::Florist, Shop::Kitchen, Shop::Stand], &current);
            }
            EditorAction::CycleDoor(at) => {
                let map = e.map.clone();
                // Só interiores livres (ou o atual): um interior tem uma porta só, e trocar
                // não pode desligar a porta de outro prédio.
                let taken = |id: &str| e.world.doors.iter().any(|d| d.to == id && !(d.map == map && d.at == at));
                let interiors: Vec<String> =
                    e.world.maps.iter().filter(|m| m.indoor && !taken(&m.id)).map(|m| m.id.clone()).collect();
                let current = e.world.door_at(&map, IVec2::from_array(at)).map(|d| d.to.clone());
                e.world.doors.retain(|d| !(d.map == map && d.at == at));
                if let Some(to) = cycle(&interiors, &current) {
                    e.world.doors.push(DoorLink { map, at, to });
                }
                e.unsaved.insert("world".into());
                e.dirty = true;
            }
            EditorAction::CyclePortalMap(edge) => {
                let others: Vec<String> = e.world.maps.iter().filter(|m| m.id != e.map).map(|m| m.id.clone()).collect();
                let map = e.map.clone();
                let current = e.world.portal(&map, edge).map(|p| p.to.clone());
                let old_edge = e.world.portal(&map, edge).map(|p| p.to_edge);
                e.world.portals.retain(|p| !(p.map == map && p.edge == edge));
                if let Some(to) = cycle(&others, &current) {
                    let to_edge = old_edge.unwrap_or(opposite(edge));
                    e.world.portals.push(PortalLink { map, edge, to, to_edge });
                }
                e.unsaved.insert("world".into());
                e.dirty = true;
            }
            EditorAction::CyclePortalEdge(edge) => {
                let map = e.map.clone();
                if let Some(p) = e.world.portals.iter_mut().find(|p| p.map == map && p.edge == edge) {
                    let i = EDGES.iter().position(|c| *c == p.to_edge).unwrap_or(0);
                    p.to_edge = EDGES[(i + 1) % EDGES.len()];
                }
                e.unsaved.insert("world".into());
                e.dirty = true;
            }
            EditorAction::Save => {
                let mut errors = Vec::new();
                for id in e.unsaved.clone() {
                    if id == "world" {
                        continue;
                    }
                    if let Some(layout) = e.layouts.get(&id)
                        && let Err(err) = save::save_map(&id, &layout.to_text())
                    {
                        errors.push(err);
                    }
                }
                if let Err(err) = save::save_world(&e.world) {
                    errors.push(err);
                }
                e.unsaved.clear();
                let issues = problems(&e.info(), &e.layout_ref());
                e.message = match (errors.is_empty(), issues.is_empty()) {
                    (false, _) => format!("Não consegui salvar: {}", errors.join("; ")),
                    (true, true) => "Tudo salvo! Já vale no próximo jogo.".into(),
                    (true, false) => format!("Salvo, mas atenção: {}", issues.join(", ")),
                };
            }
            EditorAction::Copy => {
                clipboard(&e.layout_ref().to_text());
                e.message = "Texto do mapa copiado.".into();
            }
            EditorAction::Reset => {
                let id = e.map.clone();
                save::reset_map(&id);
                e.layouts.insert(id.clone(), Layout::original(&id));
                e.dirty = true;
                e.message = format!("{} voltou ao original.", e.info().name);
            }
            EditorAction::Back => next.set(AppState::Menu),
        }
    }
}

fn opposite(edge: char) -> char {
    match edge {
        '<' => '>',
        '>' => '<',
        '^' => 'v',
        _ => '^',
    }
}

/// Renomear: o teclado escreve no nome do mapa até o Enter.
fn type_name(mut keys: MessageReader<KeyboardInput>, mut editor: ResMut<Editor>) {
    if !editor.renaming {
        keys.clear();
        return;
    }
    for key in keys.read() {
        if key.state != ButtonState::Pressed {
            continue;
        }
        let e = &mut *editor;
        match &key.logical_key {
            Key::Enter | Key::Escape => {
                e.renaming = false;
                let info = e.info_mut();
                if info.name.trim().is_empty() {
                    info.name = if info.indoor { "Novo interior".into() } else { "Nova área".into() };
                }
            }
            Key::Backspace => {
                e.info_mut().name.pop();
            }
            logical => {
                let typed = match (&key.text, logical) {
                    (Some(t), _) => t.to_string(),
                    (None, Key::Character(c)) => c.to_string(),
                    (None, Key::Space) => " ".into(),
                    _ => continue,
                };
                let info = e.info_mut();
                for ch in typed.chars().filter(|c| !c.is_control()) {
                    if info.name.chars().count() < 24 {
                        info.name.push(ch);
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn clipboard(text: &str) {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["navigator", "clipboard"], js_name = writeText)]
        fn write_text(text: &str) -> JsValue;
    }
    let _ = write_text(text);
}

#[cfg(not(target_arch = "wasm32"))]
fn clipboard(text: &str) {
    // Sem área de transferência portátil no nativo: o texto vai para o terminal.
    println!("{text}");
}

/// Posição do mouse no mundo, se não estiver em cima da interface.
fn cursor_world(window: &Window, camera: (&Camera, &GlobalTransform)) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    if cursor.x < LEFT_PANEL
        || cursor.x > window.width() - RIGHT_PANEL
        || cursor.y < TOP_BAR
        || cursor.y > window.height() - BOTTOM_BAR
    {
        return None;
    }
    camera.0.viewport_to_world_2d(camera.1, cursor).ok()
}

/// Objeto (âncora + pegada) que cobre o tile, procurando âncoras por perto.
fn prop_covering(layout: &Layout, t: IVec2) -> Option<(IVec2, Vec<IVec2>)> {
    for dy in -1..=3 {
        for dx in -4..=4 {
            let anchor = t + IVec2::new(dx, dy);
            if let Some(footprint) = footprint_of(layout.at(anchor), anchor)
                && footprint.len() > 1
                && footprint.contains(&t)
                && layout.inside(anchor)
            {
                return Some((anchor, footprint));
            }
        }
    }
    None
}

fn paint(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut editor: ResMut<Editor>,
) {
    let Some(world) = cursor_world(&window, *camera) else {
        editor.hover = None;
        return;
    };
    let t = world_to_tile(world);
    let inside = editor.layout().inside(t);
    editor.hover = inside.then_some(t);
    if !inside {
        return;
    }
    let blank = editor.info().blank();
    // Um clique rápido pode apertar e soltar no mesmo quadro: vale o "acabou de apertar".
    let held = |b: MouseButton| mouse.pressed(b) || mouse.just_pressed(b);
    let e = &mut *editor;

    // Apagar: o objeto inteiro, se o tile fizer parte de um (e a ligação da porta junto).
    if held(MouseButton::Right) {
        let covering = prop_covering(e.layout(), t);
        let footprint = covering.as_ref().map(|(_, f)| f.clone()).unwrap_or_else(|| vec![t]);
        if let Some((anchor, _)) = covering {
            let map = e.map.clone();
            let before = e.world.doors.len();
            e.world.doors.retain(|d| !(d.map == map && IVec2::from_array(d.at) == anchor));
            if e.world.doors.len() != before {
                e.unsaved.insert("world".into());
            }
        }
        let mut changed = false;
        for tile in footprint {
            if e.layout().at(tile) != blank {
                e.layout().set(tile, blank);
                changed = true;
            }
        }
        if changed {
            e.touched();
        }
        return;
    }
    if !held(MouseButton::Left) {
        return;
    }
    let brush = e.brush;
    match footprint_of(brush, t) {
        // Objetos grandes: um clique põe a âncora e reserva a pegada com 'h'.
        Some(footprint) if footprint.len() > 1 => {
            if !mouse.just_pressed(MouseButton::Left) {
                return;
            }
            for tile in footprint {
                e.layout().set(tile, 'h');
            }
            e.layout().set(t, brush);
            e.touched();
        }
        _ => {
            if e.layout().at(t) != brush {
                e.layout().set(t, brush);
                e.touched();
            }
        }
    }
}

/// Remonta o desenho do mapa no máximo ~8x por segundo enquanto pinta.
fn rebuild(
    mut commands: Commands,
    time: Res<Time<Real>>,
    sprites: Res<Sprites>,
    mut editor: ResMut<Editor>,
    mut images: ResMut<Assets<Image>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    views: Query<Entity, With<EditorView>>,
) {
    editor.since_build += time.delta_secs();
    if !editor.dirty || editor.since_build < 0.12 {
        return;
    }
    editor.dirty = false;
    editor.since_build = 0.0;
    for entity in &views {
        commands.entity(entity).despawn();
    }
    // As ligações valem enquanto edita (portas e bordas aparecem certas na lista).
    worlddef::set_world(editor.world.clone());
    let info = editor.info();
    let layout = editor.layout_ref();
    let mut painter = Painter { images: &mut images, layouts: &layouts };
    build(&mut commands, &sprites, &mut painter, &info, &layout, (EditorView, DespawnOnExit(AppState::Editor)), &MapEdits::default(), Season::Spring);
}

fn hover(editor: Res<Editor>, mut boxes: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<HoverBox>>) {
    for (mut transform, mut sprite, mut visibility) in &mut boxes {
        let Some(t) = editor.hover else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        visibility.set_if_neq(Visibility::Inherited);
        let footprint = footprint_of(editor.brush, t).unwrap_or_else(|| vec![t]);
        let mut rect = Rect::from_center_size(tile_center(t), Vec2::splat(TILE));
        for f in footprint {
            rect = rect.union(Rect::from_center_size(tile_center(f), Vec2::splat(TILE)));
        }
        transform.translation = rect.center().extend(40.0);
        sprite.custom_size = Some(rect.size());
    }
}

fn pan_zoom(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    mut editor: ResMut<Editor>,
    camera: Single<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let (mut transform, mut projection) = camera.into_inner();
    if editor.renaming {
        wheel.clear();
    } else {
        let pressed = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
        let mut dir = Vec2::ZERO;
        if pressed(KeyCode::KeyW, KeyCode::ArrowUp) {
            dir.y += 1.0;
        }
        if pressed(KeyCode::KeyS, KeyCode::ArrowDown) {
            dir.y -= 1.0;
        }
        if pressed(KeyCode::KeyA, KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if pressed(KeyCode::KeyD, KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        transform.translation += (dir * 260.0 / editor.zoom * time.delta_secs()).extend(0.0);
        let scroll: f32 = wheel.read().map(|w| w.y).sum();
        if scroll != 0.0 {
            editor.zoom = (editor.zoom + scroll.signum()).clamp(1.0, 5.0);
        }
    }
    if let Projection::Orthographic(ortho) = projection.as_mut() {
        ortho.scale = 1.0 / editor.zoom;
    }
}

fn building_name(ch: char) -> &'static str {
    match ch {
        'H' => "Casa",
        '1' => "Padaria",
        '2' => "Armazém",
        '3' => "Ferraria",
        '4' => "Prefeitura",
        '5' => "Biblioteca",
        '6' => "Floricultura",
        '7' => "Guarita",
        'A' => "Celeiro",
        'C' => "Galinheiro",
        _ => "Prédio",
    }
}

/// Chave que diz quando a interface precisa ser remontada (o resto é a linha de status).
fn ui_key(e: &Editor) -> String {
    format!("{}|{}|{}|{:?}|{:?}|{}", e.map, e.brush, e.renaming, e.world, e.unsaved.len(), e.layout_ref().size)
}

/// Painel de pincéis (esquerda), mapas e ligações (direita), arquivo (cima) e status (baixo).
#[allow(clippy::too_many_arguments)]
fn redraw_ui(
    mut commands: Commands,
    sprites: Res<Sprites>,
    editor: Res<Editor>,
    ui: Query<Entity, With<EditorUi>>,
    mut status: Query<&mut Text, With<StatusText>>,
    mut built: Local<String>,
) {
    let info = editor.info();
    let layout = editor.layout_ref();
    let issues = problems(&info, &layout);
    let status_line = format!(
        "{}{} · {}x{} · tile {} · {}{}",
        info.name,
        if editor.unsaved.is_empty() { "" } else { " *" },
        layout.size.x,
        layout.size.y,
        editor.hover.map_or("-".into(), |t| format!("{},{} '{}'", t.x, t.y, layout.at(t))),
        editor.message,
        if issues.is_empty() { String::new() } else { format!(" · atenção: {}", issues.join(", ")) },
    );
    let key = ui_key(&editor);
    if *built == key && !ui.is_empty() {
        for mut text in &mut status {
            if text.0 != status_line {
                text.0.clone_from(&status_line);
            }
        }
        return;
    }
    *built = key;
    for entity in &ui {
        commands.entity(entity).despawn();
    }

    let bar = |node: Node| (Node { position_type: PositionType::Absolute, ..node }, BackgroundColor(PANEL_SOLID));
    let act = |a: EditorAction| UiAction::Editor(a);

    // Cima: tamanho e arquivo.
    commands
        .spawn((
            EditorUi,
            bar(Node {
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Px(TOP_BAR),
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                ..default()
            }),
            DespawnOnExit(AppState::Editor),
        ))
        .with_children(|top| {
            top.spawn((Text::new("Editor de mapas"), font(&sprites, 24.0), TextColor(HINT), Node { margin: UiRect::right(Val::Px(10.0)), ..default() }));
            spawn_button(top, &sprites, "L-", 18.0, act(EditorAction::Resize(-1, 0)));
            spawn_button(top, &sprites, "L+", 18.0, act(EditorAction::Resize(1, 0)));
            spawn_button(top, &sprites, "A-", 18.0, act(EditorAction::Resize(0, -1)));
            spawn_button(top, &sprites, "A+", 18.0, act(EditorAction::Resize(0, 1)));
            top.spawn(Node { width: Val::Px(10.0), ..default() });
            spawn_button(top, &sprites, "Salvar", 18.0, act(EditorAction::Save));
            spawn_button(top, &sprites, "Copiar", 18.0, act(EditorAction::Copy));
            spawn_button(top, &sprites, "Original", 18.0, act(EditorAction::Reset));
            spawn_button(top, &sprites, "Sair", 18.0, act(EditorAction::Back));
        });

    // Esquerda: pincéis.
    commands
        .spawn((
            EditorUi,
            bar(Node {
                left: Val::Px(0.0),
                top: Val::Px(TOP_BAR),
                width: Val::Px(LEFT_PANEL),
                bottom: Val::Px(BOTTOM_BAR),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                padding: UiRect::all(Val::Px(6.0)),
                border: UiRect::right(Val::Px(3.0)),
                overflow: Overflow::clip(),
                ..default()
            }),
            BorderColor::all(PANEL_BORDER),
            DespawnOnExit(AppState::Editor),
        ))
        .with_children(|panel| {
            for (group, brushes) in BRUSHES {
                panel.spawn((Text::new(*group), font(&sprites, 18.0), TextColor(HINT)));
                panel
                    .spawn(Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(2.0), row_gap: Val::Px(2.0), ..default() })
                    .with_children(|grid| {
                        for (ch, name) in *brushes {
                            let label = if *ch == editor.brush { format!(">{name}") } else { name.to_string() };
                            spawn_button(grid, &sprites, label, 15.0, act(EditorAction::Brush(*ch)));
                        }
                    });
            }
        });

    // Direita: mapas, propriedades e ligações.
    let world = &editor.world;
    let name_of = |id: &str| world.maps.iter().find(|m| m.id == id).map_or(id.to_string(), |m| m.name.clone());
    commands
        .spawn((
            EditorUi,
            bar(Node {
                right: Val::Px(0.0),
                top: Val::Px(TOP_BAR),
                width: Val::Px(RIGHT_PANEL),
                bottom: Val::Px(BOTTOM_BAR),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                padding: UiRect::all(Val::Px(6.0)),
                border: UiRect::left(Val::Px(3.0)),
                overflow: Overflow::clip(),
                ..default()
            }),
            BorderColor::all(PANEL_BORDER),
            DespawnOnExit(AppState::Editor),
        ))
        .with_children(|panel| {
            let row = |panel: &mut ChildSpawnerCommands, f: &dyn Fn(&mut ChildSpawnerCommands)| {
                panel
                    .spawn(Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(3.0), row_gap: Val::Px(3.0), align_items: AlignItems::Center, ..default() })
                    .with_children(|r| f(r));
            };
            panel.spawn((Text::new("Mapas"), font(&sprites, 18.0), TextColor(HINT)));
            row(panel, &|r| {
                for m in &world.maps {
                    let mark = if m.id == editor.map { ">" } else { "" };
                    let unsaved = if editor.unsaved.contains(&m.id) { "*" } else { "" };
                    spawn_button(r, &sprites, format!("{mark}{}{unsaved}", m.name), 15.0, act(EditorAction::SelectMap(m.id.clone())));
                }
                spawn_button(r, &sprites, "+ Exterior", 15.0, act(EditorAction::NewMap { indoor: false }));
                spawn_button(r, &sprites, "+ Interior", 15.0, act(EditorAction::NewMap { indoor: true }));
            });

            panel.spawn((Text::new("Este mapa"), font(&sprites, 18.0), TextColor(HINT), Node { margin: UiRect::top(Val::Px(6.0)), ..default() }));
            let name = if editor.renaming { format!("Nome: {}_", info.name) } else { format!("Nome: {}", info.name) };
            row(panel, &|r| {
                r.spawn((Text::new(name.clone()), font(&sprites, 17.0), TextColor(TEXT)));
                spawn_button(r, &sprites, if editor.renaming { "OK" } else { "Renomear" }, 15.0, act(EditorAction::Rename));
            });
            let yes = |b: bool| if b { "sim" } else { "não" };
            row(panel, &|r| {
                spawn_button(r, &sprites, format!("Interior: {}", yes(info.indoor)), 15.0, act(EditorAction::ToggleIndoor));
                spawn_button(r, &sprites, format!("Fazenda: {}", yes(info.farm)), 15.0, act(EditorAction::ToggleFarm));
            });
            row(panel, &|r| {
                let keeper = info.keeper.map_or("ninguém".to_string(), |k| k.short_name().to_string());
                spawn_button(r, &sprites, format!("Balcão: {keeper}"), 15.0, act(EditorAction::CycleKeeper));
                let shop = match info.shop {
                    Some(Shop::Store) => "armazém",
                    Some(Shop::Bakery) => "padaria",
                    Some(Shop::Florist) => "floricultura",
                    Some(Shop::Kitchen) => "cozinha",
                    Some(Shop::Stand) => "banca da fazenda",
                    None => "nenhuma",
                };
                spawn_button(r, &sprites, format!("Loja: {shop}"), 15.0, act(EditorAction::CycleShop));
            });
            row(panel, &|r| {
                let start = if world.start == editor.map { "Mapa inicial ✓" } else { "Tornar inicial" };
                spawn_button(r, &sprites, start.replace('✓', "(sim)"), 15.0, act(EditorAction::SetStart));
                if worlddef::original_text(&editor.map).is_none() {
                    spawn_button(r, &sprites, "Excluir área", 15.0, act(EditorAction::DeleteMap));
                }
            });

            // Portas: cada prédio do mapa escolhe a que interior leva.
            panel.spawn((Text::new("Portas"), font(&sprites, 18.0), TextColor(HINT), Node { margin: UiRect::top(Val::Px(6.0)), ..default() }));
            let mut doors = 0;
            for y in 0..layout.size.y {
                for x in 0..layout.size.x {
                    let t = IVec2::new(x, y);
                    let ch = layout.at(t);
                    if !matches!(crate::map::prop_kind(ch), Some(Prop::House | Prop::Building(_) | Prop::Barn | Prop::Coop)) {
                        continue;
                    }
                    doors += 1;
                    let target = world.door_at(&editor.map, t).map_or("sem interior".to_string(), |d| name_of(&d.to));
                    row(panel, &|r| {
                        r.spawn((Text::new(format!("{} ({x},{y})", building_name(ch))), font(&sprites, 15.0), TextColor(MUTED)));
                        spawn_button(r, &sprites, format!("-> {target}"), 15.0, act(EditorAction::CycleDoor([x, y])));
                    });
                }
            }
            if doors == 0 {
                panel.spawn((Text::new("Pinte uma construção para ligar a porta."), font(&sprites, 15.0), TextColor(MUTED)));
            }

            // Bordas: cada marcador < > ^ v leva à borda de outro mapa.
            panel.spawn((Text::new("Bordas"), font(&sprites, 18.0), TextColor(HINT), Node { margin: UiRect::top(Val::Px(6.0)), ..default() }));
            let mut edges = 0;
            for edge in EDGES {
                if layout.find(edge).is_none() {
                    continue;
                }
                edges += 1;
                let link = world.portal(&editor.map, edge);
                row(panel, &|r| {
                    r.spawn((Text::new(format!("'{edge}'")), font(&sprites, 15.0), TextColor(MUTED)));
                    let to = link.map_or("nenhum lugar".to_string(), |l| name_of(&l.to));
                    spawn_button(r, &sprites, format!("-> {to}"), 15.0, act(EditorAction::CyclePortalMap(edge)));
                    if let Some(l) = link {
                        spawn_button(r, &sprites, format!("chega em '{}'", l.to_edge), 15.0, act(EditorAction::CyclePortalEdge(edge)));
                    }
                });
            }
            if edges == 0 {
                panel.spawn((Text::new("Pinte < > ^ v na borda para sair do mapa."), font(&sprites, 15.0), TextColor(MUTED)));
            }
        });

    // Baixo: status.
    commands
        .spawn((
            EditorUi,
            bar(Node {
                left: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Px(BOTTOM_BAR),
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(10.0)),
                ..default()
            }),
            DespawnOnExit(AppState::Editor),
        ))
        .with_children(|bottom| {
            bottom.spawn((StatusText, Text::new(status_line), font(&sprites, 17.0), TextColor(TEXT)));
        });
}
