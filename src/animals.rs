//! Bichos da fazenda: galinhas, ovelhas, cachorro, vacas, cabras, coelhos e cavalos.
//! Variantes de macho e fêmea, produtos de coleta (leite, ovos, lã, pêlo).

use bevy::prelude::*;
use bevy::sprite::Anchor;
use serde::{Deserialize, Serialize};

use crate::economy::FarmState;
use crate::inventory::Item;
use crate::map::{Map, MapState, depth, tile_base};
use crate::player::Player;
use crate::sprites::{Art, DualSprite, Sprites};
use crate::Gameplay;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        spawn_animals.in_set(crate::world::MapEntered).after(crate::world::enter_map).run_if(state_changed::<MapState>),
    )
    .add_systems(Update, roam.in_set(Gameplay));
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Species {
    Chicken,
    Sheep,
    Dog,
    Cow,
    Goat,
    Rabbit,
    Horse,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Gender {
    Female,
    Male,
}

impl Gender {
    pub fn label(self) -> &'static str {
        match self {
            Gender::Female => "Fêmea",
            Gender::Male => "Macho",
        }
    }
}

impl Species {
    pub fn name(self, gender: Gender) -> &'static str {
        match (self, gender) {
            (Species::Chicken, Gender::Female) => "a galinha",
            (Species::Chicken, Gender::Male) => "o galo",
            (Species::Sheep, Gender::Female) => "a ovelha",
            (Species::Sheep, Gender::Male) => "o carneiro",
            (Species::Dog, Gender::Female) => "a cadela",
            (Species::Dog, Gender::Male) => "o cachorro",
            (Species::Cow, Gender::Female) => "a vaca",
            (Species::Cow, Gender::Male) => "o touro",
            (Species::Goat, Gender::Female) => "a cabra",
            (Species::Goat, Gender::Male) => "o bode",
            (Species::Rabbit, Gender::Female) => "a coelha",
            (Species::Rabbit, Gender::Male) => "o coelho",
            (Species::Horse, Gender::Female) => "a égua",
            (Species::Horse, Gender::Male) => "o cavalo",
        }
    }

    pub fn product(self, gender: Gender) -> Option<Item> {
        match (self, gender) {
            (Species::Cow, Gender::Female) => Some(Item::CowMilk),
            (Species::Goat, Gender::Female) => Some(Item::GoatMilk),
            (Species::Chicken, Gender::Female) => Some(Item::Egg),
            (Species::Sheep, _) => Some(Item::Wool),
            (Species::Rabbit, _) => Some(Item::RabbitWool),
            _ => None,
        }
    }

    pub fn product_action(self, _gender: Gender) -> &'static str {
        match self {
            Species::Cow | Species::Goat => "Ordenhar",
            Species::Chicken => "Coletar ovos d",
            Species::Sheep => "Tosquiar",
            Species::Rabbit => "Coletar pêlo d",
            _ => "Cuidar d",
        }
    }

    fn speed(self) -> f32 {
        match self {
            Species::Chicken => 18.0,
            Species::Sheep => 10.0,
            Species::Dog => 40.0,
            Species::Cow => 8.0,
            Species::Goat => 14.0,
            Species::Rabbit => 28.0,
            Species::Horse => 35.0,
        }
    }

    /// Quanto se afasta de casa (em pixels).
    fn range(self) -> f32 {
        match self {
            Species::Chicken => 36.0,
            Species::Sheep => 44.0,
            Species::Dog => 0.0,
            Species::Cow => 50.0,
            Species::Goat => 48.0,
            Species::Rabbit => 32.0,
            Species::Horse => 60.0,
        }
    }

    /// Tamanho da sombra embaixo (os bichos grandes ganham uma sombra maior).
    pub fn shadow(self) -> f32 {
        match self {
            Species::Cow | Species::Horse => 1.5,
            Species::Sheep | Species::Goat => 1.1,
            Species::Rabbit => 0.6,
            _ => 0.8,
        }
    }

    pub fn art<'a>(self, sprites: &'a Sprites) -> &'a Art {
        match self {
            Species::Chicken => &sprites.chicken,
            Species::Sheep => &sprites.sheep,
            Species::Dog => &sprites.dog,
            Species::Cow => &sprites.cow,
            Species::Goat => &sprites.goat,
            Species::Rabbit => &sprites.rabbit,
            Species::Horse => &sprites.horse,
        }
    }

    /// Primeiro quadro da linha do animal: as folhas têm a fêmea na linha 0 e o macho
    /// na linha 1 (galo, carneiro, touro...). O cachorro tem uma linha só.
    pub fn first_frame(self, gender: Gender) -> usize {
        match (self, gender) {
            (Species::Dog, _) | (_, Gender::Female) => 0,
            (_, Gender::Male) => 4,
        }
    }
}

#[derive(Component)]
pub struct Animal {
    pub species: Species,
    pub gender: Gender,
    pub has_product: bool,
    pub home: Vec2,
    pub target: Vec2,
    /// Segundos parado (bicando, pastando, sentado).
    pub rest: f32,
    pub t: f32,
    pub seed: f32,
}

fn rand(seed: &mut f32) -> f32 {
    *seed = (*seed * 9301.0 + 49297.0) % 233280.0;
    *seed / 233280.0
}

fn spawn_animals(mut commands: Commands, sprites: Res<Sprites>, map: Option<Res<Map>>, farm: Option<Res<FarmState>>) {
    let Some(map) = map else { return };

    // Animais fixos do mapa
    for (i, (ch, t)) in map.animals.iter().enumerate() {
        let (species, gender) = match ch {
            'a' => (Species::Chicken, Gender::Female),
            'e' => (Species::Sheep, Gender::Female),
            _ => (Species::Dog, Gender::Male),
        };
        let home = tile_base(*t) + Vec2::Y * 2.0;
        let (sprite, dual) = DualSprite::bundle(species.art(&sprites), species.first_frame(gender));
        commands.spawn((
            Animal {
                species,
                gender,
                has_product: species.product(gender).is_some(),
                home,
                target: home,
                rest: i as f32 * 0.7,
                t: 0.0,
                seed: 17.0 + i as f32 * 31.0,
            },
            sprite,
            dual,
            Anchor::BOTTOM_CENTER,
            Transform::from_translation(home.extend(depth(home.y))),
            DespawnOnExit(MapState(map.id.clone())),
        ))
        .with_child((
            DualSprite::bundle(&sprites.shadow, 0),
            Transform::from_xyz(0.0, 0.5, -0.01).with_scale(Vec3::splat(species.shadow())),
        ));
    }

    // Animais comprados pelo jogador (só aparecem na fazenda)
    if map.info.farm {
        if let Some(farm) = farm {
            for (idx, &(species, gender)) in farm.bought_animals.iter().enumerate() {
                // Posiciona perto do celeiro/pasto (x: 28 + offset, y: -20)
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
                        rest: (idx + 1) as f32 * 0.5,
                        t: 0.0,
                        seed: 43.0 + idx as f32 * 27.0,
                    },
                    sprite,
                    dual,
                    Anchor::BOTTOM_CENTER,
                    Transform::from_translation(home.extend(depth(home.y))),
                    DespawnOnExit(MapState(map.id.clone())),
                ))
                .with_child((
                    DualSprite::bundle(&sprites.shadow, 0),
                    Transform::from_xyz(0.0, 0.5, -0.01).with_scale(Vec3::splat(species.shadow())),
                ));
            }
        }
    }
}

/// Até onde o cachorro se afasta de casa, a que distância de casa ele nota você e
/// quanto fica de lado quando chega.
const DOG_LEASH: f32 = 40.0;
const DOG_NOTICE: f32 = 32.0;
const DOG_SIDE: f32 = 20.0;

/// Passeiam perto de casa, param um pouco (bicar, pastar) e escolhem outro ponto.
/// O cachorro fica sentado abanando o rabo e vem até você quando você chega perto.
fn roam(
    time: Res<Time>,
    map: Res<Map>,
    player: Single<&Transform, (With<Player>, Without<Animal>)>,
    mut animals: Query<(&mut Animal, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    let you = player.translation.truncate();
    for (mut a, mut transform, mut sprite) in &mut animals {
        a.t += dt;
        let here = transform.translation.truncate();
        if a.species == Species::Dog {
            // Fica perto de casa (coleira curta). Se você chega perto, vem até o seu lado —
            // não na sua frente, para não atrapalhar o que você for fazer — e lá fica.
            a.rest = 0.0;
            let side = if here.x < you.x { -DOG_SIDE } else { DOG_SIDE };
            let beside = you + Vec2::new(side, 2.0);
            let close = you.distance(a.home) < DOG_NOTICE + DOG_LEASH;
            let wanted = if close { beside } else { a.home };
            let wanted = a.home + (wanted - a.home).clamp_length_max(DOG_LEASH);
            // Só se mexe se o lugar desejado estiver longe: evita ficar "tremendo" atrás de você.
            if wanted.distance(here) > 10.0 {
                a.target = wanted;
            } else if here.distance(a.target) < 2.0 {
                a.target = here;
            }
        } else if a.rest > 0.0 {
            a.rest -= dt;
        } else if here.distance(a.target) < 2.0 {
            let mut seed = a.seed;
            let angle = rand(&mut seed) * std::f32::consts::TAU;
            let r = rand(&mut seed) * a.species.range();
            a.target = a.home + Vec2::new(angle.cos(), angle.sin() * 0.6) * r;
            a.rest = 1.0 + rand(&mut seed) * 3.0;
            a.seed = seed;
        }

        let to = a.target - here;
        let moving = to.length() > 2.0 && a.rest <= 0.0;
        let frame = if moving {
            let step = to.normalize() * a.species.speed() * dt;
            let next = here + step;
            let feet = Rect::new(next.x - 3.0, next.y, next.x + 3.0, next.y + 3.0);
            if map.solid(feet) {
                // Bateu em algo: desiste e escolhe outro ponto depois.
                a.target = here;
            } else {
                transform.translation = next.extend(depth(next.y));
            }
            sprite.flip_x = to.x < 0.0;
            // Andando: quadros 0-1 (2-3 no cachorro).
            let walk = (a.t * 8.0) as usize % 2;
            if a.species == Species::Dog { 2 + walk } else { walk }
        } else {
            // Parado: bicando/pastando (2-3) ou o cachorro sentado abanando (0-1).
            let idle = (a.t * 3.0) as usize % 2;
            if a.species == Species::Dog { idle } else { 2 + idle }
        };
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = a.species.first_frame(a.gender) + frame;
        }
    }
}
