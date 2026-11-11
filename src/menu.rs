//! Menu principal: título, jogos salvos e criador/editor de personagem.
//!
//! Cada tela é redesenhada inteira quando o `MenuModel` muda (simples e barato:
//! são poucos nós de UI).

use bevy::app::AppExit;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::character::{Character, Facing, HAIR_COLORS, NAME_MAX, PANTS, SHIRTS, SKINS, Trait};
use crate::editor::EditorAction;
use crate::inventory::Item;
use crate::cycle::half_view;
use crate::map::{Layout, world_size};
use crate::player::clamp_to_map;
use crate::save::{self, ActiveSave, SLOTS, SaveData};
use crate::sprites::Sprites;
use crate::ui::{Clicked, HINT, MUTED, TEXT, font, panel, spawn_button};
use crate::{AppState, Pause};

pub fn plugin(app: &mut App) {
    app.add_sub_state::<MenuScreen>()
        .init_resource::<MenuModel>()
        .init_resource::<GameClicked>()
        .add_systems(OnEnter(AppState::Menu), |mut m: ResMut<MenuModel>| m.refresh_slots())
        .add_systems(OnEnter(MenuScreen::Title), mark_dirty)
        .add_systems(OnEnter(MenuScreen::Load), |mut m: ResMut<MenuModel>| m.refresh_slots())
        .add_systems(OnEnter(MenuScreen::Create), mark_dirty)
        .add_systems(
            Update,
            (
                handle_actions,
                (type_text, back_on_esc, pan_camera, animate_preview, redraw)
                    .run_if(in_state(AppState::Menu)),
            )
                .chain(),
        );
}

#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Menu)]
pub enum MenuScreen {
    #[default]
    Title,
    Load,
    Create,
}

/// Campo de texto com o foco no criador de personagem.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Field {
    #[default]
    Name,
    Farm,
}

/// Tudo que um botão de menu (ou da pausa) pode pedir.
#[derive(Clone, Debug, PartialEq)]
pub enum UiAction {
    NewGame,
    /// Novo jogo num slot vazio específico.
    NewIn(usize),
    OpenLoad,
    Back,
    Play(usize),
    Edit(usize),
    Delete(usize),
    Confirm,
    Cycle(Trait, i32),
    Focus(Field),
    Resume,
    QuitToMenu,
    /// Só existe no nativo: no navegador basta fechar a aba.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    Exit,
    OpenEditor,
    // Daqui para baixo, tratadas pelo jogo (shop.rs) e pelo editor (editor.rs).
    /// Clique num espaço da barra/mochila.
    Slot(usize),
    Buy(Item, u32),
    #[allow(dead_code)]
    Sell(Item, u32),
    SellQuality(Item, crate::inventory::Quality, u32),
    ShopTab(bool),
    ScrollShop(f32),
    ExportEssence,
    /// Muda a quantidade escolhida (item, é compra?, quanto somar).
    Qty(Item, bool, i32),
    /// Quantidade máxima possível (item, é compra?).
    QtyMax(Item, bool),
    QtyQuality(Item, crate::inventory::Quality, bool, i32),
    QtyQualityMax(Item, crate::inventory::Quality, bool),
    FreeBread,
    Lunch,
    Cook(Item),
    CloseOverlay,
    Editor(EditorAction),
    // Ações do Inventário Dedicado e Crafting
    BagTab(crate::hud::BagTab),
    #[allow(dead_code)]
    EquipMask(Option<crate::inventory::MaskKind>),
    CycleMask,
    /// Arruma a mochila: junta pilhas e ordena por tipo.
    SortBag,
    SelectCraftRecipe(usize),
    CraftRecipe(usize),
    CraftCategoryFilter(crate::crafting::CraftCategory),
}

/// Ações de botões que o menu repassa para o jogo ou para o editor.
#[derive(Resource, Default)]
pub struct GameClicked(pub Vec<UiAction>);

#[derive(Resource, Default)]
struct MenuModel {
    slots: Vec<Option<SaveData>>,
    character: Character,
    /// Slot onde o personagem do criador será gravado.
    target: usize,
    /// Editando um save existente (em vez de criar um jogo novo).
    editing: bool,
    field: Field,
    confirm_delete: Option<usize>,
    message: Option<String>,
}

impl MenuModel {
    fn refresh_slots(&mut self) {
        self.slots = (0..SLOTS).map(save::read).collect();
        self.confirm_delete = None;
    }
}

fn mark_dirty(mut model: ResMut<MenuModel>) {
    model.set_changed();
}

/// Raiz da tela atual; some quando a tela é redesenhada ou o menu fecha.
#[derive(Component)]
struct ScreenRoot;

/// Peça do boneco de prévia no criador.
#[derive(Component)]
struct PreviewPart;

#[allow(clippy::too_many_arguments)]
fn handle_actions(
    mut clicked: ResMut<Clicked>,
    mut model: ResMut<MenuModel>,
    mut commands: Commands,
    mut app_state: ResMut<NextState<AppState>>,
    mut screen: ResMut<NextState<MenuScreen>>,
    mut pause: ResMut<NextState<Pause>>,
    mut exit: MessageWriter<AppExit>,
    mut forward: ResMut<GameClicked>,
) {
    for action in std::mem::take(&mut clicked.0) {
        let model = &mut *model;
        if !matches!(action, UiAction::Delete(_)) {
            model.confirm_delete = None;
        }
        match action {
            UiAction::NewGame => match save::free_slot() {
                Some(slot) => start_creator(model, &mut screen, slot),
                None => {
                    model.message = Some("Os 3 slots estão ocupados. Apague um para começar outro jogo.".into());
                    screen.set(MenuScreen::Load);
                }
            },
            UiAction::NewIn(slot) => start_creator(model, &mut screen, slot),
            UiAction::OpenLoad => {
                model.message = None;
                screen.set(MenuScreen::Load);
            }
            UiAction::Back => {
                model.message = None;
                screen.set(if model.editing { MenuScreen::Load } else { MenuScreen::Title });
            }
            UiAction::Play(slot) => match save::read(slot) {
                Some(data) => {
                    commands.insert_resource(ActiveSave { slot, data });
                    app_state.set(AppState::Playing);
                }
                None => model.message = Some("Não consegui ler esse save.".into()),
            },
            UiAction::Edit(slot) => {
                if let Some(Some(data)) = model.slots.get(slot) {
                    model.character = data.character.clone();
                    model.target = slot;
                    model.editing = true;
                    model.field = Field::Name;
                    model.message = None;
                    screen.set(MenuScreen::Create);
                }
            }
            UiAction::Delete(slot) => {
                if model.confirm_delete == Some(slot) {
                    save::delete(slot);
                    model.refresh_slots();
                    model.message = Some(format!("Slot {} apagado.", slot + 1));
                } else {
                    model.confirm_delete = Some(slot);
                }
            }
            UiAction::Confirm => confirm_character(model, &mut commands, &mut screen, &mut app_state),
            UiAction::Cycle(t, step) => model.character.cycle(t, step),
            UiAction::Focus(field) => model.field = field,
            UiAction::Resume => pause.set(Pause::Running),
            UiAction::QuitToMenu => app_state.set(AppState::Menu),
            UiAction::Exit => {
                exit.write(AppExit::Success);
            }
            UiAction::OpenEditor => app_state.set(AppState::Editor),
            other => forward.0.push(other),
        }
    }
}

fn start_creator(model: &mut MenuModel, screen: &mut NextState<MenuScreen>, slot: usize) {
    model.character = Character::default();
    model.target = slot;
    model.editing = false;
    model.field = Field::Name;
    model.message = None;
    screen.set(MenuScreen::Create);
}

fn confirm_character(
    model: &mut MenuModel,
    commands: &mut Commands,
    screen: &mut NextState<MenuScreen>,
    app_state: &mut NextState<AppState>,
) {
    let c = &mut model.character;
    c.name = c.name.trim().to_string();
    c.farm = c.farm.trim().to_string();
    if c.name.is_empty() || c.farm.is_empty() {
        model.message = Some("Dê um nome a você e à sua fazenda.".into());
        return;
    }
    let slot = model.target;
    if model.editing {
        let Some(Some(mut data)) = model.slots.get(slot).cloned() else { return };
        data.character = model.character.clone();
        model.message = Some(match save::write(slot, &data) {
            Ok(()) => format!("{} atualizado.", data.character.name),
            Err(e) => format!("Não consegui salvar: {e}"),
        });
        model.refresh_slots();
        screen.set(MenuScreen::Load);
    } else {
        let data = SaveData::new(model.character.clone());
        if let Err(e) = save::write(slot, &data) {
            model.message = Some(format!("Não consegui salvar: {e}"));
            return;
        }
        commands.insert_resource(ActiveSave { slot, data });
        app_state.set(AppState::Playing);
    }
}

/// Teclado no criador: digita no campo com foco, Tab troca de campo, Enter confirma.
fn type_text(
    screen: Res<State<MenuScreen>>,
    mut keys: MessageReader<KeyboardInput>,
    mut model: ResMut<MenuModel>,
    mut clicked: ResMut<Clicked>,
) {
    if *screen.get() != MenuScreen::Create {
        keys.clear();
        return;
    }
    for key in keys.read() {
        if key.state != ButtonState::Pressed {
            continue;
        }
        let field = model.field;
        let text = match field {
            Field::Name => &mut model.character.name,
            Field::Farm => &mut model.character.farm,
        };
        match &key.logical_key {
            Key::Backspace => {
                text.pop();
            }
            Key::Tab => {
                model.field = if field == Field::Name { Field::Farm } else { Field::Name };
            }
            Key::Enter => clicked.0.push(UiAction::Confirm),
            logical => {
                // Alguns navegadores não preenchem `text`; a tecla lógica cobre o resto.
                let typed = match (&key.text, logical) {
                    (Some(t), _) => t.to_string(),
                    (None, Key::Character(c)) => c.to_string(),
                    (None, Key::Space) => " ".into(),
                    _ => continue,
                };
                {
                    for ch in typed.chars().filter(|c| !c.is_control()) {
                        if text.chars().count() < NAME_MAX {
                            text.push(ch);
                        }
                    }
                }
            }
        }
    }
}

fn back_on_esc(keys: Res<ButtonInput<KeyCode>>, screen: Res<State<MenuScreen>>, mut clicked: ResMut<Clicked>) {
    if keys.just_pressed(KeyCode::Escape) && *screen.get() != MenuScreen::Title {
        clicked.0.push(UiAction::Back);
    }
}

/// O fundo do menu é a própria fazenda, com a câmera passeando devagar.
fn pan_camera(
    time: Res<Time<Real>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&mut Transform, &Projection), With<Camera2d>>,
    mut farm_size: Local<Option<Vec2>>,
) {
    let (mut camera, projection) = camera.into_inner();
    // O tamanho da fazenda não muda com o menu aberto: lê (e interpreta) o mapa uma vez só.
    let size = *farm_size.get_or_insert_with(|| {
        let world = crate::worlddef::world();
        world_size(Layout::of(&world.farm_id().unwrap_or(world.start)).size)
    });
    let t = time.elapsed_secs();
    let p = Vec2::new(size.x * (0.5 + 0.3 * (t * 0.04).sin()), -size.y * (0.4 + 0.15 * (t * 0.03).cos()));
    let p = clamp_to_map(p, half_view(&window, projection), size);
    camera.translation = p.round().extend(camera.translation.z);
}

fn animate_preview(time: Res<Time<Real>>, mut parts: Query<&mut ImageNode, With<PreviewPart>>) {
    let t = time.elapsed_secs();
    let facing = [Facing::Down, Facing::Right, Facing::Up, Facing::Left][(t / 2.0) as usize % 4];
    let step = [0, 1, 0, 2][(t * 6.0) as usize % 4];
    for mut image in &mut parts {
        image.flip_x = facing.flip();
        if let Some(atlas) = &mut image.texture_atlas {
            atlas.index = facing.frame(step);
        }
    }
}

fn redraw(
    mut commands: Commands,
    model: Res<MenuModel>,
    screen: Res<State<MenuScreen>>,
    sprites: Res<Sprites>,
    roots: Query<Entity, With<ScreenRoot>>,
) {
    if !model.is_changed() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    let root = commands
        .spawn((
            ScreenRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.03, 0.08, 0.35)),
            DespawnOnExit(AppState::Menu),
        ))
        .id();
    commands.entity(root).with_children(|ui| match screen.get() {
        MenuScreen::Title => title_screen(ui, &sprites),
        MenuScreen::Load => load_screen(ui, &sprites, &model),
        MenuScreen::Create => create_screen(ui, &sprites, &model),
    });
}

fn title_screen(ui: &mut ChildSpawnerCommands, sprites: &Sprites) {
    ui.spawn((
        Text::new("Cozy Curses"),
        font(sprites, 120.0),
        TextColor(Color::srgb(1.0, 0.86, 0.55)),
        TextShadow { offset: Vec2::splat(5.0), color: Color::srgba(0.12, 0.05, 0.12, 0.9) },
    ));
    ui.spawn((
        Text::new("Cozy de dia. Curse de noite."),
        font(sprites, 32.0),
        TextColor(TEXT),
        TextShadow { offset: Vec2::splat(2.0), color: Color::srgba(0.0, 0.0, 0.0, 0.8) },
        Node { margin: UiRect::bottom(Val::Px(24.0)), ..default() },
    ));
    ui.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), ..default() })
        .with_children(|col| {
            spawn_button(col, sprites, "Novo jogo", 34.0, UiAction::NewGame);
            spawn_button(col, sprites, "Carregar jogo", 34.0, UiAction::OpenLoad);
            spawn_button(col, sprites, "Editor de mapas", 34.0, UiAction::OpenEditor);
            #[cfg(not(target_arch = "wasm32"))]
            spawn_button(col, sprites, "Sair", 34.0, UiAction::Exit);
        });
    ui.spawn((
        Text::new("protótipo · WASD andar · E interagir · Esc menu"),
        font(sprites, 20.0),
        TextColor(MUTED),
        Node { position_type: PositionType::Absolute, bottom: Val::Px(8.0), ..default() },
    ));
}

fn heading(ui: &mut ChildSpawnerCommands, sprites: &Sprites, text: &str) {
    ui.spawn((
        Text::new(text),
        font(sprites, 56.0),
        TextColor(Color::srgb(1.0, 0.86, 0.55)),
        TextShadow { offset: Vec2::splat(3.0), color: Color::srgba(0.12, 0.05, 0.12, 0.9) },
    ));
}

fn message(ui: &mut ChildSpawnerCommands, sprites: &Sprites, model: &MenuModel) {
    if let Some(msg) = &model.message {
        ui.spawn((Text::new(msg.clone()), font(sprites, 24.0), TextColor(HINT)));
    }
}

fn load_screen(ui: &mut ChildSpawnerCommands, sprites: &Sprites, model: &MenuModel) {
    heading(ui, sprites, "Jogos salvos");
    for (slot, data) in model.slots.iter().enumerate() {
        ui.spawn(panel(Node {
            width: Val::Px(720.0),
            padding: UiRect::all(Val::Px(12.0)),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        }))
        .with_children(|card| {
            let (summary, color) = match data {
                Some(d) => (
                    format!(
                        "Slot {} · {}\n{}\nDia {} · {} ouro · maldição {:.0}%",
                        slot + 1,
                        d.character.name,
                        d.character.farm,
                        d.day,
                        d.game.gold,
                        d.game.curse
                    ),
                    TEXT,
                ),
                None => (format!("Slot {}\n(vazio)", slot + 1), MUTED),
            };
            card.spawn((Text::new(summary), font(sprites, 24.0), TextColor(color)));
            card.spawn(Node { column_gap: Val::Px(8.0), ..default() }).with_children(|row| {
                if data.is_some() {
                    spawn_button(row, sprites, "Jogar", 26.0, UiAction::Play(slot));
                    spawn_button(row, sprites, "Editar", 26.0, UiAction::Edit(slot));
                    let delete = if model.confirm_delete == Some(slot) { "Confirmar?" } else { "Apagar" };
                    spawn_button(row, sprites, delete, 26.0, UiAction::Delete(slot));
                } else {
                    spawn_button(row, sprites, "Novo jogo aqui", 26.0, UiAction::NewIn(slot));
                }
            });
        });
    }
    message(ui, sprites, model);
    spawn_button(ui, sprites, "Voltar", 28.0, UiAction::Back);
}

fn create_screen(ui: &mut ChildSpawnerCommands, sprites: &Sprites, model: &MenuModel) {
    heading(ui, sprites, if model.editing { "Editar personagem" } else { "Novo fazendeiro" });
    ui.spawn(panel(Node { padding: UiRect::all(Val::Px(18.0)), column_gap: Val::Px(28.0), ..default() }))
        .with_children(|card| {
            // Prévia: as 4 peças empilhadas, 6x maior, girando e andando.
            card.spawn(Node { width: Val::Px(96.0), height: Val::Px(192.0), align_self: AlignSelf::Center, ..default() })
                .with_children(|preview| {
                    for (art, color) in model.character.parts(sprites) {
                        preview.spawn((
                            PreviewPart,
                            ImageNode {
                                image: art.day.clone(),
                                texture_atlas: art.layout.clone().map(|layout| TextureAtlas { layout, index: 0 }),
                                color,
                                ..default()
                            },
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                        ));
                    }
                });

            card.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), ..default() })
                .with_children(|form| {
                    text_field(form, sprites, "Nome", &model.character.name, Field::Name, model.field);
                    text_field(form, sprites, "Fazenda", &model.character.farm, Field::Farm, model.field);
                    for t in Trait::ALL {
                        trait_row(form, sprites, &model.character, t);
                    }
                });
        });
    message(ui, sprites, model);
    ui.spawn(Node { column_gap: Val::Px(12.0), ..default() }).with_children(|row| {
        let confirm = if model.editing { "Salvar" } else { "Começar" };
        spawn_button(row, sprites, confirm, 32.0, UiAction::Confirm);
        spawn_button(row, sprites, "Voltar", 32.0, UiAction::Back);
    });
}

fn text_field(
    form: &mut ChildSpawnerCommands,
    sprites: &Sprites,
    label: &str,
    value: &str,
    field: Field,
    focused: Field,
) {
    let active = field == focused;
    form.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() })
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                font(sprites, 26.0),
                TextColor(MUTED),
                Node { width: Val::Px(150.0), ..default() },
            ));
            row.spawn((
                Node {
                    width: Val::Px(240.0),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(if active { Color::srgb(0.95, 0.9, 0.78) } else { Color::srgb(0.75, 0.7, 0.6) }),
                BorderColor::all(if active { HINT } else { Color::srgb(0.3, 0.2, 0.12) }),
            ))
            .observe(move |_: On<bevy::picking::events::PointerPress>, mut clicked: ResMut<Clicked>| {
                clicked.0.push(UiAction::Focus(field));
            })
            .with_child((
                Text::new(if active { format!("{value}_") } else { value.to_string() }),
                font(sprites, 26.0),
                TextColor(Color::srgb(0.2, 0.12, 0.08)),
            ));
        });
}

fn trait_row(form: &mut ChildSpawnerCommands, sprites: &Sprites, character: &Character, t: Trait) {
    let swatch = |list: &[(u8, u8, u8)], i: usize| {
        let (r, g, b) = list[i.min(list.len() - 1)];
        Some(Color::srgb_u8(r, g, b))
    };
    let color = match t {
        Trait::Skin => swatch(&SKINS, character.skin),
        Trait::HairColor => swatch(&HAIR_COLORS, character.hair_color),
        Trait::Shirt => swatch(&SHIRTS, character.shirt),
        Trait::Pants => swatch(&PANTS, character.pants),
        Trait::Hair => None,
    };
    form.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() })
        .with_children(|row| {
            row.spawn((
                Text::new(t.label()),
                font(sprites, 26.0),
                TextColor(MUTED),
                Node { width: Val::Px(150.0), ..default() },
            ));
            spawn_button(row, sprites, "<", 24.0, UiAction::Cycle(t, -1));
            row.spawn((
                Text::new(character.value_label(t)),
                font(sprites, 26.0),
                TextColor(TEXT),
                TextLayout::justify(Justify::Center),
                Node { width: Val::Px(110.0), ..default() },
            ));
            spawn_button(row, sprites, ">", 24.0, UiAction::Cycle(t, 1));
            if let Some(color) = color {
                row.spawn((
                    Node {
                        width: Val::Px(26.0),
                        height: Val::Px(26.0),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(color),
                    BorderColor::all(Color::srgb(0.2, 0.12, 0.08)),
                ));
            }
        });
}
