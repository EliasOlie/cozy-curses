//! Lojas da vila: o Armazém, a Padaria, a Floricultura, o Fogão e a Banca da Fazenda.

use std::collections::HashMap;

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::cycle::{Clock, DayPhase, Season, MILESTONE};
use crate::economy::{Game, ESSENCE_PRICE};
use crate::hud::Held;
use crate::inventory::{HOTBAR, Inventory, Item, Quality};
use crate::menu::{GameClicked, UiAction};
use crate::sprites::Sprites;
use crate::ui::{CozyPanel, HINT, MUTED, TEXT, font, item_icon, spawn_button, text_shadow};
use crate::{AppState, Overlay};

pub fn plugin(app: &mut App) {
    app.init_resource::<ShopMessage>()
        .init_resource::<ShopTab>()
        .init_resource::<Quantities>()
        .add_systems(
            Update,
            (game_actions, shop_mouse_scroll, redraw)
                .chain()
                .run_if(in_state(AppState::Playing)),
        );
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Shop {
    Store,
    Bakery,
    Florist,
    Kitchen,
    Stand,
}

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub enum ShopTab {
    #[default]
    Buy,
    Sell,
}

#[derive(Component)]
pub struct ShopScrollList;

/// Almoço: come na hora, recupera energia e leva o resto do período (um "marco").
const LUNCH_PRICE: i32 = 12;
const LUNCH_ENERGY: f32 = 40.0;
/// Pão de graça uma vez por dia: a generosidade do Padeiro.
const FREE_BREAD_CURSE: f32 = -5.0;

pub const DISHES: [Item; 8] = [
    Item::GardenSalad,
    Item::CountryStew,
    Item::StrawberryPie,
    Item::HerbOmelet,
    Item::PumpkinSoup,
    Item::RoastChicken,
    Item::HeartyRoast,
    Item::BlueberryJam,
];

pub fn dish_recipe(dish: Item) -> (&'static str, &'static [(Item, u32)]) {
    match dish {
        Item::GardenSalad => ("1x Tomate + 1x Cenoura", &[(Item::Tomato, 1), (Item::Carrot, 1)]),
        Item::CountryStew => ("1x Carne Bovina + 1x Batata + 1x Cenoura", &[(Item::Beef, 1), (Item::Potato, 1), (Item::Carrot, 1)]),
        Item::StrawberryPie => ("2x Morango + 1x Leite de Vaca + 1x Pão", &[(Item::Strawberry, 2), (Item::CowMilk, 1), (Item::Bread, 1)]),
        Item::HerbOmelet => ("2x Ovo + 1x Alho", &[(Item::Egg, 2), (Item::Garlic, 1)]),
        Item::PumpkinSoup => ("1x Abóbora + 1x Leite de Vaca", &[(Item::Pumpkin, 1), (Item::CowMilk, 1)]),
        Item::RoastChicken => ("1x Frango + 1x Batata + 1x Alho", &[(Item::ChickenMeat, 1), (Item::Potato, 1), (Item::Garlic, 1)]),
        Item::HeartyRoast => ("1x Cordeiro + 1x Batata + 1x Alho", &[(Item::Mutton, 1), (Item::Potato, 1), (Item::Garlic, 1)]),
        Item::BlueberryJam => ("2x Mirtilo + 1x Morango", &[(Item::Blueberry, 2), (Item::Strawberry, 1)]),
        _ => ("", &[]),
    }
}

impl Shop {
    pub fn verb(self) -> &'static str {
        match self {
            Shop::Store => "Ver animais e mercadorias do Seu Bartô",
            Shop::Bakery => "Ver os pães do Seu Tobias",
            Shop::Florist => "Ver sementes e mudas da Florinda",
            Shop::Kitchen => "Cozinhar no fogão a lenha",
            Shop::Stand => "Atender na Banca da Fazenda",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Shop::Store => "Armazém & Pecuária do Seu Bartô",
            Shop::Bakery => "Padaria do Seu Tobias",
            Shop::Florist => "Floricultura da Florinda",
            Shop::Kitchen => "Fogão a Lenha · Culinária",
            Shop::Stand => "Banca da Fazenda · Venda Direta",
        }
    }

    fn greeting(self) -> &'static str {
        match self {
            Shop::Store => "\"Animais fortes, cortes de açougue e bons negócios!\"",
            Shop::Bakery => "\"Pão quentinho! Ninguém passa fome na Vila Serena.\"",
            Shop::Florist => "\"Sementes e mudas frescas para a sua estação!\"",
            Shop::Kitchen => "\"O aroma do campo na panela. Misture carnes e vegetais.\"",
            Shop::Stand => "\"Produtos frescos da fazenda para a vila! O que você colheu hoje?\"",
        }
    }

    pub fn stock(self, season: Season) -> Vec<(Item, i32)> {
        match self {
            Shop::Store => vec![
                // Animais (certidões para a fazenda)
                (Item::ChickenDeed, 50),
                (Item::RoosterDeed, 40),
                (Item::CowDeed, 200),
                (Item::BullDeed, 180),
                (Item::SheepDeed, 120),
                (Item::RamDeed, 110),
                (Item::GoatDeed, 100),
                (Item::BuckDeed, 90),
                (Item::DoeRabbitDeed, 60),
                (Item::BuckRabbitDeed, 50),
                (Item::MareDeed, 350),
                (Item::StallionDeed, 350),
                // Açougue (carnes)
                (Item::Beef, 35),
                (Item::ChickenMeat, 20),
                (Item::Mutton, 30),
                (Item::RabbitMeat, 22),
                // "Era do Xerife. Ele nunca usou." Controle de pragas noturnas.
                (Item::Sword, 60),
            ],
            Shop::Bakery => vec![(Item::Bread, 5), (Item::Cake, 15)],
            Shop::Florist => match season {
                Season::Spring => vec![
                    (Item::CarrotSeeds, 2),
                    (Item::StrawberrySeeds, 6),
                    (Item::CauliflowerSeeds, 8),
                    (Item::PotatoSeeds, 4),
                ],
                Season::Summer => vec![
                    (Item::CornSeeds, 5),
                    (Item::TomatoSeeds, 5),
                    (Item::BlueberrySeeds, 7),
                    (Item::MelonSeeds, 12),
                ],
                Season::Autumn => vec![
                    (Item::PumpkinSeeds, 12),
                    (Item::EggplantSeeds, 6),
                    (Item::GrapeSeeds, 7),
                    (Item::SweetPotatoSeeds, 5),
                ],
                Season::Winter => vec![
                    (Item::WinterRootSeeds, 5),
                    (Item::WinterCabbageSeeds, 7),
                    (Item::GarlicSeeds, 8),
                ],
            },
            Shop::Kitchen | Shop::Stand => vec![],
        }
    }
}

#[derive(Resource, Default)]
struct ShopMessage(String);

/// Quantidade escolhida em cada linha: (é compra?, item, qualidade) -> quantidade.
#[derive(Resource, Default)]
struct Quantities(HashMap<(bool, Item, Quality), u32>);

impl Quantities {
    fn get(&self, buy: bool, item: Item) -> u32 {
        self.get_quality(buy, item, Quality::Normal)
    }

    fn get_quality(&self, buy: bool, item: Item, quality: Quality) -> u32 {
        self.0.get(&(buy, item, quality)).copied().unwrap_or(1)
    }

    fn set(&mut self, buy: bool, item: Item, n: u32) {
        self.set_quality(buy, item, Quality::Normal, n);
    }

    fn set_quality(&mut self, buy: bool, item: Item, quality: Quality, n: u32) {
        self.0.insert((buy, item, quality), n);
    }
}

fn max_buy_qty(shop: Shop, item: Item, season: Season, game: &Game) -> u32 {
    let price = shop.stock(season).into_iter().find(|(i, _)| *i == item).map_or(1, |(_, p)| p).max(1);
    (game.gold / price).clamp(1, 99) as u32
}

fn shop_mouse_scroll(
    mut wheel: MessageReader<MouseWheel>,
    mut query: Query<&mut ScrollPosition, With<ShopScrollList>>,
) {
    let mut dy = 0.0;
    for ev in wheel.read() {
        dy -= ev.y * 36.0;
    }
    if dy != 0.0 {
        for mut scroll in &mut query {
            scroll.y = (scroll.y + dy).max(0.0);
        }
    }
}

#[derive(Component)]
struct ShopPanel;

/// Ações dos botões do jogo (lojas, mochila).
#[allow(clippy::too_many_arguments)]
fn game_actions(
    mut clicked: ResMut<GameClicked>,
    mut overlay: ResMut<Overlay>,
    mut held: ResMut<Held>,
    mut inv: ResMut<Inventory>,
    mut game: ResMut<Game>,
    mut clock: ResMut<Clock>,
    _phase: Res<State<DayPhase>>,
    mut message: ResMut<ShopMessage>,
    mut qty: ResMut<Quantities>,
    mut tab: ResMut<ShopTab>,
    mut scroll_query: Query<&mut ScrollPosition, With<ShopScrollList>>,
    mut bag_state: ResMut<crate::hud::BagState>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    for action in std::mem::take(&mut clicked.0) {
        let shop = match *overlay {
            Overlay::Shop(shop) => Some(shop),
            _ => None,
        };
        match action {
            UiAction::BagTab(new_tab) => {
                bag_state.tab = new_tab;
                bag_state.dirty = true;
            }
            UiAction::EquipMask(mask) => {
                game.equip_mask(mask);
                bag_state.dirty = true;
            }
            UiAction::CycleMask => {
                let next = game.cycle_mask();
                if let Some(m) = next {
                    game.say(format!("Equipou {}: {}", m.name(), m.bonus_text()));
                } else {
                    game.say("Removeu a máscara.");
                }
                bag_state.dirty = true;
            }
            UiAction::SelectCraftRecipe(idx) => {
                bag_state.selected_recipe = idx;
                bag_state.dirty = true;
            }
            UiAction::CraftRecipe(idx) => {
                if let Some(recipe) = crate::crafting::CraftRecipe::ALL.get(idx) {
                    if recipe.craft(&mut inv, &mut game) {
                        bag_state.dirty = true;
                    }
                }
            }
            UiAction::CraftCategoryFilter(cat) => {
                bag_state.craft_category = cat;
                bag_state.selected_recipe = 0;
                bag_state.dirty = true;
            }
            UiAction::SortBag => {
                inv.sort_storage();
                held.0 = None;
                game.say("Mochila arrumada.");
            }
            // Shift+clique manda direto para o outro lado (barra <-> mochila).
            UiAction::Slot(i) if *overlay == Overlay::Bag && shift => {
                held.0 = None;
                if !inv.quick_move(i) && inv.slots.get(i).is_some_and(|s| s.is_some()) {
                    game.say("Não há espaço do outro lado.");
                }
            }
            UiAction::Slot(i) if *overlay == Overlay::Bag => match held.0.take() {
                Some(first) => inv.merge_or_swap(first, i),
                // Espaço vazio não vai "para a mão".
                None => held.0 = inv.slots.get(i).is_some_and(|s| s.is_some()).then_some(i),
            },
            UiAction::Slot(i) if i < HOTBAR => inv.selected = i,
            UiAction::ShopTab(to_sell) => {
                *tab = if to_sell { ShopTab::Sell } else { ShopTab::Buy };
            }
            UiAction::ScrollShop(delta) => {
                for mut scroll in &mut scroll_query {
                    scroll.y = (scroll.y + delta).max(0.0);
                }
            }
            UiAction::Buy(item, n) => {
                let season = clock.season();
                let Some(price) = shop.and_then(|s| s.stock(season).into_iter().find(|(it, _)| *it == item)).map(|(_, p)| p) else {
                    continue;
                };
                let cost = price * n as i32;
                message.0 = if game.gold < cost {
                    format!("Faltam {} ouro.", cost - game.gold)
                } else if inv.add(item, n) > 0 {
                    "A mochila está cheia.".into()
                } else {
                    game.gold -= cost;
                    qty.set(true, item, 1);
                    format!("Comprou {n}x {} (-{cost} ouro).", item.name())
                };
            }
            UiAction::Sell(item, n) => {
                let Some(price) = item.sell_price() else { continue };
                let n = n.min(inv.count(item));
                if n == 0 || !inv.remove(item, n) {
                    continue;
                }
                let price = if item == Item::Carrot { game.carrot_price() } else { price };
                let total = price * n as i32;
                game.gold += total;
                if item == Item::Carrot {
                    game.add_curse(-(n as f32));
                }
                qty.set(false, item, 1);
                message.0 = format!("Vendeu {n}x {} (+{total} ouro).", item.name());
            }
            UiAction::SellQuality(item, quality, n) => {
                let Some(base_price) = item.sell_price() else { continue };
                let available = inv.count_quality(item, quality);
                let take = n.min(available);
                if take == 0 || !inv.remove_quality(item, quality, take) {
                    continue;
                }
                let unit_price = if item == Item::Carrot {
                    let p = game.carrot_price();
                    (p as f32 * quality.multiplier()).round() as i32
                } else {
                    (base_price as f32 * quality.multiplier()).round() as i32
                };
                let total = unit_price * take as i32;
                game.gold += total;
                if item == Item::Carrot {
                    game.add_curse(-(take as f32));
                }
                qty.set_quality(false, item, quality, 1);
                let badge = quality.badge();
                message.0 = format!("Vendeu {take}x {} {badge} (+{total} ouro).", item.name());
            }
            UiAction::Cook(dish) => {
                let (_, ingredients) = dish_recipe(dish);
                let mut can_cook = true;
                for &(ing, need) in ingredients {
                    if inv.count(ing) < need {
                        can_cook = false;
                        break;
                    }
                }
                if !can_cook {
                    message.0 = format!("Faltam ingredientes para preparar {}.", dish.name());
                } else {
                    let seed = (dish as u32 * 37 + inv.selected as u32 * 19 + game.gold as u32 * 7) % 100;
                    let quality = Quality::roll(seed);
                    if inv.add_with_quality(dish, 1, quality) > 0 {
                        message.0 = "A mochila está cheia!".into();
                    } else {
                        for &(ing, need) in ingredients {
                            inv.remove(ing, need);
                        }
                        let energy = dish.energy().unwrap_or(0.0) * quality.multiplier();
                        let badge = quality.badge();
                        message.0 = format!("Você preparou {} {}! (+{:.0} de energia)", dish.name(), badge, energy);
                        if quality == Quality::Supernatural {
                            game.say(format!("Incrível! O prato {} foi infundido com modificador SOBRENATURAL!", dish.name()));
                        } else if quality != Quality::Normal {
                            game.say(format!("Você preparou {} de qualidade {}! {}", dish.name(), quality.name(), badge));
                        }
                    }
                }
            }
            UiAction::ExportEssence => {
                let n = game.essence.floor();
                if n >= 1.0 {
                    game.essence -= n;
                    let earned = n as i32 * ESSENCE_PRICE;
                    game.gold += earned;
                    game.add_curse(n * 0.35);
                    message.0 = format!("Exportou {n} essências noturnas (+{earned} ouro, +maldição)!");
                    game.say(format!("Banca da Fazenda: {n} essências enviadas para além da névoa."));
                } else {
                    message.0 = "Sem essência suficiente para exportar (mínimo 1.0).".into();
                }
            }
            UiAction::FreeBread => {
                if !game.bread_today && inv.add(Item::Bread, 1) == 0 {
                    game.bread_today = true;
                    game.add_curse(FREE_BREAD_CURSE);
                    message.0 = "\"Por conta da casa, querido!\" (+1 pão, -5 maldição)".into();
                }
            }
            UiAction::Lunch => {
                if game.gold < LUNCH_PRICE {
                    message.0 = "O almoço custa 12 ouro.".into();
                } else if clock.minutes >= 2.0 * MILESTONE {
                    message.0 = "\"A cozinha fechou! Almoço só até as 14h.\"".into();
                } else {
                    game.gold -= LUNCH_PRICE;
                    game.rest(LUNCH_ENERGY);
                    let from = clock.milestone();
                    clock.skip_to_next_milestone();
                    game.say(format!("Almoço demorado na padaria: a {} passou. (+40 energia)", from.to_lowercase()));
                    *overlay = Overlay::None;
                }
            }
            UiAction::Qty(item, buy, delta) => {
                let Some(shop) = shop else { continue };
                let max = if buy {
                    max_buy_qty(shop, item, clock.season(), &game)
                } else {
                    inv.count(item).max(1)
                };
                let n = (qty.get(buy, item) as i32 + delta).clamp(1, max as i32) as u32;
                qty.set(buy, item, n);
            }
            UiAction::QtyMax(item, buy) => {
                let Some(shop) = shop else { continue };
                let max = if buy {
                    max_buy_qty(shop, item, clock.season(), &game)
                } else {
                    inv.count(item).max(1)
                };
                qty.set(buy, item, max);
            }
            UiAction::QtyQuality(item, quality, buy, delta) => {
                let Some(shop) = shop else { continue };
                let max = if buy {
                    max_buy_qty(shop, item, clock.season(), &game)
                } else {
                    inv.count_quality(item, quality).max(1)
                };
                let cur = qty.get_quality(buy, item, quality);
                let n = (cur as i32 + delta).clamp(1, max as i32) as u32;
                qty.set_quality(buy, item, quality, n);
            }
            UiAction::QtyQualityMax(item, quality, buy) => {
                let Some(shop) = shop else { continue };
                let max = if buy {
                    max_buy_qty(shop, item, clock.season(), &game)
                } else {
                    inv.count_quality(item, quality).max(1)
                };
                qty.set_quality(buy, item, quality, max);
            }
            UiAction::CloseOverlay => {
                *overlay = Overlay::None;
                held.0 = None;
            }
            _ => {}
        }
    }
}

fn redraw(
    mut commands: Commands,
    sprites: Res<Sprites>,
    overlay: Res<Overlay>,
    inv: Res<Inventory>,
    game: Res<Game>,
    clock: Res<Clock>,
    phase: Res<State<DayPhase>>,
    mut message: ResMut<ShopMessage>,
    qty: Res<Quantities>,
    tab: Res<ShopTab>,
    panels: Query<Entity, With<ShopPanel>>,
) {
    if !(overlay.is_changed()
        || inv.is_changed()
        || game.is_changed()
        || qty.is_changed()
        || clock.is_changed()
        || tab.is_changed()
        || phase.is_changed())
    {
        return;
    }
    let Overlay::Shop(shop) = *overlay else {
        for entity in &panels {
            commands.entity(entity).despawn();
        }
        message.0.clear();
        return;
    };
    for entity in &panels {
        commands.entity(entity).despawn();
    }

    let is_night = *phase.get() == DayPhase::Night;

    commands
        .spawn((
            ShopPanel,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(6),
            DespawnOnExit(AppState::Playing),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(820.0),
                    max_height: Val::Vh(92.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                },
                CozyPanel::Dark,
            ))
            .with_children(|card| {
                // Cabeçalho da loja
                card.spawn((Text::new(shop.title()), font(&sprites, 34.0), TextColor(HINT), text_shadow()));
                card.spawn((
                    Text::new(format!("{}   Ouro disponível: {}", shop.greeting(), game.gold)),
                    font(&sprites, 20.0),
                    TextColor(MUTED),
                    text_shadow(),
                ));

                // Barra de abas e controle de navegação
                if shop == Shop::Store {
                    card.spawn(Node {
                        column_gap: Val::Px(10.0),
                        align_items: AlignItems::Center,
                        margin: UiRect::vertical(Val::Px(4.0)),
                        ..default()
                    })
                    .with_children(|bar| {
                        let buy_label = if *tab == ShopTab::Buy { "▶ [ Comprar Animais & Carnes ]" } else { "Comprar Animais & Carnes" };
                        let sell_label = if *tab == ShopTab::Sell { "▶ [ Vender da Mochila ]" } else { "Vender da Mochila" };
                        spawn_button(bar, &sprites, buy_label, 20.0, UiAction::ShopTab(false));
                        spawn_button(bar, &sprites, sell_label, 20.0, UiAction::ShopTab(true));
                        bar.spawn(Node { flex_grow: 1.0, ..default() });
                        spawn_button(bar, &sprites, "▲ Subir", 18.0, UiAction::ScrollShop(-140.0));
                        spawn_button(bar, &sprites, "▼ Descer", 18.0, UiAction::ScrollShop(140.0));
                    });
                } else if shop == Shop::Stand {
                    card.spawn(Node {
                        column_gap: Val::Px(10.0),
                        align_items: AlignItems::Center,
                        margin: UiRect::vertical(Val::Px(4.0)),
                        ..default()
                    })
                    .with_children(|bar| {
                        bar.spawn((
                            Text::new("Venda suas colheitas, carnes e pratos com bônus de qualidade!"),
                            font(&sprites, 19.0),
                            TextColor(TEXT),
                            text_shadow(),
                        ));
                        bar.spawn(Node { flex_grow: 1.0, ..default() });
                        spawn_button(bar, &sprites, "▲ Subir", 18.0, UiAction::ScrollShop(-140.0));
                        spawn_button(bar, &sprites, "▼ Descer", 18.0, UiAction::ScrollShop(140.0));
                    });
                }

                // Lista rolável principal
                card.spawn((
                    ShopScrollList,
                    ScrollPosition::default(),
                    Node {
                        max_height: Val::Vh(58.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        padding: UiRect::right(Val::Px(8.0)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .with_children(|list| {
                    if shop == Shop::Kitchen {
                        section(list, &sprites, "Receitas Culinárias do Fogão");
                        list.spawn((
                            Text::new("• Cozinhar pratos tem chance de infundi-los com qualidade SOBRENATURAL (3x energia e preço)!"),
                            font(&sprites, 18.0),
                            TextColor(Color::srgb(0.85, 0.45, 1.0)),
                            text_shadow(),
                        ));
                        for dish in DISHES {
                            let (desc, ingredients) = dish_recipe(dish);
                            let mut has_all = true;
                            for &(ing, need) in ingredients {
                                if inv.count(ing) < need {
                                    has_all = false;
                                    break;
                                }
                            }
                            let energy = dish.energy().unwrap_or(0.0);
                            let status_label = format!("{desc} (+{energy:.0} de energia)");
                            row(list, &sprites, dish, &status_label, |r| {
                                let btn_label = if has_all { "Cozinhar" } else { "Falta item" };
                                spawn_button(r, &sprites, btn_label, 20.0, UiAction::Cook(dish));
                            });
                        }
                    } else if shop == Shop::Bakery {
                        section(list, &sprites, "Pães e Guloseimas");
                        for (item, price) in shop.stock(clock.season()) {
                            let n = qty.get(true, item);
                            row(list, &sprites, item, &format!("{price} ouro"), |r| {
                                stepper(r, &sprites, item, true, n);
                                spawn_button(r, &sprites, format!("Comprar ({})", price * n as i32), 20.0, UiAction::Buy(item, n));
                            });
                        }
                        if !game.bread_today {
                            list.spawn(Node { margin: UiRect::top(Val::Px(6.0)), ..default() }).with_children(|r| {
                                spawn_button(r, &sprites, "Pegar o pão do dia (grátis)", 22.0, UiAction::FreeBread);
                            });
                        }
                        list.spawn(Node { column_gap: Val::Px(8.0), align_items: AlignItems::Center, margin: UiRect::top(Val::Px(6.0)), ..default() })
                            .with_children(|r| {
                                spawn_button(r, &sprites, "Almoçar (12 ouro)", 22.0, UiAction::Lunch);
                                r.spawn((
                                    Text::new("+40 energia · leva o resto do período (marco)"),
                                    font(&sprites, 18.0),
                                    TextColor(MUTED),
                                    text_shadow(),
                                ));
                            });
                    } else if shop == Shop::Florist {
                        section(list, &sprites, format!("Sementes e Mudas de {}", clock.season().name()));
                        for (item, price) in shop.stock(clock.season()) {
                            let n = qty.get(true, item);
                            row(list, &sprites, item, &format!("{price} ouro cada"), |r| {
                                stepper(r, &sprites, item, true, n);
                                spawn_button(r, &sprites, format!("Comprar ({})", price * n as i32), 20.0, UiAction::Buy(item, n));
                            });
                        }
                    } else if shop == Shop::Store && *tab == ShopTab::Buy {
                        section(list, &sprites, "Comprar Animais & Açougue");
                        for (item, price) in shop.stock(clock.season()) {
                            let n = qty.get(true, item);
                            row(list, &sprites, item, &format!("{price} ouro"), |r| {
                                stepper(r, &sprites, item, true, n);
                                spawn_button(r, &sprites, format!("Comprar ({})", price * n as i32), 20.0, UiAction::Buy(item, n));
                            });
                        }
                    } else if (shop == Shop::Store && *tab == ShopTab::Sell) || shop == Shop::Stand {
                        // Venda de itens da mochila com Qualidade e Multiplicadores
                        section(list, &sprites, "Vender Produção da Fazenda");

                        if shop == Shop::Stand && is_night && game.has_mask {
                            list.spawn(Node {
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(12.0),
                                margin: UiRect::vertical(Val::Px(6.0)),
                                ..default()
                            })
                            .with_children(|r| {
                                r.spawn((
                                    Text::new(format!("Exportação Noturna: {:.1} essências disponíveis", game.essence)),
                                    font(&sprites, 20.0),
                                    TextColor(Color::srgb(0.85, 0.55, 1.0)),
                                    text_shadow(),
                                ));
                                if game.essence >= 1.0 {
                                    let n = game.essence.floor() as i32;
                                    spawn_button(r, &sprites, format!("Exportar ({}+ ouro)", n * ESSENCE_PRICE), 20.0, UiAction::ExportEssence);
                                }
                            });
                        }

                        let mut any = false;
                        for (item, quality) in inv.stacks_present() {
                            let Some(base_price) = item.sell_price() else { continue };
                            let n = inv.count_quality(item, quality);
                            if n == 0 {
                                continue;
                            }
                            any = true;
                            let unit_price = if item == Item::Carrot {
                                let p = game.carrot_price();
                                (p as f32 * quality.multiplier()).round() as i32
                            } else {
                                (base_price as f32 * quality.multiplier()).round() as i32
                            };
                            let k = qty.get_quality(false, item, quality).min(n);
                            let total = unit_price * k as i32;
                            let badge = quality.badge();
                            let stars = quality.star_symbol();
                            let label_badge = if !badge.is_empty() {
                                format!("{badge} {stars} ({})", quality.name())
                            } else {
                                "Normal".into()
                            };
                            let detail = format!("{label_badge} · tem {n} · {unit_price} ouro cada");
                            row_quality(list, &sprites, item, quality, &detail, |r| {
                                stepper_quality(r, &sprites, item, quality, false, k, n);
                                spawn_button(r, &sprites, format!("Vender (+{total})"), 20.0, UiAction::SellQuality(item, quality, k));
                            });
                        }
                        if !any {
                            list.spawn((
                                Text::new("Nenhuma colheita, carne ou prato para vender na mochila."),
                                font(&sprites, 20.0),
                                TextColor(MUTED),
                                text_shadow(),
                            ));
                        }
                    }
                });

                // Mensagem de feedback
                if !message.0.is_empty() {
                    card.spawn((Text::new(message.0.clone()), font(&sprites, 22.0), TextColor(HINT), text_shadow()));
                }

                // Botão de fechar
                card.spawn(Node { justify_content: JustifyContent::FlexEnd, margin: UiRect::top(Val::Px(6.0)), ..default() })
                    .with_children(|r| {
                        spawn_button(r, &sprites, "Fechar [Esc]", 24.0, UiAction::CloseOverlay);
                    });
            });
        });
}

/// [-] quantidade [+] [Máx] para compra genérica
fn stepper(r: &mut ChildSpawnerCommands, sprites: &Sprites, item: Item, buy: bool, n: u32) {
    spawn_button(r, sprites, "-", 20.0, UiAction::Qty(item, buy, -1));
    r.spawn((
        Text::new(n.to_string()),
        font(sprites, 24.0),
        TextColor(HINT),
        text_shadow(),
        TextLayout::justify(Justify::Center),
        Node { width: Val::Px(36.0), ..default() },
    ));
    spawn_button(r, sprites, "+", 20.0, UiAction::Qty(item, buy, 1));
    spawn_button(r, sprites, "Máx", 18.0, UiAction::QtyMax(item, buy));
}

/// [-] quantidade [+] [Máx] com Qualidade específica para venda
fn stepper_quality(
    r: &mut ChildSpawnerCommands,
    sprites: &Sprites,
    item: Item,
    quality: Quality,
    buy: bool,
    n: u32,
    _max: u32,
) {
    spawn_button(r, sprites, "-", 20.0, UiAction::QtyQuality(item, quality, buy, -1));
    r.spawn((
        Text::new(n.to_string()),
        font(sprites, 24.0),
        TextColor(HINT),
        text_shadow(),
        TextLayout::justify(Justify::Center),
        Node { width: Val::Px(36.0), ..default() },
    ));
    spawn_button(r, sprites, "+", 20.0, UiAction::QtyQuality(item, quality, buy, 1));
    spawn_button(r, sprites, "Máx", 18.0, UiAction::QtyQualityMax(item, quality, buy));
}

fn section(card: &mut ChildSpawnerCommands, sprites: &Sprites, title: impl Into<String>) {
    card.spawn((
        Text::new(title.into()),
        font(sprites, 26.0),
        TextColor(TEXT),
        text_shadow(),
        Node { margin: UiRect::vertical(Val::Px(4.0)), ..default() },
    ));
}

fn row(
    card: &mut ChildSpawnerCommands,
    sprites: &Sprites,
    item: Item,
    detail: &str,
    buttons: impl FnOnce(&mut ChildSpawnerCommands),
) {
    card.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() })
        .with_children(|r| {
            r.spawn(item_icon(sprites, item, 32.0));
            r.spawn((
                Text::new(item.name()),
                font(sprites, 22.0),
                TextColor(TEXT),
                text_shadow(),
                Node { width: Val::Px(190.0), ..default() },
            ));
            r.spawn((
                Text::new(detail),
                font(sprites, 19.0),
                TextColor(MUTED),
                text_shadow(),
                Node { width: Val::Px(240.0), ..default() },
            ));
            buttons(r);
        });
}

fn row_quality(
    card: &mut ChildSpawnerCommands,
    sprites: &Sprites,
    item: Item,
    quality: Quality,
    detail: &str,
    buttons: impl FnOnce(&mut ChildSpawnerCommands),
) {
    card.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() })
        .with_children(|r| {
            r.spawn(item_icon(sprites, item, 32.0));
            let name_str = format!("{} {}", item.name(), quality.badge());
            r.spawn((
                Text::new(name_str),
                font(sprites, 22.0),
                TextColor(quality.color()),
                text_shadow(),
                Node { width: Val::Px(190.0), ..default() },
            ));
            r.spawn((
                Text::new(detail),
                font(sprites, 19.0),
                TextColor(MUTED),
                text_shadow(),
                Node { width: Val::Px(240.0), ..default() },
            ));
            buttons(r);
        });
}
