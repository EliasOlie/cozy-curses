//! HUD contextual: só mostra o que importa agora.
//! - relógio e ouro sempre (o ouro "conta" até o valor novo);
//! - números do império aparecem quando algo muda (ou segurando Alt) e somem sozinhos;
//! - diário some alguns segundos depois da última mensagem; ajuda só no começo (ou H);
//! - barra de itens com o espaço escolhido dando um pulinho; nome do item ao trocar;
//! - F3: diagnóstico (FPS, entidades, mapa, tile).

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::picking::events::PointerPress;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::cycle::{Clock, DayPhase};
use crate::dialogue::Dialogue;
use crate::economy::{DawnCard, GOAL, Game, MAX_ENERGY, MAX_HEALTH};
use crate::inventory::{HOTBAR, Inventory, SLOTS};
use crate::map::{Map, world_to_tile};
use crate::menu::UiAction;
use crate::player::Player;
use crate::sprites::Sprites;
use crate::ui::{Clicked, CozyPanel, HINT, INK, MUTED, TEXT, UiBounce, font, item_icon, panel, sliced, spawn_button};
use crate::{AppState, Overlay};

pub fn plugin(app: &mut App) {
    app.add_plugins(FrameTimeDiagnosticsPlugin::default())
        .init_resource::<Held>()
        .init_resource::<HudMemory>()
        .init_resource::<BagState>()
        .add_systems(
            Update,
            (update_hud, update_slots, update_world_balloon, toggle_bag, redraw_bag, toggle_debug, dawn_card).run_if(in_state(AppState::Playing)),
        )
        .add_systems(Update, (bag_shortcuts, bag_details).run_if(in_state(AppState::Playing)))
        .add_systems(OnExit(AppState::Playing), |mut h: ResMut<Held>, mut m: ResMut<HudMemory>| {
            h.0 = None;
            *m = HudMemory::default();
        });
}

/// O texto dentro de um painel do HUD.
#[derive(Component)]
struct HudText;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum HudSlot {
    Clock,
    Stats,
    Hint,
    Log,
    Help,
    ItemName,
    Debug,
}

/// Um espaço de item na tela (barra ou mochila) com o índice na mochila.
#[derive(Component)]
struct SlotView(usize);

#[derive(Component)]
struct SlotIcon(usize);

#[derive(Component)]
struct SlotCount(usize);

/// Pontinho de qualidade no canto do espaço.
#[derive(Component)]
struct SlotPip(usize);

#[derive(Component)]
struct BagPanel;

/// Ícone grande do cartão de detalhes da mochila.
#[derive(Component)]
struct DetailIcon;

/// Linhas de texto do cartão de detalhes.
#[derive(Component, Clone, Copy)]
enum DetailLine {
    Title,
    Kind,
    Effect,
}

/// Espaço da mochila "na mão" (primeiro clique de uma troca).
#[derive(Resource, Default)]
pub struct Held(pub Option<usize>);

/// O que o HUD lembra para decidir o que mostrar.
#[derive(Resource, Default)]
struct HudMemory {
    /// (ouro, energia, maldição, essência, máscara) da última vez.
    last_stats: Option<(i32, i32, i32, i32, i32, bool)>,
    stats_until: f32,
    last_log: Vec<String>,
    log_until: f32,
    shown_gold: f32,
    last_selected: Option<(usize, Option<crate::inventory::Item>)>,
    item_until: f32,
    debug: bool,
}

const SLOT_SIZE: f32 = 48.0;
const STATS_SECONDS: f32 = 4.0;
const LOG_SECONDS: f32 = 6.0;
const HELP_SECONDS: f32 = 45.0;

/// Painel com moldura e um texto dentro. O `HudSlot` fica no painel (que some e aparece);
/// o texto é o filho.
fn panel_text(
    commands: &mut Commands,
    sprites: &Sprites,
    slot: HudSlot,
    node: Node,
    size: f32,
    color: Color,
    justify: Justify,
) -> Entity {
    commands
        .spawn((slot, panel(node), Visibility::Hidden, Pickable::IGNORE, DespawnOnExit(AppState::Playing)))
        .with_child((
            HudText,
            Text::new(""),
            font(sprites, size),
            TextColor(color),
            TextShadow { offset: Vec2::splat(1.5), color: Color::BLACK },
            TextLayout::justify(justify),
        ))
        .id()
}

pub fn spawn_hud(mut commands: Commands, sprites: Res<Sprites>) {
    let scoped = DespawnOnExit(AppState::Playing);
    let absolute = |node: Node| Node { position_type: PositionType::Absolute, ..node };
    let c = &mut commands;

    panel_text(c, &sprites, HudSlot::Stats, absolute(Node { left: Val::Px(8.0), top: Val::Px(8.0), ..default() }), 22.0, TEXT, Justify::Left);
    panel_text(c, &sprites, HudSlot::Clock, absolute(Node { right: Val::Px(8.0), top: Val::Px(8.0), ..default() }), 26.0, TEXT, Justify::Right);
    panel_text(
        c,
        &sprites,
        HudSlot::Log,
        absolute(Node { left: Val::Px(8.0), bottom: Val::Px(126.0), max_width: Val::Px(560.0), ..default() }),
        20.0,
        TEXT,
        Justify::Left,
    );
    commands
        .spawn((HudSlot::Help, absolute(Node { right: Val::Px(12.0), top: Val::Px(104.0), ..default() }), Visibility::Hidden, Pickable::IGNORE, scoped.clone()))
        .with_child((
            HudText,
            Text::new("WASD andar · Shift correr · E/clique dir. interagir · Espaço/clique usar\n1-0 barra · I mochila · F celular · Alt números · H ajuda · Esc menu"),
            font(&sprites, 18.0),
            TextColor(MUTED),
            TextLayout::justify(Justify::Right),
        ));
    commands
        .spawn((HudSlot::Debug, absolute(Node { left: Val::Px(10.0), bottom: Val::Px(180.0), ..default() }), Visibility::Hidden, Pickable::IGNORE, scoped.clone()))
        .with_child((HudText, Text::new(""), font(&sprites, 18.0), TextColor(Color::srgb(0.6, 1.0, 0.6))));

    // Ação disponível e nome do item, logo acima da barra.
    for slot in [HudSlot::Hint, HudSlot::ItemName] {
        let row = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(84.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
                scoped.clone(),
            ))
            .id();
        let inner = panel_text(&mut commands, &sprites, slot, Node::default(), 24.0, HINT, Justify::Center);
        commands.entity(row).add_child(inner);
    }

    // Balão contextual flutuante de QoL (sobre plantações e animais)
    commands
        .spawn((
            WorldBalloon,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(-9999.0),
                top: Val::Px(-9999.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.5)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.92)),
            BorderColor::all(Color::srgba(0.9, 0.75, 0.45, 0.95)),
            Visibility::Hidden,
            Pickable::IGNORE,
            scoped.clone(),
        ))
        .with_child((
            WorldBalloonText,
            Text::new(""),
            font(&sprites, 18.0),
            TextColor(Color::WHITE),
            TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::BLACK },
        ));

    // Barra de itens (os 10 primeiros espaços da mochila).
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(8.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            scoped,
        ))
        .with_children(|row| {
            row.spawn(panel(Node { padding: UiRect::all(Val::Px(8.0)), column_gap: Val::Px(2.0), ..default() }))
                .with_children(|bar| {
                    for i in 0..HOTBAR {
                        spawn_slot(bar, &sprites, i);
                    }
                });
        });
}

fn spawn_slot(parent: &mut ChildSpawnerCommands, sprites: &Sprites, i: usize) {
    parent
        .spawn((
            SlotView(i),
            Node {
                width: Val::Px(SLOT_SIZE),
                height: Val::Px(SLOT_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode::new(sprites.ui.slot.clone()),
            UiTransform::IDENTITY,
            Hovered::default(),
        ))
        .observe(move |_: On<PointerPress>, mut clicked: ResMut<Clicked>| {
            clicked.0.push(UiAction::Slot(i));
        })
        .with_children(|slot| {
            slot.spawn((SlotIcon(i), item_icon(sprites, crate::inventory::Item::Hoe, 32.0), Visibility::Hidden, Pickable::IGNORE));
            if i < HOTBAR {
                let digit = if i == 9 { "0" } else { &((i + 1).to_string()) };
                slot.spawn((
                    Text::new(digit),
                    font(sprites, 15.0),
                    TextColor(Color::srgb(0.78, 0.74, 0.68)),
                    TextShadow { offset: Vec2::splat(1.0), color: Color::BLACK },
                    Node { position_type: PositionType::Absolute, left: Val::Px(4.0), top: Val::Px(1.0), ..default() },
                    Pickable::IGNORE,
                ));
            }
            slot.spawn((
                SlotPip(i),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(5.0),
                    top: Val::Px(5.0),
                    width: Val::Px(8.0),
                    height: Val::Px(8.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                Visibility::Hidden,
                Pickable::IGNORE,
            ));
            slot.spawn((
                SlotCount(i),
                Text::new(""),
                font(sprites, 20.0),
                TextColor(TEXT),
                TextShadow { offset: Vec2::splat(1.5), color: Color::BLACK },
                Node { position_type: PositionType::Absolute, right: Val::Px(2.0), bottom: Val::Px(0.0), ..default() },
                Pickable::IGNORE,
            ));
        });
}

/// Ícone, quantidade e destaque de cada espaço visível; pulinho no escolhido.
#[allow(clippy::too_many_arguments)]
fn update_slots(
    mut commands: Commands,
    sprites: Res<Sprites>,
    inv: Option<Res<Inventory>>,
    held: Res<Held>,
    mut views: Query<(Entity, &SlotView, &mut ImageNode), Without<SlotIcon>>,
    mut icons: Query<(&SlotIcon, &mut ImageNode, &mut Visibility), Without<SlotView>>,
    mut counts: Query<(&SlotCount, &mut Text)>,
    mut pips: Query<(&SlotPip, &mut BackgroundColor, &mut Visibility), (Without<SlotIcon>, Without<SlotView>)>,
    mut last_selected: Local<Option<usize>>,
) {
    let Some(inv) = inv else { return };
    let changed_selection = *last_selected != Some(inv.selected);
    *last_selected = Some(inv.selected);
    for (entity, view, mut image) in &mut views {
        let lit = view.0 == inv.selected || held.0 == Some(view.0);
        let art = if lit { &sprites.ui.slot_selected } else { &sprites.ui.slot };
        if image.image != *art {
            image.image = art.clone();
        }
        if changed_selection && view.0 == inv.selected {
            commands.entity(entity).insert(UiBounce::new(0.12));
        }
    }
    for (icon, mut image, mut visibility) in &mut icons {
        match inv.slots.get(icon.0).copied().flatten() {
            Some(stack) => {
                visibility.set_if_neq(Visibility::Inherited);
                if let Some(atlas) = &mut image.texture_atlas
                    && atlas.index != stack.item.icon()
                {
                    atlas.index = stack.item.icon();
                }
            }
            None => _ = visibility.set_if_neq(Visibility::Hidden),
        }
    }
    for (count, mut text) in &mut counts {
        let label = match inv.slots.get(count.0).copied().flatten() {
            Some(stack) if stack.count > 1 => stack.count.to_string(),
            _ => String::new(),
        };
        if text.0 != label {
            text.0 = label;
        }
    }
    // Qualidade: pontinho colorido no canto (bronze, prata, ouro, sobrenatural).
    for (pip, mut color, mut visibility) in &mut pips {
        match inv.slots.get(pip.0).copied().flatten() {
            Some(stack) if stack.quality != crate::inventory::Quality::Normal => {
                visibility.set_if_neq(Visibility::Inherited);
                if color.0 != stack.quality.color() {
                    color.0 = stack.quality.color();
                }
            }
            _ => _ = visibility.set_if_neq(Visibility::Hidden),
        }
    }
}

/// Qual espaço o mouse está em cima (com a mochila aberta).
fn hovered_slot(views: &Query<(&SlotView, &Hovered)>) -> Option<usize> {
    views.iter().find(|(_, hovered)| hovered.get()).map(|(view, _)| view.0)
}

/// Atalhos com a mochila aberta: R organiza; 1..0 manda o item sob o mouse para aquele
/// espaço da barra.
fn bag_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    overlay: Res<Overlay>,
    bag_state: Res<BagState>,
    views: Query<(&SlotView, &Hovered)>,
    mut inv: Option<ResMut<Inventory>>,
    mut held: ResMut<Held>,
    mut clicked: ResMut<Clicked>,
) {
    if *overlay != Overlay::Bag || bag_state.tab != BagTab::Inventory {
        return;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        clicked.0.push(UiAction::SortBag);
    }
    const DIGITS: [KeyCode; HOTBAR] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
        KeyCode::Digit0,
    ];
    let (Some(from), Some(inv)) = (hovered_slot(&views), &mut inv) else { return };
    for (to, key) in DIGITS.iter().enumerate() {
        if keys.just_pressed(*key) {
            inv.merge_or_swap(from, to);
            held.0 = None;
        }
    }
}

/// Cartão de detalhes: o item sob o mouse (ou o que está "na mão", ou o escolhido na barra).
#[allow(clippy::type_complexity)]
fn bag_details(
    overlay: Res<Overlay>,
    inv: Option<Res<Inventory>>,
    held: Res<Held>,
    views: Query<(&SlotView, &Hovered)>,
    mut icon: Query<(&mut ImageNode, &mut Visibility), With<DetailIcon>>,
    mut lines: Query<(&DetailLine, &mut Text, &mut TextColor)>,
) {
    let Some(inv) = inv else { return };
    if *overlay != Overlay::Bag || lines.is_empty() {
        return;
    }
    let slot = hovered_slot(&views).or(held.0).unwrap_or(inv.selected);
    let stack = inv.slots.get(slot).copied().flatten();
    for (mut image, mut visibility) in &mut icon {
        visibility.set_if_neq(if stack.is_some() { Visibility::Inherited } else { Visibility::Hidden });
        if let (Some(stack), Some(atlas)) = (stack, &mut image.texture_atlas)
            && atlas.index != stack.item.icon()
        {
            atlas.index = stack.item.icon();
        }
    }
    let (title, title_color, kind, effect) = match stack {
        None => ("Espaço vazio".to_string(), INK, String::new(), String::new()),
        Some(stack) => {
            let item = stack.item;
            let quality = stack.quality;
            let title = if quality == crate::inventory::Quality::Normal {
                item.name().to_string()
            } else {
                format!("{} {}", item.name(), quality.badge())
            };
            let owned = inv.count(item);
            let kind = if owned > stack.count {
                format!("{} · {} aqui, {owned} no total", item.category().1, stack.count)
            } else if stack.count > 1 {
                format!("{} · {} unidades", item.category().1, stack.count)
            } else {
                item.category().1.to_string()
            };
            let mut effects = Vec::new();
            if let Some(energy) = stack.energy() {
                effects.push(format!("Comer: +{:.0} energia, +{:.0} vida", energy, energy * 0.5));
            }
            if let Some(damage) = item.weapon_damage() {
                effects.push(format!("Dano: {damage:.0}"));
            }
            if let Some(crop) = item.seed_crop() {
                effects.push(format!("Colhe em {} dias", crop.growth_days()));
            }
            if let Some(mask) = item.mask_kind() {
                effects.push(mask.bonus_text().to_string());
            }
            match stack.price() {
                Some(price) if stack.count > 1 => effects.push(format!("Vende por {price} ouro ({} a pilha)", price * stack.count as i32)),
                Some(price) => effects.push(format!("Vende por {price} ouro")),
                None => {}
            }
            let color = if quality == crate::inventory::Quality::Normal { INK } else { quality.color().darker(0.25) };
            (title, color, kind, effects.join(" · "))
        }
    };
    for (line, mut text, mut color) in &mut lines {
        let (wanted, wanted_color) = match line {
            DetailLine::Title => (&title, title_color),
            DetailLine::Kind => (&kind, INK),
            DetailLine::Effect => (&effect, INK),
        };
        if text.0 != *wanted {
            text.0 = wanted.clone();
        }
        if color.0 != wanted_color {
            color.0 = wanted_color;
        }
    }
}

/// Abas da tela de inventário dedicado.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BagTab {
    #[default]
    Inventory,
    Status,
    Crafting,
}

#[derive(Resource, Default)]
pub struct BagState {
    pub tab: BagTab,
    pub selected_recipe: usize,
    pub craft_category: crate::crafting::CraftCategory,
    pub dirty: bool,
}

/// I ou Tab abre/fecha o inventário dedicado. C abre Crafting, M cicla máscaras.
fn toggle_bag(
    keys: Res<ButtonInput<KeyCode>>,
    mut overlay: ResMut<Overlay>,
    mut held: ResMut<Held>,
    mut bag_state: ResMut<BagState>,
    mut game: Option<ResMut<Game>>,
) {
    if keys.just_pressed(KeyCode::KeyI) || keys.just_pressed(KeyCode::Tab) {
        *overlay = match *overlay {
            Overlay::None => {
                bag_state.dirty = true;
                Overlay::Bag
            }
            Overlay::Bag => Overlay::None,
            other => other,
        };
        held.0 = None;
    }

    if keys.just_pressed(KeyCode::KeyC) && *overlay == Overlay::None {
        bag_state.tab = BagTab::Crafting;
        bag_state.dirty = true;
        *overlay = Overlay::Bag;
        held.0 = None;
    }

    if keys.just_pressed(KeyCode::KeyM) {
        if let Some(game) = &mut game {
            if let Some(mask) = game.cycle_mask() {
                game.say(format!("Equipou {}: {}", mask.name(), mask.bonus_text()));
            } else {
                game.say("Removeu a máscara.");
            }
            bag_state.dirty = true;
        }
    }
}

fn toggle_debug(keys: Res<ButtonInput<KeyCode>>, mut memory: ResMut<HudMemory>) {
    if keys.just_pressed(KeyCode::F3) {
        memory.debug = !memory.debug;
    }
}

fn redraw_bag(
    mut commands: Commands,
    sprites: Res<Sprites>,
    overlay: Res<Overlay>,
    mut bag_state: ResMut<BagState>,
    game: Option<Res<Game>>,
    clock: Option<Res<Clock>>,
    inv: Option<Res<Inventory>>,
    panels: Query<Entity, With<BagPanel>>,
) {
    if !overlay.is_changed() && !bag_state.dirty {
        return;
    }
    bag_state.dirty = false;
    for entity in &panels {
        commands.entity(entity).despawn();
    }
    if *overlay != Overlay::Bag {
        return;
    }
    let (Some(game), Some(clock), Some(inv)) = (game, clock, inv) else { return };

    commands
        .spawn((
            BagPanel,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            GlobalZIndex(6),
            DespawnOnExit(AppState::Playing),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(840.0),
                    min_height: Val::Px(520.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                },
                CozyPanel::Dark,
                UiBounce::new(0.04),
            ))
            .with_children(|card| {
                // Header com abas
                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.0),
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|header| {
                    header.spawn((
                        Text::new(match bag_state.tab {
                            BagTab::Inventory => "MOCHILA & EQUIPAMENTO",
                            BagTab::Status => "STATUS & ATRIBUTOS",
                            BagTab::Crafting => "BANCADA DE CRIAÇÃO",
                        }),
                        font(&sprites, 26.0),
                        TextColor(HINT),
                    ));

                    header.spawn(Node {
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|tabs| {
                        spawn_button(tabs, &sprites, "Mochila", 15.0, UiAction::BagTab(BagTab::Inventory));
                        spawn_button(tabs, &sprites, "Status", 15.0, UiAction::BagTab(BagTab::Status));
                        spawn_button(tabs, &sprites, "Crafting", 15.0, UiAction::BagTab(BagTab::Crafting));
                        spawn_button(tabs, &sprites, "×", 15.0, UiAction::CloseOverlay);
                    });
                });

                // Conteúdo da Aba Ativa
                match bag_state.tab {
                    BagTab::Inventory => {
                        card.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(16.0),
                            width: Val::Percent(100.0),
                            ..default()
                        })
                        .with_children(|body| {
                            // Painel de Equipamentos (Esquerda)
                            body.spawn((
                                Node {
                                    width: Val::Px(250.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(10.0),
                                    padding: UiRect::all(Val::Px(12.0)),
                                    ..default()
                                },
                                CozyPanel::Dialog,
                            ))
                            .with_children(|equip| {
                                equip.spawn((Text::new("EQUIPAMENTO"), font(&sprites, 18.0), TextColor(INK)));

                                // Slot de Máscara
                                equip.spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(2.0),
                                    ..default()
                                })
                                .with_children(|mask_box| {
                                    let mask_name = game.equipped_mask.map(|m| m.name()).unwrap_or("Sem Máscara (Descoberto)");
                                    let mask_color = game.equipped_mask.map(|m| m.color()).unwrap_or(INK);
                                    mask_box.spawn((Text::new("Máscara:"), font(&sprites, 14.0), TextColor(INK)));
                                    mask_box.spawn((Text::new(mask_name), font(&sprites, 16.0), TextColor(mask_color)));
                                    let bonus = game.equipped_mask.map(|m| m.bonus_text()).unwrap_or("Pressione [M] para alternar");
                                    mask_box.spawn((Text::new(bonus), font(&sprites, 12.0), TextColor(INK)));
                                    spawn_button(mask_box, &sprites, "Alternar Máscara [M]", 13.0, UiAction::CycleMask);
                                });

                                // Ferramenta em mãos
                                equip.spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(2.0),
                                    ..default()
                                })
                                .with_children(|tool_box| {
                                    let sel_name = inv.slots.get(inv.selected).and_then(|s| s.as_ref()).map(|s| s.item.name()).unwrap_or("Mãos Livres");
                                    tool_box.spawn((Text::new("Em Mãos:"), font(&sprites, 14.0), TextColor(INK)));
                                    tool_box.spawn((Text::new(sel_name), font(&sprites, 16.0), TextColor(INK)));
                                });

                                // Vestimenta
                                equip.spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(2.0),
                                    ..default()
                                })
                                .with_children(|outfit_box| {
                                    outfit_box.spawn((Text::new("Vestimenta:"), font(&sprites, 14.0), TextColor(INK)));
                                    outfit_box.spawn((Text::new("Traje de Fazendeiro"), font(&sprites, 15.0), TextColor(INK)));
                                });
                            });

                            // Painel da Mochila 3x10 (Direita)
                            body.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                width: Val::Px(520.0),
                                ..default()
                            })
                            .with_children(|grid_box| {
                                grid_box.spawn((Text::new("Barra Rápida (Teclas 1..0)"), font(&sprites, 16.0), TextColor(HINT)));
                                grid_box.spawn(Node { column_gap: Val::Px(2.0), ..default() }).with_children(|line| {
                                    for col in 0..HOTBAR {
                                        spawn_slot(line, &sprites, col);
                                    }
                                });

                                grid_box.spawn((Text::new("Mochila (Armazenamento)"), font(&sprites, 16.0), TextColor(HINT)));
                                for row in 1..SLOTS / HOTBAR {
                                    grid_box.spawn(Node { column_gap: Val::Px(2.0), ..default() }).with_children(|line| {
                                        for col in 0..HOTBAR {
                                            spawn_slot(line, &sprites, row * HOTBAR + col);
                                        }
                                    });
                                }

                                // Cartão de detalhes do item sob o mouse (preenchido por `bag_details`).
                                grid_box
                                    .spawn((
                                        Node {
                                            width: Val::Percent(100.0),
                                            min_height: Val::Px(92.0),
                                            column_gap: Val::Px(14.0),
                                            align_items: AlignItems::Center,
                                            padding: UiRect::all(Val::Px(10.0)),
                                            margin: UiRect::top(Val::Px(4.0)),
                                            ..default()
                                        },
                                        CozyPanel::Dialog,
                                    ))
                                    .with_children(|card| {
                                        card.spawn((
                                            DetailIcon,
                                            item_icon(&sprites, crate::inventory::Item::Hoe, 48.0),
                                            Visibility::Hidden,
                                        ));
                                        card.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(2.0), ..default() })
                                            .with_children(|text| {
                                                text.spawn((DetailLine::Title, Text::new(""), font(&sprites, 22.0), TextColor(INK)));
                                                text.spawn((DetailLine::Kind, Text::new(""), font(&sprites, 16.0), TextColor(INK)));
                                                text.spawn((DetailLine::Effect, Text::new(""), font(&sprites, 16.0), TextColor(INK)));
                                            });
                                    });

                                grid_box
                                    .spawn(Node {
                                        width: Val::Percent(100.0),
                                        justify_content: JustifyContent::SpaceBetween,
                                        align_items: AlignItems::Center,
                                        column_gap: Val::Px(10.0),
                                        ..default()
                                    })
                                    .with_children(|row| {
                                        row.spawn((
                                            Text::new("Clique: pegar e soltar (pilhas iguais se juntam)\nShift+clique: barra <-> mochila · 1..0: mandar para a barra"),
                                            font(&sprites, 15.0),
                                            TextColor(MUTED),
                                        ));
                                        spawn_button(row, &sprites, "Organizar [R]", 15.0, UiAction::SortBag);
                                    });
                            });
                        });
                    }

                    BagTab::Status => {
                        card.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(12.0),
                                padding: UiRect::all(Val::Px(16.0)),
                                ..default()
                            },
                            CozyPanel::Dialog,
                        ))
                        .with_children(|panel| {
                            panel.spawn((
                                Text::new(format!("Vila Serena • Sítio Agroecológico • Dia {} de {} (Ano {})", clock.day, clock.season().name(), clock.year())),
                                font(&sprites, 22.0),
                                TextColor(INK),
                            ));

                            // Barra de Energia
                            let energy_pct = (game.energy / MAX_ENERGY * 100.0).clamp(0.0, 100.0);
                            panel.spawn((Text::new(format!("Energia (Stamina): {:.0} / {:.0} ({:.0}%)", game.energy, MAX_ENERGY, energy_pct)), font(&sprites, 17.0), TextColor(INK)));
                            panel.spawn((
                                Node {
                                    width: Val::Px(500.0),
                                    height: Val::Px(16.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                            )).with_children(|bar| {
                                bar.spawn((
                                    Node {
                                        width: Val::Percent(energy_pct),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.3, 0.8, 0.25)),
                                ));
                            });

                            // Barra de Maldição
                            let curse_pct = game.curse.clamp(0.0, 100.0);
                            let curse_status = if curse_pct < 25.0 {
                                "Alma Serena (Nenhum perigo)"
                            } else if curse_pct < 50.0 {
                                "Névoa Leve (Atenção às sombras)"
                            } else if curse_pct < 75.0 {
                                "Sombras Inquietas (Risco iminente de Inquisidor!)"
                            } else {
                                "Vila Amaldiçoada (Crítico: O Santo Ofício cobra dízimo!)"
                            };
                            panel.spawn((Text::new(format!("Maldição Acumulada: {:.0}% • {}", game.curse, curse_status)), font(&sprites, 17.0), TextColor(INK)));
                            panel.spawn((
                                Node {
                                    width: Val::Px(500.0),
                                    height: Val::Px(16.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                            )).with_children(|bar| {
                                bar.spawn((
                                    Node {
                                        width: Val::Percent(curse_pct),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.72, 0.22, 0.88)),
                                ));
                            });

                            // Economia
                            panel.spawn((
                                Text::new(format!("Ouro: {} moedas  •  Essência Noturna: {:.1}  •  Fornalhas: {} (+{:.1}/s à noite)", game.gold, game.essence, game.factories, game.essence_rate())),
                                font(&sprites, 17.0),
                                TextColor(INK),
                            ));

                            // Máscara
                            if let Some(mask) = game.equipped_mask {
                                panel.spawn((Text::new(format!("Máscara Equipada: {}", mask.name())), font(&sprites, 18.0), TextColor(mask.color())));
                                panel.spawn((Text::new(format!("Efeito Ativo: {}", mask.bonus_text())), font(&sprites, 16.0), TextColor(INK)));
                                panel.spawn((Text::new(mask.description()), font(&sprites, 14.0), TextColor(INK)));
                            } else {
                                panel.spawn((Text::new("Nenhuma máscara equipada. Pressione [M] para alternar entre as máscaras desbloqueadas."), font(&sprites, 15.0), TextColor(INK)));
                            }
                        });
                    }

                    BagTab::Crafting => {
                        // Filtros de Categoria
                        card.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(6.0),
                            margin: UiRect::bottom(Val::Px(6.0)),
                            ..default()
                        })
                        .with_children(|filter_row| {
                            for cat in crate::crafting::CraftCategory::ALL {
                                spawn_button(filter_row, &sprites, cat.label(), 14.0, UiAction::CraftCategoryFilter(cat));
                            }
                        });

                        // 2 Colunas: Lista e Detalhes
                        card.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(16.0),
                            width: Val::Percent(100.0),
                            ..default()
                        })
                        .with_children(|craft_body| {
                            // Lista de Receitas (Esquerda)
                            craft_body.spawn(Node {
                                width: Val::Px(280.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(4.0),
                                ..default()
                            })
                            .with_children(|list| {
                                let filtered = crate::crafting::CraftRecipe::ALL
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, r)| bag_state.craft_category == crate::crafting::CraftCategory::All || r.category == bag_state.craft_category)
                                    .collect::<Vec<_>>();

                                for (idx, recipe) in filtered {
                                    let can = recipe.can_craft(&inv, &game);
                                    let mark = if can { "•" } else { "×" };
                                    let label = format!("{} {}", mark, recipe.name);
                                    spawn_button(list, &sprites, label, 14.0, UiAction::SelectCraftRecipe(idx));
                                }
                            });

                            // Detalhes da Receita (Direita)
                            craft_body.spawn((
                                Node {
                                    width: Val::Px(480.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(8.0),
                                    padding: UiRect::all(Val::Px(16.0)),
                                    ..default()
                                },
                                CozyPanel::Dialog,
                            ))
                            .with_children(|details| {
                                if let Some(recipe) = crate::crafting::CraftRecipe::ALL.get(bag_state.selected_recipe) {
                                    details.spawn((Text::new(recipe.name), font(&sprites, 24.0), TextColor(INK)));
                                    details.spawn((Text::new(format!("Categoria: {}", recipe.category.label())), font(&sprites, 14.0), TextColor(INK)));

                                    details.spawn((Text::new("Ingredientes Necessários:"), font(&sprites, 16.0), TextColor(INK)));
                                    for &(item, req) in recipe.ingredients {
                                        let has = inv.count(item);
                                        let ok = has >= req;
                                        let mark = if ok { "•" } else { "×" };
                                        let col = if ok { Color::srgb(0.1, 0.5, 0.1) } else { Color::srgb(0.7, 0.1, 0.1) };
                                        details.spawn((Text::new(format!("• {}: {}/{} {}", item.name(), has, req, mark)), font(&sprites, 14.0), TextColor(col)));
                                    }
                                    if recipe.gold_cost > 0 {
                                        let ok = game.gold >= recipe.gold_cost;
                                        let mark = if ok { "•" } else { "×" };
                                        let col = if ok { Color::srgb(0.1, 0.5, 0.1) } else { Color::srgb(0.7, 0.1, 0.1) };
                                        details.spawn((Text::new(format!("• Ouro: {}/{} {}", game.gold, recipe.gold_cost, mark)), font(&sprites, 14.0), TextColor(col)));
                                    }
                                    if recipe.essence_cost > 0.0 {
                                        let ok = game.essence >= recipe.essence_cost;
                                        let mark = if ok { "•" } else { "×" };
                                        let col = if ok { Color::srgb(0.1, 0.5, 0.1) } else { Color::srgb(0.7, 0.1, 0.1) };
                                        details.spawn((Text::new(format!("• Essência: {:.1}/{:.1} {}", game.essence, recipe.essence_cost, mark)), font(&sprites, 14.0), TextColor(col)));
                                    }

                                    details.spawn((Text::new(recipe.description), font(&sprites, 13.0), TextColor(INK)));
                                    details.spawn((Text::new(format!("Bônus: {}", recipe.benefits)), font(&sprites, 13.0), TextColor(Color::srgb(0.1, 0.4, 0.6))));

                                    let can = recipe.can_craft(&inv, &game);
                                    let btn_txt = if can { "Fabricar Agora" } else { "Faltam Ingredientes" };
                                    spawn_button(details, &sprites, btn_txt, 16.0, UiAction::CraftRecipe(bag_state.selected_recipe));
                                }
                            });
                        });
                    }
                }
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn update_hud(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    game: Option<Res<Game>>,
    clock: Option<Res<Clock>>,
    phase: Option<Res<State<DayPhase>>>,
    inv: Option<Res<Inventory>>,
    (dialogue, overlay): (Res<Dialogue>, Res<Overlay>),
    mut memory: ResMut<HudMemory>,
    (diagnostics, map, player, entities): (Res<DiagnosticsStore>, Option<Res<Map>>, Option<Single<&Transform, With<Player>>>, Query<()>),
    mut slots: Query<(&HudSlot, &mut Visibility, &Children)>,
    mut texts: Query<(&mut Text, &mut TextColor), With<HudText>>,
    mut help_hidden: Local<bool>,
) {
    let (Some(game), Some(clock), Some(phase), Some(inv)) = (game, clock, phase, inv) else { return };
    let now = time.elapsed_secs();
    let busy = dialogue.is_open() || *overlay != Overlay::None;

    // O que mudou desde o último frame acorda o painel correspondente.
    let stats = (game.gold, game.energy as i32, game.health as i32, game.curse as i32, game.essence as i32, game.has_mask);
    if memory.last_stats.is_some_and(|s| s != stats) {
        memory.stats_until = now + STATS_SECONDS;
    }
    memory.last_stats = Some(stats);
    let log: Vec<String> = game.log.iter().cloned().collect();
    if log != memory.last_log {
        memory.log_until = now + LOG_SECONDS;
        memory.last_log = log.clone();
    }
    let selected = (inv.selected, inv.selected_item());
    if memory.last_selected.is_some_and(|s| s != selected) {
        memory.item_until = now + 1.4;
    }
    memory.last_selected = Some(selected);
    if keys.just_pressed(KeyCode::KeyH) {
        *help_hidden = !*help_hidden;
    }
    // O ouro "conta" até o valor novo.
    let gold = game.gold as f32;
    memory.shown_gold += (gold - memory.shown_gold) * (1.0 - (-10.0 * time.delta_secs()).exp());
    if (gold - memory.shown_gold).abs() < 0.5 {
        memory.shown_gold = gold;
    }

    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let label = match *phase.get() {
        DayPhase::Day => "Dia",
        DayPhase::Night => "Noite",
    };
    for (slot, mut visibility, children) in &mut slots {
        let Some(child) = children.iter().find(|c| texts.contains(*c)) else { continue };
        let Ok((mut text, mut color)) = texts.get_mut(child) else { continue };
        let (content, show) = match slot {
            HudSlot::Clock => (
                format!(
                    "{} · {} {} ({})\n{} · {} ouro",
                    clock.season().name(),
                    label,
                    clock.day_of_season(),
                    clock.milestone(),
                    clock.hour_label(),
                    memory.shown_gold.round()
                ),
                true,
            ),
            // Com pouca vida o painel fica à vista (é hora de comer ou correr para casa).
            HudSlot::Stats => (
                stats_text(&game),
                alt || *overlay == Overlay::Bag || now < memory.stats_until || game.health < MAX_HEALTH * 0.35,
            ),
            HudSlot::Hint => {
                // Ação só informativa fica mais apagada.
                let info = game.hint.as_ref().is_some_and(|h| !h.starts_with('['));
                color.0 = if info { MUTED } else { HINT };
                (game.hint.clone().unwrap_or_default(), !busy && game.hint.is_some())
            }
            HudSlot::ItemName => (
                inv.selected_stack()
                    .map(|s| {
                        let badge = s.quality.badge();
                        if badge.is_empty() { s.item.name().to_string() } else { format!("{} {badge}", s.item.name()) }
                    })
                    .unwrap_or_default(),
                !busy && game.hint.is_none() && now < memory.item_until && inv.selected_item().is_some(),
            ),
            HudSlot::Log => (log.join("\n"), !busy && now < memory.log_until && !log.is_empty()),
            HudSlot::Help => (text.0.clone(), !*help_hidden && now < HELP_SECONDS || keys.pressed(KeyCode::KeyH)),
            HudSlot::Debug => {
                let fps = diagnostics
                    .get(&FrameTimeDiagnosticsPlugin::FPS)
                    .and_then(|d| d.smoothed())
                    .unwrap_or(0.0);
                let tile = player.as_ref().map(|p| world_to_tile(p.translation.truncate()));
                let where_ = map.as_ref().map_or("-".to_string(), |m| m.id.clone());
                (format!("FPS {fps:.0} · entidades {} · mapa {where_} · tile {tile:?}", entities.iter().count()), memory.debug)
            }
        };
        visibility.set_if_neq(if show { Visibility::Inherited } else { Visibility::Hidden });
        if show && text.0 != content {
            text.0 = content;
        }
    }
}

fn bar(value: f32, max: f32) -> String {
    let filled = ((value / max) * 10.0).round().clamp(0.0, 10.0) as usize;
    format!("{}{}", "#".repeat(filled), "-".repeat(10 - filled))
}

fn stats_text(game: &Game) -> String {
    let mask = if game.has_mask { "Gárgula" } else { "nenhuma" };
    let mut s = format!(
        "Meta {} / {GOAL} ouro\n\
         Vida     [{}] {:.0}\n\
         Energia  [{}] {:.0}\n\
         Maldição [{}] {:.0}%\n\
         Essência {:.0} · Fornalhas {} ({:.1}/s à noite)\n\
         Máscara: {mask}",
        game.gold,
        bar(game.health, MAX_HEALTH),
        game.health,
        bar(game.energy, MAX_ENERGY),
        game.energy,
        bar(game.curse, 100.0),
        game.curse,
        game.essence,
        game.factories,
        game.essence_rate(),
    );
    if game.inquisitor {
        s.push_str("\n! Inquisidor na vila: preços pela metade");
    }
    s
}

#[derive(Component)]
struct DawnCardUi;

const DAWN_CARD_SECONDS: f32 = 3.5;

/// "Dia N" em pergaminho no começo do dia, com o resumo de ontem; some sozinho.
fn dawn_card(
    mut commands: Commands,
    time: Res<Time<Real>>,
    sprites: Res<Sprites>,
    card: Option<ResMut<DawnCard>>,
    ui: Query<Entity, With<DawnCardUi>>,
) {
    let Some(mut card) = card else { return };
    if card.t == 0.0 {
        commands
            .spawn((
                DawnCardUi,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(22.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
                GlobalZIndex(4),
                DespawnOnExit(AppState::Playing),
            ))
            .with_children(|row| {
                row.spawn((
                    Node { padding: UiRect::axes(Val::Px(48.0), Val::Px(28.0)), ..default() },
                    sliced(sprites.ui.panel.clone(), 8.0),
                    UiBounce::new(0.08),
                ))
                .with_child((
                    Text::new(card.text.clone()),
                    font(&sprites, 34.0),
                    TextColor(INK),
                    TextLayout::justify(Justify::Center),
                ));
            });
    }
    card.t += time.delta_secs();
    if card.t > DAWN_CARD_SECONDS {
        for entity in &ui {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<DawnCard>();
    }
}

#[derive(Component)]
struct WorldBalloon;

#[derive(Component)]
struct WorldBalloonText;

fn update_world_balloon(
    target_qol: Res<crate::economy::TargetQol>,
    overlay: Res<Overlay>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut balloon: Query<(&mut Node, &mut Visibility), With<WorldBalloon>>,
    mut balloon_text: Query<&mut Text, With<WorldBalloonText>>,
) {
    let Some(text) = &target_qol.text else {
        for (_, mut vis) in &mut balloon {
            vis.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    if *overlay != Overlay::None {
        for (_, mut vis) in &mut balloon {
            vis.set_if_neq(Visibility::Hidden);
        }
        return;
    }
    let (camera, transform) = *camera;
    if let Ok(screen) = camera.world_to_viewport(transform, target_qol.world_pos.extend(0.0)) {
        let new_left = Val::Px((screen.x - 70.0).round());
        let new_top = Val::Px((screen.y - 28.0).round());
        for (mut node, mut vis) in &mut balloon {
            if node.left != new_left {
                node.left = new_left;
            }
            if node.top != new_top {
                node.top = new_top;
            }
            vis.set_if_neq(Visibility::Inherited);
        }
        for mut t in &mut balloon_text {
            if t.0 != *text {
                t.0 = text.clone();
            }
        }
    } else {
        for (_, mut vis) in &mut balloon {
            vis.set_if_neq(Visibility::Hidden);
        }
    }
}
