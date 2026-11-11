//! Dicas discretas em cima de plantas e bichos: o que dá para fazer ali, sem precisar mirar.
//! Brilho = pronta para colher; regadorzinho = seca hoje (só de dia); ícone do produto = bicho
//! com algo para coletar. Ficam meio apagadas de longe, acendem de perto e somem quando você
//! mira no alvo (aí quem fala é a etiqueta normal da interação).

use bevy::prelude::*;

use crate::Gameplay;
use crate::animals::{Animal, Species};
use crate::cycle::DayPhase;
use crate::economy::{CropView, FarmState};
use crate::inventory::{CropKind, Item};
use crate::player::Player;
use crate::sprites::{DualSprite, Sprites, fx};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (attach, update).chain().in_set(Gameplay));
}

/// Já ganhou a sua dica (evita criar outra a cada quadro).
#[derive(Component)]
struct HasTip;

#[derive(Component)]
struct Tip {
    /// Altura da dica acima da base do dono.
    height: f32,
    /// Quadro atual mostrado (para trocar só quando muda).
    shown: Option<Shown>,
}

#[derive(Clone, Copy, PartialEq)]
enum Shown {
    Ripe,
    Thirsty,
    Product(Item),
}

fn crop_height(kind: CropKind) -> f32 {
    match kind {
        CropKind::Corn | CropKind::Tomato | CropKind::Grape => 31.0,
        _ => 19.0,
    }
}

fn animal_height(species: Species) -> f32 {
    match species {
        Species::Chicken | Species::Rabbit | Species::Dog => 15.0,
        Species::Sheep | Species::Goat | Species::Cow => 22.0,
        Species::Horse => 31.0,
    }
}

fn attach(
    mut commands: Commands,
    sprites: Res<Sprites>,
    farm: Res<FarmState>,
    crops: Query<(Entity, &CropView), Without<HasTip>>,
    animals: Query<(Entity, &Animal), Without<HasTip>>,
) {
    let mut add = |entity: Entity, height: f32| {
        let (sprite, dual) = DualSprite::bundle(&sprites.fx, fx::SPARKLE);
        commands.entity(entity).insert(HasTip).with_child((
            Tip { height, shown: None },
            sprite,
            dual,
            Transform::from_xyz(0.0, height, 0.5),
            Visibility::Hidden,
        ));
    };
    for (entity, view) in &crops {
        let height = farm.crops.get(&view.0).map_or(19.0, |c| crop_height(c.kind));
        add(entity, height);
    }
    for (entity, animal) in &animals {
        // O cachorro não produz nada: nem precisa de dica.
        if animal.species != Species::Dog {
            add(entity, animal_height(animal.species));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update(
    time: Res<Time>,
    sprites: Res<Sprites>,
    farm: Res<FarmState>,
    phase: Res<State<DayPhase>>,
    focus: Res<crate::fx::Focus>,
    player: Single<&Transform, With<Player>>,
    owners: Query<(&GlobalTransform, Option<&CropView>, Option<&Animal>)>,
    mut tips: Query<(&ChildOf, &mut Tip, &mut Transform, &mut Sprite, &mut DualSprite, &mut Visibility), Without<Player>>,
) {
    let you = player.translation.truncate();
    let day = *phase.get() == DayPhase::Day;
    let t = time.elapsed_secs();
    for (child_of, mut tip, mut transform, mut sprite, mut dual, mut visibility) in &mut tips {
        let Ok((owner, crop, animal)) = owners.get(child_of.parent()) else { continue };
        let wanted = if let Some(view) = crop {
            farm.crops.get(&view.0).and_then(|c| {
                if c.days >= c.kind.growth_days() {
                    Some(Shown::Ripe)
                } else if day && !farm.watered.contains(&view.0) {
                    Some(Shown::Thirsty)
                } else {
                    None
                }
            })
        } else if let Some(a) = animal {
            a.species.product(a.gender).filter(|_| a.has_product).map(Shown::Product)
        } else {
            None
        };
        let at = owner.translation().truncate();
        // Mirando no alvo, a etiqueta da interação já diz tudo.
        let aimed = focus.0.is_some_and(|(rect, _, _)| rect.contains(at + Vec2::Y * 4.0));
        let Some(wanted) = wanted.filter(|_| !aimed) else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        visibility.set_if_neq(Visibility::Inherited);

        if tip.shown != Some(wanted) {
            tip.shown = Some(wanted);
            let (art, index, scale) = match wanted {
                Shown::Ripe => (&sprites.fx, fx::SPARKLE, 1.0),
                Shown::Thirsty => (&sprites.items, Item::WateringCan.icon(), 0.5),
                Shown::Product(item) => (&sprites.items, item.icon(), 0.5),
            };
            *dual = DualSprite(art.clone());
            *sprite = art.sprite(*phase.get(), index);
            transform.scale = Vec3::splat(scale);
        }
        // Brilho pisca entre os 2 quadros; tudo boia 1 px, cada um no seu ritmo.
        let seed = at.x * 0.37 + at.y * 0.11;
        if wanted == Shown::Ripe
            && let Some(atlas) = &mut sprite.texture_atlas
        {
            atlas.index = fx::SPARKLE + ((t * 2.5 + seed) as usize % 2);
        }
        transform.translation.y = tip.height + (t * 2.2 + seed).sin().round();
        // Discreta de longe, nítida de perto.
        let near = 1.0 - ((you.distance(at) - 24.0) / 72.0).clamp(0.0, 1.0);
        sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.45 + 0.45 * near);
    }
}
