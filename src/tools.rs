//! Usar o item selecionado (Espaço) no tile à frente: ferramentas, sementes e comida.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::Gameplay;
use crate::bake::Painter;
use crate::cycle::DayPhase;
use crate::dialogue::Dialogue;
use crate::animals::{Animal, Gender, Species};
use crate::economy::{CropView, FarmState, Game, harvest, spawn_crop_view};
use crate::fx::{Focus, FocusKind, Pickups, Pop, Popups, spawn_burst, spawn_swing};
use crate::inventory::{CropData, CropKind, Inventory, Item};
use crate::map::{Ground, Map, MapState, Prop, TILE, tile_base, tile_center};
use crate::player::{Player, target_tile};
use crate::sprites::{DualSprite, Sprites, fx};
use crate::world::Travel;

pub fn plugin(app: &mut App) {
    // Depois de `interact`: o cursor da ação de "E" tem prioridade sobre o da ferramenta.
    app.add_systems(Update, use_item.in_set(Gameplay).after(crate::economy::interact));
}

/// Golpes para derrubar cada objeto.
fn hits_needed(kind: Prop) -> u8 {
    match kind {
        Prop::Oak | Prop::Pine => 4,
        Prop::Rock => 3,
        _ => 1,
    }
}

/// O que acontece se o item for usado neste tile (None: nada a fazer).
enum Use {
    Till,
    Water,
    Plant(CropKind),
    Reap,
    Cut,
    Chop,
    Break,
    Eat(f32),
    RegisterAnimal(Species, Gender),
}

fn energy_cost(u: &Use) -> f32 {
    match u {
        Use::Till | Use::Chop | Use::Break => 2.0,
        Use::Water | Use::Reap | Use::Cut => 1.0,
        Use::Plant(_) | Use::Eat(_) | Use::RegisterAnimal(_, _) => 0.0,
    }
}

#[allow(clippy::too_many_arguments)]
fn plan(item: Item, t: IVec2, map: &Map, farm: &FarmState, game: &Game, night: bool) -> Result<Use, &'static str> {
    if let Some(energy) = item.energy() {
        return Ok(Use::Eat(energy));
    }
    let on_farm = map.info.farm;
    let prop = map.prop_at(t).map(|p| p.kind);
    let free = prop.is_none() && !farm.factories.contains(&t);
    match item {
        _ if !on_farm && item.is_tool() => Err("Melhor não mexer no que não é seu."),
        Item::Hoe => match map.ground(t) {
            Some(Ground::Grass) if free && !map.has_decor(t) => Ok(Use::Till),
            Some(Ground::Grass) if map.has_decor(t) => Err("Corte o mato com a foice antes."),
            _ => Err("Aqui a enxada não entra."),
        },
        Item::WateringCan => match map.ground(t) {
            Some(Ground::Soil) if farm.watered.contains(&t) => Err("Já está regado."),
            Some(Ground::Soil) => Ok(Use::Water),
            _ => Err("O regador é para canteiros."),
        },
        _ if item.is_animal_deed() => {
            if !on_farm {
                Err("Animais só podem ser registrados na fazenda.")
            } else if let Some((species, gender)) = item.deed_animal() {
                Ok(Use::RegisterAnimal(species, gender))
            } else {
                Err("Certidão inválida.")
            }
        }
        _ if item.seed_crop().is_some() => match map.ground(t) {
            _ if !on_farm => Err("Sementes vão na terra da fazenda."),
            _ if night => Err("Ninguém planta de noite. Só colhe."),
            _ if game.inquisitor => Err("O Inquisidor confiscou o direito de plantar."),
            Some(Ground::Soil) if farm.crops.contains_key(&t) => Err("Já tem algo plantado aqui."),
            Some(Ground::Soil) if free => Ok(Use::Plant(item.seed_crop().unwrap())),
            _ => Err("Are a terra com a enxada primeiro."),
        },
        Item::Scythe => match (farm.ripe(t), map.has_decor(t)) {
            (Some(true), _) => Ok(Use::Reap),
            (_, true) => Ok(Use::Cut),
            _ => Err("Nada para cortar."),
        },
        Item::Axe => match prop {
            Some(Prop::Oak | Prop::Pine | Prop::Bush) => Ok(Use::Chop),
            _ => Err("O machado é para árvores e arbustos."),
        },
        Item::Pickaxe => match prop {
            Some(Prop::Rock) => Ok(Use::Break),
            _ => Err("A picareta é para pedras."),
        },
        _ => Err("Isso não se usa assim."),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn use_item(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    (mouse, window): (Res<ButtonInput<MouseButton>>, Single<&Window, With<bevy::window::PrimaryWindow>>),
    sprites: Res<Sprites>,
    phase: Res<State<DayPhase>>,
    (dialogue, travel, cutscenes): (Res<Dialogue>, Res<Travel>, Option<Res<crate::cutscene::CutsceneManager>>),
    (mut game, mut inv, mut farm): (ResMut<Game>, ResMut<Inventory>, ResMut<FarmState>),
    mut map: ResMut<Map>,
    (mut focus, mut popups, mut pickups): (ResMut<Focus>, ResMut<Popups>, ResMut<Pickups>),
    (mut images, layouts): (ResMut<Assets<Image>>, Res<Assets<TextureAtlasLayout>>),
    player: Single<(&Transform, &Player)>,
    crop_views: Query<(Entity, &CropView)>,
    mut hits: Local<HashMap<IVec2, u8>>,
) {
    if dialogue.is_open() || travel.busy() || cutscenes.is_some_and(|c| c.is_playing()) {
        return;
    }
    let Some(item) = inv.selected_item() else { return };
    let (transform, player) = *player;
    let t = target_tile(transform, player.facing);
    let night = *phase.get() == DayPhase::Night;
    let planned = plan(item, t, &map, &farm, &game, night);

    // Sem ação de "E" em vista, o cursor mostra onde a ferramenta vai bater —
    // só quando ela faz algo ali (sem poluir a tela à toa).
    if focus.0.is_none() && item.energy().is_none() && planned.is_ok() {
        let rect = Rect::from_center_size(tile_center(t), Vec2::splat(TILE));
        focus.0 = Some((rect, true, FocusKind::Tile));
    }
    if !(keys.just_pressed(KeyCode::Space) || crate::player::world_click(&mouse, &window, MouseButton::Left)) {
        return;
    }
    let head = transform.translation.truncate() + Vec2::new(0.0, 34.0);
    let action = match planned {
        Ok(action) => action,
        Err(why) => {
            game.say(why);
            return;
        }
    };
    if !game.spend(energy_cost(&action)) {
        game.say("Exausto. Coma alguma coisa ou durma.");
        return;
    }
    let base = tile_base(t);
    let here = map.id.clone();
    if item.is_tool() {
        // Arco da ferramenta na mão, para o lado que o jogador olha.
        let side = if player.facing == crate::character::Facing::Left { -1.0 } else { 1.0 };
        let hand = transform.translation.truncate() + Vec2::new(7.0 * side, 14.0);
        spawn_swing(&mut commands, &sprites, &here, hand, item.icon(), side < 0.0);
    }
    match action {
        Use::Eat(energy) => {
            inv.remove(item, 1);
            game.rest(energy);
            // Comer também fecha os arranhões das pragas (metade do que dá de energia).
            let healed = game.heal(energy * 0.5);
            let label = if healed >= 1.0 {
                format!("+{energy:.0} energia · +{healed:.0} vida")
            } else {
                format!("+{energy:.0} energia")
            };
            popups.push(head, label, Popups::GOOD);
            spawn_burst(&mut commands, &sprites, &here, head, fx::HEART, 2);
        }
        Use::Till => {
            map.set_ground(&mut Painter { images: &mut images, layouts: &layouts }, &sprites, t, Ground::Soil);
            farm.edits.ground.insert(t, Ground::Soil);
            spawn_burst(&mut commands, &sprites, &here, base, fx::DUST, 4);
        }
        Use::Water => {
            farm.watered.insert(t);
            spawn_burst(&mut commands, &sprites, &here, base + Vec2::Y * 6.0, fx::SPARKLE, 4);
        }
        Use::Plant(crop_kind) => {
            inv.remove(item, 1);
            farm.crops.insert(t, CropData { kind: crop_kind, days: 0 });
            let view = spawn_crop_view(&mut commands, &sprites, &here, t, crop_kind, 0);
            commands.entity(view).insert(Pop::default());
            spawn_burst(&mut commands, &sprites, &here, base, fx::DUST, 3);
        }
        Use::RegisterAnimal(species, gender) => {
            inv.remove(item, 1);
            farm.bought_animals.push((species, gender));
            let idx = farm.bought_animals.len() - 1;
            let base_x = 24.0 * 16.0 + (idx as f32 % 5.0) * 20.0;
            let base_y = -(12.0 * 16.0 + (idx as f32 / 5.0).floor() * 20.0);
            let home = Vec2::new(base_x, base_y);
            let (sprite, dual) = DualSprite::bundle(species.art(&sprites), species.first_frame(gender));

            commands.spawn((
                Animal {
                    species,
                    gender,
                    has_product: species.product(gender).is_some(),
                    home,
                    target: home,
                    rest: 1.0,
                    t: 0.0,
                    seed: 55.0 + idx as f32 * 23.0,
                },
                sprite,
                dual,
                Anchor::BOTTOM_CENTER,
                Transform::from_translation(home.extend(crate::map::depth(home.y))),
                DespawnOnExit(MapState(here.clone())),
            ))
            .with_child((
                DualSprite::bundle(&sprites.shadow, 0),
                Transform::from_xyz(0.0, 0.5, -0.01).with_scale(Vec3::splat(species.shadow())),
            ));

            popups.push(head, format!("+1 {}", species.name(gender)), Popups::GOOD);
            spawn_burst(&mut commands, &sprites, &here, head, fx::HEART, 4);
            game.say(format!("Você registrou {} na fazenda!", species.name(gender)));
        }
        Use::Reap => {
            if let Some((harvested, quality)) = harvest(&mut commands, &sprites, &mut farm, &mut inv, &crop_views, &here, t) {
                let badge = quality.badge();
                let pop_str = if !badge.is_empty() {
                    format!("+1 {} {}", harvested.name(), badge)
                } else {
                    format!("+1 {}", harvested.name())
                };
                popups.push(head, pop_str, Popups::GOOD);
                pickups.0.push((tile_center(t), harvested.icon()));
                if quality == crate::inventory::Quality::Supernatural {
                    game.say(format!("Incrível! A {} colhida pela foice reluz com modificador SOBRENATURAL!", harvested.name()));
                } else if quality != crate::inventory::Quality::Normal {
                    game.say(format!("Colheu {} {} de qualidade {}!", harvested.name(), badge, quality.name()));
                }
            }
        }
        Use::Cut => {
            map.remove_decor(&mut commands, t);
            farm.edits.removed.insert(t);
            spawn_burst(&mut commands, &sprites, &here, base, fx::LEAF, 3);
            // Nem todo tufo rende fibra.
            if (t.x + t.y) % 2 == 0 {
                inv.add(Item::Fiber, 1);
                popups.push(head, "+1 fibra", Popups::GOOD);
                pickups.0.push((tile_center(t), Item::Fiber.icon()));
            }
        }
        Use::Chop | Use::Break => {
            let Some(kind) = map.prop_at(t).map(|p| p.kind) else { return };
            let count = hits.entry(t).or_insert(0);
            *count += 1;
            let particle = if kind == Prop::Rock { fx::DUST } else { fx::LEAF };
            spawn_burst(&mut commands, &sprites, &here, base + Vec2::Y * 8.0, particle, 3);
            if let Some(entity) = map.prop_at(t).and_then(|p| p.entities.first().copied()) {
                commands.entity(entity).insert(Pop::default());
            }
            if *count >= hits_needed(kind) {
                hits.remove(&t);
                if let Some(anchor) = map.remove_prop(&mut commands, t) {
                    farm.edits.removed.insert(anchor);
                }
                let (loot, n) = match kind {
                    Prop::Oak | Prop::Pine => (Item::Wood, 4),
                    Prop::Bush => (Item::Wood, 1),
                    _ => (Item::Stone, 3),
                };
                inv.add(loot, n);
                if kind == Prop::Bush {
                    inv.add(Item::Fiber, 2);
                }
                popups.push(head, format!("+{n} {}", loot.name().to_lowercase()), Popups::GOOD);
                pickups.0.push((tile_center(t), loot.icon()));
            }
        }
    }
}
